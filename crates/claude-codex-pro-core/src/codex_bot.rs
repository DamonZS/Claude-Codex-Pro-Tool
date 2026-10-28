//! codex-bot 数据层：只读读取 DS-Bot 插件在 `$DSH_HOME/bot` 下的快照，并把它变成 helper 路由响应。
//!
//! 调用方只有 `launcher::handle_helper_connection`（挂 `/codex-bot/*`）与测试。
//!
//! 只读纪律：两个 SQLite 一律用 `SQLITE_OPEN_READ_ONLY` 打开，并额外 `PRAGMA query_only = 1`；
//! 本模块从不写 `$DSH_HOME`、DS-Bot 数据目录或 Codex 目录。
//!
//! 降级契约（spec/acceptance `codex-bot-injection-panel`）：
//!   * 数据源未建立（`missing-home` / `missing-state` / state.json `parse-error` /
//!     state.json `unsupported-schema`）→ `available:false` + `reason`，HTTP 仍是 200；
//!   * 一次**瞬时**的 SQLite 失败（`busy` / `parse-error` / `unsupported-schema`）→ 返回上一次
//!     可用快照并置 `stale:true`；
//!   * 数据库只是不存在（`missing-database`）是稳定的部分缺失：`available` 仍为 true，
//!     对应的 `*Available` 标志为 false，且不把 `stale` 当作噪声一直点亮。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use chrono::{Local, Utc};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

/// 进程内快照缓存的默认 TTL（spec F1：15 秒）。
pub const CACHE_TTL: Duration = Duration::from_secs(15);
/// 列表路由的默认条数。
pub const DEFAULT_LIMIT: usize = 20;
/// 列表路由的上限条数（超出按上限截断）。
pub const MAX_LIMIT: usize = 100;
/// 消息正文截断长度（字符）。
pub const MESSAGE_TEXT_LIMIT: usize = 2000;
/// 房间预览截断长度（字符）。
pub const PREVIEW_TEXT_LIMIT: usize = 60;
/// 路由前缀。
pub const ROUTE_PREFIX: &str = "/codex-bot";

/// DSH_HOME 未设置/空白时回落的主目录名（`~/.dsh`，与 DS-Bot `dshHomeOf` 一致）。
pub const DSH_DIR_NAME: &str = ".dsh";
/// DSH 主目录下 DS-Bot 的数据目录名。
pub const BOT_DIR_NAME: &str = "bot";
/// 团队状态文件名。
pub const STATE_FILE_NAME: &str = "state.json";
/// 聊天索引数据库名。
pub const CHAT_INDEX_FILE_NAME: &str = "chat-index.sqlite";
/// 用量账本数据库名。
pub const USAGE_FILE_NAME: &str = "usage.sqlite";

/// 解析出的 bot 目录不存在。
pub const REASON_MISSING_HOME: &str = "missing-home";
/// bot 目录存在但没有 state.json。
pub const REASON_MISSING_STATE: &str = "missing-state";
/// 需要的数据库不存在。
pub const REASON_MISSING_DATABASE: &str = "missing-database";
/// JSON 或数据库内容无法解析。
pub const REASON_PARSE_ERROR: &str = "parse-error";
/// SQLite 被写事务占用（SQLITE_BUSY / SQLITE_LOCKED）。
pub const REASON_BUSY: &str = "busy";
/// 文件在，但结构不是我们认识的版本（缺表/缺列/字段类型变了）。
pub const REASON_UNSUPPORTED_SCHEMA: &str = "unsupported-schema";

const DAY_MS: i64 = 86_400_000;
/// `week` 窗口：含今天在内的 7 个自然日（与 DS-Bot Usage 页 `7d` 一致）。
const WEEK_DAYS: i64 = 7;
const MESSAGE_CACHE_ENTRIES: usize = 32;
const JSON_CONTENT_TYPE: &str = "application/json; charset=utf-8";
/// 读锁等待时间：0 表示立刻看到 SQLITE_BUSY 并按契约降级（WAL 下读不会被写阻塞，所以不会误伤）。
const READ_BUSY_TIMEOUT_MS: u64 = 0;

/// `/codex-bot` 及其子路径（不含查询串）。
pub fn is_codex_bot_path(path: &str) -> bool {
    path == ROUTE_PREFIX || path.starts_with("/codex-bot/")
}

// ---------------------------------------------------------------------------
// 路径解析
// ---------------------------------------------------------------------------

/// `$DSH_HOME/bot`；DSH_HOME 为空或只有空白时回落到 `~/.dsh/bot`。
/// 与 DS-Bot `src/host/folders.js` 的 `dshHomeOf(...)+'bot'` 一致。
pub fn codex_bot_home() -> PathBuf {
    codex_bot_home_from(std::env::var("DSH_HOME").ok().as_deref())
}

/// 与 [`codex_bot_home`] 相同的规则，但由调用方给值（便于单测覆盖 已设置/空白/未设置 三种输入）。
pub fn codex_bot_home_from(value: Option<&str>) -> PathBuf {
    let root = match value.map(str::trim) {
        Some(text) if !text.is_empty() => expand_home(text),
        // 与 DS-Bot `dshHomeOf` 一致：未设置/空白都落到 `~/.dsh`，再拼 `bot`。
        _ => home_dir().join(DSH_DIR_NAME),
    };
    root.join(BOT_DIR_NAME)
}

/// `~`、`~/x`、`~\x` 展开到用户主目录，其余原样（相对路径按当前工作目录解析成绝对路径）。
pub fn expand_home(text: &str) -> PathBuf {
    let text = text.trim();
    if text == "~" {
        return home_dir();
    }
    if let Some(rest) = text.strip_prefix("~/").or_else(|| text.strip_prefix("~\\")) {
        return home_dir().join(rest);
    }
    let path = PathBuf::from(text);
    if path.is_absolute() {
        return path;
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(&path))
        .unwrap_or(path)
}

fn home_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

// ---------------------------------------------------------------------------
// 快照数据
// ---------------------------------------------------------------------------

/// state.json 里的一个 Bot。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BotEntry {
    pub id: String,
    pub name: String,
    /// 本机真实数据没有这个字段，缺失时不要做非空假设。
    pub work_dir: Option<String>,
    pub created_at: Option<i64>,
    /// DS-Bot `chatOf` 用的会话 id；缺失时 Bot 自己的会话就是 `id`。
    pub session_id: Option<String>,
}

/// state.json 里的一个房间。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoomEntry {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,
    pub work_dir: Option<String>,
    /// `botId -> sessionId`；房间自己的会话 id 也在候选里。
    pub sessions: Vec<String>,
}

/// 从 state.json 解析出的团队状态。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BotStateSnapshot {
    pub revision: Option<i64>,
    pub main_bot_ids: Vec<String>,
    pub bots: Vec<BotEntry>,
    pub rooms: Vec<RoomEntry>,
}

/// 一个窗口里的 token 合计。`total = input + output`（spec 的 summary 形状只有这两个分项）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct TokenTotals {
    pub input: i64,
    pub output: i64,
    pub total: i64,
}

impl TokenTotals {
    fn add(&mut self, input: i64, output: i64) {
        self.input += input;
        self.output += output;
        self.total = self.input + self.output;
    }
}

/// `today`（本机时区的今天）与 `week`（含今天在内的 7 个自然日）两个窗口。
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct UsageWindows {
    pub today: TokenTotals,
    pub week: TokenTotals,
}

/// usage.sqlite `calls` 表的一行（只取本模块要用的列）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageRow {
    pub session: String,
    pub time: i64,
    pub provider: String,
    pub model: String,
    pub input: i64,
    pub output: i64,
}

/// usage.sqlite `owners` 表的一行：会话属于哪个 Bot / 房间 / agent。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OwnerRow {
    pub owner: String,
    pub name: String,
}

/// chat-index.sqlite `lines` 表的一行（`text` 已按 2000 字符截断）。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ChatLine {
    pub text: String,
    pub session: String,
    pub seq: i64,
    pub time: i64,
    pub who: String,
}

/// 每个 Bot 的用量/活跃度汇总（内部使用，路由按需投影）。
#[derive(Debug, Clone, Default)]
struct BotDetails {
    today: TokenTotals,
    week: TokenTotals,
    model: Option<String>,
    last_activity_at: Option<i64>,
    message_count: Option<i64>,
    sessions: Vec<String>,
}

/// 每个房间的消息线索（内部使用）。
#[derive(Debug, Clone, Default)]
struct RoomDetails {
    last_message_at: Option<i64>,
    preview: Option<String>,
}

/// 一次数据层读取的完整结果（可被短 TTL 缓存与 `stale` 复用）。
#[derive(Debug, Clone, Default)]
pub struct CodexBotData {
    pub home: PathBuf,
    pub available: bool,
    /// 数据源不可用时的原因（`available:false` 时一定有值）。
    pub reason: Option<&'static str>,
    /// 这是上一次快照（本次读取瞬时失败）。
    pub stale: bool,
    pub revision: Option<i64>,
    pub main_bot_ids: Vec<String>,
    pub bots: Vec<BotEntry>,
    pub rooms: Vec<RoomEntry>,
    pub chat_index_available: bool,
    pub chat_index_reason: Option<&'static str>,
    pub usage_available: bool,
    pub usage_reason: Option<&'static str>,
    pub usage: UsageWindows,
    bot_details: HashMap<String, BotDetails>,
    room_details: HashMap<String, RoomDetails>,
}

// ---------------------------------------------------------------------------
// SQLite 只读打开
// ---------------------------------------------------------------------------

/// 只读打开一个 SQLite 文件（`SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only = 1`）。
///
/// 对夹具/真实库的任何写入都会失败，这正是验收标准 2 要的证据。
pub fn open_read_only(path: &Path) -> Result<Connection, &'static str> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| classify_sqlite_error(&error))?;
    connection
        .busy_timeout(Duration::from_millis(READ_BUSY_TIMEOUT_MS))
        .map_err(|error| classify_sqlite_error(&error))?;
    // 双保险：即使调用方以后改错 flag，这条连接也不会写出一个字节。
    connection
        .execute_batch("PRAGMA query_only = 1")
        .map_err(|error| classify_sqlite_error(&error))?;
    Ok(connection)
}

/// 把 rusqlite 错误归入 spec 允许的 reason 取值。
fn classify_sqlite_error(error: &rusqlite::Error) -> &'static str {
    let rusqlite::Error::SqliteFailure(code, message) = error else {
        // 类型转换/UTF-8 一类的问题：文件在，但内容不是我们认识的东西。
        return REASON_PARSE_ERROR;
    };
    match code.code {
        rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked => REASON_BUSY,
        rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase => {
            REASON_PARSE_ERROR
        }
        _ => {
            let text = message.as_deref().unwrap_or_default();
            if text.contains("no such table") || text.contains("no such column") {
                REASON_UNSUPPORTED_SCHEMA
            } else if text.contains("unable to open") || text.contains("readonly") {
                REASON_MISSING_DATABASE
            } else {
                // 未知的 SQLite 失败按瞬时处理：宁可返回上一次快照，也不要 5xx。
                REASON_BUSY
            }
        }
    }
}

/// 瞬时失败（值得用上一次快照兜底并置 `stale`）；`missing-database` 是稳定缺失，不兜底。
fn is_transient_reason(reason: &str) -> bool {
    matches!(
        reason,
        REASON_BUSY | REASON_PARSE_ERROR | REASON_UNSUPPORTED_SCHEMA
    )
}

/// 容错的列读取：NULL / 类型意外都不 panic。
fn column_i64(row: &rusqlite::Row<'_>, index: usize) -> i64 {
    match row.get_ref(index) {
        Ok(ValueRef::Integer(value)) => value,
        Ok(ValueRef::Real(value)) => value as i64,
        Ok(ValueRef::Text(bytes)) => std::str::from_utf8(bytes)
            .ok()
            .and_then(|text| text.trim().parse::<i64>().ok())
            .unwrap_or(0),
        _ => 0,
    }
}

/// 容错的列读取：非文本列转成字符串，NULL 给空串。
fn column_string(row: &rusqlite::Row<'_>, index: usize) -> String {
    match row.get_ref(index) {
        Ok(ValueRef::Text(bytes)) => String::from_utf8_lossy(bytes).into_owned(),
        Ok(ValueRef::Integer(value)) => value.to_string(),
        Ok(ValueRef::Real(value)) => value.to_string(),
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------------
// state.json
// ---------------------------------------------------------------------------

/// 解析 state.json 字节。`Err` 的取值只会是 `parse-error` 或 `unsupported-schema`。
pub fn parse_state_json(bytes: &[u8]) -> Result<BotStateSnapshot, &'static str> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| REASON_PARSE_ERROR)?;
    parse_state_value(&value)
}

fn parse_state_value(value: &serde_json::Value) -> Result<BotStateSnapshot, &'static str> {
    let object = value.as_object().ok_or(REASON_UNSUPPORTED_SCHEMA)?;

    let revision = match object.get("revision") {
        Some(value) => Some(value.as_i64().ok_or(REASON_UNSUPPORTED_SCHEMA)?),
        None => None,
    };
    let main_bot_ids = match object.get("mainBotIds") {
        Some(value) => value
            .as_array()
            .ok_or(REASON_UNSUPPORTED_SCHEMA)?
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect(),
        None => Vec::new(),
    };

    // `bots` 是必需字段：它是一张以 Bot 会话 id 为 key 的 map。
    let bots_object = object
        .get("bots")
        .ok_or(REASON_UNSUPPORTED_SCHEMA)?
        .as_object()
        .ok_or(REASON_UNSUPPORTED_SCHEMA)?;
    let mut bots = Vec::with_capacity(bots_object.len());
    for (key, entry) in bots_object {
        let entry = entry.as_object().ok_or(REASON_UNSUPPORTED_SCHEMA)?;
        let id = entry
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(key)
            .to_string();
        let name = entry
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(&id)
            .to_string();
        bots.push(BotEntry {
            work_dir: entry
                .get("workDir")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            created_at: entry.get("createdAt").and_then(serde_json::Value::as_i64),
            session_id: entry
                .get("sessionId")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            id,
            name,
        });
    }

    // `rooms` 可以是空对象（本机真实数据就是 0 条），但存在时必须是对象。
    let mut rooms = Vec::new();
    if let Some(value) = object.get("rooms") {
        let rooms_object = value.as_object().ok_or(REASON_UNSUPPORTED_SCHEMA)?;
        rooms.reserve(rooms_object.len());
        for (key, entry) in rooms_object {
            let entry = entry.as_object().ok_or(REASON_UNSUPPORTED_SCHEMA)?;
            rooms.push(RoomEntry {
                id: entry
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(key)
                    .to_string(),
                name: entry
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(key)
                    .to_string(),
                members: entry
                    .get("members")
                    .and_then(serde_json::Value::as_array)
                    .map(|members| {
                        members
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                work_dir: entry
                    .get("workDir")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                sessions: entry
                    .get("sessions")
                    .and_then(serde_json::Value::as_object)
                    .map(|sessions| {
                        sessions
                            .values()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
            });
        }
    }

    Ok(BotStateSnapshot {
        revision,
        main_bot_ids,
        bots,
        rooms,
    })
}

// ---------------------------------------------------------------------------
// usage.sqlite
// ---------------------------------------------------------------------------

struct UsageTable {
    rows: Vec<UsageRow>,
    owners: HashMap<String, OwnerRow>,
}

fn read_usage_table(path: &Path) -> Result<UsageTable, &'static str> {
    let connection = open_read_only(path)?;

    let mut rows = Vec::new();
    {
        let mut statement = connection
            .prepare("SELECT session, time, provider, model, input, output FROM calls")
            .map_err(|error| classify_sqlite_error(&error))?;
        let mut query = statement
            .query([])
            .map_err(|error| classify_sqlite_error(&error))?;
        while let Some(row) = query
            .next()
            .map_err(|error| classify_sqlite_error(&error))?
        {
            rows.push(UsageRow {
                session: column_string(row, 0),
                time: column_i64(row, 1),
                provider: column_string(row, 2),
                model: column_string(row, 3),
                input: column_i64(row, 4),
                output: column_i64(row, 5),
            });
        }
    }

    // owners 缺失（更老的账本）只影响归属，不影响合计：那时按 session == bot id 兜底。
    let mut owners = HashMap::new();
    match connection.prepare("SELECT session, owner, name FROM owners") {
        Ok(mut statement) => {
            let mut query = statement
                .query([])
                .map_err(|error| classify_sqlite_error(&error))?;
            while let Some(row) = query
                .next()
                .map_err(|error| classify_sqlite_error(&error))?
            {
                let session = column_string(row, 0);
                owners.insert(
                    session,
                    OwnerRow {
                        owner: column_string(row, 1),
                        name: column_string(row, 2),
                    },
                );
            }
        }
        Err(error) => {
            let reason = classify_sqlite_error(&error);
            if reason != REASON_UNSUPPORTED_SCHEMA {
                return Err(reason);
            }
        }
    }

    Ok(UsageTable { rows, owners })
}

/// 今天（按给定时区偏移）开始时刻的 epoch 毫秒。
fn start_of_local_day_ms(now_ms: i64, offset_seconds: i32) -> i64 {
    let shift = i64::from(offset_seconds) * 1000;
    (now_ms + shift).div_euclid(DAY_MS) * DAY_MS - shift
}

/// 汇总 `today` / `week` 两个窗口（纯函数，测试里给固定 now 与时区偏移）。
pub fn aggregate_usage(rows: &[UsageRow], now_ms: i64, offset_seconds: i32) -> UsageWindows {
    let day_start = start_of_local_day_ms(now_ms, offset_seconds);
    let week_start = day_start - (WEEK_DAYS - 1) * DAY_MS;
    let end = day_start + DAY_MS;
    let mut windows = UsageWindows::default();
    for row in rows {
        if row.time >= week_start && row.time < end {
            windows.week.add(row.input, row.output);
        }
        if row.time >= day_start && row.time < end {
            windows.today.add(row.input, row.output);
        }
    }
    windows
}

#[derive(Debug, Clone, Default)]
struct BotUsageRollup {
    today: TokenTotals,
    week: TokenTotals,
    model: Option<String>,
    last_activity_at: Option<i64>,
    sessions: Vec<String>,
}

/// 把 calls 通过 owners 归到 Bot：`owners.owner` 是已知 Bot id 就归它，
/// 否则会话 id 本身是 Bot id 时也归它（owners 表为空的旧库）。房间/agent 的 owner 不归任何 Bot。
fn attribute_usage(
    rows: &[UsageRow],
    owners: &HashMap<String, OwnerRow>,
    bot_ids: &HashSet<String>,
    now_ms: i64,
    offset_seconds: i32,
) -> HashMap<String, BotUsageRollup> {
    let day_start = start_of_local_day_ms(now_ms, offset_seconds);
    let week_start = day_start - (WEEK_DAYS - 1) * DAY_MS;
    let end = day_start + DAY_MS;
    let mut rollups: HashMap<String, BotUsageRollup> = HashMap::new();

    for row in rows {
        let owner = owners
            .get(&row.session)
            .map(|owner| owner.owner.as_str())
            .filter(|owner| bot_ids.contains(*owner))
            .or_else(|| {
                bot_ids
                    .contains(row.session.as_str())
                    .then_some(row.session.as_str())
            });
        let Some(bot_id) = owner else { continue };

        let rollup = rollups.entry(bot_id.to_string()).or_default();
        if !rollup
            .sessions
            .iter()
            .any(|session| session == &row.session)
        {
            rollup.sessions.push(row.session.clone());
        }
        if row.time >= week_start && row.time < end {
            rollup.week.add(row.input, row.output);
        }
        if row.time >= day_start && row.time < end {
            rollup.today.add(row.input, row.output);
        }
        if rollup
            .last_activity_at
            .is_none_or(|current| row.time >= current)
        {
            rollup.last_activity_at = Some(row.time);
            if !row.model.is_empty() {
                rollup.model = Some(row.model.clone());
            }
        }
    }

    rollups
}

// ---------------------------------------------------------------------------
// chat-index.sqlite
// ---------------------------------------------------------------------------

struct ChatIndex {
    connection: Connection,
}

impl ChatIndex {
    fn open(path: &Path) -> Result<Self, &'static str> {
        let connection = open_read_only(path)?;
        // 先探一次表：缺表要报 unsupported-schema，而不是把它当成空结果。
        connection
            .query_row("SELECT count(*) FROM lines", [], |row| row.get::<_, i64>(0))
            .map_err(|error| classify_sqlite_error(&error))?;
        Ok(Self { connection })
    }

    fn count(&self, session: &str) -> Result<i64, &'static str> {
        self.connection
            .query_row(
                "SELECT count(*) FROM lines WHERE session = ?1",
                rusqlite::params![session],
                |row| Ok(column_i64(row, 0)),
            )
            .map_err(|error| classify_sqlite_error(&error))
    }

    fn latest(&self, session: &str) -> Result<Option<ChatLine>, &'static str> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT text, session, seq, time, who FROM lines WHERE session = ?1 ORDER BY seq DESC LIMIT 1",
            )
            .map_err(|error| classify_sqlite_error(&error))?;
        let mut query = statement
            .query(rusqlite::params![session])
            .map_err(|error| classify_sqlite_error(&error))?;
        match query
            .next()
            .map_err(|error| classify_sqlite_error(&error))?
        {
            Some(row) => Ok(Some(ChatLine {
                text: truncate_chars(&column_string(row, 0), MESSAGE_TEXT_LIMIT),
                session: column_string(row, 1),
                seq: column_i64(row, 2),
                time: column_i64(row, 3),
                who: column_string(row, 4),
            })),
            None => Ok(None),
        }
    }

    /// 按 `seq` 倒序取最近 `limit` 行，正文截断到 2000 字符。
    fn recent(&self, session: &str, limit: usize) -> Result<Vec<ChatLine>, &'static str> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT text, session, seq, time, who FROM lines WHERE session = ?1 ORDER BY seq DESC LIMIT ?2",
            )
            .map_err(|error| classify_sqlite_error(&error))?;
        let mut query = statement
            .query(rusqlite::params![session, limit as i64])
            .map_err(|error| classify_sqlite_error(&error))?;
        let mut lines = Vec::new();
        while let Some(row) = query
            .next()
            .map_err(|error| classify_sqlite_error(&error))?
        {
            lines.push(ChatLine {
                text: truncate_chars(&column_string(row, 0), MESSAGE_TEXT_LIMIT),
                session: column_string(row, 1),
                seq: column_i64(row, 2),
                time: column_i64(row, 3),
                who: column_string(row, 4),
            });
        }
        Ok(lines)
    }
}

fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    text.chars().take(limit).collect()
}

fn preview_of(line: &ChatLine) -> String {
    let combined = if line.who.is_empty() {
        line.text.clone()
    } else {
        format!("{}：{}", line.who, line.text)
    };
    truncate_chars(&combined, PREVIEW_TEXT_LIMIT)
}

fn latest_line_of(
    index: &ChatIndex,
    sessions: &[String],
) -> Result<Option<ChatLine>, &'static str> {
    let mut best: Option<ChatLine> = None;
    for session in sessions {
        if let Some(line) = index.latest(session)? {
            if best
                .as_ref()
                .is_none_or(|current| line.time >= current.time)
            {
                best = Some(line);
            }
        }
    }
    Ok(best)
}

fn count_lines(index: &ChatIndex, sessions: &[String]) -> Result<i64, &'static str> {
    let mut total = 0;
    for session in sessions {
        total += index.count(session)?;
    }
    Ok(total)
}

// ---------------------------------------------------------------------------
// 一次完整读取
// ---------------------------------------------------------------------------

fn build_data(home: &Path, now_ms: i64, offset_seconds: i32) -> CodexBotData {
    let mut data = CodexBotData {
        home: home.to_path_buf(),
        ..CodexBotData::default()
    };

    if !home.is_dir() {
        data.reason = Some(REASON_MISSING_HOME);
        return data;
    }
    let Ok(bytes) = fs::read(home.join(STATE_FILE_NAME)) else {
        data.reason = Some(REASON_MISSING_STATE);
        return data;
    };
    let state = match parse_state_json(&bytes) {
        Ok(state) => state,
        Err(reason) => {
            data.reason = Some(reason);
            return data;
        }
    };

    data.available = true;
    data.revision = state.revision;
    data.main_bot_ids = state.main_bot_ids;
    data.bots = state.bots;
    data.rooms = state.rooms;

    // Bot 自己的会话：DS-Bot `chatOf(botId) = bot.sessionId ?? bot.id`，两个都收。
    let bot_ids: HashSet<String> = data.bots.iter().map(|bot| bot.id.clone()).collect();
    for bot in &data.bots {
        let own = bot.session_id.clone().unwrap_or_else(|| bot.id.clone());
        let mut sessions = vec![bot.id.clone()];
        if own != bot.id {
            sessions.push(own);
        }
        data.bot_details.entry(bot.id.clone()).or_default().sessions = sessions;
    }

    // 用量：文件缺失是稳定的部分缺失；读失败是瞬时的（交给上层兜底成 stale）。
    let usage_path = home.join(USAGE_FILE_NAME);
    if usage_path.is_file() {
        match read_usage_table(&usage_path) {
            Ok(table) => {
                data.usage_available = true;
                data.usage = aggregate_usage(&table.rows, now_ms, offset_seconds);
                let rollups =
                    attribute_usage(&table.rows, &table.owners, &bot_ids, now_ms, offset_seconds);
                for (bot_id, rollup) in rollups {
                    let details = data.bot_details.entry(bot_id).or_default();
                    details.today = rollup.today;
                    details.week = rollup.week;
                    if rollup.model.is_some() {
                        details.model = rollup.model;
                    }
                    if let Some(time) = rollup.last_activity_at {
                        if details
                            .last_activity_at
                            .is_none_or(|current| time > current)
                        {
                            details.last_activity_at = Some(time);
                        }
                    }
                    for session in rollup.sessions {
                        if !details.sessions.contains(&session) {
                            details.sessions.push(session);
                        }
                    }
                }
            }
            Err(reason) => data.usage_reason = Some(reason),
        }
    } else {
        data.usage_reason = Some(REASON_MISSING_DATABASE);
    }

    // 聊天索引：本机真实数据里这个文件就是缺的，所以必须优雅降级。
    let chat_path = home.join(CHAT_INDEX_FILE_NAME);
    if chat_path.is_file() {
        match ChatIndex::open(&chat_path) {
            Ok(index) => match fill_from_chat_index(&mut data, &index) {
                Ok(()) => data.chat_index_available = true,
                Err(reason) => data.chat_index_reason = Some(reason),
            },
            Err(reason) => data.chat_index_reason = Some(reason),
        }
    } else {
        data.chat_index_reason = Some(REASON_MISSING_DATABASE);
    }

    data
}

fn fill_from_chat_index(data: &mut CodexBotData, index: &ChatIndex) -> Result<(), &'static str> {
    let bots: Vec<(String, Vec<String>)> = data
        .bots
        .iter()
        .map(|bot| {
            let sessions = data
                .bot_details
                .get(&bot.id)
                .map(|details| details.sessions.clone())
                .unwrap_or_else(|| vec![bot.id.clone()]);
            (bot.id.clone(), sessions)
        })
        .collect();
    for (bot_id, sessions) in bots {
        let count = count_lines(index, &sessions)?;
        let latest = latest_line_of(index, &sessions)?;
        let details = data.bot_details.entry(bot_id).or_default();
        details.message_count = Some(count);
        if let Some(line) = latest {
            if details
                .last_activity_at
                .is_none_or(|current| line.time > current)
            {
                details.last_activity_at = Some(line.time);
            }
        }
    }

    for room in &data.rooms {
        let mut sessions = vec![room.id.clone()];
        for session in &room.sessions {
            if !sessions.contains(session) {
                sessions.push(session.clone());
            }
        }
        if let Some(line) = latest_line_of(index, &sessions)? {
            let details = data.room_details.entry(room.id.clone()).or_default();
            details.last_message_at = Some(line.time);
            details.preview = Some(preview_of(&line));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 短 TTL 缓存
// ---------------------------------------------------------------------------

struct SnapshotEntry {
    loaded_at: Instant,
    /// state.json 的 (mtime 毫秒, 长度)，用来判断"文件没变就不用重读"。
    signature: Option<(u64, u64)>,
    data: Arc<CodexBotData>,
    last_good: Option<Arc<CodexBotData>>,
}

struct MessageEntry {
    loaded_at: Instant,
    home: PathBuf,
    session: String,
    limit: usize,
    messages: Arc<Vec<ChatLine>>,
}

#[derive(Default)]
struct CacheState {
    snapshots: HashMap<PathBuf, SnapshotEntry>,
    messages: Vec<MessageEntry>,
}

fn cache() -> &'static Mutex<CacheState> {
    static CACHE: OnceLock<Mutex<CacheState>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(CacheState::default()))
}

fn lock_cache() -> MutexGuard<'static, CacheState> {
    cache().lock().unwrap_or_else(|error| error.into_inner())
}

fn state_signature(home: &Path) -> Option<(u64, u64)> {
    let metadata = fs::metadata(home.join(STATE_FILE_NAME)).ok()?;
    let millis = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0);
    Some((millis, metadata.len()))
}

/// 当前时间的 epoch 毫秒。
pub fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

/// 本机时区相对 UTC 的秒偏移。
pub fn local_offset_seconds() -> i32 {
    Local::now().offset().local_minus_utc()
}

/// 读取快照（默认 15 秒 TTL）。
pub fn snapshot(home: &Path) -> Arc<CodexBotData> {
    snapshot_at(home, now_ms(), local_offset_seconds())
}

/// 读取快照，时间由调用方给定（测试用固定 now 与时区）。
pub fn snapshot_at(home: &Path, now_ms: i64, offset_seconds: i32) -> Arc<CodexBotData> {
    snapshot_with_ttl(home, CACHE_TTL, now_ms, offset_seconds)
}

/// 读取快照，TTL 可指定（`Duration::ZERO` = 每次都真读，测试用）。
pub fn snapshot_with_ttl(
    home: &Path,
    ttl: Duration,
    now_ms: i64,
    offset_seconds: i32,
) -> Arc<CodexBotData> {
    let signature = state_signature(home);
    let previous_good = {
        let state = lock_cache();
        state
            .snapshots
            .get(home)
            .and_then(|entry| entry.last_good.clone())
            .or_else(|| {
                state
                    .snapshots
                    .get(home)
                    .filter(|entry| entry.data.available)
                    .map(|entry| entry.data.clone())
            })
    };

    {
        let state = lock_cache();
        if let Some(entry) = state.snapshots.get(home) {
            let fresh_enough = ttl > Duration::ZERO && entry.loaded_at.elapsed() < ttl;
            if fresh_enough && entry.signature == signature {
                return entry.data.clone();
            }
        }
    }

    let fresh = Arc::new(build_data(home, now_ms, offset_seconds));
    let data = rescue_with_previous(fresh, previous_good);

    let mut state = lock_cache();
    let last_good = if data.available {
        Some(data.clone())
    } else {
        state
            .snapshots
            .get(home)
            .and_then(|entry| entry.last_good.clone())
    };
    state.snapshots.insert(
        home.to_path_buf(),
        SnapshotEntry {
            loaded_at: Instant::now(),
            signature,
            data: data.clone(),
            last_good,
        },
    );
    data
}

/// 瞬时失败且有上一次可用快照 → 交回上一次快照并标 `stale`。
/// state.json 层面的失败（缺失/解析失败/结构不认识）不兜底：那表示数据源没建立。
fn rescue_with_previous(
    fresh: Arc<CodexBotData>,
    previous_good: Option<Arc<CodexBotData>>,
) -> Arc<CodexBotData> {
    let transient = [fresh.usage_reason, fresh.chat_index_reason]
        .into_iter()
        .flatten()
        .any(is_transient_reason);
    if !fresh.available || !transient {
        return fresh;
    }
    match previous_good {
        Some(previous) => {
            let mut stale = (*previous).clone();
            stale.stale = true;
            Arc::new(stale)
        }
        None => fresh,
    }
}

// ---------------------------------------------------------------------------
// 消息
// ---------------------------------------------------------------------------

/// 单会话消息读取结果。
#[derive(Debug, Clone, Default)]
pub struct MessagesResult {
    pub available: bool,
    pub reason: Option<&'static str>,
    pub stale: bool,
    pub messages: Vec<ChatLine>,
}

/// 读取一个会话的最近消息（默认 15 秒 TTL）。
pub fn messages(home: &Path, session: &str, limit: usize) -> MessagesResult {
    messages_with_ttl(home, session, limit, CACHE_TTL)
}

/// 读取一个会话的最近消息，TTL 可指定（`Duration::ZERO` = 每次都真读，测试用）。
pub fn messages_with_ttl(
    home: &Path,
    session: &str,
    limit: usize,
    ttl: Duration,
) -> MessagesResult {
    // 数据源整体没建立（缺 home / 缺 state.json / 结构不认识）时，所有路由给同一个未就绪原因；
    // 这一类失败不退回缓存消息：面板只需要一个统一的“未就绪”。
    let source = snapshot_with_ttl(home, ttl, now_ms(), local_offset_seconds());
    if !source.available {
        return MessagesResult {
            available: false,
            reason: source.reason,
            stale: source.stale,
            messages: Vec::new(),
        };
    }

    let previous = {
        let state = lock_cache();
        state
            .messages
            .iter()
            .rev()
            .find(|entry| entry.home == home && entry.session == session && entry.limit == limit)
            .map(|entry| {
                if ttl > Duration::ZERO && entry.loaded_at.elapsed() < ttl {
                    Some(entry.messages.clone())
                } else {
                    None
                }
            })
            .flatten()
    };
    if let Some(messages) = previous {
        return MessagesResult {
            available: true,
            reason: None,
            stale: false,
            messages: (*messages).clone(),
        };
    }

    let fallback = {
        let state = lock_cache();
        state
            .messages
            .iter()
            .rev()
            .find(|entry| entry.home == home && entry.session == session && entry.limit == limit)
            .map(|entry| entry.messages.clone())
    };

    match read_messages(home, session, limit) {
        Ok(messages) => {
            let stored = Arc::new(messages.clone());
            let mut state = lock_cache();
            state.messages.retain(|entry| {
                !(entry.home == home && entry.session == session && entry.limit == limit)
            });
            state.messages.push(MessageEntry {
                loaded_at: Instant::now(),
                home: home.to_path_buf(),
                session: session.to_string(),
                limit,
                messages: stored,
            });
            while state.messages.len() > MESSAGE_CACHE_ENTRIES {
                state.messages.remove(0);
            }
            MessagesResult {
                available: true,
                reason: None,
                stale: false,
                messages,
            }
        }
        Err(reason) => match fallback {
            Some(messages) if reason != REASON_MISSING_DATABASE => MessagesResult {
                available: true,
                reason: Some(reason),
                stale: true,
                messages: (*messages).clone(),
            },
            _ => MessagesResult {
                available: false,
                reason: Some(reason),
                stale: false,
                messages: Vec::new(),
            },
        },
    }
}

fn read_messages(home: &Path, session: &str, limit: usize) -> Result<Vec<ChatLine>, &'static str> {
    if !home.is_dir() {
        return Err(REASON_MISSING_HOME);
    }
    let path = home.join(CHAT_INDEX_FILE_NAME);
    if !path.is_file() {
        return Err(REASON_MISSING_DATABASE);
    }
    ChatIndex::open(&path)?.recent(session, limit)
}

// ---------------------------------------------------------------------------
// 路由
// ---------------------------------------------------------------------------

/// 纯数据的一层路由响应；由 launcher 写回 TCP 流。
#[derive(Debug)]
pub struct RouteResponse {
    pub status: &'static str,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

/// helper 路由入口：`raw_path` 含查询串（例如 `/codex-bot/bots?limit=5`）。
pub fn handle_route(method: &str, raw_path: &str) -> RouteResponse {
    handle_route_at(
        &codex_bot_home(),
        method,
        raw_path,
        now_ms(),
        local_offset_seconds(),
        CACHE_TTL,
    )
}

/// 桥模式入口：Codex 页面受 CSP 限制无法直连 helper，面板改由 CDP 桥请求。
/// 返回体与 `handle_route` 的 HTTP body 完全一致，面板可沿用同一份解析代码。
pub fn bridge_payload(path: &str) -> serde_json::Value {
    let response = handle_route("GET", path);
    serde_json::from_slice(&response.body)
        .unwrap_or_else(|_| serde_json::json!({ "status": "failed", "message": "数据源未就绪" }))
}

/// 路由的纯函数实现：路径解析 → 数据层 → 响应。`time`/`ttl` 由调用方给，便于直接测。
pub fn handle_route_at(
    home: &Path,
    method: &str,
    raw_path: &str,
    now_ms: i64,
    offset_seconds: i32,
    ttl: Duration,
) -> RouteResponse {
    if method == "OPTIONS" {
        return RouteResponse {
            status: "204 No Content",
            content_type: JSON_CONTENT_TYPE,
            body: Vec::new(),
        };
    }

    let (path, query) = match raw_path.split_once('?') {
        Some((path, query)) => (path, query),
        None => (raw_path, ""),
    };
    if !is_codex_bot_path(path) {
        return not_found("未知后端路径");
    }
    let known = matches!(
        path,
        "/codex-bot/summary" | "/codex-bot/bots" | "/codex-bot/rooms" | "/codex-bot/messages"
    );
    if !known {
        return not_found("未知后端路径");
    }
    if method != "GET" {
        return RouteResponse {
            status: "405 Method Not Allowed",
            content_type: JSON_CONTENT_TYPE,
            body: json_body(&serde_json::json!({
                "status": "failed",
                "message": "该接口只支持 GET",
            })),
        };
    }

    let params: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();

    match path {
        "/codex-bot/summary" => summary_response(home, now_ms, offset_seconds, ttl),
        "/codex-bot/bots" => {
            let limit = match parse_limit(&params) {
                Ok(limit) => limit,
                Err(()) => return bad_request("limit 参数非法：需要 1..=100 的整数"),
            };
            bots_response(home, now_ms, offset_seconds, limit, ttl)
        }
        "/codex-bot/rooms" => {
            let limit = match parse_limit(&params) {
                Ok(limit) => limit,
                Err(()) => return bad_request("limit 参数非法：需要 1..=100 的整数"),
            };
            rooms_response(home, now_ms, offset_seconds, limit, ttl)
        }
        "/codex-bot/messages" => {
            let session = match param(&params, "session") {
                Some(session) if !session.trim().is_empty() => session,
                _ => return bad_request("session 参数缺失：需要 ?session=<id>"),
            };
            let limit = match parse_limit(&params) {
                Ok(limit) => limit,
                Err(()) => return bad_request("limit 参数非法：需要 1..=100 的整数"),
            };
            messages_response(home, &session, limit, ttl)
        }
        _ => not_found("未知后端路径"),
    }
}

fn param(params: &[(String, String)], name: &str) -> Option<String> {
    params
        .iter()
        .rev()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
}

/// `limit` 缺省 20；非整数 / 0 / 负数 → 400；大于 100 按 100 截断。
fn parse_limit(params: &[(String, String)]) -> Result<usize, ()> {
    match param(params, "limit") {
        None => Ok(DEFAULT_LIMIT),
        Some(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                return Err(());
            }
            let value: i64 = raw.parse().map_err(|_| ())?;
            if value < 1 {
                return Err(());
            }
            Ok((value as usize).min(MAX_LIMIT))
        }
    }
}

fn json_body<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec())
}

fn json_response<T: Serialize>(status: &'static str, value: &T) -> RouteResponse {
    RouteResponse {
        status,
        content_type: JSON_CONTENT_TYPE,
        body: json_body(value),
    }
}

fn not_found(message: &str) -> RouteResponse {
    json_response(
        "404 Not Found",
        &serde_json::json!({ "status": "failed", "message": message }),
    )
}

fn bad_request(message: &str) -> RouteResponse {
    json_response(
        "400 Bad Request",
        &serde_json::json!({ "status": "failed", "message": message }),
    )
}

/// 未就绪：HTTP 200 + `{available:false, reason, stale}`（绝不 5xx）。
fn not_ready(reason: &'static str, stale: bool) -> RouteResponse {
    json_response(
        "200 OK",
        &serde_json::json!({
            "available": false,
            "reason": reason,
            "stale": stale,
        }),
    )
}

#[derive(Serialize)]
struct SummaryJson<'a> {
    available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<i64>,
    #[serde(rename = "botCount")]
    bot_count: usize,
    #[serde(rename = "roomCount")]
    room_count: usize,
    #[serde(rename = "mainBotId", skip_serializing_if = "Option::is_none")]
    main_bot_id: Option<&'a str>,
    stale: bool,
    /// state.json 可用但聊天索引缺失时，用这个标志把"部分缺失"和"整体不可用"分开。
    #[serde(rename = "chatIndexAvailable")]
    chat_index_available: bool,
    #[serde(rename = "usageAvailable")]
    usage_available: bool,
    usage: UsageWindows,
}

fn summary_response(home: &Path, now_ms: i64, offset_seconds: i32, ttl: Duration) -> RouteResponse {
    let data = snapshot_with_ttl(home, ttl, now_ms, offset_seconds);
    if !data.available {
        return not_ready(data.reason.unwrap_or(REASON_MISSING_STATE), data.stale);
    }
    json_response(
        "200 OK",
        &SummaryJson {
            available: true,
            reason: None,
            revision: data.revision,
            bot_count: data.bots.len(),
            room_count: data.rooms.len(),
            main_bot_id: data.main_bot_ids.first().map(String::as_str),
            stale: data.stale,
            chat_index_available: data.chat_index_available,
            usage_available: data.usage_available,
            usage: data.usage,
        },
    )
}

#[derive(Serialize)]
struct BotJson<'a> {
    id: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a str>,
    #[serde(rename = "currentSessionId", skip_serializing_if = "Option::is_none")]
    current_session_id: Option<&'a str>,
    #[serde(rename = "lastActivityAt", skip_serializing_if = "Option::is_none")]
    last_activity_at: Option<i64>,
    #[serde(rename = "messageCount", skip_serializing_if = "Option::is_none")]
    message_count: Option<i64>,
    /// 近 7 天窗口（与 summary.week 同一口径）。
    usage: TokenTotals,
}

/// 排序：主 Bot 按 mainBotIds 的顺序在前，其余按名字、id 稳定排。
fn ordered_bots<'a>(data: &'a CodexBotData) -> Vec<&'a BotEntry> {
    let mut main: Vec<&BotEntry> = Vec::new();
    let mut rest: Vec<&BotEntry> = Vec::new();
    for id in &data.main_bot_ids {
        if let Some(bot) = data.bots.iter().find(|bot| &bot.id == id) {
            main.push(bot);
        }
    }
    for bot in &data.bots {
        if !main.iter().any(|known| known.id == bot.id) {
            rest.push(bot);
        }
    }
    rest.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });
    main.extend(rest);
    main
}

fn bots_response(
    home: &Path,
    now_ms: i64,
    offset_seconds: i32,
    limit: usize,
    ttl: Duration,
) -> RouteResponse {
    let data = snapshot_with_ttl(home, ttl, now_ms, offset_seconds);
    if !data.available {
        return not_ready(data.reason.unwrap_or(REASON_MISSING_STATE), data.stale);
    }
    let items: Vec<BotJson<'_>> = ordered_bots(&data)
        .into_iter()
        .take(limit)
        .map(|bot| {
            let details = data.bot_details.get(&bot.id);
            BotJson {
                id: &bot.id,
                name: &bot.name,
                model: details.and_then(|details| details.model.as_deref()),
                current_session_id: Some(bot.session_id.as_deref().unwrap_or(bot.id.as_str())),
                last_activity_at: details.and_then(|details| details.last_activity_at),
                message_count: details.and_then(|details| details.message_count),
                usage: details.map(|details| details.week).unwrap_or_default(),
            }
        })
        .collect();
    json_response("200 OK", &items)
}

#[derive(Serialize)]
struct RoomJson<'a> {
    id: &'a str,
    name: &'a str,
    #[serde(rename = "memberCount")]
    member_count: usize,
    #[serde(rename = "lastMessageAt", skip_serializing_if = "Option::is_none")]
    last_message_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview: Option<&'a str>,
}

fn rooms_response(
    home: &Path,
    now_ms: i64,
    offset_seconds: i32,
    limit: usize,
    ttl: Duration,
) -> RouteResponse {
    let data = snapshot_with_ttl(home, ttl, now_ms, offset_seconds);
    if !data.available {
        return not_ready(data.reason.unwrap_or(REASON_MISSING_STATE), data.stale);
    }
    let mut rooms: Vec<&RoomEntry> = data.rooms.iter().collect();
    rooms.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });
    let items: Vec<RoomJson<'_>> = rooms
        .into_iter()
        .take(limit)
        .map(|room| {
            let details = data.room_details.get(&room.id);
            RoomJson {
                id: &room.id,
                name: &room.name,
                member_count: room.members.len(),
                last_message_at: details.and_then(|details| details.last_message_at),
                preview: details.and_then(|details| details.preview.as_deref()),
            }
        })
        .collect();
    json_response("200 OK", &items)
}

#[derive(Serialize)]
struct MessageJson<'a> {
    seq: i64,
    time: i64,
    who: &'a str,
    text: &'a str,
}

fn messages_response(home: &Path, session: &str, limit: usize, ttl: Duration) -> RouteResponse {
    let result = messages_with_ttl(home, session, limit, ttl);
    if !result.available {
        return not_ready(
            result.reason.unwrap_or(REASON_MISSING_DATABASE),
            result.stale,
        );
    }
    // 成功时保持 spec 的形状：一个裸数组。
    let items: Vec<MessageJson<'_>> = result
        .messages
        .iter()
        .map(|line| MessageJson {
            seq: line.seq,
            time: line.time,
            who: &line.who,
            text: &line.text,
        })
        .collect();
    json_response("200 OK", &items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_bot_paths_are_prefixed() {
        assert!(is_codex_bot_path("/codex-bot"));
        assert!(is_codex_bot_path("/codex-bot/summary"));
        assert!(!is_codex_bot_path("/codex-bots/summary"));
        assert!(!is_codex_bot_path("/backend/status"));
    }

    #[test]
    fn home_resolution_set_blank_and_unset() {
        let set = codex_bot_home_from(Some("/tmp/dsh-home"));
        assert!(set.ends_with(Path::new("bot")));
        assert!(set.to_string_lossy().contains("dsh-home"));

        // 只有空白 = 未设置
        let blank = codex_bot_home_from(Some("   "));
        let unset = codex_bot_home_from(None);
        assert_eq!(blank, unset);
        // 回落的是 `~/.dsh`（不是 `~`），再拼 `bot`，与 DS-Bot `dshHomeOf` 一致。
        assert_eq!(blank, home_dir().join(DSH_DIR_NAME).join(BOT_DIR_NAME));

        // ~ 展开与 DS-Bot expandHome 一致
        assert_eq!(expand_home("~"), home_dir());
        assert_eq!(expand_home("~/dsh-x"), home_dir().join("dsh-x"));
        assert_eq!(
            expand_home("~\\dsh-x"),
            home_dir().join("dsh-x"),
            "反斜杠形式也要展开"
        );
    }

    #[test]
    fn truncation_counts_characters() {
        assert_eq!(truncate_chars("abc", 5), "abc");
        let text = "汉字".repeat(1200);
        let cut = truncate_chars(&text, MESSAGE_TEXT_LIMIT);
        assert_eq!(cut.chars().count(), MESSAGE_TEXT_LIMIT);
        let preview = preview_of(&ChatLine {
            text: "x".repeat(200),
            who: "Chief".to_string(),
            ..ChatLine::default()
        });
        assert_eq!(preview.chars().count(), PREVIEW_TEXT_LIMIT);
        assert!(preview.starts_with("Chief："));
    }

    #[test]
    fn local_day_window_follows_offset() {
        // 2026-10-29T19:44:00Z：UTC+8 的本地时间已是 10-30 03:44，本地日比 UTC 日早一天开始。
        let now = 1_793_232_000_000 + 19 * 3_600_000 + 44 * 60_000;
        let utc = start_of_local_day_ms(now, 0);
        let east8 = start_of_local_day_ms(now, 8 * 3600);
        // 本地零点加回偏移后必须正好落在 UTC 日边界上。
        assert_eq!((east8 + 8 * 3_600_000) % DAY_MS, 0);
        // 本地日已跨天：本地零点 = 对应 UTC 零点往后一天再退 8 小时（即前一天 16:00Z）。
        assert_eq!(east8, utc + DAY_MS - 8 * 3_600_000);
        // now 落在这个本地日内，窗口长度是一整天。
        assert!(east8 <= now + 8 * 3_600_000 && now + 8 * 3_600_000 < east8 + DAY_MS);
    }

    #[test]
    fn usage_windows_split_today_and_week() {
        let now = 1_793_224_320_000; // 某个 UTC 零点
        let offset = 0;
        let day_start = start_of_local_day_ms(now, offset);
        let rows = vec![
            UsageRow {
                session: "s".into(),
                time: day_start,
                input: 10,
                output: 1,
                ..UsageRow::default()
            },
            UsageRow {
                session: "s".into(),
                time: day_start - DAY_MS,
                input: 20,
                output: 2,
                ..UsageRow::default()
            },
            UsageRow {
                session: "s".into(),
                time: day_start - 7 * DAY_MS,
                input: 999,
                output: 999,
                ..UsageRow::default()
            },
        ];
        let windows = aggregate_usage(&rows, now, offset);
        assert_eq!(windows.today.total, 11);
        assert_eq!(windows.week.total, 33, "第 8 天以前的不进 week");
    }

    #[test]
    fn limit_parsing_defaults_and_caps() {
        assert_eq!(parse_limit(&[]), Ok(DEFAULT_LIMIT));
        assert_eq!(
            parse_limit(&[("limit".to_string(), "5".to_string())]),
            Ok(5)
        );
        assert_eq!(
            parse_limit(&[("limit".to_string(), "1000".to_string())]),
            Ok(MAX_LIMIT)
        );
        for bad in ["0", "-3", "abc", "", "1.5"] {
            assert_eq!(
                parse_limit(&[("limit".to_string(), bad.to_string())]),
                Err(()),
                "{bad} 应当非法"
            );
        }
    }

    #[test]
    fn sqlite_reasons_map_to_spec_values() {
        let corrupt = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some("database is locked".to_string()),
        );
        assert_eq!(classify_sqlite_error(&corrupt), REASON_BUSY);
        let missing_table = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
            Some("no such table: lines".to_string()),
        );
        assert_eq!(
            classify_sqlite_error(&missing_table),
            REASON_UNSUPPORTED_SCHEMA
        );
    }
}
