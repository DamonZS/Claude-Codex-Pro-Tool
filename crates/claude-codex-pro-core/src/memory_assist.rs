use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: i64 = 1;
const EXPORT_SCHEMA_VERSION: &str = "memory-assist/v1";
const GLOBAL_WORKSPACE: &str = "global";
const ALL_WORKSPACES: &str = "__all__";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItem {
    pub id: String,
    pub text: String,
    pub workspace: String,
    pub category: String,
    pub tags: Vec<String>,
    pub source: String,
    pub source_session_id: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_accessed_at: i64,
    pub access_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCandidate {
    pub id: String,
    pub text: String,
    pub workspace: String,
    pub category: String,
    pub tags: Vec<String>,
    pub source: String,
    pub reason: String,
    pub source_session_id: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryQueryMatch {
    pub item: MemoryItem,
    pub score: f64,
    pub matched_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryQueryResult {
    pub query: String,
    pub workspace: String,
    pub results: Vec<MemoryQueryMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryWorkspaceSummary {
    pub workspace: String,
    pub item_count: i64,
    pub pending_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistStatus {
    pub status: String,
    pub db_path: String,
    pub total_items: i64,
    pub pending_candidates: i64,
    pub workspaces: Vec<MemoryWorkspaceSummary>,
    pub latest_backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySessionSummary {
    pub workspace: String,
    pub total_items: i64,
    pub pending_candidates: i64,
    pub injected_items: Vec<MemoryQueryMatch>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySelfCheckItem {
    pub name: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySelfCheckResult {
    pub status: String,
    pub repaired: bool,
    pub backup_path: Option<String>,
    pub checks: Vec<MemorySelfCheckItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryExport {
    pub schema_version: String,
    pub exported_at: i64,
    pub items: Vec<MemoryItem>,
    pub candidates: Vec<MemoryCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItemRequest {
    pub text: String,
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub source_session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCandidateRequest {
    pub text: String,
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub source_session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryQueryRequest {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub include_global: bool,
    #[serde(default = "default_query_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySessionRequest {
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_session_items")]
    pub max_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryImportRequest {
    pub data: MemoryExport,
    #[serde(default)]
    pub replace_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySelfCheckRequest {
    #[serde(default)]
    pub repair: bool,
}

#[derive(Debug, Clone)]
pub struct MemoryAssistStore {
    db_path: PathBuf,
}

impl Default for MemoryAssistStore {
    fn default() -> Self {
        Self::new(default_memory_assist_db_path())
    }
}

impl MemoryAssistStore {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn status(&self) -> anyhow::Result<MemoryAssistStatus> {
        let conn = self.open()?;
        let total_items = count_items(&conn)?;
        let pending_candidates = count_pending_candidates(&conn)?;
        let workspaces = workspace_summaries(&conn)?;
        let latest_backup_path = conn
            .query_row(
                "SELECT path FROM memory_backups ORDER BY created_at DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(MemoryAssistStatus {
            status: "ok".to_string(),
            db_path: self.db_path.to_string_lossy().to_string(),
            total_items,
            pending_candidates,
            workspaces,
            latest_backup_path,
        })
    }

    pub fn learn_item(&self, request: MemoryItemRequest) -> anyhow::Result<MemoryItem> {
        let conn = self.open()?;
        learn_item_with_conn(&conn, request)
    }

    pub fn update_item(&self, id: &str, request: MemoryItemRequest) -> anyhow::Result<MemoryItem> {
        let conn = self.open()?;
        let now = now_unix();
        let text = normalize_memory_text(&redact_secrets(&request.text));
        if text.is_empty() {
            anyhow::bail!("memory text is empty");
        }
        let workspace = normalize_workspace(&request.workspace);
        let category = normalize_label(&request.category, "general");
        let tags = normalize_tags(request.tags);
        let source_session_id = redact_secrets(&request.source_session_id);

        conn.execute(
            "UPDATE memory_items
             SET text = ?1, workspace = ?2, category = ?3, tags_json = ?4,
                 source = ?5, source_session_id = ?6, updated_at = ?7,
                 last_accessed_at = ?7, keywords = ?8
             WHERE id = ?9",
            params![
                text,
                workspace,
                category,
                serde_json::to_string(&tags)?,
                normalize_redacted_label(&request.source, "manual"),
                source_session_id,
                now,
                keywords_json(&keywords_for(&text)),
                id,
            ],
        )?;
        if conn.changes() == 0 {
            anyhow::bail!("memory item not found");
        }
        let item = item_by_id(&conn, id)?;
        record_event(
            &conn,
            "item_updated",
            Some(&item.id),
            None,
            &item.workspace,
            &json!({}),
        )?;
        Ok(item)
    }

    pub fn delete_item(&self, id: &str) -> anyhow::Result<MemoryItem> {
        let conn = self.open()?;
        let item = item_by_id(&conn, id)?;
        conn.execute("DELETE FROM memory_items WHERE id = ?1", [id])?;
        record_event(
            &conn,
            "item_deleted",
            Some(&item.id),
            None,
            &item.workspace,
            &json!({}),
        )?;
        Ok(item)
    }

    pub fn list_items(&self, request: MemoryQueryRequest) -> anyhow::Result<Vec<MemoryItem>> {
        let result = self.query_items(request, false)?;
        Ok(result.results.into_iter().map(|item| item.item).collect())
    }

    pub fn query(&self, request: MemoryQueryRequest) -> anyhow::Result<MemoryQueryResult> {
        self.query_items(request, true)
    }

    fn query_items(
        &self,
        request: MemoryQueryRequest,
        record_access: bool,
    ) -> anyhow::Result<MemoryQueryResult> {
        let conn = self.open()?;
        let workspace = normalize_workspace(&request.workspace);
        let limit = clamp_limit(request.limit);
        let scope = workspace_scope(&workspace, request.include_global);
        let all_workspaces = is_all_workspaces(&workspace);
        let mut items = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT id, text, workspace, category, tags_json, source, source_session_id,
                    created_at, updated_at, last_accessed_at, access_count
             FROM memory_items
             ORDER BY updated_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], row_to_item)?;
        for row in rows {
            let item = row?;
            if all_workspaces || scope.contains(&item.workspace.as_str()) {
                items.push(item);
            }
        }

        let query_keywords = keywords_for(&request.query);
        let mut matches = items
            .into_iter()
            .filter_map(|item| {
                let (score, matched_keywords) = score_item(&query_keywords, &request.query, &item);
                if request.query.trim().is_empty() || score > 0.0 {
                    Some(MemoryQueryMatch {
                        item,
                        score,
                        matched_keywords,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.item.access_count.cmp(&left.item.access_count))
                .then_with(|| right.item.updated_at.cmp(&left.item.updated_at))
        });
        matches.truncate(limit);

        if record_access {
            let now = now_unix();
            for item in &matches {
                conn.execute(
                    "UPDATE memory_items
                     SET access_count = access_count + 1, last_accessed_at = ?1
                     WHERE id = ?2",
                    params![now, item.item.id],
                )?;
            }
        }

        Ok(MemoryQueryResult {
            query: request.query,
            workspace,
            results: matches,
        })
    }

    pub fn create_candidate(
        &self,
        request: MemoryCandidateRequest,
    ) -> anyhow::Result<MemoryCandidate> {
        let conn = self.open()?;
        let now = now_unix();
        let text = normalize_memory_text(&redact_secrets(&request.text));
        if text.is_empty() {
            anyhow::bail!("candidate text is empty");
        }
        let workspace = normalize_workspace(&request.workspace);
        let category = normalize_label(&request.category, "general");
        let tags = normalize_tags(request.tags);
        let reason = redact_secrets(&request.reason);
        let source_session_id = redact_secrets(&request.source_session_id);
        if let Some(existing) = find_similar_candidate(&conn, &workspace, &text)? {
            let merged_tags = merge_tags(existing.tags.clone(), tags.clone());
            let next_text = if text.chars().count() >= existing.text.chars().count() {
                text
            } else {
                existing.text
            };
            let next_keywords = keywords_json(&keywords_for(&next_text));
            conn.execute(
                "UPDATE memory_candidates
                 SET text = ?1, category = ?2, tags_json = ?3, source = ?4,
                     reason = ?5, source_session_id = ?6, updated_at = ?7,
                     keywords = ?8
                 WHERE id = ?9",
                params![
                    next_text,
                    category,
                    serde_json::to_string(&merged_tags)?,
                    normalize_redacted_label(&request.source, "auto"),
                    reason,
                    source_session_id,
                    now,
                    next_keywords,
                    existing.id,
                ],
            )?;
            let candidate = candidate_by_id(&conn, &existing.id)?;
            record_event(
                &conn,
                "candidate_updated",
                None,
                Some(&candidate.id),
                &candidate.workspace,
                &json!({"source": candidate.source, "reason": candidate.reason}),
            )?;
            return Ok(candidate);
        }
        let id = stable_id("cand", &[&workspace, &text, &now.to_string()]);
        conn.execute(
            "INSERT INTO memory_candidates
             (id, text, workspace, category, tags_json, source, reason, source_session_id,
              status, created_at, updated_at, keywords)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9, ?9, ?10)",
            params![
                id,
                text,
                workspace,
                category,
                serde_json::to_string(&tags)?,
                normalize_redacted_label(&request.source, "auto"),
                reason,
                source_session_id,
                now,
                keywords_json(&keywords_for(&text)),
            ],
        )?;
        let candidate = candidate_by_id(&conn, &id)?;
        record_event(
            &conn,
            "candidate_created",
            None,
            Some(&candidate.id),
            &candidate.workspace,
            &json!({"source": candidate.source, "reason": candidate.reason}),
        )?;
        Ok(candidate)
    }

    pub fn list_candidates(
        &self,
        workspace: &str,
        include_global: bool,
    ) -> anyhow::Result<Vec<MemoryCandidate>> {
        let conn = self.open()?;
        let workspace = normalize_workspace(workspace);
        let scope = workspace_scope(&workspace, include_global);
        let all_workspaces = is_all_workspaces(&workspace);
        let mut stmt = conn.prepare(
            "SELECT id, text, workspace, category, tags_json, source, reason,
                    source_session_id, status, created_at, updated_at
             FROM memory_candidates
             WHERE status = 'pending'
             ORDER BY created_at DESC, id DESC",
        )?;
        let mut out = Vec::new();
        for row in stmt.query_map([], row_to_candidate)? {
            let candidate = row?;
            if all_workspaces || scope.contains(&candidate.workspace.as_str()) {
                out.push(candidate);
            }
        }
        Ok(out)
    }

    pub fn approve_candidate(&self, id: &str) -> anyhow::Result<MemoryItem> {
        let mut conn = self.open()?;
        let tx = conn.transaction()?;
        let candidate = candidate_by_id(&tx, id)?;
        if candidate.status != "pending" {
            anyhow::bail!("candidate is not pending");
        }
        let item = learn_item_with_conn(
            &tx,
            MemoryItemRequest {
                text: candidate.text,
                workspace: candidate.workspace,
                category: candidate.category,
                tags: candidate.tags,
                source: candidate.source,
                source_session_id: candidate.source_session_id,
            },
        )?;
        tx.execute(
            "UPDATE memory_candidates SET status = 'approved', updated_at = ?1 WHERE id = ?2",
            params![now_unix(), id],
        )?;
        record_event(
            &tx,
            "candidate_approved",
            Some(&item.id),
            Some(id),
            &item.workspace,
            &json!({}),
        )?;
        tx.commit()?;
        Ok(item)
    }

    pub fn reject_candidate(&self, id: &str) -> anyhow::Result<MemoryCandidate> {
        let conn = self.open()?;
        let existing = candidate_by_id(&conn, id)?;
        if existing.status != "pending" {
            anyhow::bail!("candidate is not pending");
        }
        let now = now_unix();
        conn.execute(
            "UPDATE memory_candidates SET status = 'rejected', updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        if conn.changes() == 0 {
            anyhow::bail!("candidate not found");
        }
        let candidate = candidate_by_id(&conn, id)?;
        record_event(
            &conn,
            "candidate_rejected",
            None,
            Some(&candidate.id),
            &candidate.workspace,
            &json!({}),
        )?;
        Ok(candidate)
    }

    pub fn session_summary(
        &self,
        request: MemorySessionRequest,
    ) -> anyhow::Result<MemorySessionSummary> {
        let workspace = normalize_workspace(&request.workspace);
        let max_items = clamp_limit(request.max_items);
        let status = self.status()?;
        let query = self.query(MemoryQueryRequest {
            query: request.query,
            workspace: workspace.clone(),
            include_global: true,
            limit: max_items,
        })?;
        let summary = if query.results.is_empty() {
            format!("记忆辅助已启用：{workspace} 暂无匹配记忆。")
        } else {
            let joined = query
                .results
                .iter()
                .map(|item| format!("{}: {}", item.item.category, item.item.text))
                .collect::<Vec<_>>()
                .join("；");
            format!(
                "记忆辅助已启用：{workspace} 命中 {} 条：{joined}",
                query.results.len()
            )
        };
        Ok(MemorySessionSummary {
            workspace,
            total_items: status.total_items,
            pending_candidates: status.pending_candidates,
            injected_items: query.results,
            summary,
        })
    }

    pub fn export_json(&self) -> anyhow::Result<MemoryExport> {
        let conn = self.open()?;
        let items = select_items(&conn)?;
        let candidates = select_candidates(&conn)?;
        Ok(MemoryExport {
            schema_version: EXPORT_SCHEMA_VERSION.to_string(),
            exported_at: now_unix(),
            items,
            candidates,
        })
    }

    pub fn import_json(&self, request: MemoryImportRequest) -> anyhow::Result<MemoryAssistStatus> {
        if request.data.schema_version != EXPORT_SCHEMA_VERSION {
            anyhow::bail!("unsupported memory export schema");
        }
        let mut conn = self.open()?;
        let tx = conn.transaction()?;
        if request.replace_existing {
            tx.execute("DELETE FROM memory_items", [])?;
            tx.execute("DELETE FROM memory_candidates", [])?;
        }
        for item in &request.data.items {
            let text = normalize_memory_text(&redact_secrets(&item.text));
            if text.is_empty() {
                continue;
            }
            let source_session_id = redact_secrets(&item.source_session_id);
            tx.execute(
                "INSERT OR REPLACE INTO memory_items
                 (id, text, workspace, category, tags_json, source, source_session_id,
                  created_at, updated_at, last_accessed_at, access_count, keywords)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    item.id,
                    text,
                    normalize_workspace(&item.workspace),
                    normalize_label(&item.category, "general"),
                    serde_json::to_string(&normalize_tags(item.tags.clone()))?,
                    normalize_redacted_label(&item.source, "import"),
                    source_session_id,
                    item.created_at,
                    item.updated_at,
                    item.last_accessed_at,
                    item.access_count,
                    keywords_json(&keywords_for(&text)),
                ],
            )?;
        }
        for candidate in &request.data.candidates {
            let text = normalize_memory_text(&redact_secrets(&candidate.text));
            if text.is_empty() {
                continue;
            }
            let reason = redact_secrets(&candidate.reason);
            let source_session_id = redact_secrets(&candidate.source_session_id);
            tx.execute(
                "INSERT OR REPLACE INTO memory_candidates
                 (id, text, workspace, category, tags_json, source, reason,
                  source_session_id, status, created_at, updated_at, keywords)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    candidate.id,
                    text,
                    normalize_workspace(&candidate.workspace),
                    normalize_label(&candidate.category, "general"),
                    serde_json::to_string(&normalize_tags(candidate.tags.clone()))?,
                    normalize_redacted_label(&candidate.source, "import"),
                    reason,
                    source_session_id,
                    normalize_candidate_status(&candidate.status),
                    candidate.created_at,
                    candidate.updated_at,
                    keywords_json(&keywords_for(&text)),
                ],
            )?;
        }
        tx.commit()?;
        self.status()
    }

    pub fn run_selfcheck(
        &self,
        request: MemorySelfCheckRequest,
    ) -> anyhow::Result<MemorySelfCheckResult> {
        let mut checks = Vec::new();
        {
            let conn = self.open()?;
            let user_version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
            checks.push(MemorySelfCheckItem {
                name: "schema".to_string(),
                status: if user_version == SCHEMA_VERSION {
                    "ok".to_string()
                } else {
                    "warning".to_string()
                },
                message: format!("schema version {user_version}"),
            });
            checks.push(MemorySelfCheckItem {
                name: "items".to_string(),
                status: "ok".to_string(),
                message: format!("{} memory items", count_items(&conn)?),
            });
            checks.push(MemorySelfCheckItem {
                name: "pending".to_string(),
                status: "ok".to_string(),
                message: format!("{} pending candidates", count_pending_candidates(&conn)?),
            });
            checks.push(MemorySelfCheckItem {
                name: "workspace".to_string(),
                status: "ok".to_string(),
                message: format!("{} workspaces", workspace_summaries(&conn)?.len()),
            });
        }

        let backup_path = if request.repair {
            Some(self.create_backup()?)
        } else {
            None
        };
        Ok(MemorySelfCheckResult {
            status: if checks.iter().any(|check| check.status == "failed") {
                "failed".to_string()
            } else {
                "ok".to_string()
            },
            repaired: request.repair,
            backup_path: backup_path.map(|path| path.to_string_lossy().to_string()),
            checks,
        })
    }

    fn open(&self) -> anyhow::Result<Connection> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create memory directory {}", parent.display()))?;
        }
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("open memory db {}", self.db_path.display()))?;
        ensure_schema(&conn)?;
        Ok(conn)
    }

    fn create_backup(&self) -> anyhow::Result<PathBuf> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let backup_dir = self
            .db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("memory_assist_backups");
        fs::create_dir_all(&backup_dir)?;
        let backup_path = backup_dir.join(format!(
            "memory_assist-{}-{}.sqlite",
            now_unix(),
            now_nanos()
        ));
        if self.db_path.exists() {
            fs::copy(&self.db_path, &backup_path)?;
        } else {
            fs::write(&backup_path, [])?;
        }
        let conn = self.open()?;
        conn.execute(
            "INSERT INTO memory_backups (id, path, created_at, item_count, candidate_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                stable_id("backup", &[backup_path.to_string_lossy().as_ref()]),
                backup_path.to_string_lossy(),
                now_unix(),
                count_items(&conn)?,
                count_candidates(&conn)?,
            ],
        )?;
        Ok(backup_path)
    }
}

pub fn default_memory_assist_db_path() -> PathBuf {
    if let Some(path) = memory_assist_db_path_for_tests() {
        return path;
    }
    crate::paths::default_app_state_dir().join("memory_assist.sqlite")
}

fn memory_assist_db_path_for_tests() -> Option<PathBuf> {
    MEMORY_ASSIST_DB_PATH_FOR_TESTS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|path| path.clone())
}

static MEMORY_ASSIST_DB_PATH_FOR_TESTS: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

pub fn set_memory_assist_db_path_for_tests(path: Option<PathBuf>) -> Option<PathBuf> {
    MEMORY_ASSIST_DB_PATH_FOR_TESTS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|mut current| std::mem::replace(&mut *current, path))
}

pub fn redact_secrets(input: &str) -> String {
    let with_sk = redact_prefixed_token(input, "sk-", "sk-***");
    redact_bearer_tokens(&with_sk)
}

fn learn_item_with_conn(
    conn: &Connection,
    request: MemoryItemRequest,
) -> anyhow::Result<MemoryItem> {
    let now = now_unix();
    let text = normalize_memory_text(&redact_secrets(&request.text));
    if text.is_empty() {
        anyhow::bail!("memory text is empty");
    }
    let workspace = normalize_workspace(&request.workspace);
    let category = normalize_label(&request.category, "general");
    let tags = normalize_tags(request.tags);
    let source_session_id = redact_secrets(&request.source_session_id);
    let keywords = keywords_json(&keywords_for(&text));

    if let Some(existing) = find_similar_item(conn, &workspace, &text)? {
        let merged_tags = merge_tags(existing.tags.clone(), tags.clone());
        let next_text = if text.chars().count() >= existing.text.chars().count() {
            text
        } else {
            existing.text
        };
        let next_keywords = keywords_json(&keywords_for(&next_text));
        conn.execute(
            "UPDATE memory_items
             SET text = ?1, category = ?2, tags_json = ?3, source = ?4,
                 source_session_id = ?5, updated_at = ?6, last_accessed_at = ?6,
                 keywords = ?7
             WHERE id = ?8",
            params![
                next_text,
                category,
                serde_json::to_string(&merged_tags)?,
                normalize_redacted_label(&request.source, "manual"),
                source_session_id,
                now,
                next_keywords,
                existing.id
            ],
        )?;
        let item = item_by_id(conn, &existing.id)?;
        record_event(
            conn,
            "item_updated",
            Some(&item.id),
            None,
            &item.workspace,
            &json!({"source": item.source}),
        )?;
        return Ok(item);
    }

    let id = stable_id("mem", &[&workspace, &text, &now.to_string()]);
    conn.execute(
        "INSERT INTO memory_items
         (id, text, workspace, category, tags_json, source, source_session_id,
          created_at, updated_at, last_accessed_at, access_count, keywords)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?8, 0, ?9)",
        params![
            id,
            text,
            workspace,
            category,
            serde_json::to_string(&tags)?,
            normalize_redacted_label(&request.source, "manual"),
            source_session_id,
            now,
            keywords
        ],
    )?;
    let item = item_by_id(conn, &id)?;
    record_event(
        conn,
        "item_created",
        Some(&item.id),
        None,
        &item.workspace,
        &json!({"source": item.source}),
    )?;
    Ok(item)
}

fn ensure_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        CREATE TABLE IF NOT EXISTS memory_items (
            id TEXT PRIMARY KEY,
            text TEXT NOT NULL,
            workspace TEXT NOT NULL,
            category TEXT NOT NULL,
            tags_json TEXT NOT NULL,
            source TEXT NOT NULL,
            source_session_id TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            last_accessed_at INTEGER NOT NULL,
            access_count INTEGER NOT NULL DEFAULT 0,
            keywords TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_memory_items_workspace ON memory_items(workspace);
        CREATE INDEX IF NOT EXISTS idx_memory_items_updated_at ON memory_items(updated_at);

        CREATE TABLE IF NOT EXISTS memory_candidates (
            id TEXT PRIMARY KEY,
            text TEXT NOT NULL,
            workspace TEXT NOT NULL,
            category TEXT NOT NULL,
            tags_json TEXT NOT NULL,
            source TEXT NOT NULL,
            reason TEXT NOT NULL,
            source_session_id TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            keywords TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_memory_candidates_workspace ON memory_candidates(workspace);
        CREATE INDEX IF NOT EXISTS idx_memory_candidates_status ON memory_candidates(status);

        CREATE TABLE IF NOT EXISTS memory_events (
            id TEXT PRIMARY KEY,
            item_id TEXT,
            candidate_id TEXT,
            event TEXT NOT NULL,
            workspace TEXT NOT NULL,
            detail_json TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_memory_events_workspace ON memory_events(workspace);

        CREATE TABLE IF NOT EXISTS memory_backups (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            item_count INTEGER NOT NULL,
            candidate_count INTEGER NOT NULL
        );
        PRAGMA user_version = 1;
        ",
    )?;
    Ok(())
}

fn item_by_id(conn: &Connection, id: &str) -> anyhow::Result<MemoryItem> {
    conn.query_row(
        "SELECT id, text, workspace, category, tags_json, source, source_session_id,
                created_at, updated_at, last_accessed_at, access_count
         FROM memory_items WHERE id = ?1",
        [id],
        row_to_item,
    )
    .with_context(|| format!("memory item not found: {id}"))
}

fn candidate_by_id(conn: &Connection, id: &str) -> anyhow::Result<MemoryCandidate> {
    conn.query_row(
        "SELECT id, text, workspace, category, tags_json, source, reason,
                source_session_id, status, created_at, updated_at
         FROM memory_candidates WHERE id = ?1",
        [id],
        row_to_candidate,
    )
    .with_context(|| format!("memory candidate not found: {id}"))
}

fn row_to_item(row: &Row<'_>) -> rusqlite::Result<MemoryItem> {
    let tags_json: String = row.get(4)?;
    Ok(MemoryItem {
        id: row.get(0)?,
        text: row.get(1)?,
        workspace: row.get(2)?,
        category: row.get(3)?,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        source: row.get(5)?,
        source_session_id: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        last_accessed_at: row.get(9)?,
        access_count: row.get(10)?,
    })
}

fn row_to_candidate(row: &Row<'_>) -> rusqlite::Result<MemoryCandidate> {
    let tags_json: String = row.get(4)?;
    Ok(MemoryCandidate {
        id: row.get(0)?,
        text: row.get(1)?,
        workspace: row.get(2)?,
        category: row.get(3)?,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        source: row.get(5)?,
        reason: row.get(6)?,
        source_session_id: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn select_items(conn: &Connection) -> anyhow::Result<Vec<MemoryItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, text, workspace, category, tags_json, source, source_session_id,
                created_at, updated_at, last_accessed_at, access_count
         FROM memory_items ORDER BY updated_at DESC, id DESC",
    )?;
    Ok(stmt
        .query_map([], row_to_item)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn select_candidates(conn: &Connection) -> anyhow::Result<Vec<MemoryCandidate>> {
    let mut stmt = conn.prepare(
        "SELECT id, text, workspace, category, tags_json, source, reason,
                source_session_id, status, created_at, updated_at
         FROM memory_candidates ORDER BY created_at DESC, id DESC",
    )?;
    Ok(stmt
        .query_map([], row_to_candidate)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn count_items(conn: &Connection) -> anyhow::Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM memory_items", [], |row| row.get(0))?)
}

fn count_candidates(conn: &Connection) -> anyhow::Result<i64> {
    Ok(
        conn.query_row("SELECT COUNT(*) FROM memory_candidates", [], |row| {
            row.get(0)
        })?,
    )
}

fn count_pending_candidates(conn: &Connection) -> anyhow::Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM memory_candidates WHERE status = 'pending'",
        [],
        |row| row.get(0),
    )?)
}

fn workspace_summaries(conn: &Connection) -> anyhow::Result<Vec<MemoryWorkspaceSummary>> {
    let mut item_counts = BTreeMap::<String, i64>::new();
    let mut stmt =
        conn.prepare("SELECT workspace, COUNT(*) FROM memory_items GROUP BY workspace")?;
    for row in stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })? {
        let (workspace, count) = row?;
        item_counts.insert(workspace, count);
    }
    let mut pending_counts = BTreeMap::<String, i64>::new();
    let mut stmt = conn.prepare(
        "SELECT workspace, COUNT(*) FROM memory_candidates
         WHERE status = 'pending'
         GROUP BY workspace",
    )?;
    for row in stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })? {
        let (workspace, count) = row?;
        pending_counts.insert(workspace, count);
    }
    let mut workspaces = item_counts
        .keys()
        .chain(pending_counts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    if workspaces.is_empty() {
        workspaces.insert(GLOBAL_WORKSPACE.to_string());
    }
    Ok(workspaces
        .into_iter()
        .map(|workspace| MemoryWorkspaceSummary {
            item_count: item_counts.get(&workspace).copied().unwrap_or(0),
            pending_count: pending_counts.get(&workspace).copied().unwrap_or(0),
            workspace,
        })
        .collect())
}

fn find_similar_item(
    conn: &Connection,
    workspace: &str,
    text: &str,
) -> anyhow::Result<Option<MemoryItem>> {
    let mut best: Option<(usize, MemoryItem)> = None;
    for item in select_items(conn)? {
        if item.workspace != workspace {
            continue;
        }
        let Some(score) = duplicate_memory_score(&item.text, text) else {
            continue;
        };
        if best
            .as_ref()
            .map(|(best_score, _)| score > *best_score)
            .unwrap_or(true)
        {
            best = Some((score, item));
        }
    }
    Ok(best.map(|(_, item)| item))
}

fn find_similar_candidate(
    conn: &Connection,
    workspace: &str,
    text: &str,
) -> anyhow::Result<Option<MemoryCandidate>> {
    let mut best: Option<(usize, MemoryCandidate)> = None;
    for candidate in select_candidates(conn)? {
        if candidate.workspace != workspace || candidate.status != "pending" {
            continue;
        }
        let Some(score) = duplicate_memory_score(&candidate.text, text) else {
            continue;
        };
        if best
            .as_ref()
            .map(|(best_score, _)| score > *best_score)
            .unwrap_or(true)
        {
            best = Some((score, candidate));
        }
    }
    Ok(best.map(|(_, candidate)| candidate))
}

fn duplicate_memory_score(existing: &str, incoming: &str) -> Option<usize> {
    let existing = normalize_memory_text(existing).to_ascii_lowercase();
    let incoming = normalize_memory_text(incoming).to_ascii_lowercase();
    if existing.is_empty() || incoming.is_empty() {
        return None;
    }
    if existing == incoming {
        return Some(usize::MAX);
    }
    if existing.contains(&incoming) || incoming.contains(&existing) {
        return Some(existing.len().min(incoming.len()));
    }
    let existing_keywords = keywords_for(&existing);
    let incoming_keywords = keywords_for(&incoming);
    let overlap = existing_keywords.intersection(&incoming_keywords).count();
    let min_len = existing_keywords.len().min(incoming_keywords.len());
    let max_len = existing_keywords.len().max(incoming_keywords.len());
    if min_len > 0 && max_len > 0 && overlap * 100 >= min_len * 86 && overlap * 100 >= max_len * 60
    {
        return Some(overlap);
    }
    None
}

fn score_item(
    query_keywords: &BTreeSet<String>,
    raw_query: &str,
    item: &MemoryItem,
) -> (f64, Vec<String>) {
    if raw_query.trim().is_empty() {
        return (1.0, Vec::new());
    }
    let item_keywords = keywords_for(&item.text);
    let matched = query_keywords
        .intersection(&item_keywords)
        .cloned()
        .collect::<Vec<_>>();
    let mut score = if query_keywords.is_empty() {
        0.0
    } else {
        matched.len() as f64 / query_keywords.len() as f64
    };
    if item.text.contains(raw_query.trim()) {
        score += 0.35;
    }
    if item.workspace == GLOBAL_WORKSPACE {
        score += 0.03;
    }
    (score, matched)
}

fn keywords_for(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut ascii = String::new();
    let mut cjk_run = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            if !cjk_run.is_empty() {
                push_cjk_keywords(&mut out, &cjk_run);
                cjk_run.clear();
            }
            ascii.push(ch.to_ascii_lowercase());
        } else if is_cjk(ch) {
            if ascii.len() >= 2 {
                out.insert(ascii.clone());
            }
            ascii.clear();
            cjk_run.push(ch);
        } else {
            if ascii.len() >= 2 {
                out.insert(ascii.clone());
            }
            ascii.clear();
            if !cjk_run.is_empty() {
                push_cjk_keywords(&mut out, &cjk_run);
                cjk_run.clear();
            }
        }
    }
    if ascii.len() >= 2 {
        out.insert(ascii);
    }
    if !cjk_run.is_empty() {
        push_cjk_keywords(&mut out, &cjk_run);
    }
    out
}

fn push_cjk_keywords(out: &mut BTreeSet<String>, run: &str) {
    let chars = run.chars().collect::<Vec<_>>();
    for ch in &chars {
        out.insert(ch.to_string());
    }
    for window in chars.windows(2) {
        out.insert(window.iter().collect::<String>());
    }
    for window in chars.windows(3) {
        out.insert(window.iter().collect::<String>());
    }
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
        || ('\u{3400}'..='\u{4dbf}').contains(&ch)
        || ('\u{f900}'..='\u{faff}').contains(&ch)
}

fn keywords_json(keywords: &BTreeSet<String>) -> String {
    serde_json::to_string(&keywords.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_else(|_| "[]".to_string())
}

fn record_event(
    conn: &Connection,
    event: &str,
    item_id: Option<&str>,
    candidate_id: Option<&str>,
    workspace: &str,
    detail: &serde_json::Value,
) -> anyhow::Result<()> {
    let now = now_unix();
    conn.execute(
        "INSERT INTO memory_events
         (id, item_id, candidate_id, event, workspace, detail_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            stable_id("evt", &[event, workspace, &now.to_string()]),
            item_id,
            candidate_id,
            event,
            workspace,
            redact_secrets(&detail.to_string()),
            now,
        ],
    )?;
    Ok(())
}

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(now_nanos().to_string().as_bytes());
    hasher.update(std::process::id().to_string().as_bytes());
    for part in parts {
        hasher.update([0]);
        hasher.update(part.as_bytes());
    }
    let digest = hasher.finalize();
    format!(
        "{prefix}_{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7]
    )
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn normalize_memory_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_workspace(workspace: &str) -> String {
    let workspace = workspace.trim();
    if workspace.is_empty() {
        GLOBAL_WORKSPACE.to_string()
    } else {
        workspace.to_string()
    }
}

fn normalize_label(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn normalize_redacted_label(value: &str, fallback: &str) -> String {
    normalize_label(&redact_secrets(value), fallback)
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut out = tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    out.truncate(24);
    out
}

fn merge_tags(left: Vec<String>, right: Vec<String>) -> Vec<String> {
    normalize_tags(left.into_iter().chain(right).collect())
}

fn normalize_candidate_status(status: &str) -> String {
    match status {
        "approved" | "rejected" => status.to_string(),
        _ => "pending".to_string(),
    }
}

fn workspace_scope(workspace: &str, include_global: bool) -> Vec<&str> {
    if include_global && workspace != GLOBAL_WORKSPACE {
        vec![workspace, GLOBAL_WORKSPACE]
    } else {
        vec![workspace]
    }
}

fn is_all_workspaces(workspace: &str) -> bool {
    workspace == ALL_WORKSPACES
}

fn clamp_limit(limit: usize) -> usize {
    limit.clamp(1, 100)
}

fn default_query_limit() -> usize {
    20
}

fn default_session_items() -> usize {
    5
}

fn redact_prefixed_token(input: &str, prefix: &str, replacement: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while let Some(relative) = input[i..].find(prefix) {
        let start = i + relative;
        out.push_str(&input[i..start]);
        let mut end = start + prefix.len();
        for (offset, ch) in input[end..].char_indices() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                end = start + prefix.len() + offset + ch.len_utf8();
            } else {
                break;
            }
        }
        out.push_str(replacement);
        i = end;
    }
    out.push_str(&input[i..]);
    out
}

fn redact_bearer_tokens(input: &str) -> String {
    const MARKER: &str = "bearer";
    const CANONICAL: &str = "Bearer ***";
    let lower = input.to_ascii_lowercase();
    let mut out = String::new();
    let mut i = 0;
    while let Some(relative) = lower[i..].find(MARKER) {
        let start = i + relative;
        let after_marker = start + MARKER.len();
        let Some(first_after) = input[after_marker..].chars().next() else {
            out.push_str(&input[i..]);
            return out;
        };
        if !first_after.is_whitespace() {
            out.push_str(&input[i..after_marker]);
            i = after_marker;
            continue;
        }
        out.push_str(&input[i..start]);
        let mut token_start = after_marker;
        for (offset, ch) in input[after_marker..].char_indices() {
            if ch.is_whitespace() {
                token_start = after_marker + offset + ch.len_utf8();
            } else {
                break;
            }
        }
        let mut end = token_start;
        for (offset, ch) in input[token_start..].char_indices() {
            if ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | ';') {
                break;
            }
            end = token_start + offset + ch.len_utf8();
        }
        if end == token_start {
            out.push_str(&input[start..token_start]);
        } else {
            out.push_str(CANONICAL);
        }
        i = end;
    }
    out.push_str(&input[i..]);
    out
}
