//! codex-bot 数据层集成测试。
//!
//! 夹具全部现场生成在临时目录里（写 `state.json` + 用 rusqlite 建两个小 SQLite），
//! 随 `TempDir` 析构删除，不提交任何二进制夹具。
//!
//! 形状照抄真机（`D:\DS-Bot-main` 只读参考源码，本机 `%USERPROFILE%\.dsh\bot` 只读观察结构）：
//! `bots` 是以 Bot 会话 id（`session-<uuid>`）为 key 的 map 且**没有 `workDir` 字段**、
//! `rooms` 可以是空对象、`chat-index.sqlite` 可以整个缺失、`usage.sqlite` 里多一张 `marks` 表。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use claude_codex_pro_core::codex_bot::{
    self, REASON_BUSY, REASON_MISSING_DATABASE, REASON_MISSING_HOME, REASON_MISSING_STATE,
    REASON_PARSE_ERROR, REASON_UNSUPPORTED_SCHEMA,
};
use rusqlite::Connection;
use serde_json::{Map, Value, json};
use tempfile::TempDir;

const BOT_A: &str = "session-bot-a";
const BOT_B: &str = "session-bot-b";
const GROUP_SESSION: &str = "session-group-x";
const ROOM_SESSION: &str = "session-room-1";

const DAY_MS: i64 = 86_400_000;
/// 2026-10-29T00:00:00Z（UTC 的“今天”起点）。
const TODAY_START: i64 = 1_793_232_000_000;
/// 2026-10-29T12:00:00Z：夹具固定 now，时区固定 UTC。
const NOW_MS: i64 = TODAY_START + 12 * 3_600_000;
const UTC: i32 = 0;

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

struct Fixture {
    _dir: TempDir,
    home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("临时目录");
        let home = dir.path().join("bot");
        fs::create_dir_all(&home).expect("创建 $DSH_HOME/bot");
        Self { _dir: dir, home }
    }

    fn home(&self) -> &Path {
        &self.home
    }

    fn state_path(&self) -> PathBuf {
        self.home.join("state.json")
    }

    fn usage_path(&self) -> PathBuf {
        self.home.join("usage.sqlite")
    }

    fn chat_index_path(&self) -> PathBuf {
        self.home.join("chat-index.sqlite")
    }

    fn write_state(&self, state: &Value) {
        let bytes = serde_json::to_vec_pretty(state).expect("state JSON");
        fs::write(self.state_path(), bytes).expect("写 state.json");
    }

    fn snapshot(&self) -> std::sync::Arc<codex_bot::CodexBotData> {
        codex_bot::snapshot_with_ttl(self.home(), Duration::ZERO, NOW_MS, UTC)
    }
}

/// 真机字段形状的 Bot 值：刻意不给 `workDir`（真机 state.json 没有这个字段）。
fn bot_value(id: &str, name: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "role": "Chief of Staff",
        "color": "deepseek",
        "avatar": { "shape": "whale" },
        "instructions": "",
        "createdAt": 1_793_194_469_605i64,
        "createdBy": null,
        "capabilities": []
    })
}

fn room_value(id: &str, name: &str, members: &[&str], sessions: &[(&str, &str)]) -> Value {
    let mut map = Map::new();
    for (bot_id, session_id) in sessions {
        map.insert((*bot_id).to_string(), json!(session_id));
    }
    json!({
        "id": id,
        "name": name,
        "members": members,
        "sessions": Value::Object(map)
    })
}

/// 真机 state.json 的关键字段；其余无关字段（agents/capabilities/...）留着不影响解析。
fn state_json(bots: &[Value], rooms: &[Value], main_bot_ids: &[&str], revision: i64) -> Value {
    let mut bot_map = Map::new();
    for bot in bots {
        let id = bot["id"].as_str().expect("bot id").to_string();
        bot_map.insert(id, bot.clone());
    }
    let mut room_map = Map::new();
    for room in rooms {
        let id = room["id"].as_str().expect("room id").to_string();
        room_map.insert(id, room.clone());
    }
    json!({
        "version": 1,
        "mainBotIds": main_bot_ids,
        "bots": Value::Object(bot_map),
        "rooms": Value::Object(room_map),
        "agents": {},
        "capabilities": { "items": {} },
        "exchanges": [],
        "prefs": {},
        "revision": revision,
        "whaleStored": true,
        "workspaceId": "c2268cce-8163-43b2-baf4-9a1ec79678bb"
    })
}

/// DS-Bot `usage.js` 的原样表结构（多出的 `marks` 表证明查询只挑需要的表）。
const USAGE_SCHEMA: &str = "
CREATE TABLE calls (key TEXT PRIMARY KEY, session TEXT NOT NULL, time INTEGER NOT NULL, provider TEXT NOT NULL, model TEXT NOT NULL, kind TEXT NOT NULL, live INTEGER NOT NULL, input INTEGER NOT NULL, cache_read INTEGER NOT NULL, cache_write INTEGER NOT NULL, output INTEGER NOT NULL);
CREATE INDEX calls_session ON calls (session);
CREATE TABLE marks (session TEXT PRIMARY KEY, through INTEGER NOT NULL, fold TEXT NOT NULL, sealed INTEGER NOT NULL);
CREATE TABLE owners (session TEXT PRIMARY KEY, owner TEXT NOT NULL, name TEXT NOT NULL);
CREATE TABLE kinds (session TEXT PRIMARY KEY, kind TEXT NOT NULL);";

/// DS-Bot `chat-index.js` 的原样表结构（FTS5 + trigram，与真机一致）。
const CHAT_INDEX_SCHEMA: &str = "
CREATE VIRTUAL TABLE lines USING fts5(text, session UNINDEXED, seq UNINDEXED, time UNINDEXED, who UNINDEXED, tokenize = 'trigram');
CREATE TABLE marks (session TEXT PRIMARY KEY, through INTEGER NOT NULL, keep INTEGER NOT NULL, sealed INTEGER NOT NULL, turn INTEGER NOT NULL);";

#[derive(Clone)]
struct CallRow {
    key: String,
    session: &'static str,
    time: i64,
    model: &'static str,
    input: i64,
    output: i64,
}

fn call(
    key: &str,
    session: &'static str,
    time: i64,
    model: &'static str,
    input: i64,
    output: i64,
) -> CallRow {
    CallRow {
        key: key.to_string(),
        session,
        time,
        model,
        input,
        output,
    }
}

fn create_usage_db(path: &Path, calls: &[CallRow], owners: &[(&str, &str, &str)]) {
    let conn = Connection::open(path).expect("建 usage.sqlite");
    conn.execute_batch(USAGE_SCHEMA).expect("usage 表结构");
    for row in calls {
        conn.execute(
            "INSERT INTO calls (key, session, time, provider, model, kind, live, input, cache_read, cache_write, output) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, 0, 0, ?8)",
            rusqlite::params![row.key, row.session, row.time, "openai", row.model, "chat", row.input, row.output],
        )
        .expect("插入 call");
    }
    for (session, owner, name) in owners {
        conn.execute(
            "INSERT INTO owners (session, owner, name) VALUES (?1, ?2, ?3)",
            rusqlite::params![session, owner, name],
        )
        .expect("插入 owner");
    }
}

struct LineRow {
    session: &'static str,
    seq: i64,
    time: i64,
    who: &'static str,
    text: String,
}

fn line(session: &'static str, seq: i64, time: i64, who: &'static str, text: &str) -> LineRow {
    LineRow {
        session,
        seq,
        time,
        who,
        text: text.to_string(),
    }
}

fn create_chat_index_db(path: &Path, lines: &[LineRow]) {
    let conn = Connection::open(path).expect("建 chat-index.sqlite");
    conn.execute_batch(CHAT_INDEX_SCHEMA)
        .expect("chat-index 表结构");
    for row in lines {
        conn.execute(
            "INSERT INTO lines (text, session, seq, time, who) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![row.text, row.session, row.seq, row.time, row.who],
        )
        .expect("插入 lines");
    }
}

// ---------------------------------------------------------------------------
// 断言助手
// ---------------------------------------------------------------------------

/// 从路径解析请求 → 生成响应，直接测纯函数路由（不起服务）。
fn get(fixture: &Fixture, raw_path: &str) -> (String, Value) {
    let response =
        codex_bot::handle_route_at(fixture.home(), "GET", raw_path, NOW_MS, UTC, Duration::ZERO);
    let value = if response.body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&response.body).expect("响应体是 JSON")
    };
    (response.status.to_string(), value)
}

fn assert_status(status: &str, expected: &str) {
    assert!(
        status.starts_with(expected),
        "期望 {expected}，实际 {status}"
    );
}

fn assert_not_5xx(status: &str) {
    assert!(!status.starts_with('5'), "不得返回 5xx，实际 {status}");
}

fn totals(value: &Value) -> (i64, i64, i64) {
    (
        value["input"].as_i64().expect("input"),
        value["output"].as_i64().expect("output"),
        value["total"].as_i64().expect("total"),
    )
}

fn window_totals(window: codex_bot::TokenTotals) -> (i64, i64, i64) {
    (window.input, window.output, window.total)
}

// ---------------------------------------------------------------------------
// 验收 1 / 4 / 5：数据层可用、未就绪、DSH_HOME 解析
// ---------------------------------------------------------------------------

#[test]
fn summary_reports_fixture_counts_and_partial_sources() {
    let fixture = Fixture::new();
    write_default_state(&fixture);

    let (status, value) = get(&fixture, "/codex-bot/summary");
    assert_status(&status, "200");
    assert_eq!(value["available"], json!(true));
    assert_eq!(value["revision"], json!(3));
    assert_eq!(value["botCount"], json!(2));
    assert_eq!(value["roomCount"], json!(0));
    assert_eq!(value["mainBotId"], json!(BOT_A));
    assert_eq!(value["stale"], json!(false));
    assert!(
        value.get("reason").is_none(),
        "可用时不应带 reason：{value}"
    );
    // 真机形状：chat-index.sqlite 缺失、usage.sqlite 缺失都属于“某一类数据缺失”，
    // 与整个数据源不可用（available:false）区分开。
    assert_eq!(value["chatIndexAvailable"], json!(false));
    assert_eq!(value["usageAvailable"], json!(false));
    assert_eq!(totals(&value["usage"]["today"]), (0, 0, 0));
    assert_eq!(totals(&value["usage"]["week"]), (0, 0, 0));

    // 数据层直接调用同样可用。
    let snapshot = fixture.snapshot();
    assert!(snapshot.available);
    assert_eq!(snapshot.revision, Some(3));
    assert_eq!(snapshot.bots.len(), 2);
    assert_eq!(snapshot.rooms.len(), 0);
    assert_eq!(snapshot.main_bot_ids, vec![BOT_A.to_string()]);
    assert_eq!(snapshot.chat_index_reason, Some(REASON_MISSING_DATABASE));
    assert_eq!(snapshot.usage_reason, Some(REASON_MISSING_DATABASE));
    assert!(
        !snapshot.stale,
        "稳定的“缺文件”不是 transient，不该标 stale"
    );
    // 真机没有 workDir 字段：解析成 None 而不是猜测。
    assert!(snapshot.bots.iter().all(|bot| bot.work_dir.is_none()));
}

/// 两个 Bot、空 rooms：真机形状（无 workDir、rooms 为 {}）。
fn write_default_state(fixture: &Fixture) {
    fixture.write_state(&state_json(
        &[bot_value(BOT_A, "Chief"), bot_value(BOT_B, "Writer")],
        &[],
        &[BOT_A],
        3,
    ));
}

#[test]
fn missing_state_reports_missing_state() {
    let fixture = Fixture::new();

    let snapshot = fixture.snapshot();
    assert!(!snapshot.available);
    assert_eq!(snapshot.reason, Some(REASON_MISSING_STATE));

    let (status, value) = get(&fixture, "/codex-bot/summary");
    assert_not_5xx(&status);
    assert_status(&status, "200");
    assert_eq!(value["available"], json!(false));
    assert_eq!(value["reason"], json!("missing-state"));
    assert_eq!(value["stale"], json!(false));

    // 每个路由都必须是 200 + available:false，不能 5xx。
    for path in [
        "/codex-bot/bots",
        "/codex-bot/rooms",
        "/codex-bot/messages?session=session-bot-a",
    ] {
        let (status, value) = get(&fixture, path);
        assert_not_5xx(&status);
        assert_status(&status, "200");
        assert_eq!(value["available"], json!(false), "{path}");
        assert_eq!(value["reason"], json!("missing-state"), "{path}");
    }
}

#[test]
fn missing_home_reports_missing_home() {
    let fixture = Fixture::new();
    let absent = fixture.home().join("absent");
    assert!(!absent.exists());

    let snapshot = codex_bot::snapshot_with_ttl(&absent, Duration::ZERO, NOW_MS, UTC);
    assert!(!snapshot.available);
    assert_eq!(snapshot.reason, Some(REASON_MISSING_HOME));

    let response = codex_bot::handle_route_at(
        &absent,
        "GET",
        "/codex-bot/summary",
        NOW_MS,
        UTC,
        Duration::ZERO,
    );
    assert_not_5xx(response.status);
    let value: Value = serde_json::from_slice(&response.body).expect("JSON");
    assert_eq!(value["available"], json!(false));
    assert_eq!(value["reason"], json!("missing-home"));
}

#[test]
fn state_parse_failures_report_their_reason() {
    let fixture = Fixture::new();

    fs::write(fixture.state_path(), b"{ not json").expect("写坏 state.json");
    assert_eq!(fixture.snapshot().reason, Some(REASON_PARSE_ERROR));

    fs::write(fixture.state_path(), br#"{"revision":1}"#).expect("写缺 bots 的 state.json");
    assert_eq!(
        fixture.snapshot().reason,
        Some(REASON_UNSUPPORTED_SCHEMA),
        "缺 bots 字段属于不认识的结构，不做字段猜测"
    );

    // 空 bots / 空 rooms 是合法状态（真机 rooms 就是空的）。
    let empty = state_json(&[], &[], &[], 1);
    let parsed = codex_bot::parse_state_json(serde_json::to_vec(&empty).unwrap().as_slice())
        .expect("空 bots 可解析");
    assert_eq!(parsed.bots.len(), 0);
    fixture.write_state(&empty);
    let snapshot = fixture.snapshot();
    assert!(snapshot.available);
    assert_eq!(snapshot.bots.len(), 0);
}

#[test]
fn dsh_home_resolution_covers_set_blank_and_unset() {
    let set = codex_bot::codex_bot_home_from(Some("  /tmp/ccp-dsh  "));
    assert!(
        set.ends_with(Path::new("ccp-dsh").join("bot")),
        "$DSH_HOME 已设置时用它：{set:?}"
    );

    let blank = codex_bot::codex_bot_home_from(Some("   "));
    let unset = codex_bot::codex_bot_home_from(None);
    assert_eq!(blank, unset, "空白与未设置等价");
    assert!(
        blank.ends_with(Path::new(".dsh").join("bot")),
        "回退到 ~/.dsh/bot：{blank:?}"
    );

    // 与 DS-Bot folders.js 的 expandHome 一致。
    assert_eq!(codex_bot::expand_home("~"), codex_bot::expand_home("~"));
    assert!(codex_bot::expand_home("~").is_absolute());
    assert!(codex_bot::expand_home("~/dsh-x").ends_with(Path::new("dsh-x")));
    assert!(codex_bot::expand_home("~\\dsh-x").ends_with(Path::new("dsh-x")));
}

// ---------------------------------------------------------------------------
// 验收 2：只读
// ---------------------------------------------------------------------------

#[test]
fn fixture_databases_are_opened_read_only() {
    let fixture = Fixture::new();
    create_usage_db(&fixture.usage_path(), &[], &[]);
    create_chat_index_db(&fixture.chat_index_path(), &[]);

    for path in [fixture.usage_path(), fixture.chat_index_path()] {
        let conn = codex_bot::open_read_only(&path).expect("只读打开");
        let query_only: i64 = conn
            .query_row("PRAGMA query_only", [], |row| row.get(0))
            .expect("PRAGMA query_only");
        assert_eq!(query_only, 1, "{path:?}");
        let write = conn.execute_batch("CREATE TABLE probe (id INTEGER)");
        assert!(write.is_err(), "只读连接上的写必须失败：{path:?}");
        drop(conn);

        // 夹具本身没有被改动。
        let check = Connection::open(&path).expect("重新可写打开");
        let count: i64 = check
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'probe'",
                [],
                |row| row.get(0),
            )
            .expect("查 sqlite_master");
        assert_eq!(count, 0, "写尝试不得改动夹具：{path:?}");
    }
}

// ---------------------------------------------------------------------------
// 验收 3：SQLITE_BUSY → 上一次快照 + stale:true
// ---------------------------------------------------------------------------

#[test]
fn busy_usage_database_serves_previous_snapshot_as_stale() {
    let fixture = Fixture::new();
    fixture.write_state(&state_json(&[bot_value(BOT_A, "Chief")], &[], &[BOT_A], 1));
    create_usage_db(
        &fixture.usage_path(),
        &usage_fixture_calls(),
        &usage_fixture_owners(),
    );

    // 夹具是 rusqlite 默认的回滚日志模式（非 WAL），`BEGIN EXCLUSIVE` 会挡住读连接。
    let blocker = Connection::open(fixture.usage_path()).expect("占位连接");
    blocker
        .execute_batch("BEGIN EXCLUSIVE")
        .expect("持有排它锁");

    // 还没有上一份快照时：如实报 busy，不 panic、不 5xx。
    let first_busy = fixture.snapshot();
    assert!(first_busy.available, "state.json 仍然可用，不能整体判死");
    assert_eq!(first_busy.usage_reason, Some(REASON_BUSY));
    assert!(!first_busy.stale);
    assert_eq!(window_totals(first_busy.usage.week), (0, 0, 0));

    blocker.execute_batch("ROLLBACK").expect("释放锁");

    let first = fixture.snapshot();
    assert!(first.available && first.usage_available);
    assert!(!first.stale);
    assert_eq!(first.usage_reason, None);
    let first_week = window_totals(first.usage.week);
    assert_eq!(first_week, (167, 98, 265));

    blocker
        .execute_batch("BEGIN EXCLUSIVE")
        .expect("再次持有排它锁");

    let busy = fixture.snapshot();
    assert!(busy.available, "state.json 仍然可用，不能整体判死");
    assert!(busy.usage_available);
    assert!(busy.stale, "SQLITE_BUSY 必须退回上一次快照并标 stale");
    assert_eq!(
        busy.usage_reason, None,
        "退回的就是上一次快照本身，它自己没有 reason"
    );
    assert_eq!(
        window_totals(busy.usage.week),
        first_week,
        "退回的必须是上一次的值"
    );

    let (status, value) = get(&fixture, "/codex-bot/summary");
    assert_not_5xx(&status);
    assert_status(&status, "200");
    assert_eq!(value["stale"], json!(true));
    assert_eq!(totals(&value["usage"]["week"]), first_week);

    blocker.execute_batch("ROLLBACK").expect("释放锁");
    drop(blocker);

    let recovered = fixture.snapshot();
    assert!(recovered.usage_available);
    assert!(!recovered.stale, "锁释放后必须恢复成新快照");
    assert_eq!(recovered.usage_reason, None);
}

// ---------------------------------------------------------------------------
// 验收 6：用量归属与 today / week 合计
// ---------------------------------------------------------------------------

fn usage_fixture_calls() -> Vec<CallRow> {
    vec![
        // 今天，Bot A（主 Bot 自己的会话）
        call(
            "call-1",
            BOT_A,
            TODAY_START + 3 * 3_600_000,
            "gpt-5.5",
            100,
            40,
        ),
        // 昨天，Bot A：只进 week
        call(
            "call-2",
            BOT_A,
            TODAY_START - DAY_MS + 10 * 3_600_000,
            "gpt-5.5",
            10,
            5,
        ),
        // 今天，经 owners 归属到 Bot B（群里某个会话）
        call(
            "call-3",
            GROUP_SESSION,
            TODAY_START + 5 * 3_600_000,
            "gpt-5.5-mini",
            7,
            3,
        ),
        // 今天，房间自己的会话：算进全局合计，但不归属任何 Bot
        call(
            "call-4",
            ROOM_SESSION,
            TODAY_START + 3_600_000,
            "gpt-5.5",
            50,
            50,
        ),
        // 8 天前：today / week 都不算
        call(
            "call-5",
            "session-old",
            TODAY_START - 8 * DAY_MS,
            "gpt-5.5",
            999,
            999,
        ),
    ]
}

fn usage_fixture_owners() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (BOT_A, BOT_A, "Chief"),
        (GROUP_SESSION, BOT_B, "Writer"),
        (ROOM_SESSION, "room:room-1", "Ops"),
    ]
}

#[test]
fn usage_windows_and_attribution_match_hand_computed_fixture() {
    let fixture = Fixture::new();
    fixture.write_state(&state_json(
        &[bot_value(BOT_A, "Chief"), bot_value(BOT_B, "Writer")],
        &[],
        &[BOT_A],
        7,
    ));
    create_usage_db(
        &fixture.usage_path(),
        &usage_fixture_calls(),
        &usage_fixture_owners(),
    );

    let snapshot = fixture.snapshot();
    assert!(snapshot.usage_available);
    // 手算：today = (100+40) + (7+3) + (50+50) = 250；week 再加昨天的 (10+5) = 265。
    assert_eq!(window_totals(snapshot.usage.today), (157, 93, 250));
    assert_eq!(window_totals(snapshot.usage.week), (167, 98, 265));

    let (status, value) = get(&fixture, "/codex-bot/summary");
    assert_status(&status, "200");
    assert_eq!(value["usageAvailable"], json!(true));
    assert_eq!(totals(&value["usage"]["today"]), (157, 93, 250));
    assert_eq!(totals(&value["usage"]["week"]), (167, 98, 265));

    let (status, value) = get(&fixture, "/codex-bot/bots");
    assert_status(&status, "200");
    let bots = value.as_array().expect("bots 是数组");
    assert_eq!(bots.len(), 2);
    // 主 Bot 排最前；usage 是近 7 天窗口。
    assert_eq!(bots[0]["id"], json!(BOT_A));
    assert_eq!(totals(&bots[0]["usage"]), (110, 45, 155));
    assert_eq!(bots[0]["model"], json!("gpt-5.5"));
    assert_eq!(bots[1]["id"], json!(BOT_B));
    assert_eq!(totals(&bots[1]["usage"]), (7, 3, 10), "owners 映射到 Bot B");
    assert_eq!(bots[1]["model"], json!("gpt-5.5-mini"));
    // 房间自己的会话（owner = room:room-1）不归属任何 Bot。
    let attributed: i64 = bots
        .iter()
        .map(|bot| bot["usage"]["total"].as_i64().unwrap())
        .sum();
    assert_eq!(attributed, 165, "房间用量不计入 Bot 排行");
}

// ---------------------------------------------------------------------------
// 验收 7 / 5：消息倒序、截断、非法 limit
// ---------------------------------------------------------------------------

#[test]
fn messages_are_newest_first_and_truncated_to_2000_chars() {
    let fixture = Fixture::new();
    fixture.write_state(&state_json(&[bot_value(BOT_A, "Chief")], &[], &[BOT_A], 1));
    let long_text = "长".repeat(2_500);
    create_chat_index_db(
        &fixture.chat_index_path(),
        &[
            line(BOT_A, 1, NOW_MS - 5_000, "you", "第一条"),
            line(BOT_A, 2, NOW_MS - 4_000, "Chief", "第二条"),
            line(BOT_A, 3, NOW_MS - 3_000, "you", &long_text),
            line(BOT_A, 4, NOW_MS - 2_000, "Chief", "第四条"),
            line("session-other", 9, NOW_MS - 1_000, "you", "别的会话"),
        ],
    );

    let (status, value) = get(
        &fixture,
        "/codex-bot/messages?session=session-bot-a&limit=3",
    );
    assert_status(&status, "200");
    let items = value.as_array().expect("messages 是数组").clone();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["seq"], json!(4));
    assert_eq!(items[1]["seq"], json!(3));
    assert_eq!(items[2]["seq"], json!(2));
    assert_eq!(items[0]["who"], json!("Chief"));
    assert_eq!(items[0]["time"], json!(NOW_MS - 2_000));
    assert_eq!(
        items[1]["text"].as_str().expect("text").chars().count(),
        2_000,
        "text 截断到 2000 字符"
    );
    assert!(
        !value.to_string().contains("别的会话"),
        "session 过滤必须生效"
    );

    // 缺省 limit = 20：该会话 4 条全部返回。
    let (status, value) = get(&fixture, "/codex-bot/messages?session=session-bot-a");
    assert_status(&status, "200");
    assert_eq!(value.as_array().expect("数组").len(), 4);
}

#[test]
fn invalid_params_are_400() {
    let fixture = Fixture::new();
    write_default_state(&fixture);

    for path in [
        "/codex-bot/messages?session=session-bot-a&limit=0",
        "/codex-bot/messages?session=session-bot-a&limit=-3",
        "/codex-bot/messages?session=session-bot-a&limit=abc",
        "/codex-bot/messages?session=session-bot-a&limit=",
        "/codex-bot/bots?limit=0",
        "/codex-bot/bots?limit=abc",
        "/codex-bot/rooms?limit=-1",
        "/codex-bot/messages?limit=5",
        "/codex-bot/messages?session=",
    ] {
        let (status, _) = get(&fixture, path);
        assert_status(&status, "400");
    }

    // 超上限按上限截断，不是 400。
    let (status, value) = get(&fixture, "/codex-bot/bots?limit=1000");
    assert_status(&status, "200");
    assert_eq!(value.as_array().expect("数组").len(), 2);
}

#[test]
fn messages_without_chat_index_is_not_ready_but_not_5xx() {
    let fixture = Fixture::new();
    fixture.write_state(&state_json(&[bot_value(BOT_A, "Chief")], &[], &[BOT_A], 1));
    assert!(!fixture.chat_index_path().exists());

    let result = codex_bot::messages_with_ttl(fixture.home(), BOT_A, 20, Duration::ZERO);
    assert!(!result.available);
    assert_eq!(result.reason, Some(REASON_MISSING_DATABASE));
    assert!(result.messages.is_empty());

    let (status, value) = get(&fixture, "/codex-bot/messages?session=session-bot-a");
    assert_not_5xx(&status);
    assert_status(&status, "200");
    assert_eq!(value["available"], json!(false));
    assert_eq!(value["reason"], json!("missing-database"));

    // 同一个数据源下 /summary 仍是 available:true（只是这一类数据缺失）。
    let (status, value) = get(&fixture, "/codex-bot/summary");
    assert_status(&status, "200");
    assert_eq!(value["available"], json!(true));
    assert_eq!(value["chatIndexAvailable"], json!(false));
}

// ---------------------------------------------------------------------------
// rooms / bots 路由的形状
// ---------------------------------------------------------------------------

#[test]
fn rooms_report_members_and_truncated_preview() {
    let fixture = Fixture::new();
    fixture.write_state(&state_json(
        &[bot_value(BOT_A, "Chief"), bot_value(BOT_B, "Writer")],
        &[
            room_value("room-1", "Alpha", &[BOT_A, BOT_B], &[(BOT_A, ROOM_SESSION)]),
            room_value("room-2", "Beta", &[BOT_B], &[(BOT_B, "session-room-2")]),
        ],
        &[BOT_A],
        2,
    ));
    let long_preview = "很长的预览".repeat(40); // 200 字符
    create_chat_index_db(
        &fixture.chat_index_path(),
        &[
            line(ROOM_SESSION, 1, NOW_MS - 9_000, "Chief", "今晚发布"),
            line("session-room-2", 1, NOW_MS - 8_000, "Writer", &long_preview),
        ],
    );

    let (status, value) = get(&fixture, "/codex-bot/rooms");
    assert_status(&status, "200");
    let rooms = value.as_array().expect("rooms 是数组");
    assert_eq!(rooms.len(), 2);
    assert_eq!(rooms[0]["id"], json!("room-1"));
    assert_eq!(rooms[0]["name"], json!("Alpha"));
    assert_eq!(rooms[0]["memberCount"], json!(2));
    assert_eq!(rooms[0]["lastMessageAt"], json!(NOW_MS - 9_000));
    assert_eq!(rooms[0]["preview"], json!("Chief：今晚发布"));

    assert_eq!(rooms[1]["memberCount"], json!(1));
    let preview = rooms[1]["preview"].as_str().expect("preview");
    assert_eq!(preview.chars().count(), 60, "preview 截断到 60 字符");
    assert!(preview.starts_with("Writer："));
}

#[test]
fn bots_without_chat_index_omit_message_fields_and_keep_main_first() {
    let fixture = Fixture::new();
    fixture.write_state(&state_json(
        &[
            bot_value(BOT_A, "Chief"),
            bot_value(BOT_B, "Writer"),
            bot_value("session-bot-c", "Analyst"),
        ],
        &[],
        &[BOT_A],
        5,
    ));

    let (status, value) = get(&fixture, "/codex-bot/bots?limit=2");
    assert_status(&status, "200");
    let bots = value.as_array().expect("数组");
    assert_eq!(bots.len(), 2);
    assert_eq!(bots[0]["id"], json!(BOT_A), "主 Bot 排最前");
    assert_eq!(bots[1]["name"], json!("Analyst"), "其余按名字排");
    // 缺 chat-index / usage 时消息与用量字段要么缺省要么为 0，绝不 panic。
    assert_eq!(bots[0]["currentSessionId"], json!(BOT_A));
    assert!(bots[0].get("messageCount").is_none());
    assert!(bots[0].get("lastActivityAt").is_none());
    assert_eq!(totals(&bots[0]["usage"]), (0, 0, 0));
}

#[test]
fn unknown_path_is_404_and_non_get_is_405() {
    let fixture = Fixture::new();
    write_default_state(&fixture);

    let (status, _) = get(&fixture, "/codex-bot/nope");
    assert_status(&status, "404");

    let response = codex_bot::handle_route_at(
        fixture.home(),
        "POST",
        "/codex-bot/summary",
        NOW_MS,
        UTC,
        Duration::ZERO,
    );
    assert_status(response.status, "405");

    let options = codex_bot::handle_route_at(
        fixture.home(),
        "OPTIONS",
        "/codex-bot/summary",
        NOW_MS,
        UTC,
        Duration::ZERO,
    );
    assert_status(options.status, "204");

    assert!(codex_bot::is_codex_bot_path("/codex-bot/summary"));
    assert!(codex_bot::is_codex_bot_path("/codex-bot"));
    assert!(!codex_bot::is_codex_bot_path("/backend/status"));
    assert!(!codex_bot::is_codex_bot_path("/codex-bot-other"));
}

// ---------------------------------------------------------------------------
// 真机只读冒烟（默认 #[ignore]，需要 `CCP_CODEX_BOT_REAL=1` 显式开启）
// ---------------------------------------------------------------------------

/// 对本机真实 `$DSH_HOME/bot` 走一遍只读读取：不起服务、不写任何文件、只打印汇总。
/// 只在显式 opt-in 时运行（`cargo test -p claude-codex-pro-core --test codex_bot -- --ignored --nocapture`），
/// 因此换一台没有 DS-Bot 的机器也不会失败。
#[test]
#[ignore = "读取本机真实 DSH 数据，需 CCP_CODEX_BOT_REAL=1 显式开启"]
fn real_machine_snapshot_is_read_only() {
    if std::env::var("CCP_CODEX_BOT_REAL").ok().as_deref() != Some("1") {
        println!("跳过：未设置 CCP_CODEX_BOT_REAL=1");
        return;
    }
    let home = codex_bot::codex_bot_home();
    println!("codex_bot_home = {}", home.display());
    if !home.is_dir() {
        println!("跳过：本机没有 {}", home.display());
        return;
    }

    let started = std::time::Instant::now();
    let data = codex_bot::snapshot(&home);
    println!(
        "available={} reason={:?} revision={:?} bots={} rooms={} chatIndex={} ({:?}) usage={} ({:?}) stale={} today={:?} week={:?} in {:?}",
        data.available,
        data.reason,
        data.revision,
        data.bots.len(),
        data.rooms.len(),
        data.chat_index_available,
        data.chat_index_reason,
        data.usage_available,
        data.usage_reason,
        data.stale,
        window_totals(data.usage.today),
        window_totals(data.usage.week),
        started.elapsed()
    );
    assert!(
        data.available,
        "本机 state.json 存在时数据层必须可用：{:?}",
        data.reason
    );

    // 第二次读命中 15s TTL 缓存，比第一次快；只证明缓存生效，不设死阈值。
    let second = std::time::Instant::now();
    let again = codex_bot::snapshot(&home);
    assert_eq!(again.revision, data.revision);
    println!("cached read in {:?}", second.elapsed());

    // 真机数据一律只读：这两条 SQLite 若存在，也必须能只读打开。
    for name in ["chat-index.sqlite", "usage.sqlite"] {
        let path = home.join(name);
        if !path.is_file() {
            println!("{name} 不存在（真机现状）");
            continue;
        }
        let conn = codex_bot::open_read_only(&path).expect("只读打开真机 sqlite");
        let query_only: i64 = conn
            .query_row("PRAGMA query_only", [], |row| row.get(0))
            .expect("PRAGMA query_only");
        assert_eq!(query_only, 1, "{name} 必须只读");
        println!("{name} 只读可打开，query_only=1");
    }
}
