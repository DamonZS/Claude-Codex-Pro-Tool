use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use fs2::FileExt;
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: i64 = 7;
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
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
const ACTIVITY_QUERY_SUMMARY_MAX_CHARS: usize = 160;
const ACTIVITY_SOURCE_SESSION_MAX_CHARS: usize = 160;
const ACTIVITY_METADATA_STRING_MAX_CHARS: usize = 256;
const ACTIVITY_MEMORY_SNAPSHOT_TEXT_MAX_CHARS: usize = 480;
const OUTCOME_RECENT_RECALL_LIMIT: usize = 20;
const OUTCOME_HANDOFF_LIMIT: usize = 8;
const NEW_PROJECT_GUIDE_LIMIT: usize = 12;
const MEMORY_EVENTS_RETENTION_SECS: i64 = 30 * 24 * 60 * 60;
const MEMORY_ACTIVITY_RETENTION_SECS: i64 = 90 * 24 * 60 * 60;
const MEMORY_EVENTS_MAX_ROWS: i64 = 20_000;
const MEMORY_ACTIVITY_MAX_ROWS: i64 = 50_000;
const EVENT_PRUNE_INTERVAL: u64 = 256;
const MEMORY_DB_FILE: &str = "memory_assist.sqlite";
const MEMORY_CACHE_FILE: &str = "pangu_memory_inject.md";
const MEMORY_BACKUP_DIR: &str = "memory_assist_backups";
const LESSON_MANUAL_CATEGORY: &str = "lesson-manual";
const LESSON_MANUAL_SOURCE: &str = "lesson-manual-compiler";
static ACTIVITY_EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static EVENT_PRUNE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
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

/// A durable, source-attributed memory activity record. Recall activity is
/// written once per actual hit. `memory` is the redacted, bounded snapshot that
/// was surfaced at hit time, so later edits or workspace moves cannot rewrite
/// the evidence shown by the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryActivityEvent {
    pub id: String,
    pub event_type: String,
    pub workspace: String,
    pub agent: String,
    pub memory_id: Option<String>,
    pub query_summary: String,
    pub source_session_id: Option<String>,
    pub metadata: Value,
    pub created_at: i64,
    pub memory: Option<MemoryItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryTrendPoint {
    pub date: String,
    pub captures: i64,
    pub learned: i64,
    pub recalls: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryBreakdown {
    pub key: String,
    pub count: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryOutcomeDashboard {
    pub workspace: String,
    pub range_days: usize,
    pub today_captures: i64,
    pub today_learned: i64,
    pub pending_candidates: i64,
    pub today_recalls: i64,
    pub trend: Vec<MemoryTrendPoint>,
    pub workspace_breakdown: Vec<MemoryBreakdown>,
    pub category_breakdown: Vec<MemoryBreakdown>,
    pub recent_recalls: Vec<MemoryActivityEvent>,
    pub handoff_items: Vec<MemoryItem>,
}

/// One generalized, locally-derived experience exposed by the new-project
/// guide. It intentionally carries no workspace, item, or session identifier.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryNewProjectExperience {
    pub text: String,
    pub source_count: usize,
    pub category: String,
}

/// Deterministic, read-only guidance distilled from active memories across all
/// projects. Empty vectors are the truthful empty state; no fallback experience
/// is invented when the database has no reusable lessons.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryNewProjectGuide {
    /// Stable source-data timestamp: the greatest `updated_at` among the
    /// unique memories represented in this guide, or zero for the empty state.
    pub generated_at: i64,
    /// Number of unique memory items represented by the selected experiences.
    pub source_item_count: usize,
    /// Number of unique source workspaces represented, including `global`.
    pub source_workspace_count: usize,
    /// Backward-compatible count of represented non-global projects.
    pub project_count: usize,
    pub pitfalls: Vec<MemoryNewProjectExperience>,
    pub best_practices: Vec<MemoryNewProjectExperience>,
    pub prompt: String,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCaptureProgressStatus {
    pub first_baseline_at: i64,
    pub last_scan_at: i64,
    pub total_sources: i64,
    pub codex_sources: i64,
    pub claude_sources: i64,
    /// Cumulative captured readable context units across all tracked sources.
    pub total_context_count: i64,
    /// Readable context units captured in the most recent scan wave.
    pub new_context_count: i64,
    /// Sources skipped as unchanged in the most recent scan wave.
    pub skipped_unchanged_sessions: i64,
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
    pub capture_progress: MemoryCaptureProgressStatus,
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
    pub sources_checked: usize,
    pub sources_skipped_unchanged: usize,
    pub new_contexts_seen: usize,
    pub scan_id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistMigrationRequest {
    pub target_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistMigrationResult {
    pub source_dir: String,
    pub target_dir: String,
    pub db_path: String,
    pub migrated: bool,
    pub source_retained: bool,
    pub restart_required: bool,
    pub migrated_files: Vec<String>,
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
            .join(MEMORY_CACHE_FILE)
    }

    pub fn migrate_data_dir(
        &self,
        target_dir: &Path,
    ) -> anyhow::Result<MemoryAssistMigrationResult> {
        if !target_dir.is_absolute() {
            anyhow::bail!("memory data directory must be an absolute path");
        }
        let source_dir = self
            .db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        if paths_equivalent(&source_dir, target_dir) {
            return Ok(MemoryAssistMigrationResult {
                source_dir: source_dir.to_string_lossy().into_owned(),
                target_dir: target_dir.to_string_lossy().into_owned(),
                db_path: self.db_path.to_string_lossy().into_owned(),
                migrated: false,
                source_retained: true,
                restart_required: false,
                migrated_files: Vec::new(),
            });
        }

        ensure_writable_directory(target_dir)?;
        let target_db = target_dir.join(MEMORY_DB_FILE);
        let target_cache = target_dir.join(MEMORY_CACHE_FILE);
        let target_backups = target_dir.join(MEMORY_BACKUP_DIR);
        for path in [&target_db, &target_cache, &target_backups] {
            if path.exists() {
                anyhow::bail!(
                    "target already contains memory data and will not be overwritten: {}",
                    path.display()
                );
            }
        }
        ensure_migration_space_available(&source_dir, target_dir)?;

        fs::create_dir_all(&source_dir)?;
        let lock_path = source_dir.join(".memory-assist-migration.lock");
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("open migration lock {}", lock_path.display()))?;
        lock.try_lock_exclusive()
            .with_context(|| "another memory migration is already running")?;

        let source = self.open()?;
        source
            .execute_batch("PRAGMA wal_checkpoint(FULL);")
            .context("checkpoint source memory database")?;
        let temp_db = target_dir.join(format!(
            ".{MEMORY_DB_FILE}.migrating-{}-{}",
            std::process::id(),
            now_nanos()
        ));
        let migration = (|| -> anyhow::Result<Vec<String>> {
            source
                .execute(
                    "VACUUM INTO ?1",
                    params![temp_db.to_string_lossy().as_ref()],
                )
                .with_context(|| format!("copy memory database to {}", temp_db.display()))?;
            let copied = Connection::open(&temp_db)?;
            let integrity: String =
                copied.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
            if integrity != "ok" {
                anyhow::bail!("migrated memory database failed integrity check: {integrity}");
            }
            drop(copied);
            fs::rename(&temp_db, &target_db).with_context(|| {
                format!(
                    "activate migrated memory database {} -> {}",
                    temp_db.display(),
                    target_db.display()
                )
            })?;

            let mut migrated_files = vec![MEMORY_DB_FILE.to_string()];
            let source_cache = source_dir.join(MEMORY_CACHE_FILE);
            if source_cache.is_file() {
                fs::copy(&source_cache, &target_cache)?;
                migrated_files.push(MEMORY_CACHE_FILE.to_string());
            }
            let source_backups = source_dir.join(MEMORY_BACKUP_DIR);
            if source_backups.is_dir() {
                copy_directory_tree(&source_backups, &target_backups)?;
                migrated_files.push(MEMORY_BACKUP_DIR.to_string());
            }
            Ok(migrated_files)
        })();
        if migration.is_err() {
            let _ = fs::remove_file(&temp_db);
            let _ = fs::remove_file(&target_db);
            let _ = fs::remove_file(&target_cache);
            let _ = fs::remove_dir_all(&target_backups);
        }
        let migrated_files = migration?;

        Ok(MemoryAssistMigrationResult {
            source_dir: source_dir.to_string_lossy().into_owned(),
            target_dir: target_dir.to_string_lossy().into_owned(),
            db_path: target_db.to_string_lossy().into_owned(),
            migrated: true,
            source_retained: true,
            restart_required: true,
            migrated_files,
        })
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
        let _claude_report = self.backfill_claude_history(50, false);
        let conn = self.open()?;
        let total_items = count_items(&conn)?;
        let pending_candidates = count_pending_candidates(&conn)?;
        let total_captures = count_captures(&conn)?;
        let capture_progress = capture_progress_status(&conn)?;
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
            capture_progress,
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
        let _ = record_activity_event(
            &conn,
            "learn",
            &item.workspace,
            &item.source,
            Some(&item.id),
            "",
            Some(&item.source_session_id),
            &json!({ "category": item.category }),
        );
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
        let _ = record_activity_event(
            &conn,
            "archive",
            &item.workspace,
            "manager",
            Some(&item.id),
            "",
            None,
            &json!({ "reason": "manual" }),
        );
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
        let _ = record_activity_event(
            &conn,
            "restore",
            &item.workspace,
            "manager",
            Some(&item.id),
            "",
            None,
            &json!({ "reason": "manual" }),
        );
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

    /// Runs a normal recall query and records one best-effort activity row for
    /// each returned memory. Activity failures never change the query result.
    pub fn query_with_activity(
        &self,
        request: MemoryQueryRequest,
        agent: &str,
        event_type: &str,
        source_session_id: Option<&str>,
    ) -> anyhow::Result<MemoryQueryResult> {
        let result = self.query_items(request, true)?;
        let event_type = normalize_activity_event_type(event_type);
        let is_real_recall = event_type == "inject" || !result.query.trim().is_empty();
        if is_real_recall && !result.results.is_empty() {
            if let Ok(conn) = self.open() {
                let agent = normalize_redacted_label(agent, "unknown");
                for hit in &result.results {
                    let _ = record_recall_activity_event(
                        &conn,
                        &event_type,
                        &result.workspace,
                        &agent,
                        Some(&hit.item.id),
                        &result.query,
                        source_session_id,
                        &json!({
                            "category": hit.item.category,
                            "score": hit.score,
                            "matched_keywords": hit.matched_keywords,
                        }),
                        Some(&hit.item),
                    );
                }
            }
        }
        Ok(result)
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
            let _ = record_activity_event(
                &conn,
                "candidate",
                &candidate.workspace,
                &candidate.source,
                None,
                "",
                Some(&candidate.source_session_id),
                &json!({ "candidate_id": candidate.id, "action": "updated" }),
            );
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
        let _ = record_activity_event(
            &conn,
            "candidate",
            &candidate.workspace,
            &candidate.source,
            None,
            "",
            Some(&candidate.source_session_id),
            &json!({ "candidate_id": candidate.id, "action": "created" }),
        );
        let _ = self.sync_inject_summary_cache(&conn, &candidate.workspace);
        Ok(candidate)
    }

    pub fn record_capture(
        &self,
        request: MemoryCaptureRequest,
    ) -> anyhow::Result<MemoryCaptureRecord> {
        let mut conn = self.open()?;
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
        let tx = conn.transaction()?;
        let changed = tx.execute(
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
                END
             WHERE memory_captures.source <> excluded.source
                OR memory_captures.source_session_id <> excluded.source_session_id
                OR memory_captures.text_length <> excluded.text_length
                OR memory_captures.summary <> excluded.summary
                OR memory_captures.candidate_triggered <> excluded.candidate_triggered
                OR memory_captures.candidate_reason <> excluded.candidate_reason
                OR memory_captures.skip_reason <> excluded.skip_reason",
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
        )? > 0;
        let capture = capture_by_workspace_hash(&tx, &workspace, &text_hash)?;
        if !changed {
            tx.commit()?;
            return Ok(capture);
        }
        record_event(
            &tx,
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
        let _ = record_activity_event(
            &tx,
            "capture",
            &capture.workspace,
            &capture.source,
            None,
            "",
            Some(&capture.source_session_id),
            &json!({
                "text_length": capture.text_length,
                "candidate_triggered": capture.candidate_triggered,
            }),
        );
        tx.commit()?;
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
        let mut report = MemoryHistoryCaptureReport {
            scan_id: new_scan_id("codex"),
            ..Default::default()
        };
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
                report.sources_checked += 1;
                let source_key = format!("codex:{}", row.thread_id);
                if self.source_progress_is_current(&source_key, &rollout_path) {
                    report.sources_skipped_unchanged += 1;
                    if let Err(error) = self.mark_source_skipped(
                        &source_key,
                        "codex-rollout",
                        &rollout_path,
                        if !row.cwd.trim().is_empty() {
                            row.cwd.as_str()
                        } else {
                            workspace_hint
                        },
                        &report.scan_id,
                    ) {
                        report.errors.push(format!(
                            "record codex unchanged progress for {} failed: {error}",
                            row.thread_id
                        ));
                    }
                    continue;
                }
                let messages = match read_codex_rollout_context_messages(&rollout_path, remaining) {
                    Ok(messages) => messages,
                    Err(error) => {
                        report.errors.push(format!(
                            "read rollout {} failed: {error}",
                            rollout_path.display()
                        ));
                        continue;
                    }
                };
                let scanned_units = messages.len();
                let mut captured_units = 0usize;
                for message in messages {
                    if remaining == 0 {
                        break;
                    }
                    remaining -= 1;
                    report.user_messages_seen += 1;
                    let text = message.text;
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
                            source_session_id: format!("{}#{}", row.thread_id, message.sequence),
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
                        source: format!("codex-history-rollout-{}", message.role),
                        source_session_id: format!("{}#{}", row.thread_id, message.sequence),
                        candidate_triggered,
                        candidate_reason,
                        skip_reason,
                    }) {
                        Ok(_) => {
                            report.captures_recorded += 1;
                            report.new_contexts_seen += 1;
                            captured_units += 1;
                        }
                        Err(error) => report.errors.push(format!(
                            "record history capture for {} failed: {error}",
                            row.thread_id
                        )),
                    }
                }
                if let Err(error) = self.upsert_source_progress(
                    &source_key,
                    "codex-rollout",
                    &rollout_path,
                    if !row.cwd.trim().is_empty() {
                        row.cwd.as_str()
                    } else {
                        workspace_hint
                    },
                    scanned_units,
                    captured_units,
                    &report.scan_id,
                ) {
                    report.errors.push(format!(
                        "record codex progress for {} failed: {error}",
                        row.thread_id
                    ));
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
        let query = self.query_with_activity(
            MemoryQueryRequest {
                query: request.query,
                workspace: workspace.clone(),
                include_global: true,
                limit: max_items,
                // Injection only ever surfaces active memories — decayed/archived
                // items must not leak back into the session-start summary.
                include_archived: false,
            },
            "codex",
            "inject",
            None,
        )?;
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

    /// Aggregates outcome evidence from SQLite for the selected workspace (plus
    /// global), or every real workspace when `workspace == "__all__"`.
    pub fn outcome_dashboard(
        &self,
        workspace: &str,
        range_days: usize,
    ) -> anyhow::Result<MemoryOutcomeDashboard> {
        let conn = self.open()?;
        outcome_dashboard_from_conn(&conn, workspace, range_days)
    }

    /// Builds a local, cross-project startup guide without performing a recall
    /// query or writing activity/access records.
    pub fn new_project_guide(&self) -> anyhow::Result<MemoryNewProjectGuide> {
        let conn = self.open()?;
        new_project_guide_from_conn(&conn)
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

    pub fn backfill_claude_history(
        &self,
        max_messages: usize,
        _generate_candidates: bool,
    ) -> MemoryHistoryCaptureReport {
        let mut report = MemoryHistoryCaptureReport {
            scan_id: new_scan_id("claude"),
            ..Default::default()
        };
        let mut remaining = max_messages.max(1);
        for path in discover_claude_capture_files() {
            if remaining == 0 {
                break;
            }
            if !path.is_file() {
                continue;
            }
            report.sources_checked += 1;
            let source_key = format!("claude:{}", path.to_string_lossy());
            if self.source_progress_is_current(&source_key, &path) {
                report.sources_skipped_unchanged += 1;
                if let Err(error) = self.mark_source_skipped(
                    &source_key,
                    "claude-file",
                    &path,
                    "global",
                    &report.scan_id,
                ) {
                    report.errors.push(format!(
                        "record claude unchanged progress for {} failed: {error}",
                        path.display()
                    ));
                }
                continue;
            }
            let messages = match read_claude_context_messages(&path, remaining) {
                Ok(messages) => messages,
                Err(error) => {
                    report.errors.push(format!(
                        "read claude source {} failed: {error}",
                        path.display()
                    ));
                    continue;
                }
            };
            let scanned_units = messages.len();
            let mut captured_units = 0usize;
            for message in messages {
                if remaining == 0 {
                    break;
                }
                remaining -= 1;
                report.user_messages_seen += 1;
                let workspace = normalize_workspace(&message.workspace);
                match self.record_capture(MemoryCaptureRequest {
                    text: message.text,
                    workspace,
                    source: format!("claude-history-{}", message.role),
                    source_session_id: format!("{}#{}", message.session_id, message.sequence),
                    candidate_triggered: false,
                    candidate_reason: String::new(),
                    skip_reason: "pending_core_classification".to_string(),
                }) {
                    Ok(_) => {
                        report.captures_recorded += 1;
                        report.new_contexts_seen += 1;
                        captured_units += 1;
                    }
                    Err(error) => report.errors.push(format!(
                        "record claude capture for {} failed: {error}",
                        path.display()
                    )),
                }
            }
            if let Err(error) = self.upsert_source_progress(
                &source_key,
                "claude-file",
                &path,
                "global",
                scanned_units,
                captured_units,
                &report.scan_id,
            ) {
                report.errors.push(format!(
                    "record claude progress for {} failed: {error}",
                    path.display()
                ));
            }
        }
        report
    }

    fn source_progress_is_current(&self, source_key: &str, source_path: &Path) -> bool {
        let Ok(conn) = self.open() else {
            return false;
        };
        let (modified_ms, size_bytes) = source_file_fingerprint(source_path);
        conn.query_row(
            "SELECT last_modified_ms, size_bytes FROM memory_capture_progress WHERE source_key = ?1",
            [source_key],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .ok()
        .flatten()
        .map(|(last_modified_ms, last_size)| last_modified_ms == modified_ms && last_size == size_bytes)
        .unwrap_or(false)
    }

    fn upsert_source_progress(
        &self,
        source_key: &str,
        source_kind: &str,
        source_path: &Path,
        workspace: &str,
        scanned_units: usize,
        captured_units: usize,
        scan_id: &str,
    ) -> anyhow::Result<()> {
        let conn = self.open()?;
        let now = now_unix();
        let (modified_ms, size_bytes) = source_file_fingerprint(source_path);
        let captured_units = i64::try_from(captured_units).unwrap_or(i64::MAX);
        conn.execute(
            "INSERT INTO memory_capture_progress
             (source_key, source_kind, source_path, workspace, last_modified_ms, size_bytes,
              scanned_units, captured_units, last_new_units, last_skipped_unchanged, last_scan_id,
              first_scanned_at, last_scanned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 0, ?9, ?10, ?10)
             ON CONFLICT(source_key) DO UPDATE SET
                source_kind = excluded.source_kind,
                source_path = excluded.source_path,
                workspace = excluded.workspace,
                last_modified_ms = excluded.last_modified_ms,
                size_bytes = excluded.size_bytes,
                scanned_units = excluded.scanned_units,
                captured_units = memory_capture_progress.captured_units + excluded.captured_units,
                last_new_units = excluded.last_new_units,
                last_skipped_unchanged = 0,
                last_scan_id = excluded.last_scan_id,
                last_scanned_at = excluded.last_scanned_at",
            params![
                source_key,
                source_kind,
                source_path.to_string_lossy(),
                normalize_workspace(workspace),
                modified_ms,
                size_bytes,
                i64::try_from(scanned_units).unwrap_or(i64::MAX),
                captured_units,
                scan_id,
                now,
            ],
        )?;
        Ok(())
    }

    fn mark_source_skipped(
        &self,
        source_key: &str,
        source_kind: &str,
        source_path: &Path,
        workspace: &str,
        scan_id: &str,
    ) -> anyhow::Result<()> {
        let conn = self.open()?;
        let now = now_unix();
        let (modified_ms, size_bytes) = source_file_fingerprint(source_path);
        conn.execute(
            "INSERT INTO memory_capture_progress
             (source_key, source_kind, source_path, workspace, last_modified_ms, size_bytes,
              scanned_units, captured_units, last_new_units, last_skipped_unchanged, last_scan_id,
              first_scanned_at, last_scanned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 0, 1, ?7, ?8, ?8)
             ON CONFLICT(source_key) DO UPDATE SET
                source_kind = excluded.source_kind,
                source_path = excluded.source_path,
                workspace = excluded.workspace,
                last_modified_ms = excluded.last_modified_ms,
                size_bytes = excluded.size_bytes,
                last_new_units = 0,
                last_skipped_unchanged = 1,
                last_scan_id = excluded.last_scan_id,
                last_scanned_at = excluded.last_scanned_at",
            params![
                source_key,
                source_kind,
                source_path.to_string_lossy(),
                normalize_workspace(workspace),
                modified_ms,
                size_bytes,
                scan_id,
                now,
            ],
        )?;
        Ok(())
    }

    fn open(&self) -> anyhow::Result<Connection> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create memory directory {}", parent.display()))?;
        }
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("open memory db {}", self.db_path.display()))?;
        configure_connection(&conn)?;
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
    resolved_memory_assist_data_dir().join(MEMORY_DB_FILE)
}

pub fn resolved_memory_assist_data_dir() -> PathBuf {
    let legacy_dir = crate::paths::default_app_state_dir();
    let settings = crate::settings::SettingsStore::default()
        .load()
        .unwrap_or_default();
    let configured = settings.memory_assist_data_dir.trim();
    if !configured.is_empty() {
        return PathBuf::from(configured);
    }

    // Existing installations keep using their original database until the
    // explicit migration flow has produced and verified a consistent copy.
    if legacy_dir.join(MEMORY_DB_FILE).exists() {
        return legacy_dir;
    }
    if let Some(installed_dir) = installed_memory_assist_data_dir()
        && ensure_writable_directory(&installed_dir).is_ok()
    {
        return installed_dir;
    }
    legacy_dir
}

pub fn migrate_memory_assist_data_dir(
    request: MemoryAssistMigrationRequest,
) -> anyhow::Result<MemoryAssistMigrationResult> {
    let target_dir = PathBuf::from(request.target_dir.trim());
    if request.target_dir.trim().is_empty() {
        anyhow::bail!("target memory data directory is empty");
    }
    let store = MemoryAssistStore::default();
    let result = store.migrate_data_dir(&target_dir)?;
    if let Err(error) = crate::settings::SettingsStore::default()
        .update(json!({
            "memoryAssistDataDir": target_dir.to_string_lossy().as_ref()
        }))
        .context("save migrated memory data directory")
    {
        if result.migrated {
            cleanup_migrated_target(&target_dir, &result.migrated_files).with_context(|| {
                format!("{error:#}; additionally failed to remove the uncommitted migrated copy")
            })?;
        }
        return Err(error);
    }
    Ok(result)
}

fn installed_memory_assist_data_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    installed_memory_assist_data_dir_from_exe(&exe)
}

fn installed_memory_assist_data_dir_from_exe(exe: &Path) -> Option<PathBuf> {
    let install_dir = exe.parent()?;
    let normalized = install_dir
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    if normalized.ends_with("/target/debug") || normalized.ends_with("/target/release") {
        return None;
    }

    let extension = if cfg!(windows) { ".exe" } else { "" };
    let expected = [
        format!("claude-codex-pro{extension}"),
        format!("claude-codex-pro-manager{extension}"),
        format!("claude-codex-pro-mcp{extension}"),
    ];
    if !expected.iter().all(|name| install_dir.join(name).is_file()) {
        return None;
    }
    Some(install_dir.join("data").join("memory-assist"))
}

fn ensure_writable_directory(path: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("create memory data directory {}", path.display()))?;
    if !path.is_dir() {
        anyhow::bail!("memory data path is not a directory: {}", path.display());
    }
    let probe = path.join(format!(
        ".memory-write-probe-{}-{}",
        std::process::id(),
        now_nanos()
    ));
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .with_context(|| format!("memory data directory is not writable: {}", path.display()))?;
    fs::remove_file(&probe)
        .with_context(|| format!("remove memory write probe {}", probe.display()))?;
    Ok(())
}

fn paths_equivalent(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| {
        path.to_string_lossy()
            .trim_end_matches(['/', '\\'])
            .replace('\\', "/")
            .to_ascii_lowercase()
    };
    normalize(left) == normalize(right)
}

fn ensure_migration_space_available(source_dir: &Path, target_dir: &Path) -> anyhow::Result<()> {
    const MIGRATION_SPACE_MARGIN: u64 = 16 * 1024 * 1024;

    let source_bytes = [MEMORY_DB_FILE, MEMORY_CACHE_FILE, MEMORY_BACKUP_DIR]
        .into_iter()
        .try_fold(0_u64, |total, name| {
            Ok::<_, anyhow::Error>(total.saturating_add(directory_size(&source_dir.join(name))?))
        })?;
    let required_bytes = source_bytes.saturating_add(MIGRATION_SPACE_MARGIN);
    let available_bytes = fs2::available_space(target_dir)
        .with_context(|| format!("read available space for {}", target_dir.display()))?;
    if available_bytes < required_bytes {
        anyhow::bail!(
            "target memory data directory has insufficient space: requires at least {} bytes, available {} bytes",
            required_bytes,
            available_bytes
        );
    }
    Ok(())
}

fn directory_size(path: &Path) -> anyhow::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    if path.is_file() {
        return Ok(fs::metadata(path)?.len());
    }

    let mut total = 0_u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            total = total.saturating_add(directory_size(&entry.path())?);
        } else if file_type.is_file() {
            total = total.saturating_add(entry.metadata()?.len());
        }
    }
    Ok(total)
}

fn cleanup_migrated_target(target_dir: &Path, migrated_files: &[String]) -> anyhow::Result<()> {
    for name in migrated_files.iter().rev() {
        let path = target_dir.join(name);
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .with_context(|| format!("remove migrated directory {}", path.display()))?;
        } else if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("remove migrated file {}", path.display()))?;
        }
    }
    Ok(())
}

fn copy_directory_tree(source: &Path, target: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory_tree(&source_path, &target_path)?;
        } else if entry.file_type()?.is_file() {
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
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
    let with_bearer = redact_bearer_tokens(&with_sk);
    let with_basic = redact_authorization_basic(&with_bearer);
    redact_named_secret_assignments(&with_basic)
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

fn configure_connection(conn: &Connection) -> anyhow::Result<()> {
    conn.busy_timeout(SQLITE_BUSY_TIMEOUT)
        .context("configure memory database busy timeout")?;
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .context("enable WAL journal mode for memory database")?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        anyhow::bail!("memory database did not enable WAL journal mode (actual: {journal_mode})");
    }
    Ok(())
}

fn ensure_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
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
        CREATE INDEX IF NOT EXISTS idx_memory_events_created ON memory_events(created_at DESC);

        CREATE TABLE IF NOT EXISTS memory_activity_events (
            id TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            workspace TEXT NOT NULL,
            agent TEXT NOT NULL,
            memory_id TEXT,
            query_summary TEXT NOT NULL DEFAULT '',
            source_session_id TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            memory_snapshot_json TEXT NOT NULL DEFAULT '',
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_memory_activity_workspace_created
            ON memory_activity_events(workspace, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_memory_activity_type_created
            ON memory_activity_events(event_type, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_memory_activity_created
            ON memory_activity_events(created_at DESC);

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

        CREATE TABLE IF NOT EXISTS memory_capture_progress (
            source_key TEXT PRIMARY KEY,
            source_kind TEXT NOT NULL,
            source_path TEXT NOT NULL,
            workspace TEXT NOT NULL,
            last_modified_ms INTEGER NOT NULL DEFAULT 0,
            size_bytes INTEGER NOT NULL DEFAULT 0,
            scanned_units INTEGER NOT NULL DEFAULT 0,
            captured_units INTEGER NOT NULL DEFAULT 0,
            last_new_units INTEGER NOT NULL DEFAULT 0,
            last_skipped_unchanged INTEGER NOT NULL DEFAULT 0,
            last_scan_id TEXT NOT NULL DEFAULT '',
            first_scanned_at INTEGER NOT NULL,
            last_scanned_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_memory_capture_progress_kind ON memory_capture_progress(source_kind);
        CREATE INDEX IF NOT EXISTS idx_memory_capture_progress_last_scan ON memory_capture_progress(last_scanned_at);

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
    if user_version < 5 {
        migrate_to_v5(conn)?;
    }
    if user_version < 6 {
        migrate_to_v6(conn)?;
    }
    if user_version < 7 {
        migrate_to_v7(conn)?;
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

/// v4 -> v5 migration: add recent-scan accounting to the capture progress table.
/// Existing progress rows remain valid; recent counters start at zero until the
/// next scan wave updates them.
fn migrate_to_v5(conn: &Connection) -> anyhow::Result<()> {
    if !column_exists(conn, "memory_capture_progress", "last_new_units")? {
        conn.execute_batch(
            "ALTER TABLE memory_capture_progress ADD COLUMN last_new_units INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    if !column_exists(conn, "memory_capture_progress", "last_skipped_unchanged")? {
        conn.execute_batch(
            "ALTER TABLE memory_capture_progress ADD COLUMN last_skipped_unchanged INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    if !column_exists(conn, "memory_capture_progress", "last_scan_id")? {
        conn.execute_batch(
            "ALTER TABLE memory_capture_progress ADD COLUMN last_scan_id TEXT NOT NULL DEFAULT '';",
        )?;
    }
    Ok(())
}

/// v5 -> v6 migration: add source-attributed outcome activity without touching
/// the existing memory/event/export tables. `IF NOT EXISTS` keeps partial or
/// repeated migrations safe.
fn migrate_to_v6(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS memory_activity_events (
            id TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            workspace TEXT NOT NULL,
            agent TEXT NOT NULL,
            memory_id TEXT,
            query_summary TEXT NOT NULL DEFAULT '',
            source_session_id TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_memory_activity_workspace_created
            ON memory_activity_events(workspace, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_memory_activity_type_created
            ON memory_activity_events(event_type, created_at DESC);
        ",
    )?;
    Ok(())
}

/// v6 -> v7 migration: persist a bounded, redacted recall-time snapshot. Old
/// activity rows remain valid with an empty snapshot and continue to render as
/// unavailable evidence instead of being joined to mutable current memory.
fn migrate_to_v7(conn: &Connection) -> anyhow::Result<()> {
    if !column_exists(conn, "memory_activity_events", "memory_snapshot_json")? {
        conn.execute_batch(
            "ALTER TABLE memory_activity_events
             ADD COLUMN memory_snapshot_json TEXT NOT NULL DEFAULT '';",
        )?;
    }
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

#[derive(Debug)]
struct NewProjectGuideCandidate {
    text: String,
    category: String,
    score: i32,
    source_updated_at: BTreeMap<String, i64>,
    workspaces: BTreeSet<String>,
    pitfall: bool,
}

fn new_project_guide_from_conn(conn: &Connection) -> anyhow::Result<MemoryNewProjectGuide> {
    let (private_workspaces, private_session_ids) = private_memory_identifiers(conn)?;
    let mut stmt = conn.prepare(
        "SELECT id, text, workspace, category, tags_json, source, source_session_id,
                created_at, updated_at, last_accessed_at, access_count,
                tier, strength, archived_at
         FROM memory_items
         WHERE tier = 'active'
           AND category IN ('lesson-learned', 'lesson-manual', 'safety-rule', 'project-rule')
         ORDER BY id ASC",
    )?;
    let items = stmt
        .query_map([], row_to_item)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut raw = Vec::<NewProjectGuideCandidate>::new();
    for item in items {
        for candidate in lesson_manual_candidates(&item.text) {
            if new_project_guide_line_is_unsafe(
                &candidate,
                &private_workspaces,
                &private_session_ids,
            ) {
                continue;
            }
            let line = compact_lesson_manual_line(&candidate);
            if line.chars().count() < 8
                || lesson_manual_line_is_noise(&line)
                || new_project_guide_line_is_unsafe(
                    &line,
                    &private_workspaces,
                    &private_session_ids,
                )
                || !contains_any_case_insensitive(&line, LESSON_ACTION_WORDS)
            {
                continue;
            }
            let mut score = lesson_sentence_score(&line);
            score += match item.category.as_str() {
                "lesson-learned" | LESSON_MANUAL_CATEGORY => 4,
                "safety-rule" => 3,
                "project-rule" => 2,
                _ => 0,
            };
            if score < 5 {
                continue;
            }
            let mut source_updated_at = BTreeMap::new();
            source_updated_at.insert(item.id.clone(), item.updated_at);
            let mut workspaces = BTreeSet::new();
            workspaces.insert(item.workspace.clone());
            raw.push(NewProjectGuideCandidate {
                pitfall: new_project_guide_is_pitfall(&line, &item.category),
                text: line,
                category: item.category.clone(),
                score,
                source_updated_at,
                workspaces,
            });
        }
    }

    raw.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.text.cmp(&right.text))
            .then_with(|| left.category.cmp(&right.category))
    });
    let mut merged = Vec::<NewProjectGuideCandidate>::new();
    for candidate in raw {
        if let Some(existing) = merged.iter_mut().find(|existing| {
            existing.pitfall == candidate.pitfall
                && duplicate_memory_score(&existing.text, &candidate.text).is_some()
        }) {
            existing
                .source_updated_at
                .extend(candidate.source_updated_at);
            existing.workspaces.extend(candidate.workspaces);
            continue;
        }
        merged.push(candidate);
    }

    // Global memories are explicitly curated for reuse and may stand alone.
    // Private workspace memories must be independently corroborated by at
    // least two projects before they can cross a project boundary. This keeps
    // a one-off project name or private convention out of new-project guides
    // even when it does not match one of the syntax-based secret filters.
    merged.retain(|candidate| {
        candidate.workspaces.contains(GLOBAL_WORKSPACE)
            || candidate
                .workspaces
                .iter()
                .filter(|workspace| workspace.as_str() != GLOBAL_WORKSPACE)
                .count()
                >= 2
    });

    merged.sort_by(|left, right| {
        right
            .source_updated_at
            .len()
            .cmp(&left.source_updated_at.len())
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| left.text.cmp(&right.text))
            .then_with(|| left.category.cmp(&right.category))
    });

    let selected_pitfalls = merged
        .iter()
        .filter(|candidate| candidate.pitfall)
        .take(NEW_PROJECT_GUIDE_LIMIT)
        .collect::<Vec<_>>();
    let selected_best_practices = merged
        .iter()
        .filter(|candidate| !candidate.pitfall)
        .take(NEW_PROJECT_GUIDE_LIMIT)
        .collect::<Vec<_>>();
    let selected = selected_pitfalls
        .iter()
        .chain(selected_best_practices.iter())
        .copied()
        .collect::<Vec<_>>();
    let source_updated_at = selected
        .iter()
        .flat_map(|candidate| candidate.source_updated_at.iter())
        .map(|(id, updated_at)| (id.clone(), *updated_at))
        .collect::<BTreeMap<_, _>>();
    let source_workspaces = selected
        .iter()
        .flat_map(|candidate| candidate.workspaces.iter().cloned())
        .collect::<BTreeSet<_>>();
    let generated_at = source_updated_at.values().copied().max().unwrap_or(0);
    let source_item_count = source_updated_at.len();
    let source_workspace_count = source_workspaces.len();
    let project_count = source_workspaces
        .iter()
        .filter(|workspace| workspace.as_str() != GLOBAL_WORKSPACE)
        .count();
    let to_experience = |candidate: &&NewProjectGuideCandidate| MemoryNewProjectExperience {
        text: candidate.text.clone(),
        source_count: candidate.source_updated_at.len(),
        category: candidate.category.clone(),
    };
    let pitfalls = selected_pitfalls
        .iter()
        .map(|candidate| MemoryNewProjectExperience {
            text: candidate.text.clone(),
            source_count: candidate.source_updated_at.len(),
            category: candidate.category.clone(),
        })
        .collect::<Vec<_>>();
    let best_practices = selected_best_practices
        .iter()
        .map(to_experience)
        .collect::<Vec<_>>();
    let prompt = new_project_guide_prompt(&pitfalls, &best_practices);

    Ok(MemoryNewProjectGuide {
        generated_at,
        source_item_count,
        source_workspace_count,
        project_count,
        pitfalls,
        best_practices,
        prompt,
    })
}

fn new_project_guide_prompt(
    pitfalls: &[MemoryNewProjectExperience],
    best_practices: &[MemoryNewProjectExperience],
) -> String {
    let mut sections = vec![
        "你正在启动一个新项目，请先阅读项目说明、相关规格、验收标准和源码，再开始实施。",
        "实施前总结当前目标、预计改动、禁止改动区域、验收标准与关键风险。",
        "坚持最小必要修改，不做无关重构，不擅自改变架构或生产配置。",
        "先建立可复现的验证方式，再按验证结果实施和修正。",
        "完成后列出真实运行的测试、构建或检查证据，不编造结果。",
        "历史经验仅作为参考，必须按新项目的技术栈、规则与上下文调整后再采用。",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    sections.push(new_project_guide_prompt_experiences("优先避坑", pitfalls));
    sections.push(new_project_guide_prompt_experiences(
        "优秀处理方式",
        best_practices,
    ));
    sections.join("\n")
}

fn new_project_guide_prompt_experiences(
    heading: &str,
    experiences: &[MemoryNewProjectExperience],
) -> String {
    if experiences.is_empty() {
        return format!("{heading}：暂无合格的历史经验。请以新项目资料和真实验证为准。");
    }
    let items = experiences
        .iter()
        .map(|experience| {
            format!(
                "- {}（类别：{}；唯一来源记忆：{} 条）",
                experience.text, experience.category, experience.source_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{heading}：\n{items}")
}

fn new_project_guide_is_pitfall(line: &str, category: &str) -> bool {
    category == "lesson-learned"
        || category == LESSON_MANUAL_CATEGORY
        || contains_any_case_insensitive(
            line,
            &[
                "踩坑",
                "教训",
                "根因",
                "失败",
                "错误",
                "不要",
                "不能",
                "避免",
                "否则",
                "不然",
                "never",
                "avoid",
                "failure",
                "error",
                "otherwise",
            ],
        )
}

fn private_memory_identifiers(
    conn: &Connection,
) -> anyhow::Result<(BTreeSet<String>, BTreeSet<String>)> {
    let mut workspaces = BTreeSet::new();
    let mut session_ids = BTreeSet::new();
    let mut stmt = conn.prepare("SELECT workspace, source_session_id FROM memory_items")?;
    for row in stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })? {
        let (workspace, session_id) = row?;
        let workspace = normalize_memory_text(&workspace).to_lowercase();
        if workspace != GLOBAL_WORKSPACE && workspace.chars().count() >= 3 {
            workspaces.insert(workspace);
        }
        let session_id = normalize_memory_text(&session_id).to_lowercase();
        if session_id.chars().count() >= 8 {
            session_ids.insert(session_id);
        }
    }
    Ok((workspaces, session_ids))
}

fn new_project_guide_line_is_unsafe(
    line: &str,
    private_workspaces: &BTreeSet<String>,
    private_session_ids: &BTreeSet<String>,
) -> bool {
    let normalized = normalize_memory_text(line);
    let lower = normalized.to_lowercase();
    let redacted = redact_secrets(&normalized);
    redacted != normalized
        || private_workspaces.iter().any(|value| lower.contains(value))
        || private_session_ids
            .iter()
            .any(|value| lower.contains(value))
        || lower.contains("sk-***")
        || lower.contains("bearer ***")
        || lower.contains("basic ***")
        || contains_any_case_insensitive(
            &lower,
            &[
                "api_key",
                "api-key",
                "apikey",
                "access_token",
                "auth_token",
                "authorization:",
                "password=",
                "password:",
                "token=",
                "token:",
                "secret=",
                "secret:",
            ],
        )
        || contains_concrete_filesystem_path(&lower)
        || contains_project_specific_command(&lower)
        || lower.contains("http://")
        || lower.contains("https://")
        || lower.contains("base url")
        || lower.contains("base_url")
        || lower.contains("source_session")
        || lower.contains("session id")
        || lower.contains("session_id")
        || contains_any_case_insensitive(
            &lower,
            &[
                "cargo ",
                "npm ",
                "pnpm ",
                "yarn ",
                "git ",
                "dotnet ",
                "mvn ",
                "gradle ",
                "powershell ",
                "cmd /c",
                ".exe",
                "--manifest-path",
            ],
        )
}

fn contains_project_specific_command(lower: &str) -> bool {
    let script_or_executable = lower.split_whitespace().any(|word| {
        let word = word.trim_matches(|ch: char| {
            matches!(ch, '`' | '"' | '\'' | '(' | ')' | '[' | ']' | ',' | ';')
        });
        word.starts_with("./")
            || word.starts_with("../")
            || word.ends_with(".ps1")
            || word.ends_with(".sh")
            || word.ends_with(".bat")
            || word.ends_with(".cmd")
    });
    script_or_executable
        || contains_any_case_insensitive(
            lower,
            &[
                "make ",
                "just ",
                "task ",
                "private-deploy ",
                "deploy-private",
            ],
        )
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

fn capture_progress_status(conn: &Connection) -> anyhow::Result<MemoryCaptureProgressStatus> {
    let mut status = MemoryCaptureProgressStatus::default();
    let mut stmt = conn.prepare(
        "SELECT source_kind, first_scanned_at, last_scanned_at, captured_units,
                last_new_units, last_skipped_unchanged, last_scan_id
         FROM memory_capture_progress",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;
    let mut latest_scan_id = String::new();
    for row in rows {
        let (
            kind,
            first_scanned_at,
            last_scanned_at,
            captured_units,
            last_new_units,
            last_skipped_unchanged,
            scan_id,
        ) = row?;
        status.total_sources += 1;
        if kind.starts_with("codex") {
            status.codex_sources += 1;
        } else if kind.starts_with("claude") {
            status.claude_sources += 1;
        }
        if status.first_baseline_at == 0 || first_scanned_at < status.first_baseline_at {
            status.first_baseline_at = first_scanned_at;
        }
        status.total_context_count += captured_units;
        if last_scanned_at > status.last_scan_at
            || (last_scanned_at == status.last_scan_at && scan_id > latest_scan_id)
        {
            status.last_scan_at = last_scanned_at;
            latest_scan_id = scan_id.clone();
            status.new_context_count = 0;
            status.skipped_unchanged_sessions = 0;
        }
        if !latest_scan_id.is_empty() && scan_id == latest_scan_id {
            status.new_context_count += last_new_units;
            status.skipped_unchanged_sessions += last_skipped_unchanged;
        }
    }
    Ok(status)
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

fn source_file_fingerprint(path: &Path) -> (i64, i64) {
    let Ok(metadata) = fs::metadata(path) else {
        return (0, 0);
    };
    let size_bytes = i64::try_from(metadata.len()).unwrap_or(i64::MAX);
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    (modified_ms, size_bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClaudeContextMessage {
    role: String,
    text: String,
    session_id: String,
    workspace: String,
    sequence: usize,
}

fn discover_claude_capture_files() -> Vec<PathBuf> {
    let Some(home_dir) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf())
    else {
        return Vec::new();
    };
    let mut files = Vec::new();
    let roots = [
        home_dir.join(".claude").join("sessions"),
        home_dir.join(".claude").join("claude-code-sessions"),
        home_dir.join(".claude").join("local-agent-mode-sessions"),
        home_dir.join(".config").join("claude-code-sessions"),
        home_dir.join(".config").join("local-agent-mode-sessions"),
    ];
    for root in roots {
        collect_claude_capture_files(&root, &mut files, 4);
    }
    for candidate in [
        home_dir.join(".claude").join("audit.jsonl"),
        home_dir.join(".claude").join("local.json"),
    ] {
        if candidate.is_file() {
            files.push(candidate);
        }
    }
    files.sort();
    files.dedup();
    files
}

fn collect_claude_capture_files(dir: &Path, files: &mut Vec<PathBuf>, depth: usize) {
    if depth == 0 || !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_claude_capture_files(&path, files, depth - 1);
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if lower == "audit.jsonl"
            || lower.starts_with("local_") && lower.ends_with(".json")
            || lower == "local.json"
            || lower.ends_with(".jsonl")
            || lower.ends_with(".json")
        {
            files.push(path);
        }
    }
}

fn read_claude_context_messages(
    path: &Path,
    limit: usize,
) -> anyhow::Result<Vec<ClaudeContextMessage>> {
    let content = fs::read_to_string(path)?;
    let workspace = infer_workspace_from_path(path);
    let session_id = stable_id("claude-session", &[path.to_string_lossy().as_ref()]);
    let mut messages = Vec::new();
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("jsonl"))
        .unwrap_or(false)
    {
        for line in content.lines() {
            if messages.len() >= limit {
                break;
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                collect_claude_messages_from_value(
                    &value,
                    &workspace,
                    &session_id,
                    &mut messages,
                    limit,
                );
            }
        }
    } else if let Ok(value) = serde_json::from_str::<Value>(&content) {
        collect_claude_messages_from_value(&value, &workspace, &session_id, &mut messages, limit);
    }
    Ok(messages)
}

fn collect_claude_messages_from_value(
    value: &Value,
    workspace: &str,
    session_id: &str,
    out: &mut Vec<ClaudeContextMessage>,
    limit: usize,
) {
    if out.len() >= limit {
        return;
    }
    if let Some(array) = value.as_array() {
        for item in array {
            collect_claude_messages_from_value(item, workspace, session_id, out, limit);
            if out.len() >= limit {
                break;
            }
        }
        return;
    }
    let Some(object) = value.as_object() else {
        return;
    };
    if let Some(messages) = object
        .get("messages")
        .or_else(|| object.get("conversation"))
        .or_else(|| object.get("turns"))
        .or_else(|| object.get("entries"))
        .and_then(Value::as_array)
    {
        for item in messages {
            collect_claude_messages_from_value(item, workspace, session_id, out, limit);
            if out.len() >= limit {
                break;
            }
        }
    }
    if out.len() >= limit {
        return;
    }
    let role = object
        .get("role")
        .or_else(|| object.get("type"))
        .or_else(|| object.get("speaker"))
        .and_then(Value::as_str)
        .map(normalize_capture_role)
        .unwrap_or_else(|| "unknown".to_string());
    let text = object
        .get("content")
        .or_else(|| object.get("text"))
        .or_else(|| object.get("message"))
        .or_else(|| object.get("prompt"))
        .or_else(|| object.get("response"))
        .map(extract_text_from_json_value)
        .unwrap_or_default();
    let text = normalize_memory_text(&redact_secrets(&text));
    if !text.is_empty() {
        let workspace = object
            .get("cwd")
            .or_else(|| object.get("workspace"))
            .or_else(|| object.get("project"))
            .and_then(Value::as_str)
            .map(normalize_workspace)
            .unwrap_or_else(|| normalize_workspace(workspace));
        let session_id = object
            .get("session_id")
            .or_else(|| object.get("sessionId"))
            .or_else(|| object.get("conversation_id"))
            .or_else(|| object.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(session_id)
            .to_string();
        out.push(ClaudeContextMessage {
            role,
            text,
            session_id,
            workspace,
            sequence: out.len() + 1,
        });
    }
}

fn extract_text_from_json_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(extract_text_from_json_value)
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join(
                "
",
            ),
        Value::Object(object) => object
            .get("text")
            .or_else(|| object.get("content"))
            .or_else(|| object.get("message"))
            .map(extract_text_from_json_value)
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn normalize_capture_role(role: &str) -> String {
    let role = role.trim().to_ascii_lowercase();
    match role.as_str() {
        "human" => "user".to_string(),
        "ai" | "model" => "assistant".to_string(),
        "tool_result" => "tool".to_string(),
        "system" | "developer" | "user" | "assistant" | "tool" => role,
        _ if role.is_empty() => "unknown".to_string(),
        _ => role,
    }
}

fn infer_workspace_from_path(path: &Path) -> String {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .map(normalize_workspace)
        .unwrap_or_else(|| GLOBAL_WORKSPACE.to_string())
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexRolloutContextMessage {
    role: String,
    text: String,
    sequence: usize,
}

fn read_codex_rollout_context_messages(
    path: &Path,
    limit: usize,
) -> anyhow::Result<Vec<CodexRolloutContextMessage>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();
    let mut sequence = 0usize;
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
        if payload.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        let role = payload
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .trim()
            .to_ascii_lowercase();
        let body = codex_message_content_text(&payload["content"]);
        if body.trim().is_empty() {
            continue;
        }
        sequence += 1;
        messages.push(CodexRolloutContextMessage {
            role: if role.is_empty() {
                "unknown".to_string()
            } else {
                role
            },
            text: body,
            sequence,
        });
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
                "input_text" | "output_text" | "text" => block
                    .get("text")
                    .and_then(Value::as_str)
                    .and_then(codex_visible_context_text_block),
                _ => None,
            }
        })
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn codex_visible_context_text_block(text: &str) -> Option<String> {
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
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
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
            stable_id(
                "evt",
                &[event, workspace, &now.to_string(), &now_nanos().to_string()],
            ),
            item_id,
            candidate_id,
            event,
            workspace,
            redact_secrets(&detail.to_string()),
            now,
        ],
    )?;
    maybe_prune_event_tables(conn, now)?;
    Ok(())
}

fn maybe_prune_event_tables(conn: &Connection, now: i64) -> anyhow::Result<()> {
    let sequence = EVENT_PRUNE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
    if sequence % EVENT_PRUNE_INTERVAL != 0 {
        return Ok(());
    }
    prune_event_tables(conn, now)
}

fn prune_event_tables(conn: &Connection, now: i64) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM memory_events WHERE created_at < ?1",
        params![now.saturating_sub(MEMORY_EVENTS_RETENTION_SECS)],
    )?;
    conn.execute(
        "DELETE FROM memory_events
         WHERE id IN (
            SELECT id FROM memory_events
            ORDER BY created_at DESC, rowid DESC
            LIMIT -1 OFFSET ?1
         )",
        params![MEMORY_EVENTS_MAX_ROWS],
    )?;
    conn.execute(
        "DELETE FROM memory_activity_events WHERE created_at < ?1",
        params![now.saturating_sub(MEMORY_ACTIVITY_RETENTION_SECS)],
    )?;
    conn.execute(
        "DELETE FROM memory_activity_events
         WHERE id IN (
            SELECT id FROM memory_activity_events
            ORDER BY created_at DESC, rowid DESC
            LIMIT -1 OFFSET ?1
         )",
        params![MEMORY_ACTIVITY_MAX_ROWS],
    )?;
    Ok(())
}

fn normalize_activity_event_type(event_type: &str) -> String {
    match event_type.trim() {
        "search" | "inject" | "learn" | "candidate" | "capture" | "archive" | "restore" => {
            event_type.trim().to_string()
        }
        _ => "search".to_string(),
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        let keep = max_chars.saturating_sub(1);
        format!("{}…", truncated.chars().take(keep).collect::<String>())
    } else {
        truncated
    }
}

fn sanitize_activity_text(value: &str, max_chars: usize) -> String {
    truncate_chars(&normalize_memory_text(&redact_secrets(value)), max_chars)
}

fn sanitize_activity_metadata(value: &Value) -> Value {
    match value {
        Value::String(value) => Value::String(sanitize_activity_text(
            value,
            ACTIVITY_METADATA_STRING_MAX_CHARS,
        )),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .take(32)
                .map(sanitize_activity_metadata)
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .take(32)
                .map(|(key, value)| {
                    (
                        sanitize_activity_text(key, 64),
                        sanitize_activity_metadata(value),
                    )
                })
                .collect(),
        ),
        value => value.clone(),
    }
}

fn sanitize_memory_snapshot(item: &MemoryItem) -> MemoryItem {
    let mut snapshot = item.clone();
    snapshot.text = sanitize_activity_text(&snapshot.text, ACTIVITY_MEMORY_SNAPSHOT_TEXT_MAX_CHARS);
    snapshot.workspace = sanitize_activity_text(&snapshot.workspace, 160);
    snapshot.category = sanitize_activity_text(&snapshot.category, 64);
    // The dashboard only renders the hit text/category. Do not duplicate tags
    // or the originating session id into durable activity evidence.
    snapshot.tags.clear();
    snapshot.source = sanitize_activity_text(&snapshot.source, 64);
    snapshot.source_session_id.clear();
    snapshot
}

fn record_activity_event(
    conn: &Connection,
    event_type: &str,
    workspace: &str,
    agent: &str,
    memory_id: Option<&str>,
    query: &str,
    source_session_id: Option<&str>,
    metadata: &Value,
) -> anyhow::Result<()> {
    record_activity_event_with_snapshot(
        conn,
        event_type,
        workspace,
        agent,
        memory_id,
        query,
        source_session_id,
        metadata,
        None,
    )
}

fn record_recall_activity_event(
    conn: &Connection,
    event_type: &str,
    workspace: &str,
    agent: &str,
    memory_id: Option<&str>,
    query: &str,
    source_session_id: Option<&str>,
    metadata: &Value,
    memory: Option<&MemoryItem>,
) -> anyhow::Result<()> {
    record_activity_event_with_snapshot(
        conn,
        event_type,
        workspace,
        agent,
        memory_id,
        query,
        source_session_id,
        metadata,
        memory,
    )
}

#[allow(clippy::too_many_arguments)]
fn record_activity_event_with_snapshot(
    conn: &Connection,
    event_type: &str,
    workspace: &str,
    agent: &str,
    memory_id: Option<&str>,
    query: &str,
    source_session_id: Option<&str>,
    metadata: &Value,
    memory: Option<&MemoryItem>,
) -> anyhow::Result<()> {
    let now = now_unix();
    let workspace = normalize_workspace(workspace);
    let event_type = normalize_activity_event_type(event_type);
    let agent = sanitize_activity_text(agent, 64);
    let query_summary = sanitize_activity_text(query, ACTIVITY_QUERY_SUMMARY_MAX_CHARS);
    let source_session_id = source_session_id
        .map(|value| sanitize_activity_text(value, ACTIVITY_SOURCE_SESSION_MAX_CHARS))
        .filter(|value| !value.is_empty());
    let metadata = sanitize_activity_metadata(metadata);
    let memory_snapshot_json = memory
        .map(sanitize_memory_snapshot)
        .map(|snapshot| serde_json::to_string(&snapshot))
        .transpose()?
        .unwrap_or_default();
    let unique_nonce = format!(
        "{}:{}:{}",
        now_nanos(),
        std::process::id(),
        ACTIVITY_EVENT_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed)
    );
    let event_id = stable_id(
        "act",
        &[
            &event_type,
            &workspace,
            memory_id.unwrap_or(""),
            &query_summary,
            &agent,
            &now.to_string(),
            &unique_nonce,
        ],
    );
    conn.execute(
        "INSERT INTO memory_activity_events
         (id, event_type, workspace, agent, memory_id, query_summary,
          source_session_id, metadata_json, memory_snapshot_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            event_id,
            event_type,
            workspace,
            agent,
            memory_id,
            query_summary,
            source_session_id,
            serde_json::to_string(&metadata)?,
            memory_snapshot_json,
            now,
        ],
    )?;
    maybe_prune_event_tables(conn, now)?;
    Ok(())
}

fn activity_scope_matches(selected: &str, actual: &str) -> bool {
    is_all_workspaces(selected)
        || actual == selected
        || (selected != GLOBAL_WORKSPACE && actual == GLOBAL_WORKSPACE)
}

fn recall_activity_scope_matches(selected: &str, actual: &str) -> bool {
    is_all_workspaces(selected) || actual == selected
}

fn outcome_dashboard_from_conn(
    conn: &Connection,
    workspace: &str,
    range_days: usize,
) -> anyhow::Result<MemoryOutcomeDashboard> {
    let workspace = normalize_workspace(workspace);
    let range_days = if range_days <= 7 { 7 } else { 30 };
    let today_start: i64 = conn.query_row(
        "SELECT CAST(strftime('%s', 'now', 'localtime', 'start of day', 'utc') AS INTEGER)",
        [],
        |row| row.get(0),
    )?;
    let range_start: i64 = conn.query_row(
        "SELECT CAST(strftime('%s', 'now', 'localtime', 'start of day', ?1, 'utc') AS INTEGER)",
        [format!("-{} days", range_days - 1)],
        |row| row.get(0),
    )?;

    let mut trend_by_date = BTreeMap::<String, MemoryTrendPoint>::new();
    for offset in (0..range_days).rev() {
        let date: String = conn.query_row(
            "SELECT date('now', 'localtime', 'start of day', ?1)",
            [format!("-{offset} days")],
            |row| row.get(0),
        )?;
        trend_by_date.insert(
            date.clone(),
            MemoryTrendPoint {
                date,
                ..Default::default()
            },
        );
    }

    let mut today_captures = 0;
    let mut capture_stmt = conn.prepare(
        "SELECT workspace, captured_at, date(captured_at, 'unixepoch', 'localtime')
         FROM memory_captures WHERE captured_at >= ?1",
    )?;
    for row in capture_stmt.query_map([range_start], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })? {
        let (actual_workspace, created_at, date) = row?;
        if !activity_scope_matches(&workspace, &actual_workspace) {
            continue;
        }
        if created_at >= today_start {
            today_captures += 1;
        }
        if let Some(point) = trend_by_date.get_mut(&date) {
            point.captures += 1;
        }
    }

    let mut today_learned = 0;
    let mut workspace_counts = BTreeMap::<String, i64>::new();
    let mut category_counts = BTreeMap::<String, i64>::new();
    let mut handoff_items = Vec::new();
    let mut item_stmt = conn.prepare(
        "SELECT id, text, workspace, category, tags_json, source, source_session_id,
                created_at, updated_at, last_accessed_at, access_count,
                tier, strength, archived_at
         FROM memory_items ORDER BY
             CASE category
                 WHEN 'project-rule' THEN 0 WHEN 'safety-rule' THEN 1
                 WHEN 'lesson-learned' THEN 2 WHEN 'decision' THEN 3
                 WHEN 'progress' THEN 4 ELSE 5 END,
             updated_at DESC, id DESC",
    )?;
    for row in item_stmt.query_map([], row_to_item)? {
        let mut item = row?;
        if !activity_scope_matches(&workspace, &item.workspace) {
            continue;
        }
        *workspace_counts.entry(item.workspace.clone()).or_default() += 1;
        *category_counts.entry(item.category.clone()).or_default() += 1;
        if item.created_at >= range_start {
            let date: String = conn.query_row(
                "SELECT date(?1, 'unixepoch', 'localtime')",
                [item.created_at],
                |row| row.get(0),
            )?;
            if let Some(point) = trend_by_date.get_mut(&date) {
                point.learned += 1;
            }
            if item.created_at >= today_start {
                today_learned += 1;
            }
        }
        if item.tier == TIER_ACTIVE && handoff_items.len() < OUTCOME_HANDOFF_LIMIT {
            decorate_item_decay(&mut item, now_unix());
            handoff_items.push(item);
        }
    }

    let mut today_recalls = 0;
    let mut recent_recalls = Vec::new();
    let mut activity_stmt = conn.prepare(
        "SELECT a.id, a.event_type, a.workspace, a.agent, a.memory_id,
                a.query_summary, a.source_session_id, a.metadata_json,
                a.memory_snapshot_json, a.created_at
         FROM memory_activity_events a
         WHERE a.created_at >= ?1 AND a.event_type IN ('search', 'inject')
         ORDER BY a.created_at DESC, a.id DESC",
    )?;
    let rows = activity_stmt.query_map([range_start], row_to_activity_event)?;
    for row in rows {
        let event = row?;
        if !recall_activity_scope_matches(&workspace, &event.workspace) {
            continue;
        }
        if event.created_at >= today_start {
            today_recalls += 1;
        }
        let date: String = conn.query_row(
            "SELECT date(?1, 'unixepoch', 'localtime')",
            [event.created_at],
            |row| row.get(0),
        )?;
        if let Some(point) = trend_by_date.get_mut(&date) {
            point.recalls += 1;
        }
        if recent_recalls.len() < OUTCOME_RECENT_RECALL_LIMIT {
            recent_recalls.push(event);
        }
    }

    let pending_candidates = {
        let mut stmt = conn.prepare(
            "SELECT workspace, COUNT(*) FROM memory_candidates
             WHERE status = 'pending' GROUP BY workspace",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut count = 0;
        for row in rows {
            let (actual_workspace, workspace_count) = row?;
            if activity_scope_matches(&workspace, &actual_workspace) {
                count += workspace_count;
            }
        }
        count
    };

    let mut workspace_breakdown = workspace_counts
        .into_iter()
        .map(|(key, count)| MemoryBreakdown { key, count })
        .collect::<Vec<_>>();
    workspace_breakdown.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });
    let mut category_breakdown = category_counts
        .into_iter()
        .map(|(key, count)| MemoryBreakdown { key, count })
        .collect::<Vec<_>>();
    category_breakdown.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });

    Ok(MemoryOutcomeDashboard {
        workspace,
        range_days,
        today_captures,
        today_learned,
        pending_candidates,
        today_recalls,
        trend: trend_by_date.into_values().collect(),
        workspace_breakdown,
        category_breakdown,
        recent_recalls,
        handoff_items,
    })
}

fn row_to_activity_event(row: &Row<'_>) -> rusqlite::Result<MemoryActivityEvent> {
    let metadata_json: String = row.get(7)?;
    let memory_snapshot_json: String = row.get(8)?;
    let memory = if memory_snapshot_json.is_empty() {
        None
    } else {
        serde_json::from_str::<MemoryItem>(&memory_snapshot_json).ok()
    };
    Ok(MemoryActivityEvent {
        id: row.get(0)?,
        event_type: row.get(1)?,
        workspace: row.get(2)?,
        agent: row.get(3)?,
        memory_id: row.get(4)?,
        query_summary: row.get(5)?,
        source_session_id: row.get(6)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(9)?,
        memory,
    })
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

fn new_scan_id(scope: &str) -> String {
    format!(
        "{}:{}:{}",
        now_unix(),
        normalize_label(scope, "history"),
        now_nanos()
    )
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

fn redact_authorization_basic(input: &str) -> String {
    const MARKER: &str = "basic";
    const CANONICAL: &str = "Basic ***";
    let lower = input.to_ascii_lowercase();
    let mut out = String::new();
    let mut i = 0;
    while let Some(relative) = lower[i..].find(MARKER) {
        let start = i + relative;
        let after_marker = start + MARKER.len();
        let before_is_authorization = lower[..start].trim_end().ends_with("authorization:");
        let Some(first_after) = input[after_marker..].chars().next() else {
            break;
        };
        if !before_is_authorization || !first_after.is_whitespace() {
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
            i = token_start;
            continue;
        }
        out.push_str(CANONICAL);
        i = end;
    }
    out.push_str(&input[i..]);
    out
}

fn redact_named_secret_assignments(input: &str) -> String {
    const NAMES: [&str; 8] = [
        "access_token",
        "auth_token",
        "api_key",
        "api-key",
        "apikey",
        "password",
        "secret",
        "token",
    ];
    let lower = input.to_ascii_lowercase();
    let mut out = String::new();
    let mut cursor = 0;
    while cursor < input.len() {
        let next = NAMES
            .iter()
            .filter_map(|name| {
                lower[cursor..]
                    .find(name)
                    .map(|offset| (cursor + offset, *name))
            })
            .min_by_key(|(start, name)| (*start, usize::MAX - name.len()));
        let Some((start, name)) = next else {
            break;
        };
        let end_name = start + name.len();
        let before_ok = start == 0
            || !lower[..start]
                .chars()
                .next_back()
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'));
        let after_ok = end_name == input.len()
            || !lower[end_name..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'));
        if !before_ok || !after_ok {
            out.push_str(&input[cursor..end_name]);
            cursor = end_name;
            continue;
        }

        let mut value_start = end_name;
        while input[value_start..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            value_start += input[value_start..].chars().next().unwrap().len_utf8();
        }
        let mut has_delimiter = false;
        if input[value_start..]
            .chars()
            .next()
            .is_some_and(|ch| matches!(ch, '=' | ':'))
        {
            has_delimiter = true;
            value_start += 1;
            while input[value_start..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
            {
                value_start += input[value_start..].chars().next().unwrap().len_utf8();
            }
        }
        let quote = input[value_start..]
            .chars()
            .next()
            .filter(|ch| matches!(ch, '"' | '\''));
        if !has_delimiter && quote.is_none() {
            out.push_str(&input[cursor..end_name]);
            cursor = end_name;
            continue;
        }
        if let Some(quote) = quote {
            value_start += quote.len_utf8();
        }
        let mut value_end = value_start;
        for (offset, ch) in input[value_start..].char_indices() {
            if quote.map_or_else(
                || ch.is_whitespace() || matches!(ch, ',' | ';' | '&' | '，' | '；'),
                |quote| ch == quote,
            ) {
                break;
            }
            value_end = value_start + offset + ch.len_utf8();
        }
        if value_end == value_start {
            out.push_str(&input[cursor..value_start]);
            cursor = value_start;
            continue;
        }
        out.push_str(&input[cursor..value_start]);
        out.push_str("***");
        cursor = value_end;
    }
    out.push_str(&input[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_connections_use_wal_and_busy_timeout() {
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        let conn = store.open().unwrap();

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        let busy_timeout_ms: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();

        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
        assert_eq!(busy_timeout_ms, SQLITE_BUSY_TIMEOUT.as_millis() as i64);
    }

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
    fn codex_rollout_context_reader_keeps_all_readable_roles() {
        let temp = tempfile::tempdir().unwrap();
        let rollout_path = temp.path().join("rollout.jsonl");
        std::fs::write(
            &rollout_path,
            format!(
                "{}\n{}\n{}\n{}\n{}\n",
                serde_json::json!({
                    "type": "response_item",
                    "payload": {
                        "type": "message",
                        "role": "system",
                        "content": [{"type": "text", "text": "system rule text"}]
                    }
                }),
                serde_json::json!({
                    "type": "response_item",
                    "payload": {
                        "type": "message",
                        "role": "developer",
                        "content": [{"type": "text", "text": "developer context text"}]
                    }
                }),
                serde_json::json!({
                    "type": "response_item",
                    "payload": {
                        "type": "message",
                        "role": "user",
                        "content": [{"type": "input_text", "text": "short"}]
                    }
                }),
                serde_json::json!({
                    "type": "response_item",
                    "payload": {
                        "type": "message",
                        "role": "assistant",
                        "content": [{"type": "output_text", "text": "assistant answer"}]
                    }
                }),
                serde_json::json!({
                    "type": "response_item",
                    "payload": {
                        "type": "message",
                        "role": "tool",
                        "content": [{"type": "text", "text": "tool result"}]
                    }
                }),
            ),
        )
        .unwrap();

        let messages = read_codex_rollout_context_messages(&rollout_path, 10).unwrap();
        assert_eq!(messages.len(), 5);
        assert_eq!(
            messages
                .iter()
                .map(|message| message.role.as_str())
                .collect::<Vec<_>>(),
            vec!["system", "developer", "user", "assistant", "tool"]
        );
        assert!(messages.iter().any(|message| message.text == "short"));
    }

    #[test]
    fn claude_context_reader_keeps_all_readable_roles() {
        let temp = tempfile::tempdir().unwrap();
        let audit_path = temp.path().join("audit.jsonl");
        std::fs::write(
            &audit_path,
            format!(
                "{}\n{}\n{}\n{}\n{}\n",
                serde_json::json!({"role": "system", "content": "system context"}),
                serde_json::json!({"role": "developer", "message": "developer note"}),
                serde_json::json!({"role": "user", "content": "short"}),
                serde_json::json!({"role": "assistant", "response": "assistant answer"}),
                serde_json::json!({"role": "tool", "text": "tool output"}),
            ),
        )
        .unwrap();

        let messages = read_claude_context_messages(&audit_path, 10).unwrap();
        assert_eq!(messages.len(), 5);
        assert_eq!(
            messages
                .iter()
                .map(|message| message.role.as_str())
                .collect::<Vec<_>>(),
            vec!["system", "developer", "user", "assistant", "tool"]
        );
        assert!(messages.iter().any(|message| message.text == "short"));
        assert!(
            messages
                .iter()
                .any(|message| message.text == "assistant answer")
        );
    }

    #[test]
    fn capture_progress_status_reports_latest_scan_not_lifetime_total() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO memory_capture_progress
             (source_key, source_kind, source_path, workspace, last_modified_ms, size_bytes,
              scanned_units, captured_units, last_new_units, last_skipped_unchanged, last_scan_id,
              first_scanned_at, last_scanned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                "codex:old",
                "codex-rollout",
                "old-rollout.jsonl",
                "repo-a",
                100_i64,
                10_i64,
                5_i64,
                5_i64,
                5_i64,
                0_i64,
                "100:codex:1",
                100_i64,
                100_i64,
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_capture_progress
             (source_key, source_kind, source_path, workspace, last_modified_ms, size_bytes,
              scanned_units, captured_units, last_new_units, last_skipped_unchanged, last_scan_id,
              first_scanned_at, last_scanned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                "claude:new",
                "claude-file",
                "audit.jsonl",
                "global",
                200_i64,
                20_i64,
                2_i64,
                2_i64,
                2_i64,
                0_i64,
                "200:claude:1",
                100_i64,
                200_i64,
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_capture_progress
             (source_key, source_kind, source_path, workspace, last_modified_ms, size_bytes,
              scanned_units, captured_units, last_new_units, last_skipped_unchanged, last_scan_id,
              first_scanned_at, last_scanned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                "claude:skip",
                "claude-file",
                "local_1.json",
                "global",
                200_i64,
                20_i64,
                0_i64,
                0_i64,
                0_i64,
                1_i64,
                "200:claude:1",
                100_i64,
                200_i64,
            ],
        )
        .unwrap();

        let status = capture_progress_status(&conn).unwrap();
        assert_eq!(status.total_sources, 3);
        assert_eq!(status.codex_sources, 1);
        assert_eq!(status.claude_sources, 2);
        assert_eq!(status.total_context_count, 7);
        assert_eq!(status.new_context_count, 2);
        assert_eq!(status.skipped_unchanged_sessions, 1);
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

    fn capture_request() -> MemoryCaptureRequest {
        MemoryCaptureRequest {
            text: "Keep the release build in the default target directory.".to_string(),
            workspace: "repo-a".to_string(),
            source: "codex-inject".to_string(),
            source_session_id: "session-a".to_string(),
            candidate_triggered: false,
            candidate_reason: String::new(),
            skip_reason: "no durable lesson".to_string(),
        }
    }

    #[test]
    fn duplicate_capture_is_event_idempotent_until_fields_change() {
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join(MEMORY_DB_FILE));
        let request = capture_request();

        store.record_capture(request.clone()).unwrap();
        store.record_capture(request.clone()).unwrap();

        let conn = store.open().unwrap();
        let captures: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_captures", [], |row| row.get(0))
            .unwrap();
        let events: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_events WHERE event = 'capture_recorded'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let activity: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_activity_events WHERE event_type = 'capture'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((captures, events, activity), (1, 1, 1));
        drop(conn);

        let mut changed = request;
        changed.candidate_triggered = true;
        changed.candidate_reason = "explicit user correction".to_string();
        changed.skip_reason.clear();
        store.record_capture(changed).unwrap();

        let conn = store.open().unwrap();
        let events: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_events WHERE event = 'capture_recorded'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let activity: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_activity_events WHERE event_type = 'capture'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((events, activity), (2, 2));
    }

    #[test]
    fn event_pruning_is_bounded_without_touching_durable_tables() {
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join(MEMORY_DB_FILE));
        store.record_capture(capture_request()).unwrap();
        let conn = store.open().unwrap();
        conn.execute_batch(
            "WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x + 1 FROM n WHERE x < 20010)
             INSERT INTO memory_events(id, event, workspace, detail_json, created_at)
             SELECT 'old-event-' || x, 'test', 'repo-a', '{}', 1 FROM n;
             WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x + 1 FROM n WHERE x < 50010)
             INSERT INTO memory_activity_events
                (id, event_type, workspace, agent, query_summary, metadata_json,
                 memory_snapshot_json, created_at)
             SELECT 'old-activity-' || x, 'capture', 'repo-a', 'test', '', '{}', '', 1 FROM n;",
        )
        .unwrap();

        prune_event_tables(&conn, now_unix()).unwrap();

        let events: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_events", [], |row| row.get(0))
            .unwrap();
        let activity: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_activity_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        let captures: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_captures", [], |row| row.get(0))
            .unwrap();
        assert!(events <= MEMORY_EVENTS_MAX_ROWS);
        assert!(activity <= MEMORY_ACTIVITY_MAX_ROWS);
        assert_eq!(captures, 1);
    }

    #[test]
    fn migration_creates_verified_copy_and_retains_source() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let target_dir = temp.path().join("target");
        let source_db = source_dir.join(MEMORY_DB_FILE);
        let store = MemoryAssistStore::new(source_db.clone());
        store.record_capture(capture_request()).unwrap();
        fs::write(store.inject_summary_cache_path(), "local inject summary").unwrap();
        let backup_dir = source_dir.join(MEMORY_BACKUP_DIR);
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(backup_dir.join("known.sqlite"), "backup").unwrap();

        let result = store.migrate_data_dir(&target_dir).unwrap();

        assert!(result.migrated);
        assert!(result.source_retained);
        assert!(result.restart_required);
        assert!(source_db.is_file());
        assert!(target_dir.join(MEMORY_DB_FILE).is_file());
        assert!(target_dir.join(MEMORY_CACHE_FILE).is_file());
        assert!(
            target_dir
                .join(MEMORY_BACKUP_DIR)
                .join("known.sqlite")
                .is_file()
        );
        let copied = Connection::open(target_dir.join(MEMORY_DB_FILE)).unwrap();
        let integrity: String = copied
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        let captures: i64 = copied
            .query_row("SELECT COUNT(*) FROM memory_captures", [], |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        assert_eq!(captures, 1);
    }

    #[test]
    fn migration_refuses_to_overwrite_existing_database() {
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("source").join(MEMORY_DB_FILE));
        store.record_capture(capture_request()).unwrap();
        let target_dir = temp.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join(MEMORY_DB_FILE), "different database").unwrap();

        let error = store.migrate_data_dir(&target_dir).unwrap_err().to_string();

        assert!(error.contains("will not be overwritten"));
        assert!(store.db_path().is_file());
    }

    #[test]
    fn migration_cleanup_removes_only_files_created_by_migration() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path().join("target");
        fs::create_dir_all(target_dir.join(MEMORY_BACKUP_DIR)).unwrap();
        fs::write(target_dir.join(MEMORY_DB_FILE), "database").unwrap();
        fs::write(target_dir.join(MEMORY_CACHE_FILE), "cache").unwrap();
        fs::write(
            target_dir.join(MEMORY_BACKUP_DIR).join("backup.sqlite"),
            "backup",
        )
        .unwrap();
        fs::write(target_dir.join("keep.txt"), "unrelated").unwrap();

        cleanup_migrated_target(
            &target_dir,
            &[
                MEMORY_DB_FILE.to_string(),
                MEMORY_CACHE_FILE.to_string(),
                MEMORY_BACKUP_DIR.to_string(),
            ],
        )
        .unwrap();

        assert!(!target_dir.join(MEMORY_DB_FILE).exists());
        assert!(!target_dir.join(MEMORY_CACHE_FILE).exists());
        assert!(!target_dir.join(MEMORY_BACKUP_DIR).exists());
        assert_eq!(
            fs::read_to_string(target_dir.join("keep.txt")).unwrap(),
            "unrelated"
        );
    }

    #[test]
    fn installed_data_dir_requires_all_companion_binaries_and_rejects_target_output() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = temp.path().join("Claude Codex Pro");
        fs::create_dir_all(&install_dir).unwrap();
        let extension = if cfg!(windows) { ".exe" } else { "" };
        for binary in [
            "claude-codex-pro",
            "claude-codex-pro-manager",
            "claude-codex-pro-mcp",
        ] {
            fs::write(install_dir.join(format!("{binary}{extension}")), []).unwrap();
        }
        let exe = install_dir.join(format!("claude-codex-pro-manager{extension}"));
        assert_eq!(
            installed_memory_assist_data_dir_from_exe(&exe),
            Some(install_dir.join("data").join("memory-assist"))
        );

        let dev_dir = temp.path().join("target").join("release");
        fs::create_dir_all(&dev_dir).unwrap();
        for binary in [
            "claude-codex-pro",
            "claude-codex-pro-manager",
            "claude-codex-pro-mcp",
        ] {
            fs::write(dev_dir.join(format!("{binary}{extension}")), []).unwrap();
        }
        assert_eq!(
            installed_memory_assist_data_dir_from_exe(
                &dev_dir.join(format!("claude-codex-pro-manager{extension}"))
            ),
            None
        );
    }
}
