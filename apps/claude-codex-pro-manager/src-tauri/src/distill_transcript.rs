//! Distill-only transcript reader with AITracker's 2000-message window
//! (`transcript-reader.server.ts` MAX_MESSAGES). The 会话 page keeps using the
//! shared `aitracker_session_transcript`; the distillation drawer and pipeline
//! both use this reader so selected message indices always align.

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{AitrackerSessionTranscript, AitrackerTranscriptMessage};

pub const DISTILL_TRANSCRIPT_MESSAGE_CAP: usize = 2000;

/// Message window for whole-session distillation. The drawer keeps the
/// 2000-message window so picked indices line up; whole sessions and chains
/// may be far longer, and the pipeline then batches them to fit the model.
pub const WHOLE_SESSION_MESSAGE_CAP: usize = 20_000;

pub fn aitracker_session_transcript_for_distill(
    agent: &str,
    private_session_id: &str,
) -> Option<AitrackerSessionTranscript> {
    transcript_with_cap(agent, private_session_id, DISTILL_TRANSCRIPT_MESSAGE_CAP)
}

/// Same reader with a larger window, for distilling a whole session or chain.
pub fn whole_session_transcript_for_distill(
    agent: &str,
    private_session_id: &str,
) -> Option<AitrackerSessionTranscript> {
    transcript_with_cap(agent, private_session_id, WHOLE_SESSION_MESSAGE_CAP)
}

/// Private (anonymised) session key the usage snapshot uses for a Claude session.
pub fn claude_private_key(raw_id: &str) -> Option<String> {
    claude_codex_pro_data::local_usage::session_id_from_structured(
        "claude-code",
        Some(&Value::String(raw_id.to_string())),
    )
}

/// Continuation chains over the Claude session inventory, oldest segment first.
/// "Same project" uses the canonical project folder name, so worktrees of one
/// repository count as one project.
pub fn claude_continuation_chains(
    sessions: &[claude_codex_pro_core::claude_sessions::ClaudeSession],
) -> Vec<Vec<claude_codex_pro_core::claude_sessions::ClaudeSession>> {
    claude_codex_pro_core::claude_session_chain::claude_session_chains(sessions, &|session| {
        claude_codex_pro_data::local_usage::canonical_project_label(&session.cwd)
            .unwrap_or_else(|| session.cwd.clone())
    })
}

/// Usage-snapshot sessions with Claude continuation chains folded into one row
/// each. Workbench and pipeline both call this so a chain's representative id
/// is the same on both sides.
pub fn merged_sessions(
    summaries: &[claude_codex_pro_data::aitracker::AgentSessionSummary],
) -> Vec<claude_codex_pro_data::aitracker::MergedSession> {
    let chains: Vec<Vec<String>> = claude_codex_pro_core::claude_sessions::list_claude_sessions()
        .map(|inventory| claude_continuation_chains(&inventory.sessions))
        .unwrap_or_default()
        .into_iter()
        .map(|chain| {
            chain
                .iter()
                .filter_map(|session| claude_private_key(&session.id))
                .collect()
        })
        .collect();
    claude_codex_pro_data::aitracker::merge_session_chains(summaries, "claude-code", &chains)
}

fn transcript_with_cap(
    agent: &str,
    private_session_id: &str,
    cap: usize,
) -> Option<AitrackerSessionTranscript> {
    let matches = |raw_id: &str| {
        claude_codex_pro_data::local_usage::session_id_from_structured(
            agent,
            Some(&Value::String(raw_id.to_string())),
        )
        .as_deref()
            == Some(private_session_id)
    };
    match agent {
        "codex" => {
            for db_path in super::session_candidate_db_paths(None) {
                let Ok(sessions) = super::local_session_adapter(&db_path).list_local_sessions()
                else {
                    continue;
                };
                let Some(session) = sessions.into_iter().find(|session| matches(&session.id))
                else {
                    continue;
                };
                // The paged context loader caps at 200 messages; read the
                // rollout directly to expose AITracker's first 2000 messages.
                let Ok(Some(thread)) =
                    claude_codex_pro_data::resolve_codex_thread(&db_path, &session.id)
                else {
                    continue;
                };
                let Ok(all_messages) =
                    claude_codex_pro_data::load_session_messages(&thread.rollout_path)
                else {
                    continue;
                };
                let total_messages = all_messages.len();
                return Some(AitrackerSessionTranscript {
                    title: thread.title,
                    total_messages,
                    has_more_before: false,
                    messages: all_messages
                        .into_iter()
                        .take(cap)
                        .map(|message| AitrackerTranscriptMessage {
                            role: message.role,
                            text: message.body,
                            timestamp: message.timestamp,
                        })
                        .collect(),
                });
            }
            None
        }
        "claude-code" => {
            let inventory = claude_codex_pro_core::claude_sessions::list_claude_sessions().ok()?;
            let session = inventory
                .sessions
                .iter()
                .find(|session| matches(&session.id))?
                .clone();
            // Any segment of a continuation chain reads as the whole chain, so a
            // conversation that outgrew its context is distilled as one piece.
            let chain = claude_continuation_chains(&inventory.sessions)
                .into_iter()
                .find(|chain| chain.iter().any(|member| member.id == session.id))
                .unwrap_or_else(|| vec![session]);
            stitch_claude_chain(&chain, cap)
        }
        "cursor" | "openclaw" | "workbuddy" => {
            jsonl_session_transcript_for_distill(agent, private_session_id)
        }
        _ => None,
    }
}

/// Concatenate the segments of a chain in order. Later segments open with the
/// continuation prompt, which only repeats what the earlier segments already
/// contain, so it is dropped. Over the cap, keep the start of the conversation
/// and (mostly) its end, since the end is the most recent and least compressed.
fn stitch_claude_chain(
    chain: &[claude_codex_pro_core::claude_sessions::ClaudeSession],
    cap: usize,
) -> Option<AitrackerSessionTranscript> {
    let mut title = String::new();
    let mut messages: Vec<AitrackerTranscriptMessage> = Vec::new();
    for (position, session) in chain.iter().enumerate() {
        let Ok(page) = claude_codex_pro_core::claude_sessions::load_claude_session_messages(
            &session.id,
            Path::new(&session.source_path),
            cap,
        ) else {
            // One unreadable segment should not sink the whole chain, but a
            // chain of one has nothing to fall back on.
            if chain.len() == 1 {
                return None;
            }
            continue;
        };
        if title.is_empty() {
            title = page.title.clone();
        }
        let mut segment = page.messages.into_iter().peekable();
        if position > 0
            && segment.peek().is_some_and(|first| {
                first.role == "user"
                    && claude_codex_pro_core::claude_session_chain::is_continuation_prompt(
                        &first.text,
                    )
            })
        {
            segment.next();
        }
        messages.extend(segment.map(|message| AitrackerTranscriptMessage {
            role: message.role,
            text: message.text,
            timestamp: message.timestamp_ms.map(|value| value.to_string()),
        }));
    }
    if messages.is_empty() && chain.len() == 1 {
        return None;
    }
    let total_messages = messages.len();
    if total_messages > cap {
        let head = cap / 5;
        let tail = cap - head;
        let tail_start = total_messages - tail;
        let mut kept = Vec::with_capacity(cap);
        kept.extend(messages.drain(tail_start..));
        kept.splice(0..0, messages.drain(..head));
        messages = kept;
    }
    Some(AitrackerSessionTranscript {
        title,
        total_messages,
        has_more_before: false,
        messages,
    })
}

/// Read transcript-bearing JSONL files for local clients whose usage adapter
/// already exposes session ids but whose message format is not represented by
/// the Codex/Claude loaders above. The lookup matches both privacy-safe file
/// ids and structured session ids used by the scanner.
fn jsonl_session_transcript_for_distill(
    agent: &str,
    private_session_id: &str,
) -> Option<AitrackerSessionTranscript> {
    let claude_home = super::claude_code_home_dir();
    let home = claude_home.parent().unwrap_or(&claude_home);
    let root = match agent {
        "cursor" => home.join(".cursor"),
        "openclaw" => home.join(".openclaw").join("agents"),
        "workbuddy" => home.join(".workbuddy").join("projects"),
        _ => return None,
    };
    let mut files = Vec::new();
    super::collect_jsonl_files(&root, &mut files, 512);
    for path in files {
        let file_identity = claude_codex_pro_data::local_usage::session_id_from_relative_file(
            agent,
            &path.to_string_lossy(),
        );
        let bytes = match fs::read(&path) {
            Ok(bytes) if bytes.len() <= 24 * 1024 * 1024 => bytes,
            _ => continue,
        };
        let mut messages = Vec::new();
        let mut matched_structured = file_identity == private_session_id;
        let title = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Session")
            .to_string();
        for line in bytes.split(|byte| *byte == b'\n') {
            let Ok(value) = serde_json::from_slice::<Value>(line) else {
                continue;
            };
            if !matched_structured {
                let raw_id = [
                    value.get("sessionId"),
                    value.get("session_id"),
                    value.get("conversationId"),
                    value.get("conversation_id"),
                    value.pointer("/session/id"),
                    value.pointer("/conversation/id"),
                ]
                .into_iter()
                .flatten()
                .find_map(Value::as_str);
                matched_structured = raw_id
                    .and_then(|id| {
                        claude_codex_pro_data::local_usage::session_id_from_structured(
                            agent,
                            Some(&Value::String(id.to_string())),
                        )
                    })
                    .is_some_and(|id| id == private_session_id);
            }
            if let Some(candidate) = super::aitracker_jsonl_message(&value) {
                if !candidate.text.trim().is_empty() {
                    messages.push(candidate);
                }
            }
            if messages.len() >= DISTILL_TRANSCRIPT_MESSAGE_CAP {
                break;
            }
        }
        if matched_structured && !messages.is_empty() {
            let total_messages = messages.len();
            return Some(AitrackerSessionTranscript {
                title,
                messages,
                total_messages,
                has_more_before: false,
            });
        }
    }
    None
}
