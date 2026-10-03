//! Input budget and batching for distillation.
//!
//! AITracker hard-codes a 48k-character input cap. CCP lets the configured
//! provider's `contextWindow` decide instead, and splits oversized material
//! into ordered batches that are distilled one at a time and then merged.
//! This deliberately departs from AITracker's fixed 48k/6k behaviour.

use std::time::Duration;

use super::aitracker_distillation::{MAX_INPUT_CHARS, SegmentMaterial, SegmentMessage};

/// Fraction of the context window one call may use (the rest is left for the
/// system prompt, the model's output, and token-estimate error).
const WINDOW_SHARE: f64 = 0.60;
/// Conservative characters-per-token for mixed Chinese/English text.
const CHARS_PER_TOKEN: f64 = 2.0;
/// Window assumed when the provider has no usable `contextWindow`.
const UNKNOWN_WINDOW_FALLBACK: u64 = 0;
/// A merge call must still fit: each partial note is capped to this many chars.
const MIN_NOTE_CHARS: usize = 2_000;
/// Hard ceiling so a mis-typed window cannot produce an absurd request.
const MAX_WINDOW_TOKENS: u64 = 2_000_000;

/// Parse `1000000`, `200k`, `1m`, `1M tokens`, `1,000,000`. Returns `None` for
/// empty or unparseable text.
pub fn parse_context_window(raw: &str) -> Option<u64> {
    let cleaned: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != ',' && *ch != '_')
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    let (digits, multiplier) = if let Some(rest) = cleaned.strip_suffix("tokens") {
        return parse_context_window(rest);
    } else if let Some(rest) = cleaned.strip_suffix('m') {
        (rest, 1_000_000.0)
    } else if let Some(rest) = cleaned.strip_suffix('k') {
        (rest, 1_000.0)
    } else {
        (cleaned.as_str(), 1.0)
    };
    let value: f64 = digits.parse().ok()?;
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    Some(((value * multiplier) as u64).min(MAX_WINDOW_TOKENS))
}

/// Input character budget for one model call. Falls back to AITracker's 48k
/// when the window is unknown, and never goes below it.
pub fn input_char_budget(context_window: &str) -> usize {
    let window = parse_context_window(context_window).unwrap_or(UNKNOWN_WINDOW_FALLBACK);
    if window == 0 {
        return MAX_INPUT_CHARS;
    }
    let budget = (window as f64 * WINDOW_SHARE * CHARS_PER_TOKEN) as usize;
    budget.max(MAX_INPUT_CHARS)
}

/// Request timeout that grows with the input: 120 s base plus 1 s per 8k
/// characters, capped at 10 minutes. A 1M-window request cannot finish in the
/// fixed 120 s used for small inputs.
pub fn timeout_for_input(input_chars: usize) -> Duration {
    let extra = (input_chars / 8_000) as u64;
    Duration::from_secs((120 + extra).min(600))
}

fn message_len(message: &SegmentMessage) -> usize {
    message.text.chars().count() + message.role.chars().count() + 8
}

/// Split materials into ordered batches of at most `budget` characters. A
/// material that spans batches is repeated with its header so each batch is
/// self-describing. Batch order follows the input order.
pub fn plan_batches(materials: &[SegmentMaterial], budget: usize) -> Vec<Vec<SegmentMaterial>> {
    let budget = budget.max(MAX_INPUT_CHARS);
    let mut batches: Vec<Vec<SegmentMaterial>> = Vec::new();
    let mut current: Vec<SegmentMaterial> = Vec::new();
    let mut used = 0usize;

    for material in materials {
        let header = material.source.chars().count() + material.session_id.chars().count() + 24;
        let mut piece: Vec<SegmentMessage> = Vec::new();
        let mut piece_len = 0usize;
        let flush_piece = |piece: &mut Vec<SegmentMessage>,
                           piece_len: &mut usize,
                           current: &mut Vec<SegmentMaterial>,
                           used: &mut usize| {
            if piece.is_empty() {
                return;
            }
            current.push(SegmentMaterial {
                messages: std::mem::take(piece),
                ..material.clone()
            });
            *used += *piece_len + header;
            *piece_len = 0;
        };

        for message in &material.messages {
            let len = message_len(message);
            if used + piece_len + header + len > budget
                && (!piece.is_empty() || !current.is_empty())
            {
                flush_piece(&mut piece, &mut piece_len, &mut current, &mut used);
                if !current.is_empty() {
                    batches.push(std::mem::take(&mut current));
                    used = 0;
                }
            }
            piece.push(message.clone());
            piece_len += len;
        }
        flush_piece(&mut piece, &mut piece_len, &mut current, &mut used);
    }
    if !current.is_empty() {
        batches.push(current);
    }
    batches
}

/// Cap each partial note so all of them fit in one merge call.
pub fn cap_notes(notes: &[String], budget: usize) -> Vec<String> {
    if notes.is_empty() {
        return Vec::new();
    }
    let per_note = (budget.max(MAX_INPUT_CHARS) / notes.len()).max(MIN_NOTE_CHARS);
    notes
        .iter()
        .map(|note| {
            let note = note.trim();
            if note.chars().count() <= per_note {
                note.to_string()
            } else {
                let head: String = note.chars().take(per_note * 2 / 3).collect();
                let tail: String = note
                    .chars()
                    .rev()
                    .take(per_note / 3)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                format!("{head}\n[…内容已压缩…]\n{tail}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(session: &str, count: usize, len: usize) -> SegmentMaterial {
        SegmentMaterial {
            source: "claude-code".into(),
            session_id: session.into(),
            title: Some("t".into()),
            messages: (0..count)
                .map(|i| SegmentMessage {
                    role: if i % 2 == 0 { "user" } else { "assistant" }.into(),
                    text: "x".repeat(len),
                })
                .collect(),
        }
    }

    #[test]
    fn parses_common_context_window_spellings() {
        assert_eq!(parse_context_window("1000000"), Some(1_000_000));
        assert_eq!(parse_context_window("1,000,000"), Some(1_000_000));
        assert_eq!(parse_context_window("200k"), Some(200_000));
        assert_eq!(parse_context_window("1M"), Some(1_000_000));
        assert_eq!(parse_context_window(" 1m tokens "), Some(1_000_000));
        assert_eq!(parse_context_window("1.5m"), Some(1_500_000));
        assert_eq!(parse_context_window(""), None);
        assert_eq!(parse_context_window("abc"), None);
        assert_eq!(parse_context_window("0"), None);
        assert_eq!(parse_context_window("-5k"), None);
        assert_eq!(parse_context_window("999999m"), Some(MAX_WINDOW_TOKENS));
    }

    #[test]
    fn budget_follows_window_and_never_drops_below_default() {
        assert_eq!(input_char_budget(""), MAX_INPUT_CHARS);
        assert_eq!(input_char_budget("nonsense"), MAX_INPUT_CHARS);
        assert_eq!(input_char_budget("8k"), MAX_INPUT_CHARS);
        // 1M tokens * 60% * 2 chars/token
        assert_eq!(input_char_budget("1000000"), 1_200_000);
        assert_eq!(input_char_budget("200k"), 240_000);
    }

    #[test]
    fn timeout_grows_with_input_and_is_capped() {
        assert_eq!(timeout_for_input(1_000), Duration::from_secs(120));
        assert_eq!(timeout_for_input(800_000), Duration::from_secs(220));
        assert_eq!(timeout_for_input(100_000_000), Duration::from_secs(600));
    }

    #[test]
    fn small_material_is_one_batch() {
        let batches = plan_batches(&[material("a", 10, 100)], 48_000);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0][0].messages.len(), 10);
    }

    #[test]
    fn large_material_splits_in_order_without_losing_messages() {
        // 200 messages of ~1000 chars each = ~200k chars against a 48k budget.
        let source = material("big", 200, 1000);
        let batches = plan_batches(std::slice::from_ref(&source), 48_000);
        assert!(batches.len() >= 4, "got {} batches", batches.len());
        let rebuilt: Vec<&String> = batches
            .iter()
            .flatten()
            .flat_map(|m| m.messages.iter().map(|msg| &msg.text))
            .collect();
        assert_eq!(rebuilt.len(), 200);
        // Every batch stays within budget and keeps its header fields.
        for batch in &batches {
            let size: usize = batch
                .iter()
                .flat_map(|m| &m.messages)
                .map(message_len)
                .sum();
            assert!(size <= 48_000, "batch too large: {size}");
            assert!(batch.iter().all(|m| m.session_id == "big"));
        }
    }

    #[test]
    fn multiple_materials_keep_their_order() {
        let batches = plan_batches(
            &[material("first", 40, 1000), material("second", 40, 1000)],
            48_000,
        );
        let order: Vec<&str> = batches
            .iter()
            .flatten()
            .map(|m| m.session_id.as_str())
            .collect();
        let first_second = order.iter().position(|id| *id == "second").unwrap();
        assert!(order[..first_second].iter().all(|id| *id == "first"));
        assert!(order[first_second..].iter().all(|id| *id == "second"));
    }

    #[test]
    fn empty_input_has_no_batches() {
        assert!(plan_batches(&[], 48_000).is_empty());
    }

    #[test]
    fn notes_are_capped_to_fit_one_merge_call() {
        let notes = vec!["a".repeat(100_000); 10];
        let capped = cap_notes(&notes, 48_000);
        assert_eq!(capped.len(), 10);
        assert!(capped.iter().all(|note| note.chars().count() < 10_000));
        let short = cap_notes(&["ok".to_string()], 48_000);
        assert_eq!(short, vec!["ok".to_string()]);
    }
}
