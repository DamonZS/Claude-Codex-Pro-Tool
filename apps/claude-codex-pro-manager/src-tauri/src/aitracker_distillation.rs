//! CCP-native runtime port of the AITRACKER distillation primitives.
//!
//! The workbench UI follows AITRACKER's distillation flow, while this module
//! contains only the small server-side primitives that CCP executes locally.
//! No copy of the upstream source tree is required at runtime.

use regex::Regex;
use std::sync::OnceLock;

const MAX_INPUT_CHARS: usize = 48_000;
const MAX_MESSAGE_CHARS: usize = 6_000;

static PRIVATE_PATH: OnceLock<Regex> = OnceLock::new();
static CREDENTIAL_VALUE: OnceLock<Regex> = OnceLock::new();

fn private_path() -> &'static Regex {
    PRIVATE_PATH.get_or_init(|| {
        Regex::new(r#"(?:[A-Za-z]:\\[^\s\"'<>|]+|/Users/[A-Za-z0-9._-]+[^\s]*|/home/[A-Za-z0-9._-]+[^\s]*)"#).expect("valid AITRACKER path regex")
    })
}

fn credential_value() -> &'static Regex {
    CREDENTIAL_VALUE.get_or_init(|| {
        Regex::new(r#"(?i)(?:sk-[A-Za-z0-9_-]{8,}|pk-[A-Za-z0-9_-]{8,}|bearer\s+[A-Za-z0-9._~-]{12,}|(?:api[_-]?key|password|secret|token)\s*[:=]\s*[A-Za-z0-9._~/+=-]{8,})"#).expect("valid AITRACKER credential regex")
    })
}

/// Mirrors AITRACKER's redaction policy: redact private roots and credential
/// values while retaining ordinary technical prose.
pub fn sanitize_text(value: &str) -> String {
    let paths = private_path().replace_all(value, "~");
    credential_value()
        .replace_all(&paths, "[REDACTED]")
        .to_string()
}

/// Mirrors `compactSegmentMaterials`: preserve message boundaries and both
/// ends of oversized messages before the model receives the selected material.
pub fn compact_messages(messages: &[(String, String, String)]) -> String {
    let mut remaining = MAX_INPUT_CHARS;
    let mut output = Vec::new();
    for (session, role, text) in messages {
        if remaining == 0 {
            break;
        }
        let header = format!("### {session}\n**{role}**:");
        let allowance = MAX_MESSAGE_CHARS.min(remaining.saturating_sub(header.chars().count() + 2));
        if allowance == 0 {
            break;
        }
        let chars = text.trim().chars().collect::<Vec<_>>();
        if chars.is_empty() {
            continue;
        }
        let content = if chars.len() <= allowance {
            chars.iter().collect::<String>()
        } else if allowance < 200 {
            chars.iter().take(allowance).collect::<String>()
        } else {
            let head = (allowance * 65 / 100).max(1);
            let tail = (allowance * 30 / 100).max(1);
            format!(
                "{}\n[…内容已压缩…]\n{}",
                chars.iter().take(head).collect::<String>(),
                chars
                    .iter()
                    .skip(chars.len().saturating_sub(tail))
                    .collect::<String>()
            )
        };
        remaining = remaining.saturating_sub(header.chars().count() + content.chars().count() + 2);
        output.push(format!("{header}\n{content}"));
    }
    output.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_technical_terms_and_redacts_private_values() {
        let text = "npm run check at C:\\Users\\Damon\\repo; token=secret-value-123456";
        let value = sanitize_text(text);
        assert!(value.contains("npm run check"));
        assert!(value.contains("~"));
        assert!(value.contains("[REDACTED]"));
        assert!(!value.contains("secret-value-123456"));
    }

    #[test]
    fn compacts_without_crossing_message_boundaries() {
        let value = compact_messages(&[("s".into(), "user".into(), "x".repeat(7_000))]);
        assert!(value.starts_with("### s\n**user**:"));
        assert!(value.contains("[…内容已压缩…]"));
        assert!(value.contains(&"x".repeat(100)));
    }
}
