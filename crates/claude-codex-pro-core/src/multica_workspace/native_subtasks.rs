//! Bounded read-only native child-thread metadata, separate from editable Issues.
use super::{
    MulticaWorkspaceCollection, MulticaWorkspaceIdentity, MulticaWorkspaceQuery, collection,
};
use anyhow::bail;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

const RECENT_LIMIT: usize = 500;
const READ_FAILED: &str = "codex_native_agents_read_failed";
const SCHEMA_UNSUPPORTED: &str = "codex_native_agents_schema_unsupported";
const HISTORY_UNAVAILABLE: &str = "codex_native_agents_history_unavailable";
const HISTORY_READ_FAILED: &str = "codex_native_agents_history_read_failed";
const HISTORY_SCHEMA_UNSUPPORTED: &str = "codex_native_agents_history_schema_unsupported";

pub(super) fn query(
    workspace: &MulticaWorkspaceIdentity,
    query: MulticaWorkspaceQuery,
    home: &Path,
) -> anyhow::Result<MulticaWorkspaceCollection> {
    query.validate()?;
    let mut items = BTreeMap::<String, Value>::new();
    let mut diagnostic = None;
    let mut readable = false;
    for path in crate::codex_sqlite::codex_session_db_paths_from_home(home) {
        // The path resolver includes the optional legacy path even when absent.
        match path.try_exists() {
            Ok(false) => continue,
            Err(_) => {
                diagnostic = Some(READ_FAILED);
                continue;
            }
            Ok(true) => {}
        }
        match read_database(&path) {
            Ok(None) => {} // A session DB may contain only automation/inbox tables.
            Ok(Some(rows)) => {
                readable = true;
                for row in rows {
                    let id = row["id"].as_str().unwrap().to_owned();
                    if items.get(&id).is_none_or(|old| {
                        old["updated_at_ms"].as_i64() < row["updated_at_ms"].as_i64()
                    }) {
                        items.insert(id, row);
                    }
                }
            }
            Err(error) => {
                diagnostic = Some(if error.to_string() == SCHEMA_UNSUPPORTED {
                    SCHEMA_UNSUPPORTED
                } else {
                    READ_FAILED
                });
            }
        }
    }
    if !readable {
        bail!(diagnostic.unwrap_or("codex_native_agents_database_unavailable"));
    }
    let mut items: Vec<_> = items.into_values().collect();
    items.sort_by(|a, b| {
        b["updated_at_ms"]
            .as_i64()
            .cmp(&a["updated_at_ms"].as_i64())
            .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
    });
    items.truncate(RECENT_LIMIT);
    let total = items.len() as u64;
    let mut page: Vec<Value> = items
        .into_iter()
        .skip(query.offset as usize)
        .take(query.limit as usize)
        .collect();
    if let Err(error) = read_turn_statuses(home, &mut page) {
        let code = match error.to_string().as_str() {
            HISTORY_UNAVAILABLE => HISTORY_UNAVAILABLE,
            HISTORY_SCHEMA_UNSUPPORTED => HISTORY_SCHEMA_UNSUPPORTED,
            _ => HISTORY_READ_FAILED,
        };
        diagnostic = diagnostic.or(Some(code));
    }
    let mut result = collection(
        workspace,
        query.resource,
        page,
        total,
        query.limit,
        query.offset,
    );
    result.stale = diagnostic.is_some();
    result.diagnostic = diagnostic.map(str::to_owned);
    Ok(result)
}

fn columns(db: &Connection, table: &str) -> anyhow::Result<BTreeSet<String>> {
    // All callers supply a source-code table name, never renderer input.
    let mut statement = db.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<BTreeSet<_>, _>>()?;
    Ok(columns)
}

fn read_turn_statuses(home: &Path, items: &mut [Value]) -> anyhow::Result<()> {
    let path = home.join("thread_history_1.sqlite");
    if !path.try_exists()? {
        bail!(HISTORY_UNAVAILABLE);
    }
    let mut db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    db.busy_timeout(Duration::from_millis(250))?;
    let transaction = db.transaction()?;
    let turns = columns(&transaction, "thread_turns")?;
    if !["thread_id", "turn_id", "rollout_ordinal", "status"]
        .iter()
        .all(|c| turns.contains(*c))
    {
        bail!(HISTORY_SCHEMA_UNSUPPORTED);
    }
    let mut statement = transaction.prepare(
        "SELECT status FROM thread_turns WHERE thread_id = ?1
         ORDER BY rollout_ordinal DESC, turn_id DESC LIMIT 1",
    )?;
    // Buffer the bounded page so a failed read never yields partially updated statuses.
    let statuses = items
        .iter()
        .map(|item| {
            statement
                .query_row([item["id"].as_str().unwrap()], |row| {
                    row.get::<_, String>(0)
                })
                .optional()
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (item, status) in items.iter_mut().zip(statuses) {
        item["status"] = json!(match status.as_deref() {
            Some(value @ ("inProgress" | "completed" | "failed" | "interrupted")) => value,
            _ => "unknown",
        });
    }
    Ok(())
}

fn read_database(path: &Path) -> anyhow::Result<Option<Vec<Value>>> {
    let mut db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    db.busy_timeout(Duration::from_millis(250))?;
    let transaction = db.transaction()?;
    let thread_columns = columns(&transaction, "threads")?;
    if thread_columns.is_empty() {
        return Ok(None);
    }
    let edge_columns = columns(&transaction, "thread_spawn_edges")?;
    if !thread_columns.contains("id")
        || !["child_thread_id", "parent_thread_id", "status"]
            .iter()
            .all(|c| edge_columns.contains(*c))
    {
        bail!(SCHEMA_UNSUPPORTED);
    }
    let updated = match (
        thread_columns.contains("updated_at_ms"),
        thread_columns.contains("updated_at"),
    ) {
        (true, true) => "COALESCE(t.updated_at_ms, t.updated_at * 1000, 0)",
        (true, false) => "COALESCE(t.updated_at_ms, 0)",
        (false, true) => "COALESCE(t.updated_at, 0) * 1000",
        _ => bail!(SCHEMA_UNSUPPORTED),
    };
    let nickname = if thread_columns.contains("agent_nickname") {
        "substr(t.agent_nickname, 1, 120)"
    } else {
        "NULL"
    };
    // Never read title, first_user_message, rollout paths, prompts or runtime configuration.
    let sql = format!(
        "WITH edges AS (
           SELECT child_thread_id, parent_thread_id, status,
                  ROW_NUMBER() OVER (PARTITION BY child_thread_id ORDER BY parent_thread_id, status) AS ordinal
           FROM thread_spawn_edges
           WHERE parent_thread_id IS NOT NULL AND parent_thread_id != '' AND child_thread_id != parent_thread_id
         )
         SELECT t.id, e.parent_thread_id, {nickname}, e.status, {updated} AS updated_ms
         FROM threads t JOIN edges e ON e.child_thread_id = t.id AND e.ordinal = 1
         ORDER BY updated_ms DESC, t.id, e.parent_thread_id LIMIT {RECENT_LIMIT}"
    );
    let mut statement = transaction.prepare(&sql)?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        let (id, parent, nickname, edge, updated) = row?;
        if id.is_empty() || id.len() > 240 || parent.len() > 240 || updated < 0 {
            bail!(SCHEMA_UNSUPPORTED);
        }
        let nickname = nickname
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| id.chars().take(8).collect());
        let edge_status = match edge.as_deref() {
            Some(value @ ("open" | "closed")) => value,
            _ => "unknown",
        };
        items.push(
            json!({"id":id, "parent_thread_id":parent, "agent_nickname":nickname,
            "status":"unknown", "edge_status":edge_status, "updated_at_ms":updated,
            "source":"codex_native", "read_only":true}),
        );
    }
    Ok(Some(items))
}

#[cfg(test)]
#[path = "native_subtasks_tests.rs"]
mod tests;
