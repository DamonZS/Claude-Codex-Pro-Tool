//! Pure distillation domain helpers, ported from AITracker
//! `src/modules/distillation/domain.ts` and `compression.ts`
//! (Copyright (C) 2026 AITracker contributors, used with permission).
//!
//! Lengths follow JavaScript semantics (UTF-16 code units) so budgets and
//! truncation points match AITracker, while never splitting a character.

use regex::Regex;
use std::sync::OnceLock;

pub const MAX_INPUT_CHARS: usize = 48_000;
pub const MAX_MESSAGE_CHARS: usize = 6_000;
pub const MAX_TITLE: usize = 120;
pub const MAX_SUMMARY: usize = 24_000;
pub const SEGMENT_SECTION: &str = "--- 用户选择片段 ---";
pub const COMPRESSED_MARKER: &str = "\n[…内容已压缩…]\n";

fn private_path() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?:/Users/[A-Za-z0-9._-]+|/home/[A-Za-z0-9._-]+|[A-Za-z]:(?:\\[^\s"'<>|\\]*)+|\\\\[A-Za-z0-9._-]+\\[^\s"'<>|\\]+)"#,
        )
        .expect("valid AITracker path regex")
    })
}

fn credential_value() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)(?:sk-[A-Za-z0-9_-]{8,}|pk-[A-Za-z0-9_-]{8,}|bearer\s+[A-Za-z0-9._~-]{12,}|(?:api[_-]?key|password|secret|token)\s*[:=]\s*[A-Za-z0-9._~/+=-]{8,})"#)
            .expect("valid AITracker credential regex")
    })
}

fn opaque_id() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$").expect("opaque regex"))
}

/// `OPAQUE.test(value)`.
pub fn is_opaque_id(value: &str) -> bool {
    opaque_id().is_match(value)
}

/// `sanitizeDistilledText`: private path roots -> `~`, credential values ->
/// `[REDACTED]`; everyday technical prose is kept.
pub fn sanitize_text(value: &str) -> String {
    let paths = private_path().replace_all(value, "~");
    credential_value()
        .replace_all(&paths, "[REDACTED]")
        .to_string()
}

pub fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

/// Longest prefix whose UTF-16 length is <= `units` (`slice(0, units)`).
pub fn utf16_prefix(value: &str, units: usize) -> &str {
    let mut used = 0;
    for (index, ch) in value.char_indices() {
        if used + ch.len_utf16() > units {
            return &value[..index];
        }
        used += ch.len_utf16();
    }
    value
}

/// Longest suffix whose UTF-16 length is <= `units` (`slice(-units)`).
fn utf16_suffix(value: &str, units: usize) -> &str {
    let mut used = 0;
    for (index, ch) in value.char_indices().rev() {
        if used + ch.len_utf16() > units {
            return &value[index + ch.len_utf8()..];
        }
        used += ch.len_utf16();
    }
    value
}

/// `safeText`: sanitize, trim, cap, and never return an empty string.
pub fn safe_text(value: &str, max_length: usize) -> String {
    let sanitized = sanitize_text(value);
    let capped = utf16_prefix(sanitized.trim(), max_length);
    if capped.is_empty() {
        "[REDACTED]".into()
    } else {
        capped.to_string()
    }
}

/// `ControlledSessionSummary`: renderer-safe metadata for one selected session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledRow {
    pub source: String,
    pub session_id: String,
    pub title: String,
    pub project_key: String,
    pub model: Option<String>,
    pub started_at: String,
    pub ended_at: String,
    pub turns: usize,
    pub edit_turns: usize,
    pub retry_turns: usize,
    pub subagent_calls: usize,
    pub status: String,
}

/// Port of `controlledContext`.
pub fn controlled_context(rows: &[ControlledRow]) -> String {
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            [
                format!("Session {}: {}:{}", index + 1, row.source, row.session_id),
                format!("Title: {}", row.title),
                format!("Project: {}", row.project_key),
                format!("Model: {}", row.model.as_deref().unwrap_or("unknown")),
                format!(
                    "Turns: {}; edits: {}; retries: {}; subagents: {}",
                    row.turns, row.edit_turns, row.retry_turns, row.subagent_calls
                ),
                format!(
                    "Status: {}; started: {}; ended: {}",
                    row.status, row.started_at, row.ended_at
                ),
            ]
            .join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentMessage {
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentMaterial {
    pub source: String,
    pub session_id: String,
    pub title: Option<String>,
    pub messages: Vec<SegmentMessage>,
}

/// Port of `extractSegmentMessages`: inclusive, clamped window; empty text
/// messages are dropped.
pub fn extract_segment_messages(
    messages: &[SegmentMessage],
    start_index: usize,
    end_index: usize,
) -> Vec<SegmentMessage> {
    if messages.is_empty() {
        return Vec::new();
    }
    let end = end_index.min(messages.len() - 1);
    if start_index > end {
        return Vec::new();
    }
    messages[start_index..=end]
        .iter()
        .filter(|message| !message.text.trim().is_empty())
        .cloned()
        .collect()
}

/// Port of `compactSegmentMaterials`.
pub fn compact_segment_materials(
    materials: &[SegmentMaterial],
    limit: usize,
) -> Vec<SegmentMaterial> {
    let mut remaining = limit.max(1_000) as i64;
    let mut result = Vec::new();
    for material in materials {
        if remaining <= 0 {
            break;
        }
        let mut messages = Vec::new();
        for message in &material.messages {
            if remaining <= 0 {
                break;
            }
            let raw = message.text.trim();
            if raw.is_empty() {
                continue;
            }
            let allowance = (MAX_MESSAGE_CHARS as i64).min(remaining) as usize;
            let text = if utf16_len(raw) <= allowance {
                raw.to_string()
            } else if allowance < 200 {
                utf16_prefix(raw, allowance).to_string()
            } else {
                let head = (allowance as f64 * 0.65).floor() as usize;
                let tail = (allowance as f64 * 0.3).floor() as usize;
                format!(
                    "{}{COMPRESSED_MARKER}{}",
                    utf16_prefix(raw, head),
                    utf16_suffix(raw, tail)
                )
            };
            remaining -= utf16_len(&text) as i64;
            messages.push(SegmentMessage {
                role: message.role.clone(),
                text,
            });
        }
        if !messages.is_empty() {
            result.push(SegmentMaterial {
                messages,
                ..material.clone()
            });
        }
    }
    result
}

/// Port of `segmentMarkdown`.
pub fn segment_markdown(materials: &[SegmentMaterial]) -> String {
    materials
        .iter()
        .map(|material| {
            let header = match &material.title {
                Some(title) if !title.is_empty() => {
                    format!("### {}:{} — {title}", material.source, material.session_id)
                }
                _ => format!("### {}:{}", material.source, material.session_id),
            };
            let body = material
                .messages
                .iter()
                .map(|message| format!("**{}**: {}", message.role, message.text))
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("{header}\n\n{body}")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The full model input: controlled context plus optional segment block.
pub fn distillation_input(rows: &[ControlledRow], materials: &[SegmentMaterial]) -> String {
    let context = controlled_context(rows);
    let compacted = compact_segment_materials(materials, MAX_INPUT_CHARS);
    if compacted.is_empty() {
        context
    } else {
        format!(
            "{context}\n\n{SEGMENT_SECTION}\n{}",
            segment_markdown(&compacted)
        )
    }
}

fn strip_redundant_heading(value: &str, kind: &str) -> String {
    static MEMORY: OnceLock<Regex> = OnceLock::new();
    static PERSONA: OnceLock<Regex> = OnceLock::new();
    let re = match kind {
        "memory" => MEMORY.get_or_init(|| Regex::new(r"\A#\s*任务记忆[ \t]*\n+").unwrap()),
        "persona" => PERSONA.get_or_init(|| Regex::new(r"\A#\s*用户画像[ \t]*\n+").unwrap()),
        _ => return value.to_string(),
    };
    re.replacen(value, 1, "").to_string()
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// Port of `candidateText`.
pub fn candidate_text(text: Option<&str>, row_count: usize, kind: &str) -> String {
    let text = text.map(str::trim).unwrap_or("");
    if text.is_empty() {
        let asset = match kind {
            "skill" => "skill package",
            "brief" => "workflow",
            "prompt" => "prompt template",
            "persona" => "persona memory",
            _ => "task memory",
        };
        return format!(
            "Distilled {asset} for {row_count} selected session{}.",
            plural(row_count)
        );
    }
    let sanitized = strip_redundant_heading(&sanitize_text(text), kind);
    let sanitized = sanitized.trim();
    if sanitized.is_empty() {
        let asset = match kind {
            "persona" => "persona memory",
            "memory" => "task memory",
            other => other,
        };
        return format!(
            "Distilled {asset} for {row_count} selected session{}.",
            plural(row_count)
        );
    }
    utf16_prefix(sanitized, MAX_SUMMARY).to_string()
}

/// Port of `candidateTitle`.
pub fn candidate_title(rows: &[ControlledRow], kind: &str) -> String {
    let mut project_keys: Vec<&str> = Vec::new();
    for row in rows {
        if !row.project_key.is_empty() && !project_keys.contains(&row.project_key.as_str()) {
            project_keys.push(&row.project_key);
        }
    }
    let lead = project_keys
        .first()
        .copied()
        .or_else(|| rows.first().map(|row| row.title.as_str()))
        .unwrap_or("Session");
    let suffix = if project_keys.len() > 1 {
        format!(" +{} projects", project_keys.len() - 1)
    } else if rows.len() > 1 {
        format!(" · {} sessions", rows.len())
    } else {
        String::new()
    };
    let label = match kind {
        "skill" => "Skill Package",
        "brief" => "Workflow",
        "prompt" => "Prompt Template",
        "persona" => "Persona Memory",
        _ => "Task Memory",
    };
    format!("{lead} {label}{suffix}")
}

/// First non-empty line, capped at 200 characters, for list summaries.
pub fn summary_line(output: &str) -> String {
    let line = output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    utf16_prefix(line, 200).to_string()
}

/// Port of `markDistilledManifest`.
pub fn mark_distilled_manifest(content: &str) -> String {
    static EXISTING: OnceLock<Regex> = OnceLock::new();
    static OPENING: OnceLock<Regex> = OnceLock::new();
    let marker = "aitracker-origin: distilled";
    let existing =
        EXISTING.get_or_init(|| Regex::new(r"(?im)^aitracker-origin:\s*[^\r\n]*$").unwrap());
    if existing.is_match(content) {
        return existing.replacen(content, 1, marker).to_string();
    }
    let opening = OPENING.get_or_init(|| Regex::new(r"\A---\r?\n").unwrap());
    if opening.is_match(content) {
        return opening
            .replacen(content, 1, format!("---\n{marker}\n").as_str())
            .to_string();
    }
    format!("---\n{marker}\n---\n\n{content}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(source: &str, id: &str, project: &str) -> ControlledRow {
        ControlledRow {
            source: source.into(),
            session_id: id.into(),
            title: project.into(),
            project_key: project.into(),
            model: None,
            started_at: "s".into(),
            ended_at: "e".into(),
            turns: 3,
            edit_turns: 0,
            retry_turns: 0,
            subagent_calls: 0,
            status: "completed".into(),
        }
    }

    fn message(role: &str, text: &str) -> SegmentMessage {
        SegmentMessage {
            role: role.into(),
            text: text.into(),
        }
    }

    #[test]
    fn distill_sanitize_preserves_technical_terms_and_redacts_private_values() {
        let value =
            sanitize_text("npm run check at C:\\Users\\Damon\\repo; token=secret-value-123456");
        assert!(value.contains("npm run check"));
        assert!(value.contains("at ~ "));
        assert!(value.contains("[REDACTED]"));
        assert!(!value.contains("secret-value-123456"));
        assert_eq!(sanitize_text("/Users/me/project"), "~/project");
        assert_eq!(safe_text("   ", 120), "[REDACTED]");
        assert_eq!(safe_text(&"a".repeat(200), 120).len(), 120);
    }

    #[test]
    fn distill_opaque_ids() {
        assert!(is_opaque_id("claude-code"));
        assert!(is_opaque_id("a1:b.c_d-e"));
        assert!(!is_opaque_id("-leading"));
        assert!(!is_opaque_id("has space"));
        assert!(!is_opaque_id("../x"));
        assert!(!is_opaque_id(&"a".repeat(129)));
        assert!(is_opaque_id(&"a".repeat(128)));
    }

    #[test]
    fn distill_controlled_context_matches_aitracker_format() {
        let mut first = row("codex", "s1", "ccp");
        first.model = Some("gpt-5".into());
        let rows = [first, row("claude-code", "s2", "web")];
        assert_eq!(
            controlled_context(&rows),
            "Session 1: codex:s1\nTitle: ccp\nProject: ccp\nModel: gpt-5\nTurns: 3; edits: 0; retries: 0; subagents: 0\nStatus: completed; started: s; ended: e\n\nSession 2: claude-code:s2\nTitle: web\nProject: web\nModel: unknown\nTurns: 3; edits: 0; retries: 0; subagents: 0\nStatus: completed; started: s; ended: e"
        );
    }

    #[test]
    fn distill_compression_keeps_head_tail_and_stops_at_budget() {
        let long = format!("{}{}", "h".repeat(5_000), "t".repeat(5_000));
        let materials = [SegmentMaterial {
            source: "codex".into(),
            session_id: "s".into(),
            title: None,
            messages: vec![message("user", &long), message("assistant", "  ")],
        }];
        let compacted = compact_segment_materials(&materials, MAX_INPUT_CHARS);
        let text = &compacted[0].messages[0].text;
        assert_eq!(compacted[0].messages.len(), 1, "blank messages dropped");
        assert!(text.starts_with(&"h".repeat(3_900)));
        assert!(text.contains(COMPRESSED_MARKER));
        assert!(text.ends_with(&"t".repeat(1_800)));
        assert_eq!(
            utf16_len(text),
            3_900 + COMPRESSED_MARKER.chars().count() + 1_800
        );

        // Budget: min 1000; small allowance (<200) is a plain prefix.
        let many = SegmentMaterial {
            source: "codex".into(),
            session_id: "s".into(),
            title: None,
            messages: (0..5).map(|_| message("user", &"x".repeat(450))).collect(),
        };
        let compacted = compact_segment_materials(&[many.clone(), many], 10);
        assert_eq!(
            compacted.len(),
            1,
            "second material is skipped once budget is spent"
        );
        let lengths = compacted[0]
            .messages
            .iter()
            .map(|message| message.text.len())
            .collect::<Vec<_>>();
        assert_eq!(lengths, vec![450, 450, 100]);
    }

    #[test]
    fn distill_segment_markdown_and_input_shape() {
        let materials = [SegmentMaterial {
            source: "codex".into(),
            session_id: "s1".into(),
            title: Some("ccp".into()),
            messages: vec![message("user", "hi"), message("assistant", "ok")],
        }];
        assert_eq!(
            segment_markdown(&materials),
            "### codex:s1 — ccp\n\n**user**: hi\n\n**assistant**: ok"
        );
        let rows = [row("codex", "s1", "ccp")];
        let input = distillation_input(&rows, &materials);
        assert!(input.starts_with("Session 1: codex:s1"));
        assert!(input.contains("\n\n--- 用户选择片段 ---\n### codex:s1 — ccp"));
        assert_eq!(distillation_input(&rows, &[]), controlled_context(&rows));
    }

    #[test]
    fn distill_extract_segment_clamps_window() {
        let messages = vec![
            message("user", "a"),
            message("assistant", " "),
            message("user", "c"),
        ];
        assert_eq!(extract_segment_messages(&messages, 0, 99).len(), 2);
        assert_eq!(extract_segment_messages(&messages, 2, 2)[0].text, "c");
        assert!(extract_segment_messages(&messages, 5, 9).is_empty());
        assert!(extract_segment_messages(&[], 0, 0).is_empty());
    }

    #[test]
    fn distill_candidate_title_and_text_rules() {
        let one = [row("codex", "s1", "ccp")];
        assert_eq!(candidate_title(&one, "skill"), "ccp Skill Package");
        let same = [row("codex", "s1", "ccp"), row("codex", "s2", "ccp")];
        assert_eq!(candidate_title(&same, "brief"), "ccp Workflow · 2 sessions");
        let multi = [
            row("codex", "s1", "ccp"),
            row("codex", "s2", "web"),
            row("codex", "s3", "api"),
        ];
        assert_eq!(
            candidate_title(&multi, "memory"),
            "ccp Task Memory +2 projects"
        );
        assert_eq!(candidate_title(&[], "persona"), "Session Persona Memory");

        assert_eq!(
            candidate_text(Some("# 任务记忆\n\n## 当前目标\nship"), 1, "memory"),
            "## 当前目标\nship"
        );
        assert_eq!(
            candidate_text(Some("# 用户画像 \n内容 at /home/bob/x"), 1, "persona"),
            "内容 at ~/x"
        );
        assert_eq!(
            candidate_text(None, 2, "prompt"),
            "Distilled prompt template for 2 selected sessions."
        );
        // Same as AITracker: input is trimmed first, so a heading-only body
        // has no trailing newline and the heading regex does not match.
        assert_eq!(
            candidate_text(Some("# 任务记忆\n"), 1, "memory"),
            "# 任务记忆"
        );
        assert_eq!(
            candidate_text(Some("   "), 1, "memory"),
            "Distilled task memory for 1 selected session."
        );
        assert_eq!(
            utf16_len(&candidate_text(Some(&"z".repeat(30_000)), 1, "skill")),
            MAX_SUMMARY
        );
        assert_eq!(summary_line("\n  ## Title  \nbody"), "## Title");
    }

    #[test]
    fn distill_mark_manifest_semantics() {
        assert_eq!(
            mark_distilled_manifest("---\nname: x\n---\n"),
            "---\naitracker-origin: distilled\nname: x\n---\n"
        );
        assert_eq!(
            mark_distilled_manifest("---\naitracker-origin: manual\n---"),
            "---\naitracker-origin: distilled\n---"
        );
        assert_eq!(
            mark_distilled_manifest("# body"),
            "---\naitracker-origin: distilled\n---\n\n# body"
        );
    }
}
