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
const SCHEMA_VERSION: i64 = 2;

/// `kv` marker written once the `providers` table has been seeded from
/// `settings.json`. Its absence means "never migrated", which is what tells
/// the settings store apart from a database that legitimately holds zero
/// suppliers.
const PROVIDERS_MIGRATED_KEY: &str = "providers.migrated";

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
    if version < 2 {
        transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS providers (
                 id TEXT PRIMARY KEY,
                 position INTEGER NOT NULL,
                 profile_json TEXT NOT NULL
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

/// Read the supplier profiles stored in the `providers` table.
///
/// `Ok(None)` means "this database has never been migrated from
/// `settings.json`" and is distinct from `Ok(Some(vec![]))`, which means the
/// user really has no suppliers left. Rows whose `profile_json` no longer
/// parses are dropped with a warning that names the row id only; the stored
/// profile content (which may hold an API key) is never echoed.
pub fn providers_load(conn: &Connection) -> anyhow::Result<Option<Vec<serde_json::Value>>> {
    if kv_get(conn, PROVIDERS_MIGRATED_KEY)?.is_none() {
        return Ok(None);
    }

    let mut statement = conn.prepare("SELECT id, profile_json FROM providers ORDER BY position")?;
    let mut rows = statement.query([])?;
    let mut profiles = Vec::new();
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let profile_json: String = row.get(1)?;
        match serde_json::from_str::<serde_json::Value>(&profile_json) {
            Ok(profile) => profiles.push(profile),
            Err(error) => eprintln!("供应商记录 {id} 解析失败，已跳过: {error}"),
        }
    }
    Ok(Some(profiles))
}

/// Replace every stored supplier profile with `profiles`, in the given order.
///
/// Entries without a usable `"id"` are skipped, and the first entry wins when
/// ids repeat, so the `providers` primary key can never be violated. The whole
/// rewrite runs in one transaction together with the `providers.migrated`
/// marker, so a failure leaves the previous suppliers untouched.
pub fn providers_replace(
    conn: &mut Connection,
    profiles: &[serde_json::Value],
) -> anyhow::Result<()> {
    let transaction = conn.transaction()?;
    transaction.execute("DELETE FROM providers", [])?;
    {
        let mut statement = transaction
            .prepare("INSERT INTO providers (id, position, profile_json) VALUES (?1, ?2, ?3)")?;
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for (position, profile) in profiles.iter().enumerate() {
            let Some(id) = profile.get("id").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !seen.insert(id) {
                continue;
            }
            statement.execute(rusqlite::params![id, position as i64, profile.to_string()])?;
        }
    }
    transaction.execute(
        "INSERT INTO kv (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [PROVIDERS_MIGRATED_KEY, "1"],
    )?;
    transaction.commit()?;
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
        assert_eq!(version, 2);
        assert!(path.is_file(), "数据库文件应已创建");

        for table in [
            "kv",
            "proxy_config",
            "failover_queue",
            "circuit_state",
            "providers",
        ] {
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
        assert_eq!(version, 2);
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

    fn provider(id: &str) -> serde_json::Value {
        serde_json::json!({ "id": id, "name": format!("供应商 {id}") })
    }

    fn provider_rows(conn: &Connection) -> Vec<(String, i64, String)> {
        let mut statement = conn
            .prepare("SELECT id, position, profile_json FROM providers ORDER BY position")
            .expect("准备查询 providers");
        let mut rows = statement.query([]).expect("查询 providers");
        let mut collected = Vec::new();
        while let Some(row) = rows.next().expect("读取 providers 行") {
            collected.push((
                row.get::<_, String>(0).expect("读取 id"),
                row.get::<_, i64>(1).expect("读取 position"),
                row.get::<_, String>(2).expect("读取 profile_json"),
            ));
        }
        collected
    }

    #[test]
    fn ccp_db_providers_are_unmigrated_until_marked() {
        let dir = temp_dir("providers-unmigrated");
        let path = dir.join("ccp.db");
        let mut conn = open(&path).expect("打开数据库");

        // 从未迁移过的数据库与"迁移过但真的没有供应商"必须可区分。
        assert_eq!(providers_load(&conn).expect("读取未迁移的供应商"), None);

        providers_replace(&mut conn, &[]).expect("写入空供应商列表");
        assert_eq!(
            providers_load(&conn).expect("读取空供应商列表"),
            Some(Vec::new())
        );

        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ccp_db_providers_roundtrip_preserves_order() {
        let dir = temp_dir("providers-order");
        let path = dir.join("ccp.db");
        let mut conn = open(&path).expect("打开数据库");

        let profiles = vec![provider("b"), provider("a"), provider("c")];
        providers_replace(&mut conn, &profiles).expect("写入供应商");

        assert_eq!(
            providers_load(&conn).expect("读取供应商"),
            Some(profiles.clone())
        );
        let rows = provider_rows(&conn);
        assert_eq!(
            rows.iter()
                .map(|(id, position, _)| (id.as_str(), *position))
                .collect::<Vec<_>>(),
            vec![("b", 0), ("a", 1), ("c", 2)]
        );

        // 再次写入必须整体替换，不留下旧行。
        providers_replace(&mut conn, &profiles[..1]).expect("覆盖供应商");
        assert_eq!(
            providers_load(&conn).expect("读取供应商"),
            Some(vec![provider("b")])
        );
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM providers", [], |row| row.get(0))
            .expect("统计 providers 行数");
        assert_eq!(count, 1);

        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ccp_db_providers_skip_missing_and_duplicate_ids() {
        let dir = temp_dir("providers-ids");
        let path = dir.join("ccp.db");
        let mut conn = open(&path).expect("打开数据库");

        let first = provider("dup");
        let second = serde_json::json!({ "id": "dup", "name": "第二个重复 id" });
        providers_replace(
            &mut conn,
            &[
                serde_json::json!({ "name": "没有 id" }),
                first.clone(),
                second,
                serde_json::json!({ "id": "unique", "name": "唯一" }),
            ],
        )
        .expect("写入供应商");

        let rows = provider_rows(&conn);
        assert_eq!(
            rows.iter()
                .map(|(id, _, _)| id.as_str())
                .collect::<Vec<_>>(),
            vec!["dup", "unique"]
        );
        assert_eq!(rows[0].2, first.to_string(), "重复 id 必须保留第一条");
        assert_eq!(rows[0].1, 1, "position 使用原始下标");
        assert_eq!(rows[1].1, 3);

        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ccp_db_version_one_upgrades_to_two_with_providers_table() {
        let dir = temp_dir("providers-upgrade");
        let path = dir.join("ccp.db");

        // 手工造一个版本 1 的数据库，模拟已经装过旧版的用户。
        {
            let conn = Connection::open(&path).expect("创建版本 1 数据库");
            conn.execute_batch(
                "CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE proxy_config (app_id TEXT PRIMARY KEY, config_json TEXT NOT NULL);
                 CREATE TABLE failover_queue (
                     app_id TEXT NOT NULL,
                     provider_id TEXT NOT NULL,
                     priority INTEGER NOT NULL,
                     PRIMARY KEY(app_id, provider_id)
                 );
                 CREATE TABLE circuit_state (
                     app_id TEXT NOT NULL,
                     provider_id TEXT NOT NULL,
                     state TEXT NOT NULL DEFAULT 'closed',
                     consecutive_failures INTEGER NOT NULL DEFAULT 0,
                     consecutive_successes INTEGER NOT NULL DEFAULT 0,
                     opened_at_ms INTEGER,
                     PRIMARY KEY(app_id, provider_id)
                 );
                 PRAGMA user_version = 1;",
            )
            .expect("写入版本 1 结构");
        }

        let mut conn = open(&path).expect("升级版本 1 数据库");
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("读取 user_version");
        assert_eq!(version, 2);
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'providers'",
                [],
                |row| row.get(0),
            )
            .expect("查询 providers 表");
        assert_eq!(tables, 1, "升级后必须存在 providers 表");

        // 升级后的库仍然可以正常读写供应商。
        providers_replace(&mut conn, &[provider("after-upgrade")]).expect("写入供应商");
        assert_eq!(
            providers_load(&conn).expect("读取供应商"),
            Some(vec![provider("after-upgrade")])
        );

        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
