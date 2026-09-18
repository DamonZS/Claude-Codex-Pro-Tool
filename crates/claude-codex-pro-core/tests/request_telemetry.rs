use std::io::Write;
use std::path::Path;

use claude_codex_pro_core::request_telemetry::{
    MAX_RECENT_REQUESTS, MAX_REQUEST_LOG_BYTES, RequestFailure, RequestObservation,
    append_request_at, read_recent_requests_at,
};
use serde_json::json;

fn observation(streaming: bool, protocol: &str) -> RequestObservation {
    let mut observer = RequestObservation::new(
        "codex",
        "responses",
        r#"{"model":"requested-model","input":"PRIVATE REQUEST","api_key":"PRIVATE KEY"}"#,
    );
    observer.upstream(
        200,
        streaming,
        protocol,
        Some("actual-profile"),
        Some("upstream-model"),
    );
    observer
}

#[test]
fn json_usage_metrics_are_allowlisted_and_missing_fields_stay_unknown() {
    let mut observer = observation(false, "chat_completions");
    observer.push_bytes(br#"{"model":"actual-model","choices":[{"message":{"content":"PRIVATE RESPONSE"}}],"usage":{"prompt_tokens":30,"completion_tokens":7,"total_tokens":37,"prompt_tokens_details":{"cached_tokens":20},"completion_tokens_details":{"reasoning_tokens":3}}}"#);
    let record = observer.finish();
    assert_eq!(record.status, "success");
    assert_eq!(record.model.as_deref(), Some("actual-model"));
    assert_eq!(record.provider.as_deref(), Some("actual-profile"));
    assert_eq!(record.input_tokens, Some(30));
    assert_eq!(record.output_tokens, Some(7));
    assert_eq!(record.cached_tokens, Some(20));
    assert_eq!(record.reasoning_tokens, Some(3));
    assert_eq!(record.total_tokens, Some(37));
    assert_eq!(record.cache_creation_tokens, None);
    assert!(record.duration_ms >= record.first_byte_ms);
    assert!(record.first_byte_ms.is_some());
    let serialized = serde_json::to_string(&record).unwrap();
    assert!(!serialized.contains("PRIVATE"));
    assert!(serialized.contains("input_tokens"));
    assert!(!serialized.contains("inputTokens"));
}

#[test]
fn sse_handles_every_utf8_crlf_chunk_boundary_and_does_not_accumulate_snapshots() {
    let sse = concat!(
        ": heartbeat\r\n\r\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"中文\"}}],\"usage\":{\"prompt_tokens\":12,\"completion_tokens\":1}}\r\n\r\n",
        "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":12,\"completion_tokens\":8,\"prompt_tokens_details\":{\"cached_tokens\":4}}}\r\n\r\n",
        "data: [DONE]\r\n\r\n"
    ).as_bytes();
    for split in 0..=sse.len() {
        let mut observer = observation(true, "chat_completions");
        observer.push_bytes(&sse[..split]);
        observer.push_bytes(&sse[split..]);
        let record = observer.finish();
        assert_eq!(record.status, "success", "split {split}");
        assert_eq!(record.input_tokens, Some(12));
        assert_eq!(record.output_tokens, Some(8));
        assert_eq!(record.cached_tokens, Some(4));
    }
    let mut observer = observation(true, "chat_completions");
    for byte in sse {
        observer.push_bytes(&[*byte]);
    }
    assert_eq!(observer.finish().output_tokens, Some(8));
}

#[test]
fn anthropic_incremental_usage_preserves_initial_input_and_cache() {
    let mut observer = observation(true, "messages");
    for data in [
        json!({"type":"message_start","message":{"type":"message","model":"claude-model","usage":{"input_tokens":10,"output_tokens":1,"cache_read_input_tokens":90,"cache_creation_input_tokens":40}}}),
        json!({"type":"message_delta","usage":{"output_tokens":6}}),
        json!({"type":"message_delta","usage":{"output_tokens":9}}),
        json!({"type":"message_stop"}),
    ] {
        observer.push_bytes(format!("data: {data}\n\n").as_bytes());
    }
    let record = observer.finish();
    assert_eq!(record.status, "success");
    assert_eq!(record.input_tokens, Some(140));
    assert_eq!(record.output_tokens, Some(9));
    assert_eq!(record.cached_tokens, Some(90));
    assert_eq!(record.cache_creation_tokens, Some(40));
    assert_eq!(record.total_tokens, None);
}

#[test]
fn anthropic_cache_normalization_is_checked_and_missing_fields_serialize_as_null() {
    let mut observer = observation(true, "messages");
    for usage in [
        json!({"input_tokens":10,"output_tokens":1,"cache_read_input_tokens":30}),
        json!({"output_tokens":4}),
        json!({"output_tokens":6}),
    ] {
        observer.push_bytes(
            format!(
                "data: {}\n\n",
                json!({"type":"message_delta","usage":usage})
            )
            .as_bytes(),
        );
    }
    observer.push_bytes(b"data: {\"type\":\"message_stop\"}\n\n");
    let record = observer.finish();
    assert_eq!(record.input_tokens, Some(40));
    assert_eq!(record.cached_tokens, Some(30));
    assert_eq!(record.output_tokens, Some(6));
    let value = serde_json::to_value(record).unwrap();
    assert_eq!(
        value.get("cache_creation_tokens"),
        Some(&serde_json::Value::Null)
    );
    assert_eq!(
        value.get("reasoning_tokens"),
        Some(&serde_json::Value::Null)
    );
    assert_eq!(value.get("total_tokens"), Some(&serde_json::Value::Null));

    for usage in [
        json!({"cache_read_input_tokens":5,"output_tokens":1}),
        json!({"input_tokens":u64::MAX,"cache_read_input_tokens":5,"output_tokens":1}),
    ] {
        let mut observer = observation(false, "messages");
        observer.push_bytes(
            json!({"type":"message","usage":usage})
                .to_string()
                .as_bytes(),
        );
        let record = observer.finish();
        assert_eq!(record.input_tokens, None);
        assert_eq!(record.status, "observed");
    }
    let record = RequestObservation::new("codex", "responses", "{}").finish();
    let value = serde_json::to_value(record).unwrap();
    for field in [
        "provider",
        "model",
        "upstream_protocol",
        "http_status",
        "first_byte_ms",
        "input_tokens",
        "output_tokens",
        "cached_tokens",
        "cache_creation_tokens",
        "reasoning_tokens",
        "total_tokens",
    ] {
        assert_eq!(value.get(field), Some(&serde_json::Value::Null), "{field}");
    }
}

#[test]
fn identifiers_and_deserialized_logs_do_not_expose_credentials_or_free_text() {
    for identifier in [
        "sk-fixture",
        "Bearer-key",
        "token-value",
        "key-value",
        "api_key-value",
        "https://example.test",
        "PRIVATE BODY",
        "model\ncontent",
    ] {
        let request = json!({"model":identifier}).to_string();
        let mut observer = RequestObservation::new("codex", "responses", &request);
        observer.upstream(200, false, "responses", Some(identifier), Some(identifier));
        let record = observer.finish();
        assert_eq!(record.model, None, "{identifier}");
        assert_eq!(record.provider, None, "{identifier}");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("requests.jsonl");
    let mut value = serde_json::to_value(observation(false, "messages").finish()).unwrap();
    value["model"] = json!("sk-fixture");
    value["provider"] = json!("https://private.test");
    value["protocol"] = json!("PRIVATE BODY");
    value["upstream_protocol"] = json!("Bearer-key");
    value["raw_body"] = json!("PRIVATE RESPONSE");
    std::fs::write(&path, format!("{value}\n")).unwrap();
    let records = read_recent_requests_at(&path, 20).unwrap();
    assert_eq!(records.len(), 1);
    let cleaned = serde_json::to_string(&records).unwrap();
    for denied in [
        "sk-fixture",
        "https://private",
        "PRIVATE",
        "Bearer",
        "raw_body",
    ] {
        assert!(!cleaned.contains(denied));
    }
    for field in ["id", "source", "status", "agent"] {
        let mut tampered = value.clone();
        tampered[field] = json!("PRIVATE BODY");
        std::fs::write(&path, format!("{tampered}\n")).unwrap();
        assert!(
            read_recent_requests_at(&path, 20).unwrap().is_empty(),
            "{field}"
        );
    }
}

#[test]
fn contended_storage_lock_has_a_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("requests.jsonl");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))
        .unwrap();
    fs2::FileExt::lock_exclusive(&lock).unwrap();
    let started = std::time::Instant::now();
    let record = observation(false, "messages").finish();
    assert!(append_request_at(&path, &record).is_err());
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    fs2::FileExt::unlock(&lock).unwrap();
}

#[test]
fn responses_completed_nested_usage_and_multiline_sse() {
    let mut observer = observation(true, "responses");
    observer.push_bytes(b"event: response.completed\ndata: {\"type\":\"response.completed\",\ndata: \"response\":{\"object\":\"response\",\"usage\":{\"input_tokens\":10,\"output_tokens\":2,\"input_tokens_details\":{\"cached_tokens\":3}}}}\n\n");
    let record = observer.finish();
    assert_eq!(record.status, "success");
    assert_eq!(record.cached_tokens, Some(3));
}

#[test]
fn missing_malformed_or_overflow_usage_never_becomes_zero_or_success() {
    for body in [
        r#"{"choices":[]}"#,
        r#"{"choices":[],"usage":null}"#,
        r#"{"choices":[],"usage":{"prompt_tokens":-1,"completion_tokens":"2"}}"#,
        r#"{"choices":[],"usage":{"prompt_tokens":18446744073709551616,"completion_tokens":1.5}}"#,
        r#"{"choices":[],"usage":{"prompt_tokens":9}}"#,
        "not json",
    ] {
        let mut observer = observation(false, "chat_completions");
        observer.push_bytes(body.as_bytes());
        let record = observer.finish();
        assert_eq!(record.status, "observed", "{body}");
        assert_eq!(record.output_tokens, None);
        assert_eq!(record.cached_tokens, None);
    }
    let mut observer = observation(false, "messages");
    observer.push_bytes(br#"{"type":"message","usage":{"input_tokens":0,"output_tokens":0}}"#);
    assert_eq!(observer.finish().input_tokens, Some(0));
}

#[test]
fn failures_and_truncated_streams_preserve_outcome_and_reported_usage() {
    let mut observer = RequestObservation::new("claude", "messages", "{}");
    observer.fail(RequestFailure::Network);
    let record = observer.finish();
    assert_eq!(record.status, "failed");
    assert_eq!(record.http_status, None);
    assert_eq!(record.first_byte_ms, None);
    for code in [400, 401, 429, 500, 503] {
        let mut observer = observation(false, "messages");
        observer.upstream(code, false, "messages", None, None);
        observer.push_bytes(br#"{"error":{"message":"PRIVATE upstream error"},"usage":{"input_tokens":4,"output_tokens":1}}"#);
        let record = observer.finish();
        assert_eq!(record.status, "failed");
        assert_eq!(record.input_tokens, None);
        assert_eq!(record.provider, None);
    }
    for terminal in ["", "data: [DONE]\n\n"] {
        for failure in [
            None,
            Some(RequestFailure::Stream),
            Some(RequestFailure::ClientDisconnect),
        ] {
            let mut observer = observation(true, "chat_completions");
            observer.push_bytes(b"data: {\"choices\":[],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2}}\n\n");
            observer.push_bytes(terminal.as_bytes());
            if let Some(failure) = failure {
                observer.fail(failure);
            }
            let record = observer.finish();
            assert_eq!(
                record.status,
                if terminal.is_empty() || failure.is_some() {
                    "interrupted"
                } else {
                    "success"
                }
            );
            assert_eq!(record.output_tokens, Some(2));
        }
    }
    for payload in [
        r#"{"type":"error","error":{"message":"PRIVATE"}}"#,
        r#"{"type":"response.failed","response":{"status":"failed"}}"#,
    ] {
        let mut observer = observation(true, "messages");
        observer.push_bytes(format!("data: {payload}\n\n").as_bytes());
        assert_eq!(observer.finish().status, "failed");
    }
}

#[test]
fn oversized_or_malformed_sse_stays_bounded_and_does_not_claim_success() {
    let mut observer = observation(true, "chat_completions");
    observer.push_bytes(b"data: ");
    observer.push_bytes(&vec![b'x'; 2 * 1024 * 1024]);
    observer.push_bytes(b"\n\ndata: {\"choices\":[],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\ndata: [DONE]\n\n");
    let record = observer.finish();
    assert_eq!(record.status, "observed");
    assert_eq!(record.output_tokens, Some(1));
    let mut observer = observation(false, "chat_completions");
    observer.push_bytes(&vec![b'x'; 2 * 1024 * 1024]);
    assert_eq!(observer.finish().status, "observed");
}

fn append_fixture(path: &Path, timestamp: u64) {
    let mut record = observation(false, "messages").finish();
    record.timestamp_ms = timestamp;
    append_request_at(path, &record).unwrap();
}

#[test]
fn storage_recovers_partial_lines_sorts_and_bounds_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("requests.jsonl");
    assert!(read_recent_requests_at(&path, 10).unwrap().is_empty());
    append_fixture(&path, 100);
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"{torn")
        .unwrap();
    assert_eq!(read_recent_requests_at(&path, 10).unwrap().len(), 1);
    append_fixture(&path, 300);
    append_fixture(&path, 200);
    let records = read_recent_requests_at(&path, 2).unwrap();
    assert_eq!(
        records.iter().map(|r| r.timestamp_ms).collect::<Vec<_>>(),
        [300, 200]
    );
    assert!(read_recent_requests_at(&path, 0).unwrap().is_empty());
    assert_eq!(read_recent_requests_at(&path, usize::MAX).unwrap().len(), 3);
    assert!(MAX_RECENT_REQUESTS < usize::MAX);
}

#[test]
fn storage_rotates_one_generation_and_reads_a_bounded_tail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("requests.jsonl");
    let rotated = path.with_extension("jsonl.1");
    std::fs::write(&path, vec![b'x'; MAX_REQUEST_LOG_BYTES as usize + 1024]).unwrap();
    assert!(read_recent_requests_at(&path, 10).unwrap().is_empty());
    append_fixture(&path, 100);
    assert!(rotated.exists());
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    file.write_all(&vec![b'\n'; MAX_REQUEST_LOG_BYTES as usize])
        .unwrap();
    drop(file);
    append_fixture(&path, 200);
    let records = read_recent_requests_at(&path, 10).unwrap();
    assert_eq!(records[0].timestamp_ms, 200);
    assert!(std::fs::metadata(&path).unwrap().len() <= MAX_REQUEST_LOG_BYTES);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3);
}

#[test]
fn child_writer() {
    let Some(path) = std::env::var_os("CCP_TELEMETRY_TEST_PATH") else {
        return;
    };
    for index in 0..30 {
        append_fixture(Path::new(&path), index);
    }
}

#[test]
fn independent_processes_share_the_log_without_losing_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("requests.jsonl");
    let mut children = (0..3)
        .map(|_| {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "child_writer", "--nocapture"])
                .env("CCP_TELEMETRY_TEST_PATH", &path)
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    let records = read_recent_requests_at(&path, 200).unwrap();
    assert_eq!(records.len(), 90);
    let ids = records
        .iter()
        .map(|r| &r.id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 90);
}

struct FixturePaths;

impl Drop for FixturePaths {
    fn drop(&mut self) {
        claude_codex_pro_core::paths::set_settings_path_for_tests(None);
        claude_codex_pro_core::diagnostic_log::set_diagnostic_log_path_for_tests(None);
    }
}

#[tokio::test]
async fn helper_handlers_record_real_http_and_preserve_streams() {
    use claude_codex_pro_core::request_telemetry::read_recent_requests;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");
    claude_codex_pro_core::paths::set_settings_path_for_tests(Some(settings_path.clone()));
    claude_codex_pro_core::diagnostic_log::set_diagnostic_log_path_for_tests(Some(
        dir.path().join("diagnostic.log"),
    ));
    let _paths = FixturePaths;
    let reservation = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let helper_port = reservation.local_addr().unwrap().port();
    drop(reservation);
    claude_codex_pro_core::launcher::ensure_detached_helper(helper_port)
        .await
        .unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();
    let chat_json = r#"{"id":"chatcmpl-test","model":"actual-model","choices":[{"message":{"role":"assistant","content":"PRIVATE RESPONSE"},"finish_reason":"stop"}],"usage":{"prompt_tokens":11,"completion_tokens":3}}"#;
    let chat_sse = "data: {\"model\":\"actual-model\",\"choices\":[{\"delta\":{\"content\":\"PRIVATE RESPONSE\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":3}}\r\n\r\ndata: [DONE]\r\n\r\n";
    let claude_json = r#"{"type":"message","model":"actual-model","content":[],"usage":{"input_tokens":11,"output_tokens":3,"cache_read_input_tokens":8}}"#;
    let claude_sse = "event: message_start\r\ndata: {\"type\":\"message_start\",\"message\":{\"type\":\"message\",\"model\":\"actual-model\",\"usage\":{\"input_tokens\":11,\"output_tokens\":0,\"cache_read_input_tokens\":8}}}\r\n\r\nevent: message_delta\r\ndata: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":3}}\r\n\r\nevent: message_stop\r\ndata: {\"type\":\"message_stop\"}\r\n\r\n";
    let cases = [
        (
            "/v1/chat/completions",
            chat_json,
            false,
            200,
            false,
            "success",
        ),
        ("/v1/responses", chat_json, false, 200, false, "success"),
        (
            "/claude-desktop/v1/messages",
            claude_json,
            false,
            200,
            false,
            "success",
        ),
        (
            "/v1/chat/completions",
            chat_sse,
            true,
            200,
            false,
            "success",
        ),
        ("/v1/responses", chat_sse, true, 200, false, "success"),
        (
            "/claude-desktop/v1/messages",
            claude_sse,
            true,
            200,
            false,
            "success",
        ),
        (
            "/v1/chat/completions",
            "{\"error\":\"PRIVATE upstream error\"}",
            false,
            429,
            false,
            "failed",
        ),
        (
            "/v1/chat/completions",
            "{\"choices\":[]}",
            false,
            200,
            false,
            "observed",
        ),
        (
            "/v1/chat/completions",
            chat_sse,
            true,
            200,
            true,
            "interrupted",
        ),
        ("/v1/responses", chat_sse, true, 200, true, "interrupted"),
        (
            "/claude-desktop/v1/messages",
            claude_sse,
            true,
            200,
            true,
            "interrupted",
        ),
        ("/v1/chat/completions", "", false, 0, false, "failed"),
    ];
    for (index, (route, body, streaming, code, truncated, expected)) in
        cases.into_iter().enumerate()
    {
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = upstream.local_addr().unwrap();
        let settings = json!({
            "activeRelayId":"fixture-codex", "activeClaudeDesktopRelayId":"fixture-claude",
            "relayProfiles":[
                {"id":"fixture-codex","name":"Fixture","targetApp":"codex","protocol":"chatCompletions","routeEnabled":true,"baseUrl":format!("http://{address}"),"upstreamBaseUrl":format!("http://{address}"),"apiKey":"fixture-secret"},
                {"id":"fixture-claude","name":"Fixture","targetApp":"claude-desktop","routeEnabled":true,"baseUrl":format!("http://{address}"),"upstreamBaseUrl":format!("http://{address}"),"apiKey":"fixture-secret"}
            ]
        });
        std::fs::write(&settings_path, serde_json::to_vec(&settings).unwrap()).unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = upstream.accept().await.unwrap();
            let mut received = Vec::new();
            loop {
                let mut buf = [0; 2048];
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                received.extend_from_slice(&buf[..n]);
                if let Some(end) = received.windows(4).position(|v| v == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&received[..end]);
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse().unwrap())
                        })
                        .unwrap();
                    if received.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            if code == 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(15)).await;
            let content_type = if streaming {
                "text/event-stream"
            } else {
                "application/json"
            };
            socket.write_all(format!("HTTP/1.1 {code} Test\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len() + if truncated { 5 } else { 0 }).as_bytes()).await.unwrap();
            for chunk in body.as_bytes().chunks(17) {
                socket.write_all(chunk).await.unwrap();
            }
            socket.shutdown().await.unwrap();
        });
        let response = client.post(format!("http://127.0.0.1:{helper_port}{route}"))
            .json(&json!({"model":"fixture-model","messages":[{"role":"user","content":"PRIVATE REQUEST"}],"input":"PRIVATE REQUEST","stream":streaming}))
            .send().await.unwrap();
        let received = response.bytes().await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), server)
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "upstream not reached: {}",
                    String::from_utf8_lossy(&received)
                )
            })
            .unwrap();
        if code == 200 && !truncated && route != "/v1/responses" {
            assert_eq!(
                received.as_ref(),
                body.as_bytes(),
                "pass-through bytes changed for {route}"
            );
        }
        let mut records = Vec::new();
        for _ in 0..100 {
            records = read_recent_requests(100).unwrap();
            if records.len() == index + 1 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert_eq!(records.len(), index + 1, "missing record for {route}");
        let record = &records[0];
        assert_eq!(record.status, expected, "{route} / case {index}");
        assert_eq!(
            record.http_status,
            if code == 0 { None } else { Some(code) }
        );
        if code > 0 {
            assert_eq!(
                record.provider.as_deref(),
                Some(if route.contains("claude") {
                    "fixture-claude"
                } else {
                    "fixture-codex"
                })
            );
            assert!(record.first_byte_ms.unwrap() >= 10);
            assert!(record.duration_ms >= record.first_byte_ms);
        }
        if expected == "success" {
            assert_eq!(
                record.input_tokens,
                Some(if route.contains("claude") { 19 } else { 11 })
            );
            assert_eq!(record.output_tokens, Some(3));
        }
    }
    let log = std::fs::read_to_string(dir.path().join("requests.jsonl")).unwrap();
    assert!(!log.contains("PRIVATE"));
    assert!(!log.contains("fixture-secret"));
}
