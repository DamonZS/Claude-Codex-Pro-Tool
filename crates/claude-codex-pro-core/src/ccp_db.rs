//! The CCP-owned SQLite database (`ccp.db`).
//!
//! Everything the routing layer persists (per-app proxy parameters, per-app
//! failover queues, circuit state, and small key/value pairs such as the
//! global outbound proxy password) lives here. The file sits next to
//! `settings.json` in the CCP application state directory.
//!
//! Schema versioning uses `PRAGMA user_version`, so opening an existing
//! database twice is safe and migrations are idempotent.

use std::path::{Path, PathBuf};

use anyhow::Context;
use rusqlite::Connection;

/// Current schema version written to `PRAGMA user_version`.
const SCHEMA_VERSION: i64 = 1;

/// Default location: `<app state dir>/ccp.db`.
pub fn default_db_path() -> PathBuf {
    crate::paths::default_app_state_dir().join("ccp.db")
}

/// Open (creating if needed) the CCP database at `path` and run migrations.
pub fn open(path: &Path) -> anyhow::Result<Connection> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建数据库目录失败：{}", parent.display()))?;
    }

    let conn =
        Connection::open(path).with_context(|| format!("打开数据库失败：{}", path.display()))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;",
    )
    .with_context(|| format!("初始化数据库连接失败：{}", path.display()))?;
    migrate(&conn)?;
    Ok(conn)
}

/// Create or upgrade the schema, tracking progress in `PRAGMA user_version`.
fn migrate(conn: &Connection) -> anyhow::Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version >= SCHEMA_VERSION {
        return Ok(());
    }

    let transaction = conn.unchecked_transaction()?;
    if version < 1 {
        transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS kv (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS proxy_config (
                 app_id TEXT PRIMARY KEY,
                 config_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS failover_queue (
                 app_id TEXT NOT NULL,
                 provider_id TEXT NOT NULL,
                 priority INTEGER NOT NULL,
                 PRIMARY KEY(app_id, provider_id)
             );
             CREATE TABLE IF NOT EXISTS circuit_state (
                 app_id TEXT NOT NULL,
                 provider_id TEXT NOT NULL,
                 state TEXT NOT NULL DEFAULT 'closed',
                 consecutive_failures INTEGER NOT NULL DEFAULT 0,
                 consecutive_successes INTEGER NOT NULL DEFAULT 0,
                 opened_at_ms INTEGER,
                 PRIMARY KEY(app_id, provider_id)
             );",
        )?;
    }
    // `PRAGMA` does not accept bound parameters; the value is a private
    // integer constant, never user input.
    transaction.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
    transaction.commit()?;
    Ok(())
}

/// Read a key from the `kv` table.
pub fn kv_get(conn: &Connection, key: &str) -> anyhow::Result<Option<String>> {
    let mut statement = conn.prepare("SELECT value FROM kv WHERE key = ?1")?;
    let mut rows = statement.query([key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get::<_, String>(0)?)),
        None => Ok(None),
    }
}

/// Write a key into the `kv` table, replacing any previous value.
pub fn kv_set(conn: &Connection, key: &str, value: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO kv (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let unique = format!(
            "ccp-ccp-db-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).expect("创建测试临时目录");
        dir
    }

    #[test]
    fn ccp_db_open_migrates_and_is_idempotent() {
        let dir = temp_dir("migrate");
        let path = dir.join("nested").join("ccp.db");

        let conn = open(&path).expect("首次打开数据库");
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("读取 user_version");
        assert_eq!(version, SCHEMA_VERSION);
        assert!(path.is_file(), "数据库文件应已创建");

        for table in ["kv", "proxy_config", "failover_queue", "circuit_state"] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("查询表是否存在");
            assert_eq!(count, 1, "缺少数据表 {table}");
        }
        drop(conn);

        // 第二次打开同一路径不得失败，且版本保持不变。
        let conn = open(&path).expect("第二次打开数据库");
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("读取 user_version");
        assert_eq!(version, SCHEMA_VERSION);
        drop(conn);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ccp_db_kv_roundtrip() {
        let dir = temp_dir("kv");
        let path = dir.join("ccp.db");
        let conn = open(&path).expect("打开数据库");

        assert_eq!(kv_get(&conn, "routing.config").expect("读取缺失的键"), None);

        kv_set(&conn, "routing.config", "{\"enabled\":true}").expect("写入键");
        assert_eq!(
            kv_get(&conn, "routing.config").expect("读取键"),
            Some("{\"enabled\":true}".to_string())
        );

        // 同一个键再次写入时应覆盖，而不是插入第二行。
        kv_set(&conn, "routing.config", "{\"enabled\":false}").expect("覆盖键");
        assert_eq!(
            kv_get(&conn, "routing.config").expect("读取键"),
            Some("{\"enabled\":false}".to_string())
        );
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM kv", [], |row| row.get(0))
            .expect("统计 kv 行数");
        assert_eq!(rows, 1);

        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
