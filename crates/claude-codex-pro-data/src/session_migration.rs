use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::markdown::{
    ResolvedCodexThread, SessionMessage, load_session_messages, resolve_codex_thread,
};

/// Target serialization format for a universal session export (阶段 A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionExportFormat {
    /// Human-readable Markdown transcript.
    Markdown,
    /// Provider-neutral one-object-per-line JSONL of `{role, timestamp, text}`.
    Jsonl,
    /// Claude Code turn schema JSONL (`~/.claude/projects/<slug>/<uuid>.jsonl`).
    ClaudeCodeJsonl,
}

impl SessionExportFormat {
    fn file_extension(self) -> &'static str {
        match self {
            SessionExportFormat::Markdown => "md",
            SessionExportFormat::Jsonl | SessionExportFormat::ClaudeCodeJsonl => "jsonl",
        }
    }
}

/// Result of a universal export: the serialized text plus a suggested filename.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionExport {
    pub session_id: String,
    pub title: String,
    pub format: SessionExportFormat,
    pub filename: String,
    pub content: String,
    pub message_count: usize,
}

/// 阶段 A: read a Codex thread from `db_path` and serialize it into `format`.
/// Returns `Ok(None)` when the thread cannot be resolved (missing/unsupported),
/// so the caller can distinguish "not found" from a hard error.
pub fn export_session_universal(
    db_path: &Path,
    session_id: &str,
    format: SessionExportFormat,
) -> anyhow::Result<Option<SessionExport>> {
    let Some(thread) = resolve_codex_thread(db_path, session_id)? else {
        return Ok(None);
    };
    let messages = load_session_messages(&thread.rollout_path)?;
    if messages.is_empty() {
        return Ok(None);
    }
    let thread_id = normalize_session_id(session_id);
    let content = match format {
        SessionExportFormat::Markdown => render_markdown(&thread.title, &messages),
        SessionExportFormat::Jsonl => render_universal_jsonl(&messages)?,
        SessionExportFormat::ClaudeCodeJsonl => {
            render_claude_code_jsonl(&thread, &thread_id, &messages)?
        }
    };
    Ok(Some(SessionExport {
        filename: build_export_filename(&thread.title, &thread_id, format),
        title: thread.title,
        format,
        content,
        message_count: messages.len(),
        session_id: thread_id,
    }))
}

/// Outcome of a Codex → Claude Code migration (阶段 B).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeCodeMigration {
    pub session_id: String,
    pub project_slug: String,
    pub written_path: String,
    pub message_count: usize,
    pub already_migrated: bool,
}

/// 阶段 B: migrate a Codex thread into a Claude Code project JSONL under
/// `<claude_home>/projects/<slug>/<uuid>.jsonl`. The slug derives from the
/// thread cwd (Claude Code's own convention) or an explicit `target_cwd`
/// override. `already_migrated_uuid` short-circuits when settings recorded a
/// prior migration, keeping the operation idempotent.
pub fn migrate_codex_thread_to_claude_code(
    db_path: &Path,
    session_id: &str,
    claude_home: &Path,
    target_cwd: Option<&str>,
    already_migrated_uuid: Option<&str>,
) -> anyhow::Result<Option<ClaudeCodeMigration>> {
    let projects_dir = claude_home.join("projects");
    let Some(thread) = resolve_codex_thread(db_path, session_id)? else {
        return Ok(None);
    };
    let messages = load_session_messages(&thread.rollout_path)?;
    if messages.is_empty() {
        return Ok(None);
    }
    let thread_id = normalize_session_id(session_id);
    let cwd = target_cwd
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| thread.cwd.clone());
    let slug = claude_code_project_slug(&cwd);
    let project_dir = projects_dir.join(&slug);

    // Idempotency: if a prior migration UUID is still on disk, do not write a
    // duplicate transcript.
    if let Some(existing_uuid) = already_migrated_uuid {
        let existing = project_dir.join(format!("{existing_uuid}.jsonl"));
        if existing.is_file() {
            return Ok(Some(ClaudeCodeMigration {
                session_id: thread_id,
                project_slug: slug,
                written_path: existing.to_string_lossy().to_string(),
                message_count: messages.len(),
                already_migrated: true,
            }));
        }
    }

    std::fs::create_dir_all(&project_dir)?;
    let session_uuid = uuid::Uuid::new_v4().to_string();
    let content = render_claude_code_jsonl(&thread, &thread_id, &messages)?;
    let written = project_dir.join(format!("{session_uuid}.jsonl"));
    // The filename carries a fresh random UUID, so a pre-existing file here would
    // be an astronomically unlikely collision; refuse to clobber it rather than
    // overwrite an unrelated Claude Code transcript.
    if written.exists() {
        anyhow::bail!(
            "refusing to overwrite existing Claude Code transcript: {}",
            written.display()
        );
    }
    std::fs::write(&written, content.as_bytes())?;
    Ok(Some(ClaudeCodeMigration {
        session_id: thread_id,
        project_slug: slug,
        written_path: written.to_string_lossy().to_string(),
        message_count: messages.len(),
        already_migrated: false,
    }))
}

/// Whether Claude Code appears installed for this user (its projects dir exists).
pub fn claude_code_projects_dir(claude_home: &Path) -> PathBuf {
    claude_home.join("projects")
}

/// Claude Code names each project directory after its cwd with every path
/// separator and `.`/`_`/space collapsed to `-`. Mirror that so a migrated
/// thread lands in the same project the user would see for that folder.
fn claude_code_project_slug(cwd: &str) -> String {
    let trimmed = cwd.trim();
    if trimmed.is_empty() {
        return "codex-migrated".to_string();
    }
    let mut slug = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
        } else {
            slug.push('-');
        }
    }
    let collapsed = slug
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if collapsed.is_empty() {
        "codex-migrated".to_string()
    } else {
        collapsed
    }
}

fn render_universal_jsonl(messages: &[SessionMessage]) -> anyhow::Result<String> {
    let mut out = String::new();
    for message in messages {
        let line = json!({
            "role": message.role,
            "timestamp": message.timestamp,
            "text": message.body,
        });
        out.push_str(&serde_json::to_string(&line)?);
        out.push('\n');
    }
    Ok(out)
}

fn render_claude_code_jsonl(
    thread: &ResolvedCodexThread,
    thread_id: &str,
    messages: &[SessionMessage],
) -> anyhow::Result<String> {
    let mut out = String::new();
    let mut previous_uuid: Option<String> = None;
    for message in messages {
        let uuid = uuid::Uuid::new_v4().to_string();
        let line = json!({
            "type": message.role,
            "uuid": uuid,
            "parentUuid": previous_uuid,
            "timestamp": message.timestamp,
            "cwd": thread.cwd,
            "sessionId": thread_id,
            "message": {
                "role": message.role,
                "content": [{ "type": "text", "text": message.body }],
            },
        });
        out.push_str(&serde_json::to_string(&line)?);
        out.push('\n');
        previous_uuid = Some(uuid);
    }
    Ok(out)
}

fn render_markdown(title: &str, messages: &[SessionMessage]) -> String {
    let mut lines = vec![format!("# {title}"), String::new()];
    for message in messages {
        let speaker = if message.role == "user" {
            "User"
        } else {
            "Assistant"
        };
        lines.push(format!("### {speaker}"));
        if let Some(timestamp) = &message.timestamp {
            lines.push(format!("_{timestamp}_"));
        }
        lines.push(String::new());
        lines.push(message.body.trim_end().to_string());
        lines.push(String::new());
    }
    format!("{}\n", lines.join("\n").trim_end())
}

fn build_export_filename(title: &str, thread_id: &str, format: SessionExportFormat) -> String {
    let cleaned = title
        .chars()
        .map(|ch| {
            if matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || ch.is_control()
            {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>();
    let mut safe_title = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(80)
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string();
    if safe_title.is_empty() {
        safe_title = "Untitled session".to_string();
    }
    let safe_thread_id = thread_id.replace(
        |ch: char| matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'),
        "-",
    );
    format!(
        "{safe_title}-{}.{}",
        safe_thread_id.trim(),
        format.file_extension()
    )
}

fn normalize_session_id(session_id: &str) -> String {
    session_id
        .strip_prefix("local:")
        .unwrap_or(session_id)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_slug_matches_claude_code_convention() {
        assert_eq!(
            claude_code_project_slug("D:\\Project\\Claude-Codex-Pro-Tool"),
            "D-Project-Claude-Codex-Pro-Tool"
        );
        assert_eq!(
            claude_code_project_slug("/home/me/my project"),
            "home-me-my-project"
        );
        assert_eq!(claude_code_project_slug("   "), "codex-migrated");
    }

    #[test]
    fn universal_jsonl_has_one_object_per_message() {
        let messages = vec![
            SessionMessage {
                role: "user".to_string(),
                timestamp: Some("2025-01-01 10:00:00".to_string()),
                body: "hello".to_string(),
            },
            SessionMessage {
                role: "assistant".to_string(),
                timestamp: None,
                body: "hi there".to_string(),
            },
        ];
        let jsonl = render_universal_jsonl(&messages).unwrap();
        let lines = jsonl.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["role"], "user");
        assert_eq!(first["text"], "hello");
    }

    #[test]
    fn claude_code_jsonl_links_turns_with_parent_uuid() {
        let thread = ResolvedCodexThread {
            title: "Demo".to_string(),
            rollout_path: PathBuf::from("unused"),
            cwd: "/home/me/proj".to_string(),
        };
        let messages = vec![
            SessionMessage {
                role: "user".to_string(),
                timestamp: None,
                body: "first".to_string(),
            },
            SessionMessage {
                role: "assistant".to_string(),
                timestamp: None,
                body: "second".to_string(),
            },
        ];
        let jsonl = render_claude_code_jsonl(&thread, "t1", &messages).unwrap();
        let lines = jsonl.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first["type"], "user");
        assert_eq!(first["parentUuid"], serde_json::Value::Null);
        assert_eq!(first["message"]["content"][0]["text"], "first");
        // The second turn's parent must be the first turn's uuid.
        assert_eq!(second["parentUuid"], first["uuid"]);
        assert_eq!(second["sessionId"], "t1");
    }

    #[test]
    fn export_filename_sanitizes_and_uses_extension() {
        let name = build_export_filename(
            "My: session / title",
            "abc123",
            SessionExportFormat::ClaudeCodeJsonl,
        );
        assert!(name.ends_with("-abc123.jsonl"));
        assert!(!name.contains(':'));
        assert!(!name.contains('/'));
    }
}
