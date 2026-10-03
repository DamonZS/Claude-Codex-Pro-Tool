//! AITRACKER-compatible local usage collection primitives.

use chrono::{DateTime, Local, Utc};
use claude_codex_pro_core::request_telemetry::RequestRecord;
use rusqlite::{Connection, OpenFlags, types::ValueRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_FILES: usize = 1_200;
pub const MAX_ENTRIES: usize = 30_000;
pub const MAX_EVENTS: usize = 100_000;
pub const MAX_SQLITE_ROWS: usize = 100_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UsageFormat {
    Json,
    Jsonl,
    Sqlite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsagePath {
    pub root: PathBuf,
    pub glob: String,
    pub format: UsageFormat,
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageFieldMapping {
    #[serde(default)]
    pub records: Vec<String>,
    #[serde(default)]
    pub event_id: Vec<String>,
    pub timestamp: Vec<String>,
    #[serde(default)]
    pub session_id: Vec<String>,
    #[serde(default)]
    pub model: Vec<String>,
    #[serde(default)]
    pub provider: Vec<String>,
    #[serde(default)]
    pub agent: Vec<String>,
    #[serde(default)]
    pub status: Vec<String>,
    #[serde(default)]
    pub duration_ms: Vec<String>,
    #[serde(default)]
    pub project: Vec<String>,
    #[serde(default)]
    pub input_tokens: Vec<String>,
    #[serde(default)]
    pub cached_input_tokens: Vec<String>,
    #[serde(default)]
    pub cache_creation_input_tokens: Vec<String>,
    #[serde(default)]
    pub output_tokens: Vec<String>,
    #[serde(default)]
    pub reasoning_output_tokens: Vec<String>,
    #[serde(default)]
    pub total_tokens: Vec<String>,
    #[serde(default)]
    pub tool_name: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageAdapter {
    pub source: String,
    pub paths: Vec<UsagePath>,
    pub mapping: UsageFieldMapping,
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: u64,
}

fn default_max_file_bytes() -> u64 {
    MAX_FILE_BYTES
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalUsageEvent {
    pub id: String,
    pub source: String,
    pub agent: String,
    pub provider: String,
    pub status: String,
    pub duration_ms: Option<u64>,
    pub timestamp: String,
    pub model: String,
    pub project: String,
    pub session_id: Option<String>,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
    pub measurement: String,
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticCode {
    FileTooLarge,
    MalformedJson,
    ReadFailed,
    FieldMismatch,
    Truncated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalUsageDiagnostic {
    pub code: DiagnosticCode,
    pub source: String,
    pub path: Option<String>,
    pub count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalTokenCounts {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalUsageTotals {
    pub events: usize,
    #[serde(flatten)]
    pub tokens: LocalTokenCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalUsageBreakdown {
    pub key: String,
    #[serde(flatten)]
    pub totals: LocalUsageTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalUsageDaily {
    pub date: String,
    #[serde(flatten)]
    pub totals: LocalUsageTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalUsageSession {
    pub id: String,
    pub source: String,
    pub agent: String,
    pub provider: String,
    pub model: String,
    pub project: String,
    pub started_at: String,
    pub ended_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalUsageSnapshot {
    pub generated_at: String,
    pub mode: String,
    pub events: usize,
    pub totals: LocalUsageTotals,
    pub by_source: Vec<LocalUsageBreakdown>,
    pub by_model: Vec<LocalUsageBreakdown>,
    pub by_project: Vec<LocalUsageBreakdown>,
    pub daily: Vec<LocalUsageDaily>,
    pub details: Vec<LocalUsageEvent>,
    pub recent: Vec<LocalUsageEvent>,
    pub sessions: Vec<LocalUsageSession>,
    pub diagnostics: Vec<LocalUsageDiagnostic>,
}

fn identifier(value: Option<&Value>) -> Option<String> {
    let text = value.and_then(Value::as_str).map(str::trim)?;
    (0 < text.len() && text.len() <= 200 && !text.chars().any(char::is_control))
        .then(|| text.to_string())
}

fn private_id(source: &str, kind: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ccp-aitracker-local-usage\0");
    hasher.update(source.as_bytes());
    hasher.update([0]);
    hasher.update(kind.as_bytes());
    hasher.update([0]);
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    let mut result = String::with_capacity(20);
    for byte in digest.iter().take(10) {
        result.push_str(&format!("{byte:02x}"));
    }
    format!("session_{result}")
}

pub fn session_id_from_structured(source: &str, value: Option<&Value>) -> Option<String> {
    identifier(value).map(|value| private_id(source, "structured", &value))
}

pub fn session_id_from_relative_file(source: &str, identity: &str) -> String {
    private_id(source, "file", &identity.replace('\\', "/"))
}

/// Return the display-safe project identity used by usage aggregation.
///
/// AITracker keeps non-path project references as their own bucket. Absolute
/// paths are reduced to the nearest Git repository root before taking the
/// final display segment, so nested working directories do not fragment one
/// repository.
pub fn canonical_project_label(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return None;
    }
    let has_path_shape = value.contains(['/', '\\']) || Path::new(value).is_absolute();
    if !has_path_shape {
        return Some(value.to_string());
    }

    let raw_path = Path::new(value);
    let normalized = fs::canonicalize(raw_path).unwrap_or_else(|_| raw_path.to_path_buf());
    let mut directory = if normalized.is_dir() {
        normalized.clone()
    } else {
        normalized
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| raw_path.to_path_buf())
    };
    let mut repository_root = None;
    loop {
        let git_path = directory.join(".git");
        if let Ok(metadata) = fs::metadata(&git_path) {
            let valid = if metadata.is_dir() {
                true
            } else if metadata.is_file() {
                fs::read_to_string(&git_path)
                    .ok()
                    .and_then(|contents| {
                        contents
                            .lines()
                            .find_map(|line| line.strip_prefix("gitdir:"))
                            .map(str::trim)
                            .map(str::to_string)
                    })
                    .map(|gitdir| directory.join(gitdir.trim()).exists())
                    .unwrap_or(false)
            } else {
                false
            };
            if valid {
                repository_root = Some(directory.clone());
                break;
            }
        }
        let Some(parent) = directory.parent() else {
            break;
        };
        if parent == directory {
            break;
        }
        directory = parent.to_path_buf();
    }

    // A linked worktree is the same project as the repository it was created
    // from. Resolve it through git metadata first; when the worktree directory
    // (or even the repository) is already gone, fall back to the conventional
    // `<repo>/.claude/worktrees/<name>` layout so old sessions still group.
    let repository_root = repository_root
        .as_deref()
        .and_then(linked_worktree_main_root)
        .or(repository_root)
        .or_else(|| claude_worktree_parent(&normalized));

    let display_path = repository_root.as_deref().unwrap_or(raw_path);
    display_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && !name.eq_ignore_ascii_case("."))
        .map(str::to_string)
        .or_else(|| value.rsplit(['/', '\\']).next().map(str::to_string))
        .filter(|name| !name.is_empty() && !name.eq_ignore_ascii_case("."))
}

/// If `root/.git` is a file of a linked worktree (`gitdir: <repo>/.git/worktrees/<name>`),
/// return the main repository's working directory.
fn linked_worktree_main_root(root: &Path) -> Option<PathBuf> {
    let git_path = root.join(".git");
    if !fs::metadata(&git_path).ok()?.is_file() {
        return None;
    }
    let contents = fs::read_to_string(&git_path).ok()?;
    let gitdir = contents
        .lines()
        .find_map(|line| line.strip_prefix("gitdir:"))?
        .trim();
    let gitdir = root.join(gitdir);
    // `.git/worktrees/<name>` -> `.git`
    let common_dir = gitdir
        .parent()
        .filter(|parent| parent.file_name().and_then(|name| name.to_str()) == Some("worktrees"))?;
    let git_dir = common_dir.parent()?;
    if git_dir.file_name().and_then(|name| name.to_str()) != Some(".git") {
        return None;
    }
    git_dir.parent().map(Path::to_path_buf)
}

/// `<repo>/.claude/worktrees/<name>[/...]` -> `<repo>`, purely from the path.
fn claude_worktree_parent(path: &Path) -> Option<PathBuf> {
    let components: Vec<_> = path.components().collect();
    let index = components.windows(3).position(|window| {
        let name = |component: &std::path::Component<'_>| {
            component.as_os_str().to_string_lossy().to_ascii_lowercase()
        };
        name(&window[0]) == ".claude" && name(&window[1]) == "worktrees"
    })?;
    if index == 0 {
        return None;
    }
    Some(components[..index].iter().collect())
}

pub fn project_label(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .and_then(canonical_project_label)
        .unwrap_or_else(|| "unknown".into())
}

fn value_at_path<'a>(record: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(record, |value, segment| value.get(segment))
}

fn first_value<'a>(record: &'a Value, paths: &[String]) -> Option<&'a Value> {
    paths.iter().find_map(|path| value_at_path(record, path))
}

fn token_value(record: &Value, paths: &[String]) -> u64 {
    first_value(record, paths)
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
        .unwrap_or(0)
}

fn timestamp_value(record: &Value, mapping: &UsageFieldMapping) -> Option<DateTime<Utc>> {
    let value = first_value(record, &mapping.timestamp)?;
    value
        .as_i64()
        .and_then(DateTime::from_timestamp_millis)
        .or_else(|| {
            value
                .as_str()
                .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
                .map(|value| value.with_timezone(&Utc))
        })
}

fn records_from_json(value: &Value, mapping: &UsageFieldMapping) -> Vec<Value> {
    mapping
        .records
        .iter()
        .find_map(|path| {
            value_at_path(value, path)
                .and_then(Value::as_array)
                .cloned()
        })
        .unwrap_or_else(|| {
            value
                .as_array()
                .cloned()
                .unwrap_or_else(|| vec![value.clone()])
        })
}

pub fn event_from_record(
    record: &Value,
    adapter: &UsageAdapter,
    fallback_session_id: Option<String>,
) -> Option<LocalUsageEvent> {
    let timestamp = timestamp_value(record, &adapter.mapping)?;
    let input_tokens = token_value(record, &adapter.mapping.input_tokens);
    let cached_input_tokens = token_value(record, &adapter.mapping.cached_input_tokens);
    let cache_creation_input_tokens =
        token_value(record, &adapter.mapping.cache_creation_input_tokens);
    let output_tokens = token_value(record, &adapter.mapping.output_tokens);
    let reasoning_output_tokens = token_value(record, &adapter.mapping.reasoning_output_tokens);
    let components = input_tokens
        .saturating_add(cached_input_tokens)
        .saturating_add(cache_creation_input_tokens)
        .saturating_add(output_tokens)
        .saturating_add(reasoning_output_tokens);
    let total_tokens = if components > 0 {
        components
    } else {
        token_value(record, &adapter.mapping.total_tokens)
    };
    if total_tokens == 0 {
        return None;
    }
    let session_id = session_id_from_structured(
        &adapter.source,
        first_value(record, &adapter.mapping.session_id),
    )
    .or(fallback_session_id);
    let event_identity = identifier(first_value(record, &adapter.mapping.event_id))
        .unwrap_or_else(|| format!("{timestamp}:{total_tokens}"));
    let id = private_id(
        &adapter.source,
        "event",
        &format!("{}:{}", session_id.as_deref().unwrap_or(""), event_identity,),
    );
    Some(LocalUsageEvent {
        id,
        source: adapter.source.clone(),
        timestamp: timestamp.to_rfc3339(),
        model: identifier(first_value(record, &adapter.mapping.model))
            .unwrap_or_else(|| "unknown".into()),
        agent: identifier(first_value(record, &adapter.mapping.agent))
            .unwrap_or_else(|| adapter.source.clone()),
        provider: identifier(first_value(record, &adapter.mapping.provider))
            .unwrap_or_else(|| "unknown".into()),
        status: identifier(first_value(record, &adapter.mapping.status))
            .unwrap_or_else(|| "observed".into()),
        duration_ms: first_value(record, &adapter.mapping.duration_ms)
            .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok())),
        project: project_label(first_value(record, &adapter.mapping.project)),
        session_id,
        input_tokens,
        cached_input_tokens,
        cache_creation_input_tokens,
        output_tokens,
        reasoning_output_tokens,
        total_tokens,
        measurement: "observed".into(),
        tool_name: first_value(record, &adapter.mapping.tool_name)
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

/// Build a zero-token session marker from a transcript record. AITracker
/// treats local sessions as first-class material even when a provider did not
/// persist usage counters; the marker keeps those sessions visible without
/// inventing token totals.
fn session_marker_from_record(
    record: &Value,
    adapter: &UsageAdapter,
    fallback_session_id: Option<String>,
    fallback_timestamp: DateTime<Utc>,
) -> Option<LocalUsageEvent> {
    let timestamp = timestamp_value(record, &adapter.mapping).unwrap_or(fallback_timestamp);
    let session_id = session_id_from_structured(
        &adapter.source,
        first_value(record, &adapter.mapping.session_id),
    )
    .or(fallback_session_id)?;
    let event_identity = identifier(first_value(record, &adapter.mapping.event_id))
        .unwrap_or_else(|| timestamp.to_rfc3339());
    Some(LocalUsageEvent {
        id: private_id(
            &adapter.source,
            "session-marker",
            &format!("{}:{}", session_id, event_identity),
        ),
        source: adapter.source.clone(),
        agent: identifier(first_value(record, &adapter.mapping.agent))
            .unwrap_or_else(|| adapter.source.clone()),
        provider: identifier(first_value(record, &adapter.mapping.provider))
            .unwrap_or_else(|| "unknown".into()),
        status: identifier(first_value(record, &adapter.mapping.status))
            .unwrap_or_else(|| "observed".into()),
        duration_ms: None,
        timestamp: timestamp.to_rfc3339(),
        model: identifier(first_value(record, &adapter.mapping.model))
            .unwrap_or_else(|| "unknown".into()),
        project: project_label(first_value(record, &adapter.mapping.project)),
        session_id: Some(session_id),
        input_tokens: 0,
        cached_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        reasoning_output_tokens: 0,
        total_tokens: 0,
        measurement: "session".into(),
        tool_name: first_value(record, &adapter.mapping.tool_name)
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn workbuddy_event_from_record(
    record: &Value,
    adapter: &UsageAdapter,
    fallback_session_id: Option<String>,
) -> Option<LocalUsageEvent> {
    let raw_usage = value_at_path(record, "providerData.rawUsage")?;
    if !raw_usage.is_object() {
        return None;
    }
    let timestamp = timestamp_value(record, &adapter.mapping)?;
    let prompt_tokens = token_value(raw_usage, &["prompt_tokens".into()]);
    let cached_input_tokens = [
        token_value(raw_usage, &["cache_read_input_tokens".into()]),
        token_value(raw_usage, &["prompt_cache_hit_tokens".into()]),
        token_value(raw_usage, &["prompt_tokens_details.cached_tokens".into()]),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    let cache_creation_input_tokens =
        token_value(raw_usage, &["cache_creation_input_tokens".into()]);
    let input_tokens = prompt_tokens
        .saturating_sub(cached_input_tokens)
        .saturating_sub(cache_creation_input_tokens);
    let completion_tokens = token_value(raw_usage, &["completion_tokens".into()]);
    let reasoning_output_tokens = [
        token_value(raw_usage, &["completion_thinking_tokens".into()]),
        token_value(
            raw_usage,
            &["completion_tokens_details.reasoning_tokens".into()],
        ),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
    .min(completion_tokens);
    let output_tokens = completion_tokens.saturating_sub(reasoning_output_tokens);
    let total_tokens = input_tokens
        .saturating_add(cached_input_tokens)
        .saturating_add(cache_creation_input_tokens)
        .saturating_add(output_tokens)
        .saturating_add(reasoning_output_tokens);
    if total_tokens == 0 {
        return None;
    }
    let session_id = session_id_from_structured(
        &adapter.source,
        first_value(record, &adapter.mapping.session_id),
    )
    .or(fallback_session_id);
    let event_identity = identifier(first_value(record, &adapter.mapping.event_id))
        .or_else(|| identifier(value_at_path(record, "providerData.messageId")))
        .unwrap_or_else(|| format!("{timestamp}:{total_tokens}"));
    Some(LocalUsageEvent {
        id: private_id(
            &adapter.source,
            "event",
            &format!("{}:{}", session_id.as_deref().unwrap_or(""), event_identity),
        ),
        source: adapter.source.clone(),
        agent: "workbuddy".into(),
        provider: identifier(first_value(record, &adapter.mapping.provider))
            .unwrap_or_else(|| "unknown".into()),
        status: identifier(first_value(record, &adapter.mapping.status))
            .unwrap_or_else(|| "observed".into()),
        duration_ms: first_value(record, &adapter.mapping.duration_ms)
            .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok())),
        timestamp: timestamp.to_rfc3339(),
        model: identifier(first_value(record, &adapter.mapping.model))
            .or_else(|| identifier(value_at_path(record, "providerData.requestModelName")))
            .or_else(|| identifier(value_at_path(record, "providerData.requestModelId")))
            .unwrap_or_else(|| "auto".into()),
        project: project_label(first_value(record, &adapter.mapping.project)),
        session_id,
        input_tokens,
        cached_input_tokens,
        cache_creation_input_tokens,
        output_tokens,
        reasoning_output_tokens,
        total_tokens,
        measurement: "observed".into(),
        tool_name: first_value(record, &adapter.mapping.tool_name)
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let (mut pattern_index, mut value_index) = (0usize, 0usize);
    let (mut star, mut star_value) = (None, 0usize);
    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?' || pattern[pattern_index] == value[value_index])
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star = Some(pattern_index);
            pattern_index += 1;
            star_value = value_index;
        } else if let Some(star_index) = star {
            pattern_index = star_index + 1;
            star_value += 1;
            value_index = star_value;
        } else {
            return false;
        }
    }
    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

fn matches_glob(path: &Path, glob: &str) -> bool {
    let normalized_path = path.to_string_lossy().replace('\\', "/");
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let normalized_glob = glob.replace('\\', "/");
    let file_glob = normalized_glob
        .strip_prefix("**/")
        .unwrap_or(&normalized_glob);
    wildcard_match(file_glob, name) || wildcard_match(&normalized_glob, &normalized_path)
}

fn display_path(path: &Path) -> Option<String> {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
}

fn sqlite_query_is_read_only(query: &str) -> bool {
    let normalized = query.trim().to_ascii_lowercase();
    (normalized.starts_with("select ")
        || normalized.starts_with("select\n")
        || normalized.starts_with("with ")
        || normalized.starts_with("with\n"))
        && !normalized.contains(';')
}

fn sqlite_row_value(row: &rusqlite::Row<'_>, index: usize) -> Value {
    match row.get_ref(index) {
        Ok(ValueRef::Null) => Value::Null,
        Ok(ValueRef::Integer(value)) => Value::from(value),
        Ok(ValueRef::Real(value)) => Value::from(value),
        Ok(ValueRef::Text(value)) => Value::String(String::from_utf8_lossy(value).into_owned()),
        Ok(ValueRef::Blob(_)) | Err(_) => Value::Null,
    }
}

fn scan_sqlite_file(
    file_path: &Path,
    usage_path: &UsagePath,
    adapter: &UsageAdapter,
    events: &mut Vec<LocalUsageEvent>,
    diagnostics: &mut Vec<LocalUsageDiagnostic>,
) {
    let Some(query) = usage_path.query.as_deref() else {
        diagnostics.push(LocalUsageDiagnostic {
            code: DiagnosticCode::FieldMismatch,
            source: adapter.source.clone(),
            path: display_path(file_path),
            count: 1,
            message: "SQLite adapter 缺少只读查询。".into(),
        });
        return;
    };
    if !sqlite_query_is_read_only(query) {
        diagnostics.push(LocalUsageDiagnostic {
            code: DiagnosticCode::FieldMismatch,
            source: adapter.source.clone(),
            path: display_path(file_path),
            count: 1,
            message: "SQLite adapter 只允许单条 SELECT/WITH 查询。".into(),
        });
        return;
    }
    let connection = match Connection::open_with_flags(file_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
    {
        Ok(connection) => connection,
        Err(_) => {
            diagnostics.push(LocalUsageDiagnostic {
                code: DiagnosticCode::ReadFailed,
                source: adapter.source.clone(),
                path: display_path(file_path),
                count: 1,
                message: "读取 SQLite 用量数据库失败。".into(),
            });
            return;
        }
    };
    let mut statement = match connection.prepare(query) {
        Ok(statement) => statement,
        Err(_) => {
            diagnostics.push(LocalUsageDiagnostic {
                code: DiagnosticCode::MalformedJson,
                source: adapter.source.clone(),
                path: display_path(file_path),
                count: 1,
                message: "SQLite 用量查询无法准备。".into(),
            });
            return;
        }
    };
    let column_names = statement
        .column_names()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let rows = statement.query_map([], |row| {
        let mut value = serde_json::Map::new();
        for (index, name) in column_names.iter().enumerate() {
            value.insert(name.clone(), sqlite_row_value(row, index));
        }
        Ok(Value::Object(value))
    });
    let Ok(rows) = rows else {
        diagnostics.push(LocalUsageDiagnostic {
            code: DiagnosticCode::ReadFailed,
            source: adapter.source.clone(),
            path: display_path(file_path),
            count: 1,
            message: "SQLite 用量查询读取失败。".into(),
        });
        return;
    };
    for (index, row) in rows.enumerate() {
        if index >= MAX_SQLITE_ROWS {
            diagnostics.push(LocalUsageDiagnostic {
                code: DiagnosticCode::Truncated,
                source: adapter.source.clone(),
                path: display_path(file_path),
                count: 1,
                message: "SQLite 用量已达到行读取上限。".into(),
            });
            break;
        }
        if let Ok(value) = row {
            let fallback =
                session_id_from_relative_file(&adapter.source, &file_path.to_string_lossy());
            if let Some(event) = event_from_record(&value, adapter, Some(fallback)) {
                events.push(event);
            }
        }
    }
}

fn collect_files(
    path: &UsagePath,
    source: &str,
    diagnostics: &mut Vec<LocalUsageDiagnostic>,
) -> Vec<PathBuf> {
    let mut pending = vec![(path.root.clone(), 0usize)];
    let mut files = Vec::new();
    let mut seen = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => {
                diagnostics.push(LocalUsageDiagnostic {
                    code: DiagnosticCode::ReadFailed,
                    source: source.into(),
                    path: display_path(&directory),
                    count: 1,
                    message: "读取本地用量目录失败。".into(),
                });
                continue;
            }
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > MAX_ENTRIES {
                return files;
            }
            let file_path = entry.path();
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                if depth < 32 {
                    pending.push((file_path, depth + 1));
                }
            } else if matches_glob(&file_path, &path.glob) {
                files.push(file_path);
            }
        }
    }
    files.sort_by_key(|path| fs::metadata(path).and_then(|meta| meta.modified()).ok());
    files.reverse();
    files.truncate(MAX_FILES);
    files
}

pub fn scan_adapter(
    adapter: &UsageAdapter,
    limit: usize,
) -> (Vec<LocalUsageEvent>, Vec<LocalUsageDiagnostic>) {
    let mut diagnostics = Vec::new();
    let mut events = Vec::new();
    for usage_path in &adapter.paths {
        if usage_path.format == UsageFormat::Sqlite {
            let files = if usage_path.root.is_file() {
                vec![usage_path.root.clone()]
            } else {
                collect_files(usage_path, &adapter.source, &mut diagnostics)
            };
            for file_path in files {
                bound_scan_events(&mut events, adapter, &mut diagnostics);
                scan_sqlite_file(
                    &file_path,
                    usage_path,
                    adapter,
                    &mut events,
                    &mut diagnostics,
                );
            }
            continue;
        }
        for path in collect_files(usage_path, &adapter.source, &mut diagnostics) {
            bound_scan_events(&mut events, adapter, &mut diagnostics);
            if usage_path.format == UsageFormat::Jsonl
                && matches!(
                    adapter.source.as_str(),
                    "codex" | "claude-code" | "every-code"
                )
            {
                let (records, warnings) = crate::request_history::read_native_usage_file(
                    &path,
                    &adapter.source,
                    MAX_EVENTS,
                );
                events.extend(records.iter().map(|record| {
                    let mut event = event_from_request_record(record);
                    event.source = adapter.source.clone();
                    event.agent = adapter.source.clone();
                    event
                }));
                diagnostics.extend(warnings.into_iter().map(|message| LocalUsageDiagnostic {
                    code: if message.contains("上限") {
                        DiagnosticCode::Truncated
                    } else {
                        DiagnosticCode::ReadFailed
                    },
                    source: adapter.source.clone(),
                    path: display_path(&path),
                    count: 1,
                    message,
                }));
                continue;
            }
            if usage_path.format == UsageFormat::Jsonl {
                scan_jsonl_file(&path, adapter, &mut events, &mut diagnostics);
                continue;
            }
            let bytes = match fs::metadata(&path) {
                Ok(meta) if meta.len() <= adapter.max_file_bytes => match fs::read(&path) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        diagnostics.push(LocalUsageDiagnostic {
                            code: DiagnosticCode::ReadFailed,
                            source: adapter.source.clone(),
                            path: display_path(&path),
                            count: 1,
                            message: "读取本地用量文件失败。".into(),
                        });
                        continue;
                    }
                },
                Ok(_) => {
                    diagnostics.push(LocalUsageDiagnostic {
                        code: DiagnosticCode::FileTooLarge,
                        source: adapter.source.clone(),
                        path: display_path(&path),
                        count: 1,
                        message: "日志超过读取上限，已跳过。".into(),
                    });
                    continue;
                }
                Err(_) => continue,
            };
            let values = match serde_json::from_slice::<Value>(&bytes) {
                Ok(value) => records_from_json(&value, &adapter.mapping),
                Err(_) => {
                    diagnostics.push(LocalUsageDiagnostic {
                        code: DiagnosticCode::MalformedJson,
                        source: adapter.source.clone(),
                        path: display_path(&path),
                        count: 1,
                        message: "JSON 文件无法解析。".into(),
                    });
                    Vec::new()
                }
            };
            let fallback = session_id_from_relative_file(&adapter.source, &path.to_string_lossy());
            for value in values {
                let event = if adapter.source == "workbuddy" {
                    workbuddy_event_from_record(&value, adapter, Some(fallback.clone()))
                } else {
                    event_from_record(&value, adapter, Some(fallback.clone()))
                };
                if let Some(event) = event {
                    events.push(event);
                }
            }
        }
    }
    dedupe_events(&mut events);
    events.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    if events.len() > limit.min(MAX_EVENTS) {
        diagnostics.push(LocalUsageDiagnostic {
            code: DiagnosticCode::Truncated,
            source: adapter.source.clone(),
            path: None,
            count: events.len() - limit.min(MAX_EVENTS),
            message: "本地用量超过事件预算，仅保留近期记录。".into(),
        });
    }
    events.truncate(limit.min(MAX_EVENTS));
    (events, diagnostics)
}

fn bound_scan_events(
    events: &mut Vec<LocalUsageEvent>,
    adapter: &UsageAdapter,
    diagnostics: &mut Vec<LocalUsageDiagnostic>,
) {
    if events.len() <= MAX_EVENTS * 2 {
        return;
    }
    dedupe_events(events);
    events.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    if events.len() > MAX_EVENTS {
        diagnostics.push(LocalUsageDiagnostic {
            code: DiagnosticCode::Truncated,
            source: adapter.source.clone(),
            path: None,
            count: events.len() - MAX_EVENTS,
            message: "本地用量超过事件预算，仅保留近期记录。".into(),
        });
        events.truncate(MAX_EVENTS);
    }
}

fn scan_jsonl_file(
    path: &Path,
    adapter: &UsageAdapter,
    events: &mut Vec<LocalUsageEvent>,
    diagnostics: &mut Vec<LocalUsageDiagnostic>,
) {
    let file = fs::File::open(path).and_then(|file| Ok((file.metadata()?.len(), file)));
    let (len, file) = match file {
        Ok(value) => value,
        Err(_) => {
            diagnostics.push(LocalUsageDiagnostic {
                code: DiagnosticCode::ReadFailed,
                source: adapter.source.clone(),
                path: display_path(path),
                count: 1,
                message: "读取本地用量文件失败。".into(),
            });
            return;
        }
    };
    if len > adapter.max_file_bytes {
        diagnostics.push(LocalUsageDiagnostic {
            code: DiagnosticCode::FileTooLarge,
            source: adapter.source.clone(),
            path: display_path(path),
            count: 1,
            message: "日志超过读取上限，已跳过。".into(),
        });
        return;
    }
    let fallback = session_id_from_relative_file(&adapter.source, &path.to_string_lossy());
    let fallback_timestamp = file
        .metadata()
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|duration| DateTime::from_timestamp_millis(duration.as_millis() as i64))
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
    let mut malformed = 0;
    for line in BufReader::new(file).split(b'\n') {
        let Ok(line) = line else {
            diagnostics.push(LocalUsageDiagnostic {
                code: DiagnosticCode::ReadFailed,
                source: adapter.source.clone(),
                path: display_path(path),
                count: 1,
                message: "读取本地用量记录失败。".into(),
            });
            break;
        };
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        match serde_json::from_slice::<Value>(&line) {
            Ok(value) => {
                let event = if adapter.source == "workbuddy" {
                    workbuddy_event_from_record(&value, adapter, Some(fallback.clone()))
                } else {
                    event_from_record(&value, adapter, Some(fallback.clone()))
                };
                if let Some(event) = event {
                    events.push(event);
                    bound_scan_events(events, adapter, diagnostics);
                } else if let Some(marker) = session_marker_from_record(
                    &value,
                    adapter,
                    Some(fallback.clone()),
                    fallback_timestamp,
                ) {
                    events.push(marker);
                    bound_scan_events(events, adapter, diagnostics);
                }
            }
            Err(_) => malformed += 1,
        }
    }
    if malformed > 0 {
        diagnostics.push(LocalUsageDiagnostic {
            code: DiagnosticCode::MalformedJson,
            source: adapter.source.clone(),
            path: display_path(path),
            count: malformed,
            message: "JSONL 包含无法解析的记录。".into(),
        });
    }
}

pub fn event_from_request_record(record: &RequestRecord) -> LocalUsageEvent {
    let is_codex_rollout = record.source == "codex_rollout";
    let input = if is_codex_rollout {
        record
            .input_tokens
            .unwrap_or(0)
            .saturating_sub(record.cached_tokens.unwrap_or(0))
    } else {
        record.input_tokens.unwrap_or(0)
    };
    let output = record.output_tokens.unwrap_or(0);
    let source = match record.source.as_str() {
        "codex_rollout" => "codex",
        "claude_code" => "claude-code",
        source => source,
    };
    let components = input
        .saturating_add(record.cached_tokens.unwrap_or(0))
        .saturating_add(record.cache_creation_tokens.unwrap_or(0))
        .saturating_add(output)
        .saturating_add(record.reasoning_tokens.unwrap_or(0));
    LocalUsageEvent {
        id: record.id.clone(),
        source: source.into(),
        agent: if record.source == "claude_code" {
            "claude-code".into()
        } else {
            record.agent.clone()
        },
        provider: record.provider.clone().unwrap_or_else(|| "unknown".into()),
        status: record.status.clone(),
        duration_ms: record.duration_ms,
        timestamp: DateTime::from_timestamp_millis(record.timestamp_ms as i64)
            .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
            .to_rfc3339(),
        model: record.model.clone().unwrap_or_else(|| "unknown".into()),
        project: record
            .project
            .as_deref()
            .and_then(canonical_project_label)
            .unwrap_or_else(|| "unknown".into()),
        session_id: record
            .session_id
            .as_deref()
            .map(|value| private_id(source, "structured", value)),
        input_tokens: input,
        cached_input_tokens: record.cached_tokens.unwrap_or(0),
        cache_creation_input_tokens: record.cache_creation_tokens.unwrap_or(0),
        output_tokens: output,
        reasoning_output_tokens: record.reasoning_tokens.unwrap_or(0),
        total_tokens: if is_codex_rollout && components > 0 {
            components
        } else {
            record.total_tokens.unwrap_or(components)
        },
        measurement: "observed".into(),
        tool_name: None,
    }
}

pub fn request_record_from_event(event: &LocalUsageEvent) -> RequestRecord {
    let timestamp_ms = DateTime::parse_from_rfc3339(&event.timestamp)
        .map(|value| value.timestamp_millis().max(0) as u64)
        .unwrap_or_default();
    RequestRecord {
        id: event.id.clone(),
        timestamp_ms,
        source: event.source.clone(),
        agent: event.agent.clone(),
        session_id: event.session_id.clone(),
        project: (!event.project.is_empty() && event.project != "unknown")
            .then(|| event.project.clone()),
        provider: (!event.provider.is_empty() && event.provider != "unknown")
            .then(|| event.provider.clone()),
        model: (event.model != "unknown").then(|| event.model.clone()),
        protocol: None,
        upstream_protocol: None,
        status: event.status.clone(),
        http_status: None,
        duration_ms: event.duration_ms,
        first_byte_ms: None,
        input_tokens: Some(event.input_tokens),
        output_tokens: Some(event.output_tokens),
        cached_tokens: Some(event.cached_input_tokens),
        cache_creation_tokens: Some(event.cache_creation_input_tokens),
        reasoning_tokens: Some(event.reasoning_output_tokens),
        total_tokens: Some(event.total_tokens),
        streaming: false,
    }
}

fn add_event(totals: &mut LocalUsageTotals, event: &LocalUsageEvent) {
    totals.events += 1;
    totals.tokens.input_tokens = totals
        .tokens
        .input_tokens
        .saturating_add(event.input_tokens);
    totals.tokens.cached_input_tokens = totals
        .tokens
        .cached_input_tokens
        .saturating_add(event.cached_input_tokens);
    totals.tokens.cache_creation_input_tokens = totals
        .tokens
        .cache_creation_input_tokens
        .saturating_add(event.cache_creation_input_tokens);
    totals.tokens.output_tokens = totals
        .tokens
        .output_tokens
        .saturating_add(event.output_tokens);
    totals.tokens.reasoning_output_tokens = totals
        .tokens
        .reasoning_output_tokens
        .saturating_add(event.reasoning_output_tokens);
    totals.tokens.total_tokens = totals
        .tokens
        .total_tokens
        .saturating_add(event.total_tokens);
}

fn dedupe_events(events: &mut Vec<LocalUsageEvent>) {
    let mut unique = HashMap::<String, LocalUsageEvent>::new();
    for event in events.drain(..) {
        match unique.get_mut(&event.id) {
            Some(previous) if event.total_tokens > previous.total_tokens => *previous = event,
            Some(_) => {}
            None => {
                unique.insert(event.id.clone(), event);
            }
        }
    }
    events.extend(unique.into_values());
}

fn sqlite_timestamp(value: i64) -> String {
    DateTime::from_timestamp_millis(value)
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339()
}

/// Reads only WorkBuddy's session index; message bodies and settings stay untouched.
pub fn collect_workbuddy_sessions(home: &Path) -> Vec<LocalUsageSession> {
    let path = home.join(".workbuddy").join("workbuddy.db");
    let Ok(connection) = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
    else {
        return Vec::new();
    };
    let Ok(mut statement) = connection.prepare(
        "SELECT id, cwd, model, status, created_at, updated_at FROM sessions WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT ?1",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map([MAX_SQLITE_ROWS as i64], |row| {
        let id: String = row.get(0)?;
        let cwd: Option<String> = row.get(1).ok();
        let model: Option<String> = row.get(2).ok();
        let status: Option<String> = row.get(3).ok();
        let created_at: i64 = row.get(4).unwrap_or_default();
        let updated_at: i64 = row.get(5).unwrap_or(created_at);
        let project = cwd
            .as_deref()
            .map(|value| project_label(Some(&Value::String(value.to_string()))))
            .unwrap_or_else(|| "unknown".into());
        Ok(LocalUsageSession {
            id: private_id("workbuddy", "structured", &id),
            source: "workbuddy".into(),
            agent: "workbuddy".into(),
            provider: "unknown".into(),
            model: model
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "unknown".into()),
            project,
            started_at: sqlite_timestamp(created_at),
            ended_at: sqlite_timestamp(updated_at),
            status: status
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "observed".into()),
        })
    }) else {
        return Vec::new();
    };
    rows.filter_map(Result::ok).collect()
}

pub fn aggregate(
    events: Vec<LocalUsageEvent>,
    diagnostics: Vec<LocalUsageDiagnostic>,
) -> LocalUsageSnapshot {
    let mut details = events;
    dedupe_events(&mut details);
    details.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    let mut totals = LocalUsageTotals::default();
    let mut by_source = HashMap::new();
    let mut by_model = HashMap::new();
    let mut by_project = HashMap::new();
    let mut daily = HashMap::new();
    for event in &details {
        add_event(&mut totals, event);
        add_event(by_source.entry(event.source.clone()).or_default(), event);
        add_event(by_model.entry(event.model.clone()).or_default(), event);
        add_event(by_project.entry(event.project.clone()).or_default(), event);
        let date = DateTime::parse_from_rfc3339(&event.timestamp)
            .map(|value| value.with_timezone(&Local).format("%Y-%m-%d").to_string())
            .unwrap_or_else(|_| "unknown".into());
        add_event(daily.entry(date).or_default(), event);
    }
    fn breakdown(values: HashMap<String, LocalUsageTotals>) -> Vec<LocalUsageBreakdown> {
        let mut values = values
            .into_iter()
            .map(|(key, totals)| LocalUsageBreakdown { key, totals })
            .collect::<Vec<_>>();
        values.sort_by(|left, right| {
            right
                .totals
                .tokens
                .total_tokens
                .cmp(&left.totals.tokens.total_tokens)
                .then(left.key.cmp(&right.key))
        });
        values
    }
    let mut daily = daily
        .into_iter()
        .map(|(date, totals)| LocalUsageDaily { date, totals })
        .collect::<Vec<_>>();
    daily.sort_by(|left, right| left.date.cmp(&right.date));
    LocalUsageSnapshot {
        generated_at: Utc::now().to_rfc3339(),
        mode: if details.is_empty() { "empty" } else { "real" }.into(),
        events: details.len(),
        totals,
        by_source: breakdown(by_source),
        by_model: breakdown(by_model),
        by_project: breakdown(by_project),
        daily,
        recent: details.iter().take(50).cloned().collect(),
        details,
        sessions: Vec::new(),
        diagnostics,
    }
}

const AITRACKER_DEFINITIONS: &[(&str, &str)] = &[
    (
        "claude-code",
        include_str!("../../../assets/aitracker/definitions/claude-code.tool.json"),
    ),
    (
        "codex",
        include_str!("../../../assets/aitracker/definitions/codex.tool.json"),
    ),
    (
        "cursor",
        include_str!("../../../assets/aitracker/definitions/cursor.tool.json"),
    ),
    (
        "kiro",
        include_str!("../../../assets/aitracker/definitions/kiro.tool.json"),
    ),
    (
        "gemini-cli",
        include_str!("../../../assets/aitracker/definitions/gemini-cli.tool.json"),
    ),
    (
        "opencode",
        include_str!("../../../assets/aitracker/definitions/opencode.tool.json"),
    ),
    (
        "openclaw",
        include_str!("../../../assets/aitracker/definitions/openclaw.tool.json"),
    ),
    (
        "every-code",
        include_str!("../../../assets/aitracker/definitions/every-code.tool.json"),
    ),
    (
        "hermes",
        include_str!("../../../assets/aitracker/definitions/hermes.tool.json"),
    ),
    (
        "github-copilot",
        include_str!("../../../assets/aitracker/definitions/github-copilot.tool.json"),
    ),
    (
        "kimi-code",
        include_str!("../../../assets/aitracker/definitions/kimi-code.tool.json"),
    ),
    (
        "omp",
        include_str!("../../../assets/aitracker/definitions/omp.tool.json"),
    ),
    (
        "codebuddy",
        include_str!("../../../assets/aitracker/definitions/codebuddy.tool.json"),
    ),
    (
        "workbuddy",
        include_str!("../../../assets/aitracker/definitions/workbuddy.tool.json"),
    ),
    (
        "grok",
        include_str!("../../../assets/aitracker/definitions/grok.tool.json"),
    ),
    (
        "kilo-cli",
        include_str!("../../../assets/aitracker/definitions/kilo-cli.tool.json"),
    ),
    (
        "kilocode",
        include_str!("../../../assets/aitracker/definitions/kilocode.tool.json"),
    ),
    (
        "antigravity",
        include_str!("../../../assets/aitracker/definitions/antigravity.tool.json"),
    ),
    (
        "pi",
        include_str!("../../../assets/aitracker/definitions/pi.tool.json"),
    ),
    (
        "craft",
        include_str!("../../../assets/aitracker/definitions/craft.tool.json"),
    ),
    (
        "roo-code",
        include_str!("../../../assets/aitracker/definitions/roo-code.tool.json"),
    ),
    (
        "zed",
        include_str!("../../../assets/aitracker/definitions/zed.tool.json"),
    ),
    (
        "goose",
        include_str!("../../../assets/aitracker/definitions/goose.tool.json"),
    ),
    (
        "droid",
        include_str!("../../../assets/aitracker/definitions/droid.tool.json"),
    ),
    (
        "mimo",
        include_str!("../../../assets/aitracker/definitions/mimo.tool.json"),
    ),
    (
        "zcode",
        include_str!("../../../assets/aitracker/definitions/zcode.tool.json"),
    ),
    (
        "anythingllm",
        include_str!("../../../assets/aitracker/definitions/anythingllm.tool.json"),
    ),
    (
        "dsh",
        include_str!("../../../assets/aitracker/definitions/dsh.tool.json"),
    ),
    (
        "aipy",
        include_str!("../../../assets/aitracker/definitions/aipy.tool.json"),
    ),
    (
        "cline",
        include_str!("../../../assets/aitracker/definitions/cline.tool.json"),
    ),
    (
        "qwen",
        include_str!("../../../assets/aitracker/definitions/qwen.tool.json"),
    ),
    (
        "commandcode",
        include_str!("../../../assets/aitracker/definitions/commandcode.tool.json"),
    ),
    (
        "proma",
        include_str!("../../../assets/aitracker/definitions/proma.tool.json"),
    ),
    (
        "qodercn",
        include_str!("../../../assets/aitracker/definitions/qodercn.tool.json"),
    ),
    (
        "reasonix",
        include_str!("../../../assets/aitracker/definitions/reasonix.tool.json"),
    ),
    (
        "cherrystudio",
        include_str!("../../../assets/aitracker/definitions/cherrystudio.tool.json"),
    ),
];

fn generic_mapping() -> UsageFieldMapping {
    UsageFieldMapping {
        event_id: vec!["id", "eventId", "message.id"]
            .into_iter()
            .map(String::from)
            .collect(),
        timestamp: vec!["timestamp", "created_at", "createdAt"]
            .into_iter()
            .map(String::from)
            .collect(),
        session_id: vec!["sessionId", "session_id", "session.id"]
            .into_iter()
            .map(String::from)
            .collect(),
        model: vec!["model", "message.model", "providerData.model"]
            .into_iter()
            .map(String::from)
            .collect(),
        provider: vec!["provider", "providerId", "provider_id", "modelProvider"]
            .into_iter()
            .map(String::from)
            .collect(),
        agent: vec!["agent", "tool", "client"]
            .into_iter()
            .map(String::from)
            .collect(),
        status: vec!["status", "state"]
            .into_iter()
            .map(String::from)
            .collect(),
        duration_ms: vec!["duration_ms", "durationMs"]
            .into_iter()
            .map(String::from)
            .collect(),
        project: vec!["cwd", "project", "workspace"]
            .into_iter()
            .map(String::from)
            .collect(),
        input_tokens: vec![
            "usage.input_tokens",
            "usage.inputTokens",
            "providerData.usage.inputTokens",
            "providerData.rawUsage.prompt_tokens",
            "message.usage.input_tokens",
            "input_tokens",
            "inputTokens",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        cached_input_tokens: vec![
            "usage.cache_read_input_tokens",
            "usage.cached_input_tokens",
            "providerData.rawUsage.cached_tokens",
            "providerData.rawUsage.cache_read_input_tokens",
            "message.usage.cache_read_input_tokens",
            "cached_input_tokens",
            "cachedInputTokens",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        cache_creation_input_tokens: vec![
            "usage.cache_creation_input_tokens",
            "usage.cache_write_input_tokens",
            "providerData.rawUsage.cache_creation_input_tokens",
            "providerData.rawUsage.prompt_cache_write_tokens",
            "message.usage.cache_creation_input_tokens",
            "cache_creation_input_tokens",
            "cacheCreationInputTokens",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        output_tokens: vec![
            "usage.output_tokens",
            "usage.outputTokens",
            "providerData.usage.outputTokens",
            "providerData.rawUsage.completion_tokens",
            "message.usage.output_tokens",
            "output_tokens",
            "outputTokens",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        reasoning_output_tokens: vec![
            "usage.reasoning_output_tokens",
            "providerData.rawUsage.completion_thinking_tokens",
            "message.usage.reasoning_output_tokens",
            "reasoning_output_tokens",
            "reasoningOutputTokens",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        total_tokens: vec![
            "usage.total_tokens",
            "usage.totalTokens",
            "providerData.usage.totalTokens",
            "providerData.rawUsage.total_tokens",
            "message.usage.total_tokens",
            "total_tokens",
            "totalTokens",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        tool_name: vec!["toolName", "tool_name", "tool.name", "name", "tool"]
            .into_iter()
            .map(String::from)
            .collect(),
        records: Vec::new(),
    }
}

fn mapping_list(mapping: &Value, key: &str) -> Vec<String> {
    mapping
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn definition_mapping(usage: &Value) -> UsageFieldMapping {
    let mut mapping = generic_mapping();
    let Some(definition) = usage.get("mapping") else {
        return mapping;
    };
    macro_rules! replace {
        ($field:ident, $key:literal) => {
            let values = mapping_list(definition, $key);
            if !values.is_empty() {
                mapping.$field = values;
            }
        };
    }
    replace!(records, "records");
    replace!(event_id, "eventId");
    replace!(timestamp, "timestamp");
    replace!(session_id, "sessionId");
    replace!(model, "model");
    replace!(provider, "provider");
    replace!(agent, "agent");
    replace!(status, "status");
    replace!(duration_ms, "durationMs");
    replace!(project, "project");
    replace!(input_tokens, "inputTokens");
    replace!(cached_input_tokens, "cachedInputTokens");
    replace!(cache_creation_input_tokens, "cacheCreationInputTokens");
    replace!(output_tokens, "outputTokens");
    replace!(reasoning_output_tokens, "reasoningOutputTokens");
    replace!(total_tokens, "totalTokens");
    replace!(tool_name, "toolName");
    mapping
}

pub(crate) fn definition_base(home: &Path, base: &str) -> PathBuf {
    let roaming = env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData/Roaming"));
    let local = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData/Local"));
    match base {
        "home" | "userProfile" => home.to_path_buf(),
        "appDataRoaming" => roaming,
        "appData" | "dataHome" | "cacheHome" => local,
        "configHome" => home.join(".config"),
        _ => home.to_path_buf(),
    }
}

fn definition_targets_windows(path: &Value) -> bool {
    path.get("targets")
        .and_then(Value::as_array)
        .map(|targets| {
            targets.iter().any(|target| {
                matches!(
                    target.as_str(),
                    Some("windows") | Some("windows10") | Some("windows11")
                )
            })
        })
        .unwrap_or(true)
}

fn definition_adapters(home: &Path) -> Vec<UsageAdapter> {
    let mut adapters = Vec::new();
    for (source, raw) in AITRACKER_DEFINITIONS {
        let Ok(definition) = serde_json::from_str::<Value>(raw) else {
            continue;
        };
        let Some(usage) = definition
            .get("capabilities")
            .and_then(|value| value.get("usage"))
        else {
            continue;
        };
        let Some(paths) = usage.get("paths").and_then(Value::as_array) else {
            continue;
        };
        let mapping = definition_mapping(usage);
        let max_file_bytes = usage
            .get("maxFileSizeBytes")
            .and_then(Value::as_u64)
            .unwrap_or(MAX_FILE_BYTES);
        for path in paths {
            if !definition_targets_windows(path) {
                continue;
            }
            let Some(format) = path
                .get("format")
                .and_then(Value::as_str)
                .and_then(|format| match format {
                    "json" => Some(UsageFormat::Json),
                    "jsonl" => Some(UsageFormat::Jsonl),
                    "sqlite" => Some(UsageFormat::Sqlite),
                    _ => None,
                })
            else {
                continue;
            };
            let glob = path.get("glob").and_then(Value::as_str).unwrap_or("*");
            if glob.ends_with(".zstd") {
                continue;
            }
            let base = path.get("base").and_then(Value::as_str).unwrap_or("home");
            let relative = path.get("path").and_then(Value::as_str).unwrap_or_default();
            let query = path
                .get("query")
                .and_then(Value::as_str)
                .or_else(|| usage.get("query").and_then(Value::as_str))
                .map(str::to_string);
            adapters.push(UsageAdapter {
                source: (*source).to_string(),
                paths: vec![UsagePath {
                    root: definition_base(home, base).join(relative),
                    glob: glob.to_string(),
                    format,
                    query,
                }],
                mapping: mapping.clone(),
                max_file_bytes,
            });
        }
    }
    adapters
}

pub fn builtin_adapters(home: &Path) -> Vec<UsageAdapter> {
    let generic = |source: &str, root: &str, glob: &str| UsageAdapter {
        source: source.into(),
        paths: vec![UsagePath {
            root: home.join(root),
            glob: glob.into(),
            format: if glob.ends_with(".json") {
                UsageFormat::Json
            } else {
                UsageFormat::Jsonl
            },
            query: None,
        }],
        mapping: UsageFieldMapping {
            event_id: vec!["id".into(), "eventId".into(), "message.id".into()],
            timestamp: vec!["timestamp".into(), "created_at".into(), "createdAt".into()],
            session_id: vec!["sessionId".into(), "session_id".into(), "session.id".into()],
            model: vec![
                "model".into(),
                "message.model".into(),
                "providerData.model".into(),
            ],
            provider: vec![
                "provider".into(),
                "providerId".into(),
                "provider_id".into(),
                "modelProvider".into(),
            ],
            agent: vec!["agent".into(), "tool".into(), "client".into()],
            status: vec!["status".into(), "state".into()],
            duration_ms: vec!["duration_ms".into(), "durationMs".into()],
            project: vec!["cwd".into(), "project".into(), "workspace".into()],
            input_tokens: vec![
                "usage.input_tokens".into(),
                "usage.inputTokens".into(),
                "input_tokens".into(),
            ],
            cached_input_tokens: vec![
                "usage.cache_read_input_tokens".into(),
                "usage.cached_input_tokens".into(),
                "cached_input_tokens".into(),
            ],
            cache_creation_input_tokens: vec![
                "usage.cache_creation_input_tokens".into(),
                "usage.cache_write_input_tokens".into(),
                "cache_creation_input_tokens".into(),
            ],
            output_tokens: vec![
                "usage.output_tokens".into(),
                "usage.outputTokens".into(),
                "output_tokens".into(),
            ],
            reasoning_output_tokens: vec![
                "usage.reasoning_output_tokens".into(),
                "reasoning_output_tokens".into(),
            ],
            total_tokens: vec!["usage.total_tokens".into(), "total_tokens".into()],
            tool_name: vec![
                "toolName".into(),
                "tool_name".into(),
                "tool.name".into(),
                "tool".into(),
            ],
            ..UsageFieldMapping::default()
        },
        max_file_bytes: MAX_FILE_BYTES,
    };
    let legacy = [
        ("claude-code", ".claude/projects", "**/*.jsonl"),
        ("codex", ".codex/sessions", "**/*.jsonl"),
        ("cursor", ".cursor", "**/*.jsonl"),
        ("gemini-cli", ".gemini", "**/*.jsonl"),
        ("kimi-code", ".kimi", "**/*.jsonl"),
        ("opencode", ".opencode", "**/*.jsonl"),
        ("grok", ".grok", "**/*.jsonl"),
        ("github-copilot", ".config/github-copilot", "**/*.jsonl"),
        (
            "cline",
            ".config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks",
            "**/*.json",
        ),
        (
            "roo-code",
            ".config/Code/User/globalStorage/rooveterinaryinc.roo-cline/tasks",
            "**/*.json",
        ),
        ("workbuddy", ".workbuddy/projects", "**/*.jsonl"),
        ("openclaw", ".openclaw/agents", "**/*.jsonl"),
        ("antigravity", ".gemini/antigravity", "**/*.jsonl"),
    ];
    let mut adapters = definition_adapters(home);
    for (source, root, glob) in legacy {
        if !adapters.iter().any(|adapter| adapter.source == source) {
            adapters.push(generic(source, root, glob));
        }
    }
    adapters
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn adapter(source: &str) -> UsageAdapter {
        UsageAdapter {
            source: source.into(),
            paths: Vec::new(),
            mapping: UsageFieldMapping {
                timestamp: vec!["timestamp".into()],
                session_id: vec!["sessionId".into()],
                model: vec!["model".into()],
                project: vec!["cwd".into()],
                input_tokens: vec!["usage.input_tokens".into(), "input_tokens".into()],
                cached_input_tokens: vec!["usage.cache_read_input_tokens".into()],
                output_tokens: vec!["usage.output_tokens".into(), "output_tokens".into()],
                reasoning_output_tokens: vec!["usage.reasoning_tokens".into()],
                ..UsageFieldMapping::default()
            },
            max_file_bytes: MAX_FILE_BYTES,
        }
    }

    #[test]
    fn session_ids_are_stable_and_private() {
        let value = Value::String("real-session".into());
        let left = session_id_from_structured("codex", Some(&value)).unwrap();
        assert_eq!(
            left,
            session_id_from_structured("codex", Some(&value)).unwrap()
        );
        assert!(left.starts_with("session_"));
        assert!(!left.contains("real-session"));
    }

    #[test]
    fn project_identity_matches_aitracker_for_git_and_non_path_values() {
        let directory = tempdir().unwrap();
        let root = directory.path().join("workspace");
        let nested = root.join("packages").join("agent");
        fs::create_dir_all(nested.join("src")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();

        assert_eq!(
            canonical_project_label(&nested.join("src").to_string_lossy()),
            Some("workspace".into())
        );
        assert_eq!(canonical_project_label("qi"), Some("qi".into()));
        assert_eq!(
            canonical_project_label("2026-09-23-18-01-11"),
            Some("2026-09-23-18-01-11".into())
        );
    }

    #[test]
    fn linked_worktree_resolves_to_main_repository_name() {
        let directory = tempdir().unwrap();
        let main = directory.path().join("MainRepo");
        let worktree = main
            .join(".claude")
            .join("worktrees")
            .join("auto-name-1a2b3c");
        let admin = main.join(".git").join("worktrees").join("auto-name-1a2b3c");
        fs::create_dir_all(&admin).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", admin.to_string_lossy()),
        )
        .unwrap();

        assert_eq!(
            canonical_project_label(&worktree.to_string_lossy()),
            Some("MainRepo".into())
        );
        assert_eq!(
            canonical_project_label(&worktree.join("src").to_string_lossy()),
            Some("MainRepo".into())
        );
    }

    #[test]
    fn deleted_worktree_still_groups_under_its_repository() {
        let directory = tempdir().unwrap();
        let main = directory.path().join("MainRepo");
        fs::create_dir_all(main.join(".git")).unwrap();
        // The worktree directory itself no longer exists.
        let gone = main
            .join(".claude")
            .join("worktrees")
            .join("old-name-9f8e7d");

        assert_eq!(
            canonical_project_label(&gone.to_string_lossy()),
            Some("MainRepo".into())
        );
    }

    #[test]
    fn worktree_path_groups_by_layout_when_repository_is_gone() {
        let directory = tempdir().unwrap();
        let gone = directory
            .path()
            .join("LostRepo")
            .join(".claude")
            .join("worktrees")
            .join("old-name-9f8e7d");

        assert_eq!(
            canonical_project_label(&gone.to_string_lossy()),
            Some("LostRepo".into())
        );
    }

    #[test]
    fn mapping_counts_all_token_components() {
        let adapter = adapter("claude-code");
        let value = serde_json::json!({"timestamp":"2026-09-27T00:00:00Z","sessionId":"s1","model":"m","cwd":"C:\\work\\demo","usage":{"input_tokens":10,"cache_read_input_tokens":2,"output_tokens":3,"reasoning_tokens":4}});
        let event = event_from_record(&value, &adapter, None).unwrap();
        assert_eq!(event.total_tokens, 19);
        assert_eq!(event.project, "demo");
    }

    #[test]
    fn workbuddy_provider_usage_becomes_real_event() {
        let adapter = UsageAdapter {
            source: "workbuddy".into(),
            paths: Vec::new(),
            mapping: generic_mapping(),
            max_file_bytes: MAX_FILE_BYTES,
        };
        let value = serde_json::json!({
            "id": "event-1",
            "type": "message",
            "sessionId": "session-1",
            "timestamp": 1790157672285_i64,
            "providerData": {
                "model": "auto",
                "usage": {"inputTokens": 52606, "outputTokens": 324, "totalTokens": 52930}
            }
        });
        let event = event_from_record(&value, &adapter, None).unwrap();
        assert_eq!(event.agent, "workbuddy");
        assert_eq!(event.model, "auto");
        assert_eq!(event.total_tokens, 52930);
        assert_eq!(
            event.session_id,
            Some(private_id("workbuddy", "structured", "session-1"))
        );
    }

    #[test]
    fn workbuddy_raw_usage_subtracts_cached_prompt_tokens() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("events.jsonl");
        let value = serde_json::json!({
            "id": "response-1",
            "sessionId": "session-1",
            "timestamp": 1_790_467_200_000_i64,
            "providerData": {
                "requestModelName": "work-model",
                "rawUsage": {
                    "prompt_tokens": 100,
                    "prompt_tokens_details": {"cached_tokens": 80},
                    "completion_tokens": 20,
                    "completion_tokens_details": {"reasoning_tokens": 7}
                }
            }
        });
        fs::write(&path, format!("{}\n", value)).unwrap();
        let adapter = UsageAdapter {
            source: "workbuddy".into(),
            paths: vec![UsagePath {
                root: directory.path().into(),
                glob: "*.jsonl".into(),
                format: UsageFormat::Jsonl,
                query: None,
            }],
            mapping: generic_mapping(),
            max_file_bytes: MAX_FILE_BYTES,
        };
        let (events, _) = scan_adapter(&adapter, 10);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].input_tokens, 20);
        assert_eq!(events[0].cached_input_tokens, 80);
        assert_eq!(events[0].output_tokens, 13);
        assert_eq!(events[0].reasoning_output_tokens, 7);
        assert_eq!(events[0].total_tokens, 120);
        assert_eq!(events[0].model, "work-model");
    }

    #[test]
    fn jsonl_scanner_skips_bad_lines_and_aggregates() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("events.jsonl");
        fs::write(&path, "{bad}\n{\"timestamp\":\"2026-09-27T00:00:00Z\",\"model\":\"m\",\"usage\":{\"input_tokens\":2,\"output_tokens\":3}}\n").unwrap();
        let mut adapter = adapter("fixture");
        adapter.paths.push(UsagePath {
            root: directory.path().into(),
            glob: "*.jsonl".into(),
            format: UsageFormat::Jsonl,
            query: None,
        });
        let (events, diagnostics) = scan_adapter(&adapter, 10);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].total_tokens, 5);
        assert!(
            diagnostics
                .iter()
                .any(|item| item.code == DiagnosticCode::MalformedJson)
        );
        assert_eq!(aggregate(events, diagnostics).mode, "real");
    }

    #[test]
    fn duplicate_event_keeps_larger_snapshot() {
        let adapter = adapter("fixture");
        let value = serde_json::json!({"timestamp":"2026-09-27T00:00:00Z","sessionId":"s1","model":"m","usage":{"input_tokens":2,"output_tokens":3}});
        let event = event_from_record(&value, &adapter, None).unwrap();
        let mut larger = event.clone();
        larger.total_tokens = 8;
        let snapshot = aggregate(vec![event, larger], Vec::new());
        assert_eq!(snapshot.events, 1);
        assert_eq!(snapshot.totals.tokens.total_tokens, 8);
    }

    #[test]
    fn stable_message_identity_deduplicates_changed_usage_and_timestamp() {
        let mut adapter = adapter("fixture");
        adapter.mapping.event_id = vec!["id".into()];
        let first = serde_json::json!({"id":"message-1","sessionId":"session-1","timestamp":"2026-09-27T00:00:00Z","input_tokens":2});
        let later = serde_json::json!({"id":"message-1","sessionId":"session-1","timestamp":"2026-09-27T00:00:01Z","input_tokens":8});
        let snapshot = aggregate(
            vec![
                event_from_record(&first, &adapter, None).unwrap(),
                event_from_record(&later, &adapter, None).unwrap(),
            ],
            Vec::new(),
        );
        assert_eq!(snapshot.events, 1);
        assert_eq!(snapshot.totals.tokens.total_tokens, 8);
    }

    #[test]
    fn native_codex_adapter_reads_context_and_exclusive_cached_input() {
        let directory = tempdir().unwrap();
        let root = directory.path().join(".codex/sessions");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("rollout-fixture.jsonl");
        fs::write(&path, [
            serde_json::json!({"type":"session_meta","payload":{"id":"session-1","cwd":"C:/work/project-1","model_provider":"provider-1"}}),
            serde_json::json!({"type":"turn_context","payload":{"model":"model-1"}}),
            serde_json::json!({"type":"event_msg","timestamp":"2026-09-27T00:00:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"cached_input_tokens":80,"output_tokens":20,"reasoning_output_tokens":7,"total_tokens":120}}}}),
        ].into_iter().map(|line| format!("{line}\n")).collect::<String>()).unwrap();
        let adapter = builtin_adapters(directory.path())
            .into_iter()
            .find(|adapter| adapter.source == "codex")
            .unwrap();
        let (events, diagnostics) = scan_adapter(&adapter, MAX_EVENTS);
        assert!(diagnostics.is_empty());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model, "model-1");
        assert_eq!(events[0].project, "project-1");
        assert_eq!(events[0].input_tokens, 20);
        assert_eq!(events[0].cached_input_tokens, 80);
        assert_eq!(events[0].total_tokens, 127);
        assert_eq!(
            events[0].session_id,
            session_id_from_structured("codex", Some(&Value::String("session-1".into())))
        );
    }

    #[test]
    fn native_claude_adapter_and_legacy_reader_share_event_identity() {
        let directory = tempdir().unwrap();
        let root = directory.path().join(".claude/projects/project-1");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("session-1.jsonl");
        fs::write(&path, [2, 8].into_iter().map(|input| format!("{}\n", serde_json::json!({"type":"assistant","sessionId":"session-1","cwd":"C:/work/project-1","timestamp":"2026-09-27T00:00:00Z","message":{"id":"message-1","model":"model-1","usage":{"input_tokens":input,"output_tokens":1}}}))).collect::<String>()).unwrap();
        let adapter = builtin_adapters(directory.path())
            .into_iter()
            .find(|adapter| adapter.source == "claude-code")
            .unwrap();
        let (mut events, diagnostics) = scan_adapter(&adapter, MAX_EVENTS);
        let (legacy, _) = crate::request_history::read_recent_local_requests(
            &[],
            &directory.path().join(".claude/projects"),
            4096,
        );
        events.extend(legacy.iter().map(event_from_request_record));
        let snapshot = aggregate(events, diagnostics);
        assert_eq!(snapshot.events, 1);
        assert_eq!(snapshot.totals.tokens.total_tokens, 9);
        assert_eq!(snapshot.by_source[0].key, "claude-code");
        assert_eq!(snapshot.details[0].agent, "claude-code");
    }

    #[test]
    fn usage_snapshot_keeps_models_older_than_request_list_budget() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("events.jsonl");
        fs::write(&path, (0..4100).map(|index| format!("{}\n", serde_json::json!({"timestamp":1_790_467_200_000_i64+index,"id":format!("event-{index}"),"model":if index == 0 { "old-model" } else { "new-model" },"input_tokens":1}))).collect::<String>()).unwrap();
        let mut adapter = adapter("fixture");
        adapter.paths.push(UsagePath {
            root: directory.path().into(),
            glob: "*.jsonl".into(),
            format: UsageFormat::Jsonl,
            query: None,
        });
        let (events, diagnostics) = scan_adapter(&adapter, MAX_EVENTS);
        assert!(diagnostics.is_empty());
        assert_eq!(events.len(), 4100);
        assert!(
            aggregate(events, diagnostics)
                .by_model
                .iter()
                .any(|model| model.key == "old-model")
        );
        let (events, diagnostics) = scan_adapter(&adapter, 4096);
        assert_eq!(events.len(), 4096);
        assert!(
            diagnostics
                .iter()
                .any(|item| item.code == DiagnosticCode::Truncated && item.count == 4)
        );
    }

    #[test]
    fn sqlite_adapter_reads_bounded_read_only_rows() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("usage.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE usage (timestamp TEXT, model TEXT, input_tokens INTEGER, output_tokens INTEGER); \
                 INSERT INTO usage VALUES ('2026-09-27T00:00:00Z', 'sqlite-model', 7, 5);",
            )
            .unwrap();
        let mut adapter = adapter("sqlite-fixture");
        adapter.max_file_bytes = 1;
        adapter.paths.push(UsagePath {
            root: path,
            glob: String::new(),
            format: UsageFormat::Sqlite,
            query: Some("SELECT timestamp, model, input_tokens, output_tokens FROM usage".into()),
        });
        let (events, diagnostics) = scan_adapter(&adapter, 10);
        assert!(diagnostics.is_empty());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].total_tokens, 12);
        assert_eq!(events[0].model, "sqlite-model");
    }

    #[test]
    fn builtin_adapters_cover_every_definition_with_usage_paths() {
        let home = tempdir().unwrap();
        let adapters = builtin_adapters(home.path());
        let sources = adapters
            .iter()
            .map(|adapter| adapter.source.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let expected = AITRACKER_DEFINITIONS
            .iter()
            .filter_map(|(_, raw)| serde_json::from_str::<Value>(raw).ok())
            .filter(|definition| {
                definition
                    .get("capabilities")
                    .and_then(|capabilities| capabilities.get("usage"))
                    .and_then(|usage| usage.get("paths"))
                    .and_then(Value::as_array)
                    .is_some_and(|paths| !paths.is_empty())
            })
            .count();
        assert_eq!(sources.len(), expected);
        assert!(sources.contains("codex"));
        assert!(sources.contains("claude-code"));
    }

    #[test]
    fn sqlite_directory_glob_scans_nested_databases() {
        let directory = tempdir().unwrap();
        let nested = directory.path().join("nested");
        fs::create_dir_all(&nested).unwrap();
        for (name, model) in [("first.sqlite", "first"), ("second.sqlite", "second")] {
            let path = nested.join(name);
            let connection = Connection::open(path).unwrap();
            connection
                .execute_batch(&format!(
                    "CREATE TABLE usage (timestamp TEXT, model TEXT, input_tokens INTEGER, output_tokens INTEGER); INSERT INTO usage VALUES ('2026-09-27T00:00:00Z', '{model}', 1, 2);"
                ))
                .unwrap();
        }
        let mut adapter = adapter("sqlite-directory-fixture");
        adapter.paths.push(UsagePath {
            root: directory.path().into(),
            glob: "**/*.sqlite".into(),
            format: UsageFormat::Sqlite,
            query: Some("SELECT timestamp, model, input_tokens, output_tokens FROM usage".into()),
        });
        let (events, diagnostics) = scan_adapter(&adapter, 10);
        assert!(diagnostics.is_empty());
        assert_eq!(events.len(), 2);
        assert!(events.iter().any(|event| event.model == "first"));
        assert!(events.iter().any(|event| event.model == "second"));
    }

    #[test]
    #[ignore = "Explicit opt-in: reads local usage, prints only aggregate statistics"]
    fn local_usage_read_only_parity_smoke() {
        let home = env::var_os("USERPROFILE")
            .or_else(|| env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap();
        let started = std::time::Instant::now();
        let mut events = Vec::new();
        let mut diagnostics = Vec::new();
        for adapter in builtin_adapters(&home).into_iter().filter(|adapter| {
            matches!(
                adapter.source.as_str(),
                "codex" | "claude-code" | "workbuddy"
            )
        }) {
            let (found, warnings) = scan_adapter(&adapter, MAX_EVENTS);
            events.extend(found);
            diagnostics.extend(warnings);
        }
        let snapshot = aggregate(events, diagnostics);
        let first_day = Local::now().date_naive() - chrono::Duration::days(6);
        let mut seven_day = LocalUsageTotals::default();
        let mut seven_day_sources = HashMap::<String, LocalUsageTotals>::new();
        for event in snapshot.details.iter().filter(|event| {
            DateTime::parse_from_rfc3339(&event.timestamp)
                .map(|date| date.with_timezone(&Local).date_naive() >= first_day)
                .unwrap_or(false)
        }) {
            add_event(&mut seven_day, event);
            add_event(
                seven_day_sources.entry(event.source.clone()).or_default(),
                event,
            );
        }
        println!(
            "{}",
            serde_json::json!({"events":snapshot.events,"models":snapshot.by_model.len(),"sources":snapshot.by_source,"tokens":snapshot.totals.tokens,"seven_day":seven_day,"seven_day_sources":seven_day_sources,"truncations":snapshot.diagnostics.iter().filter(|item| item.code == DiagnosticCode::Truncated).map(|item| serde_json::json!({"source":item.source,"count":item.count,"message":item.message})).collect::<Vec<_>>(),"elapsed_ms":started.elapsed().as_millis()})
        );
        assert!(snapshot.events > 0);
    }
}
