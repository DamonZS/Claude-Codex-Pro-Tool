use claude_codex_pro_core::request_telemetry::RequestRecord;
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_DATABASES: usize = 8;
const MAX_FILES: usize = 32;
const MAX_RECORDS: usize = 500;
const TAIL_BYTES: u64 = 512 * 1024;
const HEAD_BYTES: u64 = 64 * 1024;
const MAX_ENTRIES: usize = 4096;
const COVERAGE: &str = "本地用量仅覆盖前 8 个数据库、最近 32 个 Codex 会话和 32 个 Claude 文件；每文件尾部 512 KiB，Codex 另读头部 64 KiB。Claude 最多扫描 4096 个条目、向下 3 层目录。最多返回 500 条，实际条数受页面上限约束。HTTP 状态、协议、耗时及流式状态未观测；缺失用量、供应商和模型保持未知，不代表完整请求日志。Codex 同会话的异路径副本可能重复计数。";
const TRUNCATED: &str = "已达到文件、目录或记录读取上限，仅展示覆盖范围内的近期用量。";
const READ_ERROR: &str = "部分本地用量来源读取失败，已跳过。";
const PARSE_ERROR: &str = "部分本地用量记录不完整或格式异常，已跳过。";

// Only deserialize telemetry fields. Message content, prompts and credentials are ignored.
#[derive(Default, Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    #[serde(default)]
    payload: Payload,
    message: Option<Message>,
}

#[derive(Default, Deserialize)]
struct Payload {
    #[serde(rename = "type")]
    kind: Option<String>,
    model: Option<String>,
    model_provider: Option<String>,
    info: Option<Info>,
}

#[derive(Deserialize)]
struct Info {
    last_token_usage: Option<Usage>,
    total_token_usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Clone, Default, Deserialize, Hash, PartialEq, Eq)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    #[serde(alias = "cache_read_input_tokens")]
    cached_input_tokens: Option<u64>,
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
/// At most 8 databases, 32 files per agent, 512 KiB per tail (plus 64 KiB Codex
/// metadata header), 4096 Claude directory entries and 500 returned records.
/// Codex selection uses updated_at_ms, then updated_at seconds, then created_at_ms;
/// at most 33 rows per database are inspected before path deduplication. Claude
/// files are selected by modification time within the bounded traversal (root
/// plus three directory levels, without following directory symlinks).
/// Tail reads include one extra byte to find a complete line boundary. Long or
/// partial lines outside these windows are skipped; model context preceding the
/// tail is unknown. No turn duration is substituted for HTTP latency. The DTO's
/// required streaming bool is false for local observations, not proof of a
/// non-streaming request. At most four distinct, static warning strings return.
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
        if item.source == "claude_code" {
            // Normalize only after merging fragments, including cross-file copies.
            item.input_tokens = item
                .input_tokens
                .zip(item.cached_tokens)
                .and_then(|(input, cached)| input.checked_add(cached))
                .zip(item.cache_creation_tokens)
                .and_then(|(input, created)| input.checked_add(created));
        }
        if item.total_tokens.is_none() {
            item.total_tokens = item
                .input_tokens
                .zip(item.output_tokens)
                .and_then(|(input, output)| input.checked_add(output));
        }
    }
    (records, warnings)
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
                if depth < 3 {
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

fn read_bytes(path: &Path, header: bool, warnings: &mut Vec<String>) -> Option<Vec<u8>> {
    let result = (|| -> std::io::Result<Vec<u8>> {
        let mut file = File::open(path)?;
        let len = file.metadata()?.len();
        let start = if header {
            0
        } else {
            len.saturating_sub(TAIL_BYTES)
        };
        if start > 0 {
            warn(warnings, TRUNCATED);
        }
        // Read one preceding byte so an exact line-boundary tail keeps its first line.
        file.seek(SeekFrom::Start(start.saturating_sub(1)))?;
        let mut bytes = Vec::new();
        file.take(if header { HEAD_BYTES } else { TAIL_BYTES + 1 })
            .read_to_end(&mut bytes)?;
        if start > 0 {
            let next = bytes
                .iter()
                .position(|byte| *byte == b'\n')
                .map(|i| i + 1)
                .unwrap_or(bytes.len());
            bytes.drain(..next);
        }
        if header && len > HEAD_BYTES {
            let end = bytes
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            bytes.truncate(end);
        }
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

fn record(id: String, time: u64, source: &str, agent: &str, usage: &Usage) -> RequestRecord {
    RequestRecord {
        id,
        timestamp_ms: time,
        source: source.to_string(),
        agent: agent.to_string(),
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
    if let Some(bytes) = read_bytes(path, true, warnings) {
        for line in lines(&bytes, warnings) {
            if line.kind.as_deref() == Some("session_meta") {
                provider = telemetry_identifier(line.payload.model_provider, 128);
                break;
            }
        }
    }
    let Some(bytes) = read_bytes(path, false, warnings) else {
        return;
    };
    let mut model = None;
    let mut seen = HashSet::new();
    for line in lines(&bytes, warnings) {
        match line.kind.as_deref() {
            Some("session_meta") => {
                provider = telemetry_identifier(line.payload.model_provider, 128)
            }
            Some("turn_context") => model = telemetry_identifier(line.payload.model, 200),
            Some("event_msg") if line.payload.kind.as_deref() == Some("token_count") => {
                let Some(time) = timestamp(line.timestamp.as_deref()) else {
                    continue;
                };
                let Some(info) = line.payload.info else {
                    continue;
                };
                let Some(usage) = info.last_token_usage else {
                    continue;
                };
                if usage == Usage::default() {
                    continue;
                }
                // Cumulative usage is a notification identity only, never per-request usage.
                let key = if let Some(total) =
                    info.total_token_usage.filter(|u| *u != Usage::default())
                {
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
                    records.push(item);
                }
            }
            _ => {}
        }
    }
}

fn read_claude(path: &Path, records: &mut Vec<RequestRecord>, warnings: &mut Vec<String>) {
    let Some(bytes) = read_bytes(path, false, warnings) else {
        return;
    };
    let mut messages: HashMap<String, RequestRecord> = HashMap::new();
    for line in lines(&bytes, warnings) {
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
        item.model = telemetry_identifier(message.model, 200);
        // Repeated assistant fragments report snapshots, not incremental usage.
        if let Some(previous) = messages.get_mut(&key) {
            merge_snapshot(previous, item);
        } else {
            messages.insert(key, item);
        }
    }
    records.extend(messages.into_values());
}
