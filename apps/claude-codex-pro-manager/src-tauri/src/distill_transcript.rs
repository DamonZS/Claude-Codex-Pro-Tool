//! Distill-only transcript reader with AITracker's 2000-message window
//! (`transcript-reader.server.ts` MAX_MESSAGES). The 会话 page keeps using the
//! shared `aitracker_session_transcript`; the distillation drawer and pipeline
//! both use this reader so selected message indices always align.

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{AitrackerSessionTranscript, AitrackerTranscriptMessage};

pub const DISTILL_TRANSCRIPT_MESSAGE_CAP: usize = 2000;

pub fn aitracker_session_transcript_for_distill(
    agent: &str,
    private_session_id: &str,
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
                        .take(DISTILL_TRANSCRIPT_MESSAGE_CAP)
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
                .into_iter()
                .find(|session| matches(&session.id))?;
            // The context loader pages at most 200 messages per call; walk
            // pages from the start up to AITracker's 2000-message window.
            const PAGE: usize = 200;
            let mut title = String::new();
            let mut total_messages = 0;
            let mut messages = Vec::new();
            while messages.len() < DISTILL_TRANSCRIPT_MESSAGE_CAP {
                let page = claude_codex_pro_core::claude_sessions::load_claude_session_context(
                    &session.id,
                    Path::new(&session.source_path),
                    Some(messages.len()),
                    Some(PAGE),
                )
                .ok()?;
                title = page.title;
                total_messages = page.total_messages;
                let count = page.messages.len();
                messages.extend(page.messages.into_iter().map(|message| {
                    AitrackerTranscriptMessage {
                        role: message.role,
                        text: message.text,
                        timestamp: message.timestamp_ms.map(|value| value.to_string()),
                    }
                }));
                if count < PAGE || messages.len() >= total_messages {
                    break;
                }
            }
            messages.truncate(DISTILL_TRANSCRIPT_MESSAGE_CAP);
            Some(AitrackerSessionTranscript {
                title,
                total_messages,
                has_more_before: false,
                messages,
            })
        }
        "cursor" | "openclaw" | "workbuddy" => {
            jsonl_session_transcript_for_distill(agent, private_session_id)
        }
        _ => None,
    }
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
