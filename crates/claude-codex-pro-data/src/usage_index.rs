//! SQLite-backed normalized session index for the 会话 page and workbench.
//!
//! The local-usage scan stays the single source of truth. After every scan the
//! command layer calls [`write_usage`] / [`write_workbench`], which replace the
//! whole corresponding table inside one `BEGIN IMMEDIATE` transaction, so
//! readers see either the previous complete snapshot or the new one. The page
//! asks [`index_ready`] first and only uses the index when a complete snapshot
//! has been committed; otherwise it keeps reading the JSON usage snapshot.

use crate::aitracker::{AgentSessionSummary, AgentTokenTotals, AitrackerSnapshot};
use crate::local_usage::LocalUsageSnapshot;
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, TransactionBehavior, params, params_from_iter};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration as StdDuration;

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

const SCHEMA_VERSION: i64 = 1;
const DB_FILE_NAME: &str = "usage-index-v1.db";
const CONNECTION_BUSY_TIMEOUT: StdDuration = StdDuration::from_millis(5_000);

const META_SESSIONS_WRITTEN_AT: &str = "sessions_written_at_ms";
const META_WORKBENCH_WRITTEN_AT: &str = "workbench_written_at_ms";
const META_SNAPSHOT_GENERATED_AT: &str = "snapshot_generated_at";
const META_SNAPSHOT_MODE: &str = "snapshot_mode";

/// A workbench row as the distillation page renders it. Stored separately from
/// the AITRACKER session projection because the workbench merges continuation
/// chains, limits the list to the newest rows, and adds title/segment data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchRow {
    pub agent: String,
    pub session_id: String,
    pub title: String,
    pub project: String,
    pub project_key: String,
    pub is_git_project: bool,
    pub model: String,
    pub started_at: String,
    pub updated_at: String,
    pub events: usize,
    pub turns: usize,
    pub status: String,
    pub tokens: u64,
    pub segments: usize,
}

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS sessions(
  source TEXT NOT NULL, session_id TEXT NOT NULL,
  title TEXT NOT NULL DEFAULT '', project TEXT NOT NULL DEFAULT '',
  model TEXT NOT NULL DEFAULT '', provider TEXT NOT NULL DEFAULT '',
  started_at TEXT NOT NULL DEFAULT '', ended_at TEXT NOT NULL DEFAULT '',
  started_at_ms INTEGER NOT NULL DEFAULT 0, ended_at_ms INTEGER NOT NULL DEFAULT 0,
  events INTEGER NOT NULL DEFAULT 0, tool_calls INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT '', total_tokens INTEGER NOT NULL DEFAULT 0,
  input_tokens INTEGER NOT NULL DEFAULT 0, cached_input_tokens INTEGER NOT NULL DEFAULT 0,
  cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0, output_tokens INTEGER NOT NULL DEFAULT 0,
  reasoning_output_tokens INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(source, session_id)
);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_source ON sessions(source, started_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project);
CREATE INDEX IF NOT EXISTS idx_sessions_tokens ON sessions(total_tokens DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE TABLE IF NOT EXISTS workbench(
  agent TEXT NOT NULL, session_id TEXT NOT NULL, title TEXT NOT NULL DEFAULT '',
  project TEXT NOT NULL DEFAULT '', project_key TEXT NOT NULL DEFAULT '',
  is_git_project INTEGER NOT NULL DEFAULT 0, model TEXT NOT NULL DEFAULT '',
  started_at TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL DEFAULT '',
  events INTEGER NOT NULL DEFAULT 0, turns INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT '', tokens INTEGER NOT NULL DEFAULT 0,
  segments INTEGER NOT NULL DEFAULT 1, "rank" INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(agent, session_id)
);
"#;

const SESSION_INSERT_SQL: &str = "INSERT OR REPLACE INTO sessions(\
     source, session_id, title, project, model, provider, started_at, ended_at, \
     started_at_ms, ended_at_ms, events, tool_calls, status, total_tokens, \
     input_tokens, cached_input_tokens, cache_creation_input_tokens, output_tokens, \
     reasoning_output_tokens\
     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)";

/// Column order shared by the query and `session_from_row`.
const SESSION_COLUMNS: &str = "session_id, source, provider, model, project, title, \
     started_at, ended_at, events, tool_calls, status, total_tokens, input_tokens, \
     cached_input_tokens, cache_creation_input_tokens, output_tokens, reasoning_output_tokens";

const WORKBENCH_INSERT_SQL: &str = "INSERT OR REPLACE INTO workbench(\
     agent, session_id, title, project, project_key, is_git_project, model, started_at, \
     updated_at, events, turns, status, tokens, segments, \"rank\"\
     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)";

const WORKBENCH_COLUMNS: &str = "agent, session_id, title, project, project_key, \
     is_git_project, model, started_at, updated_at, events, turns, status, tokens, segments";

#[cfg(test)]
static TEST_DB_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

#[cfg(test)]
pub(crate) fn set_db_path_for_tests(path: std::path::PathBuf) {
    let slot = TEST_DB_PATH.get_or_init(|| Mutex::new(None));
    *slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(path);
}

#[cfg(test)]
fn test_db_path() -> Option<PathBuf> {
    TEST_DB_PATH
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn db_path() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = test_db_path() {
        return path;
    }
    claude_codex_pro_core::paths::default_app_state_dir()
        .join("aitracker")
        .join("cache")
        .join(DB_FILE_NAME)
}

/// `true` when the path does not exist yet or starts with the SQLite header.
/// A missing file is left to SQLite (it creates it); a file with foreign bytes
/// is treated as corrupt and rebuilt by the caller.
fn has_sqlite_header(path: &Path) -> bool {
    use std::io::Read;

    let Ok(mut file) = fs::File::open(path) else {
        return true;
    };
    let mut header = [0_u8; 16];
    file.read_exact(&mut header).is_ok() && &header == b"SQLite format 3\0"
}

fn discard_database_files(path: &Path) {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let mut name = path.as_os_str().to_os_string();
        name.push(suffix);
        let _ = fs::remove_file(PathBuf::from(name));
    }
}

fn is_corruption_error(error: &rusqlite::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("not a database")
        || message.contains("malformed")
        || message.contains("corrupt")
}

fn error_is_corruption(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<rusqlite::Error>())
        .any(is_corruption_error)
}

fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version != 0 && version != SCHEMA_VERSION {
        // The file belongs to an incompatible build. It is only a cache, so
        // drop the known tables and recreate the current schema.
        connection.execute_batch(
            "DROP TABLE IF EXISTS sessions; \
             DROP TABLE IF EXISTS workbench; \
             DROP TABLE IF EXISTS meta;",
        )?;
    }
    connection.execute_batch(SCHEMA_SQL)?;
    if version != SCHEMA_VERSION {
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    Ok(())
}

fn open_schema(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    connection.busy_timeout(CONNECTION_BUSY_TIMEOUT)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    ensure_schema(&connection)?;
    Ok(connection)
}

/// Write connection. A cache file that is not a database (or that SQLite
/// reports as malformed) is discarded and rebuilt; an error reported mid-write
/// never reaches this path and therefore cannot delete healthy old rows.
fn open_write_connection() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create usage index directory {}", parent.display()))?;
    }
    if path.exists() && !has_sqlite_header(&path) {
        discard_database_files(&path);
    }
    match open_schema(&path) {
        Ok(connection) => Ok(connection),
        Err(error) if is_corruption_error(&error) => {
            discard_database_files(&path);
            open_schema(&path).with_context(|| format!("rebuild usage index at {}", path.display()))
        }
        Err(error) => Err(error).with_context(|| format!("open usage index at {}", path.display())),
    }
}

/// Short-lived read connection. The index is only a cache: any failure turns
/// into "not ready"/empty results in the public functions, never a panic.
fn open_read_connection() -> Option<Connection> {
    let path = db_path();
    if !path.is_file() {
        return None;
    }
    let connection = Connection::open(&path).ok()?;
    let _ = connection.busy_timeout(CONNECTION_BUSY_TIMEOUT);
    Some(connection)
}

fn upsert_meta(connection: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn now_ms_string() -> String {
    Utc::now().timestamp_millis().max(0).to_string()
}

/// Unix milliseconds for a session timestamp.
///
/// `aitracker.rs` has no reusable parser (its query inlines
/// `DateTime::parse_from_rfc3339(...).unwrap_or(true)`), so this mirrors that
/// parse and stores failures as `0`. The range predicate below keeps rows with
/// `started_at_ms = 0`, matching the existing in-memory semantics where an
/// unparseable start is treated as inside every range. Keeping that behavior
/// means a range filter never silently drops sessions that the current page
/// already shows.
fn timestamp_ms(value: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(value.trim())
        .map(|date| date.timestamp_millis())
        .unwrap_or(0)
}

fn range_cutoff_ms(range: &str) -> Option<i64> {
    let days = match range {
        "7d" => 7,
        "30d" => 30,
        "90d" => 90,
        _ => return None,
    };
    Some((Utc::now() - Duration::days(days)).timestamp_millis())
}

fn sql_u64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn sql_usize(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn read_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn read_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or(0)
}

fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn session_filters(
    agent: Option<&str>,
    status: Option<&str>,
    keyword: Option<&str>,
    range: Option<&str>,
) -> (String, Vec<SqlValue>) {
    let mut clauses: Vec<&str> = Vec::new();
    let mut values: Vec<SqlValue> = Vec::new();

    if let Some(agent) = agent.filter(|value| !value.is_empty()) {
        clauses.push("source = ?");
        values.push(SqlValue::Text(agent.to_string()));
    }
    if let Some(status) = status.filter(|value| !value.is_empty()) {
        clauses.push("status = ?");
        values.push(SqlValue::Text(status.to_string()));
    }
    if let Some(keyword) = keyword.map(str::trim).filter(|value| !value.is_empty()) {
        let pattern = format!("%{}%", escape_like(&keyword.to_lowercase()));
        // `source` carries the agent id (the index has no separate agent
        // column), so keyword matching still covers agent/provider/model/
        // project/title/session_id exactly like `aitracker::query_sessions`.
        clauses.push(concat!(
            r"(source LIKE ? ESCAPE '\' OR provider LIKE ? ESCAPE '\'",
            r" OR model LIKE ? ESCAPE '\' OR project LIKE ? ESCAPE '\'",
            r" OR title LIKE ? ESCAPE '\' OR session_id LIKE ? ESCAPE '\')",
        ));
        for _ in 0..6 {
            values.push(SqlValue::Text(pattern.clone()));
        }
    }
    if let Some(cutoff) = range.and_then(range_cutoff_ms) {
        // Parse failures are stored as 0; treat those as matching (rather
        // than excluding them) to preserve the existing query's behavior.
        clauses.push("(started_at_ms >= ? OR started_at_ms = 0)");
        values.push(SqlValue::Integer(cutoff));
    }

    let clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    (clause, values)
}

fn session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentSessionSummary> {
    Ok(AgentSessionSummary {
        session_id: row.get(0)?,
        agent: row.get(1)?,
        provider: row.get(2)?,
        model: row.get(3)?,
        project: row.get(4)?,
        title: row.get(5)?,
        started_at: row.get(6)?,
        ended_at: row.get(7)?,
        events: read_usize(row.get(8)?),
        tool_calls: read_usize(row.get(9)?),
        status: row.get(10)?,
        totals: AgentTokenTotals {
            total_tokens: read_u64(row.get(11)?),
            input_tokens: read_u64(row.get(12)?),
            cached_input_tokens: read_u64(row.get(13)?),
            cache_creation_input_tokens: read_u64(row.get(14)?),
            output_tokens: read_u64(row.get(15)?),
            reasoning_output_tokens: read_u64(row.get(16)?),
        },
    })
}

fn workbench_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkbenchRow> {
    Ok(WorkbenchRow {
        agent: row.get(0)?,
        session_id: row.get(1)?,
        title: row.get(2)?,
        project: row.get(3)?,
        project_key: row.get(4)?,
        is_git_project: row.get::<_, i64>(5)? != 0,
        model: row.get(6)?,
        started_at: row.get(7)?,
        updated_at: row.get(8)?,
        events: read_usize(row.get(9)?),
        turns: read_usize(row.get(10)?),
        status: row.get(11)?,
        tokens: read_u64(row.get(12)?),
        segments: read_usize(row.get(13)?),
    })
}

/// Replace the whole session index with the projection of one usage scan.
///
/// `snapshot.sessions` is the AITRACKER projection (the same rows the JSON
/// snapshot persists); `usage` contributes the snapshot metadata that lets a
/// reader serve `generated_at` / `mode` without loading the JSON snapshot.
pub fn write_usage(usage: &LocalUsageSnapshot, snapshot: &AitrackerSnapshot) -> Result<()> {
    match write_usage_once(usage, snapshot) {
        // Corruption can surface after the schema was opened (for example on
        // the first DELETE). The cache is disposable, so drop it and retry
        // once. Non-corruption errors (locks, disk full) leave the file alone.
        Err(error) if error_is_corruption(&error) => {
            discard_database_files(&db_path());
            write_usage_once(usage, snapshot)
        }
        result => result,
    }
}

fn write_usage_once(usage: &LocalUsageSnapshot, snapshot: &AitrackerSnapshot) -> Result<()> {
    let mut connection = open_write_connection()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute("DELETE FROM sessions", [])?;
    {
        let mut statement = transaction.prepare(SESSION_INSERT_SQL)?;
        for session in &snapshot.sessions {
            statement.execute(params![
                session.agent,
                session.session_id,
                session.title,
                session.project,
                session.model,
                session.provider,
                session.started_at,
                session.ended_at,
                timestamp_ms(&session.started_at),
                timestamp_ms(&session.ended_at),
                sql_usize(session.events),
                sql_usize(session.tool_calls),
                session.status,
                sql_u64(session.totals.total_tokens),
                sql_u64(session.totals.input_tokens),
                sql_u64(session.totals.cached_input_tokens),
                sql_u64(session.totals.cache_creation_input_tokens),
                sql_u64(session.totals.output_tokens),
                sql_u64(session.totals.reasoning_output_tokens),
            ])?;
        }
    }
    upsert_meta(&transaction, META_SESSIONS_WRITTEN_AT, &now_ms_string())?;
    upsert_meta(
        &transaction,
        META_SNAPSHOT_GENERATED_AT,
        &usage.generated_at,
    )?;
    upsert_meta(&transaction, META_SNAPSHOT_MODE, &usage.mode)?;
    transaction.commit().context("commit usage index")?;
    Ok(())
}

/// Replace the whole workbench projection; `rank` is the array index so a
/// reader gets the exact workbench order back.
pub fn write_workbench(rows: &[WorkbenchRow]) -> Result<()> {
    match write_workbench_once(rows) {
        Err(error) if error_is_corruption(&error) => {
            discard_database_files(&db_path());
            write_workbench_once(rows)
        }
        result => result,
    }
}

fn write_workbench_once(rows: &[WorkbenchRow]) -> Result<()> {
    let mut connection = open_write_connection()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute("DELETE FROM workbench", [])?;
    {
        let mut statement = transaction.prepare(WORKBENCH_INSERT_SQL)?;
        for (rank, row) in rows.iter().enumerate() {
            statement.execute(params![
                row.agent,
                row.session_id,
                row.title,
                row.project,
                row.project_key,
                i64::from(row.is_git_project),
                row.model,
                row.started_at,
                row.updated_at,
                sql_usize(row.events),
                sql_usize(row.turns),
                row.status,
                sql_u64(row.tokens),
                sql_usize(row.segments),
                sql_usize(rank),
            ])?;
        }
    }
    upsert_meta(&transaction, META_WORKBENCH_WRITTEN_AT, &now_ms_string())?;
    transaction.commit().context("commit workbench index")?;
    Ok(())
}

/// `true` when a complete session snapshot has been committed.
pub fn index_ready() -> bool {
    let Some(connection) = open_read_connection() else {
        return false;
    };
    connection
        .query_row(
            "SELECT 1 FROM meta WHERE key = ?1 LIMIT 1",
            params![META_SESSIONS_WRITTEN_AT],
            |_| Ok(()),
        )
        .is_ok()
}

/// `(generated_at, mode)` of the usage snapshot that produced the index.
///
/// This is what lets the indexed query path fill the existing payload without
/// reading/parsing the full JSON snapshot on a cold page load.
pub fn index_metadata() -> (String, String) {
    let Some(connection) = open_read_connection() else {
        return (String::new(), "empty".into());
    };
    let read = |key: &str| {
        connection
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .ok()
    };
    (
        read(META_SNAPSHOT_GENERATED_AT).unwrap_or_default(),
        read(META_SNAPSHOT_MODE).unwrap_or_else(|| "empty".into()),
    )
}

fn query_sessions_inner(
    agent: Option<&str>,
    status: Option<&str>,
    keyword: Option<&str>,
    range: Option<&str>,
    page: usize,
    page_size: usize,
) -> Result<(Vec<AgentSessionSummary>, usize)> {
    let Some(connection) = open_read_connection() else {
        return Ok((Vec::new(), 0));
    };
    // Count and page come from one read transaction so a concurrent index
    // rebuild cannot mix an old total with a new page.
    let transaction = connection.unchecked_transaction()?;
    let (where_clause, mut values) = session_filters(agent, status, keyword, range);
    let total: i64 = transaction.query_row(
        &format!("SELECT COUNT(*) FROM sessions{where_clause}"),
        params_from_iter(values.iter()),
        |row| row.get(0),
    )?;

    let page_size = page_size.clamp(1, 200);
    let limit = i64::try_from(page_size).unwrap_or(i64::MAX);
    let offset =
        i64::try_from(page.saturating_sub(1).saturating_mul(page_size)).unwrap_or(i64::MAX);
    let limit_index = values.len() + 1;
    let offset_index = values.len() + 2;
    values.push(SqlValue::Integer(limit));
    values.push(SqlValue::Integer(offset));

    let sql = format!(
        "SELECT {SESSION_COLUMNS} FROM sessions{where_clause} \
         ORDER BY started_at_ms DESC, session_id ASC, source ASC \
         LIMIT ?{limit_index} OFFSET ?{offset_index}"
    );
    let sessions = {
        let mut statement = transaction.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values.iter()), session_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    // A read transaction has nothing to persist; dropping it ends the snapshot.
    drop(transaction);
    Ok((sessions, read_usize(total)))
}

/// Query the index exactly like `aitracker::query_sessions` (1-based page,
/// page size clamped to 1..=200). Any read failure returns an empty result
/// instead of panicking so the command layer can fall back safely.
pub fn query_sessions(
    agent: Option<&str>,
    status: Option<&str>,
    keyword: Option<&str>,
    range: Option<&str>,
    page: usize,
    page_size: usize,
) -> (Vec<AgentSessionSummary>, usize) {
    query_sessions_inner(agent, status, keyword, range, page, page_size).unwrap_or_default()
}

pub fn load_workbench() -> Vec<WorkbenchRow> {
    let Some(connection) = open_read_connection() else {
        return Vec::new();
    };
    let load = || -> rusqlite::Result<Vec<WorkbenchRow>> {
        let mut statement = connection.prepare(&format!(
            "SELECT {WORKBENCH_COLUMNS} FROM workbench ORDER BY \"rank\" ASC"
        ))?;
        let rows = statement.query_map([], workbench_from_row)?;
        rows.collect()
    };
    load().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aitracker::AitrackerSnapshot;
    use crate::local_usage::aggregate;
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn install_temp_db() -> tempfile::TempDir {
        let directory = tempfile::tempdir().expect("tempdir");
        set_db_path_for_tests(directory.path().join(DB_FILE_NAME));
        directory
    }

    fn summary(
        agent: &str,
        session_id: &str,
        title: &str,
        project: &str,
        status: &str,
        started_at: &str,
        total_tokens: u64,
    ) -> AgentSessionSummary {
        AgentSessionSummary {
            session_id: session_id.into(),
            agent: agent.into(),
            provider: format!("{agent}-provider"),
            model: format!("{agent}-model"),
            project: project.into(),
            title: title.into(),
            started_at: started_at.into(),
            ended_at: started_at.into(),
            events: 3,
            tool_calls: 1,
            status: status.into(),
            totals: AgentTokenTotals {
                total_tokens,
                input_tokens: total_tokens / 2,
                cached_input_tokens: 7,
                cache_creation_input_tokens: 11,
                output_tokens: total_tokens / 3,
                reasoning_output_tokens: 13,
            },
        }
    }

    fn snapshot(sessions: Vec<AgentSessionSummary>) -> AitrackerSnapshot {
        AitrackerSnapshot {
            registry: Vec::new(),
            sessions,
            tool_calls: Vec::new(),
            details: Vec::new(),
        }
    }

    fn ids(sessions: &[AgentSessionSummary]) -> Vec<&str> {
        sessions
            .iter()
            .map(|session| session.session_id.as_str())
            .collect()
    }

    #[test]
    fn usage_index_writes_and_queries_with_filters_pagination_and_counts() {
        let _guard = test_guard();
        let directory = install_temp_db();
        let now = Utc::now();
        let sessions = vec![
            summary(
                "codex",
                "s1",
                "Alpha 100%",
                "proj-a",
                "completed",
                &(now - Duration::days(1)).to_rfc3339(),
                100,
            ),
            summary(
                "codex",
                "s2",
                "Beta",
                "proj-b",
                "error",
                &(now - Duration::days(40)).to_rfc3339(),
                50,
            ),
            summary(
                "claude-code",
                "s3",
                "Gamma",
                "proj-a",
                "completed",
                &(now - Duration::days(3)).to_rfc3339(),
                30,
            ),
            summary("cursor", "s4", "Delta_under", "proj-c", "running", "", 0),
        ];
        let usage = aggregate(Vec::new(), Vec::new());
        let write_started = Instant::now();
        write_usage(&usage, &snapshot(sessions)).expect("write usage index");
        eprintln!(
            "usage index write (4 sessions): {:?}",
            write_started.elapsed()
        );
        assert!(index_ready());
        // Connection/schema invariants are persisted with the database.
        let raw = Connection::open(directory.path().join(DB_FILE_NAME)).expect("open raw index");
        let journal_mode: String = raw
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("journal mode");
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
        // `synchronous` is connection-local, so it cannot be observed from a
        // fresh read connection; it is set by `open_schema` on every writer.
        let version: i64 = raw
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("user version");
        assert_eq!(version, 1);
        drop(raw);
        assert_eq!(
            index_metadata(),
            (usage.generated_at.clone(), usage.mode.clone())
        );

        let query_started = Instant::now();
        let (page, total) = query_sessions(None, None, None, None, 1, 2);
        eprintln!(
            "usage index first page (4 sessions): {:?}",
            query_started.elapsed()
        );
        assert_eq!(total, 4);
        assert_eq!(ids(&page), vec!["s1", "s3"]);
        // All token columns round-trip.
        assert_eq!(
            page[0].totals,
            AgentTokenTotals {
                total_tokens: 100,
                input_tokens: 50,
                cached_input_tokens: 7,
                cache_creation_input_tokens: 11,
                output_tokens: 33,
                reasoning_output_tokens: 13,
            }
        );
        let (page, total) = query_sessions(None, None, None, None, 2, 2);
        assert_eq!(total, 4);
        assert_eq!(ids(&page), vec!["s2", "s4"]);
        let (page, total) = query_sessions(None, None, None, None, 3, 2);
        assert_eq!(total, 4);
        assert!(page.is_empty());
        // Page size is clamped and page 0 behaves like page 1.
        let (page, total) = query_sessions(None, None, None, None, 0, 0);
        assert_eq!(total, 4);
        assert_eq!(ids(&page), vec!["s1"]);

        let (page, total) = query_sessions(Some("codex"), None, None, None, 1, 20);
        assert_eq!(total, 2);
        assert_eq!(ids(&page), vec!["s1", "s2"]);
        let (_, total) = query_sessions(Some(""), None, None, None, 1, 20);
        assert_eq!(total, 4);
        let (page, total) = query_sessions(None, Some("completed"), None, None, 1, 20);
        assert_eq!(total, 2);
        assert_eq!(ids(&page), vec!["s1", "s3"]);

        // Keyword covers project/title/session_id and is case-insensitive for
        // ASCII; `%` and `_` are escaped rather than acting as wildcards.
        let (_, total) = query_sessions(None, None, Some("proj-a"), None, 1, 20);
        assert_eq!(total, 2);
        let (_, total) = query_sessions(None, None, Some("GAMMA"), None, 1, 20);
        assert_eq!(total, 1);
        let (page, total) = query_sessions(None, None, Some("%"), None, 1, 20);
        assert_eq!(total, 1);
        assert_eq!(ids(&page), vec!["s1"]);
        let (page, total) = query_sessions(None, None, Some("_"), None, 1, 20);
        assert_eq!(total, 1);
        assert_eq!(ids(&page), vec!["s4"]);
        let (_, total) = query_sessions(None, None, Some("s2"), None, 1, 20);
        assert_eq!(total, 1);

        // Range: the unparseable start (s4) is kept, mirroring the existing
        // in-memory query's `unwrap_or(true)` behavior.
        let (page, total) = query_sessions(None, None, None, Some("7d"), 1, 20);
        assert_eq!(total, 3);
        assert_eq!(ids(&page), vec!["s1", "s3", "s4"]);
        let (_, total) = query_sessions(None, None, None, Some("30d"), 1, 20);
        assert_eq!(total, 3);
        let (_, total) = query_sessions(None, None, None, Some("90d"), 1, 20);
        assert_eq!(total, 4);
        let (_, total) = query_sessions(None, None, None, Some("all"), 1, 20);
        assert_eq!(total, 4);
        let (_, total) = query_sessions(None, None, None, Some("unknown"), 1, 20);
        assert_eq!(total, 4);
    }

    #[test]
    fn workbench_roundtrips_in_rank_order() {
        let _guard = test_guard();
        let _directory = install_temp_db();
        assert!(!index_ready());
        assert!(load_workbench().is_empty());

        let rows = vec![
            WorkbenchRow {
                agent: "codex".into(),
                session_id: "w1".into(),
                title: "First".into(),
                project: "proj-a".into(),
                project_key: "proj-a".into(),
                is_git_project: true,
                model: "gpt-5".into(),
                started_at: "2026-10-01T00:00:00Z".into(),
                updated_at: "2026-10-02T00:00:00Z".into(),
                events: 12,
                turns: 5,
                status: "completed".into(),
                tokens: 1_234,
                segments: 2,
            },
            WorkbenchRow {
                agent: "claude-code".into(),
                session_id: "w2".into(),
                title: "Second".into(),
                project: "unknown".into(),
                project_key: "unknown".into(),
                is_git_project: false,
                model: "claude-sonnet".into(),
                started_at: String::new(),
                updated_at: String::new(),
                events: 0,
                turns: 0,
                status: String::new(),
                tokens: 0,
                segments: 1,
            },
        ];
        write_workbench(&rows).expect("write workbench index");
        assert_eq!(load_workbench(), rows);
        // Workbench data alone does not mark the usage session index ready.
        assert!(!index_ready());
    }

    #[test]
    fn corrupt_database_file_is_rebuilt_on_write() {
        let _guard = test_guard();
        let directory = install_temp_db();
        let path = directory.path().join(DB_FILE_NAME);
        let usage = aggregate(Vec::new(), Vec::new());
        let recovered = || {
            snapshot(vec![summary(
                "codex",
                "recovered",
                "Recovered",
                "proj",
                "completed",
                "2026-10-01T00:00:00Z",
                5,
            )])
        };

        // A foreign file is rejected by the SQLite header check.
        fs::write(&path, b"definitely not a sqlite database").expect("seed corrupt db");
        write_usage(&usage, &recovered()).expect("rebuild foreign file");
        assert!(index_ready());
        assert!(has_sqlite_header(&path));
        let (page, total) = query_sessions(None, None, None, None, 1, 20);
        assert_eq!(total, 1);
        assert_eq!(ids(&page), vec!["recovered"]);

        // A file with a valid header but unreadable pages is reported by
        // SQLite itself; the write path must discard and rebuild it too.
        fs::write(
            &path,
            b"SQLite format 3\0this is not a real database body".as_slice(),
        )
        .expect("seed header-only db");
        write_usage(&usage, &recovered()).expect("rebuild malformed sqlite file");
        assert!(index_ready());
        let (page, total) = query_sessions(None, None, None, None, 1, 20);
        assert_eq!(total, 1);
        assert_eq!(ids(&page), vec!["recovered"]);
    }

    #[test]
    fn missing_index_reports_not_ready_and_reads_empty() {
        let _guard = test_guard();
        let _directory = install_temp_db();
        assert!(!index_ready());
        assert_eq!(
            query_sessions(None, None, None, None, 1, 20),
            (Vec::new(), 0)
        );
        assert!(load_workbench().is_empty());
        assert_eq!(index_metadata(), (String::new(), "empty".into()));
    }

    #[test]
    fn failed_write_rolls_back_and_keeps_previous_index() {
        let _guard = test_guard();
        let directory = install_temp_db();
        let usage = aggregate(Vec::new(), Vec::new());
        write_usage(
            &usage,
            &snapshot(vec![summary(
                "codex",
                "old",
                "Old",
                "proj",
                "completed",
                "2026-10-01T00:00:00Z",
                1,
            )]),
        )
        .expect("seed index");

        let connection =
            Connection::open(directory.path().join(DB_FILE_NAME)).expect("open raw db");
        connection
            .execute_batch(
                "CREATE TRIGGER fail_sessions_insert BEFORE INSERT ON sessions \
                 BEGIN SELECT RAISE(ABORT, 'forced failure'); END;",
            )
            .expect("create failure trigger");
        drop(connection);

        let error = write_usage(
            &usage,
            &snapshot(vec![summary(
                "codex",
                "new",
                "New",
                "proj",
                "completed",
                "2026-10-02T00:00:00Z",
                2,
            )]),
        );
        assert!(error.is_err());
        let (page, total) = query_sessions(None, None, None, None, 1, 20);
        assert_eq!(total, 1);
        assert_eq!(ids(&page), vec!["old"]);
    }

    #[test]
    fn usage_index_scales_to_ten_thousand_sessions() {
        let _guard = test_guard();
        let _directory = install_temp_db();
        let now = Utc::now();
        let sessions = (0..10_000)
            .map(|index| {
                let started_at = (now - Duration::minutes(index)).to_rfc3339();
                summary(
                    "codex",
                    &format!("session-{index:05}"),
                    &format!("bulk session title {index}"),
                    "bulk",
                    "completed",
                    &started_at,
                    index as u64,
                )
            })
            .collect::<Vec<_>>();
        let usage = aggregate(Vec::new(), Vec::new());

        let write_started = Instant::now();
        write_usage(&usage, &snapshot(sessions)).expect("write 10k index");
        let write_elapsed = write_started.elapsed();

        let query_started = Instant::now();
        let (page, total) = query_sessions(
            Some("codex"),
            Some("completed"),
            Some("bulk session title"),
            Some("90d"),
            1,
            50,
        );
        let query_elapsed = query_started.elapsed();
        eprintln!(
            "usage index 10k sessions: write={write_elapsed:?} filtered-query={query_elapsed:?}"
        );

        assert_eq!(total, 10_000);
        assert_eq!(page.len(), 50);
        assert!(
            write_elapsed < std::time::Duration::from_secs(10),
            "write took {write_elapsed:?}"
        );
        assert!(
            query_elapsed < std::time::Duration::from_secs(2),
            "query took {query_elapsed:?}"
        );
    }
}
