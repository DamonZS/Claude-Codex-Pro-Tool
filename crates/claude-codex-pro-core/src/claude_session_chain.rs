//! Detect Claude Code "continued from a previous conversation" chains.
//!
//! When a conversation runs out of context Claude Code starts a new session
//! file whose first user message is a continuation prompt. From the user's
//! point of view it is still one conversation, but on disk it is many sessions
//! with the same title and no parent pointer. This module groups them.
//!
//! A chain is: same project, same non-empty title, every later segment opens
//! with the continuation prompt, and each segment starts right where the
//! previous one ended.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use chrono::DateTime;
use serde_json::Value;

use crate::claude_sessions::ClaudeSession;

/// Opening words Claude Code writes into a continuation segment.
const CONTINUATION_PREFIX: &str = "this session is being continued from a previous conversation";
/// A segment may start at most this long after the previous one ended.
const MAX_GAP_MS: i64 = 30 * 60 * 1000;
/// Clock skew tolerated when a segment appears to start before the previous ended.
const MAX_OVERLAP_MS: i64 = 5 * 60 * 1000;
/// Only the head of a file is read to find its first timestamp and message.
const PROBE_MAX_LINES: usize = 80;

/// What the head of one session file says about where it sits in a chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentProbe {
    pub first_ms: Option<i64>,
    pub continues: bool,
}

pub fn is_continuation_prompt(text: &str) -> bool {
    text.trim_start()
        .to_ascii_lowercase()
        .starts_with(CONTINUATION_PREFIX)
}

fn first_user_text(record: &Value) -> Option<String> {
    let content = record.get("message")?.get("content")?;
    match content {
        Value::String(text) => Some(text.clone()),
        Value::Array(blocks) => blocks.iter().find_map(|block| {
            (block.get("type").and_then(Value::as_str) == Some("text"))
                .then(|| {
                    block
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .flatten()
        }),
        _ => None,
    }
}

/// Read the head of a `.jsonl` session file. Returns `None` when the file
/// cannot be read or is not JSONL, in which case the session is never chained.
pub fn probe_segment(path: &Path) -> Option<SegmentProbe> {
    if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
        return None;
    }
    let reader = BufReader::new(File::open(path).ok()?);
    let mut first_ms = None;
    let mut continues = None;
    for line in reader.lines().take(PROBE_MAX_LINES) {
        let Ok(line) = line else { continue };
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if first_ms.is_none() {
            first_ms = record
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
                .map(|time| time.timestamp_millis());
        }
        if continues.is_none() && record.get("type").and_then(Value::as_str) == Some("user") {
            if let Some(text) = first_user_text(&record) {
                continues = Some(is_continuation_prompt(&text));
            }
        }
        if first_ms.is_some() && continues.is_some() {
            break;
        }
    }
    Some(SegmentProbe {
        first_ms,
        continues: continues.unwrap_or(false),
    })
}

/// Facts the grouping needs about one session, independent of where they came from.
#[derive(Debug, Clone)]
pub struct ChainCandidate {
    pub project_key: String,
    pub title: String,
    pub end_ms: Option<i64>,
}

/// Group candidate indices into chains, oldest segment first. Only chains of
/// two or more segments are returned. `probe` is called only for candidates
/// that share a project and title with at least one other candidate.
pub fn build_chains<F>(candidates: &[ChainCandidate], mut probe: F) -> Vec<Vec<usize>>
where
    F: FnMut(usize) -> Option<SegmentProbe>,
{
    let mut groups: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.title.trim().is_empty() || candidate.end_ms.is_none() {
            continue;
        }
        groups
            .entry((candidate.project_key.as_str(), candidate.title.as_str()))
            .or_default()
            .push(index);
    }

    let mut chains = Vec::new();
    for (_, mut members) in groups {
        if members.len() < 2 {
            continue;
        }
        members.sort_by_key(|&index| candidates[index].end_ms);
        let mut current: Vec<usize> = Vec::new();
        for index in members {
            let Some(probe) = probe(index) else {
                // Unreadable segment: never chain it, and end the current run.
                if current.len() >= 2 {
                    chains.push(std::mem::take(&mut current));
                }
                current.clear();
                continue;
            };
            let extends = match (current.last(), probe.first_ms) {
                (Some(&previous), Some(first_ms)) if probe.continues => {
                    let previous_end = candidates[previous].end_ms.unwrap_or(i64::MIN);
                    let gap = first_ms - previous_end;
                    (-MAX_OVERLAP_MS..=MAX_GAP_MS).contains(&gap)
                }
                _ => false,
            };
            if !extends && current.len() >= 2 {
                chains.push(std::mem::take(&mut current));
            }
            if !extends {
                current.clear();
            }
            current.push(index);
        }
        if current.len() >= 2 {
            chains.push(current);
        }
    }
    chains
}

/// Group Claude sessions into continuation chains, oldest segment first.
/// `project_key` decides what counts as the same project.
pub fn claude_session_chains(
    sessions: &[ClaudeSession],
    project_key: &dyn Fn(&ClaudeSession) -> String,
) -> Vec<Vec<ClaudeSession>> {
    let candidates: Vec<ChainCandidate> = sessions
        .iter()
        .map(|session| {
            let source = Path::new(&session.source_path);
            let stem_matches =
                source.file_stem().and_then(|stem| stem.to_str()) == Some(session.id.as_str());
            ChainCandidate {
                project_key: project_key(session),
                title: session.title.clone(),
                // A file holding several sessions cannot be chained safely.
                end_ms: stem_matches.then_some(session.updated_at_ms).flatten(),
            }
        })
        .collect();
    build_chains(&candidates, |index| {
        probe_segment(Path::new(&sessions[index].source_path))
    })
    .into_iter()
    .map(|chain| {
        chain
            .into_iter()
            .map(|index| sessions[index].clone())
            .collect()
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const MIN: i64 = 60_000;

    fn candidate(project: &str, title: &str, end_min: Option<i64>) -> ChainCandidate {
        ChainCandidate {
            project_key: project.into(),
            title: title.into(),
            end_ms: end_min.map(|minutes| minutes * MIN),
        }
    }

    fn probe(first_min: Option<i64>, continues: bool) -> Option<SegmentProbe> {
        Some(SegmentProbe {
            first_ms: first_min.map(|minutes| minutes * MIN),
            continues,
        })
    }

    #[test]
    fn continuation_prompt_is_detected_case_insensitively() {
        assert!(is_continuation_prompt(
            "This session is being continued from a previous conversation that ran out of context."
        ));
        assert!(is_continuation_prompt(
            "  this session is being continued from a previous conversation"
        ));
        assert!(!is_continuation_prompt("请继续"));
        assert!(!is_continuation_prompt("hello"));
    }

    #[test]
    fn adjacent_continued_segments_form_one_chain_in_time_order() {
        // Deliberately shuffled; ends at 10, 20, 30; each starts where the last ended.
        let candidates = vec![
            candidate("p", "T", Some(30)),
            candidate("p", "T", Some(10)),
            candidate("p", "T", Some(20)),
        ];
        let probes = [
            probe(Some(21), true),
            probe(Some(0), false),
            probe(Some(11), true),
        ];
        let chains = build_chains(&candidates, |index| probes[index]);
        assert_eq!(chains, vec![vec![1, 2, 0]]);
    }

    #[test]
    fn different_title_or_project_never_chains() {
        let candidates = vec![
            candidate("p", "A", Some(10)),
            candidate("p", "B", Some(20)),
            candidate("q", "A", Some(30)),
        ];
        let chains = build_chains(&candidates, |_| probe(Some(0), true));
        assert!(chains.is_empty());
    }

    #[test]
    fn same_title_without_continuation_prompt_is_not_merged() {
        let candidates = vec![candidate("p", "T", Some(10)), candidate("p", "T", Some(20))];
        let probes = [probe(Some(0), false), probe(Some(11), false)];
        assert!(build_chains(&candidates, |index| probes[index]).is_empty());
    }

    #[test]
    fn a_long_gap_breaks_the_chain() {
        let candidates = vec![
            candidate("p", "T", Some(10)),
            candidate("p", "T", Some(20)),
            candidate("p", "T", Some(400)),
            candidate("p", "T", Some(410)),
        ];
        // Third segment starts 2 h after the second ended -> new chain.
        let probes = [
            probe(Some(0), false),
            probe(Some(11), true),
            probe(Some(300), true),
            probe(Some(401), true),
        ];
        let chains = build_chains(&candidates, |index| probes[index]);
        assert_eq!(chains, vec![vec![0, 1], vec![2, 3]]);
    }

    #[test]
    fn small_clock_overlap_is_tolerated_but_large_overlap_is_not() {
        let base = vec![candidate("p", "T", Some(10)), candidate("p", "T", Some(20))];
        let ok = build_chains(&base, |index| {
            [probe(Some(0), false), probe(Some(8), true)][index]
        });
        assert_eq!(ok, vec![vec![0, 1]]);
        let bad = build_chains(&base, |index| {
            [probe(Some(0), false), probe(Some(2), true)][index]
        });
        assert!(bad.is_empty());
    }

    #[test]
    fn unreadable_or_untimed_segments_are_never_chained() {
        let candidates = vec![
            candidate("p", "T", Some(10)),
            candidate("p", "T", Some(20)),
            candidate("p", "", Some(30)),
            candidate("p", "T", None),
        ];
        let none = build_chains(&candidates, |_| None);
        assert!(none.is_empty());
        let no_first_ts = build_chains(&candidates, |_| probe(None, true));
        assert!(no_first_ts.is_empty());
    }

    #[test]
    fn probe_is_not_called_for_lone_candidates() {
        let candidates = vec![candidate("p", "A", Some(10)), candidate("p", "B", Some(20))];
        let mut calls = 0;
        build_chains(&candidates, |_| {
            calls += 1;
            None
        });
        assert_eq!(calls, 0);
    }

    #[test]
    fn probe_segment_reads_first_timestamp_and_continuation_flag() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc.jsonl");
        let mut file = File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"type":"file-history-snapshot","snapshot":{{}}}}"#
        )
        .unwrap();
        writeln!(file, r#"{{"not json"#).unwrap();
        writeln!(
            file,
            r#"{{"type":"user","timestamp":"2026-10-03T15:06:00.000Z","message":{{"role":"user","content":"This session is being continued from a previous conversation that ran out of context."}}}}"#
        )
        .unwrap();
        let probed = probe_segment(&path).unwrap();
        assert!(probed.continues);
        let expected = DateTime::parse_from_rfc3339("2026-10-03T15:06:00.000Z")
            .unwrap()
            .timestamp_millis();
        assert_eq!(probed.first_ms, Some(expected));

        let plain = dir.path().join("plain.jsonl");
        let mut file = File::create(&plain).unwrap();
        writeln!(
            file,
            r#"{{"type":"user","timestamp":"2026-10-03T15:06:00.000Z","message":{{"role":"user","content":[{{"type":"text","text":"请帮我看看"}}]}}}}"#
        )
        .unwrap();
        assert!(!probe_segment(&plain).unwrap().continues);

        let not_jsonl = dir.path().join("x.json");
        File::create(&not_jsonl).unwrap();
        assert!(probe_segment(&not_jsonl).is_none());
        assert!(probe_segment(&dir.path().join("missing.jsonl")).is_none());
    }

    /// Read-only: lists the continuation chains found in the real Claude
    /// inventory. Prints ids truncated to 8 characters and never any content.
    #[test]
    #[ignore = "reads the real ~/.claude session inventory; run with --ignored --nocapture"]
    fn real_inventory_chain_smoke() {
        let inventory = crate::claude_sessions::list_claude_sessions().unwrap();
        // Approximates the manager's canonical project: a worktree folds into its repository.
        let project = |session: &ClaudeSession| {
            let cwd = session.cwd.replace('\\', "/");
            match cwd.find("/.claude/worktrees/") {
                Some(index) => cwd[..index].to_string(),
                None => cwd,
            }
        };
        let chains = claude_session_chains(&inventory.sessions, &project);
        println!(
            "sessions={} chains={}",
            inventory.sessions.len(),
            chains.len()
        );
        for chain in &chains {
            let ids: Vec<String> = chain
                .iter()
                .map(|s| s.id.chars().take(8).collect())
                .collect();
            println!(
                "len={} title={:?} ids={}",
                chain.len(),
                chain[0].title,
                ids.join(" -> ")
            );
        }
    }
}
