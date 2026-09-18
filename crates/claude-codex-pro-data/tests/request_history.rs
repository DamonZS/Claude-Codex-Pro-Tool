use claude_codex_pro_data::read_recent_local_requests;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn jsonl(path: &Path, values: &[Value]) {
    fs::write(
        path,
        values.iter().map(|v| format!("{v}\n")).collect::<String>(),
    )
    .unwrap();
}

fn database(root: &Path, rows: &[(i64, &Path)], milliseconds: bool) -> PathBuf {
    let path = root.join(if milliseconds {
        "modern.db"
    } else {
        "legacy.sqlite"
    });
    let db = Connection::open(&path).unwrap();
    let column = if milliseconds {
        "updated_at_ms"
    } else {
        "updated_at"
    };
    db.execute(
        &format!("CREATE TABLE threads (rollout_path TEXT, {column} INTEGER)"),
        [],
    )
    .unwrap();
    for (time, rollout) in rows {
        db.execute(
            &format!("INSERT INTO threads VALUES (?1, ?2)"),
            (rollout.to_string_lossy(), time),
        )
        .unwrap();
    }
    path
}

fn codex(time: &str, last: Value, total: Value) -> Value {
    json!({"type":"event_msg","timestamp":time,"payload":{"type":"token_count","info":{
        "last_token_usage":last,"total_token_usage":total
    }}})
}

fn claude(id: &str, time: &str, usage: Value) -> Value {
    json!({"type":"assistant","timestamp":time,"message":{
        "id":id,"model":"claude-fixture","usage":usage,
        "content":[{"type":"text","text":"PRIVATE_BODY_WITH_TOKEN"}]
    }})
}

#[test]
fn codex_reads_real_rollout_paths_and_deduplicates_cumulative_notifications() {
    let root = tempdir().unwrap();
    let path = root.path().join("arbitrary-name.jsonl");
    let usage = json!({"input_tokens":100,"output_tokens":20,"cached_input_tokens":80,
        "reasoning_output_tokens":7,"total_tokens":120});
    jsonl(
        &path,
        &[
            json!({"type":"session_meta","payload":{"model_provider":"historical-provider","cwd":"PRIVATE_PATH"}}),
            json!({"type":"turn_context","payload":{"model":"codex-fixture"}}),
            codex(
                "2026-09-18T01:02:03Z",
                usage.clone(),
                json!({"total_tokens":120}),
            ),
            codex(
                "2026-09-18T01:02:04Z",
                usage.clone(),
                json!({"total_tokens":120}),
            ),
            codex("2026-09-18T01:02:05Z", usage, json!({"total_tokens":240})),
            codex(
                "2026-09-18T01:02:06Z",
                Value::Null,
                json!({"total_tokens":99999}),
            ),
            json!({"type":"event_msg","timestamp":"2026-09-18T01:02:07Z","payload":{"type":"task_complete","duration_ms":1234}}),
        ],
    );
    let db = database(root.path(), &[(10, &path)], false);
    let before = fs::read(&db).unwrap();
    let (records, warnings) =
        read_recent_local_requests(&[db.clone(), db.clone()], &root.path().join("absent"), 200);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].timestamp_ms, 1_789_693_325_000);
    for record in &records {
        assert_eq!(record.input_tokens, Some(100));
        assert_eq!(record.output_tokens, Some(20));
        assert_eq!(record.cached_tokens, Some(80));
        assert_eq!(record.reasoning_tokens, Some(7));
        assert_eq!(record.total_tokens, Some(120));
        assert_eq!(record.provider.as_deref(), Some("historical-provider"));
        assert_eq!(record.model.as_deref(), Some("codex-fixture"));
        assert_eq!(record.status, "observed");
        assert_eq!(record.source, "codex_rollout");
        assert_eq!(record.agent, "codex");
        assert_eq!(record.duration_ms, None);
        assert_eq!(record.first_byte_ms, None);
        assert_eq!(record.http_status, None);
        assert_eq!(record.protocol, None);
    }
    assert_eq!(warnings.len(), 1);
    assert_eq!(fs::read(db).unwrap(), before);
    let exported = serde_json::to_string(&records).unwrap();
    assert!(!exported.contains("PRIVATE_PATH"));
    assert!(!exported.contains("arbitrary-name"));
}

#[test]
fn claude_merges_fragments_without_summing_cached_or_created_tokens() {
    let root = tempdir().unwrap();
    let project = root.path().join("project");
    fs::create_dir(&project).unwrap();
    jsonl(
        &project.join("chat.jsonl"),
        &[
            claude(
                "m1",
                "2026-09-18T01:00:00Z",
                json!({"input_tokens":10,"output_tokens":2,"cache_read_input_tokens":40,"cache_creation_input_tokens":50}),
            ),
            claude(
                "m1",
                "2026-09-18T01:00:01Z",
                json!({"input_tokens":10,"output_tokens":9,"cache_read_input_tokens":40,"cache_creation_input_tokens":50}),
            ),
            claude("m2", "2026-09-18T01:00:02Z", json!({"output_tokens":0})),
        ],
    );
    let (records, _) = read_recent_local_requests(&[], root.path(), 200);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].output_tokens, Some(0));
    assert_eq!(records[0].input_tokens, None);
    let item = &records[1];
    assert_eq!(item.input_tokens, Some(100));
    assert_eq!(item.output_tokens, Some(9));
    assert_eq!(item.cached_tokens, Some(40));
    assert_eq!(item.cache_creation_tokens, Some(50));
    assert_eq!(item.total_tokens, Some(109));
    assert_eq!(item.provider, None);
    assert_eq!(item.model.as_deref(), Some("claude-fixture"));
    assert_eq!(item.source, "claude_code");
    assert_eq!(item.agent, "claude");
    assert_eq!(item.status, "observed");
    let exported = serde_json::to_string(&records).unwrap();
    assert!(!exported.contains("PRIVATE_BODY_WITH_TOKEN"));
    assert!(!exported.contains("content"));
}

#[test]
fn damaged_lines_and_missing_fields_do_not_invent_metrics() {
    let root = tempdir().unwrap();
    let path = root.path().join("codex.jsonl");
    let good = codex(
        "2026-09-18T01:00:00Z",
        json!({"output_tokens":5}),
        Value::Null,
    );
    let same_tokens_new_request = codex(
        "2026-09-18T01:00:01Z",
        json!({"output_tokens":5}),
        Value::Null,
    );
    fs::write(
        &path,
        format!("invalid\n{good}\n{good}\n{same_tokens_new_request}\n{{\"type\":"),
    )
    .unwrap();
    let db = database(root.path(), &[(10, &path)], true);
    let (records, warnings) = read_recent_local_requests(&[db], &root.path().join("absent"), 200);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].input_tokens, None);
    assert_eq!(records[0].total_tokens, None);
    assert_eq!(records[0].model, None);
    assert_eq!(records[0].provider, None);
    assert!(warnings.iter().any(|s| s.contains("格式异常")));
    assert!(
        warnings
            .iter()
            .all(|s| !s.contains(&root.path().to_string_lossy().to_string()))
    );
}

#[test]
fn missing_sqlite_is_not_created_and_errors_are_static() {
    let root = tempdir().unwrap();
    let missing = root.path().join("private-user-database.db");
    let broken = root.path().join("broken.db");
    fs::write(&broken, "not sqlite").unwrap();
    let (records, warnings) =
        read_recent_local_requests(&[missing.clone(), broken], &root.path().join("absent"), 200);
    assert!(records.is_empty());
    assert!(!missing.exists());
    assert!(warnings.iter().any(|s| s.contains("读取失败")));
    assert!(
        warnings
            .iter()
            .all(|s| !s.contains("private-user") && !s.contains("broken.db"))
    );
}

#[test]
fn tail_limit_preserves_complete_records_and_header_provider_but_not_old_model() {
    let root = tempdir().unwrap();
    let path = root.path().join("large.jsonl");
    let meta = json!({"type":"session_meta","payload":{"model_provider":"old-provider"}});
    let context = json!({"type":"turn_context","payload":{"model":"old-model"}});
    let padding = "x".repeat(600 * 1024);
    let usage = codex(
        "2026-09-18T01:00:00Z",
        json!({"input_tokens":3}),
        Value::Null,
    );
    fs::write(
        &path,
        format!("{meta}\n{context}\n{{\"body\":\"{padding}\"}}\n{usage}\n"),
    )
    .unwrap();
    let db = database(root.path(), &[(10, &path)], false);
    let (records, warnings) = read_recent_local_requests(&[db], &root.path().join("absent"), 200);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].provider.as_deref(), Some("old-provider"));
    assert_eq!(records[0].model, None);
    assert!(warnings.iter().any(|s| s.contains("已达到")));
    assert!(!warnings.iter().any(|s| s.contains("格式异常")));
}

#[test]
fn newest_threads_and_record_limits_are_enforced() {
    let root = tempdir().unwrap();
    let paths = (0..40)
        .map(|i| root.path().join(format!("{i}.jsonl")))
        .collect::<Vec<_>>();
    for (i, path) in paths.iter().enumerate() {
        jsonl(
            path,
            &[codex(
                &format!("2026-09-18T01:00:{i:02}Z"),
                json!({"output_tokens":i}),
                Value::Null,
            )],
        );
    }
    let rows = paths
        .iter()
        .enumerate()
        .map(|(i, p)| (i as i64, p.as_path()))
        .collect::<Vec<_>>();
    let db = database(root.path(), &rows, true);
    let absent = root.path().join("absent");
    let (records, warnings) = read_recent_local_requests(std::slice::from_ref(&db), &absent, 200);
    assert_eq!(records.len(), 32);
    assert_eq!(records[0].output_tokens, Some(39));
    assert_eq!(records.last().unwrap().output_tokens, Some(8));
    assert!(warnings.iter().any(|s| s.contains("已达到")));
    assert_eq!(
        read_recent_local_requests(std::slice::from_ref(&db), &absent, 2)
            .0
            .len(),
        2
    );
    assert!(read_recent_local_requests(&[db], &absent, 0).0.is_empty());
}

#[test]
fn claude_subagent_and_copied_messages_are_deduplicated() {
    let root = tempdir().unwrap();
    let sub = root.path().join("project/session/subagents");
    fs::create_dir_all(&sub).unwrap();
    let item = claude(
        "same-id",
        "2026-09-18T01:00:00Z",
        json!({"output_tokens":5}),
    );
    jsonl(
        &root.path().join("project/main.jsonl"),
        std::slice::from_ref(&item),
    );
    let mut partial = item;
    partial["message"]["usage"] = json!({"output_tokens":1,"cache_read_input_tokens":10});
    jsonl(&sub.join("agent.jsonl"), &[partial]);
    let (records, _) = read_recent_local_requests(&[], root.path(), 200);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].output_tokens, Some(5));
    assert_eq!(records[0].cached_tokens, Some(10));
}

#[test]
fn exact_tail_boundary_retains_the_first_complete_line() {
    let root = tempdir().unwrap();
    let item = claude(
        "boundary",
        "2026-09-18T01:00:00Z",
        json!({"output_tokens":7}),
    );
    let mut tail = format!("{item}\n");
    tail.push_str(&"\n".repeat(512 * 1024 - tail.len()));
    fs::write(root.path().join("boundary.jsonl"), format!("{{}}\n{tail}")).unwrap();
    let (records, warnings) = read_recent_local_requests(&[], root.path(), 200);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].output_tokens, Some(7));
    assert!(warnings.iter().any(|s| s.contains("已达到")));
}

#[test]
fn final_cap_is_500_and_invalid_timestamps_are_not_guessed() {
    let root = tempdir().unwrap();
    let mut items = (0..550)
        .map(|i| {
            claude(
                &format!("m{i}"),
                "2026-09-18T01:00:00Z",
                json!({"output_tokens":1}),
            )
        })
        .collect::<Vec<_>>();
    items.push(claude("bad-time", "invalid", json!({"output_tokens":999})));
    jsonl(&root.path().join("chat.jsonl"), &items);
    let (records, warnings) = read_recent_local_requests(&[], root.path(), usize::MAX);
    assert_eq!(records.len(), 500);
    assert!(records.iter().all(|r| r.output_tokens == Some(1)));
    assert!(warnings.iter().any(|s| s.contains("已达到")));
}

#[test]
fn claude_normalization_requires_all_input_components_and_checks_overflow() {
    let root = tempdir().unwrap();
    let usages = [
        json!({"input_tokens":5,"output_tokens":2,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}),
        json!({"input_tokens":5,"output_tokens":2,"cache_read_input_tokens":3}),
        json!({"input_tokens":5,"output_tokens":2,"cache_creation_input_tokens":3}),
        json!({"output_tokens":2,"cache_read_input_tokens":3,"cache_creation_input_tokens":3}),
        json!({"input_tokens":u64::MAX,"output_tokens":2,"cache_read_input_tokens":1,"cache_creation_input_tokens":0}),
        json!({"input_tokens":5,"output_tokens":2,"total_tokens":123}),
    ];
    let items = usages
        .into_iter()
        .enumerate()
        .map(|(i, usage)| {
            claude(
                &format!("private-id-{i}"),
                &format!("2026-09-18T01:00:{i:02}Z"),
                usage,
            )
        })
        .collect::<Vec<_>>();
    jsonl(&root.path().join("chat.jsonl"), &items);
    let (records, _) = read_recent_local_requests(&[], root.path(), 200);
    assert_eq!(records.len(), 6);
    assert_eq!(records[5].input_tokens, Some(5));
    assert_eq!(records[5].total_tokens, Some(7));
    for record in &records[..5] {
        assert_eq!(record.input_tokens, None);
    }
    for record in &records[1..5] {
        assert_eq!(record.total_tokens, None);
    }
    assert_eq!(records[0].total_tokens, Some(123));
    let ids = records
        .iter()
        .map(|r| &r.id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 6);
    assert!(ids.iter().all(|id| !id.contains("private-id")));
}

#[test]
fn claude_normalizes_after_combining_partial_snapshots_across_files() {
    let root = tempdir().unwrap();
    jsonl(
        &root.path().join("first.jsonl"),
        &[claude(
            "shared",
            "2026-09-18T01:00:00Z",
            json!({"input_tokens":10,"cache_read_input_tokens":30}),
        )],
    );
    jsonl(
        &root.path().join("second.jsonl"),
        &[claude(
            "shared",
            "2026-09-18T01:00:01Z",
            json!({"output_tokens":5,"cache_creation_input_tokens":20}),
        )],
    );
    let (records, _) = read_recent_local_requests(&[], root.path(), 200);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].input_tokens, Some(60));
    assert_eq!(records[0].cached_tokens, Some(30));
    assert_eq!(records[0].total_tokens, Some(65));
}

#[test]
fn warnings_are_bounded_static_deduplicated_and_describe_coverage() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("bad.jsonl"), "broken\n".repeat(100)).unwrap();
    let missing = vec![root.path().join("private.db"); 10];
    let (_, warnings) = read_recent_local_requests(&missing, root.path(), 200);
    assert_eq!(warnings.len(), 4);
    assert_eq!(
        warnings
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
    assert!(warnings[0].contains("32 个 Codex"));
    assert!(warnings[0].contains("512 KiB"));
    assert!(warnings[0].contains("4096"));
    assert!(warnings.iter().all(|s| !s.contains("private.db")));
}

#[test]
fn local_identifiers_reject_urls_credentials_and_overlong_values() {
    let bad = [
        "https://private.invalid/token".to_string(),
        "www.private.invalid".to_string(),
        "sk-private-fixture".to_string(),
        "SK-private-fixture".to_string(),
        "Bearer private-fixture".to_string(),
        "Bearer-private-fixture".to_string(),
        "client_secret_fixture".to_string(),
        "api_key_fixture".to_string(),
        "auth_token_fixture".to_string(),
        "PRIVATE\nMESSAGE".to_string(),
        "x".repeat(201),
    ];
    for value in bad {
        let root = tempdir().unwrap();
        let rollout = root.path().join("rollout.jsonl");
        let project = root.path().join("project");
        fs::create_dir(&project).unwrap();
        jsonl(
            &rollout,
            &[
                json!({"type":"session_meta","payload":{"model_provider":value}}),
                json!({"type":"turn_context","payload":{"model":value}}),
                codex(
                    "2026-09-18T01:00:00Z",
                    json!({"output_tokens":1}),
                    Value::Null,
                ),
            ],
        );
        let mut item = claude(
            "private-id",
            "2026-09-18T01:00:00Z",
            json!({"output_tokens":1}),
        );
        item["message"]["model"] = json!(value);
        jsonl(&project.join("chat.jsonl"), &[item]);
        let db = database(root.path(), &[(1, &rollout)], true);
        let (records, warnings) = read_recent_local_requests(&[db], &project, 200);
        assert_eq!(records.len(), 2);
        assert!(
            records
                .iter()
                .all(|r| r.provider.is_none() && r.model.is_none())
        );
        let exported = serde_json::to_string(&(records, warnings)).unwrap();
        assert!(!exported.contains(&value));
        assert!(!exported.contains("private-id"));
    }
}

#[test]
fn identifier_limits_preserve_valid_models_and_providers() {
    let root = tempdir().unwrap();
    let rollout = root.path().join("rollout.jsonl");
    let provider = "p".repeat(128);
    let model = "m".repeat(200);
    jsonl(
        &rollout,
        &[
            json!({"type":"session_meta","payload":{"model_provider":provider}}),
            json!({"type":"turn_context","payload":{"model":model}}),
            codex(
                "2026-09-18T01:00:00Z",
                json!({"output_tokens":1}),
                Value::Null,
            ),
            json!({"type":"session_meta","payload":{"model_provider":"p".repeat(129)}}),
            json!({"type":"turn_context","payload":{"model":"gpt-5.5_fixture"}}),
            codex(
                "2026-09-18T01:00:01Z",
                json!({"output_tokens":1}),
                Value::Null,
            ),
        ],
    );
    let db = database(root.path(), &[(1, &rollout)], true);
    let (records, _) = read_recent_local_requests(&[db], &root.path().join("absent"), 200);
    assert_eq!(records.len(), 2);
    assert_eq!(records[1].provider.as_deref(), Some(provider.as_str()));
    assert_eq!(records[1].model.as_deref(), Some(model.as_str()));
    assert_eq!(records[0].provider, None);
    assert_eq!(records[0].model.as_deref(), Some("gpt-5.5_fixture"));
}

#[test]
#[ignore = "Explicit opt-in: reads real local usage; prints aggregate counts only"]
fn local_request_history_read_only_smoke() {
    use claude_codex_pro_core::codex_sqlite::{
        codex_session_db_paths_from_home, default_codex_home_dir,
    };
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .expect("A local home directory is required");
    let db_paths = codex_session_db_paths_from_home(&default_codex_home_dir());
    let started = std::time::Instant::now();
    let (records, warnings) =
        read_recent_local_requests(&db_paths, &home.join(".claude/projects"), 200);
    let elapsed_ms = started.elapsed().as_millis();
    let sources = ["codex_rollout", "claude_code"].map(|source| {
        let items = records.iter().filter(|r| r.source == source).collect::<Vec<_>>();
        json!({
            "source": source,
            "records": items.len(),
            "positive_token_records": items.iter().filter(|r| r.input_tokens.is_some_and(|v| v > 0) || r.output_tokens.is_some_and(|v| v > 0)).count(),
            "present": {
                "provider": items.iter().filter(|r| r.provider.is_some()).count(),
                "model": items.iter().filter(|r| r.model.is_some()).count(),
                "protocol": items.iter().filter(|r| r.protocol.is_some()).count(),
                "upstream_protocol": items.iter().filter(|r| r.upstream_protocol.is_some()).count(),
                "http_status": items.iter().filter(|r| r.http_status.is_some()).count(),
                "duration_ms": items.iter().filter(|r| r.duration_ms.is_some()).count(),
                "first_byte_ms": items.iter().filter(|r| r.first_byte_ms.is_some()).count(),
                "input_tokens": items.iter().filter(|r| r.input_tokens.is_some()).count(),
                "output_tokens": items.iter().filter(|r| r.output_tokens.is_some()).count(),
                "cached_tokens": items.iter().filter(|r| r.cached_tokens.is_some()).count(),
                "cache_creation_tokens": items.iter().filter(|r| r.cache_creation_tokens.is_some()).count(),
                "reasoning_tokens": items.iter().filter(|r| r.reasoning_tokens.is_some()).count(),
                "total_tokens": items.iter().filter(|r| r.total_tokens.is_some()).count(),
            }
        })
    });
    println!(
        "{}",
        json!({"records":records.len(),"elapsed_ms":elapsed_ms,"sources":sources,"warnings":warnings})
    );
    assert!(
        records.iter().any(|r| r.source == "codex_rollout"
            && (r.input_tokens.is_some_and(|v| v > 0) || r.output_tokens.is_some_and(|v| v > 0))),
        "No positive Codex usage observed within the bounded local coverage"
    );
}
