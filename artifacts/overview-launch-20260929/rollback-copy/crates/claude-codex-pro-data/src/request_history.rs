use claude_codex_pro_core::request_telemetry::RequestRecord;
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_DATABASES: usize = 32;
const MAX_FILES: usize = 1_200;
const MAX_RECORDS: usize = 4_096;
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ENTRIES: usize = 30_000;
const MAX_DEPTH: usize = 32;
const COVERAGE: &str = "本地用量按 AITracker 方式扫描最近 10 年内最多 32 个数据库、1200 个 Codex/Claude JSONL 文件和 30000 个目录条目；单文件最多 256 MiB，最多返回 4096 条规范化记录。HTTP 状态、协议、耗时及流式状态仅来自 CCP 代理记录；本地会话缺失字段保持未知。";
const TRUNCATED: &str = "已达到文件、目录或记录读取上限，仅展示覆盖范围内的近期用量。";
const READ_ERROR: &str = "部分本地用量来源读取失败，已跳过。";
const PARSE_ERROR: &str = "部分本地用量记录不完整或格式异常，已跳过。";

// Only deserialize telemetry fields. Message content, prompts and credentials are ignored.
#[derive(Default, Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    id: Option<String>,
    #[serde(default)]
    payload: Payload,
    message: Option<Message>,
    model: Option<String>,
    #[serde(alias = "sessionId", alias = "session_id")]
    session_id: Option<String>,
    #[serde(alias = "project")]
    cwd: Option<String>,
    #[serde(alias = "providerId", alias = "provider_id")]
    provider: Option<String>,
    #[serde(alias = "modelProvider", alias = "model_provider")]
    model_provider: Option<String>,
}

#[derive(Default, Deserialize)]
struct Payload {
    #[serde(rename = "type")]
    kind: Option<String>,
    id: Option<String>,
    model: Option<String>,
    model_provider: Option<String>,
    #[serde(alias = "sessionId", alias = "session_id")]
    session_id: Option<String>,
    #[serde(alias = "cwd", alias = "project")]
    cwd: Option<String>,
    info: Option<Info>,
    msg: Option<TokenPayload>,
    timestamp: Option<String>,
    duration_ms: Option<u64>,
}

#[derive(Clone, Default, Deserialize)]
struct Info {
    last_token_usage: Option<Usage>,
    total_token_usage: Option<Usage>,
}

#[derive(Clone, Default, Deserialize)]
struct TokenPayload {
    #[serde(rename = "type")]
    kind: Option<String>,
    info: Option<Info>,
    timestamp: Option<String>,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
    #[serde(alias = "sessionId", alias = "session_id")]
    session_id: Option<String>,
    #[serde(alias = "cwd", alias = "project")]
    cwd: Option<String>,
    #[serde(alias = "modelProvider", alias = "model_provider")]
    model_provider: Option<String>,
    provider: Option<String>,
}

#[derive(Clone, Default, Deserialize, Hash, PartialEq, Eq)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    #[serde(alias = "cache_read_input_tokens")]
    cached_input_tokens: Option<u64>,
    #[serde(alias = "cache_write_input_tokens")]
    cache_creation_input_tokens: Option<u64>,
    reasoning_output_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

fn warn(warnings: &mut Vec<String>, text: &str) {
    if !warnings.iter().any(|value| value == text) {
        warnings.push(text.to_string());
    }
}

fn opaque_id(value: impl Hash) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn telemetry_identifier(value: Option<String>, max_len: usize) -> Option<String> {
    value.filter(|value| {
        let lowered = value.to_ascii_lowercase();
        !value.is_empty()
            && value.len() <= max_len
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
            && !lowered.starts_with("www.")
            && ![
                "sk-",
                "bearer",
                "secret",
                "api_key",
                "api-key",
                "apikey",
                "access_token",
                "auth_token",
            ]
            .iter()
            .any(|pattern| lowered.contains(pattern))
    })
}

/// Read-only, bounded local observations, newest first. This is not an HTTP audit log.
/// At most 32 databases, 1200 files per agent, 256 MiB per file, 30000 Claude
/// directory entries and 4096 returned records are inspected. Codex selection
/// uses updated_at_ms, then updated_at seconds, then created_at_ms. Claude files
/// are selected by modification time within the bounded traversal without
/// following directory symlinks. No turn duration is substituted for HTTP
/// latency. The DTO's required streaming bool is false for local observations,
/// not proof of a non-streaming request. At most four distinct, static warning
/// strings return.
/// Claude input includes cache read and creation only when all three components
/// are known. Total tokens are reported or derived from known input and output.
/// Codex deduplication is scoped to the rollout path, so different-path copies
/// of the same session may be counted twice. Provider/model identifiers use an
/// ASCII allowlist and reject recognizable credential forms; invalid values are unknown.
pub fn read_recent_local_requests(
    db_paths: &[PathBuf],
    claude_projects: &Path,
    limit: usize,
) -> (Vec<RequestRecord>, Vec<String>) {
    let mut warnings = vec![COVERAGE.to_string()];
    let limit = limit.min(MAX_RECORDS);
    if limit == 0 {
        return (Vec::new(), warnings);
    }
    let mut records = Vec::new();
    for path in recent_rollouts(db_paths, &mut warnings) {
        read_codex(&path, &mut records, &mut warnings);
        trim_records(&mut records, MAX_RECORDS, &mut warnings);
    }
    for path in recent_claude_files(claude_projects, &mut warnings) {
        read_claude(&path, &mut records, &mut warnings);
        trim_records(&mut records, MAX_RECORDS, &mut warnings);
    }
    trim_records(&mut records, limit, &mut warnings);
    for item in &mut records {
        normalize_local_usage(item);
    }
    (records, warnings)
}

fn normalize_local_usage(item: &mut RequestRecord) {
    if item.total_tokens.is_none() {
        item.total_tokens = [
            item.input_tokens,
            item.output_tokens,
            item.cached_tokens,
            item.cache_creation_tokens,
            item.reasoning_tokens,
        ]
        .into_iter()
        .flatten()
        .try_fold(0u64, u64::checked_add);
    }
}

fn trim_records(records: &mut Vec<RequestRecord>, limit: usize, warnings: &mut Vec<String>) {
    // The same message can occur in both a main and a subagent transcript.
    let mut unique: HashMap<String, RequestRecord> = HashMap::new();
    for item in records.drain(..) {
        if let Some(previous) = unique.get_mut(&item.id) {
            merge_snapshot(previous, item);
        } else {
            unique.insert(item.id.clone(), item);
        }
    }
    records.extend(unique.into_values());
    records.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms).then(a.id.cmp(&b.id)));
    if records.len() > limit {
        warn(warnings, TRUNCATED);
        records.truncate(limit);
    }
}

fn merge_snapshot(previous: &mut RequestRecord, item: RequestRecord) {
    previous.timestamp_ms = previous.timestamp_ms.min(item.timestamp_ms);
    previous.input_tokens = previous.input_tokens.max(item.input_tokens);
    previous.output_tokens = previous.output_tokens.max(item.output_tokens);
    previous.cached_tokens = previous.cached_tokens.max(item.cached_tokens);
    previous.cache_creation_tokens = previous
        .cache_creation_tokens
        .max(item.cache_creation_tokens);
    previous.reasoning_tokens = previous.reasoning_tokens.max(item.reasoning_tokens);
    previous.total_tokens = previous.total_tokens.max(item.total_tokens);
    if item.model.is_some() {
        previous.model = item.model;
    }
    if previous.session_id.is_none() {
        previous.session_id = item.session_id;
    }
    if previous.project.is_none() {
        previous.project = item.project;
    }
}

fn recent_rollouts(paths: &[PathBuf], warnings: &mut Vec<String>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if paths.len() > MAX_DATABASES {
        warn(warnings, TRUNCATED);
    }
    for path in paths.iter().take(MAX_DATABASES) {
        let result = (|| -> rusqlite::Result<Vec<(i64, PathBuf)>> {
            let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            db.busy_timeout(Duration::from_millis(50))?;
            let mut stmt = db.prepare("PRAGMA table_info(threads)")?;
            let columns = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<rusqlite::Result<HashSet<_>>>()?;
            if !columns.contains("rollout_path") {
                return Ok(Vec::new());
            }
            let updated = if columns.contains("updated_at_ms") {
                "updated_at_ms"
            } else if columns.contains("updated_at") {
                "updated_at * 1000"
            } else if columns.contains("created_at_ms") {
                "created_at_ms"
            } else {
                "NULL"
            };
            let mut stmt = db.prepare(&format!(
                "SELECT COALESCE({updated}, 0), rollout_path FROM threads \
                 WHERE rollout_path IS NOT NULL AND rollout_path <> '' \
                 ORDER BY COALESCE({updated}, 0) DESC, rollout_path LIMIT ?1"
            ))?;
            stmt.query_map([MAX_FILES as i64 + 1], |row| {
                Ok((row.get(0)?, PathBuf::from(row.get::<_, String>(1)?)))
            })?
            .collect()
        })();
        match result {
            Ok(rows) => candidates.extend(rows),
            Err(_) => warn(warnings, READ_ERROR),
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut seen = HashSet::new();
    candidates.retain(|(_, path)| seen.insert(path.clone()));
    if candidates.len() > MAX_FILES {
        warn(warnings, TRUNCATED);
    }
    candidates
        .into_iter()
        .take(MAX_FILES)
        .map(|(_, p)| p)
        .collect()
}

fn recent_claude_files(root: &Path, warnings: &mut Vec<String>) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }
    let mut pending = vec![(root.to_path_buf(), 0)];
    let mut files = Vec::new();
    let mut count = 0;
    while let Some((dir, depth)) = pending.pop() {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => {
                warn(warnings, READ_ERROR);
                continue;
            }
        };
        for entry in entries {
            count += 1;
            if count > MAX_ENTRIES {
                warn(warnings, TRUNCATED);
                pending.clear();
                break;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    warn(warnings, READ_ERROR);
                    continue;
                }
            };
            let kind = match entry.file_type() {
                Ok(kind) => kind,
                Err(_) => {
                    warn(warnings, READ_ERROR);
                    continue;
                }
            };
            if kind.is_dir() {
                if depth < MAX_DEPTH {
                    pending.push((entry.path(), depth + 1));
                } else {
                    warn(warnings, TRUNCATED);
                }
            } else if kind.is_file() && entry.path().extension().is_some_and(|ext| ext == "jsonl") {
                match entry.metadata().and_then(|meta| meta.modified()) {
                    Ok(time) => files.push((time, entry.path())),
                    Err(_) => warn(warnings, READ_ERROR),
                }
            }
        }
    }
    files.sort_by(|a: &(SystemTime, PathBuf), b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    if files.len() > MAX_FILES {
        warn(warnings, TRUNCATED);
    }
    files.into_iter().take(MAX_FILES).map(|(_, p)| p).collect()
}

fn read_bytes(path: &Path, _header: bool, warnings: &mut Vec<String>) -> Option<Vec<u8>> {
    let result = (|| -> std::io::Result<Vec<u8>> {
        let mut file = File::open(path)?;
        let len = file.metadata()?.len();
        if len > MAX_FILE_BYTES {
            warn(warnings, TRUNCATED);
            return Ok(Vec::new());
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    })();
    match result {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            warn(warnings, READ_ERROR);
            None
        }
    }
}

fn lines<'a>(bytes: &'a [u8], warnings: &'a mut Vec<String>) -> impl Iterator<Item = Line> + 'a {
    bytes.split(|byte| *byte == b'\n').filter_map(|line| {
        if line.iter().all(u8::is_ascii_whitespace) {
            return None;
        }
        match serde_json::from_slice(line) {
            Ok(value) => Some(value),
            Err(_) => {
                warn(warnings, PARSE_ERROR);
                None
            }
        }
    })
}

fn timestamp(value: Option<&str>) -> Option<u64> {
    chrono::DateTime::parse_from_rfc3339(value?)
        .ok()?
        .timestamp_millis()
        .try_into()
        .ok()
}

fn line_type(line: &Line) -> Option<&str> {
    line.kind
        .as_deref()
        .or(line.payload.kind.as_deref())
        .or_else(|| {
            line.payload
                .msg
                .as_ref()
                .and_then(|msg| msg.kind.as_deref())
        })
}

fn token_info(line: &Line) -> Option<(Info, Option<String>)> {
    let payload = &line.payload;
    if line.kind.as_deref() == Some("token_count") || payload.kind.as_deref() == Some("token_count")
    {
        return payload.info.clone().map(|info| {
            (
                info,
                payload.timestamp.clone().or_else(|| line.timestamp.clone()),
            )
        });
    }
    payload
        .msg
        .as_ref()
        .filter(|msg| msg.kind.as_deref() == Some("token_count"))
        .and_then(|msg| {
            msg.info.clone().map(|info| {
                (
                    info,
                    msg.timestamp.clone().or_else(|| line.timestamp.clone()),
                )
            })
        })
}

fn usage_delta(current: &Usage, previous: &Usage) -> Usage {
    let delta = |current: Option<u64>, previous: Option<u64>| {
        current
            .zip(previous)
            .map(|(current, previous)| current.saturating_sub(previous))
    };
    let input_tokens = delta(current.input_tokens, previous.input_tokens);
    let output_tokens = delta(current.output_tokens, previous.output_tokens);
    let cached_input_tokens = delta(current.cached_input_tokens, previous.cached_input_tokens);
    let cache_creation_input_tokens = delta(
        current.cache_creation_input_tokens,
        previous.cache_creation_input_tokens,
    );
    let reasoning_output_tokens = delta(
        current.reasoning_output_tokens,
        previous.reasoning_output_tokens,
    );
    let component_values = [
        input_tokens,
        output_tokens,
        cached_input_tokens,
        cache_creation_input_tokens,
        reasoning_output_tokens,
    ];
    let total_tokens = if component_values.iter().any(Option::is_some) {
        component_values
            .into_iter()
            .flatten()
            .try_fold(0u64, u64::checked_add)
    } else {
        delta(current.total_tokens, previous.total_tokens)
    };
    Usage {
        input_tokens,
        output_tokens,
        cached_input_tokens,
        cache_creation_input_tokens,
        reasoning_output_tokens,
        total_tokens,
    }
}

fn has_positive_usage(usage: &Usage) -> bool {
    [
        usage.input_tokens,
        usage.output_tokens,
        usage.cached_input_tokens,
        usage.cache_creation_input_tokens,
        usage.reasoning_output_tokens,
        usage.total_tokens,
    ]
    .into_iter()
    .flatten()
    .any(|value| value > 0)
}

fn record(id: String, time: u64, source: &str, agent: &str, usage: &Usage) -> RequestRecord {
    RequestRecord {
        id,
        timestamp_ms: time,
        source: source.to_string(),
        agent: agent.to_string(),
        session_id: None,
        project: None,
        provider: None,
        model: None,
        protocol: None,
        upstream_protocol: None,
        status: "observed".to_string(),
        http_status: None,
        duration_ms: None,
        first_byte_ms: None,
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cached_tokens: usage.cached_input_tokens,
        cache_creation_tokens: usage.cache_creation_input_tokens,
        reasoning_tokens: usage.reasoning_output_tokens,
        total_tokens: usage.total_tokens,
        streaming: false,
    }
}

fn read_codex(path: &Path, records: &mut Vec<RequestRecord>, warnings: &mut Vec<String>) {
    let mut provider = None;
    let mut session_id = None;
    let mut project = None;
    if let Some(bytes) = read_bytes(path, true, warnings) {
        for line in lines(&bytes, warnings) {
            if line_type(&line) == Some("session_meta") {
                provider =
                    telemetry_identifier(line.payload.model_provider.or(line.model_provider), 128);
                session_id = line
                    .payload
                    .id
                    .or(line.payload.session_id)
                    .or(line.session_id)
                    .or(line.id)
                    .and_then(|value| telemetry_identifier(Some(value), 200));
                project = line.payload.cwd.or(line.cwd).and_then(sanitize_project);
                break;
            }
        }
    }
    let Some(bytes) = read_bytes(path, false, warnings) else {
        return;
    };
    let mut model = None;
    let mut previous_usage: Option<Usage> = None;
    let mut seen = HashSet::new();
    let mut last_record_index: Option<usize> = None;
    for line in lines(&bytes, warnings) {
        match line_type(&line) {
            Some("session_meta") => {
                provider =
                    telemetry_identifier(line.payload.model_provider.or(line.model_provider), 128);
                session_id = line
                    .payload
                    .id
                    .or(line.payload.session_id)
                    .or(line.session_id)
                    .or(line.id)
                    .and_then(|value| telemetry_identifier(Some(value), 200));
                project = line.payload.cwd.or(line.cwd).and_then(sanitize_project);
            }
            Some("turn_context") => {
                model = telemetry_identifier(line.payload.model, 200);
                if project.is_none() {
                    project = line.payload.cwd.or(line.cwd).and_then(sanitize_project);
                }
            }
            Some("event_msg") if line.payload.kind.as_deref() == Some("task_complete") => {
                if let (Some(index), Some(duration_ms)) =
                    (last_record_index, line.payload.duration_ms)
                {
                    records[index].duration_ms = Some(duration_ms);
                }
            }
            Some("task_complete") => {
                if let (Some(index), Some(duration_ms)) =
                    (last_record_index, line.payload.duration_ms)
                {
                    records[index].duration_ms = Some(duration_ms);
                }
            }
            Some("event_msg") | Some("token_count") => {
                let Some((info, event_timestamp)) = token_info(&line) else {
                    continue;
                };
                let Some(time) =
                    timestamp(line.timestamp.as_deref().or(event_timestamp.as_deref()))
                else {
                    continue;
                };
                let total = info.total_token_usage.clone().filter(has_positive_usage);
                let usage = info
                    .last_token_usage
                    .clone()
                    .filter(has_positive_usage)
                    .or_else(|| {
                        total
                            .as_ref()
                            .zip(previous_usage.as_ref())
                            .map(|(current, previous)| usage_delta(current, previous))
                    });
                previous_usage = total.clone();
                let Some(usage) = usage.filter(has_positive_usage) else {
                    continue;
                };
                // Cumulative usage is a notification identity only, never per-request usage.
                let key = if let Some(total) = total {
                    opaque_id((path, total))
                } else {
                    opaque_id((path, time, &usage))
                };
                if seen.insert(key.clone()) {
                    let mut item = record(
                        format!("codex-local-{key}"),
                        time,
                        "codex_rollout",
                        "codex",
                        &usage,
                    );
                    item.provider = provider.clone();
                    item.model = model.clone();
                    item.session_id = session_id.clone().or_else(|| rollout_session_id(path));
                    item.project = project.clone();
                    records.push(item);
                    last_record_index = Some(records.len() - 1);
                }
            }
            _ => {}
        }
    }
}

fn sanitize_project(value: String) -> Option<String> {
    let value = value.trim().trim_end_matches(['/', '\\']);
    let project = value.rsplit(['/', '\\']).next()?.trim();
    (value.contains(['/', '\\'])
        && !project.is_empty()
        && project.len() <= 128
        && !project.chars().any(|character| character.is_control()))
    .then(|| project.to_string())
}

fn rollout_session_id(path: &Path) -> Option<String> {
    Some(format!("local-session-{}", opaque_id(("codex-file", path))))
}

fn read_claude(path: &Path, records: &mut Vec<RequestRecord>, warnings: &mut Vec<String>) {
    let Some(bytes) = read_bytes(path, false, warnings) else {
        return;
    };
    let mut messages: HashMap<String, RequestRecord> = HashMap::new();
    let mut session_id = None;
    let mut project = None;
    let mut provider = None;
    let fallback_session_id = claude_file_session_id(path);
    for line in lines(&bytes, warnings) {
        let line_session_id = line
            .session_id
            .or(line
                .message
                .as_ref()
                .and_then(|message| message.session_id.clone()))
            .and_then(|value| telemetry_identifier(Some(value), 200));
        let line_project = line
            .cwd
            .or(line
                .message
                .as_ref()
                .and_then(|message| message.cwd.clone()))
            .and_then(sanitize_project);
        if line_session_id.is_some() {
            session_id = line_session_id.clone();
        }
        if line_project.is_some() {
            project = line_project.clone();
        }
        provider = telemetry_identifier(
            line.model_provider
                .clone()
                .or_else(|| line.provider.clone()),
            128,
        )
        .or(provider);
        if line.kind.as_deref() != Some("assistant") {
            continue;
        }
        let Some(time) = timestamp(line.timestamp.as_deref()) else {
            continue;
        };
        let Some(message) = line.message else {
            continue;
        };
        let Some(usage) = message.usage else {
            continue;
        };
        if usage == Usage::default() {
            continue;
        }
        let key = match message.id.filter(|id| !id.is_empty()) {
            Some(id) => opaque_id(id),
            None => opaque_id((path, time, &usage)),
        };
        let mut item = record(
            format!("claude-local-{key}"),
            time,
            "claude_code",
            "claude",
            &usage,
        );
        item.provider = telemetry_identifier(
            message
                .model_provider
                .clone()
                .or_else(|| message.provider.clone()),
            128,
        )
        .or_else(|| provider.clone());
        item.model = telemetry_identifier(message.model.clone(), 200)
            .or_else(|| telemetry_identifier(line.model, 200));
        item.session_id = line_session_id
            .or_else(|| session_id.clone())
            .or_else(|| fallback_session_id.clone());
        item.project = line_project.or_else(|| project.clone());
        // Repeated assistant fragments report snapshots, not incremental usage.
        if let Some(previous) = messages.get_mut(&key) {
            merge_snapshot(previous, item);
        } else {
            messages.insert(key, item);
        }
    }
    records.extend(messages.into_values());
}

fn claude_file_session_id(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy();
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    Some(format!(
        "local-session-{}",
        opaque_id(("claude-file", parent, file_name.as_ref()))
    ))
}
