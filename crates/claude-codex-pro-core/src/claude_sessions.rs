use anyhow::{Context, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const PROJECT_SOURCE_KIND: &str = "claude-projects";
const SESSIONS_SOURCE_KIND: &str = "claude-sessions";
const CLAUDE_CODE_SESSIONS_SOURCE_KIND: &str = "claude-code-sessions";
const LOCAL_AGENT_SESSIONS_SOURCE_KIND: &str = "local-agent-mode-sessions";
const AUDIT_SOURCE_KIND: &str = "claude-audit";
const LOCAL_SOURCE_KIND: &str = "claude-local";
const TITLE_MAX_CHARS: usize = 120;
const DEFAULT_CONTEXT_PAGE_SIZE: usize = 80;
const MAX_CONTEXT_PAGE_SIZE: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSession {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub model_provider: String,
    pub archived: bool,
    pub updated_at_ms: Option<i64>,
    pub source_path: String,
    pub source_kind: String,
    pub message_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSessionsInventory {
    pub source_root: String,
    pub source_paths: Vec<String>,
    pub sessions: Vec<ClaudeSession>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSessionContextMessage {
    pub sequence: usize,
    pub role: String,
    pub text: String,
    pub timestamp_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSessionContextPage {
    pub session_id: String,
    pub title: String,
    pub cwd: String,
    pub source_path: String,
    pub source_kind: String,
    pub total_messages: usize,
    pub offset: usize,
    pub messages: Vec<ClaudeSessionContextMessage>,
    pub has_more_before: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSessionDeleteResult {
    pub session_id: String,
    pub backup_path: String,
    pub message: String,
}

#[derive(Debug, Clone)]
struct SessionSource {
    path: PathBuf,
    trusted_root: PathBuf,
    source_kind: &'static str,
    fallback_project: String,
}

#[derive(Debug)]
struct SourceDiscovery {
    source_root: PathBuf,
    sources: Vec<SessionSource>,
    warnings: Vec<String>,
}

#[derive(Debug, Default)]
struct SourceParse {
    sessions: Vec<ClaudeSession>,
    warnings: Vec<String>,
    unsafe_to_delete: bool,
}

#[derive(Debug, Default)]
struct SessionAccumulator {
    custom_title: Option<String>,
    ai_title: Option<String>,
    first_user_input: Option<String>,
    cwd: Option<String>,
    model_provider: Option<String>,
    archived: Option<bool>,
    updated_at_ms: Option<i64>,
    message_count: usize,
    meaningful: bool,
}

impl SessionAccumulator {
    fn merge(&mut self, other: SessionAccumulator) {
        if other.custom_title.is_some() {
            self.custom_title = other.custom_title;
        }
        if other.ai_title.is_some() {
            self.ai_title = other.ai_title;
        }
        if self.first_user_input.is_none() {
            self.first_user_input = other.first_user_input;
        }
        if other.cwd.is_some() {
            self.cwd = other.cwd;
        }
        if other.model_provider.is_some() {
            self.model_provider = other.model_provider;
        }
        if other.archived.is_some() {
            self.archived = other.archived;
        }
        self.updated_at_ms = later_timestamp(self.updated_at_ms, other.updated_at_ms);
        self.message_count = self.message_count.saturating_add(other.message_count);
        self.meaningful |= other.meaningful;
    }

    fn merge_as_history(&mut self, other: SessionAccumulator) {
        if self.custom_title.is_none() {
            self.custom_title = other.custom_title;
        }
        if self.ai_title.is_none() {
            self.ai_title = other.ai_title;
        }
        if self.first_user_input.is_none() {
            self.first_user_input = other.first_user_input;
        }
        if self.cwd.is_none() {
            self.cwd = other.cwd;
        }
        if self.model_provider.is_none() {
            self.model_provider = other.model_provider;
        }
        if self.archived.is_none() {
            self.archived = other.archived;
        }
        self.updated_at_ms = later_timestamp(self.updated_at_ms, other.updated_at_ms);
        self.message_count = self.message_count.saturating_add(other.message_count);
        self.meaningful |= other.meaningful;
    }
}

#[derive(Debug, Default)]
struct SessionBuilder {
    keyed: BTreeMap<String, SessionAccumulator>,
    unkeyed: SessionAccumulator,
    unkeyed_containers: usize,
}

impl SessionBuilder {
    fn accumulator(&mut self, session_id: Option<String>) -> &mut SessionAccumulator {
        match session_id.filter(|id| !id.trim().is_empty()) {
            Some(id) => self.keyed.entry(id).or_default(),
            None => &mut self.unkeyed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFingerprint {
    len: u64,
    modified: Option<SystemTime>,
    digest: [u8; 32],
}

#[derive(Debug)]
struct ContextCollector {
    requested_offset: Option<usize>,
    limit: usize,
    total_messages: usize,
    messages: VecDeque<ClaudeSessionContextMessage>,
}

impl ContextCollector {
    fn new(offset: Option<usize>, limit: Option<usize>) -> Self {
        Self {
            requested_offset: offset,
            limit: limit
                .unwrap_or(DEFAULT_CONTEXT_PAGE_SIZE)
                .clamp(1, MAX_CONTEXT_PAGE_SIZE),
            total_messages: 0,
            messages: VecDeque::new(),
        }
    }

    fn push(&mut self, role: &str, text: String, timestamp_ms: Option<i64>) {
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }
        let index = self.total_messages;
        self.total_messages = self.total_messages.saturating_add(1);
        let message = ClaudeSessionContextMessage {
            sequence: index.saturating_add(1),
            role: role.to_string(),
            text,
            timestamp_ms,
        };
        if let Some(offset) = self.requested_offset {
            if index >= offset && index < offset.saturating_add(self.limit) {
                self.messages.push_back(message);
            }
        } else {
            self.messages.push_back(message);
            while self.messages.len() > self.limit {
                self.messages.pop_front();
            }
        }
    }

    fn finish(self) -> (usize, usize, Vec<ClaudeSessionContextMessage>, bool) {
        let message_count = self.messages.len();
        let offset = self
            .requested_offset
            .map(|offset| offset.min(self.total_messages))
            .unwrap_or_else(|| self.total_messages.saturating_sub(message_count));
        (
            self.total_messages,
            offset,
            self.messages.into_iter().collect(),
            offset > 0,
        )
    }
}

pub fn list_claude_sessions() -> anyhow::Result<ClaudeSessionsInventory> {
    let home = default_user_home()?;
    list_claude_sessions_from_home(&home)
}

pub fn list_claude_sessions_from_home(home: &Path) -> anyhow::Result<ClaudeSessionsInventory> {
    let discovery = discover_sources(home);
    let mut warnings = discovery.warnings;
    let mut source_paths = Vec::with_capacity(discovery.sources.len());
    let mut sessions_by_source_and_id = BTreeMap::new();

    for source in &discovery.sources {
        let source_path = path_string(&source.path);
        source_paths.push(source_path.clone());
        let parsed = parse_source(source);
        warnings.extend(parsed.warnings);
        for session in parsed.sessions {
            sessions_by_source_and_id
                .entry((source_path.clone(), session.id.clone()))
                .or_insert(session);
        }
    }

    source_paths.sort();
    source_paths.dedup();
    warnings.sort();
    warnings.dedup();
    let mut sessions = sessions_by_source_and_id.into_values().collect::<Vec<_>>();
    sessions.sort_by(|left, right| {
        right
            .updated_at_ms
            .cmp(&left.updated_at_ms)
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| left.id.cmp(&right.id))
            .then_with(|| left.source_path.cmp(&right.source_path))
    });

    Ok(ClaudeSessionsInventory {
        source_root: path_string(&discovery.source_root),
        source_paths,
        sessions,
        warnings,
    })
}

pub fn load_claude_session_context(
    session_id: &str,
    source_path: &Path,
    offset: Option<usize>,
    limit: Option<usize>,
) -> anyhow::Result<ClaudeSessionContextPage> {
    let home = default_user_home()?;
    load_claude_session_context_from_home(&home, session_id, source_path, offset, limit)
}

pub fn load_claude_session_context_from_home(
    home: &Path,
    session_id: &str,
    source_path: &Path,
    offset: Option<usize>,
    limit: Option<usize>,
) -> anyhow::Result<ClaudeSessionContextPage> {
    if session_id.trim().is_empty() {
        bail!("Claude session id must not be empty");
    }

    let source = rediscover_trusted_source(home, source_path)?;
    let parsed = parse_source(&source);
    let session = parsed
        .sessions
        .into_iter()
        .find(|session| session.id == session_id)
        .ok_or_else(|| anyhow!("Claude session was not found in the requested source"))?;
    let mut collector = ContextCollector::new(offset, limit);
    read_context_source(&source, session_id, &mut collector)?;
    let (total_messages, offset, messages, has_more_before) = collector.finish();

    Ok(ClaudeSessionContextPage {
        session_id: session.id,
        title: session.title,
        cwd: session.cwd,
        source_path: session.source_path,
        source_kind: session.source_kind,
        total_messages,
        offset,
        messages,
        has_more_before,
    })
}

pub fn delete_claude_session(
    backup_root: &Path,
    session_id: &str,
    source_path: &Path,
) -> anyhow::Result<ClaudeSessionDeleteResult> {
    let home = default_user_home()?;
    delete_claude_session_from_home(&home, backup_root, session_id, source_path)
}

pub fn delete_claude_session_from_home(
    home: &Path,
    backup_root: &Path,
    session_id: &str,
    source_path: &Path,
) -> anyhow::Result<ClaudeSessionDeleteResult> {
    if session_id.trim().is_empty() {
        bail!("Claude session id must not be empty");
    }

    let source = rediscover_trusted_source(home, source_path)?;
    let requested_path = source.path.clone();

    let before_validation = fingerprint_file(&source.path)?;
    let parsed = parse_source(&source);
    let after_validation = fingerprint_file(&source.path)?;
    if before_validation != after_validation {
        bail!("Claude session source changed while it was being validated");
    }
    if parsed.unsafe_to_delete {
        bail!("Claude session source could not be parsed safely for deletion");
    }
    if parsed.sessions.len() > 1 {
        bail!("Claude session source contains multiple sessions and cannot be deleted safely");
    }
    let discovered_session = parsed
        .sessions
        .first()
        .ok_or_else(|| anyhow!("Claude session was not found in the requested source"))?;
    if discovered_session.id != session_id {
        bail!("Claude session id does not match the rediscovered source");
    }

    let backup_dir = backup_root.join("claude-sessions");
    fs::create_dir_all(&backup_dir)
        .with_context(|| "failed to create the Claude session backup directory")?;
    let backup_path = available_backup_path(
        &backup_dir,
        session_id,
        &source.path,
        &after_validation.digest,
    )?;
    let copied = match fs::copy(&source.path, &backup_path) {
        Ok(copied) => copied,
        Err(error) => {
            let _ = fs::remove_file(&backup_path);
            return Err(error).context("failed to back up the Claude session source");
        }
    };

    let source_after_backup = fingerprint_file(&source.path);
    let backup_fingerprint = fingerprint_file(&backup_path);
    let source_is_stable = source_after_backup
        .as_ref()
        .is_ok_and(|fingerprint| fingerprint == &after_validation);
    let backup_is_complete = backup_fingerprint.as_ref().is_ok_and(|fingerprint| {
        fingerprint.len == after_validation.len
            && fingerprint.digest == after_validation.digest
            && copied == after_validation.len
    });
    let path_is_stable = fs::canonicalize(&source.path)
        .map(|path| path == requested_path)
        .unwrap_or(false);
    if !source_is_stable || !backup_is_complete || !path_is_stable {
        let _ = fs::remove_file(&backup_path);
        bail!("Claude session source changed or its backup could not be verified");
    }

    fs::remove_file(&source.path)
        .with_context(|| "backup succeeded but deleting the Claude session source failed")?;

    Ok(ClaudeSessionDeleteResult {
        session_id: session_id.to_string(),
        backup_path: path_string(&backup_path),
        message: "Claude session deleted after a verified backup.".to_string(),
    })
}

fn rediscover_trusted_source(home: &Path, source_path: &Path) -> anyhow::Result<SessionSource> {
    let requested_path = fs::canonicalize(source_path)
        .with_context(|| "failed to canonicalize the requested Claude session source")?;
    let discovery = discover_sources(home);
    let source = discovery
        .sources
        .into_iter()
        .find(|candidate| candidate.path == requested_path)
        .ok_or_else(|| anyhow!("Claude session source is not a trusted rediscovered path"))?;
    if !requested_path.starts_with(&source.trusted_root) {
        bail!("Claude session source is outside its trusted root");
    }
    Ok(source)
}

fn default_user_home() -> anyhow::Result<PathBuf> {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .ok_or_else(|| anyhow!("could not determine the current user home directory"))
}

fn discover_sources(home: &Path) -> SourceDiscovery {
    let claude_root = home.join(".claude");
    let source_root = fs::canonicalize(&claude_root).unwrap_or(claude_root.clone());
    let mut sources = BTreeMap::<PathBuf, SessionSource>::new();
    let mut warnings = Vec::new();

    discover_project_sources(&claude_root.join("projects"), &mut sources, &mut warnings);
    for (relative_root, source_kind) in [
        (Path::new(".claude").join("sessions"), SESSIONS_SOURCE_KIND),
        (
            Path::new(".claude").join("claude-code-sessions"),
            CLAUDE_CODE_SESSIONS_SOURCE_KIND,
        ),
        (
            Path::new(".claude").join("local-agent-mode-sessions"),
            LOCAL_AGENT_SESSIONS_SOURCE_KIND,
        ),
        (
            Path::new(".config").join("claude-code-sessions"),
            CLAUDE_CODE_SESSIONS_SOURCE_KIND,
        ),
        (
            Path::new(".config").join("local-agent-mode-sessions"),
            LOCAL_AGENT_SESSIONS_SOURCE_KIND,
        ),
    ] {
        discover_supplemental_sources(
            &home.join(relative_root),
            source_kind,
            &mut sources,
            &mut warnings,
        );
    }
    discover_direct_claude_sources(&claude_root, &mut sources, &mut warnings);

    SourceDiscovery {
        source_root,
        sources: sources.into_values().collect(),
        warnings,
    }
}

fn discover_direct_claude_sources(
    claude_root: &Path,
    sources: &mut BTreeMap<PathBuf, SessionSource>,
    warnings: &mut Vec<String>,
) {
    if !claude_root.exists() {
        return;
    }
    let trusted_root = match fs::canonicalize(claude_root) {
        Ok(path) => path,
        Err(error) => {
            warnings.push(format!(
                "Failed to canonicalize Claude root {}: {error}",
                claude_root.display()
            ));
            return;
        }
    };
    let entries = match fs::read_dir(claude_root) {
        Ok(entries) => entries,
        Err(error) => {
            warnings.push(format!(
                "Failed to scan Claude root {}: {error}",
                claude_root.display()
            ));
            return;
        }
    };
    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if file_type.is_symlink() || !file_type.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let source_kind = if file_name == "audit.jsonl" {
            Some(AUDIT_SOURCE_KIND)
        } else if file_name == "local.json"
            || (file_name.starts_with("local_")
                && (file_name.ends_with(".json") || file_name.ends_with(".jsonl")))
        {
            Some(LOCAL_SOURCE_KIND)
        } else {
            None
        };
        let Some(source_kind) = source_kind else {
            continue;
        };
        insert_source(
            entry.path(),
            &trusted_root,
            source_kind,
            source_kind.to_string(),
            sources,
            warnings,
        );
    }
}

fn discover_project_sources(
    projects_root: &Path,
    sources: &mut BTreeMap<PathBuf, SessionSource>,
    warnings: &mut Vec<String>,
) {
    if !projects_root.exists() {
        return;
    }
    let trusted_root = match fs::canonicalize(projects_root) {
        Ok(path) => path,
        Err(error) => {
            warnings.push(format!(
                "Failed to canonicalize Claude projects root {}: {error}",
                projects_root.display()
            ));
            return;
        }
    };
    let project_entries = match fs::read_dir(projects_root) {
        Ok(entries) => entries,
        Err(error) => {
            warnings.push(format!(
                "Failed to scan Claude projects root {}: {error}",
                projects_root.display()
            ));
            return;
        }
    };

    for project_entry in project_entries {
        let project_entry = match project_entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(format!("Failed to read a Claude project entry: {error}"));
                continue;
            }
        };
        let file_type = match project_entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                warnings.push(format!(
                    "Failed to inspect Claude project entry {}: {error}",
                    project_entry.path().display()
                ));
                continue;
            }
        };
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let project_path = project_entry.path();
        let project_name = project_entry.file_name().to_string_lossy().into_owned();
        let entries = match fs::read_dir(&project_path) {
            Ok(entries) => entries,
            Err(error) => {
                warnings.push(format!(
                    "Failed to scan Claude project {}: {error}",
                    project_path.display()
                ));
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    warnings.push(format!("Failed to read a Claude session entry: {error}"));
                    continue;
                }
            };
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    warnings.push(format!(
                        "Failed to inspect Claude session entry {}: {error}",
                        entry.path().display()
                    ));
                    continue;
                }
            };
            if file_type.is_symlink()
                || !file_type.is_file()
                || !has_extension(&entry.path(), "jsonl")
            {
                continue;
            }
            insert_source(
                entry.path(),
                &trusted_root,
                PROJECT_SOURCE_KIND,
                project_name.clone(),
                sources,
                warnings,
            );
        }
    }
}

fn discover_supplemental_sources(
    root: &Path,
    source_kind: &'static str,
    sources: &mut BTreeMap<PathBuf, SessionSource>,
    warnings: &mut Vec<String>,
) {
    if !root.exists() {
        return;
    }
    let trusted_root = match fs::canonicalize(root) {
        Ok(path) => path,
        Err(error) => {
            warnings.push(format!(
                "Failed to canonicalize Claude session root {}: {error}",
                root.display()
            ));
            return;
        }
    };
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                warnings.push(format!(
                    "Failed to scan Claude session directory {}: {error}",
                    directory.display()
                ));
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    warnings.push(format!("Failed to read a Claude session entry: {error}"));
                    continue;
                }
            };
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    warnings.push(format!(
                        "Failed to inspect Claude session entry {}: {error}",
                        entry.path().display()
                    ));
                    continue;
                }
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(entry.path());
                continue;
            }
            if !file_type.is_file()
                || (!has_extension(&entry.path(), "json") && !has_extension(&entry.path(), "jsonl"))
            {
                continue;
            }
            let fallback_project = entry
                .path()
                .parent()
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| source_kind.to_string());
            insert_source(
                entry.path(),
                &trusted_root,
                source_kind,
                fallback_project,
                sources,
                warnings,
            );
        }
    }
}

fn insert_source(
    path: PathBuf,
    trusted_root: &Path,
    source_kind: &'static str,
    fallback_project: String,
    sources: &mut BTreeMap<PathBuf, SessionSource>,
    warnings: &mut Vec<String>,
) {
    let canonical_path = match fs::canonicalize(&path) {
        Ok(path) => path,
        Err(error) => {
            warnings.push(format!(
                "Failed to canonicalize Claude session source {}: {error}",
                path.display()
            ));
            return;
        }
    };
    if !canonical_path.starts_with(trusted_root) {
        warnings.push(format!(
            "Skipped Claude session source outside its trusted root: {}",
            path.display()
        ));
        return;
    }
    sources
        .entry(canonical_path.clone())
        .or_insert(SessionSource {
            path: canonical_path,
            trusted_root: trusted_root.to_path_buf(),
            source_kind,
            fallback_project,
        });
}

fn read_context_source(
    source: &SessionSource,
    session_id: &str,
    collector: &mut ContextCollector,
) -> anyhow::Result<()> {
    if has_extension(&source.path, "jsonl") {
        let file = File::open(&source.path)
            .with_context(|| "failed to open the Claude session context source")?;
        let mut reader = BufReader::new(file);
        let mut line = Vec::new();
        loop {
            line.clear();
            let read = reader
                .read_until(b'\n', &mut line)
                .with_context(|| "failed while reading the Claude session context source")?;
            if read == 0 {
                break;
            }
            let trimmed = trim_ascii_whitespace(&line);
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_slice::<Value>(trimmed) {
                process_context_json_value(&value, None, source, session_id, collector);
            }
        }
    } else {
        let file = File::open(&source.path)
            .with_context(|| "failed to open the Claude session context source")?;
        let value = serde_json::from_reader::<_, Value>(BufReader::new(file))
            .with_context(|| "failed to parse the Claude session context source")?;
        process_context_json_value(&value, None, source, session_id, collector);
    }
    Ok(())
}

fn process_context_json_value(
    value: &Value,
    inherited_id: Option<&str>,
    source: &SessionSource,
    session_id: &str,
    collector: &mut ContextCollector,
) {
    match value {
        Value::Array(values) => {
            for value in values {
                process_context_json_value(value, inherited_id, source, session_id, collector);
            }
        }
        Value::Object(object) => {
            if let Some(collection) = object
                .get("sessions")
                .or_else(|| object.get("conversations"))
            {
                match collection {
                    Value::Array(sessions) => {
                        for session in sessions {
                            process_context_json_value(
                                session, None, source, session_id, collector,
                            );
                        }
                    }
                    Value::Object(sessions) => {
                        for (collection_id, session) in sessions {
                            process_context_json_value(
                                session,
                                Some(collection_id),
                                source,
                                session_id,
                                collector,
                            );
                        }
                    }
                    _ => {}
                }
                return;
            }
            if let Some(messages) = object.get("messages").and_then(Value::as_array) {
                let container_id = explicit_session_id(object)
                    .or_else(|| generic_container_id(object))
                    .or_else(|| inherited_id.map(str::to_string));
                for message in messages {
                    process_context_json_value(
                        message,
                        container_id.as_deref(),
                        source,
                        session_id,
                        collector,
                    );
                }
                return;
            }
            if let Some(data) = object.get("data") {
                if object.len() == 1 && (data.is_array() || data.is_object()) {
                    process_context_json_value(data, inherited_id, source, session_id, collector);
                    return;
                }
            }
            process_context_record(object, inherited_id, source, session_id, collector);
        }
        _ => {}
    }
}

fn process_context_record(
    object: &Map<String, Value>,
    inherited_id: Option<&str>,
    source: &SessionSource,
    session_id: &str,
    collector: &mut ContextCollector,
) {
    let record_session_id =
        explicit_session_id(object).or_else(|| inherited_id.map(str::to_string));
    let belongs_to_session = source.source_kind == PROJECT_SOURCE_KIND
        || record_session_id.as_deref() == Some(session_id)
        || (record_session_id.is_none() && file_stem(&source.path) == session_id);
    if !belongs_to_session {
        return;
    }

    let record_type = string_field(object, &["type", "recordType", "record_type"])
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        record_type.as_str(),
        "custom-title" | "custom_title" | "customtitle" | "ai-title" | "ai_title" | "aititle"
    ) {
        return;
    }
    let Some(role) = context_role(record_role(object).as_deref(), &record_type) else {
        return;
    };
    let timestamp_ms =
        timestamp_field(object).or_else(|| message_object(object).and_then(timestamp_field));
    let content = message_object(object)
        .and_then(|message| message.get("content"))
        .or_else(|| object.get("content"))
        .or_else(|| object.get("summary"))
        .or_else(|| object.get("text"));
    let Some(content) = content else {
        return;
    };
    let mut parts = Vec::new();
    extract_context_parts(content, role, &mut parts);
    for (part_role, text) in parts {
        collector.push(part_role, text, timestamp_ms);
    }
}

fn context_role(role: Option<&str>, record_type: &str) -> Option<&'static str> {
    let role = role.unwrap_or_default().to_ascii_lowercase();
    match role.as_str() {
        "user" => Some("user"),
        "assistant" => Some("assistant"),
        "tool" => Some("tool"),
        "system" => Some("system"),
        "developer" => Some("developer"),
        _ => match record_type {
            "user" => Some("user"),
            "assistant" => Some("assistant"),
            "tool" | "tool_result" | "tool-result" | "tool_use" | "tool-use" => Some("tool"),
            "system" | "summary" => Some("system"),
            "developer" => Some("developer"),
            _ => None,
        },
    }
}

fn extract_context_parts(
    value: &Value,
    fallback_role: &'static str,
    output: &mut Vec<(&'static str, String)>,
) {
    match value {
        Value::String(text) => output.push((fallback_role, text.clone())),
        Value::Array(values) => {
            for value in values {
                extract_context_parts(value, fallback_role, output);
            }
        }
        Value::Object(object) => {
            let block_type = string_field(object, &["type"])
                .unwrap_or_default()
                .to_ascii_lowercase();
            match block_type.as_str() {
                "thinking" | "redacted_thinking" | "image" | "document" => {}
                "tool_use" | "tool-use" => {
                    if let Some(name) = string_field(object, &["name", "toolName", "tool_name"]) {
                        output.push(("tool", format!("Tool call: {name}")));
                    }
                }
                "tool_result" | "tool-result" => {
                    if let Some(content) = object
                        .get("content")
                        .or_else(|| object.get("output"))
                        .or_else(|| object.get("text"))
                    {
                        extract_context_parts(content, "tool", output);
                    }
                }
                _ => {
                    if let Some(text) = object.get("text").and_then(Value::as_str) {
                        output.push((fallback_role, text.to_string()));
                    } else if let Some(content) = object.get("content") {
                        extract_context_parts(content, fallback_role, output);
                    } else if let Some(output_value) = object.get("output") {
                        extract_context_parts(output_value, fallback_role, output);
                    }
                }
            }
        }
        _ => {}
    }
}

fn parse_source(source: &SessionSource) -> SourceParse {
    let mut builder = SessionBuilder::default();
    let mut warnings = Vec::new();
    let mut unsafe_to_delete = false;
    if has_extension(&source.path, "jsonl") {
        parse_jsonl_source(
            &source.path,
            &mut builder,
            &mut warnings,
            &mut unsafe_to_delete,
        );
    } else {
        parse_json_source(
            &source.path,
            &mut builder,
            &mut warnings,
            &mut unsafe_to_delete,
        );
    }

    let modified_at_ms = file_modified_ms(&source.path);
    let mut keyed = builder.keyed;
    if source.source_kind == PROJECT_SOURCE_KIND && keyed.len() > 1 {
        let source_session_id = file_stem(&source.path);
        let mut session = keyed.remove(&source_session_id).unwrap_or_default();
        for history in keyed.into_values() {
            session.merge_as_history(history);
        }
        session.merge_as_history(builder.unkeyed);
        keyed = BTreeMap::new();
        if session.meaningful {
            keyed.insert(source_session_id, session);
        }
    } else if keyed.len() == 1 {
        if let Some(accumulator) = keyed.values_mut().next() {
            accumulator.merge(builder.unkeyed);
        }
    } else if keyed.is_empty() {
        if builder.unkeyed.meaningful && builder.unkeyed_containers <= 1 {
            keyed.insert(file_stem(&source.path), builder.unkeyed);
        } else if builder.unkeyed.meaningful {
            warnings.push(format!(
                "Claude session source {} contains ambiguous unkeyed sessions",
                source.path.display()
            ));
            unsafe_to_delete = true;
        }
    } else if builder.unkeyed.meaningful {
        warnings.push(format!(
            "Claude session source {} contains records without an assignable session id",
            source.path.display()
        ));
        unsafe_to_delete = true;
    }

    let mut sessions = keyed
        .into_iter()
        .filter(|(_, accumulator)| accumulator.meaningful)
        .map(|(id, accumulator)| accumulator_to_session(id, accumulator, source, modified_at_ms))
        .collect::<Vec<_>>();
    sessions.sort_by(|left, right| left.id.cmp(&right.id));
    if sessions.is_empty() && !unsafe_to_delete {
        warnings.push(format!(
            "No recognizable Claude session records found in {}",
            source.path.display()
        ));
        unsafe_to_delete = true;
    }

    SourceParse {
        sessions,
        warnings,
        unsafe_to_delete,
    }
}

fn parse_jsonl_source(
    path: &Path,
    builder: &mut SessionBuilder,
    warnings: &mut Vec<String>,
    unsafe_to_delete: &mut bool,
) {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            warnings.push(format!(
                "Failed to open Claude JSONL source {}: {error}",
                path.display()
            ));
            *unsafe_to_delete = true;
            return;
        }
    };
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    let mut malformed_lines = 0usize;
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = trim_ascii_whitespace(&line);
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_slice::<Value>(trimmed) {
                    Ok(value) => process_json_value(&value, None, builder),
                    Err(_) => malformed_lines = malformed_lines.saturating_add(1),
                }
            }
            Err(error) => {
                warnings.push(format!(
                    "Failed while streaming Claude JSONL source {}: {error}",
                    path.display()
                ));
                *unsafe_to_delete = true;
                break;
            }
        }
    }
    if malformed_lines > 0 {
        warnings.push(format!(
            "{} malformed JSONL line(s) skipped in {}",
            malformed_lines,
            path.display()
        ));
        *unsafe_to_delete = true;
    }
}

fn parse_json_source(
    path: &Path,
    builder: &mut SessionBuilder,
    warnings: &mut Vec<String>,
    unsafe_to_delete: &mut bool,
) {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            warnings.push(format!(
                "Failed to open Claude JSON source {}: {error}",
                path.display()
            ));
            *unsafe_to_delete = true;
            return;
        }
    };
    match serde_json::from_reader::<_, Value>(BufReader::new(file)) {
        Ok(value) => process_json_value(&value, None, builder),
        Err(error) => {
            warnings.push(format!(
                "Failed to parse Claude JSON source {}: {error}",
                path.display()
            ));
            *unsafe_to_delete = true;
        }
    }
}

fn process_json_value(value: &Value, inherited_id: Option<&str>, builder: &mut SessionBuilder) {
    match value {
        Value::Array(values) => {
            for value in values {
                process_json_value(value, inherited_id, builder);
            }
        }
        Value::Object(object) => {
            if let Some(collection) = object
                .get("sessions")
                .or_else(|| object.get("conversations"))
            {
                process_session_collection(collection, builder);
                return;
            }
            if object.get("messages").is_some_and(Value::is_array) {
                process_session_container(object, inherited_id, builder);
                return;
            }
            if let Some(data) = object.get("data") {
                if object.len() == 1 && (data.is_array() || data.is_object()) {
                    process_json_value(data, inherited_id, builder);
                    return;
                }
            }
            process_record(object, inherited_id, builder);
        }
        _ => {}
    }
}

fn process_session_collection(collection: &Value, builder: &mut SessionBuilder) {
    match collection {
        Value::Array(sessions) => {
            for session in sessions {
                process_json_value(session, None, builder);
            }
        }
        Value::Object(sessions) => {
            for (session_id, session) in sessions {
                process_json_value(session, Some(session_id), builder);
            }
        }
        _ => {}
    }
}

fn process_session_container(
    object: &Map<String, Value>,
    inherited_id: Option<&str>,
    builder: &mut SessionBuilder,
) {
    let session_id = explicit_session_id(object)
        .or_else(|| generic_container_id(object))
        .or_else(|| inherited_id.map(str::to_string));
    if session_id.is_none() {
        builder.unkeyed_containers = builder.unkeyed_containers.saturating_add(1);
    }
    {
        let accumulator = builder.accumulator(session_id.clone());
        accumulator.meaningful = true;
        apply_common_metadata(object, accumulator);
        if let Some(title) = normalized_field(object, &["customTitle", "custom_title", "title"]) {
            accumulator.custom_title = Some(title);
        }
        if let Some(title) = normalized_field(object, &["aiTitle", "ai_title"]) {
            accumulator.ai_title = Some(title);
        }
    }
    if let Some(messages) = object.get("messages").and_then(Value::as_array) {
        for message in messages {
            if let Value::Object(message) = message {
                process_record(message, session_id.as_deref(), builder);
            }
        }
    }
}

fn process_record(
    object: &Map<String, Value>,
    inherited_id: Option<&str>,
    builder: &mut SessionBuilder,
) {
    let session_id = explicit_session_id(object).or_else(|| inherited_id.map(str::to_string));
    let record_type = string_field(object, &["type", "recordType", "record_type"])
        .unwrap_or_default()
        .to_ascii_lowercase();
    let role = record_role(object).unwrap_or_default().to_ascii_lowercase();
    let is_user = record_type == "user" || role == "user";
    let is_assistant = record_type == "assistant" || role == "assistant";
    let is_custom_title = matches!(
        record_type.as_str(),
        "custom-title" | "custom_title" | "customtitle"
    ) || object.contains_key("customTitle")
        || object.contains_key("custom_title");
    let is_ai_title = matches!(record_type.as_str(), "ai-title" | "ai_title" | "aititle")
        || object.contains_key("aiTitle")
        || object.contains_key("ai_title");
    let meaningful = is_user || is_assistant || is_custom_title || is_ai_title;
    if session_id.is_none() && !meaningful {
        return;
    }

    let accumulator = builder.accumulator(session_id);
    apply_common_metadata(object, accumulator);
    if is_user {
        accumulator.meaningful = true;
        accumulator.message_count = accumulator.message_count.saturating_add(1);
        if accumulator.first_user_input.is_none() {
            accumulator.first_user_input = user_record_title(object);
        }
    }
    if is_assistant {
        accumulator.meaningful = true;
        accumulator.message_count = accumulator.message_count.saturating_add(1);
    }
    if is_custom_title {
        accumulator.meaningful = true;
        if let Some(title) = normalized_field_with_message(
            object,
            &["customTitle", "custom_title", "title", "content"],
        ) {
            accumulator.custom_title = Some(title);
        }
    }
    if is_ai_title {
        accumulator.meaningful = true;
        if let Some(title) =
            normalized_field_with_message(object, &["aiTitle", "ai_title", "title", "content"])
        {
            accumulator.ai_title = Some(title);
        }
    }
}

fn apply_common_metadata(object: &Map<String, Value>, accumulator: &mut SessionAccumulator) {
    if let Some(cwd) = normalized_field(
        object,
        &["cwd", "projectPath", "project_path", "workingDirectory"],
    ) {
        accumulator.cwd = Some(cwd);
    }
    if let Some(model) = normalized_field(object, &["modelProvider", "model_provider", "model"])
        .or_else(|| {
            message_object(object).and_then(|message| {
                normalized_field(message, &["modelProvider", "model_provider", "model"])
            })
        })
    {
        accumulator.model_provider = Some(model);
    }
    if let Some(archived) = bool_field(object, &["archived", "isArchived", "is_archived"]) {
        accumulator.archived = Some(archived);
    }
    if let Some(timestamp) = timestamp_field(object) {
        accumulator.updated_at_ms = later_timestamp(accumulator.updated_at_ms, Some(timestamp));
    }
}

fn accumulator_to_session(
    id: String,
    accumulator: SessionAccumulator,
    source: &SessionSource,
    modified_at_ms: Option<i64>,
) -> ClaudeSession {
    let title = accumulator
        .custom_title
        .or(accumulator.ai_title)
        .or(accumulator.first_user_input)
        .unwrap_or_else(|| file_stem(&source.path));
    ClaudeSession {
        id,
        title,
        cwd: accumulator
            .cwd
            .unwrap_or_else(|| source.fallback_project.clone()),
        model_provider: accumulator.model_provider.unwrap_or_default(),
        archived: accumulator.archived.unwrap_or(false),
        updated_at_ms: accumulator.updated_at_ms.or(modified_at_ms),
        source_path: path_string(&source.path),
        source_kind: source.source_kind.to_string(),
        message_count: accumulator.message_count,
    }
}

fn explicit_session_id(object: &Map<String, Value>) -> Option<String> {
    normalized_field(
        object,
        &[
            "sessionId",
            "session_id",
            "conversationId",
            "conversation_id",
        ],
    )
    .or_else(|| {
        object
            .get("session")
            .and_then(Value::as_object)
            .and_then(|session| normalized_field(session, &["id", "sessionId", "session_id"]))
    })
}

fn generic_container_id(object: &Map<String, Value>) -> Option<String> {
    normalized_field(object, &["id", "uuid"])
}

fn record_role(object: &Map<String, Value>) -> Option<String> {
    string_field(object, &["role"])
        .or_else(|| message_object(object).and_then(|message| string_field(message, &["role"])))
}

fn user_record_title(object: &Map<String, Value>) -> Option<String> {
    let content = message_object(object)
        .and_then(|message| message.get("content"))
        .or_else(|| object.get("content"))?;
    readable_text(content).and_then(|text| normalize_title(&text))
}

fn readable_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(values) => values.iter().find_map(readable_text),
        Value::Object(object) => {
            let block_type = string_field(object, &["type"]).unwrap_or_default();
            if block_type.is_empty()
                || matches!(block_type.as_str(), "text" | "input_text" | "output_text")
            {
                object
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| object.get("content").and_then(readable_text))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn normalized_field(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    string_field(object, keys).and_then(|value| normalize_title(&value))
}

fn normalized_field_with_message(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    normalized_field(object, keys)
        .or_else(|| message_object(object).and_then(|message| normalized_field(message, keys)))
}

fn string_field(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        object
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn bool_field(object: &Map<String, Value>, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_bool))
}

fn message_object(object: &Map<String, Value>) -> Option<&Map<String, Value>> {
    object.get("message").and_then(Value::as_object)
}

fn normalize_title(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return None;
    }
    let mut chars = normalized.chars();
    let title = chars.by_ref().take(TITLE_MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        Some(format!("{title}..."))
    } else {
        Some(title)
    }
}

fn timestamp_field(object: &Map<String, Value>) -> Option<i64> {
    [
        "timestamp",
        "updatedAt",
        "updated_at",
        "createdAt",
        "created_at",
    ]
    .iter()
    .find_map(|key| object.get(*key).and_then(parse_timestamp_ms))
}

fn parse_timestamp_ms(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .and_then(normalize_integer_timestamp)
            .or_else(|| number.as_f64().and_then(normalize_float_timestamp)),
        Value::String(value) => value
            .parse::<i64>()
            .ok()
            .and_then(normalize_integer_timestamp)
            .or_else(|| {
                value
                    .parse::<f64>()
                    .ok()
                    .and_then(normalize_float_timestamp)
            })
            .or_else(|| parse_rfc3339_ms(value)),
        _ => None,
    }
}

fn normalize_integer_timestamp(value: i64) -> Option<i64> {
    let magnitude = value.unsigned_abs();
    if magnitude >= 100_000_000_000_000_000 {
        Some(value / 1_000_000)
    } else if magnitude >= 100_000_000_000_000 {
        Some(value / 1_000)
    } else if magnitude >= 100_000_000_000 {
        Some(value)
    } else {
        value.checked_mul(1_000)
    }
}

fn normalize_float_timestamp(value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    let millis = if value.abs() >= 100_000_000_000.0 {
        value
    } else {
        value * 1_000.0
    };
    if millis < i64::MIN as f64 || millis > i64::MAX as f64 {
        None
    } else {
        Some(millis.round() as i64)
    }
}

fn parse_rfc3339_ms(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || !matches!(bytes.get(10), Some(b'T' | b't' | b' '))
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return None;
    }
    let year = parse_digits(bytes, 0, 4)? as i64;
    let month = parse_digits(bytes, 5, 2)? as u32;
    let day = parse_digits(bytes, 8, 2)? as u32;
    let hour = parse_digits(bytes, 11, 2)? as u32;
    let minute = parse_digits(bytes, 14, 2)? as u32;
    let second = parse_digits(bytes, 17, 2)? as u32;
    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }

    let mut index = 19usize;
    let mut fraction_ms = 0i64;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fraction_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == fraction_start {
            return None;
        }
        let digits = &bytes[fraction_start..index];
        for offset in 0..3 {
            fraction_ms *= 10;
            if let Some(digit) = digits.get(offset) {
                fraction_ms += i64::from(*digit - b'0');
            }
        }
    }

    let offset_seconds = match bytes.get(index).copied()? {
        b'Z' | b'z' if index + 1 == bytes.len() => 0i64,
        sign @ (b'+' | b'-') => {
            let remaining = &bytes[index + 1..];
            let (offset_hour, offset_minute) = match remaining.len() {
                5 if remaining.get(2) == Some(&b':') => (
                    parse_digits(remaining, 0, 2)?,
                    parse_digits(remaining, 3, 2)?,
                ),
                4 => (
                    parse_digits(remaining, 0, 2)?,
                    parse_digits(remaining, 2, 2)?,
                ),
                _ => return None,
            };
            if offset_hour > 23 || offset_minute > 59 {
                return None;
            }
            let offset = i64::from(offset_hour * 3_600 + offset_minute * 60);
            if sign == b'-' { -offset } else { offset }
        }
        _ => return None,
    };

    let days = days_from_civil(year, month, day);
    let seconds = days
        .checked_mul(86_400)?
        .checked_add(i64::from(hour) * 3_600)?
        .checked_add(i64::from(minute) * 60)?
        .checked_add(i64::from(second))?
        .checked_sub(offset_seconds)?;
    seconds.checked_mul(1_000)?.checked_add(fraction_ms)
}

fn parse_digits(bytes: &[u8], start: usize, len: usize) -> Option<u32> {
    let digits = bytes.get(start..start.checked_add(len)?)?;
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    digits.iter().try_fold(0u32, |value, digit| {
        value.checked_mul(10)?.checked_add(u32::from(*digit - b'0'))
    })
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn later_timestamp(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn file_modified_ms(path: &Path) -> Option<i64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()
        .and_then(system_time_ms)
}

fn system_time_ms(value: SystemTime) -> Option<i64> {
    match value.duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).ok(),
        Err(error) => i64::try_from(error.duration().as_millis())
            .ok()
            .and_then(i64::checked_neg),
    }
}

fn fingerprint_file(path: &Path) -> anyhow::Result<SourceFingerprint> {
    let metadata =
        fs::metadata(path).with_context(|| "failed to read Claude session source metadata")?;
    if !metadata.is_file() {
        bail!("Claude session source is no longer a regular file");
    }
    let mut file = File::open(path).with_context(|| "failed to open Claude session source")?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| "failed to hash Claude session source")?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(SourceFingerprint {
        len: metadata.len(),
        modified: metadata.modified().ok(),
        digest: hasher.finalize().into(),
    })
}

fn available_backup_path(
    backup_dir: &Path,
    session_id: &str,
    source_path: &Path,
    digest: &[u8; 32],
) -> anyhow::Result<PathBuf> {
    let safe_session_id = safe_file_component(session_id, 72);
    let safe_source_stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| safe_file_component(stem, 72))
        .unwrap_or_else(|| "session".to_string());
    let digest_prefix = digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let extension = source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.is_empty())
        .unwrap_or("jsonl");
    let base = format!("{safe_session_id}--{safe_source_stem}--{digest_prefix}");
    for suffix in 0..10_000usize {
        let file_name = if suffix == 0 {
            format!("{base}.{extension}")
        } else {
            format!("{base}--{suffix}.{extension}")
        };
        let candidate = backup_dir.join(file_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!("could not allocate a unique Claude session backup path")
}

fn safe_file_component(value: &str, max_chars: usize) -> String {
    let mut result = value
        .chars()
        .take(max_chars)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    while result.starts_with('.') {
        result.remove(0);
    }
    if result.is_empty() {
        "session".to_string()
    } else {
        result
    }
}

fn trim_ascii_whitespace(mut value: &[u8]) -> &[u8] {
    while value.first().is_some_and(u8::is_ascii_whitespace) {
        value = &value[1..];
    }
    while value.last().is_some_and(u8::is_ascii_whitespace) {
        value = &value[..value.len() - 1];
    }
    value
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("untitled")
        .to_string()
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn write_jsonl(path: &Path, records: &[Value]) {
        let body = records
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        write_file(path, &format!("{body}\n"));
    }

    fn write_file(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture dir");
        fs::write(path, body).expect("write fixture");
    }

    fn project_session_path(home: &Path, project: &str, session: &str) -> PathBuf {
        home.join(".claude")
            .join("projects")
            .join(project)
            .join(format!("{session}.jsonl"))
    }

    fn session_by_id<'a>(inventory: &'a ClaudeSessionsInventory, id: &str) -> &'a ClaudeSession {
        inventory
            .sessions
            .iter()
            .find(|session| session.id == id)
            .expect("session should be listed")
    }

    #[test]
    fn models_serialize_with_camel_case_fields() {
        let session = ClaudeSession {
            id: "session-a".to_string(),
            title: "Title".to_string(),
            cwd: "C:/workspace".to_string(),
            model_provider: "claude-sonnet".to_string(),
            archived: false,
            updated_at_ms: Some(1_000),
            source_path: "C:/source.jsonl".to_string(),
            source_kind: "claude-projects".to_string(),
            message_count: 2,
        };
        let value = serde_json::to_value(&session).expect("serialize session");

        assert_eq!(value["modelProvider"], "claude-sonnet");
        assert_eq!(value["updatedAtMs"], 1_000);
        assert_eq!(value["sourcePath"], "C:/source.jsonl");
        assert_eq!(value["sourceKind"], "claude-projects");
        assert_eq!(value["messageCount"], 2);
        assert!(value.get("model_provider").is_none());

        let inventory = ClaudeSessionsInventory {
            source_root: "C:/.claude".to_string(),
            source_paths: vec!["C:/source.jsonl".to_string()],
            sessions: vec![session],
            warnings: Vec::new(),
        };
        let value = serde_json::to_value(&inventory).expect("serialize inventory");
        assert_eq!(value["sourceRoot"], "C:/.claude");
        assert!(value.get("sourcePaths").is_some());

        let outcome = ClaudeSessionDeleteResult {
            session_id: "session-a".to_string(),
            backup_path: "C:/backup.jsonl".to_string(),
            message: "deleted".to_string(),
        };
        let value = serde_json::to_value(&outcome).expect("serialize deletion result");
        assert_eq!(value["sessionId"], "session-a");
        assert_eq!(value["backupPath"], "C:/backup.jsonl");
    }

    #[test]
    fn discovers_project_jsonl_and_parses_session_metadata() {
        let temp = tempfile::tempdir().expect("temp home");
        let path = project_session_path(temp.path(), "-workspace-alpha", "file-session");
        write_jsonl(
            &path,
            &[
                json!({
                    "type": "user",
                    "sessionId": "session-a",
                    "cwd": "C:/workspace/alpha",
                    "timestamp": "1970-01-01T00:00:01.250Z",
                    "message": {"role": "user", "content": "First readable prompt"}
                }),
                json!({
                    "type": "assistant",
                    "sessionId": "session-a",
                    "timestamp": "1970-01-01T00:00:02.500Z",
                    "message": {
                        "role": "assistant",
                        "model": "claude-sonnet-4-5",
                        "content": [{"type": "text", "text": "Assistant response"}]
                    }
                }),
                json!({
                    "type": "ai-title",
                    "sessionId": "session-a",
                    "timestamp": "1970-01-01T00:00:03Z",
                    "aiTitle": "Generated title"
                }),
                json!({
                    "type": "custom-title",
                    "sessionId": "session-a",
                    "timestamp": "1970-01-01T00:00:04Z",
                    "customTitle": "Pinned title"
                }),
            ],
        );

        let inventory = list_claude_sessions_from_home(temp.path()).expect("list sessions");
        assert_eq!(inventory.source_paths.len(), 1);
        assert_eq!(inventory.sessions.len(), 1);
        assert!(inventory.warnings.is_empty(), "{:?}", inventory.warnings);

        let session = session_by_id(&inventory, "session-a");
        assert_eq!(session.title, "Pinned title");
        assert_eq!(session.cwd, "C:/workspace/alpha");
        assert_eq!(session.model_provider, "claude-sonnet-4-5");
        assert!(!session.archived);
        assert_eq!(session.updated_at_ms, Some(4_000));
        assert_eq!(session.source_kind, "claude-projects");
        assert_eq!(session.message_count, 2);
        assert_eq!(
            Path::new(&session.source_path),
            fs::canonicalize(path).unwrap()
        );
    }

    #[test]
    fn project_source_merges_resumed_internal_session_ids_into_the_source_file_session() {
        let temp = tempfile::tempdir().expect("temp home");
        let source = project_session_path(temp.path(), "repo", "current-session");
        write_jsonl(
            &source,
            &[
                json!({
                    "type": "user",
                    "sessionId": "previous-session",
                    "timestamp": "2026-07-10T10:00:00Z",
                    "cwd": "D:/Project/repo",
                    "message": { "role": "user", "content": "older prompt" }
                }),
                json!({
                    "type": "custom-title",
                    "sessionId": "current-session",
                    "timestamp": "2026-07-11T10:00:00Z",
                    "customTitle": "Current conversation"
                }),
                json!({
                    "type": "user",
                    "sessionId": "current-session",
                    "timestamp": "2026-07-11T10:01:00Z",
                    "cwd": "D:/Project/repo",
                    "message": { "role": "user", "content": "current prompt" }
                }),
            ],
        );

        let inventory = list_claude_sessions_from_home(temp.path()).expect("list sessions");
        assert_eq!(inventory.sessions.len(), 1);
        assert_eq!(inventory.sessions[0].id, "current-session");
        assert_eq!(inventory.sessions[0].title, "Current conversation");
        assert_eq!(inventory.sessions[0].message_count, 2);

        let backup_root = temp.path().join("backups");
        let deleted =
            delete_claude_session_from_home(temp.path(), &backup_root, "current-session", &source)
                .expect("delete resumed project session");
        assert!(!source.exists());
        assert!(Path::new(&deleted.backup_path).is_file());
    }

    #[test]
    fn applies_title_priority_and_filename_fallback() {
        let temp = tempfile::tempdir().expect("temp home");
        let project = "-workspace-titles";
        write_jsonl(
            &project_session_path(temp.path(), project, "ai-fallback"),
            &[
                json!({
                    "type": "user",
                    "sessionId": "ai-session",
                    "message": {"content": "User title"}
                }),
                json!({
                    "type": "ai-title",
                    "sessionId": "ai-session",
                    "title": "AI title"
                }),
            ],
        );
        write_jsonl(
            &project_session_path(temp.path(), project, "user-fallback"),
            &[json!({
                "type": "user",
                "sessionId": "user-session",
                "message": {"content": "  First   user\ninput  "}
            })],
        );
        write_jsonl(
            &project_session_path(temp.path(), project, "filename-fallback"),
            &[json!({
                "type": "assistant",
                "sessionId": "filename-session",
                "message": {"content": [{"type": "text", "text": "Not a title"}]}
            })],
        );

        let inventory = list_claude_sessions_from_home(temp.path()).expect("list sessions");
        assert_eq!(session_by_id(&inventory, "ai-session").title, "AI title");
        assert_eq!(
            session_by_id(&inventory, "user-session").title,
            "First user input"
        );
        assert_eq!(
            session_by_id(&inventory, "filename-session").title,
            "filename-fallback"
        );
        assert_eq!(session_by_id(&inventory, "filename-session").cwd, project);
    }

    #[test]
    fn malformed_jsonl_lines_do_not_hide_valid_records_and_subagents_are_excluded() {
        let temp = tempfile::tempdir().expect("temp home");
        let path = project_session_path(temp.path(), "-workspace-beta", "root-session");
        let valid_user = json!({
            "type": "user",
            "sessionId": "root-session",
            "message": {"content": "Root prompt"}
        });
        let valid_assistant = json!({
            "type": "assistant",
            "sessionId": "root-session",
            "message": {"content": [{"type": "text", "text": "Root response"}]}
        });
        write_file(
            &path,
            &format!("{}\n{{not-json\n{}\n", valid_user, valid_assistant),
        );
        write_jsonl(
            &path
                .parent()
                .unwrap()
                .join("subagents")
                .join("agent-session.jsonl"),
            &[json!({
                "type": "user",
                "sessionId": "agent-session",
                "message": {"content": "Nested agent prompt"}
            })],
        );

        let inventory = list_claude_sessions_from_home(temp.path()).expect("list sessions");
        assert_eq!(inventory.source_paths.len(), 1);
        assert_eq!(inventory.sessions.len(), 1);
        assert_eq!(session_by_id(&inventory, "root-session").message_count, 2);
        assert!(
            inventory
                .warnings
                .iter()
                .any(|warning| warning.contains("malformed") && warning.contains("1"))
        );
        assert!(
            inventory
                .sessions
                .iter()
                .all(|session| session.id != "agent-session")
        );
    }

    #[test]
    fn discovers_all_supplemental_json_and_jsonl_roots() {
        let temp = tempfile::tempdir().expect("temp home");
        let fixtures = [
            (
                ".claude/sessions",
                "claude-sessions",
                "supplement-1",
                "json",
            ),
            (
                ".claude/claude-code-sessions",
                "claude-code-sessions",
                "supplement-2",
                "jsonl",
            ),
            (
                ".claude/local-agent-mode-sessions",
                "local-agent-mode-sessions",
                "supplement-3",
                "json",
            ),
            (
                ".config/claude-code-sessions",
                "claude-code-sessions",
                "supplement-4",
                "jsonl",
            ),
            (
                ".config/local-agent-mode-sessions",
                "local-agent-mode-sessions",
                "supplement-5",
                "json",
            ),
        ];

        for (root, _, id, extension) in fixtures {
            let path = temp
                .path()
                .join(root)
                .join("nested")
                .join(format!("{id}.{extension}"));
            if extension == "json" {
                write_file(
                    &path,
                    &json!({
                        "sessionId": id,
                        "title": format!("Title {id}"),
                        "messages": [
                            {"role": "user", "content": "Prompt"},
                            {"role": "assistant", "model": "claude-opus", "content": [{"type": "text", "text": "Response"}]}
                        ]
                    })
                    .to_string(),
                );
            } else {
                write_jsonl(
                    &path,
                    &[json!({
                        "type": "user",
                        "sessionId": id,
                        "message": {"content": "Prompt"}
                    })],
                );
            }
        }

        let inventory = list_claude_sessions_from_home(temp.path()).expect("list sessions");
        assert_eq!(inventory.source_paths.len(), fixtures.len());
        assert_eq!(inventory.sessions.len(), fixtures.len());
        for (_, kind, id, _) in fixtures {
            assert_eq!(session_by_id(&inventory, id).source_kind, kind);
        }
        assert_eq!(session_by_id(&inventory, "supplement-1").message_count, 2);
        assert_eq!(
            session_by_id(&inventory, "supplement-1").model_provider,
            "claude-opus"
        );
    }

    #[test]
    fn discovers_direct_audit_and_local_session_sources() {
        let temp = tempfile::tempdir().expect("temp home");
        let audit = temp.path().join(".claude/audit.jsonl");
        let local = temp.path().join(".claude/local_workspace.json");
        write_jsonl(
            &audit,
            &[json!({
                "type": "user",
                "sessionId": "audit-session",
                "message": {"content": "Audit prompt"}
            })],
        );
        write_file(
            &local,
            &json!({
                "sessionId": "local-session",
                "messages": [{"role": "user", "content": "Local prompt"}]
            })
            .to_string(),
        );

        let inventory = list_claude_sessions_from_home(temp.path()).expect("list sessions");
        assert_eq!(inventory.source_paths.len(), 2);
        assert_eq!(
            session_by_id(&inventory, "audit-session").source_kind,
            "claude-audit"
        );
        assert_eq!(
            session_by_id(&inventory, "local-session").source_kind,
            "claude-local"
        );
    }

    #[test]
    fn deletion_backs_up_the_complete_single_session_source_before_removal() {
        let temp = tempfile::tempdir().expect("temp home");
        let backup_root = temp.path().join("backups");
        let source = project_session_path(temp.path(), "-workspace-delete", "delete-me");
        let body = format!(
            "{}\n",
            json!({
                "type": "user",
                "sessionId": "delete-session",
                "message": {"content": "Delete fixture"}
            })
        );
        write_file(&source, &body);
        let inventory = list_claude_sessions_from_home(temp.path()).expect("list sessions");
        let listed = session_by_id(&inventory, "delete-session");

        let result = delete_claude_session_from_home(
            temp.path(),
            &backup_root,
            &listed.id,
            Path::new(&listed.source_path),
        )
        .expect("delete session");

        assert_eq!(result.session_id, "delete-session");
        assert!(!source.exists());
        let backup = PathBuf::from(result.backup_path);
        assert!(backup.starts_with(backup_root.join("claude-sessions")));
        assert_eq!(fs::read_to_string(backup).unwrap(), body);
        assert!(!result.message.is_empty());
    }

    #[test]
    fn deletion_rejects_untrusted_paths_without_touching_the_file() {
        let temp = tempfile::tempdir().expect("temp home");
        let untrusted = temp.path().join("outside.jsonl");
        write_jsonl(
            &untrusted,
            &[json!({
                "type": "user",
                "sessionId": "outside-session",
                "message": {"content": "Outside"}
            })],
        );

        let error = delete_claude_session_from_home(
            temp.path(),
            &temp.path().join("backups"),
            "outside-session",
            &untrusted,
        )
        .expect_err("untrusted path must fail");

        assert!(error.to_string().contains("trusted"));
        assert!(untrusted.exists());
    }

    #[test]
    fn deletion_rejects_shared_multi_session_sources() {
        let temp = tempfile::tempdir().expect("temp home");
        let source = temp
            .path()
            .join(".claude")
            .join("sessions")
            .join("shared.jsonl");
        write_jsonl(
            &source,
            &[
                json!({
                    "type": "user",
                    "sessionId": "shared-a",
                    "message": {"content": "A"}
                }),
                json!({
                    "type": "user",
                    "sessionId": "shared-b",
                    "message": {"content": "B"}
                }),
            ],
        );

        let error = delete_claude_session_from_home(
            temp.path(),
            &temp.path().join("backups"),
            "shared-a",
            &source,
        )
        .expect_err("shared source must fail");

        assert!(error.to_string().contains("multiple"));
        assert!(source.exists());
    }

    #[test]
    fn backup_failure_preserves_the_source_file() {
        let temp = tempfile::tempdir().expect("temp home");
        let source = project_session_path(temp.path(), "-workspace-backup", "keep-me");
        write_jsonl(
            &source,
            &[json!({
                "type": "user",
                "sessionId": "keep-session",
                "message": {"content": "Keep this"}
            })],
        );
        let blocked_backup_root = temp.path().join("blocked-backup-root");
        fs::write(&blocked_backup_root, "not a directory").unwrap();

        delete_claude_session_from_home(temp.path(), &blocked_backup_root, "keep-session", &source)
            .expect_err("backup failure must stop deletion");

        assert!(source.exists());
    }

    #[test]
    fn loads_latest_context_page_with_resumed_history_and_roles() {
        let temp = tempfile::tempdir().expect("temp home");
        let source = project_session_path(temp.path(), "repo", "current-session");
        write_jsonl(
            &source,
            &[
                json!({
                    "type": "user",
                    "sessionId": "previous-session",
                    "timestamp": "2026-07-10T10:00:00Z",
                    "cwd": "D:/Project/repo",
                    "message": { "role": "user", "content": "older prompt" }
                }),
                json!({
                    "type": "assistant",
                    "sessionId": "previous-session",
                    "timestamp": "2026-07-10T10:01:00Z",
                    "message": { "role": "assistant", "content": [{"type": "text", "text": "older response"}] }
                }),
                json!({
                    "type": "user",
                    "sessionId": "current-session",
                    "timestamp": "2026-07-11T10:00:00Z",
                    "message": { "role": "user", "content": "current prompt" }
                }),
                json!({
                    "type": "assistant",
                    "sessionId": "current-session",
                    "timestamp": "2026-07-11T10:01:00Z",
                    "message": { "role": "assistant", "content": [{"type": "text", "text": "current response"}] }
                }),
                json!({
                    "type": "user",
                    "sessionId": "current-session",
                    "timestamp": "2026-07-11T10:02:00Z",
                    "message": { "role": "user", "content": [{"type": "tool_result", "content": "tool output"}] }
                }),
                json!({
                    "type": "system",
                    "sessionId": "current-session",
                    "timestamp": "2026-07-11T10:03:00Z",
                    "message": { "role": "system", "content": "system note" }
                }),
            ],
        );

        let page = load_claude_session_context_from_home(
            temp.path(),
            "current-session",
            &source,
            None,
            Some(3),
        )
        .expect("load latest context page");

        assert_eq!(page.session_id, "current-session");
        assert_eq!(page.cwd, "D:/Project/repo");
        assert_eq!(page.total_messages, 6);
        assert_eq!(page.offset, 3);
        assert!(page.has_more_before);
        assert_eq!(
            page.messages
                .iter()
                .map(|message| (
                    message.sequence,
                    message.role.as_str(),
                    message.text.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![
                (4, "assistant", "current response"),
                (5, "tool", "tool output"),
                (6, "system", "system note"),
            ]
        );
    }

    #[test]
    fn loads_earlier_context_page_without_repeating_messages() {
        let temp = tempfile::tempdir().expect("temp home");
        let source = project_session_path(temp.path(), "repo", "paged-session");
        let records = (1..=6)
            .map(|index| {
                json!({
                    "type": if index % 2 == 0 { "assistant" } else { "user" },
                    "sessionId": "paged-session",
                    "message": {
                        "role": if index % 2 == 0 { "assistant" } else { "user" },
                        "content": format!("message {index}")
                    }
                })
            })
            .collect::<Vec<_>>();
        write_jsonl(&source, &records);

        let page = load_claude_session_context_from_home(
            temp.path(),
            "paged-session",
            &source,
            Some(2),
            Some(2),
        )
        .expect("load earlier context page");

        assert_eq!(page.total_messages, 6);
        assert_eq!(page.offset, 2);
        assert!(page.has_more_before);
        assert_eq!(
            page.messages
                .iter()
                .map(|message| message.text.as_str())
                .collect::<Vec<_>>(),
            vec!["message 3", "message 4"]
        );
    }

    #[test]
    fn context_loading_filters_shared_sources_and_rejects_untrusted_paths() {
        let temp = tempfile::tempdir().expect("temp home");
        let source = temp.path().join(".claude/sessions/shared.jsonl");
        write_jsonl(
            &source,
            &[
                json!({
                    "type": "user",
                    "sessionId": "session-a",
                    "message": { "role": "user", "content": "only A" }
                }),
                json!({
                    "type": "assistant",
                    "sessionId": "session-b",
                    "message": { "role": "assistant", "content": "only B" }
                }),
            ],
        );

        let page =
            load_claude_session_context_from_home(temp.path(), "session-a", &source, None, None)
                .expect("load one session from shared source");
        assert_eq!(page.total_messages, 1);
        assert_eq!(page.messages[0].text, "only A");

        let untrusted = temp.path().join("outside.jsonl");
        write_jsonl(
            &untrusted,
            &[json!({
                "type": "user",
                "sessionId": "outside",
                "message": { "role": "user", "content": "outside" }
            })],
        );
        let error =
            load_claude_session_context_from_home(temp.path(), "outside", &untrusted, None, None)
                .expect_err("untrusted context source must fail");
        assert!(error.to_string().contains("trusted"));
    }
}
