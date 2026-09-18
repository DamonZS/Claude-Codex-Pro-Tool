use super::*;
use std::sync::Arc;

fn autopilot() -> Value {
    json!({"id":"auto-a", "title":"Webhook task", "description":"Inspect current changes",
        "status":"active", "execution_mode":"run_only", "assignee_id":"agent-a",
        "triggers":[{"id":"trigger-a","kind":"webhook","enabled":true}]})
}

fn target() -> WebhookTarget {
    WebhookTarget::from_autopilot("workspace-a", &autopilot(), "trigger-a").unwrap()
}

fn fixture() -> (
    tempfile::TempDir,
    MulticaWebhookStore,
    WebhookTarget,
    String,
) {
    let dir = tempfile::tempdir().unwrap();
    let store = MulticaWebhookStore::new(dir.path().join("webhooks.json"));
    let target = target();
    let provisioned = store.provision(&target, "provision-a", 10).unwrap();
    let token = provisioned["webhook_token"].as_str().unwrap().to_owned();
    (dir, store, target, token)
}

fn headers(token: &str, key: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("Authorization".into(), format!("Bearer {token}")),
        ("Content-Type".into(), "application/json".into()),
        ("Idempotency-Key".into(), key.into()),
    ])
}

fn error<T>(result: anyhow::Result<T>) -> String {
    result.err().expect("expected error").to_string()
}

#[test]
fn configured_helper_url_is_public_and_changes_without_persisting_credentials() {
    let (dir, store, target, token) = fixture();
    assert!(store.trigger(&target).unwrap()["webhook_url"].is_null());
    let configured = store.with_helper_port(43127);
    let expected = format!("http://127.0.0.1:43127{}", target.ingress_path());
    let rotated = configured.rotate(&target, 1, "url-rotate", 20).unwrap();
    assert_eq!(rotated["webhook_url"], expected);
    assert!(!expected.contains(&token));
    let next_token = rotated["webhook_token"].as_str().unwrap();
    assert!(!expected.contains(next_token));
    for dto in [
        configured.trigger(&target).unwrap(),
        configured.rotate(&target, 1, "url-rotate", 21).unwrap(),
    ] {
        assert_eq!(dto["webhook_url"], expected);
        assert!(dto["webhook_token"].is_null());
    }
    let reopened =
        MulticaWebhookStore::new(dir.path().join("webhooks.json")).with_helper_port(43128);
    assert_eq!(
        reopened.trigger(&target).unwrap()["webhook_url"],
        format!("http://127.0.0.1:43128{}", target.ingress_path())
    );
    let stored = std::fs::read_to_string(dir.path().join("webhooks.json")).unwrap();
    assert!(!stored.contains(&token));
    assert!(!stored.contains(next_token));
    assert!(!stored.contains("43127"));
}

#[test]
fn provisioning_rotation_cas_replay_and_revocation_keep_secrets_private() {
    let (_dir, store, target, token) = fixture();
    assert_eq!(token.len(), 64);
    assert!(!target.ingress_path().contains(&token));
    assert!(store.trigger(&target).unwrap()["webhook_token"].is_null());
    let replay = store.provision(&target, "provision-a", 11).unwrap();
    assert!(replay["webhook_token"].is_null());
    assert_eq!(replay["credential_replay"], true);
    assert_eq!(
        error(store.provision(&target, "different", 12)),
        "webhook_revision_conflict"
    );
    assert_eq!(
        error(store.rotate(&target, 0, "rotate-a", 12)),
        "webhook_revision_conflict"
    );
    let rotated = store.rotate(&target, 1, "rotate-a", 12).unwrap();
    let next = rotated["webhook_token"].as_str().unwrap();
    assert_ne!(next, token);
    assert_eq!(rotated["credential_revision"], 2);
    assert_eq!(
        error(store.receive(&target, &headers(&token, "one"), b"{}", 13)),
        "webhook_credential_invalid"
    );
    assert!(store.rotate(&target, 1, "rotate-a", 14).unwrap()["webhook_token"].is_null());
    assert_eq!(
        error(store.rotate(&target, 2, "rotate-a", 15)),
        "webhook_command_conflict"
    );
    let disk = fs::read_to_string(&store.path).unwrap();
    assert!(!disk.contains(&token));
    assert!(!disk.contains(next));
    assert_eq!(error(store.revoke(&target, 1)), "webhook_revision_conflict");
    store.revoke(&target, 2).unwrap();
    store.revoke(&target, 2).unwrap();
    assert_eq!(
        error(store.receive(&target, &headers(next, "one"), b"{}", 16)),
        "webhook_credential_invalid"
    );
}

#[test]
fn webhook_hmac_matches_rfc4231_vectors() {
    assert_eq!(
        hex(&hmac_sha256(&[0x0b; 20], b"Hi There")),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
    assert_eq!(
        hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
    assert_eq!(
        hex(&hmac_sha256(
            &[0xaa; 131],
            b"Test Using Larger Than Block-Size Key - Hash Key First"
        )),
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
    );
}

#[test]
fn rotate_signature_replays_across_refresh_without_bypassing_first_cas() {
    let (_dir, store, target, _token) = fixture();
    let signature = "a".repeat(64);
    assert_eq!(
        error(store.rotate_with_signature(&target, 0, "rotate-refresh", &signature, 19)),
        "webhook_revision_conflict"
    );
    let first = store
        .rotate_with_signature(&target, 1, "rotate-refresh", &signature, 20)
        .unwrap();
    assert_eq!(first["credential_revision"], 2);
    let token = first["webhook_token"].as_str().unwrap();
    let restored = MulticaWebhookStore::new(store.path.clone());
    let replay = restored
        .rotate_with_signature(&target, 2, "rotate-refresh", &signature, 21)
        .unwrap();
    assert_eq!(replay["credential_revision"], 2);
    assert_eq!(replay["credential_replay"], true);
    assert!(replay["webhook_token"].is_null());
    assert!(
        restored
            .receive(&target, &headers(token, "still-valid"), b"{}", 22)
            .is_ok()
    );
    assert_eq!(
        error(restored.rotate_with_signature(&target, 2, "rotate-refresh", &"b".repeat(64), 23)),
        "webhook_command_conflict"
    );
    assert_eq!(
        error(restored.rotate_with_signature(&target, 1, "new-command", &signature, 23)),
        "webhook_revision_conflict"
    );
    assert_eq!(
        error(restored.rotate_with_signature(&target, 2, "new-command", "short", 23)),
        "webhook_command_signature_invalid"
    );
    let mut other = target.clone();
    other.trigger_id = "other-trigger".into();
    assert_eq!(
        error(restored.rotate_with_signature(&other, 2, "rotate-refresh", &signature, 23)),
        "webhook_command_conflict"
    );
    assert!(!fs::read_to_string(&store.path).unwrap().contains(token));
}

#[test]
fn token_signature_validation_and_idempotence_survive_reopen() {
    let (_dir, store, target, token) = fixture();
    let body = b"{\"action\":\"opened\"}";
    let mut h = headers(&token, "delivery-a");
    h.insert(
        "X-Hub-Signature-256".into(),
        format!("sha256={}", hex(&hmac_sha256(token.as_bytes(), body))),
    );
    let first = store.receive(&target, &h, body, 20).unwrap();
    assert_eq!(first.http_status, 202);
    assert!(first.should_dispatch);
    assert!(!first.duplicate);
    assert_eq!(first.delivery["signature_status"], "valid");
    let reopened = MulticaWebhookStore::new(store.path.clone());
    let second = reopened.receive(&target, &h, body, 21).unwrap();
    assert_eq!(first.delivery["id"], second.delivery["id"]);
    assert!(second.duplicate);
    assert_eq!(second.delivery["attempt_count"], 2);
    h.insert("X-Hub-Signature-256".into(), "sha256=invalid".into());
    assert_eq!(
        error(reopened.receive(&target, &h, body, 22)),
        "webhook_signature_invalid"
    );
    h.remove("X-Hub-Signature-256");
    assert_eq!(
        error(reopened.receive(&target, &h, b"{}", 23)),
        "webhook_idempotency_conflict"
    );
    assert_eq!(
        store.list("workspace-a", "auto-a", 10, 0).unwrap()["total"],
        1
    );
}

#[test]
fn invalid_signatures_are_audited_but_never_replayed() {
    let (_dir, store, target, token) = fixture();
    let mut h = headers(&token, "delivery-a");
    h.insert("X-Hub-Signature-256".into(), "sha256=invalid".into());
    let receipt = store.receive(&target, &h, b"{}", 20).unwrap();
    assert_eq!(receipt.http_status, 401);
    assert!(!receipt.should_dispatch);
    assert_eq!(receipt.delivery["status"], "rejected");
    assert!(receipt.delivery["raw_body"].is_null());
    assert_eq!(
        error(store.replay(
            &target,
            receipt.delivery["id"].as_str().unwrap(),
            "replay-a",
            21
        )),
        "webhook_replay_rejected"
    );
    h.insert("Authorization".into(), format!("Bearer {}", "a".repeat(64)));
    assert_eq!(
        error(store.receive(&target, &h, b"{}", 21)),
        "webhook_credential_invalid"
    );
    assert_eq!(
        store.list("workspace-a", "auto-a", 10, 0).unwrap()["total"],
        1
    );
}

#[test]
fn filters_paused_targets_and_header_bounds_stop_dispatch() {
    let (_dir, store, mut target, token) = fixture();
    target.trigger["event_filters"] = json!([{"event":"issues","actions":["opened"]}]);
    let mut h = headers(&token, "first");
    h.insert("X-GitHub-Event".into(), "issues".into());
    let ignored = store
        .receive(&target, &h, br#"{"action":"closed"}"#, 20)
        .unwrap();
    assert_eq!(ignored.delivery["reason_code"], "webhook_event_filtered");
    assert!(!ignored.should_dispatch);
    h.insert("Idempotency-Key".into(), "second".into());
    assert!(
        store
            .receive(&target, &h, br#"{"action":"opened"}"#, 20)
            .unwrap()
            .should_dispatch
    );
    target.active = false;
    h.insert("Idempotency-Key".into(), "third".into());
    assert_eq!(
        store
            .receive(&target, &h, br#"{"action":"opened"}"#, 20)
            .unwrap()
            .http_status,
        409
    );
    h.insert("authorization".into(), "duplicate".into());
    assert_eq!(
        error(store.receive(&target, &h, b"{}", 21)),
        "webhook_headers_invalid"
    );
    assert_eq!(
        error(store.receive(
            &target,
            &headers(&token, "long"),
            &vec![0; MAX_WEBHOOK_BODY_BYTES + 1],
            21
        )),
        "webhook_body_too_large"
    );
    let mut h = headers(&token, "long");
    h.insert("Extra".into(), "x".repeat(MAX_WEBHOOK_HEADER_BYTES));
    assert_eq!(
        error(store.receive(&target, &h, b"{}", 21)),
        "webhook_headers_too_large"
    );
    assert!(validate_filters(&json!([{"event":"issues","unknown":true}])).is_err());
    assert!(validate_filters(&json!([{"event":"issues","actions":[1]}])).is_err());
}

#[test]
fn audit_redacts_credentials_and_omits_detail_from_list() {
    let (_dir, store, target, token) = fixture();
    let body = serde_json::to_vec(
        &json!({"action":"opened","token":token,"nested":{"apiKey":"secret-material"},
        "note":format!("credential {token}"),"headers":{"Authorization":"Bearer secret-material"},
        "url":"https://example.test/path?key=secret-material","prompt":"full-private-prompt"}),
    )
    .unwrap();
    let row = store
        .receive(&target, &headers(&token, "redacted"), &body, 20)
        .unwrap()
        .delivery;
    let detail = store
        .get("workspace-a", "auto-a", row["id"].as_str().unwrap())
        .unwrap();
    assert!(detail["raw_body"].as_str().unwrap().contains("opened"));
    let disk = fs::read_to_string(&store.path).unwrap();
    for secret in [&token, "secret-material", "full-private-prompt", "Bearer "] {
        assert!(!disk.contains(secret));
    }
    let list = store.list("workspace-a", "auto-a", 10, 0).unwrap();
    assert!(list["deliveries"][0].get("raw_body").is_none());
    assert!(list["deliveries"][0].get("selected_headers").is_none());
    assert_eq!(
        error(store.get("other", "auto-a", row["id"].as_str().unwrap())),
        "webhook_delivery_not_found"
    );
    let body = serde_json::to_vec(&json!({"text":"x".repeat(MAX_AUDIT_BODY_BYTES)})).unwrap();
    let row = store
        .receive(&target, &headers(&token, "large"), &body, 20)
        .unwrap()
        .delivery;
    assert!(row["raw_body"].as_str().unwrap().len() < MAX_AUDIT_BODY_BYTES);
}

#[test]
fn replay_creates_one_new_occurrence_per_command() {
    let (_dir, store, target, token) = fixture();
    let original = store
        .receive(&target, &headers(&token, "original"), b"{}", 20)
        .unwrap()
        .delivery;
    let id = original["id"].as_str().unwrap();
    let replay = store.replay(&target, id, "replay-a", 21).unwrap();
    assert_ne!(replay["id"], original["id"]);
    assert_eq!(replay["replayed_from_delivery_id"], original["id"]);
    assert_eq!(store.replay(&target, id, "replay-a", 22).unwrap(), replay);
    assert_ne!(
        store.replay(&target, id, "replay-b", 22).unwrap()["id"],
        replay["id"]
    );
    assert_eq!(
        error(store.replay(&target, replay["id"].as_str().unwrap(), "replay-a", 22)),
        "webhook_command_conflict"
    );
}

#[test]
fn audit_bounds_preserve_pending_rows_and_dedupe_tombstones() {
    let (_dir, store, target, token) = fixture();
    let first = store
        .receive(&target, &headers(&token, "original"), b"{}", 20)
        .unwrap()
        .delivery;
    store
        .update(|state| {
            state.deliveries[0]["status"] = json!("dispatched");
            while state.deliveries.len() < MAX_DELIVERIES {
                let mut row = first.clone();
                row["id"] = json!(format!("pending-{}", state.deliveries.len()));
                state.deliveries.push(row);
            }
            Ok(())
        })
        .unwrap();
    store
        .receive(&target, &headers(&token, "next"), b"{}", 21)
        .unwrap();
    assert_eq!(store.load().unwrap().deliveries.len(), MAX_DELIVERIES);
    assert_eq!(
        error(store.receive(&target, &headers(&token, "original"), b"{}", 22)),
        "webhook_delivery_expired"
    );
    assert_eq!(
        error(store.receive(&target, &headers(&token, "overflow"), b"{}", 22)),
        "webhook_delivery_queue_full"
    );
    store
        .update(|state| {
            while state.receipts.len() < MAX_RECEIPTS {
                state.receipts.push(Receipt {
                    key: format!("receipt-{}", state.receipts.len()),
                    signature: String::new(),
                    delivery_id: None,
                    credential_revision: None,
                });
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(
        error(store.receive(&target, &headers(&token, "capacity"), b"{}", 22)),
        "webhook_receipts_full"
    );
}

#[test]
fn concurrent_ingress_reserves_one_delivery_under_store_lock() {
    let (_dir, store, target, token) = fixture();
    let threads: Vec<_> = (0..6)
        .map(|_| {
            let store = store.clone();
            let target = target.clone();
            let token = token.clone();
            std::thread::spawn(move || {
                store
                    .receive(&target, &headers(&token, "one"), b"{}", 20)
                    .unwrap()
                    .delivery["id"]
                    .clone()
            })
        })
        .collect();
    let ids: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_eq!(store.load().unwrap().deliveries.len(), 1);
}

#[tokio::test]
async fn real_core_dispatch_replay_and_crash_recovery_use_one_native_host_thread() {
    use crate::codex_execution::{
        CodexPageExecutionClient, CodexPageHostMethod, CodexRuntimeBinding,
        FakeCodexPageHostTransport,
    };
    use crate::multica_execution_store::MulticaExecutionStore;
    use crate::multica_workspace::{
        LocalMulticaWorkspaceStore, LocalWorkspaceEntityUpsert, MulticaWorkspaceResourceKey,
    };
    use crate::routes::CoreRuntimeService;
    let dir = tempfile::tempdir().unwrap();
    let workspace_id = crate::multica_workspace::local_workspace_id();
    let workspace = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    for (resource, entity) in [
        (
            MulticaWorkspaceResourceKey::Agents,
            json!({"id":"agent-a","name":"Worker"}),
        ),
        (MulticaWorkspaceResourceKey::Autopilots, autopilot()),
    ] {
        workspace
            .upsert(
                &workspace_id,
                LocalWorkspaceEntityUpsert {
                    resource,
                    entity,
                    expected_revision: Some(0),
                },
                1,
            )
            .unwrap();
    }
    let executions = MulticaExecutionStore::new(dir.path().join("execution.json"));
    let transport = FakeCodexPageHostTransport::default();
    let client = CodexPageExecutionClient::new(
        transport.clone(),
        CodexRuntimeBinding {
            runtime_id: "current-page".into(),
            provider: "codex".into(),
            app_server_version: None,
            declared_capabilities: vec![],
        },
    )
    .unwrap();
    let runtime = CoreRuntimeService::new(0, crate::status::StatusStore::default())
        .with_multica_execution_store(executions.clone())
        .with_multica_workspace_store(workspace)
        .with_codex_execution_service(Arc::new(client));
    let target = WebhookTarget::from_autopilot(&workspace_id, &autopilot(), "trigger-a").unwrap();
    let store = MulticaWebhookStore::new(dir.path().join("webhooks.json"));
    let dto = store.provision(&target, "provision-a", 10).unwrap();
    let token = dto["webhook_token"].as_str().unwrap();
    let receipt = store
        .receive(&target, &headers(token, "original"), b"{}", 20)
        .unwrap();
    let id = receipt.delivery["id"].as_str().unwrap();
    assert_eq!(store.pending(10).unwrap().len(), 1);
    let first = store.dispatch(&runtime, &target, id, 21).await.unwrap();
    assert_eq!(first["status"], "dispatched", "{first}");
    let run = executions
        .get_autopilot_run(first["autopilot_run_id"].as_str().unwrap())
        .unwrap();
    let binding = executions
        .get_execution(run.task_id.as_deref().unwrap())
        .unwrap();
    assert!(binding.codex_thread_id.is_some());
    assert_eq!(store.pending(10).unwrap().len(), 0);
    // Simulate the crash window after native commit but before delivery acknowledgement.
    store
        .update(|state| {
            state.deliveries[0]["status"] = json!("queued");
            Ok(())
        })
        .unwrap();
    let reopened = MulticaWebhookStore::new(store.path.clone());
    for _ in 0..3 {
        assert_eq!(
            reopened.dispatch(&runtime, &target, id, 22).await.unwrap()["autopilot_run_id"],
            first["autopilot_run_id"]
        );
    }
    assert_eq!(executions.list_autopilot_runs("auto-a").unwrap().len(), 1);
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|call| call.method == CodexPageHostMethod::ThreadStart)
            .count(),
        1
    );
}

#[tokio::test]
async fn offline_host_is_not_reported_as_native_completion() {
    let (_dir, store, target, token) = fixture();
    let runtime = crate::routes::CoreRuntimeService::new(0, crate::status::StatusStore::default())
        .with_multica_workspace_store(crate::multica_workspace::LocalMulticaWorkspaceStore::new(
            _dir.path().join("workspace.json"),
        ))
        .with_multica_execution_store(crate::multica_execution_store::MulticaExecutionStore::new(
            _dir.path().join("execution.json"),
        ));
    let row = store
        .receive(&target, &headers(&token, "one"), b"{}", 20)
        .unwrap()
        .delivery;
    let result = store
        .dispatch(&runtime, &target, row["id"].as_str().unwrap(), 21)
        .await
        .unwrap();
    assert_eq!(result["status"], "failed");
    assert!(result["autopilot_run_id"].is_null());
    assert_eq!(result["error"], "webhook_dispatch_failed");
}

#[test]
fn failed_before_dispatch_survives_reload_duplicates_and_replay_without_counting() {
    use crate::multica_execution_store::AutopilotRunTransition;

    for (reason, previous) in [(Some("autopilot_assignee_unavailable"), 0), (None, 3)] {
        let (dir, store, target, token) = fixture();
        let execution_path = dir.path().join("execution.json");
        let executions = MulticaExecutionStore::new(execution_path.clone());
        let received = store
            .receive_and_enqueue(&executions, &target, &headers(&token, "one"), b"{}", 20)
            .unwrap();
        let id = received.delivery["id"].as_str().unwrap();
        let run = executions
            .get_autopilot_run(received.delivery["autopilot_run_id"].as_str().unwrap())
            .unwrap();
        let failed_run = executions
            .transition_autopilot_run(AutopilotRunTransition {
                autopilot_id: run.autopilot_id,
                run_id: run.id,
                expected_revision: run.revision,
                next_status: "failed".into(),
                issue_id: None,
                task_id: None,
                failure_reason: reason.map(str::to_owned),
                reason_code: reason.map(str::to_owned),
                now_ms: 21,
            })
            .unwrap();
        store
            .update(|state| {
                state.deliveries[0]["dispatch_attempts"] = json!(previous);
                Ok(())
            })
            .unwrap();

        let reopened = MulticaWebhookStore::new(store.path.clone());
        let executions = MulticaExecutionStore::new(execution_path);
        let expected_reason = reason.unwrap_or("webhook_dispatch_failed");
        for now in 22..25 {
            let failed = reopened.enqueue(&executions, &target, id, now).unwrap();
            assert_eq!(failed["status"], "failed");
            assert_eq!(failed["dispatch_attempts"], previous);
            assert_eq!(failed["autopilot_run_id"], failed_run.id);
            assert_eq!(failed["error"], expected_reason);
            assert_eq!(failed["reason_code"], expected_reason);
            assert_eq!(failed["response_status"], 503);
            assert!(reopened.pending(100).unwrap().is_empty());
            let duplicate = reopened
                .receive_and_enqueue(&executions, &target, &headers(&token, "one"), b"{}", now)
                .unwrap();
            assert!(duplicate.duplicate);
            assert_eq!(duplicate.http_status, 503);
            assert_eq!(duplicate.delivery["status"], "failed");
            assert_eq!(duplicate.delivery["dispatch_attempts"], previous);
        }
        assert_eq!(
            executions.list_autopilot_runs("auto-a").unwrap(),
            vec![failed_run.clone()]
        );
        let persisted = MulticaWebhookStore::new(store.path.clone())
            .get("workspace-a", "auto-a", id)
            .unwrap();
        assert_eq!(persisted["status"], "failed");
        assert_eq!(persisted["error"], expected_reason);

        let replay = reopened.replay(&target, id, "replay-failed", 26).unwrap();
        let queued = reopened
            .enqueue(&executions, &target, replay["id"].as_str().unwrap(), 27)
            .unwrap();
        assert_eq!(queued["status"], "queued");
        assert_eq!(queued["dispatch_attempts"], 0);
        assert!(queued["error"].is_null());
        assert!(queued["reason_code"].is_null());
        assert_ne!(queued["autopilot_run_id"], failed_run.id);
        assert_eq!(executions.list_autopilot_runs("auto-a").unwrap().len(), 2);
        assert_eq!(
            executions.get_autopilot_run(&failed_run.id).unwrap(),
            failed_run
        );
    }
}

#[test]
fn failed_after_binding_keeps_dispatched_projection_and_single_count() {
    use crate::multica_execution_store::AutopilotRunTransition;

    for project_before_failure in [false, true] {
        let (dir, store, target, token) = fixture();
        let executions = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let received = store
            .receive_and_enqueue(&executions, &target, &headers(&token, "one"), b"{}", 20)
            .unwrap();
        let id = received.delivery["id"].as_str().unwrap();
        let mut run = executions
            .get_autopilot_run(received.delivery["autopilot_run_id"].as_str().unwrap())
            .unwrap();
        for (status, now) in [("running", 21), ("failed", 23)] {
            run = executions
                .transition_autopilot_run(AutopilotRunTransition {
                    autopilot_id: run.autopilot_id,
                    run_id: run.id,
                    expected_revision: run.revision,
                    next_status: status.into(),
                    issue_id: None,
                    task_id: Some("binding-fixture".into()),
                    failure_reason: (status == "failed").then(|| "execution_failed".into()),
                    reason_code: (status == "failed").then(|| "execution_failed".into()),
                    now_ms: now,
                })
                .unwrap();
            if project_before_failure && status == "running" {
                assert_eq!(
                    store.enqueue(&executions, &target, id, 22).unwrap()["status"],
                    "dispatched"
                );
            }
        }
        let reopened = MulticaWebhookStore::new(store.path.clone());
        for now in 24..26 {
            let delivery = reopened.enqueue(&executions, &target, id, now).unwrap();
            assert_eq!(delivery["status"], "dispatched");
            assert_eq!(delivery["dispatch_attempts"], 1);
            assert_eq!(delivery["autopilot_run_id"], run.id);
            assert!(delivery["error"].is_null());
            assert!(delivery["reason_code"].is_null());
        }
    }
}

#[test]
fn queued_handoff_counts_once_across_duplicates_recovery_and_replay() {
    use crate::multica_execution_store::AutopilotRunTransition;
    let (dir, store, target, token) = fixture();
    let executions = MulticaExecutionStore::new(dir.path().join("execution.json"));
    let received = store
        .receive_and_enqueue(&executions, &target, &headers(&token, "one"), b"{}", 20)
        .unwrap();
    let id = received.delivery["id"].as_str().unwrap();
    assert_eq!(received.delivery["dispatch_attempts"], 0);
    let run = executions
        .get_autopilot_run(received.delivery["autopilot_run_id"].as_str().unwrap())
        .unwrap();
    executions
        .transition_autopilot_run(AutopilotRunTransition {
            autopilot_id: run.autopilot_id,
            run_id: run.id,
            expected_revision: run.revision,
            next_status: "pending".into(),
            issue_id: None,
            task_id: Some("binding-fixture".into()),
            failure_reason: None,
            reason_code: None,
            now_ms: 21,
        })
        .unwrap();
    let reopened = MulticaWebhookStore::new(store.path.clone());
    let dispatched = reopened.enqueue(&executions, &target, id, 22).unwrap();
    assert_eq!(dispatched["status"], "dispatched");
    assert_eq!(dispatched["dispatch_attempts"], 1);
    for now in 23..25 {
        let duplicate = reopened
            .receive_and_enqueue(&executions, &target, &headers(&token, "one"), b"{}", now)
            .unwrap();
        assert_eq!(duplicate.delivery["dispatch_attempts"], 1);
        assert_eq!(
            reopened.enqueue(&executions, &target, id, now).unwrap()["dispatch_attempts"],
            1
        );
    }
    assert_eq!(
        reopened.get("workspace-a", "auto-a", id).unwrap()["attempt_count"],
        3
    );
    for previous in [1, 3] {
        reopened
            .update(|state| {
                state.deliveries[0]["status"] = json!("queued");
                state.deliveries[0]["dispatch_attempts"] = json!(previous);
                Ok(())
            })
            .unwrap();
        assert_eq!(
            reopened.enqueue(&executions, &target, id, 25).unwrap()["dispatch_attempts"],
            previous
        );
    }
    let replay = reopened.replay(&target, id, "replay-count", 26).unwrap();
    assert_eq!(replay["dispatch_attempts"], 0);
    assert_eq!(
        reopened
            .enqueue(&executions, &target, replay["id"].as_str().unwrap(), 27)
            .unwrap()["dispatch_attempts"],
        0
    );
}

#[test]
fn helper_ingress_only_queues_existing_engine_and_recovers_both_crash_windows() {
    use crate::multica_execution_store::MulticaExecutionStore;
    let (_dir, store, target, token) = fixture();
    let executions = MulticaExecutionStore::new(_dir.path().join("execution.json"));
    // Crash after receipt persistence, before run reservation.
    let row = store
        .receive(&target, &headers(&token, "one"), b"{}", 20)
        .unwrap()
        .delivery;
    assert!(executions.list_autopilot_runs("auto-a").unwrap().is_empty());
    assert!(row["autopilot_run_id"].is_null());
    let id = row["id"].as_str().unwrap();
    let queued = store.enqueue(&executions, &target, id, 21).unwrap();
    assert_eq!(queued["status"], "queued");
    assert!(queued["autopilot_run_id"].as_str().is_some());
    // Crash after run reservation, before receipt update.
    store
        .update(|state| {
            state.deliveries[0]["autopilot_run_id"] = Value::Null;
            Ok(())
        })
        .unwrap();
    let reopened = MulticaWebhookStore::new(store.path.clone());
    let retried = reopened
        .receive_and_enqueue(&executions, &target, &headers(&token, "one"), b"{}", 22)
        .unwrap();
    assert!(retried.duplicate);
    assert_eq!(retried.http_status, 202);
    assert_eq!(retried.delivery["status"], "queued");
    assert_eq!(
        retried.delivery["autopilot_run_id"],
        queued["autopilot_run_id"]
    );
    let runs = executions.list_autopilot_runs("auto-a").unwrap();
    assert_eq!(runs.len(), 1);
    assert!(runs[0].task_id.is_none());
    let due = executions.tick_autopilots(&[autopilot()], 23).unwrap();
    assert_eq!(due.runs.len(), 1);
    assert_eq!(due.runs[0].occurrence_id.as_deref(), Some(id));
}
