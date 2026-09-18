//! Bounded, body-free observations of requests that actually traverse the helper.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_REQUEST_LOG_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_RECENT_REQUESTS: usize = 2_000;
const MAX_RECORD_BYTES: usize = 4_096;
const MAX_PARSE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    pub id: String,
    pub timestamp_ms: u64,
    pub source: String,
    pub agent: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub protocol: Option<String>,
    pub upstream_protocol: Option<String>,
    pub status: String,
    pub http_status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub first_byte_ms: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_tokens: Option<u64>,
    pub cache_creation_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub streaming: bool,
}

pub fn request_log_path() -> PathBuf {
    crate::paths::default_settings_path().with_file_name("requests.jsonl")
}

struct LogLock(File);

impl LogLock {
    fn acquire(path: &Path, exclusive: bool) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            crate::settings::create_private_dir_all(parent).map_err(io::Error::other)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.with_extension("lock"))?;
        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            let result = if exclusive {
                FileExt::try_lock_exclusive(&file)
            } else {
                FileExt::try_lock_shared(&file)
            };
            match result {
                Ok(()) => break,
                Err(error)
                    if (error.kind() == io::ErrorKind::WouldBlock
                        || error.raw_os_error() == fs2::lock_contended_error().raw_os_error())
                        && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error),
            }
        }
        Ok(Self(file))
    }
}

impl Drop for LogLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

pub fn append_request_at(path: &Path, record: &RequestRecord) -> io::Result<()> {
    let mut line = serde_json::to_vec(record)?;
    line.push(b'\n');
    if line.len() > MAX_RECORD_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "record_too_large",
        ));
    }
    let _lock = LogLock::acquire(path, true)?;
    let length = match std::fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
        Err(error) => return Err(error),
    };
    if length + line.len() as u64 + 1 > MAX_REQUEST_LOG_BYTES {
        let rotated = path.with_extension("jsonl.1");
        match std::fs::remove_file(&rotated) {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
        std::fs::rename(path, rotated)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    // Recover a torn final append without combining it with the next record.
    if file.metadata()?.len() > 0 {
        let mut tail = File::open(path)?;
        tail.seek(SeekFrom::End(-1))?;
        let mut last = [0];
        tail.read_exact(&mut last)?;
        if last[0] != b'\n' {
            file.write_all(b"\n")?;
        }
    }
    file.write_all(&line)
}

pub fn read_recent_requests(limit: usize) -> io::Result<Vec<RequestRecord>> {
    read_recent_requests_at(&request_log_path(), limit)
}

pub fn read_recent_requests_at(path: &Path, limit: usize) -> io::Result<Vec<RequestRecord>> {
    let limit = limit.min(MAX_RECENT_REQUESTS);
    if limit == 0 || !path.parent().is_some_and(Path::exists) {
        return Ok(Vec::new());
    }
    let _lock = LogLock::acquire(path, false)?;
    let mut records = Vec::new();
    for generation in [path.to_path_buf(), path.with_extension("jsonl.1")] {
        let mut file = match File::open(generation) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        let start = file.metadata()?.len().saturating_sub(MAX_REQUEST_LOG_BYTES);
        file.seek(SeekFrom::Start(start))?;
        let mut bytes = Vec::new();
        file.take(MAX_REQUEST_LOG_BYTES).read_to_end(&mut bytes)?;
        // Ignore both an initial partial line and a torn final append.
        let first = if start > 0 {
            bytes.iter().position(|byte| *byte == b'\n').map(|i| i + 1)
        } else {
            Some(0)
        };
        let Some((first, last)) = first.zip(bytes.iter().rposition(|byte| *byte == b'\n')) else {
            continue;
        };
        if first > last {
            continue;
        }
        for line in bytes[first..last].rsplit(|byte| *byte == b'\n') {
            if line.len() <= MAX_RECORD_BYTES {
                if let Ok(record) = serde_json::from_slice::<RequestRecord>(line) {
                    if let Some(record) = sanitize_record(record) {
                        records.push(record);
                    }
                }
            }
        }
    }
    records.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
    records.truncate(limit);
    Ok(records)
}

#[derive(Debug, Clone, Copy)]
pub enum RequestFailure {
    Network,
    Stream,
    ClientDisconnect,
    InvalidResponse,
}

/// Retains only bounded parser state and allowlisted metrics. Bytes are never rewritten.
pub struct RequestObservation {
    record: RequestRecord,
    started: Instant,
    buffer: Vec<u8>,
    line: Vec<u8>,
    event_data: Vec<u8>,
    discard_event: bool,
    oversized_json: bool,
    terminal: bool,
    valid_response: bool,
    anthropic_input_tokens: Option<u64>,
    malformed: bool,
    upstream_error: bool,
    failure: Option<RequestFailure>,
    persist: bool,
}

impl RequestObservation {
    pub fn new(agent: &str, protocol: &str, request: &str) -> Self {
        let request = serde_json::from_str::<Value>(request).ok();
        Self {
            record: RequestRecord {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp_ms: now_ms(),
                source: "proxy".to_string(),
                agent: agent.to_string(),
                provider: None,
                model: request.as_ref().and_then(|v| identifier(v.get("model"))),
                protocol: Some(protocol.to_string()),
                upstream_protocol: None,
                status: "failed".to_string(),
                http_status: None,
                duration_ms: None,
                first_byte_ms: None,
                input_tokens: None,
                output_tokens: None,
                cached_tokens: None,
                cache_creation_tokens: None,
                reasoning_tokens: None,
                total_tokens: None,
                streaming: request
                    .as_ref()
                    .and_then(|v| v.get("stream"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            },
            started: Instant::now(),
            buffer: Vec::new(),
            line: Vec::new(),
            event_data: Vec::new(),
            discard_event: false,
            oversized_json: false,
            terminal: false,
            valid_response: false,
            anthropic_input_tokens: None,
            malformed: false,
            upstream_error: false,
            failure: None,
            persist: false,
        }
    }

    pub(crate) fn capture(agent: &str, protocol: &str, request: &str) -> Self {
        let mut observation = Self::new(agent, protocol, request);
        observation.persist = true;
        observation
    }

    pub fn upstream(
        &mut self,
        status: u16,
        streaming: bool,
        protocol: &str,
        provider: Option<&str>,
        model: Option<&str>,
    ) {
        self.record.http_status = Some(status);
        self.record.streaming = streaming;
        self.record.upstream_protocol = Some(protocol.to_string());
        self.record.provider = provider.and_then(identifier_str);
        if let Some(model) = model.and_then(identifier_str) {
            self.record.model = Some(model);
        }
        self.record.status = if (200..300).contains(&status) {
            "interrupted"
        } else {
            "failed"
        }
        .to_string();
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) {
        if !bytes.is_empty() && self.record.first_byte_ms.is_none() {
            self.record.first_byte_ms = Some(self.elapsed_ms());
        }
        if !self
            .record
            .http_status
            .is_some_and(|s| (200..300).contains(&s))
        {
            return;
        }
        if !self.record.streaming {
            if !self.oversized_json && self.buffer.len() + bytes.len() <= MAX_PARSE_BYTES {
                self.buffer.extend_from_slice(bytes);
            } else {
                self.buffer.clear();
                self.oversized_json = true;
            }
            return;
        }
        // Split on bytes, not lossy UTF-8 chunks: a code point may cross reads.
        for byte in bytes {
            if *byte == b'\n' {
                let mut line = std::mem::take(&mut self.line);
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                if line.is_empty() {
                    if !self.discard_event && !self.event_data.is_empty() {
                        let data = std::mem::take(&mut self.event_data);
                        self.parse_payload(&data);
                    }
                    self.event_data.clear();
                    self.discard_event = false;
                } else if !self.discard_event {
                    if let Some(data) = line.strip_prefix(b"data:") {
                        let data = data.strip_prefix(b" ").unwrap_or(data);
                        if self.event_data.len() + data.len() + 1 <= MAX_PARSE_BYTES {
                            if !self.event_data.is_empty() {
                                self.event_data.push(b'\n');
                            }
                            self.event_data.extend_from_slice(data);
                        } else {
                            self.discard_event = true;
                            self.malformed = true;
                            self.event_data.clear();
                        }
                    }
                }
            } else if self.line.len() < MAX_PARSE_BYTES {
                self.line.push(*byte);
            } else {
                self.discard_event = true;
                self.malformed = true;
            }
        }
    }

    fn parse_payload(&mut self, data: &[u8]) {
        if data == b"[DONE]" {
            self.terminal = true;
            return;
        }
        let Ok(value) = serde_json::from_slice::<Value>(data) else {
            self.malformed = true;
            return;
        };
        let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
        let response = value
            .get("response")
            .or_else(|| value.get("message"))
            .unwrap_or(&value);
        if value.get("error").is_some_and(|v| !v.is_null())
            || matches!(kind, "error" | "response.failed")
            || response.get("status").and_then(Value::as_str) == Some("failed")
        {
            self.upstream_error = true;
            return;
        }
        if kind == "response.incomplete"
            || response.get("status").and_then(Value::as_str) == Some("incomplete")
        {
            self.failure = Some(RequestFailure::Stream);
        }
        self.terminal |= matches!(kind, "message_stop" | "response.completed");
        self.valid_response |= response.get("choices").is_some_and(Value::is_array)
            || response.get("object").and_then(Value::as_str) == Some("response")
            || response.get("type").and_then(Value::as_str) == Some("message")
            || matches!(
                kind,
                "message_start" | "message_delta" | "message_stop" | "response.completed"
            );
        if let Some(model) = identifier(response.get("model")) {
            self.record.model = Some(model);
        }
        if let Some(usage) = response.get("usage").or_else(|| value.get("usage")) {
            self.merge_usage(usage);
        }
    }

    fn merge_usage(&mut self, usage: &Value) {
        // Anthropic message_delta counters are cumulative, not additive. Preserve
        // input/cache counts from message_start when later updates omit them.
        for (target, paths) in [
            (
                &mut self.record.input_tokens,
                &["/input_tokens", "/prompt_tokens"][..],
            ),
            (
                &mut self.record.output_tokens,
                &["/output_tokens", "/completion_tokens"][..],
            ),
            (
                &mut self.record.cached_tokens,
                &[
                    "/cache_read_input_tokens",
                    "/input_tokens_details/cached_tokens",
                    "/prompt_tokens_details/cached_tokens",
                ][..],
            ),
            (
                &mut self.record.cache_creation_tokens,
                &["/cache_creation_input_tokens"][..],
            ),
            (
                &mut self.record.reasoning_tokens,
                &[
                    "/output_tokens_details/reasoning_tokens",
                    "/completion_tokens_details/reasoning_tokens",
                ][..],
            ),
            (&mut self.record.total_tokens, &["/total_tokens"][..]),
        ] {
            if let Some(value) = paths.iter().find_map(|path| usage.pointer(path)) {
                *target = value.as_u64();
            }
        }
        if self.record.upstream_protocol.as_deref() == Some("messages") {
            if let Some(value) = usage.get("input_tokens") {
                self.anthropic_input_tokens = value.as_u64();
            }
            // Normalize Anthropic's uncached input into the cache-inclusive
            // denominator used by Codex. Omitted counters remain null.
            self.record.input_tokens = self.anthropic_input_tokens.and_then(|input| {
                [self.record.cached_tokens, self.record.cache_creation_tokens]
                    .into_iter()
                    .flatten()
                    .try_fold(input, u64::checked_add)
            });
        }
    }

    pub fn fail(&mut self, failure: RequestFailure) {
        self.failure = Some(failure);
        self.record.status = if self
            .record
            .http_status
            .is_some_and(|s| (200..300).contains(&s))
            && matches!(
                failure,
                RequestFailure::Stream | RequestFailure::ClientDisconnect
            ) {
            "interrupted"
        } else {
            "failed"
        }
        .to_string();
    }

    pub fn finish(&mut self) -> RequestRecord {
        if !self.record.streaming && !self.buffer.is_empty() {
            let bytes = std::mem::take(&mut self.buffer);
            self.parse_payload(&bytes);
        }
        if self.failure.is_none()
            && self
                .record
                .http_status
                .is_some_and(|s| (200..300).contains(&s))
        {
            self.record.status = if self.upstream_error {
                "failed"
            } else if self.record.streaming
                && (!self.terminal
                    || !self.line.is_empty()
                    || !self.event_data.is_empty()
                    || self.discard_event)
            {
                "interrupted"
            } else if self.valid_response
                && !self.malformed
                && self.record.input_tokens.is_some()
                && self.record.output_tokens.is_some()
            {
                "success"
            } else {
                "observed"
            }
            .to_string();
        } else if let Some(failure) = self.failure {
            self.fail(failure);
        }
        self.record.duration_ms = Some(self.elapsed_ms());
        self.record.clone()
    }

    fn elapsed_ms(&self) -> u64 {
        self.started.elapsed().as_millis().min(u64::MAX as u128) as u64
    }
}

impl Drop for RequestObservation {
    fn drop(&mut self) {
        if self.persist {
            // Early exits retain failed/interrupted, even after a terminal usage
            // event, until downstream writes and shutdown have completed.
            self.record.duration_ms = Some(self.elapsed_ms());
            let path = request_log_path();
            let record = self.record.clone();
            let persist = move || {
                if append_request_at(&path, &record).is_err() {
                    let _ = crate::diagnostic_log::append_diagnostic_log(
                        "request_telemetry.persist_failed",
                        serde_json::json!({"reason": "storage_unavailable"}),
                    );
                }
            };
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn_blocking(persist);
            } else {
                persist();
            }
        }
    }
}

fn identifier(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).and_then(identifier_str)
}

fn identifier_str(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    (!value.is_empty()
        && value.len() <= 200
        && !lower.contains("://")
        && !lower.starts_with("sk-")
        && !lower.contains("bearer")
        && !lower.contains("api_key")
        && !lower.contains("apikey")
        && !lower.contains("token")
        && !lower.contains("secret")
        && !lower.starts_with("key-")
        && !lower.starts_with("key_")
        && !lower.starts_with("sess-")
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || "-_.:/".contains(c)))
    .then(|| value.to_string())
}

fn sanitize_record(mut record: RequestRecord) -> Option<RequestRecord> {
    uuid::Uuid::parse_str(&record.id).ok()?;
    if record.source != "proxy"
        || !matches!(record.agent.as_str(), "codex" | "claude" | "claude-desktop")
        || !matches!(
            record.status.as_str(),
            "success" | "failed" | "interrupted" | "observed"
        )
    {
        return None;
    }
    record.provider = record.provider.as_deref().and_then(identifier_str);
    record.model = record.model.as_deref().and_then(identifier_str);
    for protocol in [&mut record.protocol, &mut record.upstream_protocol] {
        if !matches!(
            protocol.as_deref(),
            Some("responses" | "chat_completions" | "messages")
        ) {
            *protocol = None;
        }
    }
    record.http_status = record
        .http_status
        .filter(|status| (100..600).contains(status));
    if record.status == "success" {
        if record.http_status.is_some_and(|status| status >= 400) {
            record.status = "failed".to_string();
        } else if record.input_tokens.is_none() || record.output_tokens.is_none() {
            record.status = "observed".to_string();
        }
    }
    Some(record)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
