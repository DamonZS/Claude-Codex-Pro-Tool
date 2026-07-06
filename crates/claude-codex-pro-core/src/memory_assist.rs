use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: i64 = 4;
const EXPORT_SCHEMA_VERSION: &str = "memory-assist/v1";
const GLOBAL_WORKSPACE: &str = "global";
/// Tiering (phase 2). Items live in one of two tiers; archived is a soft,
/// recoverable visibility state, never a physical delete.
const TIER_ACTIVE: &str = "active";
const TIER_ARCHIVED: &str = "archived";
/// Ebbinghaus decay half-life in seconds (~30 days). retention decays as
/// strength * 0.5^(elapsed / HALF_LIFE) since the last access, so an untouched
/// item halves every ~30 days and crosses the archive threshold around ~90 days.
const DECAY_HALF_LIFE_SECS: f64 = 30.0 * 24.0 * 60.0 * 60.0;
/// Below this decayed retention an item is eligible for auto-archive (~12%,
/// i.e. roughly three half-lives / ~90 days without a hit).
const ARCHIVE_RETENTION_THRESHOLD: f64 = 0.12;
/// Each access adds this to the stored base strength (capped at STRENGTH_MAX),
/// so frequently-hit memories decay from a higher plateau.
const STRENGTH_ACCESS_BOOST: f64 = 0.25;
/// Upper bound on stored base strength so boosts can't grow unbounded.
const STRENGTH_MAX: f64 = 3.0;
/// Dimensionality of the local deterministic embedding. Feature-hashing maps any
/// text into this fixed-size dense vector; 256 keeps the BLOB small (256 * 4B =
/// 1KiB per item) while giving enough buckets to separate distinct vocabularies.
const LOCAL_EMBEDDING_DIM: usize = 256;
/// Model tag stored alongside each embedding so a future switch to a real
/// embedding model can detect and re-embed rows produced by this offline scheme.
const LOCAL_EMBEDDING_MODEL: &str = "local-hash-v1";
const ALL_WORKSPACES: &str = "__all__";
const LESSON_MANUAL_CATEGORY: &str = "lesson-manual";
const LESSON_MANUAL_SOURCE: &str = "lesson-manual-compiler";
const CAPTURE_USER_EVIDENCE_SQL: &str = "summary NOT LIKE '<environment_context%'
    AND summary NOT LIKE '<codex_internal_context%'
    AND summary NOT LIKE '<system%'
    AND summary NOT LIKE '<developer%'
    AND summary NOT LIKE '<image %'
    AND summary NOT LIKE '<attachment %'
    AND summary NOT LIKE 'Another language model started to solve this problem%'";

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
    /// Tiering (phase 2): "active" (default) or "archived". Archived items are a
    /// soft, recoverable visibility state — never physically deleted by decay.
    #[serde(default = "default_tier")]
    pub tier: String,
    /// Base retention strength, boosted on each access. Combined with the elapsed
    /// time since `last_accessed_at` it yields the decayed retention at read time.
    #[serde(default = "default_strength")]
    pub strength: f64,
    /// Unix seconds when the item was archived (0 = not archived). Kept for audit
    /// and restore.
    #[serde(default)]
    pub archived_at: i64,
    /// Computed at read time (not stored): Ebbinghaus-decayed retention in 0..1.
    /// Exempt items report 1.0. Drives the frontend strength bar.
    #[serde(default = "default_strength")]
    pub retention: f64,
    /// Computed at read time (not stored): true when the item is exempt from decay
    /// (manual source, safety-rule, or project-rule). Drives the "常驻" badge.
    #[serde(default)]
    pub exempt: bool,
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
pub struct MemoryCaptureRecord {
    pub id: String,
    pub workspace: String,
    pub source: String,
    pub source_session_id: String,
    pub text_length: i64,
    pub text_hash: String,
    pub summary: String,
    pub candidate_triggered: bool,
    pub candidate_reason: String,
    pub skip_reason: String,
    pub captured_at: i64,
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
    pub capture_count: i64,
    pub session_count: i64,
    pub latest_capture_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistStatus {
    pub status: String,
    pub db_path: String,
    pub inject_summary_cache_path: String,
    pub total_items: i64,
    pub pending_candidates: i64,
    pub total_captures: i64,
    pub workspaces: Vec<MemoryWorkspaceSummary>,
    pub latest_backup_path: Option<String>,
    pub enabled: bool,
    pub inject_enabled: bool,
    pub auto_suggest_enabled: bool,
    pub runtime_status: String,
    pub runtime_message: String,
    pub codex_injected: bool,
    pub claude_injected: bool,
    pub codex_workspace: String,
    pub active: bool,
    pub active_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySessionSummary {
    pub workspace: String,
    pub inject_summary_cache_path: String,
    pub total_items: i64,
    pub pending_candidates: i64,
    pub injected_items: Vec<MemoryQueryMatch>,
    pub recent_captures: Vec<MemoryCaptureRecord>,
    pub capture_summary: String,
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
pub struct MemoryCaptureRequest {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub source_session_id: String,
    #[serde(default)]
    pub candidate_triggered: bool,
    #[serde(default)]
    pub candidate_reason: String,
    #[serde(default)]
    pub skip_reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryHistoryCaptureReport {
    pub db_paths_checked: usize,
    pub rollout_files_checked: usize,
    pub user_messages_seen: usize,
    pub captures_recorded: usize,
    pub candidates_created: usize,
    pub items_learned: usize,
    pub errors: Vec<String>,
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
    /// Tiering (phase 2): when false (default) only `active` items are returned,
    /// so injection and normal search never surface faded-out memories. The
    /// manager's search can opt in to include the `archived` tier.
    #[serde(default)]
    pub include_archived: bool,
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

    pub fn inject_summary_cache_path(&self) -> PathBuf {
        self.db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("pangu_memory_inject.md")
    }

    pub fn status(&self) -> anyhow::Result<MemoryAssistStatus> {
        let codex_home = crate::codex_sqlite::default_codex_home_dir();
        self.status_from_codex_home(&codex_home)
    }

    pub fn status_from_codex_home(&self, codex_home: &Path) -> anyhow::Result<MemoryAssistStatus> {
        // The status panel polls this on a timer. Re-scanning every Codex SQLite
        // DB and rollout file on every poll was the dominant lag source, so only
        // backfill when a cheap metadata fingerprint shows the Codex history
        // actually changed since our last poll (keyed by memory-db path).
        let fingerprint = codex_history_fingerprint(codex_home);
        let should_backfill = {
            let cache = STATUS_BACKFILL_FINGERPRINT.get_or_init(|| Mutex::new(BTreeMap::new()));
            match cache.lock() {
                Ok(guard) => guard.get(&self.db_path).copied() != Some(fingerprint),
                // A poisoned lock should not silently disable backfill forever.
                Err(_) => true,
            }
        };
        if should_backfill {
            let _history_report = self.backfill_codex_history_from_home(codex_home, "", 50, false);
            if let Ok(mut guard) = STATUS_BACKFILL_FINGERPRINT
                .get_or_init(|| Mutex::new(BTreeMap::new()))
                .lock()
            {
                guard.insert(self.db_path.clone(), fingerprint);
            }
        }
        let conn = self.open()?;
        let total_items = count_items(&conn)?;
        let pending_candidates = count_pending_candidates(&conn)?;
        let total_captures = count_captures(&conn)?;
        let session_counts =
            codex_session_workspace_counts_from_home(codex_home, 500).unwrap_or_default();
        let workspaces = workspace_summaries(&conn, &session_counts)?;
        let _ = self.sync_inject_summary_cache(&conn, "");
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
            inject_summary_cache_path: self
                .inject_summary_cache_path()
                .to_string_lossy()
                .to_string(),
            total_items,
            pending_candidates,
            total_captures,
            workspaces,
            latest_backup_path,
            enabled: true,
            inject_enabled: true,
            auto_suggest_enabled: true,
            runtime_status: "not_checked".to_string(),
            runtime_message: "盘古记忆运行态尚未检测。".to_string(),
            codex_injected: false,
            claude_injected: false,
            codex_workspace: String::new(),
            active: false,
            active_source: "idle".to_string(),
        })
    }

    pub fn learn_item(&self, request: MemoryItemRequest) -> anyhow::Result<MemoryItem> {
        let conn = self.open()?;
        let item = learn_item_with_conn(&conn, request)?;
        let _ = self.sync_inject_summary_cache(&conn, &item.workspace);
        Ok(item)
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
        let _ = self.sync_inject_summary_cache(&conn, &item.workspace);
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
        let _ = self.sync_inject_summary_cache(&conn, &item.workspace);
        Ok(item)
    }

    /// Manually archive an item (phase 2): a soft, recoverable visibility state,
    /// never a physical delete. Injection and default search stop surfacing it.
    /// Idempotent — archiving an already-archived item is a no-op that still
    /// returns the current row.
    pub fn archive_item(&self, id: &str) -> anyhow::Result<MemoryItem> {
        let conn = self.open()?;
        let now = now_unix();
        conn.execute(
            "UPDATE memory_items SET tier = ?1, archived_at = ?2, updated_at = ?2
             WHERE id = ?3",
            params![TIER_ARCHIVED, now, id],
        )?;
        if conn.changes() == 0 {
            anyhow::bail!("memory item not found");
        }
        let mut item = item_by_id(&conn, id)?;
        decorate_item_decay(&mut item, now);
        record_event(
            &conn,
            "item_archived",
            Some(&item.id),
            None,
            &item.workspace,
            &json!({ "reason": "manual" }),
        )?;
        let _ = self.sync_inject_summary_cache(&conn, &item.workspace);
        Ok(item)
    }

    /// Restore an archived item back to the active tier (phase 2). Resets the
    /// decay clock and base strength so a deliberately-restored memory gets a
    /// fresh lease rather than immediately re-archiving on the next read.
    pub fn restore_item(&self, id: &str) -> anyhow::Result<MemoryItem> {
        let conn = self.open()?;
        let now = now_unix();
        conn.execute(
            "UPDATE memory_items
             SET tier = ?1, archived_at = 0, strength = 1.0,
                 last_accessed_at = ?2, updated_at = ?2
             WHERE id = ?3",
            params![TIER_ACTIVE, now, id],
        )?;
        if conn.changes() == 0 {
            anyhow::bail!("memory item not found");
        }
        let mut item = item_by_id(&conn, id)?;
        decorate_item_decay(&mut item, now);
        record_event(
            &conn,
            "item_restored",
            Some(&item.id),
            None,
            &item.workspace,
            &json!({ "reason": "manual" }),
        )?;
        let _ = self.sync_inject_summary_cache(&conn, &item.workspace);
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
                    created_at, updated_at, last_accessed_at, access_count,
                    tier, strength, archived_at
             FROM memory_items
             ORDER BY updated_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], row_to_item)?;
        let now = now_unix();
        // Read-time lazy auto-archive (phase 2): active, non-exempt items whose
        // decayed retention has fallen below the threshold are archived as a side
        // effect of this scan. We only collect ids here and apply the writes after
        // the read cursor (`stmt`) is dropped, to avoid writing to the same
        // connection while a query is still stepping.
        let mut to_auto_archive: Vec<(String, String)> = Vec::new();
        for row in rows {
            let mut item = row?;
            if !(all_workspaces || scope.contains(&item.workspace.as_str())) {
                continue;
            }
            // Tiering (phase 2): archived items are hidden unless explicitly asked
            // for, so injection and normal search never surface faded-out memories.
            if !request.include_archived && item.tier == TIER_ARCHIVED {
                continue;
            }
            // Fill read-time retention/exempt so ranking and the UI strength bar
            // have data without a stored column.
            decorate_item_decay(&mut item, now);
            // A still-active item that has decayed below the archive threshold is
            // marked for auto-archive. It stays visible in this response only when
            // the caller explicitly asked for archived items (manager search);
            // otherwise it is dropped so injection never surfaces a fading memory.
            if item.tier == TIER_ACTIVE
                && !item.exempt
                && item.retention < ARCHIVE_RETENTION_THRESHOLD
            {
                to_auto_archive.push((item.id.clone(), item.workspace.clone()));
                if !request.include_archived {
                    continue;
                }
                item.tier = TIER_ARCHIVED.to_string();
                item.archived_at = now;
            }
            items.push(item);
        }
        drop(stmt);
        for (id, workspace) in &to_auto_archive {
            conn.execute(
                "UPDATE memory_items SET tier = ?1, archived_at = ?2, updated_at = ?2
                 WHERE id = ?3 AND tier = ?4",
                params![TIER_ARCHIVED, now, id, TIER_ACTIVE],
            )?;
            record_event(
                &conn,
                "item_archived",
                Some(id),
                None,
                workspace,
                &json!({ "reason": "decay" }),
            )?;
        }

        // Full-text signal: FTS5 (trigram) gives a bm25 rank per item that
        // catches substring/token matches the in-memory keyword scan can miss.
        // It only activates for queries the trigram tokenizer can handle (>= 3
        // chars); shorter/CJK-2 queries still rely on the keyword score below.
        let fts_scores = fts_match_scores(&conn, &request.query).unwrap_or_default();
        // Semantic signal: cosine similarity over local deterministic embeddings.
        // Surfaces memories that share vocabulary/co-occurrence even when the
        // exact query tokens are absent. Backfills legacy rows in place.
        let vector_scores = vector_match_scores(&conn, &request.query).unwrap_or_default();

        let query_keywords = keywords_for(&request.query);
        let mut matches = items
            .into_iter()
            .filter_map(|item| {
                let (keyword_score, matched_keywords) =
                    score_item(&query_keywords, &request.query, &item);
                let fts_score = fts_scores.get(&item.id).copied().unwrap_or(0.0);
                let vector_score = vector_scores.get(&item.id).copied().unwrap_or(0.0);
                // Hybrid fusion: exact keyword score is the base; FTS bm25 and the
                // semantic cosine are added as complementary signals (each squashed
                // to 0..1) so neither exact-match nor semantic recall dominates.
                let score = keyword_score + fts_score * 0.5 + vector_score * 0.4;
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
                // Decayed retention breaks ties so fresher/stronger memories win
                // over faded ones at the same lexical+semantic score.
                .then_with(|| {
                    right
                        .item
                        .retention
                        .partial_cmp(&left.item.retention)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| right.item.access_count.cmp(&left.item.access_count))
                .then_with(|| right.item.updated_at.cmp(&left.item.updated_at))
        });
        matches.truncate(limit);

        if record_access {
            for item in &matches {
                // Access enhancement (phase 2): a hit resets the decay clock
                // (last_accessed_at = now) and boosts base strength (capped), so
                // frequently-used memories decay from a higher plateau. Exempt
                // items don't need the boost but the clock reset is harmless.
                conn.execute(
                    "UPDATE memory_items
                     SET access_count = access_count + 1,
                         last_accessed_at = ?1,
                         strength = MIN(strength + ?2, ?3)
                     WHERE id = ?4",
                    params![now, STRENGTH_ACCESS_BOOST, STRENGTH_MAX, item.item.id],
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
            let _ = self.sync_inject_summary_cache(&conn, &candidate.workspace);
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
        let _ = self.sync_inject_summary_cache(&conn, &candidate.workspace);
        Ok(candidate)
    }

    pub fn record_capture(
        &self,
        request: MemoryCaptureRequest,
    ) -> anyhow::Result<MemoryCaptureRecord> {
        let conn = self.open()?;
        let now = now_unix();
        let redacted = redact_secrets(&request.text);
        let normalized = normalize_memory_text(&redacted);
        let text_length = normalized.chars().count() as i64;
        if text_length == 0 {
            anyhow::bail!("capture text is empty");
        }
        let workspace = normalize_workspace(&request.workspace);
        let source = normalize_redacted_label(&request.source, "codex-capture");
        let source_session_id = redact_secrets(&request.source_session_id);
        let text_hash = capture_text_hash(&workspace, &normalized);
        let summary = capture_summary(&normalized);
        let candidate_reason = normalize_redacted_label(&request.candidate_reason, "");
        let skip_reason = normalize_redacted_label(&request.skip_reason, "");
        let id = stable_id("cap", &[&workspace, &text_hash]);
        conn.execute(
            "INSERT INTO memory_captures
             (id, workspace, source, source_session_id, text_length, text_hash, summary,
              candidate_triggered, candidate_reason, skip_reason, captured_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
             ON CONFLICT(workspace, text_hash) DO UPDATE SET
                source = excluded.source,
                source_session_id = excluded.source_session_id,
                text_length = excluded.text_length,
                summary = excluded.summary,
                candidate_triggered = excluded.candidate_triggered,
                candidate_reason = excluded.candidate_reason,
                skip_reason = excluded.skip_reason,
                updated_at = CASE
                    WHEN memory_captures.source <> excluded.source
                      OR memory_captures.source_session_id <> excluded.source_session_id
                      OR memory_captures.text_length <> excluded.text_length
                      OR memory_captures.summary <> excluded.summary
                      OR memory_captures.candidate_triggered <> excluded.candidate_triggered
                      OR memory_captures.candidate_reason <> excluded.candidate_reason
                      OR memory_captures.skip_reason <> excluded.skip_reason
                    THEN excluded.updated_at
                    ELSE memory_captures.updated_at
                END",
            params![
                id,
                workspace,
                source,
                source_session_id,
                text_length,
                text_hash,
                summary,
                if request.candidate_triggered { 1 } else { 0 },
                candidate_reason,
                skip_reason,
                now,
            ],
        )?;
        let capture = capture_by_workspace_hash(&conn, &workspace, &text_hash)?;
        record_event(
            &conn,
            "capture_recorded",
            None,
            None,
            &capture.workspace,
            &json!({
                "source": capture.source,
                "text_length": capture.text_length,
                "candidate_triggered": capture.candidate_triggered,
                "candidate_reason": capture.candidate_reason,
                "skip_reason": capture.skip_reason,
            }),
        )?;
        let _ = self.sync_inject_summary_cache(&conn, &capture.workspace);
        Ok(capture)
    }

    pub fn backfill_codex_history(
        &self,
        workspace_hint: &str,
        max_messages: usize,
        generate_candidates: bool,
    ) -> MemoryHistoryCaptureReport {
        let home = crate::codex_sqlite::default_codex_home_dir();
        self.backfill_codex_history_from_home(
            &home,
            workspace_hint,
            max_messages,
            generate_candidates,
        )
    }

    pub fn backfill_codex_history_from_home(
        &self,
        codex_home: &Path,
        workspace_hint: &str,
        max_messages: usize,
        generate_candidates: bool,
    ) -> MemoryHistoryCaptureReport {
        let mut report = MemoryHistoryCaptureReport::default();
        let limit = max_messages.max(1);
        let mut remaining = limit;
        for db_path in crate::codex_sqlite::codex_session_db_paths_from_home(codex_home) {
            if remaining == 0 {
                break;
            }
            if !db_path.is_file() {
                continue;
            }
            report.db_paths_checked += 1;
            let db = match Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            ) {
                Ok(db) => db,
                Err(error) => {
                    report.errors.push(format!(
                        "open codex db {} failed: {error}",
                        db_path.display()
                    ));
                    continue;
                }
            };
            let rows = match recent_codex_rollout_rows(&db, remaining) {
                Ok(rows) => rows,
                Err(error) => {
                    report.errors.push(format!(
                        "read codex db {} failed: {error}",
                        db_path.display()
                    ));
                    continue;
                }
            };
            for row in rows {
                if remaining == 0 {
                    break;
                }
                let rollout_path = PathBuf::from(&row.rollout_path);
                if !rollout_path.is_file() {
                    report.errors.push(format!(
                        "rollout file not found for {}: {}",
                        row.thread_id,
                        rollout_path.display()
                    ));
                    continue;
                }
                report.rollout_files_checked += 1;
                let messages = match read_codex_rollout_user_messages(&rollout_path, remaining) {
                    Ok(messages) => messages,
                    Err(error) => {
                        report.errors.push(format!(
                            "read rollout {} failed: {error}",
                            rollout_path.display()
                        ));
                        continue;
                    }
                };
                for text in messages {
                    if remaining == 0 {
                        break;
                    }
                    if !memory_capture_text_is_user_evidence(&text) {
                        continue;
                    }
                    remaining -= 1;
                    report.user_messages_seen += 1;
                    let workspace = if !row.cwd.trim().is_empty() {
                        row.cwd.as_str()
                    } else {
                        workspace_hint
                    };
                    let extraction = if generate_candidates {
                        extract_learnable_memory(&text)
                    } else {
                        None
                    };
                    let learn_result = extraction.as_ref().map(|memory| {
                        self.learn_item(MemoryItemRequest {
                            text: memory.text.clone(),
                            workspace: workspace.to_string(),
                            category: memory.category.clone(),
                            tags: memory.tags.clone(),
                            source: "codex-history-auto".to_string(),
                            source_session_id: row.thread_id.clone(),
                        })
                    });
                    let (candidate_triggered, candidate_reason, skip_reason) = match learn_result {
                        Some(Ok(_)) => {
                            report.items_learned += 1;
                            (
                                true,
                                extraction
                                    .as_ref()
                                    .map(|memory| format!("auto_learned: {}", memory.reason))
                                    .unwrap_or_default(),
                                String::new(),
                            )
                        }
                        Some(Err(error)) => (
                            false,
                            extraction
                                .as_ref()
                                .map(|memory| memory.reason.clone())
                                .unwrap_or_default(),
                            format!("learn_failed: {error}"),
                        ),
                        None => (false, String::new(), "history_not_learnable".to_string()),
                    };
                    match self.record_capture(MemoryCaptureRequest {
                        text,
                        workspace: workspace.to_string(),
                        source: "codex-history-rollout".to_string(),
                        source_session_id: row.thread_id.clone(),
                        candidate_triggered,
                        candidate_reason,
                        skip_reason,
                    }) {
                        Ok(_) => report.captures_recorded += 1,
                        Err(error) => report.errors.push(format!(
                            "record history capture for {} failed: {error}",
                            row.thread_id
                        )),
                    }
                }
            }
        }
        if generate_candidates && report.items_learned > 0 {
            match self.open().and_then(|mut conn| {
                let manual = consolidate_items_into_lesson_manual(&mut conn)?;
                let _ = self.sync_inject_summary_cache(&conn, "");
                Ok(manual)
            }) {
                Ok(Some(_)) => report.items_learned = 1,
                Ok(None) => {}
                Err(error) => report
                    .errors
                    .push(format!("compact learned items into manual failed: {error}")),
            }
        }
        report
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
        let conn = self.open()?;
        let _ = self.sync_inject_summary_cache(&conn, &item.workspace);
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
        let _ = self.sync_inject_summary_cache(&conn, &candidate.workspace);
        Ok(candidate)
    }

    pub fn session_summary(
        &self,
        request: MemorySessionRequest,
    ) -> anyhow::Result<MemorySessionSummary> {
        let workspace = normalize_workspace(&request.workspace);
        let max_items = clamp_limit(request.max_items);
        let history_report = self.backfill_codex_history(&workspace, 8, true);
        let status = self.status()?;
        let query = self.query(MemoryQueryRequest {
            query: request.query,
            workspace: workspace.clone(),
            include_global: true,
            limit: max_items,
            // Injection only ever surfaces active memories — decayed/archived
            // items must not leak back into the session-start summary.
            include_archived: false,
        })?;
        let conn = self.open()?;
        let _ = self.sync_inject_summary_cache(&conn, &workspace);
        let recent_captures = recent_captures(&conn, &workspace, 5)?;
        let capture_summary = summarize_recent_captures(&recent_captures);
        let learned_suffix = if history_report.items_learned > 0 {
            format!(
                "；本次启动从历史会话自动学习 {} 条",
                history_report.items_learned
            )
        } else {
            String::new()
        };
        let summary = if query.results.is_empty() {
            format!("盘古记忆已启用：{workspace} 暂无匹配记忆{learned_suffix}。")
        } else {
            let joined = query
                .results
                .iter()
                .map(|item| format!("{}: {}", item.item.category, item.item.text))
                .collect::<Vec<_>>()
                .join("；");
            format!(
                "盘古记忆已启用：{workspace} 命中 {} 条：{joined}{learned_suffix}",
                query.results.len()
            )
        };
        Ok(MemorySessionSummary {
            workspace,
            inject_summary_cache_path: self
                .inject_summary_cache_path()
                .to_string_lossy()
                .to_string(),
            total_items: status.total_items,
            pending_candidates: status.pending_candidates,
            injected_items: query.results,
            recent_captures,
            capture_summary,
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
        let conn = self.open()?;
        let _ = self.sync_inject_summary_cache(&conn, "");
        self.status()
    }

    pub fn run_selfcheck(
        &self,
        request: MemorySelfCheckRequest,
    ) -> anyhow::Result<MemorySelfCheckResult> {
        self.run_selfcheck_with_summaries(request, &BTreeMap::new())
    }

    /// Self-check + repair, optionally supplied with LLM-generated per-workspace
    /// summaries (phase 3 module C). The LLM call is async and privacy-sensitive
    /// (it ships memory text to the active relay), so it happens in the async
    /// command layer *before* this synchronous path; here we only pass the
    /// resolved text into consolidation, which falls back to the rule-based
    /// summarizer for any workspace missing from `summaries`.
    pub fn run_selfcheck_with_summaries(
        &self,
        request: MemorySelfCheckRequest,
        summaries: &BTreeMap<String, String>,
    ) -> anyhow::Result<MemorySelfCheckResult> {
        let mut checks = Vec::new();
        let history_report = self.backfill_codex_history("", usize::MAX, request.repair);
        {
            let mut conn = self.open()?;
            if request.repair {
                let _ = consolidate_items_with_summaries(&mut conn, summaries)?;
                let _ = self.sync_inject_summary_cache(&conn, "");
            }
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
                name: "history".to_string(),
                status: if history_report.errors.is_empty() {
                    "ok".to_string()
                } else if history_report.user_messages_seen > 0 {
                    "warning".to_string()
                } else {
                    "failed".to_string()
                },
                message: format!(
                    "{} dbs, {} rollout files, {} user messages, {} captures, {} learned items, {} candidates{}",
                    history_report.db_paths_checked,
                    history_report.rollout_files_checked,
                    history_report.user_messages_seen,
                    history_report.captures_recorded,
                    history_report.items_learned,
                    history_report.candidates_created,
                    if history_report.errors.is_empty() {
                        String::new()
                    } else {
                        format!(", errors: {}", history_report.errors.join(" | "))
                    }
                ),
            });
            checks.push(MemorySelfCheckItem {
                name: "capture".to_string(),
                status: "ok".to_string(),
                message: format!("{} captured user messages", count_captures(&conn)?),
            });
            let latest_capture = latest_capture(&conn)?;
            checks.push(MemorySelfCheckItem {
                name: "candidate".to_string(),
                status: if latest_capture
                    .as_ref()
                    .map(|capture| {
                        capture.candidate_triggered || !capture.skip_reason.trim().is_empty()
                    })
                    .unwrap_or(false)
                {
                    "ok".to_string()
                } else {
                    "warning".to_string()
                },
                message: latest_capture
                    .as_ref()
                    .map(|capture| {
                        if capture.candidate_triggered {
                            format!(
                                "latest capture triggered candidate: {}",
                                capture.candidate_reason
                            )
                        } else {
                            format!("latest capture skipped candidate: {}", capture.skip_reason)
                        }
                    })
                    .unwrap_or_else(|| "no captured user messages yet".to_string()),
            });
            checks.push(MemorySelfCheckItem {
                name: "database".to_string(),
                status: "ok".to_string(),
                message: format!(
                    "{} items, {} pending candidates, {} captures",
                    count_items(&conn)?,
                    count_pending_candidates(&conn)?,
                    count_captures(&conn)?
                ),
            });
            checks.push(MemorySelfCheckItem {
                name: "workspace".to_string(),
                status: "ok".to_string(),
                message: format!(
                    "{} workspaces",
                    workspace_summaries(&conn, &BTreeMap::new())?.len()
                ),
            });
            checks.push(MemorySelfCheckItem {
                name: "injection".to_string(),
                status: "warning".to_string(),
                message: "injection heartbeat is checked by manager runtime diagnostics"
                    .to_string(),
            });
            checks.push(MemorySelfCheckItem {
                name: "runtime".to_string(),
                status: "warning".to_string(),
                message: "renderer runtime snapshot is checked by manager status sync".to_string(),
            });
            checks.push(MemorySelfCheckItem {
                name: "manager".to_string(),
                status: "warning".to_string(),
                message: "manager status sync is checked by Tauri command layer".to_string(),
            });
        }

        let backup_path = if request.repair {
            Some(self.create_backup()?)
        } else {
            None
        };
        if let Ok(conn) = self.open() {
            let _ = self.sync_inject_summary_cache(&conn, "");
        }
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

    /// Collect the raw text that would be consolidated per workspace (phase 3
    /// module C). The async command layer calls this to build LLM summary prompts
    /// *before* the synchronous consolidation pass. Only workspaces with enough
    /// consolidatable source items are returned, using the same grouping rules as
    /// `consolidate_items_with_summaries` (active, non-exempt, non-summary sources,
    /// with any existing summary carried forward). Returns a map of workspace ->
    /// combined source text.
    pub fn collect_consolidation_inputs(&self) -> anyhow::Result<BTreeMap<String, String>> {
        let conn = self.open()?;
        let all = select_items(&conn)?;
        let mut existing_summary: BTreeMap<String, MemoryItem> = BTreeMap::new();
        let mut by_workspace: BTreeMap<String, Vec<MemoryItem>> = BTreeMap::new();
        for item in all {
            if item.category == LESSON_MANUAL_CATEGORY {
                existing_summary.insert(item.workspace.clone(), item);
                continue;
            }
            if item.tier != TIER_ACTIVE {
                continue;
            }
            if item_is_decay_exempt(&item.source, &item.category) {
                continue;
            }
            by_workspace
                .entry(item.workspace.clone())
                .or_default()
                .push(item);
        }
        let mut inputs = BTreeMap::new();
        for (workspace, sources) in by_workspace {
            if sources.len() < CONSOLIDATE_MIN_ITEMS {
                continue;
            }
            let mut lines = Vec::new();
            if let Some(prev) = existing_summary.get(&workspace) {
                lines.push(prev.text.clone());
            }
            for item in &sources {
                lines.push(item.text.clone());
            }
            inputs.insert(workspace, lines.join("\n"));
        }
        Ok(inputs)
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

    fn sync_inject_summary_cache(
        &self,
        conn: &Connection,
        workspace_hint: &str,
    ) -> anyhow::Result<PathBuf> {
        let path = self.inject_summary_cache_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = build_inject_summary_cache(conn, workspace_hint, &self.db_path)?;
        fs::write(&path, content)?;
        Ok(path)
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

/// Remembers the fingerprint of the Codex session store the last time a status
/// poll ran a backfill, keyed by memory-db path. The status panel polls every
/// few seconds; without this guard every poll re-scanned every Codex SQLite DB
/// and rollout file, which is the dominant "UI gets laggier the more history
/// you have" cost. We skip the scan when nothing changed.
static STATUS_BACKFILL_FINGERPRINT: OnceLock<Mutex<BTreeMap<PathBuf, u64>>> = OnceLock::new();

/// Cheap change-detector for the Codex session store: hashes only filesystem
/// metadata (path, length, mtime) of each session DB — never opens or reads
/// them. A changed fingerprint means a session DB was written since the last
/// poll, so new history may be worth backfilling.
fn codex_history_fingerprint(codex_home: &Path) -> u64 {
    let mut hasher = Sha256::new();
    for db_path in crate::codex_sqlite::codex_session_db_paths_from_home(codex_home) {
        let Ok(metadata) = std::fs::metadata(&db_path) else {
            continue;
        };
        hasher.update(db_path.to_string_lossy().as_bytes());
        hasher.update(metadata.len().to_le_bytes());
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.duration_since(UNIX_EPOCH) {
                hasher.update(elapsed.as_nanos().to_le_bytes());
            }
        }
    }
    let digest = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(bytes)
}

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
            keywords TEXT NOT NULL,
            embedding BLOB,
            embedding_model TEXT NOT NULL DEFAULT '',
            tier TEXT NOT NULL DEFAULT 'active',
            strength REAL NOT NULL DEFAULT 1.0,
            archived_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_memory_items_workspace ON memory_items(workspace);
        CREATE INDEX IF NOT EXISTS idx_memory_items_updated_at ON memory_items(updated_at);
        -- Note: the tier index is created in migrate_to_v4, not here, because for a
        -- migrating v2/v3 DB the tier column does not exist yet when this batch runs
        -- (CREATE TABLE IF NOT EXISTS is a no-op on the pre-existing table). A fresh
        -- DB still gets the index via migrate_to_v4 (user_version 0 < 4).

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

        CREATE TABLE IF NOT EXISTS memory_captures (
            id TEXT PRIMARY KEY,
            workspace TEXT NOT NULL,
            source TEXT NOT NULL,
            source_session_id TEXT NOT NULL,
            text_length INTEGER NOT NULL,
            text_hash TEXT NOT NULL,
            summary TEXT NOT NULL,
            candidate_triggered INTEGER NOT NULL DEFAULT 0,
            candidate_reason TEXT NOT NULL,
            skip_reason TEXT NOT NULL,
            captured_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(workspace, text_hash)
        );
        CREATE INDEX IF NOT EXISTS idx_memory_captures_workspace ON memory_captures(workspace);
        CREATE INDEX IF NOT EXISTS idx_memory_captures_updated_at ON memory_captures(updated_at);

        CREATE TABLE IF NOT EXISTS memory_backups (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            item_count INTEGER NOT NULL,
            candidate_count INTEGER NOT NULL
        );
        ",
    )?;

    // v3: full-text search + embedding columns. Migrate old v2 databases in place
    // (never drop existing memory_items / memory_candidates).
    let user_version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if user_version < 3 {
        migrate_to_v3(conn)?;
    }
    if user_version < 4 {
        migrate_to_v4(conn)?;
    }
    conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
    Ok(())
}

/// Returns true when `table` already has a column named `column`. Used to make
/// the v2 -> v3 `ALTER TABLE ADD COLUMN` steps idempotent (SQLite errors if the
/// column already exists, and a partially-migrated DB is possible after a crash).
fn column_exists(conn: &Connection, table: &str, column: &str) -> anyhow::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// v2 -> v3 migration: add embedding storage to `memory_items` and build the
/// FTS5 full-text index (trigram tokenizer, which handles both ASCII and CJK
/// substring search, unlike the old in-memory keyword-intersection scan). The
/// FTS content is kept in sync by triggers, and this backfills it from every
/// existing row so old memories become searchable immediately.
fn migrate_to_v3(conn: &Connection) -> anyhow::Result<()> {
    if !column_exists(conn, "memory_items", "embedding")? {
        conn.execute_batch("ALTER TABLE memory_items ADD COLUMN embedding BLOB;")?;
    }
    if !column_exists(conn, "memory_items", "embedding_model")? {
        conn.execute_batch(
            "ALTER TABLE memory_items ADD COLUMN embedding_model TEXT NOT NULL DEFAULT '';",
        )?;
    }
    conn.execute_batch(
        "
        CREATE VIRTUAL TABLE IF NOT EXISTS memory_items_fts USING fts5(
            item_id UNINDEXED,
            text,
            tokenize = 'trigram'
        );

        DROP TRIGGER IF EXISTS memory_items_fts_ai;
        DROP TRIGGER IF EXISTS memory_items_fts_ad;
        DROP TRIGGER IF EXISTS memory_items_fts_au;

        CREATE TRIGGER memory_items_fts_ai AFTER INSERT ON memory_items BEGIN
            INSERT INTO memory_items_fts(item_id, text) VALUES (new.id, new.text);
        END;
        CREATE TRIGGER memory_items_fts_ad AFTER DELETE ON memory_items BEGIN
            DELETE FROM memory_items_fts WHERE item_id = old.id;
        END;
        CREATE TRIGGER memory_items_fts_au AFTER UPDATE ON memory_items BEGIN
            DELETE FROM memory_items_fts WHERE item_id = old.id;
            INSERT INTO memory_items_fts(item_id, text) VALUES (new.id, new.text);
        END;

        DELETE FROM memory_items_fts;
        INSERT INTO memory_items_fts(item_id, text) SELECT id, text FROM memory_items;
        ",
    )?;
    Ok(())
}

/// v3 -> v4 migration: add the phase-2 tiering columns to `memory_items`. Every
/// existing row becomes an `active` item at the default strength (never archived
/// retroactively), so upgrading is non-destructive. Idempotent via `column_exists`
/// so a crash mid-migration can be re-run safely.
fn migrate_to_v4(conn: &Connection) -> anyhow::Result<()> {
    if !column_exists(conn, "memory_items", "tier")? {
        conn.execute_batch(&format!(
            "ALTER TABLE memory_items ADD COLUMN tier TEXT NOT NULL DEFAULT '{TIER_ACTIVE}';"
        ))?;
    }
    if !column_exists(conn, "memory_items", "strength")? {
        conn.execute_batch(
            "ALTER TABLE memory_items ADD COLUMN strength REAL NOT NULL DEFAULT 1.0;",
        )?;
    }
    if !column_exists(conn, "memory_items", "archived_at")? {
        conn.execute_batch(
            "ALTER TABLE memory_items ADD COLUMN archived_at INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_memory_items_tier ON memory_items(tier);")?;
    Ok(())
}

fn item_by_id(conn: &Connection, id: &str) -> anyhow::Result<MemoryItem> {
    conn.query_row(
        "SELECT id, text, workspace, category, tags_json, source, source_session_id,
                created_at, updated_at, last_accessed_at, access_count,
                tier, strength, archived_at
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
    // Columns 11..=13 (tier/strength/archived_at) are only present in SELECTs that
    // opt into them; read them defensively so the many 11-column SELECTs still work
    // and legacy rows fall back to sane defaults. retention/exempt are computed at
    // read time by `decorate_item_decay`, not stored — seed them with defaults here.
    let strength = row.get::<_, f64>(12).unwrap_or_else(|_| default_strength());
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
        tier: row.get::<_, String>(11).unwrap_or_else(|_| default_tier()),
        strength,
        archived_at: row.get::<_, i64>(13).unwrap_or(0),
        retention: strength,
        exempt: false,
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

fn row_to_capture(row: &Row<'_>) -> rusqlite::Result<MemoryCaptureRecord> {
    Ok(MemoryCaptureRecord {
        id: row.get(0)?,
        workspace: row.get(1)?,
        source: row.get(2)?,
        source_session_id: row.get(3)?,
        text_length: row.get(4)?,
        text_hash: row.get(5)?,
        summary: row.get(6)?,
        candidate_triggered: row.get::<_, i64>(7)? != 0,
        candidate_reason: row.get(8)?,
        skip_reason: row.get(9)?,
        captured_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn select_items(conn: &Connection) -> anyhow::Result<Vec<MemoryItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, text, workspace, category, tags_json, source, source_session_id,
                created_at, updated_at, last_accessed_at, access_count,
                tier, strength, archived_at
         FROM memory_items ORDER BY updated_at DESC, id DESC",
    )?;
    Ok(stmt
        .query_map([], row_to_item)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Minimum consolidatable source items in a workspace before we fold them into a
/// summary layer. Below this there is nothing worth compressing, so consolidation
/// is a no-op for that workspace.
const CONSOLIDATE_MIN_ITEMS: usize = 2;

fn consolidate_items_into_lesson_manual(
    conn: &mut Connection,
) -> anyhow::Result<Option<MemoryItem>> {
    consolidate_items_with_summaries(conn, &BTreeMap::new())
}

/// Core consolidation (phase 3). Replaces the old full-table `DELETE` rewrite:
/// for each workspace with enough consolidatable source items it archives those
/// sources (a soft, recoverable tier flip — never a physical delete) and upserts
/// a single summary item with a *deterministic* id keyed on (workspace, manual),
/// so repeated runs update the same row instead of spawning jittered duplicates.
///
/// `summaries` optionally supplies an LLM-generated summary text per workspace;
/// any workspace missing from the map (or with blank text) falls back to the
/// rule-based `build_lesson_manual_text`. The LLM call is async and must happen
/// *outside* this synchronous transaction — the caller resolves the text first
/// and passes it in here.
///
/// Consolidation only folds active, non-exempt, non-summary items. Exempt
/// memories (manual / safety-rule / project-rule) stay authoritative and are
/// never hidden behind a summary. Returns the representative summary (the
/// global-workspace one if present, else the first) to preserve the existing
/// single-item caller contract.
fn consolidate_items_with_summaries(
    conn: &mut Connection,
    summaries: &BTreeMap<String, String>,
) -> anyhow::Result<Option<MemoryItem>> {
    let all = select_items(conn)?;
    // Remember any existing summary per workspace so incremental runs carry old
    // bullets forward instead of dropping them when regenerating from only the
    // newly-accumulated sources.
    let mut existing_summary: BTreeMap<String, MemoryItem> = BTreeMap::new();
    let mut by_workspace: BTreeMap<String, Vec<MemoryItem>> = BTreeMap::new();
    for item in all {
        if item.category == LESSON_MANUAL_CATEGORY {
            existing_summary.insert(item.workspace.clone(), item);
            continue;
        }
        if item.tier != TIER_ACTIVE {
            continue;
        }
        if item_is_decay_exempt(&item.source, &item.category) {
            continue;
        }
        by_workspace
            .entry(item.workspace.clone())
            .or_default()
            .push(item);
    }

    let now = now_unix();
    let mut created: Vec<MemoryItem> = Vec::new();

    for (workspace, sources) in &by_workspace {
        if sources.len() < CONSOLIDATE_MIN_ITEMS {
            continue;
        }
        // Carry the existing summary forward so its distilled bullets survive an
        // incremental re-consolidation (the archived originals remain recoverable
        // regardless, but the live summary should not lose old lessons).
        let mut text_inputs: Vec<MemoryItem> = Vec::with_capacity(sources.len() + 1);
        if let Some(prev) = existing_summary.get(workspace) {
            text_inputs.push(prev.clone());
        }
        text_inputs.extend(sources.iter().cloned());

        let text = summaries
            .get(workspace)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| build_lesson_manual_text(&text_inputs));

        let id = deterministic_id("mem", &[workspace, LESSON_MANUAL_CATEGORY]);
        let tags = vec![
            "lesson".to_string(),
            "manual".to_string(),
            "pangu".to_string(),
        ];
        let source_ids: Vec<&str> = sources.iter().map(|item| item.id.as_str()).collect();

        // Archive-then-upsert must be atomic: a crash between them should leave
        // either the old active items or the finished summary, never a half state.
        // Because we archive (not delete), even a mid-run crash keeps every source
        // recoverable.
        let tx = conn.transaction()?;
        for src in sources {
            tx.execute(
                "UPDATE memory_items SET tier = ?1, archived_at = ?2, updated_at = ?2
                 WHERE id = ?3 AND tier = ?4",
                params![TIER_ARCHIVED, now, src.id, TIER_ACTIVE],
            )?;
        }
        // Deterministic-id upsert: first run inserts, later runs update the same
        // row in place (keeping it active), so the summary layer never duplicates.
        tx.execute(
            "INSERT INTO memory_items
             (id, text, workspace, category, tags_json, source, source_session_id,
              created_at, updated_at, last_accessed_at, access_count, keywords,
              tier, strength, archived_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?8, 0, ?9, ?10, 1.0, 0)
             ON CONFLICT(id) DO UPDATE SET
                 text = excluded.text,
                 tags_json = excluded.tags_json,
                 updated_at = excluded.updated_at,
                 last_accessed_at = excluded.updated_at,
                 keywords = excluded.keywords,
                 tier = excluded.tier,
                 archived_at = 0",
            params![
                id,
                text,
                workspace,
                LESSON_MANUAL_CATEGORY,
                serde_json::to_string(&tags)?,
                LESSON_MANUAL_SOURCE,
                "compiled",
                now,
                keywords_json(&keywords_for(&text)),
                TIER_ACTIVE,
            ],
        )?;
        let item = item_by_id(&tx, &id)?;
        record_event(
            &tx,
            "items_compacted_to_lesson_manual",
            Some(&item.id),
            None,
            &item.workspace,
            &json!({
                "sourceItems": sources.len(),
                "sourceIds": source_ids,
                "archived": true,
                "summarizer": if summaries.contains_key(workspace) { "llm" } else { "rule" },
            }),
        )?;
        tx.commit()?;
        created.push(item);
    }

    if created.is_empty() {
        return Ok(None);
    }
    let representative = created
        .iter()
        .find(|item| item.workspace == GLOBAL_WORKSPACE)
        .cloned()
        .unwrap_or_else(|| created[0].clone());
    Ok(Some(representative))
}

fn build_lesson_manual_text(items: &[MemoryItem]) -> String {
    let mut seen = BTreeSet::<String>::new();
    let mut lines = Vec::<(i32, String)>::new();

    for item in items {
        for candidate in lesson_manual_candidates(&item.text) {
            let line = compact_lesson_manual_line(&candidate);
            if line.chars().count() < 8 || lesson_manual_line_is_noise(&line) {
                continue;
            }
            let key = normalize_memory_text(&line).to_lowercase();
            if !seen.insert(key) {
                continue;
            }
            let mut score = lesson_sentence_score(&line);
            if item.category == "lesson-learned" || item.category == LESSON_MANUAL_CATEGORY {
                score += 4;
            }
            if item.category.contains("rule") || item.category.contains("safety") {
                score += 2;
            }
            if !contains_any_case_insensitive(&line, LESSON_ACTION_WORDS) || score < 6 {
                continue;
            }
            lines.push((score, line));
        }
    }

    if lines.is_empty() {
        for item in items.iter().take(8) {
            let line = compact_lesson_manual_line(&item.text);
            if line.chars().count() >= 8 && !lesson_manual_line_is_noise(&line) {
                lines.push((0, line));
            }
        }
    }

    lines.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.chars().count().cmp(&right.1.chars().count()))
            .then_with(|| left.1.cmp(&right.1))
    });

    let bullets = lines
        .into_iter()
        .map(|(_, line)| line)
        .take(10)
        .collect::<Vec<_>>();
    if bullets.is_empty() {
        "经验教训手册：\n- 暂无可沉淀的经验教训。".to_string()
    } else {
        format!(
            "经验教训手册：\n{}",
            bullets
                .into_iter()
                .map(|line| format!("- {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

fn lesson_manual_candidates(text: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            candidates.push(trimmed[2..].trim().to_string());
        }
    }
    candidates.extend(lesson_sentence_candidates(text));
    candidates
}

fn compact_lesson_manual_line(text: &str) -> String {
    let mut line = normalize_memory_text(&redact_secrets(text));
    for prefix in [
        "经验教训手册：",
        "经验教训:",
        "经验教训：",
        "经验：",
        "教训：",
        "lesson:",
        "Lesson:",
    ] {
        if let Some(rest) = line.strip_prefix(prefix) {
            line = rest.trim().to_string();
        }
    }
    line = line
        .trim_start_matches(|ch: char| {
            ch == '-' || ch == '*' || ch.is_ascii_digit() || ch == '.' || ch == '、'
        })
        .trim()
        .to_string();
    let max_chars = 110;
    if line.chars().count() > max_chars {
        let mut compact = line.chars().take(max_chars).collect::<String>();
        compact.push_str("...");
        compact
    } else {
        line
    }
}

fn lesson_manual_line_is_noise(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("</image>")
        || lower.contains("<image")
        || lower.contains("codex-clipboard")
        || lower.contains("验证结果")
        || lower.contains("cargo test")
        || lower.contains("npm --prefix")
        || lower.contains("git diff")
        || lower.contains("target\\debug")
        // A concrete filesystem path (drive-letter or POSIX absolute) is machine-
        // specific noise, not a reusable lesson. The old code hard-coded the
        // author's own `d:\project` / `c:\users`; detect the *shape* instead so it
        // generalizes to every user's machine.
        || contains_concrete_filesystem_path(&lower)
        || lower.contains("这个不需要")
        || lower.contains("为什么")
        || lower.contains("怎么")
        || lower.contains("什么")
}

/// Whether `lower` (already lowercased) contains a machine-specific absolute
/// path: a Windows drive path like `x:\...` or a deep POSIX path under a
/// well-known root (`/home/`, `/users/`, `/mnt/`, `/c/`). Kept deliberately
/// conservative so a lesson that merely mentions a relative path such as
/// `src/foo.rs` is not discarded.
fn contains_concrete_filesystem_path(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    for index in 0..bytes.len() {
        // Windows drive path: an ASCII letter followed by ":\" or ":/".
        if bytes[index] == b':'
            && index > 0
            && bytes[index - 1].is_ascii_alphabetic()
            && index + 1 < bytes.len()
            && (bytes[index + 1] == b'\\' || bytes[index + 1] == b'/')
        {
            // Require the letter to be a standalone drive (start of string or
            // preceded by a non-word byte) so "http://" style text is ignored.
            let preceding_is_word = index >= 2 && is_ascii_word_byte(bytes[index - 2]);
            if !preceding_is_word {
                return true;
            }
        }
    }
    for root in ["/home/", "/users/", "/mnt/", "/root/", "/var/", "/usr/"] {
        if lower.contains(root) {
            return true;
        }
    }
    false
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

fn capture_by_workspace_hash(
    conn: &Connection,
    workspace: &str,
    text_hash: &str,
) -> anyhow::Result<MemoryCaptureRecord> {
    conn.query_row(
        "SELECT id, workspace, source, source_session_id, text_length, text_hash,
                summary, candidate_triggered, candidate_reason, skip_reason,
                captured_at, updated_at
         FROM memory_captures WHERE workspace = ?1 AND text_hash = ?2",
        params![workspace, text_hash],
        row_to_capture,
    )
    .with_context(|| format!("memory capture not found: {workspace}/{text_hash}"))
}

fn latest_capture(conn: &Connection) -> anyhow::Result<Option<MemoryCaptureRecord>> {
    let sql = format!(
        "SELECT id, workspace, source, source_session_id, text_length, text_hash,
                summary, candidate_triggered, candidate_reason, skip_reason,
                captured_at, updated_at
         FROM memory_captures
         WHERE {CAPTURE_USER_EVIDENCE_SQL}
         ORDER BY updated_at DESC, id DESC LIMIT 1",
    );
    conn.query_row(&sql, [], row_to_capture)
        .optional()
        .map_err(Into::into)
}

fn recent_captures(
    conn: &Connection,
    workspace: &str,
    limit: usize,
) -> anyhow::Result<Vec<MemoryCaptureRecord>> {
    let workspace = normalize_workspace(workspace);
    let limit = clamp_limit(limit) as i64;
    let sql = format!(
        "SELECT id, workspace, source, source_session_id, text_length, text_hash,
                summary, candidate_triggered, candidate_reason, skip_reason,
                captured_at, updated_at
         FROM memory_captures
         WHERE (workspace = ?1 OR workspace = ?2)
           AND {CAPTURE_USER_EVIDENCE_SQL}
         ORDER BY updated_at DESC, id DESC
         LIMIT ?3",
    );
    let mut stmt = conn.prepare(&sql)?;
    Ok(stmt
        .query_map(params![workspace, GLOBAL_WORKSPACE, limit], row_to_capture)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

#[derive(Debug, Clone)]
struct CodexRolloutRow {
    thread_id: String,
    cwd: String,
    rollout_path: String,
}

fn codex_session_workspace_counts_from_home(
    codex_home: &Path,
    limit: usize,
) -> anyhow::Result<BTreeMap<String, i64>> {
    let mut counts = BTreeMap::<String, i64>::new();
    let limit = limit.clamp(1, 1000);
    for db_path in crate::codex_sqlite::codex_session_db_paths_from_home(codex_home) {
        if !db_path.is_file() {
            continue;
        }
        let db =
            match Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            {
                Ok(db) => db,
                Err(_) => continue,
            };
        for workspace in codex_thread_workspaces(&db, limit)? {
            *counts.entry(workspace).or_insert(0) += 1;
        }
        for workspace in codex_catalog_workspaces(&db, limit)? {
            *counts.entry(workspace).or_insert(0) += 1;
        }
    }
    Ok(counts)
}

fn codex_thread_workspaces(db: &Connection, limit: usize) -> anyhow::Result<Vec<String>> {
    if !sqlite_has_table(db, "threads")? {
        return Ok(Vec::new());
    }
    let columns = sqlite_table_columns(db, "threads")?;
    if !columns.contains(&"cwd".to_string()) {
        return Ok(Vec::new());
    }
    let updated = if columns.contains(&"updated_at_ms".to_string()) {
        "updated_at_ms"
    } else if columns.contains(&"updated_at".to_string()) {
        "updated_at * 1000"
    } else if columns.contains(&"created_at_ms".to_string()) {
        "created_at_ms"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT cwd FROM threads
         WHERE COALESCE(cwd, '') <> ''
         ORDER BY COALESCE({updated}, 0) DESC
         LIMIT ?1"
    );
    let mut stmt = db.prepare(&sql)?;
    let rows = stmt.query_map([sqlite_limit_arg(limit)], |row| {
        let workspace = normalize_workspace(&row.get::<_, Option<String>>(0)?.unwrap_or_default());
        Ok(workspace)
    })?;
    Ok(rows
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter(|workspace| !workspace.trim().is_empty())
        .collect())
}

fn codex_catalog_workspaces(db: &Connection, limit: usize) -> anyhow::Result<Vec<String>> {
    if !sqlite_has_table(db, "local_thread_catalog")? {
        return Ok(Vec::new());
    }
    let columns = sqlite_table_columns(db, "local_thread_catalog")?;
    if !columns.contains(&"path".to_string()) {
        return Ok(Vec::new());
    }
    let updated = if columns.contains(&"updated_at_ms".to_string()) {
        "updated_at_ms"
    } else if columns.contains(&"updated_at".to_string()) {
        "updated_at * 1000"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT path FROM local_thread_catalog
         WHERE COALESCE(path, '') <> ''
         ORDER BY COALESCE({updated}, 0) DESC
         LIMIT ?1"
    );
    let mut stmt = db.prepare(&sql)?;
    let rows = stmt.query_map([sqlite_limit_arg(limit)], |row| {
        let workspace = normalize_workspace(&row.get::<_, Option<String>>(0)?.unwrap_or_default());
        Ok(workspace)
    })?;
    Ok(rows
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter(|workspace| !workspace.trim().is_empty())
        .collect())
}

fn recent_codex_rollout_rows(
    db: &Connection,
    limit: usize,
) -> anyhow::Result<Vec<CodexRolloutRow>> {
    if !sqlite_has_table(db, "threads")? {
        return Ok(Vec::new());
    }
    let columns = sqlite_table_columns(db, "threads")?;
    if !columns.contains(&"id".to_string()) || !columns.contains(&"rollout_path".to_string()) {
        return Ok(Vec::new());
    }
    let cwd = sqlite_optional_column_expression(&columns, "cwd", "''");
    let updated = if columns.contains(&"updated_at_ms".to_string()) {
        "updated_at_ms"
    } else if columns.contains(&"updated_at".to_string()) {
        "updated_at * 1000"
    } else if columns.contains(&"created_at_ms".to_string()) {
        "created_at_ms"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT id, {cwd}, rollout_path FROM threads
         WHERE COALESCE(rollout_path, '') <> ''
         ORDER BY COALESCE({updated}, 0) DESC, id DESC
         LIMIT ?1"
    );
    let mut stmt = db.prepare(&sql)?;
    let rows = stmt.query_map([sqlite_limit_arg(limit)], |row| {
        Ok(CodexRolloutRow {
            thread_id: row.get::<_, String>(0)?,
            cwd: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            rollout_path: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn read_codex_rollout_user_messages(path: &Path, limit: usize) -> anyhow::Result<Vec<String>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if event.get("type").and_then(Value::as_str) != Some("response_item") {
            continue;
        }
        let payload = &event["payload"];
        if payload.get("type").and_then(Value::as_str) != Some("message")
            || payload.get("role").and_then(Value::as_str) != Some("user")
        {
            continue;
        }
        let body = codex_message_content_text(&payload["content"]);
        if body.trim().is_empty() {
            continue;
        }
        messages.push(body);
        if messages.len() >= limit {
            break;
        }
    }
    Ok(messages)
}

fn sqlite_limit_arg(limit: usize) -> i64 {
    i64::try_from(limit).unwrap_or(i64::MAX)
}

fn codex_message_content_text(content: &Value) -> String {
    let Some(items) = content.as_array() else {
        return String::new();
    };
    items
        .iter()
        .filter_map(|block| {
            let block_type = block.get("type").and_then(Value::as_str)?;
            match block_type {
                "input_text" => block
                    .get("text")
                    .and_then(Value::as_str)
                    .and_then(codex_visible_user_text_block),
                _ => None,
            }
        })
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn codex_visible_user_text_block(text: &str) -> Option<String> {
    let mut raw = text.trim().replace("\r\n", "\n").replace('\r', "\n");
    if raw.contains("## My request for Codex:") {
        if let Some((_, request)) = raw.split_once("## My request for Codex:") {
            raw = request.trim().to_string();
        }
    } else if raw.starts_with("# Context from my IDE setup:") {
        return None;
    }

    let filtered = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("# Files mentioned by the user:"))
        .filter(|line| !line.starts_with("## "))
        .filter(|line| !line.starts_with("<image "))
        .filter(|line| !line.starts_with("<attachment "))
        .collect::<Vec<_>>()
        .join("\n");
    let normalized = normalize_memory_text(&filtered);
    memory_capture_text_is_user_evidence(&normalized).then_some(normalized)
}

fn memory_capture_text_is_user_evidence(text: &str) -> bool {
    let normalized = normalize_memory_text(text);
    if normalized.is_empty() {
        return false;
    }
    let lower = normalized.to_ascii_lowercase();
    let internal_prefixes = [
        "<environment_context",
        "<codex_internal_context",
        "<system",
        "<developer",
        "<image ",
        "<attachment ",
        "another language model started to solve this problem",
    ];
    !internal_prefixes
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

#[derive(Debug, Clone)]
struct LearnableMemory {
    text: String,
    category: String,
    tags: Vec<String>,
    reason: String,
}

fn extract_learnable_memory(text: &str) -> Option<LearnableMemory> {
    let normalized = normalize_memory_text(&redact_secrets(text));
    let classification = classify_learnable_memory(&normalized)?;
    let text = build_learnable_memory_text(&normalized, &classification);
    Some(LearnableMemory {
        text,
        category: classification.category,
        tags: classification.tags,
        reason: classification.reason,
    })
}

#[derive(Debug, Clone)]
struct LearnableClassification {
    category: String,
    tags: Vec<String>,
    reason: String,
}

fn classify_learnable_memory(text: &str) -> Option<LearnableClassification> {
    let normalized = normalize_memory_text(text);
    if normalized.chars().count() < 16 {
        return None;
    }
    if looks_like_transient_command_output(&normalized) {
        return None;
    }

    let signals = learnable_signal_scores(&normalized);
    let confidence: i32 = signals.iter().map(|(_, score)| *score).sum();
    let has_lesson_signal = contains_any_case_insensitive(&normalized, LESSON_WORDS);
    let has_actionable_lesson =
        has_lesson_signal && contains_any_case_insensitive(&normalized, LESSON_ACTION_WORDS);
    if confidence < 3 {
        return None;
    }

    let has_scope = contains_any_case_insensitive(&normalized, PROJECT_CONTEXT_WORDS)
        || contains_any_case_insensitive(&normalized, WORKFLOW_CONTEXT_WORDS)
        || contains_any_case_insensitive(&normalized, PREFERENCE_CONTEXT_WORDS)
        || contains_any_case_insensitive(&normalized, LESSON_CONTEXT_WORDS);
    if !has_scope {
        return None;
    }
    if has_lesson_signal && !has_actionable_lesson {
        return None;
    }

    let (category, reason_base) = if contains_any_case_insensitive(&normalized, SAFETY_WORDS) {
        ("safety-rule", "history safety boundary")
    } else if has_actionable_lesson {
        ("lesson-learned", "history actionable lesson")
    } else if contains_any_case_insensitive(&normalized, PREFERENCE_CONTEXT_WORDS) {
        ("preference", "history user preference")
    } else if contains_any_case_insensitive(&normalized, WORKFLOW_WORDS)
        || contains_any_case_insensitive(&normalized, WORKFLOW_CONTEXT_WORDS)
    {
        ("workflow-rule", "history workflow rule")
    } else if contains_any_case_insensitive(&normalized, UI_WORDS) {
        ("ui-rule", "history ui requirement")
    } else {
        ("project-rule", "history project requirement")
    };

    let mut tags = vec![
        "history".to_string(),
        "codex".to_string(),
        "auto-learned".to_string(),
        category.to_string(),
    ];
    // Every fired signal becomes its own tag (multi-label): a "safety-related
    // workflow lesson" now carries both `safety` and `workflow`, so tag-based
    // queries surface it under either lens instead of a single hard category.
    for (tag, _) in signals {
        if !tags.iter().any(|existing| existing == tag) {
            tags.push(tag.to_string());
        }
    }
    // Record confidence as a sortable structured tag (not just buried in the free
    // text reason) so the UI can rank/filter candidates by strength without a
    // schema migration.
    tags.push(format!("confidence:{}", confidence_bucket(confidence)));

    Some(LearnableClassification {
        category: category.to_string(),
        tags,
        reason: format!("{reason_base}; confidence={confidence}"),
    })
}

/// Bucket a raw confidence score into a coarse, stable label the UI can sort on.
/// Kept coarse so small scoring tweaks don't churn the label.
fn confidence_bucket(confidence: i32) -> &'static str {
    if confidence >= 8 {
        "high"
    } else if confidence >= 5 {
        "medium"
    } else {
        "low"
    }
}

fn contains_any_case_insensitive(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_lowercase();
    needles
        .iter()
        .any(|needle| contains_needle(&lower, &needle.to_lowercase()))
}

/// Match `needle` inside the already-lowercased `haystack`. Pure-ASCII needles
/// (English keywords like "must"/"fix"/"keep") require ASCII word boundaries so
/// they no longer false-match inside longer words ("must" in "mustard", "fix"
/// in "prefix"). CJK/mixed needles keep plain substring matching because Chinese
/// has no whitespace word boundaries.
fn contains_needle(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    if !needle.is_ascii() {
        return haystack.contains(needle);
    }
    let bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    let mut search_from = 0;
    while let Some(offset) = haystack[search_from..].find(needle) {
        let start = search_from + offset;
        let end = start + needle_bytes.len();
        let before_ok = start == 0 || !is_ascii_word_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_ascii_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        // Advance one byte past this start so overlapping matches are still found.
        search_from = start + 1;
    }
    false
}

fn is_ascii_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn looks_like_transient_command_output(text: &str) -> bool {
    let lower = text.to_lowercase();
    let has_shell_prompt = lower.starts_with("ps ")
        || lower.contains("\nps ")
        || lower.contains("powershell")
        || lower.contains("cmd.exe")
        || lower.contains("$ git ")
        || lower.contains("> git ");
    let has_transient_error = lower.contains("unrecognized command or argument")
        || lower.contains("usage: git")
        || lower.contains("fatal: unable to access")
        || lower.contains("remote: permission to")
        || lower.contains("the requested url returned error")
        || lower.contains("error 403")
        || lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("拒绝访问");
    has_shell_prompt && has_transient_error
}

fn learnable_signal_scores(text: &str) -> Vec<(&'static str, i32)> {
    let mut signals = Vec::new();
    if contains_any_case_insensitive(text, LESSON_WORDS)
        && contains_any_case_insensitive(text, LESSON_ACTION_WORDS)
        && contains_any_case_insensitive(text, LESSON_CONTEXT_WORDS)
    {
        signals.push(("lesson", 3));
    }
    if contains_any_case_insensitive(text, RULE_WORDS) {
        signals.push(("rule", 2));
    }
    if contains_any_case_insensitive(text, PROJECT_CONTEXT_WORDS) {
        signals.push(("project", 2));
    }
    if contains_any_case_insensitive(text, WORKFLOW_CONTEXT_WORDS) {
        signals.push(("workflow", 2));
    }
    if contains_any_case_insensitive(text, UI_WORDS) {
        signals.push(("ui", 1));
    }
    if contains_any_case_insensitive(text, MEMORY_WORDS) {
        signals.push(("memory", 1));
    }
    if contains_any_case_insensitive(text, SAFETY_WORDS) {
        signals.push(("safety", 2));
    }
    if contains_any_case_insensitive(text, PREFERENCE_CONTEXT_WORDS) {
        signals.push(("preference", 2));
    }
    signals
}

const RULE_WORDS: &[&str] = &[
    "必须",
    "不要",
    "不能",
    "需要",
    "保持",
    "保留",
    "删除",
    "改成",
    "修复",
    "默认",
    "统一",
    "优先",
    "禁止",
    "避免",
    "先",
    "要有",
    "必须有",
    "需要有",
    "应该",
    "always",
    "never",
    "must",
    "should",
    "prefer",
    "default",
    "keep",
    "remove",
    "fix",
];

const PROJECT_CONTEXT_WORDS: &[&str] = &[
    "这个项目",
    "本项目",
    "当前项目",
    "这个仓库",
    "本仓库",
    "仓库",
    "项目",
    "codex",
    "claude",
    "manager",
    "盘古记忆",
];

const WORKFLOW_CONTEXT_WORDS: &[&str] = &[
    "构建",
    "测试",
    "验证",
    "提交",
    "spec",
    "acceptance",
    "agents.md",
    "workflow",
    "工作流",
    "规格",
    "验收",
];

const WORKFLOW_WORDS: &[&str] = &[
    "先读",
    "先写",
    "再开发",
    "验证后",
    "交付",
    "构建新版",
    "重新构建",
    "提交摘要",
];

const UI_WORDS: &[&str] = &[
    "UI",
    "界面",
    "前端",
    "布局",
    "样式",
    "主题",
    "按钮",
    "开关",
    "卡片",
    "页面",
    "供应商",
    "工具与插件",
];

const MEMORY_WORDS: &[&str] = &[
    "记忆",
    "盘古",
    "经验教训",
    "会话",
    "注入",
    "摘要",
    "采集",
    "监听",
];

const SAFETY_WORDS: &[&str] = &[
    "不能杀",
    "不要杀",
    "不能破坏",
    "不要删除",
    "不重置",
    "不执行",
    "不接入",
    "敏感",
    "api key",
    "bearer",
    "sk-",
];

const PREFERENCE_CONTEXT_WORDS: &[&str] = &[
    "我喜欢",
    "我偏好",
    "我习惯",
    "我的偏好",
    "按我",
    "以后",
    "注意",
    "记得",
];

const LESSON_WORDS: &[&str] = &[
    "经验",
    "教训",
    "踩坑",
    "复盘",
    "根因",
    "原因",
    "以后",
    "下次",
    "不要再",
    "不能再",
    "又",
    "仍然",
    "还是",
    "没变化",
    "没有变化",
    "没反馈",
    "没有反馈",
    "不生效",
    "失败",
    "错了",
    "不对",
    "看不到",
    "无法",
    "被占用",
    "regression",
    "lesson",
    "postmortem",
    "root cause",
    "next time",
];

const LESSON_ACTION_WORDS: &[&str] = &[
    "必须", "需要", "应该", "要有", "先", "再", "不要", "不能", "避免", "保留", "删除", "改成",
    "修复", "验证", "构建", "检查", "写入", "显示", "记录", "日志", "反馈", "must", "should",
    "need", "verify", "build", "log",
];

const LESSON_CONTEXT_WORDS: &[&str] = &[
    "盘古记忆",
    "经验教训",
    "记忆",
    "codex",
    "claude",
    "manager",
    "管理工具",
    "前端",
    "后端",
    "按钮",
    "开关",
    "构建",
    "验证",
    "日志",
    "反馈",
    "自检",
    "数据库",
    "sqlite",
    "会话",
    "工作区",
    "注入",
    "target",
];

fn build_learnable_memory_text(
    normalized: &str,
    classification: &LearnableClassification,
) -> String {
    let base = if classification.category == "lesson-learned" {
        extract_lesson_sentence(normalized)
    } else {
        normalized.to_string()
    };
    let text = if classification.category == "lesson-learned"
        && !base.starts_with("经验教训")
        && !base.starts_with("经验：")
        && !base.starts_with("教训：")
    {
        format!("经验教训：{base}")
    } else {
        base
    };
    text.chars().take(2000).collect()
}

fn extract_lesson_sentence(text: &str) -> String {
    let mut best = String::new();
    let mut best_score = i32::MIN;
    for segment in lesson_sentence_candidates(text) {
        let normalized = normalize_memory_text(&segment);
        if normalized.chars().count() < 8 {
            continue;
        }
        let score = lesson_sentence_score(&normalized);
        if score > best_score {
            best_score = score;
            best = normalized;
        }
    }
    if best.is_empty() {
        normalize_memory_text(text)
    } else {
        best
    }
}

fn lesson_sentence_candidates(text: &str) -> Vec<String> {
    text.split(|ch| matches!(ch, '\n' | '。' | '！' | '？' | ';' | '；'))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn lesson_sentence_score(segment: &str) -> i32 {
    let mut score = 0;
    if contains_any_case_insensitive(segment, LESSON_WORDS) {
        score += 4;
    }
    if contains_any_case_insensitive(segment, LESSON_ACTION_WORDS) {
        score += 3;
    }
    if contains_any_case_insensitive(segment, LESSON_CONTEXT_WORDS) {
        score += 3;
    }
    if contains_any_case_insensitive(segment, RULE_WORDS) {
        score += 2;
    }
    if contains_any_case_insensitive(segment, SAFETY_WORDS) {
        score += 2;
    }
    // A causal/contrast structure ("因为…所以…", "应该…而不是…", "否则…", "下次…")
    // carries the full actionable lesson, so it should win over a shorter
    // keyword-only fragment. Boost it and, when present, cancel the long-sentence
    // penalty so the complete instruction is preserved rather than truncated.
    let has_structure = has_causal_or_contrast_structure(segment);
    if has_structure {
        score += 3;
    }
    if segment.chars().count() > 180 && !has_structure {
        score -= 2;
    }
    score
}

/// Whether `segment` uses a causal or contrastive construction that ties a
/// recommendation to its rationale/alternative. These sentences state the
/// reusable lesson in full, so extraction should keep them intact.
fn has_causal_or_contrast_structure(segment: &str) -> bool {
    const CAUSAL_CONTRAST_MARKERS: &[&str] = &[
        "因为",
        "所以",
        "因此",
        "否则",
        "不然",
        "下次",
        "以后",
        "而不是",
        "而非",
        "应该",
        "应当",
        "本应",
        "instead of",
        "rather than",
        "so that",
        "because",
        "otherwise",
    ];
    contains_any_case_insensitive(segment, CAUSAL_CONTRAST_MARKERS)
}

fn sqlite_has_table(db: &Connection, table: &str) -> anyhow::Result<bool> {
    Ok(db
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            [table],
            |_| Ok(()),
        )
        .is_ok())
}

fn sqlite_table_columns(db: &Connection, table: &str) -> anyhow::Result<Vec<String>> {
    let mut stmt = db.prepare(&format!(
        "PRAGMA table_info(\"{}\")",
        table.replace('"', "\"\"")
    ))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn sqlite_optional_column_expression<'a>(
    columns: &[String],
    column: &'a str,
    fallback: &'a str,
) -> &'a str {
    if columns.iter().any(|existing| existing == column) {
        column
    } else {
        fallback
    }
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

fn count_captures(conn: &Connection) -> anyhow::Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM memory_captures WHERE {CAPTURE_USER_EVIDENCE_SQL}");
    Ok(conn.query_row(&sql, [], |row| row.get(0))?)
}

fn workspace_summaries(
    conn: &Connection,
    session_counts: &BTreeMap<String, i64>,
) -> anyhow::Result<Vec<MemoryWorkspaceSummary>> {
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
    let mut capture_counts = BTreeMap::<String, i64>::new();
    let mut latest_capture_at = BTreeMap::<String, i64>::new();
    let sql = format!(
        "SELECT workspace, COUNT(*), MAX(updated_at)
         FROM memory_captures
         WHERE {CAPTURE_USER_EVIDENCE_SQL}
         GROUP BY workspace",
    );
    let mut stmt = conn.prepare(&sql)?;
    for row in stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<i64>>(2)?.unwrap_or(0),
        ))
    })? {
        let (workspace, count, latest) = row?;
        capture_counts.insert(workspace.clone(), count);
        latest_capture_at.insert(workspace, latest);
    }
    let mut workspaces = item_counts
        .keys()
        .chain(pending_counts.keys())
        .chain(capture_counts.keys())
        .chain(session_counts.keys())
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
            capture_count: capture_counts.get(&workspace).copied().unwrap_or(0),
            session_count: session_counts.get(&workspace).copied().unwrap_or(0),
            latest_capture_at: latest_capture_at.get(&workspace).copied().unwrap_or(0),
            workspace,
        })
        .collect())
}

fn select_items_in_workspace(
    conn: &Connection,
    workspace: &str,
) -> anyhow::Result<Vec<MemoryItem>> {
    // Push the workspace filter into SQL so duplicate detection only loads rows
    // from the same workspace instead of the whole table on every insert (the
    // old full-table scan made bulk history backfill O(n^2)).
    let mut stmt = conn.prepare(
        "SELECT id, text, workspace, category, tags_json, source, source_session_id,
                created_at, updated_at, last_accessed_at, access_count,
                tier, strength, archived_at
         FROM memory_items WHERE workspace = ?1 ORDER BY updated_at DESC, id DESC",
    )?;
    Ok(stmt
        .query_map([workspace], row_to_item)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn find_similar_item(
    conn: &Connection,
    workspace: &str,
    text: &str,
) -> anyhow::Result<Option<MemoryItem>> {
    let mut best: Option<(usize, MemoryItem)> = None;
    for item in select_items_in_workspace(conn, workspace)? {
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

fn select_pending_candidates_in_workspace(
    conn: &Connection,
    workspace: &str,
) -> anyhow::Result<Vec<MemoryCandidate>> {
    // Same rationale as select_items_in_workspace: filter in SQL so we only load
    // the pending candidates from this workspace, not the whole table.
    let mut stmt = conn.prepare(
        "SELECT id, text, workspace, category, tags_json, source, reason,
                source_session_id, status, created_at, updated_at
         FROM memory_candidates
         WHERE workspace = ?1 AND status = 'pending'
         ORDER BY created_at DESC, id DESC",
    )?;
    Ok(stmt
        .query_map([workspace], row_to_candidate)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn find_similar_candidate(
    conn: &Connection,
    workspace: &str,
    text: &str,
) -> anyhow::Result<Option<MemoryCandidate>> {
    let mut best: Option<(usize, MemoryCandidate)> = None;
    for candidate in select_pending_candidates_in_workspace(conn, workspace)? {
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
    let simhash_distance = simhash_hamming_distance(&existing_keywords, &incoming_keywords);
    if simhash_distance <= 2 {
        return Some(existing_keywords.len().min(incoming_keywords.len()));
    }
    let overlap = existing_keywords.intersection(&incoming_keywords).count();
    let min_len = existing_keywords.len().min(incoming_keywords.len());
    let max_len = existing_keywords.len().max(incoming_keywords.len());
    if min_len > 0 && max_len > 0 && overlap * 100 >= min_len * 86 && overlap * 100 >= max_len * 60
    {
        return Some(overlap);
    }
    None
}

fn simhash_hamming_distance(left: &BTreeSet<String>, right: &BTreeSet<String>) -> u32 {
    if left.is_empty() || right.is_empty() {
        return u32::MAX;
    }
    (simhash64(left) ^ simhash64(right)).count_ones()
}

fn simhash64(tokens: &BTreeSet<String>) -> u64 {
    let mut weights = [0_i32; 64];
    for token in tokens {
        let hash = stable_token_hash64(token);
        for (bit, weight) in weights.iter_mut().enumerate() {
            if ((hash >> bit) & 1) == 1 {
                *weight += 1;
            } else {
                *weight -= 1;
            }
        }
    }
    weights
        .iter()
        .enumerate()
        .fold(0_u64, |acc, (bit, weight)| {
            if *weight >= 0 {
                acc | (1_u64 << bit)
            } else {
                acc
            }
        })
}

fn stable_token_hash64(token: &str) -> u64 {
    let digest = Sha256::digest(token.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(bytes)
}

/// Query the FTS5 trigram index and return a normalized (0..1) full-text score
/// per item id. The trigram tokenizer needs at least 3 characters, so shorter
/// queries return an empty map and the caller falls back to the keyword scan.
///
/// The raw query is wrapped as a single quoted FTS5 phrase (with embedded
/// double-quotes doubled) so user input containing FTS operators (`AND`, `*`,
/// `:`, `-`, ...) is treated as literal text rather than query syntax.
fn fts_match_scores(
    conn: &Connection,
    raw_query: &str,
) -> anyhow::Result<std::collections::HashMap<String, f64>> {
    let trimmed = raw_query.trim();
    // Trigram FTS matches on 3-char windows; anything shorter can't be indexed.
    if trimmed.chars().count() < 3 {
        return Ok(std::collections::HashMap::new());
    }
    let phrase = format!("\"{}\"", trimmed.replace('"', "\"\""));
    let mut stmt = conn.prepare(
        "SELECT item_id, bm25(memory_items_fts) AS rank
         FROM memory_items_fts
         WHERE memory_items_fts MATCH ?1",
    )?;
    let rows = stmt.query_map(params![phrase], |row| {
        let item_id: String = row.get(0)?;
        let rank: f64 = row.get(1)?;
        Ok((item_id, rank))
    });
    // A malformed MATCH expression (should not happen after quoting) must not
    // break querying — degrade to keyword-only rather than propagate the error.
    let rows = match rows {
        Ok(rows) => rows,
        Err(_) => return Ok(std::collections::HashMap::new()),
    };
    // bm25() returns a negative score where more-negative = more relevant.
    // Flip the sign and squash into 0..1 with x/(1+x) so it composes with the
    // keyword score without any single hit dominating.
    let mut scores = std::collections::HashMap::new();
    for row in rows {
        let Ok((item_id, rank)) = row else { continue };
        let relevance = (-rank).max(0.0);
        scores.insert(item_id, relevance / (1.0 + relevance));
    }
    Ok(scores)
}

/// Build a local, deterministic embedding for `text` using feature hashing over
/// the same keyword set the lexical layer produces (ASCII tokens + CJK n-grams).
/// Each keyword is hashed to a bucket and a sign, accumulated, then L2-normalized
/// so cosine similarity is a plain dot product. This is offline and dependency-
/// free — no network, no model download. It is not a learned semantic embedding,
/// but it captures token co-occurrence, so memories sharing vocabulary land close
/// together and complement the exact-match lexical/FTS signals. The stored BLOB
/// leaves room to swap in a real embedding model later (see [LOCAL_EMBEDDING_MODEL]).
fn local_embedding(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0_f32; LOCAL_EMBEDDING_DIM];
    for keyword in keywords_for(text) {
        let hash = stable_token_hash64(&keyword);
        let bucket = (hash % LOCAL_EMBEDDING_DIM as u64) as usize;
        // A second bit of the hash decides the sign so distinct tokens colliding
        // on the same bucket don't always reinforce each other.
        let sign = if (hash >> 63) & 1 == 1 { 1.0 } else { -1.0 };
        vector[bucket] += sign;
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

/// Serialize an embedding to a little-endian f32 BLOB for SQLite storage.
fn embedding_to_blob(vector: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(vector.len() * 4);
    for value in vector {
        blob.extend_from_slice(&value.to_le_bytes());
    }
    blob
}

/// Parse a little-endian f32 BLOB back into an embedding, or None if the byte
/// length is not a whole number of f32s (corrupt/legacy row → skip the vector
/// signal rather than error).
fn blob_to_embedding(blob: &[u8]) -> Option<Vec<f32>> {
    if blob.is_empty() || blob.len() % 4 != 0 {
        return None;
    }
    Some(
        blob.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect(),
    )
}

/// Cosine similarity of two already-L2-normalized vectors (a plain dot product).
/// Returns 0 for mismatched dimensions so a legacy/other-model row is ignored
/// rather than skewing the ranking.
fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() {
        return 0.0;
    }
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| a * b)
        .sum::<f32>()
        .clamp(-1.0, 1.0)
}

/// Read every item's stored embedding and score it against the query embedding.
/// Rows with a missing/empty/legacy embedding are backfilled in place so the
/// vector signal becomes complete over time without a blocking migration pass.
fn vector_match_scores(
    conn: &Connection,
    raw_query: &str,
) -> anyhow::Result<std::collections::HashMap<String, f64>> {
    let trimmed = raw_query.trim();
    if trimmed.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let query_vector = local_embedding(trimmed);
    let mut stmt = conn.prepare("SELECT id, text, embedding, embedding_model FROM memory_items")?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let text: String = row.get(1)?;
        let embedding: Option<Vec<u8>> = row.get(2)?;
        let model: String = row.get(3)?;
        Ok((id, text, embedding, model))
    })?;

    let mut scores = std::collections::HashMap::new();
    let mut backfill: Vec<(String, Vec<u8>)> = Vec::new();
    for row in rows {
        let Ok((id, text, embedding, model)) = row else {
            continue;
        };
        let vector = match embedding
            .as_deref()
            .filter(|_| model == LOCAL_EMBEDDING_MODEL)
            .and_then(blob_to_embedding)
        {
            Some(vector) => vector,
            None => {
                // Missing or produced by a different scheme: recompute with the
                // current local model and remember it for a single write pass.
                let vector = local_embedding(&text);
                backfill.push((id.clone(), embedding_to_blob(&vector)));
                vector
            }
        };
        let similarity = cosine_similarity(&query_vector, &vector);
        if similarity > 0.0 {
            scores.insert(id, similarity as f64);
        }
    }
    for (id, blob) in backfill {
        let _ = conn.execute(
            "UPDATE memory_items SET embedding = ?1, embedding_model = ?2 WHERE id = ?3",
            params![blob, LOCAL_EMBEDDING_MODEL, id],
        );
    }
    Ok(scores)
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
    let raw_query_lower = raw_query.trim().to_lowercase();
    if !raw_query_lower.is_empty() {
        if item.category.to_lowercase().contains(&raw_query_lower) {
            score += 0.12;
        }
        if item
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(&raw_query_lower))
        {
            score += 0.12;
        }
        if item
            .text
            .to_lowercase()
            .split(['。', '，', ',', '.', ';', '；', '\n'])
            .any(|part| part.trim() == raw_query_lower)
        {
            score += 0.18;
        }
    }
    score += (item.access_count.min(20) as f64) * 0.005;
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

fn capture_text_hash(workspace: &str, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(workspace.as_bytes());
    hasher.update([0]);
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .take(12)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn capture_summary(text: &str) -> String {
    let normalized = normalize_memory_text(&redact_secrets(text));
    let chars = normalized.chars().collect::<Vec<_>>();
    if chars.len() <= 160 {
        normalized
    } else {
        format!("{}…", chars.into_iter().take(160).collect::<String>())
    }
}

fn summarize_recent_captures(captures: &[MemoryCaptureRecord]) -> String {
    if captures.is_empty() {
        return "暂无最近用户消息采集。".to_string();
    }
    let latest = &captures[0];
    let reason = if latest.candidate_triggered {
        format!("候选：{}", latest.candidate_reason)
    } else if latest.skip_reason.trim().is_empty() {
        "未生成候选：原因待诊断".to_string()
    } else {
        format!("未生成候选：{}", latest.skip_reason)
    };
    format!(
        "最近采集 {} 条；最新 {} 字，{}。",
        captures.len(),
        latest.text_length,
        reason
    )
}

fn build_inject_summary_cache(
    conn: &Connection,
    workspace_hint: &str,
    db_path: &Path,
) -> anyhow::Result<String> {
    let workspace = normalize_workspace(workspace_hint);
    let total_items = count_items(conn)?;
    let pending_candidates = count_pending_candidates(conn)?;
    let total_captures = count_captures(conn)?;
    let captures = recent_captures(conn, &workspace, 5)?;
    let context_query = inject_context_query(&workspace, &captures);
    let items = ranked_items_for_inject_cache(conn, &workspace, &context_query, 8)?;
    let hot_items = high_frequency_items_for_inject_cache(conn, &workspace, 5)?;
    let mut lines = vec![
        "# 盘古记忆会话启动摘要".to_string(),
        String::new(),
        "> 该文件由 Claude Codex Pro Tool 自动生成。内容只来自 memory_assist.sqlite 与真实 Codex 会话采集结果。".to_string(),
        String::new(),
        format!("- 数据库: {}", db_path.display()),
        format!("- 工作区: {workspace}"),
        format!("- 经验教训: {total_items} 条"),
        format!("- 待确认候选: {pending_candidates} 条"),
        format!("- 采集证据: {total_captures} 条"),
        String::new(),
        "## 相关经验教训".to_string(),
    ];
    if items.is_empty() {
        lines.push("- 暂无可注入经验教训。".to_string());
    } else {
        for item in items {
            lines.push(format!(
                "- [{} | {}] {}",
                item.workspace, item.category, item.text
            ));
        }
    }
    lines.push(String::new());
    lines.push("## 高频经验教训".to_string());
    if hot_items.is_empty() {
        lines.push("- 暂无高频经验教训。".to_string());
    } else {
        for item in hot_items {
            lines.push(format!(
                "- [{} | 访问{}次 | {}] {}",
                item.workspace, item.access_count, item.category, item.text
            ));
        }
    }
    lines.push(String::new());
    lines.push("## 最近采集证据".to_string());
    if captures.is_empty() {
        lines.push("- 暂无最近用户消息采集。".to_string());
    } else {
        for capture in captures {
            let reason = if capture.candidate_triggered {
                format!("学习: {}", capture.candidate_reason)
            } else if capture.skip_reason.trim().is_empty() {
                "未学习: 原因待诊断".to_string()
            } else {
                format!("未学习: {}", capture.skip_reason)
            };
            lines.push(format!(
                "- [{} | {}字] {}；摘要: {}",
                capture.workspace, capture.text_length, reason, capture.summary
            ));
        }
    }
    lines.push(String::new());
    lines.push("## 使用规则".to_string());
    lines.push("- 新会话开始时应先读取本摘要，再结合当前用户请求回答。".to_string());
    lines.push(
        "- 本摘要不包含 API key、Bearer token、sk- 原文；如发现敏感信息应忽略并报告。".to_string(),
    );
    Ok(lines.join("\n"))
}

fn inject_context_query(workspace: &str, captures: &[MemoryCaptureRecord]) -> String {
    let mut parts = vec![workspace.to_string()];
    for capture in captures.iter().take(3) {
        parts.push(capture.summary.clone());
        if capture.candidate_triggered {
            parts.push(capture.candidate_reason.clone());
        } else {
            parts.push(capture.skip_reason.clone());
        }
    }
    normalize_memory_text(&parts.join("\n"))
}

fn ranked_items_for_inject_cache(
    conn: &Connection,
    workspace: &str,
    query: &str,
    limit: usize,
) -> anyhow::Result<Vec<MemoryItem>> {
    let workspace = normalize_workspace(workspace);
    let limit = clamp_limit(limit);
    let all_workspaces = is_all_workspaces(&workspace);
    let scope = workspace_scope(&workspace, true);
    let query_keywords = keywords_for(query);
    // Injection only surfaces active-tier memories: archived (faded-out) items
    // must never re-enter the session-start summary.
    let mut stmt = conn.prepare(
        "SELECT id, text, workspace, category, tags_json, source, source_session_id,
                created_at, updated_at, last_accessed_at, access_count
         FROM memory_items
         WHERE tier = 'active'
         ORDER BY updated_at DESC, access_count DESC, id DESC",
    )?;
    let mut matches = Vec::new();
    for row in stmt.query_map([], row_to_item)? {
        let item = row?;
        if all_workspaces || scope.contains(&item.workspace.as_str()) {
            let (score, _) = score_item(&query_keywords, query, &item);
            matches.push((score, item));
        }
    }
    matches.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.1.access_count.cmp(&left.1.access_count))
            .then_with(|| right.1.updated_at.cmp(&left.1.updated_at))
    });
    let items = matches
        .into_iter()
        .take(limit)
        .map(|(_, item)| item)
        .collect::<Vec<_>>();
    Ok(items)
}

fn high_frequency_items_for_inject_cache(
    conn: &Connection,
    workspace: &str,
    limit: usize,
) -> anyhow::Result<Vec<MemoryItem>> {
    let workspace = normalize_workspace(workspace);
    let limit = clamp_limit(limit);
    let all_workspaces = is_all_workspaces(&workspace);
    let scope = workspace_scope(&workspace, true);
    // Injection only surfaces active-tier memories (see ranked_items_for_inject_cache).
    let mut stmt = conn.prepare(
        "SELECT id, text, workspace, category, tags_json, source, source_session_id,
                created_at, updated_at, last_accessed_at, access_count
         FROM memory_items
         WHERE access_count > 0 AND tier = 'active'
         ORDER BY access_count DESC, last_accessed_at DESC, updated_at DESC, id DESC",
    )?;
    let mut items = Vec::new();
    for row in stmt.query_map([], row_to_item)? {
        let item = row?;
        if all_workspaces || scope.contains(&item.workspace.as_str()) {
            items.push(item);
        }
        if items.len() >= limit {
            break;
        }
    }
    Ok(items)
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

/// Deterministic id derived purely from `parts` — no time/pid entropy. Unlike
/// `stable_id`, calling this with the same parts always yields the same id, so a
/// consolidation summary layer keyed on (workspace, category) upserts the same
/// row across repeated runs instead of spawning a jittered duplicate each time
/// (the phase-3 fix for the old `now_nanos()`/pid id churn).
fn deterministic_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
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

/// True when an item is exempt from decay: user-fixed memories (`source=manual`)
/// and hard rules (`safety-rule` / `project-rule`) must never fade out of the
/// active tier on their own. Exempt items always report retention = 1.0.
fn item_is_decay_exempt(source: &str, category: &str) -> bool {
    source == "manual" || category == "safety-rule" || category == "project-rule"
}

/// Ebbinghaus-decayed retention in 0..1 for an item at time `now`. Exempt items
/// report 1.0. Otherwise retention = min(1, strength * 0.5^(elapsed/half_life)),
/// where elapsed is measured from `last_accessed_at` (each access resets the
/// clock and boosts `strength`, so frequently-hit memories decay from a higher
/// plateau). A missing/zero `last_accessed_at` falls back to `created_at`.
fn decayed_retention(
    source: &str,
    category: &str,
    strength: f64,
    last_accessed_at: i64,
    created_at: i64,
    now: i64,
) -> f64 {
    if item_is_decay_exempt(source, category) {
        return 1.0;
    }
    let anchor = if last_accessed_at > 0 {
        last_accessed_at
    } else {
        created_at
    };
    let elapsed = (now - anchor).max(0) as f64;
    let base = if strength > 0.0 { strength } else { 1.0 };
    let decayed = base * 0.5_f64.powf(elapsed / DECAY_HALF_LIFE_SECS);
    decayed.clamp(0.0, 1.0)
}

/// Fills the read-time `retention` and `exempt` fields on an item (they are not
/// stored columns). Call this before returning items to callers/UI so the
/// strength bar and "常驻" badge have data.
fn decorate_item_decay(item: &mut MemoryItem, now: i64) {
    item.exempt = item_is_decay_exempt(&item.source, &item.category);
    item.retention = decayed_retention(
        &item.source,
        &item.category,
        item.strength,
        item.last_accessed_at,
        item.created_at,
        now,
    );
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

fn default_tier() -> String {
    TIER_ACTIVE.to_string()
}

fn default_strength() -> f64 {
    1.0
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_keyword_matching_respects_word_boundaries() {
        // English keywords must not false-match inside longer words: the old
        // substring `contains` treated "mustard"/"prefix"/"keeper" as hits.
        assert!(!contains_needle("i love mustard sauce", "must"));
        assert!(!contains_needle("this is a prefix only", "fix"));
        assert!(!contains_needle("the keeper stood there", "keep"));
        // Genuine standalone words still match, case-insensitively upstream.
        assert!(contains_needle("you must rebuild first", "must"));
        assert!(contains_needle("please fix the bug", "fix"));
        // CJK needles keep substring matching (no whitespace word boundaries).
        assert!(contains_needle("以后必须重新构建", "必须"));
    }

    #[test]
    fn noise_filter_generalizes_machine_specific_paths() {
        // Concrete absolute paths are machine noise regardless of drive letter or
        // OS — the old code only knew the author's own d:\project / c:\users.
        assert!(lesson_manual_line_is_noise("see e:\\work\\build output"));
        assert!(lesson_manual_line_is_noise("logs under /home/alice/app"));
        assert!(lesson_manual_line_is_noise("check /mnt/data/cache"));
        // But a relative path in a genuine lesson must survive, and a URL must not
        // be mistaken for a drive path.
        assert!(!contains_concrete_filesystem_path(
            "edit src/foo.rs then rebuild"
        ));
        assert!(!contains_concrete_filesystem_path(
            "open https://example.com/docs"
        ));
    }

    #[test]
    fn causal_contrast_sentence_wins_over_keyword_only_fragment() {
        // A full causal/contrast instruction must outscore a shorter fragment that
        // merely trips a keyword, so extraction keeps the actionable sentence whole.
        let structured = "下次改完前端必须重新构建并验证，否则管理工具不会更新";
        let keyword_only = "构建";
        assert!(
            lesson_sentence_score(structured) > lesson_sentence_score(keyword_only),
            "structured lesson should score higher than a bare keyword fragment"
        );
        assert!(has_causal_or_contrast_structure(structured));
        assert!(has_causal_or_contrast_structure(
            "fix the flag instead of removing it"
        ));
        assert!(!has_causal_or_contrast_structure("rebuild the frontend"));
    }

    #[test]
    fn confidence_bucket_is_monotonic_and_tagged_on_classification() {
        // Buckets are coarse and ordered so small scoring tweaks don't churn them.
        assert_eq!(confidence_bucket(2), "low");
        assert_eq!(confidence_bucket(4), "low");
        assert_eq!(confidence_bucket(5), "medium");
        assert_eq!(confidence_bucket(7), "medium");
        assert_eq!(confidence_bucket(8), "high");
        assert_eq!(confidence_bucket(20), "high");
        // A classified lesson carries a sortable confidence tag, not just a
        // free-text reason, so the UI can rank candidates by strength.
        let classification =
            classify_learnable_memory("以后前端 UI 改动后必须重新构建并验证，否则管理工具不会更新")
                .expect("actionable lesson should classify");
        assert!(
            classification
                .tags
                .iter()
                .any(|tag| tag.starts_with("confidence:")),
            "classification must emit a confidence:<bucket> tag"
        );
    }

    #[test]
    fn simhash_similarity_is_stable_for_reordered_memory_rules() {
        let left = keywords_for(
            "project rule: always read spec acceptance and run verification before delivery",
        );
        let right = keywords_for(
            "before delivery always run verification and read acceptance spec project rule",
        );

        assert_eq!(simhash_hamming_distance(&left, &right), 0);
        assert!(
            duplicate_memory_score(
                "project rule: always read spec acceptance and run verification before delivery",
                "before delivery always run verification and read acceptance spec project rule",
            )
            .is_some()
        );
    }

    #[test]
    fn terminal_git_error_output_is_not_learnable_memory() {
        let text = "PS D:\\Project\\Claude-Codex-Pro-Tool> git credential-manager erase https://github.com Unrecognized command or argument 'https://github.com'. Description: [Git] Erase a stored credential Usage: git-credential-manager erase [options] Options: --no-ui Do not use graphical user interface prompts -?, -h, --help Show help and usage information PS D:\\Project\\Claude-Codex-Pro-Tool> git push -u origin main remote: Permission to DamonZS/Claude-Codex-Pro-Tool.git denied to DamonZS. fatal: unable to access 'https://github.com/DamonZS/Claude-Codex-Pro-Tool.git/': The requested URL returned error: 403";

        assert!(extract_learnable_memory(text).is_none());
    }

    #[test]
    fn actionable_feedback_is_extracted_as_lesson_memory() {
        let text = "你没有构建新版吗？以后前端 UI 改动后必须重新构建 target\\debug\\claude-codex-pro-manager.exe 并验证，否则管理工具应用里不会变化。";

        let memory = extract_learnable_memory(text).expect("actionable lesson should be learned");

        assert_eq!(memory.category, "lesson-learned");
        assert!(memory.text.starts_with("经验教训："));
        assert!(memory.text.contains("前端 UI 改动后必须重新构建"));
        assert!(memory.tags.iter().any(|tag| tag == "lesson"));
        assert!(memory.reason.contains("history actionable lesson"));
    }

    #[test]
    fn memory_refine_feedback_is_extracted_as_lesson_memory() {
        let text = "点击提炼经验教训后没有反馈。经验教训：盘古记忆按钮必须显示使用 Codex SQLite、会话文件和 memory_assist.sqlite，并在结束后输出遍历结果。";

        let memory =
            extract_learnable_memory(text).expect("memory refine lesson should be learned");

        assert_eq!(memory.category, "lesson-learned");
        assert!(memory.text.starts_with("经验教训："));
        assert!(memory.text.contains("盘古记忆按钮必须显示"));
        assert!(memory.text.contains("遍历结果"));
    }

    #[test]
    fn vague_complaint_without_actionable_context_is_not_learnable_memory() {
        let text = "太丑了，全都错了，还是不行。";

        assert!(extract_learnable_memory(text).is_none());
    }

    #[test]
    fn codex_history_fingerprint_is_stable_until_a_session_db_changes() {
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path();
        std::fs::create_dir_all(codex_home).unwrap();
        // The legacy state_5.sqlite path is fingerprinted purely by metadata (no
        // session-table validation), so it is the reliable fixture for asserting
        // that a size change flips the fingerprint.
        let db_path = codex_home.join("state_5.sqlite");
        std::fs::write(&db_path, b"first").unwrap();

        let first = codex_history_fingerprint(codex_home);
        // Unchanged files must yield the same fingerprint so status polling can
        // skip the expensive history backfill.
        assert_eq!(first, codex_history_fingerprint(codex_home));

        // Growing the DB (as Codex does when a session is written) must change
        // the fingerprint so the next poll re-backfills.
        std::fs::write(&db_path, b"first-plus-more-bytes").unwrap();
        assert_ne!(first, codex_history_fingerprint(codex_home));
    }

    #[test]
    fn codex_history_fingerprint_ignores_missing_home() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("does-not-exist");
        // No session DBs → a stable, well-defined fingerprint rather than a panic.
        assert_eq!(
            codex_history_fingerprint(&missing),
            codex_history_fingerprint(&missing)
        );
    }

    #[test]
    fn fts_index_retrieves_substring_matches_the_keyword_scan_would_miss() {
        // Regression guard for the v3 FTS5 layer: the trigram tokenizer must be
        // available in the bundled SQLite and must surface a substring match even
        // when the query is a fragment embedded inside a longer token — something
        // the word-boundary keyword scan cannot catch on its own.
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        store
            .learn_item(MemoryItemRequest {
                text: "部署脚本 deployment 走 kubernetes 集群".to_string(),
                workspace: "global".to_string(),
                category: "workflow-rule".to_string(),
                tags: vec![],
                source: "manual".to_string(),
                source_session_id: String::new(),
            })
            .expect("learn item");

        // "deploy" is a strict substring of "deployment"; the keyword scan treats
        // them as different tokens, so a hit here proves the FTS layer is live.
        let result = store
            .query(MemoryQueryRequest {
                query: "deploy".to_string(),
                workspace: "global".to_string(),
                include_global: true,
                include_archived: false,
                limit: 5,
            })
            .expect("query");
        assert!(
            result
                .results
                .iter()
                .any(|m| m.item.text.contains("deployment")),
            "FTS5 trigram search should retrieve the 'deployment' item for query 'deploy'"
        );
    }

    #[test]
    fn v2_database_migrates_to_v3_without_losing_items() {
        // A v2 DB (no embedding columns, no FTS table) must upgrade in place: keep
        // every existing memory_items row and become full-text searchable, never
        // drop data.
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("memory_assist.sqlite");
        {
            // Hand-build a minimal v2 schema + one row, exactly as the old code did.
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "
                CREATE TABLE memory_items (
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
                INSERT INTO memory_items VALUES
                    ('mem-legacy', '旧版长期记忆 legacy retrieval rule', 'global',
                     'general', '[]', 'manual', '', 100, 100, 100, 0, '[]');
                PRAGMA user_version = 2;
                ",
            )
            .unwrap();
        }

        // Opening through the store triggers ensure_schema → migrate_to_v3.
        let store = MemoryAssistStore::new(db_path.clone());
        let items = store
            .list_items(MemoryQueryRequest {
                query: String::new(),
                workspace: ALL_WORKSPACES.to_string(),
                include_global: true,
                include_archived: false,
                limit: 50,
            })
            .expect("list items after migration");
        assert!(
            items.iter().any(|item| item.id == "mem-legacy"),
            "v2 -> v3 migration must preserve existing memory_items"
        );

        // The migrated row must be reachable through the FTS-backed query too.
        let conn = Connection::open(&db_path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            version, SCHEMA_VERSION,
            "schema version should be bumped to v3"
        );
        let fts_count: i64 = conn
            .query_row("SELECT count(*) FROM memory_items_fts", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            fts_count, 1,
            "migration must backfill the FTS index from existing rows"
        );
    }

    #[test]
    fn local_embedding_is_deterministic_and_normalized() {
        // The offline embedding must be stable (same text → same vector) so stored
        // BLOBs stay comparable across runs, and L2-normalized so cosine is a plain
        // dot product bounded in [-1, 1].
        let a = local_embedding("以后改完前端必须重新构建并验证");
        let b = local_embedding("以后改完前端必须重新构建并验证");
        assert_eq!(a, b, "embedding must be deterministic for identical text");
        assert_eq!(a.len(), LOCAL_EMBEDDING_DIM);
        let norm = a.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-4,
            "embedding must be L2-normalized, got norm={norm}"
        );

        // A blob round-trips losslessly, and self-similarity is 1.
        let blob = embedding_to_blob(&a);
        let restored = blob_to_embedding(&blob).expect("blob round-trip");
        assert_eq!(a, restored);
        assert!((cosine_similarity(&a, &a) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn vector_signal_ranks_shared_vocabulary_above_unrelated_memory() {
        // The vector layer must rank a memory that shares vocabulary with the query
        // above an unrelated one, even when neither is an exact substring match —
        // this is the semantic-adjacency signal the pure keyword scan lacks.
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        for text in [
            "构建 前端 之后 必须 运行 验证 测试",
            "供应商 API key 切换 会 回滚 设置",
        ] {
            store
                .learn_item(MemoryItemRequest {
                    text: text.to_string(),
                    workspace: "global".to_string(),
                    category: "general".to_string(),
                    tags: vec![],
                    source: "manual".to_string(),
                    source_session_id: String::new(),
                })
                .expect("learn item");
        }

        let result = store
            .query(MemoryQueryRequest {
                query: "前端 构建 验证".to_string(),
                workspace: "global".to_string(),
                include_global: true,
                include_archived: false,
                limit: 5,
            })
            .expect("query");
        assert!(
            result
                .results
                .first()
                .map(|m| m.item.text.contains("验证"))
                .unwrap_or(false),
            "the vocabulary-sharing memory should rank first, got {:?}",
            result
                .results
                .iter()
                .map(|m| &m.item.text)
                .collect::<Vec<_>>()
        );

        // Backfill happened during the query: every row now carries a local embedding.
        let conn = Connection::open(store.db_path()).unwrap();
        let missing: i64 = conn
            .query_row(
                "SELECT count(*) FROM memory_items WHERE embedding IS NULL OR embedding_model != ?1",
                params![LOCAL_EMBEDDING_MODEL],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            missing, 0,
            "query must lazily backfill embeddings for all rows"
        );
    }

    #[test]
    fn decay_exempt_covers_manual_and_hard_rules_only() {
        // Exemption is the safety valve: user-fixed memories and hard rules must
        // never fade, but ordinary categories (including preference/lesson) decay.
        assert!(item_is_decay_exempt("manual", "general"));
        assert!(item_is_decay_exempt("codex-history-auto", "safety-rule"));
        assert!(item_is_decay_exempt("auto", "project-rule"));
        assert!(!item_is_decay_exempt("auto", "preference"));
        assert!(!item_is_decay_exempt(
            "codex-history-auto",
            "lesson-learned"
        ));
        assert!(!item_is_decay_exempt("auto", "workflow-rule"));
    }

    #[test]
    fn decayed_retention_halves_each_half_life_and_exempts_stay_full() {
        let now = 1_000_000_000_i64;
        let half = DECAY_HALF_LIFE_SECS as i64;
        // A fresh non-exempt item is at full retention.
        let fresh = decayed_retention("auto", "general", 1.0, now, now, now);
        assert!((fresh - 1.0).abs() < 1e-6, "fresh item should be ~1.0");
        // One half-life without a hit halves it.
        let one = decayed_retention("auto", "general", 1.0, now - half, now - half, now);
        assert!(
            (one - 0.5).abs() < 0.01,
            "one half-life should be ~0.5, got {one}"
        );
        // Exactly three half-lives is 0.125 — deliberately just *above* the 0.12
        // threshold, so a memory survives ~90 days before fading (matches the ADR).
        let three = decayed_retention("auto", "general", 1.0, now - 3 * half, now - 3 * half, now);
        assert!(
            three > ARCHIVE_RETENTION_THRESHOLD,
            "three half-lives ({three}) should still be above archive threshold {ARCHIVE_RETENTION_THRESHOLD}"
        );
        // A bit past three half-lives (~105 days) falls below the archive threshold.
        let past = decayed_retention(
            "auto",
            "general",
            1.0,
            now - 7 * half / 2,
            now - 7 * half / 2,
            now,
        );
        assert!(
            past < ARCHIVE_RETENTION_THRESHOLD,
            "3.5 half-lives ({past}) should be below archive threshold {ARCHIVE_RETENTION_THRESHOLD}"
        );
        // Exempt item ignores elapsed time entirely.
        let exempt = decayed_retention(
            "manual",
            "general",
            1.0,
            now - 10 * half,
            now - 10 * half,
            now,
        );
        assert!((exempt - 1.0).abs() < 1e-6, "exempt item stays at 1.0");
    }

    #[test]
    fn read_time_query_auto_archives_faded_non_exempt_items() {
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        // A learned auto item, then backdate its clock far past the archive
        // threshold so the next read should auto-archive it.
        let item = store
            .learn_item(MemoryItemRequest {
                text: "一条很久没有再被命中的自动记忆 stale auto memory".to_string(),
                workspace: "repo-x".to_string(),
                category: "general".to_string(),
                tags: vec![],
                source: "auto".to_string(),
                source_session_id: String::new(),
            })
            .expect("learn item");
        let ancient = now_unix() - (DECAY_HALF_LIFE_SECS as i64) * 6;
        {
            let conn = Connection::open(store.db_path()).unwrap();
            conn.execute(
                "UPDATE memory_items SET last_accessed_at = ?1, created_at = ?1, strength = 1.0 WHERE id = ?2",
                params![ancient, item.id],
            )
            .unwrap();
        }

        // A normal (active-only) query must not surface the faded item and must
        // archive it as a side effect.
        let visible = store
            .query(MemoryQueryRequest {
                query: String::new(),
                workspace: "repo-x".to_string(),
                include_global: true,
                include_archived: false,
                limit: 20,
            })
            .expect("query active");
        assert!(
            !visible.results.iter().any(|m| m.item.id == item.id),
            "faded item must not appear in an active-only query"
        );

        // It still exists, now in the archived tier, and is reachable when asked.
        let with_archived = store
            .list_items(MemoryQueryRequest {
                query: String::new(),
                workspace: "repo-x".to_string(),
                include_global: true,
                include_archived: true,
                limit: 20,
            })
            .expect("list with archived");
        let archived = with_archived
            .iter()
            .find(|it| it.id == item.id)
            .expect("archived item must still exist (never deleted)");
        assert_eq!(archived.tier, TIER_ARCHIVED);
    }

    #[test]
    fn exempt_item_is_never_auto_archived_however_old() {
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        let item = store
            .learn_item(MemoryItemRequest {
                text: "手动固化的项目铁律 permanent manual rule".to_string(),
                workspace: "repo-x".to_string(),
                category: "general".to_string(),
                tags: vec![],
                source: "manual".to_string(),
                source_session_id: String::new(),
            })
            .expect("learn item");
        let ancient = now_unix() - (DECAY_HALF_LIFE_SECS as i64) * 20;
        {
            let conn = Connection::open(store.db_path()).unwrap();
            conn.execute(
                "UPDATE memory_items SET last_accessed_at = ?1, created_at = ?1 WHERE id = ?2",
                params![ancient, item.id],
            )
            .unwrap();
        }
        let visible = store
            .query(MemoryQueryRequest {
                query: String::new(),
                workspace: "repo-x".to_string(),
                include_global: true,
                include_archived: false,
                limit: 20,
            })
            .expect("query active");
        let found = visible
            .results
            .iter()
            .find(|m| m.item.id == item.id)
            .expect("exempt manual item must stay active regardless of age");
        assert_eq!(found.item.tier, TIER_ACTIVE);
        assert!(found.item.exempt, "manual item must report exempt");
        assert!(
            (found.item.retention - 1.0).abs() < 1e-6,
            "exempt item reports full retention"
        );
    }

    #[test]
    fn manual_archive_and_restore_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        let item = store
            .learn_item(MemoryItemRequest {
                text: "可以被手动归档再恢复的记忆 archivable memory".to_string(),
                workspace: "repo-x".to_string(),
                category: "general".to_string(),
                tags: vec![],
                source: "auto".to_string(),
                source_session_id: String::new(),
            })
            .expect("learn item");

        let archived = store.archive_item(&item.id).expect("archive");
        assert_eq!(archived.tier, TIER_ARCHIVED);
        assert!(archived.archived_at > 0);

        // Active-only query hides it; include_archived surfaces it.
        let active_only = store
            .list_items(MemoryQueryRequest {
                query: String::new(),
                workspace: "repo-x".to_string(),
                include_global: true,
                include_archived: false,
                limit: 20,
            })
            .expect("active list");
        assert!(!active_only.iter().any(|it| it.id == item.id));

        let restored = store.restore_item(&item.id).expect("restore");
        assert_eq!(restored.tier, TIER_ACTIVE);
        assert_eq!(restored.archived_at, 0);

        let active_again = store
            .list_items(MemoryQueryRequest {
                query: String::new(),
                workspace: "repo-x".to_string(),
                include_global: true,
                include_archived: false,
                limit: 20,
            })
            .expect("active list after restore");
        assert!(active_again.iter().any(|it| it.id == item.id));
    }

    #[test]
    fn v3_database_migrates_to_v4_adding_tier_without_losing_items() {
        // A v3 DB (embeddings + FTS but no tier columns) must upgrade in place:
        // every row becomes an active item, none is dropped or archived.
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("memory_assist.sqlite");
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "
                CREATE TABLE memory_items (
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
                    keywords TEXT NOT NULL,
                    embedding BLOB,
                    embedding_model TEXT NOT NULL DEFAULT ''
                );
                INSERT INTO memory_items
                    (id, text, workspace, category, tags_json, source, source_session_id,
                     created_at, updated_at, last_accessed_at, access_count, keywords,
                     embedding_model)
                VALUES
                    ('mem-v3', 'v3 时代的长期记忆 legacy tiering rule', 'global',
                     'general', '[]', 'manual', '', 100, 100, 100, 0, '[]', '');
                PRAGMA user_version = 3;
                ",
            )
            .unwrap();
        }

        let store = MemoryAssistStore::new(db_path.clone());
        let items = store
            .list_items(MemoryQueryRequest {
                query: String::new(),
                workspace: ALL_WORKSPACES.to_string(),
                include_global: true,
                include_archived: true,
                limit: 50,
            })
            .expect("list after v4 migration");
        let migrated = items
            .iter()
            .find(|it| it.id == "mem-v3")
            .expect("v3 -> v4 migration must preserve existing rows");
        assert_eq!(
            migrated.tier, TIER_ACTIVE,
            "migrated rows default to active"
        );

        let conn = Connection::open(&db_path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION, "schema version should bump to v4");
    }

    #[test]
    fn deterministic_id_is_stable_across_calls() {
        // The consolidation summary id must be a pure function of its parts (no
        // now_nanos()/pid jitter), so repeated runs upsert the same row.
        let a = deterministic_id("mem", &["repo-a", LESSON_MANUAL_CATEGORY]);
        let b = deterministic_id("mem", &["repo-a", LESSON_MANUAL_CATEGORY]);
        assert_eq!(a, b, "same parts must yield the same id");
        let c = deterministic_id("mem", &["repo-b", LESSON_MANUAL_CATEGORY]);
        assert_ne!(a, c, "different workspace must yield a different id");
    }

    /// Learn `count` distinct auto memories into `workspace`, returning the store.
    /// Each memory uses genuinely different vocabulary so `find_similar_item` does
    /// not merge them into one row (which would defeat consolidation tests).
    fn seed_workspace(store: &MemoryAssistStore, workspace: &str, count: usize) {
        seed_workspace_from(store, workspace, 0, count);
    }

    /// Same as `seed_workspace` but starts numbering at `start` so a second call
    /// on the same workspace produces genuinely new rows instead of duplicates
    /// that `find_similar_item` would merge into (already-archived) originals.
    fn seed_workspace_from(store: &MemoryAssistStore, workspace: &str, start: usize, count: usize) {
        let distinct = [
            "改完前端界面后必须重新构建 debug manager 才能看到最新效果",
            "切换供应商配置以后要同步历史会话，否则旧线程读不到新 key",
            "发布打包前先把源码备份到外置磁盘，避免误删无法恢复",
            "插件中心安装脚本要先校验校验和，防止装到被篡改的第三方包",
            "自检修复会先创建备份再整合记忆，任何一步失败都能回滚",
            "注入脚本只在 Codex 页面生效，不要污染其他窗口的 DOM 结构",
        ];
        for idx in start..start + count {
            let base = distinct[idx % distinct.len()];
            store
                .learn_item(MemoryItemRequest {
                    text: format!("{base}（{workspace} 条目 {idx} 号补充说明）"),
                    workspace: workspace.to_string(),
                    category: "lesson-learned".to_string(),
                    tags: vec![],
                    source: "auto".to_string(),
                    source_session_id: String::new(),
                })
                .expect("learn item");
        }
    }

    #[test]
    fn consolidation_archives_sources_and_never_deletes_rows() {
        // Phase 3 core contract: consolidation must NOT run `DELETE FROM
        // memory_items`. Sources are archived (recoverable) and a summary layer is
        // upserted, so the total row count only grows.
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        seed_workspace(&store, "repo-a", 3);

        let mut conn = store.open().expect("open");
        let total_before: i64 = conn
            .query_row("SELECT count(*) FROM memory_items", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total_before, 3);

        let summary = consolidate_items_with_summaries(&mut conn, &BTreeMap::new())
            .expect("consolidate")
            .expect("a summary should be produced");
        assert_eq!(summary.category, LESSON_MANUAL_CATEGORY);
        assert_eq!(summary.tier, TIER_ACTIVE);
        assert_eq!(summary.workspace, "repo-a");

        // Nothing was deleted: 3 archived sources + 1 active summary = 4 rows.
        let total_after: i64 = conn
            .query_row("SELECT count(*) FROM memory_items", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            total_after, 4,
            "sources archived, summary added, none deleted"
        );
        let archived: i64 = conn
            .query_row(
                "SELECT count(*) FROM memory_items WHERE tier = ?1",
                params![TIER_ARCHIVED],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(archived, 3, "all three sources archived, still recoverable");
    }

    #[test]
    fn consolidation_reruns_with_stable_id_no_duplicate_summary() {
        // Re-running consolidation must update the same deterministic-id summary in
        // place, never spawn a second summary row for the workspace.
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        seed_workspace(&store, "repo-a", 3);
        let mut conn = store.open().expect("open");

        let first = consolidate_items_with_summaries(&mut conn, &BTreeMap::new())
            .expect("first consolidate")
            .expect("summary");
        // Add more sources, consolidate again. Start numbering past the first
        // batch so these are genuinely new active rows, not duplicates that
        // find_similar_item would merge into the (now archived) originals.
        drop(conn);
        seed_workspace_from(&store, "repo-a", 3, 2);
        let mut conn = store.open().expect("reopen");
        let second = consolidate_items_with_summaries(&mut conn, &BTreeMap::new())
            .expect("second consolidate")
            .expect("summary");
        assert_eq!(first.id, second.id, "summary id must be stable across runs");

        let summary_rows: i64 = conn
            .query_row(
                "SELECT count(*) FROM memory_items WHERE category = ?1 AND workspace = ?2",
                params![LESSON_MANUAL_CATEGORY, "repo-a"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(summary_rows, 1, "exactly one summary layer per workspace");
    }

    #[test]
    fn consolidation_prefers_supplied_llm_summary_and_falls_back_to_rules() {
        // With a supplied per-workspace summary the summary item carries that text
        // verbatim; without one it falls back to the rule-based bullet compiler.
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        seed_workspace(&store, "repo-a", 3);
        let mut conn = store.open().expect("open");

        let mut summaries = BTreeMap::new();
        summaries.insert(
            "repo-a".to_string(),
            "LLM 浓缩：始终先构建再验证。".to_string(),
        );
        let with_llm = consolidate_items_with_summaries(&mut conn, &summaries)
            .expect("consolidate")
            .expect("summary");
        assert!(
            with_llm.text.contains("LLM 浓缩"),
            "supplied LLM summary text must be used, got {:?}",
            with_llm.text
        );

        // After the first consolidate, repo-a's sources must be archived (0 active
        // non-summary rows), so the second call only touches the newly-seeded repo-b.
        let repo_a_active: i64 = conn
            .query_row(
                "SELECT count(*) FROM memory_items WHERE workspace = ?1 AND tier = ?2 AND category != ?3",
                params!["repo-a", TIER_ACTIVE, LESSON_MANUAL_CATEGORY],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            repo_a_active, 0,
            "repo-a sources should be archived after first consolidate"
        );

        // Re-seed a different workspace with no supplied summary → rule fallback.
        drop(conn);
        seed_workspace(&store, "repo-b", 3);
        let mut conn = store.open().expect("reopen");
        let rule = consolidate_items_with_summaries(&mut conn, &BTreeMap::new())
            .expect("consolidate")
            .expect("summary");
        // The fallback summary belongs to repo-b and is the rule-based bullet manual,
        // never the LLM sentinel text (repo-a is already archived, so it is not
        // re-consolidated).
        assert_eq!(rule.workspace, "repo-b");
        assert!(!rule.text.contains("LLM 浓缩"));
        assert!(rule.text.starts_with("经验教训手册："));
    }

    #[test]
    fn consolidation_never_folds_exempt_memories() {
        // Exempt memories (manual / safety-rule / project-rule) must stay active and
        // authoritative — consolidation must not archive them behind a summary.
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        let manual = store
            .learn_item(MemoryItemRequest {
                text: "用户手动固化：发布前必须在 F 盘备份源码。".to_string(),
                workspace: "repo-a".to_string(),
                category: "general".to_string(),
                tags: vec![],
                source: "manual".to_string(),
                source_session_id: String::new(),
            })
            .expect("learn manual");
        let safety = store
            .learn_item(MemoryItemRequest {
                text: "安全边界：不得删除官方 MSIX 文件。".to_string(),
                workspace: "repo-a".to_string(),
                category: "safety-rule".to_string(),
                tags: vec![],
                source: "auto".to_string(),
                source_session_id: String::new(),
            })
            .expect("learn safety");

        let mut conn = store.open().expect("open");
        // Only exempt items exist, so there is nothing consolidatable → no summary.
        let result = consolidate_items_with_summaries(&mut conn, &BTreeMap::new()).expect("run");
        assert!(result.is_none(), "no consolidatable non-exempt items");

        for id in [&manual.id, &safety.id] {
            let tier: String = conn
                .query_row(
                    "SELECT tier FROM memory_items WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(tier, TIER_ACTIVE, "exempt item {id} must stay active");
        }
    }
}
