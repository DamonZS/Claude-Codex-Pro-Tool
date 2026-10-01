//! AITRACKER capability projection over CCP's local, privacy-filtered snapshot.

use crate::local_usage::{LocalUsageEvent, LocalUsageSnapshot, definition_base};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST: &str = include_str!("../../../assets/aitracker/definitions/manifest.json");
const DEFINITION_FILES: &[(&str, &str)] = &[
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolDefinition {
    pub id: String,
    pub name: String,
    pub name_zh: String,
    pub icon: String,
    pub color: String,
    pub platforms: BTreeMap<String, String>,
    pub usage_mode: String,
    pub context_mode: String,
    pub sessions_mode: String,
    pub detected: bool,
    pub events: usize,
    pub skill_count: Option<usize>,
    pub skill_scan_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentTokenTotals {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionSummary {
    pub session_id: String,
    pub agent: String,
    pub provider: String,
    pub model: String,
    pub project: String,
    pub started_at: String,
    pub ended_at: String,
    pub events: usize,
    pub tool_calls: usize,
    pub status: String,
    pub totals: AgentTokenTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCallEvent {
    pub id: String,
    pub agent: String,
    pub tool: String,
    pub session_id: Option<String>,
    pub timestamp: String,
    pub status: String,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentDetail {
    pub id: String,
    pub events: usize,
    pub sessions: usize,
    pub tools: Vec<(String, usize)>,
    pub models: Vec<(String, usize)>,
    pub providers: Vec<(String, usize)>,
    pub totals: AgentTokenTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AitrackerSnapshot {
    pub registry: Vec<AgentToolDefinition>,
    pub sessions: Vec<AgentSessionSummary>,
    pub tool_calls: Vec<AgentToolCallEvent>,
    pub details: Vec<AgentDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AitrackerSessionDetail {
    pub summary: AgentSessionSummary,
    pub events: Vec<LocalUsageEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DistillationSourceRef {
    pub agent: String,
    pub session_id: String,
    pub project: String,
    pub start_index: usize,
    pub end_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DistillationCandidate {
    pub id: String,
    pub agent: String,
    pub session_id: String,
    pub summary: String,
    pub status: String,
    pub created_at: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub provider_id: String,
    #[serde(default)]
    pub model_id: String,
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub source_refs: Vec<DistillationSourceRef>,
}

fn value<'a>(record: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(record, |current, part| current.get(part))
}

fn string(record: &Value, paths: &[&str], fallback: &str) -> String {
    paths
        .iter()
        .find_map(|path| value(record, path).and_then(Value::as_str))
        .unwrap_or(fallback)
        .to_string()
}

fn definition(id: &str, source: &str, events: usize) -> AgentToolDefinition {
    let json: Value = serde_json::from_str(source).unwrap_or_default();
    let display = json.get("display").cloned().unwrap_or_default();
    let capabilities = json.get("capabilities").cloned().unwrap_or_default();
    let platforms = json
        .get("platforms")
        .and_then(Value::as_object)
        .map(|items| {
            items
                .iter()
                .map(|(key, value)| (key.clone(), value.as_str().unwrap_or("planned").to_string()))
                .collect()
        })
        .unwrap_or_default();
    AgentToolDefinition {
        id: id.to_string(),
        name: string(&display, &["name"], id),
        name_zh: string(&display, &["nameZh", "name"], id),
        icon: string(&display, &["icon"], "other"),
        color: string(&display, &["color"], "#54AFFF"),
        platforms,
        usage_mode: string(&capabilities, &["usage.mode"], "unsupported"),
        context_mode: string(&capabilities, &["context.mode"], "unsupported"),
        sessions_mode: string(&capabilities, &["sessions.mode"], "unsupported"),
        detected: events > 0,
        events,
        skill_count: None,
        skill_scan_status: "unsupported".into(),
    }
}

fn skill_roots(skills: &Value, home: &Path) -> Vec<PathBuf> {
    let env_home = skills
        .get("envHome")
        .and_then(Value::as_str)
        .and_then(std::env::var_os)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    skills
        .get("rootSpecs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|spec| {
            let relative = spec.get("path")?.as_str()?;
            let base = spec.get("base")?.as_str()?;
            let root = if let Some(override_home) = &env_home {
                override_home.join(Path::new(relative).file_name()?)
            } else {
                match base {
                    "home" | "userProfile" => home.join(relative),
                    _ => return None,
                }
            };
            Some(root)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Resolves the writable local Skill roots declared by every AITRACKER tool definition.
pub fn skill_roots_for_agents(home: &Path) -> BTreeMap<String, Vec<PathBuf>> {
    DEFINITION_FILES
        .iter()
        .filter_map(|(id, source)| {
            let definition = serde_json::from_str::<Value>(source).ok()?;
            let skills = definition.pointer("/storage/skills")?;
            let roots = skill_roots(skills, home);
            (!roots.is_empty()).then_some(((*id).to_string(), roots))
        })
        .collect()
}

fn scan_skill_roots(
    roots: &[PathBuf],
    markers: &[&str],
    max_depth: usize,
) -> (Option<usize>, &'static str) {
    let mut names = BTreeSet::new();
    let mut found_root = false;
    let mut visited = 0usize;
    let mut stack = roots
        .iter()
        .map(|root| (root.clone(), 0usize))
        .collect::<Vec<_>>();
    while let Some((directory, depth)) = stack.pop() {
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return (None, "error"),
        }
        if depth == 0 {
            found_root = true;
        }
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => return (None, "error"),
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => return (None, "error"),
            };
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            let path = entry.path();
            let kind = match entry.file_type() {
                Ok(kind) => kind,
                Err(_) => return (None, "error"),
            };
            if !kind.is_dir() || kind.is_symlink() {
                continue;
            }
            visited += 1;
            if visited > 100_000 {
                return (None, "error");
            }
            let has_marker = markers.iter().any(|marker| {
                fs::symlink_metadata(path.join(marker))
                    .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
            });
            if has_marker {
                names.insert(name.to_string_lossy().to_lowercase());
            } else if depth + 1 < max_depth {
                stack.push((path, depth + 1));
            }
        }
    }
    if found_root {
        (Some(names.len()), "ok")
    } else {
        (Some(0), "missing")
    }
}

fn location_matches_platform(location: &Value) -> bool {
    location
        .get("targets")
        .and_then(Value::as_array)
        .map(|targets| {
            targets.iter().any(|target| match target.as_str() {
                Some("windows" | "windows10" | "windows11") => cfg!(windows),
                Some("macos") => cfg!(target_os = "macos"),
                Some("linux") => cfg!(target_os = "linux"),
                _ => false,
            })
        })
        .unwrap_or(true)
}

fn installation_detected(definition: &Value, home: &Path) -> bool {
    definition
        .pointer("/detection/locations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|location| location_matches_platform(location))
        .any(|location| {
            let Some(relative) = location.get("path").and_then(Value::as_str) else {
                return false;
            };
            if relative.is_empty() {
                return false;
            }
            let base = location
                .get("base")
                .and_then(Value::as_str)
                .unwrap_or("home");
            fs::metadata(definition_base(home, base).join(relative)).is_ok()
        })
}

pub fn populate_local_capabilities(registry: &mut [AgentToolDefinition], home: &Path) {
    for agent in registry {
        let Some((_, source)) = DEFINITION_FILES.iter().find(|(id, _)| *id == agent.id) else {
            continue;
        };
        let Ok(definition) = serde_json::from_str::<Value>(source) else {
            agent.skill_scan_status = "error".into();
            continue;
        };
        agent.detected |= installation_detected(&definition, home);
        let Some(skills) = definition.pointer("/storage/skills") else {
            continue;
        };
        let roots = skill_roots(skills, home);
        let markers = skills
            .get("markers")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .filter(|items| !items.is_empty())
            .unwrap_or_else(|| vec!["SKILL.md", "skill.md"]);
        let depth = skills
            .get("maxDepth")
            .and_then(Value::as_u64)
            .unwrap_or(3)
            .clamp(1, 16) as usize;
        let (count, status) = scan_skill_roots(&roots, &markers, depth);
        agent.skill_count = count;
        agent.skill_scan_status = status.into();
    }
}

pub fn agent_registry(snapshot: &LocalUsageSnapshot) -> Vec<AgentToolDefinition> {
    let mut counts = BTreeMap::<String, usize>::new();
    for event in &snapshot.details {
        *counts.entry(event.agent.clone()).or_default() += 1;
    }
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap_or_default();
    manifest
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let id = entry.get("id")?.as_str()?;
            let source = DEFINITION_FILES
                .iter()
                .find(|(key, _)| *key == id)
                .map(|(_, source)| *source)?;
            Some(definition(
                id,
                source,
                counts.get(id).copied().unwrap_or_default(),
            ))
        })
        .collect()
}

fn add_tokens(target: &mut AgentTokenTotals, event: &LocalUsageEvent) {
    target.input_tokens += event.input_tokens;
    target.cached_input_tokens += event.cached_input_tokens;
    target.cache_creation_input_tokens += event.cache_creation_input_tokens;
    target.output_tokens += event.output_tokens;
    target.reasoning_output_tokens += event.reasoning_output_tokens;
    target.total_tokens += event.total_tokens;
}

pub fn session_summaries(snapshot: &LocalUsageSnapshot) -> Vec<AgentSessionSummary> {
    let mut groups = BTreeMap::<String, Vec<&LocalUsageEvent>>::new();
    for event in &snapshot.details {
        if let Some(session) = &event.session_id {
            groups
                .entry(format!("{}:{session}", event.agent))
                .or_default()
                .push(event);
        }
    }
    let mut summaries = groups
        .into_iter()
        .map(|(key, events)| {
            let first = events.first().expect("group is non-empty");
            let last = events.last().unwrap_or(first);
            let mut totals = AgentTokenTotals::default();
            let mut statuses = BTreeMap::<String, usize>::new();
            for event in &events {
                add_tokens(&mut totals, event);
                *statuses.entry(event.status.clone()).or_default() += 1;
            }
            AgentSessionSummary {
                session_id: key
                    .split_once(':')
                    .map(|(_, id)| id.to_string())
                    .unwrap_or(key),
                agent: first.agent.clone(),
                provider: first.provider.clone(),
                model: first.model.clone(),
                project: first.project.clone(),
                started_at: first.timestamp.clone().min(last.timestamp.clone()),
                ended_at: first.timestamp.clone().max(last.timestamp.clone()),
                events: events.len(),
                tool_calls: events
                    .iter()
                    .filter(|event| event.tool_name.is_some())
                    .count(),
                status: statuses
                    .into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(status, _)| status)
                    .unwrap_or_else(|| "observed".into()),
                totals,
            }
        })
        .collect::<Vec<_>>();
    let existing = summaries
        .iter()
        .map(|summary| format!("{}:{}", summary.agent, summary.session_id))
        .collect::<BTreeSet<_>>();
    summaries.extend(
        snapshot
            .sessions
            .iter()
            .filter(|session| !existing.contains(&format!("{}:{}", session.agent, session.id)))
            .map(|session| AgentSessionSummary {
                session_id: session.id.clone(),
                agent: session.agent.clone(),
                provider: session.provider.clone(),
                model: session.model.clone(),
                project: session.project.clone(),
                started_at: session.started_at.clone(),
                ended_at: session.ended_at.clone(),
                events: 0,
                tool_calls: 0,
                status: session.status.clone(),
                totals: AgentTokenTotals::default(),
            }),
    );
    summaries.sort_by(|left, right| right.ended_at.cmp(&left.ended_at));
    summaries
}

pub fn tool_calls(snapshot: &LocalUsageSnapshot) -> Vec<AgentToolCallEvent> {
    snapshot
        .details
        .iter()
        .filter_map(|event| {
            event.tool_name.as_ref().map(|tool| AgentToolCallEvent {
                id: event.id.clone(),
                agent: event.agent.clone(),
                tool: tool.clone(),
                session_id: event.session_id.clone(),
                timestamp: event.timestamp.clone(),
                status: event.status.clone(),
                duration_ms: event.duration_ms,
            })
        })
        .collect()
}

pub fn agent_details(snapshot: &LocalUsageSnapshot) -> Vec<AgentDetail> {
    let mut grouped = BTreeMap::<String, AgentDetail>::new();
    let mut sessions = BTreeMap::<String, BTreeSet<String>>::new();
    for event in &snapshot.details {
        let detail = grouped
            .entry(event.agent.clone())
            .or_insert_with(|| AgentDetail {
                id: event.agent.clone(),
                events: 0,
                sessions: 0,
                tools: Vec::new(),
                models: Vec::new(),
                providers: Vec::new(),
                totals: AgentTokenTotals::default(),
            });
        detail.events += 1;
        add_tokens(&mut detail.totals, event);
        if let Some(session_id) = &event.session_id {
            sessions
                .entry(event.agent.clone())
                .or_default()
                .insert(session_id.clone());
        }
        for (items, key) in [
            (&mut detail.models, event.model.clone()),
            (&mut detail.providers, event.provider.clone()),
        ] {
            if let Some(row) = items.iter_mut().find(|(name, _)| *name == key) {
                row.1 += 1;
            } else {
                items.push((key, 1));
            }
        }
        if let Some(tool) = &event.tool_name {
            if let Some(row) = detail.tools.iter_mut().find(|(name, _)| name == tool) {
                row.1 += 1;
            } else {
                detail.tools.push((tool.clone(), 1));
            }
        }
    }
    grouped
        .into_values()
        .map(|mut detail| {
            detail.sessions = sessions.get(&detail.id).map_or(0, BTreeSet::len);
            detail.tools.sort_by(|a, b| b.1.cmp(&a.1));
            detail.models.sort_by(|a, b| b.1.cmp(&a.1));
            detail.providers.sort_by(|a, b| b.1.cmp(&a.1));
            detail
        })
        .collect()
}

pub fn project_snapshot(snapshot: &LocalUsageSnapshot) -> AitrackerSnapshot {
    AitrackerSnapshot {
        registry: agent_registry(snapshot),
        sessions: session_summaries(snapshot),
        tool_calls: tool_calls(snapshot),
        details: agent_details(snapshot),
    }
}

pub fn query_sessions(
    snapshot: &LocalUsageSnapshot,
    agent: Option<&str>,
    status: Option<&str>,
    keyword: Option<&str>,
    range: Option<&str>,
    page: usize,
    page_size: usize,
) -> (Vec<AgentSessionSummary>, usize) {
    let keyword = keyword
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);
    let cutoff = match range {
        Some("7d") => Some(Utc::now() - Duration::days(7)),
        Some("30d") => Some(Utc::now() - Duration::days(30)),
        Some("90d") => Some(Utc::now() - Duration::days(90)),
        _ => None,
    };
    let mut matches = session_summaries(snapshot)
        .into_iter()
        .filter(|item| {
            cutoff.map_or(true, |value| {
                chrono::DateTime::parse_from_rfc3339(&item.started_at)
                    .map(|date| date.with_timezone(&Utc) >= value)
                    .unwrap_or(true)
            })
        })
        .filter(|item| {
            agent
                .map(|value| value.is_empty() || item.agent == value)
                .unwrap_or(true)
        })
        .filter(|item| {
            status
                .map(|value| value.is_empty() || item.status == value)
                .unwrap_or(true)
        })
        .filter(|item| {
            keyword
                .as_deref()
                .map(|needle| {
                    [
                        item.agent.as_str(),
                        item.provider.as_str(),
                        item.model.as_str(),
                        item.project.as_str(),
                        item.session_id.as_str(),
                    ]
                    .into_iter()
                    .any(|value| value.to_lowercase().contains(needle))
                })
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| right.ended_at.cmp(&left.ended_at));
    let total = matches.len();
    let page_size = page_size.clamp(1, 200);
    let start = page.saturating_sub(1).saturating_mul(page_size).min(total);
    let end = start.saturating_add(page_size).min(total);
    (matches[start..end].to_vec(), total)
}

pub fn session_detail(
    snapshot: &LocalUsageSnapshot,
    agent: &str,
    session_id: &str,
) -> Option<AitrackerSessionDetail> {
    let mut events = snapshot
        .details
        .iter()
        .filter(|event| event.agent == agent && event.session_id.as_deref() == Some(session_id))
        .cloned()
        .collect::<Vec<_>>();
    events.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
    let summary = session_summaries(snapshot)
        .into_iter()
        .find(|item| item.agent == agent && item.session_id == session_id)?;
    Some(AitrackerSessionDetail { summary, events })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_usage::{LocalUsageEvent, aggregate};

    fn event(agent: &str, session: &str, tool: Option<&str>) -> LocalUsageEvent {
        LocalUsageEvent {
            id: format!("{agent}-{session}-{tool:?}"),
            source: agent.into(),
            agent: agent.into(),
            provider: "p".into(),
            status: "ok".into(),
            duration_ms: Some(12),
            timestamp: "2026-09-27T00:00:00Z".into(),
            model: "m".into(),
            project: "demo".into(),
            session_id: Some(session.into()),
            input_tokens: 1,
            cached_input_tokens: 0,
            cache_creation_input_tokens: 0,
            output_tokens: 2,
            reasoning_output_tokens: 0,
            total_tokens: 3,
            measurement: "observed".into(),
            tool_name: tool.map(str::to_string),
        }
    }
    #[test]
    fn distillation_candidate_keeps_selected_source_ranges_and_reads_legacy_records() {
        let legacy: DistillationCandidate = serde_json::from_value(serde_json::json!({
            "id": "old",
            "agent": "codex",
            "sessionId": "session-1",
            "summary": "old candidate",
            "status": "pending",
            "createdAt": "1"
        }))
        .unwrap();
        assert!(legacy.source_refs.is_empty());

        let source = DistillationSourceRef {
            agent: "codex".into(),
            session_id: "session-2".into(),
            project: "ccp".into(),
            start_index: 3,
            end_index: 7,
        };
        let encoded = serde_json::to_value(source).unwrap();
        assert_eq!(encoded["startIndex"], 3);
        assert_eq!(encoded["endIndex"], 7);
        assert_eq!(encoded["project"], "ccp");
    }

    #[test]
    fn registry_matches_manifest_and_projects_tool_calls() {
        let snapshot = aggregate(vec![event("codex", "s", Some("terminal"))], Vec::new());
        assert_eq!(agent_registry(&snapshot).len(), 36);
        assert_eq!(tool_calls(&snapshot)[0].tool, "terminal");
        assert_eq!(session_summaries(&snapshot)[0].tool_calls, 1);
    }

    #[test]
    fn session_query_applies_time_range_before_pagination() {
        let now = Utc::now();
        let mut recent = event("codex", "recent", None);
        recent.timestamp = (now - Duration::days(3)).to_rfc3339();
        let mut older = event("codex", "older", None);
        older.timestamp = (now - Duration::days(12)).to_rfc3339();
        let snapshot = aggregate(vec![recent, older], Vec::new());
        let (sessions, total) = query_sessions(&snapshot, None, None, None, Some("7d"), 1, 20);
        assert_eq!(total, 1);
        assert_eq!(sessions[0].session_id, "recent");
    }

    #[test]
    fn skill_coverage_scans_definition_roots_and_distinguishes_missing() {
        let fixture = tempfile::tempdir().unwrap();
        let home = fixture.path();
        for path in [
            ".workbuddy/skills/alpha/SKILL.md",
            ".workbuddy/plugins/cache/alpha/skill.md",
            ".workbuddy/plugins/cache/beta/SKILL.md",
            ".workbuddy/skills/.hidden/SKILL.md",
            ".workbuddy/skills/bare.md",
        ] {
            let target = home.join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, "test").unwrap();
        }
        let mut registry = agent_registry(&aggregate(Vec::new(), Vec::new()));
        populate_local_capabilities(&mut registry, home);
        let workbuddy = registry
            .iter()
            .find(|agent| agent.id == "workbuddy")
            .unwrap();
        assert_eq!(workbuddy.skill_count, Some(2));
        assert_eq!(workbuddy.skill_scan_status, "ok");
        assert!(workbuddy.detected);
        let claude = registry
            .iter()
            .find(|agent| agent.id == "claude-code")
            .unwrap();
        assert_eq!(claude.skill_count, Some(0));
        assert_eq!(claude.skill_scan_status, "missing");
        let unsupported = registry
            .iter()
            .find(|agent| agent.id == "cherrystudio")
            .unwrap();
        assert_eq!(unsupported.skill_count, None);
        assert_eq!(unsupported.skill_scan_status, "unsupported");
    }

    #[test]
    fn installed_workbuddy_is_detected_without_usage_or_skill_records() {
        let fixture = tempfile::tempdir().unwrap();
        fs::create_dir(fixture.path().join(".workbuddy")).unwrap();
        let mut registry = agent_registry(&aggregate(Vec::new(), Vec::new()));
        populate_local_capabilities(&mut registry, fixture.path());
        let workbuddy = registry
            .iter()
            .find(|agent| agent.id == "workbuddy")
            .unwrap();
        assert!(workbuddy.detected);
        assert_eq!(workbuddy.skill_count, Some(0));
        assert_eq!(workbuddy.skill_scan_status, "missing");
    }

    #[test]
    fn skill_coverage_respects_depth_and_skips_directory_links() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("skills");
        for path in [
            "direct/SKILL.md",
            "group/nested/SKILL.md",
            "group/too/deep/SKILL.md",
        ] {
            let target = root.join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, "test").unwrap();
        }
        assert_eq!(
            scan_skill_roots(&[root.clone()], &["SKILL.md"], 2),
            (Some(2), "ok")
        );
        #[cfg(windows)]
        if std::os::windows::fs::symlink_dir(root.join("direct"), root.join("linked")).is_ok() {
            assert_eq!(scan_skill_roots(&[root], &["SKILL.md"], 2), (Some(2), "ok"));
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("direct"), root.join("linked")).unwrap();
            assert_eq!(scan_skill_roots(&[root], &["SKILL.md"], 2), (Some(2), "ok"));
        }
    }
}
