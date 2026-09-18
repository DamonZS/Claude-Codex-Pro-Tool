use super::*;
use crate::multica_workspace::{
    LocalMulticaWorkspaceStore, LocalWorkspaceEntityUpsert, MulticaWorkspaceResourceKey,
};
use rusqlite::params;

fn fixture(path: &Path) -> Connection {
    let db = Connection::open(path).unwrap();
    // Relevant columns from the inspected native schema; title deliberately holds instructions.
    db.execute_batch("CREATE TABLE threads (id TEXT PRIMARY KEY, title TEXT, agent_nickname TEXT, updated_at INTEGER, updated_at_ms INTEGER, archived INTEGER);
        CREATE TABLE thread_spawn_edges (parent_thread_id TEXT NOT NULL, child_thread_id TEXT NOT NULL, status TEXT NOT NULL);").unwrap();
    db
}

fn history(home: &Path) -> Connection {
    let db = Connection::open(home.join("thread_history_1.sqlite")).unwrap();
    db.execute_batch(
        "CREATE TABLE thread_turns (
        thread_id TEXT NOT NULL, turn_id TEXT NOT NULL, rollout_ordinal INTEGER NOT NULL,
        status TEXT NOT NULL, error_json TEXT, started_at INTEGER, completed_at INTEGER,
        duration_ms INTEGER, first_user_item_id TEXT, final_agent_item_id TEXT,
        rollout_byte_offset INTEGER, rollout_end_ordinal INTEGER, rollout_end_byte_offset INTEGER,
        PRIMARY KEY (thread_id, turn_id));",
    )
    .unwrap();
    db
}

fn child(db: &Connection, id: &str, nickname: Option<&str>, edge: &str, updated: i64) {
    db.execute(
        "INSERT INTO threads VALUES (?1, 'PRIVATE TASK INSTRUCTIONS', ?2, 1, ?3, 1)",
        params![id, nickname, updated],
    )
    .unwrap();
    db.execute(
        "INSERT INTO thread_spawn_edges VALUES ('parent-thread', ?1, ?2)",
        params![id, edge],
    )
    .unwrap();
}

fn turn(db: &Connection, child: &str, turn: &str, status: &str, ordinal: i64) {
    db.execute(
        "INSERT INTO thread_turns (thread_id, turn_id, status, rollout_ordinal, started_at) VALUES (?1, ?2, ?3, ?4, 1000 - ?4)",
        params![child, turn, status, ordinal],
    )
    .unwrap();
}

fn request(limit: u16, offset: u32) -> MulticaWorkspaceQuery {
    MulticaWorkspaceQuery {
        resource: MulticaWorkspaceResourceKey::CodexNativeAgents,
        limit,
        offset,
    }
}

fn read(home: &Path, limit: u16, offset: u32) -> anyhow::Result<MulticaWorkspaceCollection> {
    query(
        &super::super::local_workspace_identity(),
        request(limit, offset),
        home,
    )
}

#[test]
fn native_subtasks_latest_turn_is_independent_of_edge_and_archival() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state_5.sqlite");
    let db = fixture(&path);
    let history = history(dir.path());
    child(&db, "child-one", Some("Descartes"), "closed", 5000);
    child(&db, "child-two", None, "open", 4000);
    child(&db, "child-unknown", Some(" "), "closed", 3000);
    child(&db, "child-empty", Some(&"N".repeat(200)), "open", 2000);
    db.execute("INSERT INTO threads VALUES ('ordinary', 'PRIVATE TASK INSTRUCTIONS', 'Not a child', 10, 9999, 0)", []).unwrap();
    // IDs and started_at both disagree with rollout order; ordinal is authoritative.
    turn(&history, "child-one", "z-old", "completed", 100);
    turn(&history, "child-one", "a-new", "inProgress", 200);
    turn(&history, "child-two", "turn", "completed", 100);
    turn(
        &history,
        "child-unknown",
        "turn",
        "unrecognized future status",
        100,
    );
    db.execute(
        "INSERT INTO thread_spawn_edges VALUES ('parent-thread', 'child-one', 'closed')",
        [],
    )
    .unwrap();
    drop(db);
    drop(history);
    let before = std::fs::read(&path).unwrap();
    let history_path = dir.path().join("thread_history_1.sqlite");
    let history_before = std::fs::read(&history_path).unwrap();
    let result = read(dir.path(), 100, 0).unwrap();
    assert_eq!(result.total, 4);
    assert!(!result.stale);
    assert_eq!(
        result.items[0],
        json!({"id":"child-one","parent_thread_id":"parent-thread","agent_nickname":"Descartes",
        "status":"inProgress","edge_status":"closed","updated_at_ms":5000,"source":"codex_native","read_only":true})
    );
    assert_eq!(result.items[1]["status"], "completed");
    assert_eq!(result.items[1]["edge_status"], "open");
    assert_eq!(result.items[1]["agent_nickname"], "child-tw");
    assert_eq!(result.items[2]["status"], "unknown");
    assert_eq!(result.items[3]["status"], "unknown");
    assert_eq!(
        result.items[3]["agent_nickname"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        120
    );
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("PRIVATE TASK")
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(&history_path).unwrap(), history_before);
    assert!(!path.with_extension("sqlite-journal").exists());
}

#[test]
fn native_subtasks_missing_history_is_stale_and_legacy_seconds_are_milliseconds() {
    let dir = tempfile::tempdir().unwrap();
    let db = fixture(&dir.path().join("state_5.sqlite"));
    child(&db, "legacy-child", None, "closed", 2000);
    db.execute(
        "UPDATE threads SET updated_at_ms = NULL, updated_at = 123",
        [],
    )
    .unwrap();
    let result = read(dir.path(), 1, 0).unwrap();
    assert_eq!(result.items[0]["updated_at_ms"], 123000);
    assert_eq!(result.items[0]["status"], "unknown");
    assert_eq!(result.items[0]["edge_status"], "closed");
    assert!(result.stale);
    assert_eq!(result.diagnostic.as_deref(), Some(HISTORY_UNAVAILABLE));
    assert!(!dir.path().join("thread_history_1.sqlite").exists());
    db.execute_batch("ALTER TABLE threads DROP COLUMN updated_at_ms;")
        .unwrap();
    assert_eq!(
        read(dir.path(), 1, 0).unwrap().items[0]["updated_at_ms"],
        123000
    );
}

#[test]
fn native_subtasks_bounded_pagination_and_cross_database_deduplication() {
    let dir = tempfile::tempdir().unwrap();
    let sqlite_dir = dir.path().join("sqlite");
    std::fs::create_dir(&sqlite_dir).unwrap();
    let db = fixture(&sqlite_dir.join("state_5.sqlite"));
    for i in 0..505 {
        child(&db, &format!("child-{i:03}"), None, "open", 1000 + i);
    }
    let legacy = fixture(&dir.path().join("state_5.sqlite"));
    child(&legacy, "child-504", Some("Newer copy"), "closed", 9000);
    let first = read(dir.path(), 100, 0).unwrap();
    assert_eq!(first.total, 500);
    assert_eq!(first.items.len(), 100);
    assert_eq!(first.items[0]["agent_nickname"], "Newer copy");
    let last = read(dir.path(), 100, 495).unwrap();
    assert_eq!(last.items.len(), 5);
    assert_eq!(last.items.last().unwrap()["id"], "child-005");
    assert!(read(dir.path(), 100, 500).unwrap().items.is_empty());
    assert!(read(dir.path(), 101, 0).is_err());
    assert!(read(dir.path(), 0, 0).is_err());
}

#[test]
fn native_subtasks_read_failures_are_errors_or_explicitly_stale() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state_5.sqlite");
    assert_eq!(
        read(dir.path(), 100, 0).unwrap_err().to_string(),
        "codex_native_agents_database_unavailable"
    );
    assert!(!path.exists());
    std::fs::write(&path, b"not a sqlite database").unwrap();
    assert_eq!(
        read(dir.path(), 100, 0).unwrap_err().to_string(),
        READ_FAILED
    );
    let sqlite_dir = dir.path().join("sqlite");
    std::fs::create_dir(&sqlite_dir).unwrap();
    let db = fixture(&sqlite_dir.join("state_5.sqlite"));
    child(&db, "readable-child", None, "open", 10);
    let result = read(dir.path(), 100, 0).unwrap();
    assert_eq!(result.items.len(), 1);
    assert!(result.stale);
    assert_eq!(result.diagnostic.as_deref(), Some(READ_FAILED));
}

#[test]
fn native_subtasks_schema_errors_and_empty_valid_inventory_are_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let db = fixture(&dir.path().join("state_5.sqlite"));
    let history = history(dir.path());
    let empty = read(dir.path(), 100, 0).unwrap();
    assert_eq!(empty.total, 0);
    assert!(!empty.stale);
    history
        .execute_batch(
            "DROP TABLE thread_turns; CREATE TABLE thread_turns (thread_id TEXT, status TEXT);",
        )
        .unwrap();
    let stale = read(dir.path(), 100, 0).unwrap();
    assert!(stale.stale);
    assert_eq!(
        stale.diagnostic.as_deref(),
        Some(HISTORY_SCHEMA_UNSUPPORTED)
    );
    db.execute_batch("DROP TABLE thread_spawn_edges;").unwrap();
    assert_eq!(
        read(dir.path(), 100, 0).unwrap_err().to_string(),
        SCHEMA_UNSUPPORTED
    );
}

#[test]
fn native_subtasks_history_statuses_and_read_failure_do_not_use_edge_state() {
    let dir = tempfile::tempdir().unwrap();
    let db = fixture(&dir.path().join("state_5.sqlite"));
    let history = history(dir.path());
    for (index, status) in ["inProgress", "completed", "failed", "interrupted"]
        .iter()
        .enumerate()
    {
        let id = format!("child-{index}");
        child(&db, &id, None, "open", 100 - index as i64);
        turn(&history, &id, "turn", status, 1);
    }
    let result = read(dir.path(), 100, 0).unwrap();
    assert!(!result.stale);
    for (item, status) in
        result
            .items
            .iter()
            .zip(["inProgress", "completed", "failed", "interrupted"])
    {
        assert_eq!(item["status"], status);
        assert_eq!(item["edge_status"], "open");
    }
    drop(history);
    std::fs::write(
        dir.path().join("thread_history_1.sqlite"),
        b"corrupt fixture",
    )
    .unwrap();
    let stale = read(dir.path(), 100, 0).unwrap();
    assert_eq!(stale.items.len(), 4);
    assert!(stale.items.iter().all(|item| item["status"] == "unknown"));
    assert!(stale.stale);
    assert_eq!(stale.diagnostic.as_deref(), Some(HISTORY_READ_FAILED));
}

#[test]
fn native_subtasks_resource_contract_is_query_only() {
    let query: MulticaWorkspaceQuery =
        serde_json::from_value(json!({"resource":"codex_native_agents","limit":100,"offset":0}))
            .unwrap();
    assert_eq!(query.resource.key(), "codex_native_agents");
    assert!(MulticaWorkspaceResourceKey::ALL.contains(&query.resource));
    let dir = tempfile::tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    assert!(
        store
            .upsert(
                "local-test",
                LocalWorkspaceEntityUpsert {
                    resource: query.resource,
                    entity: json!({"id":"child"}),
                    expected_revision: Some(0)
                },
                1
            )
            .is_err()
    );
    assert!(!store.path().exists());
}
