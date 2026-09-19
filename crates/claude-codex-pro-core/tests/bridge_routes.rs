#![recursion_limit = "256"]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use claude_codex_pro_core::codex_execution::{
    CodexPageExecutionClient, CodexPageHostMethod, CodexRuntimeBinding, FakeCodexPageHostTransport,
};
use claude_codex_pro_core::launcher::{
    CodexLaunch, LaunchHooks, LaunchOptions, ProcessWaitStrategy, launch_and_inject_with_hooks,
};
use claude_codex_pro_core::models::{
    DeleteResult, DeleteStatus, ExportResult, ExportStatus, SessionRef,
};
use claude_codex_pro_core::multica_execution_store::MulticaExecutionStore;
use claude_codex_pro_core::multica_webhooks::{MulticaWebhookStore, WebhookTarget};
use claude_codex_pro_core::multica_workspace::{LocalMulticaWorkspaceStore, MulticaWorkspaceQuery};
use claude_codex_pro_core::routes::{
    BridgeContext, BridgeDataService, BridgeRuntimeService, BridgeSettingsService,
    CoreRuntimeService, handle_bridge_request,
};
use claude_codex_pro_core::settings::BackendSettings;
use claude_codex_pro_core::status::StatusStore;
use claude_codex_pro_core::user_scripts::UserScriptManager;
use serde_json::{Value, json};
use sha2::Digest;

#[tokio::test]
async fn bridge_routes_cover_all_current_paths() {
    let ctx = test_context();

    let cases = [
        ("/settings/get", json!({})),
        ("/settings/set", json!({"providerSyncEnabled": true})),
        ("/user-scripts/list", json!({})),
        ("/user-scripts/set-enabled", json!({"enabled": false})),
        (
            "/user-scripts/set-script-enabled",
            json!({"key": "user:a.js", "enabled": false}),
        ),
        ("/user-scripts/delete", json!({"key": "user:a.js"})),
        ("/user-scripts/reload", json!({})),
        ("/devtools/open", json!({})),
        ("/manager/open", json!({})),
        ("/multica/workspace/bootstrap", json!({})),
        ("/multica/autopilots/tick", json!({})),
        (
            "/multica/autopilots/cron-preview",
            json!({"expr":"* * * * *","tz":"UTC"}),
        ),
        (
            "/multica/workspace/query",
            json!({"resource": "skills", "limit": 25, "offset": 0}),
        ),
        (
            "/multica/workspace/move-issue",
            json!({"issueId": "issue-a", "beforeId": null, "afterId": null, "expectedRevision": 1}),
        ),
        ("/multica/skills/resolve", json!({"bindings": {}})),
        (
            "/multica/skills/review",
            json!({"id": "skill:review", "trusted": true}),
        ),
        (
            "/multica/skills/bind",
            json!({
                "scopeKind": "agent",
                "scopeId": "agent-a",
                "skillRef": {"id": "skill:review"},
                "enabled": true
            }),
        ),
        (
            "/multica/skills/unbind",
            json!({"scopeKind": "agent", "scopeId": "agent-a", "skillId": "skill:review"}),
        ),
        ("/multica/skills/bindings", json!({})),
        (
            "/multica/skills/bindings/replace",
            json!({"scopeKind": "agent", "scopeId": "agent-a", "skills": []}),
        ),
        (
            "/multica/executions/lease/claim",
            json!({"bindingId":"binding-a","expectedRevision":1,"leaseToken":"token-a","leaseDurationMs":1000}),
        ),
        (
            "/multica/executions/lease/renew",
            json!({"bindingId":"binding-a","expectedRevision":1,"leaseToken":"token-a","leaseDurationMs":1000}),
        ),
        (
            "/multica/executions/lease/release",
            json!({"bindingId":"binding-a","expectedRevision":1,"leaseToken":"token-a"}),
        ),
        (
            "/multica/executions/messages",
            json!({"message":{"messageId":"message-a","bindingId":"binding-a","seq":1,"messageType":"assistant","createdAtMs":1}}),
        ),
        (
            "/multica/executions/messages/list",
            json!({"bindingId":"binding-a"}),
        ),
        ("/backend/status", json!({})),
        ("/backend/repair", json!({})),
        ("/claude-desktop/status", json!({})),
        ("/claude-desktop/integrity", json!({})),
        ("/claude-desktop/focus", json!({})),
        ("/claude-desktop/verify", json!({})),
        ("/claude-desktop/open", json!({})),
        ("/claude-desktop/new-chat", json!({})),
        (
            "/claude-desktop/paste-draft",
            json!({"text": "hello Claude"}),
        ),
        ("/claude-desktop/submit", json!({"text": "hello Claude"})),
        ("/codex-model-catalog", json!({})),
        ("/codex-config-model", json!({})),
        ("/ads", json!({})),
        ("/zed-remote/status", json!({})),
        (
            "/zed-remote/resolve-host",
            json!({"hostId": "remote-ssh-codex-managed:remote"}),
        ),
        (
            "/zed-remote/fallback-request",
            json!({"hostId": "remote-ssh-codex-managed:remote"}),
        ),
        (
            "/zed-remote/open",
            json!({"ssh": {"host": "example.com"}, "path": "/home/app.py"}),
        ),
        ("/zed-remote/projects", json!({})),
        (
            "/zed-remote/remember-project",
            json!({"ssh": {"host": "example.com"}, "path": "/home/app.py"}),
        ),
        (
            "/zed-remote/forget-project",
            json!({"id": "zed-remote-project:test"}),
        ),
        ("/upstream-worktree/status", json!({})),
        ("/upstream-worktree/defaults", json!({"repoPath": "/repo"})),
        (
            "/upstream-worktree/prepare",
            json!({"repoPath": "/repo", "remote": "upstream", "baseBranch": "main"}),
        ),
        (
            "/upstream-worktree/create",
            json!({"repoPath": "/repo", "branchName": "feature/demo"}),
        ),
        ("/session-availability", json!({"session_ids": ["s1"]})),
        ("/delete", json!({"session_id": "s1", "title": "First"})),
        ("/undo", json!({"undo_token": "undo-1"})),
        (
            "/export-markdown",
            json!({"session_id": "s1", "title": "First"}),
        ),
        (
            "/thread-usage-history",
            json!({"session_id": "s1", "title": "First"}),
        ),
        ("/archived-thread", json!({"title": "Archived"})),
        (
            "/move-thread-workspace",
            json!({"session_id": "s1", "title": "First", "target_cwd": "/new"}),
        ),
        (
            "/thread-sort-key",
            json!({"session_id": "s1", "title": "First"}),
        ),
        (
            "/thread-sort-keys",
            json!({"sessions": [{"session_id": "s1", "title": "First"}]}),
        ),
    ];

    for (path, payload) in cases {
        let result = handle_bridge_request(ctx.clone(), path, payload).await;
        assert_ne!(
            result["message"], "Unknown bridge path",
            "{path} should be routed"
        );
    }
}

#[tokio::test]
async fn multica_workspace_bridge_rejects_transport_passthrough_before_runtime() {
    let runtime = Arc::new(FakeRuntime::default());
    let ctx = BridgeContext::new(
        Arc::new(FakeSettings::default()),
        runtime.clone(),
        Arc::new(FakeData::default()),
    );

    for payload in [
        json!({"resource": "skills", "url": "https://evil.example"}),
        json!({"resource": "issues", "method": "DELETE"}),
        json!({"resource": "runtimes", "headers": {"Authorization": "Bearer sentinel"}}),
        json!({"resource": "projects", "token": "sentinel"}),
        json!({"resource": "unknown"}),
        json!({"resource": "issues", "limit": 101}),
    ] {
        let response =
            handle_bridge_request(ctx.clone(), "/multica/workspace/query", payload).await;
        assert_eq!(response["status"], "failed");
    }
    for (path, payload) in [
        (
            "/multica/skills/bind",
            json!({"scopeKind": "agent", "scopeId": "agent-a", "skillRef": {"id": ""}}),
        ),
        (
            "/multica/skills/unbind",
            json!({"scopeKind": "agent", "scopeId": "", "skillId": "skill:a"}),
        ),
        (
            "/multica/skills/bindings",
            json!({"url": "https://evil.example"}),
        ),
        (
            "/multica/skills/bindings/replace",
            json!({"scopeKind": "agent", "scopeId": "agent-a", "skills": [], "url": "https://evil.example"}),
        ),
    ] {
        let response = handle_bridge_request(ctx.clone(), path, payload).await;
        assert_eq!(response["status"], "failed");
        assert_ne!(response["message"], "Unknown bridge path");
    }
    let bootstrap = handle_bridge_request(
        ctx.clone(),
        "/multica/workspace/bootstrap",
        json!({"url": "https://evil.example"}),
    )
    .await;
    let unknown = handle_bridge_request(
        ctx,
        "/multica/workspace/mutate",
        json!({"action": "shell", "command": "whoami"}),
    )
    .await;

    assert_eq!(bootstrap["message"], "multica_payload_invalid");
    assert_eq!(unknown["message"], "Unknown bridge path");
    assert!(runtime.multica_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn multica_autopilot_tick_routes_only_empty_enabled_requests() {
    let runtime = Arc::new(FakeRuntime::default());
    let settings = Arc::new(FakeSettings::default());
    let ctx = BridgeContext::new(
        settings.clone(),
        runtime.clone(),
        Arc::new(FakeData::default()),
    );
    let response = handle_bridge_request(ctx.clone(), "/multica/autopilots/tick", json!({})).await;
    assert_eq!(response["status"], "ok");
    assert_eq!(runtime.multica_calls.lock().unwrap().as_slice(), ["tick"]);
    for payload in [
        json!({"nowMs":1}),
        json!({"url":"unexpected"}),
        json!([]),
        json!(null),
    ] {
        let response =
            handle_bridge_request(ctx.clone(), "/multica/autopilots/tick", payload).await;
        assert_eq!(response["message"], "autopilot_tick_payload_invalid");
    }
    settings.settings.lock().unwrap().multica_workspace_enabled = false;
    let response = handle_bridge_request(ctx, "/multica/autopilots/tick", json!({})).await;
    assert_eq!(response["message"], "multica_workspace_disabled");
    assert_eq!(runtime.multica_calls.lock().unwrap().as_slice(), ["tick"]);
}

#[tokio::test]
async fn multica_cron_preview_is_read_only_and_rejects_extra_fields() {
    let runtime = Arc::new(FakeRuntime::default());
    let settings = Arc::new(FakeSettings::default());
    let ctx = BridgeContext::new(
        settings.clone(),
        runtime.clone(),
        Arc::new(FakeData::default()),
    );
    let payload = json!({"expr":"0 9 * * *","tz":"Asia/Shanghai"});
    let response = handle_bridge_request(
        ctx.clone(),
        "/multica/autopilots/cron-preview",
        payload.clone(),
    )
    .await;
    assert_eq!(response["status"], "ok");
    let runs = response["next_runs"].as_array().unwrap();
    assert_eq!(runs.len(), 5);
    for run in runs {
        assert!(chrono::DateTime::parse_from_rfc3339(run.as_str().unwrap()).is_ok());
    }
    for field in ["count", "nowMs", "url", "timezone", "cronExpression"] {
        let mut invalid = payload.clone();
        invalid[field] = json!(1);
        let response =
            handle_bridge_request(ctx.clone(), "/multica/autopilots/cron-preview", invalid).await;
        assert_eq!(
            response["message"],
            "autopilot_cron_preview_payload_invalid"
        );
    }
    settings.settings.lock().unwrap().multica_workspace_enabled = false;
    let response = handle_bridge_request(ctx, "/multica/autopilots/cron-preview", payload).await;
    assert_eq!(response["message"], "multica_workspace_disabled");
    assert!(runtime.multica_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn multica_skill_bridge_rejects_renderer_inventory_and_transport_fields() {
    let runtime = Arc::new(FakeRuntime::default());
    let ctx = BridgeContext::new(
        Arc::new(FakeSettings::default()),
        runtime.clone(),
        Arc::new(FakeData::default()),
    );
    for payload in [
        json!({"bindings": {}, "inventory": []}),
        json!({"bindings": {}, "runtimeCapabilities": ["skill-bundles-v1"]}),
        json!({"bindings": {"task": [{"id": "skill:a"}]}, "url": "https://evil.example"}),
        json!({"bindings": {"task": [{"id": "skill:a"}]}, "headers": {"Authorization": "Bearer secret"}}),
    ] {
        let response = handle_bridge_request(ctx.clone(), "/multica/skills/resolve", payload).await;
        assert_eq!(response["status"], "failed");
        assert_ne!(response["message"], "Unknown bridge path");
    }
    for payload in [
        json!({"id": "", "trusted": true}),
        json!({"id": "skill:a", "trusted": true, "manifestDigest": "not a digest"}),
        json!({"id": "skill:a", "trusted": true, "url": "https://evil.example"}),
    ] {
        let response = handle_bridge_request(ctx.clone(), "/multica/skills/review", payload).await;
        assert_eq!(response["status"], "failed");
        assert_ne!(response["message"], "Unknown bridge path");
    }
    assert!(runtime.multica_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn multica_workspace_bridge_accepts_only_typed_read_operations() {
    let runtime = Arc::new(FakeRuntime::default());
    let ctx = BridgeContext::new(
        Arc::new(FakeSettings::default()),
        runtime.clone(),
        Arc::new(FakeData::default()),
    );

    let bootstrap =
        handle_bridge_request(ctx.clone(), "/multica/workspace/bootstrap", json!({})).await;
    let query = handle_bridge_request(
        ctx,
        "/multica/workspace/query",
        json!({"resource": "skills", "limit": 25, "offset": 0}),
    )
    .await;

    assert_eq!(bootstrap["status"], "ok");
    assert_eq!(query["status"], "ok");
    assert_eq!(query["resource"], "skills");
    assert_eq!(
        runtime.multica_calls.lock().unwrap().as_slice(),
        ["bootstrap", "query:skills:25:0"]
    );
}

#[tokio::test]
async fn core_multica_skills_query_fails_closed_without_codex_page_host() {
    let ctx = BridgeContext::core(Arc::new(CoreRuntimeService::new(
        9229,
        StatusStore::default(),
    )));

    let response = handle_bridge_request(
        ctx,
        "/multica/workspace/query",
        json!({"resource": "skills", "limit": 25, "offset": 0}),
    )
    .await;

    assert_eq!(response["status"], "failed");
    assert_eq!(response["message"], "codex_page_host_unavailable");
}

#[tokio::test]
async fn core_multica_skill_resolve_fails_closed_without_codex_page_host() {
    let ctx = BridgeContext::core(Arc::new(CoreRuntimeService::new(
        9229,
        StatusStore::default(),
    )));

    let response =
        handle_bridge_request(ctx, "/multica/skills/resolve", json!({"bindings": {}})).await;

    assert_eq!(response["status"], "failed");
    assert_eq!(response["message"], "codex_page_host_unavailable");
}

#[tokio::test]
async fn multica_execution_bridge_routes_cover_native_lifecycle_and_replay() {
    let (ctx, transport, _store_dir) = multica_execution_test_context();
    let create_payload = json!({
        "workspaceId": "workspace-1",
        "issueId": "issue-1",
        "prompt": "first prompt",
        "cwd": "C:/workspace",
        "idempotencyKey": "create-1",
        "bindings": {}
    });

    let created = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/create",
        create_payload.clone(),
    )
    .await;
    assert_eq!(created["status"], "ok");
    assert_eq!(created["handle"]["runtimeId"], "codex-current-page");
    assert_eq!(created["handle"]["threadId"], "thread-fake-0");
    assert_eq!(created["handle"]["executionId"], "turn-fake-0");
    let created_revision = created["binding"]["revision"].as_u64().unwrap();
    assert_eq!(created_revision, 3); // reserve, claim, commit
    assert_eq!(created["binding"]["state"], "dispatched");
    let binding_id = created["binding"]["bindingId"]
        .as_str()
        .expect("binding id")
        .to_string();
    assert_eq!(transport.calls().len(), 3);

    // Replaying the same create is answered from the persisted binding and
    // must not send another request to the current page host.
    let replay =
        handle_bridge_request(ctx.clone(), "/multica/executions/create", create_payload).await;
    assert_eq!(replay, created);
    assert_eq!(transport.calls().len(), 3);

    let opened = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/open",
        json!({"bindingId": binding_id}),
    )
    .await;
    assert_eq!(opened["status"], "ok");
    assert_eq!(opened["handle"]["threadId"], "thread-fake-0");
    assert_eq!(opened["handle"]["executionId"], Value::Null);

    transport.push_response(
        CodexPageHostMethod::ThreadRead,
        Ok(json!({"thread":{
            "id":"thread-fake-0","turns":[{"id":"turn-fake-0","status":"completed","items":[]}]
        }})),
    );
    let completed = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/status",
        json!({"bindingId":binding_id}),
    )
    .await;
    assert_eq!(completed["binding"]["state"], "completed", "{completed}");
    let completed_revision = completed["binding"]["revision"].as_u64().unwrap();
    let continued = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/continue",
        json!({
            "bindingId": binding_id,
            "prompt": "next prompt",
            "cwd": "C:/workspace",
            "idempotencyKey": "continue-1",
            "bindings": {},
            "expectedRevision": completed_revision
        }),
    )
    .await;
    assert_eq!(continued["status"], "ok");
    assert_eq!(continued["handle"]["threadId"], "thread-fake-0");
    assert_eq!(continued["handle"]["executionId"], "turn-fake-1");
    let continued_revision = completed_revision + 1;
    assert_eq!(continued["binding"]["revision"], continued_revision);

    let continue_replay = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/continue",
        json!({
            "bindingId": binding_id,
            "prompt": "next prompt",
            "cwd": "C:/workspace",
            "idempotencyKey": "continue-1",
            "bindings": {},
            "expectedRevision": completed_revision
        }),
    )
    .await;
    assert_eq!(continue_replay["status"], "ok");
    assert_eq!(continue_replay["handle"]["executionId"], "turn-fake-1");
    assert_eq!(continue_replay["binding"]["revision"], continued_revision);
    assert_eq!(continue_replay, continued);
    let calls = transport.calls().len();
    for (field, value) in [
        ("prompt", json!("changed prompt")),
        ("cwd", json!("C:/another-workspace")),
        ("bindings", json!({"task":[{"id":"changed-skill"}]})),
        ("expectedRevision", json!(continued_revision)),
    ] {
        let mut changed = json!({"bindingId":binding_id,"prompt":"next prompt","cwd":"C:/workspace",
            "idempotencyKey":"continue-1","bindings":{},"expectedRevision":completed_revision});
        changed[field] = value;
        let result =
            handle_bridge_request(ctx.clone(), "/multica/executions/continue", changed).await;
        assert_eq!(
            result["message"], "execution_command_idempotency_conflict",
            "{field}: {result}"
        );
        assert_eq!(transport.calls().len(), calls);
    }

    let status = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/status",
        json!({"bindingId": binding_id}),
    )
    .await;
    assert_eq!(status["status"], "ok");
    assert_eq!(status["executionStatus"]["threadId"], "thread-fake-0");
    assert_eq!(status["executionStatus"]["executionId"], "turn-fake-1");
    assert_eq!(status["executionStatus"]["state"], "unknown");
    assert_eq!(
        status["executionStatus"]["diagnostic"],
        "codex_turn_unavailable"
    );
    assert_eq!(status["binding"]["state"], "dispatched");
    assert_eq!(status["binding"]["revision"], continued_revision);

    let cancelled = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/cancel",
        json!({
            "bindingId": binding_id,
            "idempotencyKey": "cancel-1",
            "expectedRevision": continued_revision
        }),
    )
    .await;
    assert_eq!(cancelled["status"], "ok");
    assert_eq!(cancelled["executionStatus"]["state"], "cancel_pending");
    assert_eq!(cancelled["binding"]["state"], "cancel_pending");
    let cancelled_revision = continued_revision + 1;
    assert_eq!(cancelled["binding"]["revision"], cancelled_revision);

    let cancel_replay = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/cancel",
        json!({
            "bindingId": binding_id,
            "idempotencyKey": "cancel-1",
            "expectedRevision": continued_revision
        }),
    )
    .await;
    assert_eq!(cancel_replay, cancelled);
    let calls = transport.calls().len();
    let changed = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/cancel",
        json!({
            "bindingId":binding_id,"idempotencyKey":"cancel-1","expectedRevision":cancelled_revision
        }),
    )
    .await;
    assert_eq!(changed["message"], "execution_command_idempotency_conflict");
    assert_eq!(transport.calls().len(), calls);
    // A later operation changed the binding, but the original Continue reply is stable.
    let late_replay = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/continue",
        json!({
            "bindingId":binding_id,"prompt":"next prompt","cwd":"C:/workspace",
            "idempotencyKey":"continue-1","bindings":{},"expectedRevision":completed_revision
        }),
    )
    .await;
    assert_eq!(late_replay, continued);
    assert_eq!(transport.calls().len(), calls);

    let listed = handle_bridge_request(
        ctx,
        "/multica/executions/list",
        json!({
            "workspaceId": "workspace-1",
            "issueId": "issue-1",
            "limit": 50,
            "offset": 0
        }),
    )
    .await;
    assert_eq!(listed["status"], "ok");
    assert_eq!(listed["total"], 1);
    assert_eq!(listed["items"].as_array().unwrap().len(), 1);
    assert_eq!(listed["items"][0]["bindingId"], binding_id);
    assert_eq!(listed["items"][0]["state"], "cancel_pending");

    let methods = transport
        .calls()
        .into_iter()
        .map(|call| call.method)
        .collect::<Vec<_>>();
    assert_eq!(
        methods,
        vec![
            CodexPageHostMethod::Initialize,
            CodexPageHostMethod::ThreadStart,
            CodexPageHostMethod::TurnStart,
            CodexPageHostMethod::ThreadRead,
            CodexPageHostMethod::ThreadRead,
            CodexPageHostMethod::TurnStart,
            CodexPageHostMethod::ThreadRead,
            CodexPageHostMethod::TurnInterrupt,
        ]
    );
    let persisted = MulticaExecutionStore::new(_store_dir.path().join("multica-execution.json"));
    persisted
        .record_status(
            &binding_id,
            cancelled_revision,
            &claude_codex_pro_core::codex_execution::CodexExecutionStatus {
                runtime_id: "codex-current-page".into(),
                thread_id: "thread-fake-0".into(),
                execution_id: "turn-fake-1".into(),
                state: claude_codex_pro_core::codex_execution::CodexExecutionState::Cancelled,
                diagnostic: None,
            },
            1_900_000_000_000,
        )
        .unwrap();
    let fresh_transport = FakeCodexPageHostTransport::default();
    let fresh_client = CodexPageExecutionClient::new(
        fresh_transport.clone(),
        CodexRuntimeBinding {
            runtime_id: "codex-current-page".into(),
            provider: "codex".into(),
            app_server_version: None,
            declared_capabilities: vec![],
        },
    )
    .unwrap();
    let fresh = BridgeContext::new(
        Arc::new(FakeSettings::default()),
        Arc::new(
            CoreRuntimeService::new(
                9229,
                StatusStore::new(_store_dir.path().join("status.json")),
            )
            .with_codex_execution_service(Arc::new(fresh_client))
            .with_multica_workspace_store(LocalMulticaWorkspaceStore::new(
                _store_dir.path().join("workspace.json"),
            ))
            .with_multica_execution_store(MulticaExecutionStore::new(
                _store_dir.path().join("multica-execution.json"),
            )),
        ),
        Arc::new(FakeData::default()),
    )
    .without_diagnostics();
    let continued_after_reload = handle_bridge_request(
        fresh.clone(),
        "/multica/executions/continue",
        json!({
            "bindingId":binding_id,"prompt":"next prompt","cwd":"C:/workspace",
            "idempotencyKey":"continue-1","bindings":{},"expectedRevision":completed_revision
        }),
    )
    .await;
    assert_eq!(continued_after_reload, continued);
    let cancelled_after_reload = handle_bridge_request(
        fresh,
        "/multica/executions/cancel",
        json!({
            "bindingId":binding_id,"idempotencyKey":"cancel-1","expectedRevision":continued_revision
        }),
    )
    .await;
    assert_eq!(cancelled_after_reload, cancelled);
    assert!(fresh_transport.calls().is_empty());
}

#[tokio::test]
async fn multica_continue_rejects_active_turn_without_native_call_or_command_reservation() {
    let (ctx, transport, dir) = multica_execution_test_context();
    let created=handle_bridge_request(ctx.clone(),"/multica/executions/create",json!({
        "workspaceId":"workspace-1","issueId":"issue-1","prompt":"work","idempotencyKey":"active-create","bindings":{}
    })).await;
    assert_eq!(created["status"], "ok", "{created}");
    let binding = created["binding"].clone();
    for native_status in [None, Some("inProgress")] {
        let binding = if let Some(status) = native_status {
            transport.push_response(CodexPageHostMethod::ThreadRead,Ok(json!({"thread":{
                "id":binding["codexThreadId"],"turns":[{"id":binding["codexExecutionId"],"status":status,"items":[]}]
            }})));
            let polled = handle_bridge_request(
                ctx.clone(),
                "/multica/executions/status",
                json!({"bindingId":binding["bindingId"]}),
            )
            .await;
            assert_eq!(polled["binding"]["state"], "running", "{polled}");
            polled["binding"].clone()
        } else {
            binding.clone()
        };
        let calls = transport.calls().len();
        let denied=handle_bridge_request(ctx.clone(),"/multica/executions/continue",json!({
            "bindingId":binding["bindingId"],"expectedRevision":binding["revision"],"prompt":"second turn",
            "idempotencyKey":"active-continue","bindings":{}
        })).await;
        assert_eq!(denied["message"], "execution_not_continuable", "{denied}");
        assert_eq!(transport.calls().len(), calls);
    }
    assert!(
        MulticaExecutionStore::new(dir.path().join("multica-execution.json"))
            .load()
            .unwrap()
            .execution_commands
            .iter()
            .all(|command| command.command_id != "active-continue")
    );
}

#[tokio::test]
async fn multica_execution_bridge_create_failure_is_persisted_and_retryable() {
    let (ctx, transport, _store_dir) = multica_execution_test_context();
    transport.push_response(
        CodexPageHostMethod::TurnStart,
        Err(anyhow::anyhow!("transient page host failure")),
    );
    let first = json!({
        "workspaceId": "workspace-1",
        "issueId": "issue-2",
        "prompt": "will fail",
        "idempotencyKey": "create-failed",
        "bindings": {}
    });
    let failed =
        handle_bridge_request(ctx.clone(), "/multica/executions/create", first.clone()).await;
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["message"], "codex_execution_failed");

    // The failed idempotency key is stable and does not replay a new page
    // request. A fresh key creates the next persisted attempt instead.
    let failed_replay =
        handle_bridge_request(ctx.clone(), "/multica/executions/create", first).await;
    assert_eq!(failed_replay["status"], "failed");
    assert_eq!(failed_replay["message"], "codex_execution_failed");
    assert_eq!(transport.calls().len(), 3);

    let recovered = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/create",
        json!({
            "workspaceId": "workspace-1",
            "issueId": "issue-2",
            "prompt": "retry",
            "idempotencyKey": "create-retry",
            "bindings": {}
        }),
    )
    .await;
    assert_eq!(recovered["status"], "ok");
    assert_eq!(recovered["handle"]["threadId"], "thread-fake-1");
    // The failed TurnStart response does not consume a fake turn sequence
    // number, so the recovered request receives the first successful ID.
    assert_eq!(recovered["handle"]["executionId"], "turn-fake-0");
    assert_eq!(recovered["binding"]["attemptNo"], 2);
    assert_eq!(recovered["binding"]["state"], "dispatched");

    let listed = handle_bridge_request(
        ctx,
        "/multica/executions/list",
        json!({"workspaceId": "workspace-1", "issueId": "issue-2"}),
    )
    .await;
    assert_eq!(listed["status"], "ok");
    assert_eq!(listed["total"], 2);
    assert_eq!(listed["items"].as_array().unwrap().len(), 2);
    assert!(
        listed["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["state"] == "failed")
    );
    assert!(
        listed["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["state"] == "dispatched")
    );
}

#[tokio::test]
async fn core_multica_execution_routes_fail_closed_without_codex_page_host() {
    let settings = Arc::new(FakeSettings::default());
    let runtime = Arc::new(CoreRuntimeService::new(9229, StatusStore::default()));
    let ctx = BridgeContext::new(settings, runtime, Arc::new(FakeData::default()));

    let response = handle_bridge_request(
        ctx,
        "/multica/executions/create",
        json!({
            "workspaceId": "workspace-1",
            "issueId": "issue-1",
            "prompt": "no page host",
            "idempotencyKey": "create-no-host",
            "bindings": {}
        }),
    )
    .await;

    assert_eq!(response["status"], "failed");
    assert_eq!(response["message"], "codex_page_host_unavailable");
}

#[tokio::test]
async fn codex_model_catalog_routes_remain_read_only() {
    let ctx = test_context();

    // The renderer may read this catalog for service-tier eligibility, but the
    // route must continue to return the runtime catalog unchanged.
    let catalog = handle_bridge_request(ctx.clone(), "/codex-model-catalog", json!({})).await;
    assert_eq!(catalog["status"], "ok");
    assert_eq!(catalog["model"], "qwen3-coder");
    assert_eq!(catalog["model_provider"], "relay");
    assert_eq!(catalog["provider_name"], "Relay");
    assert_eq!(catalog["models"], json!(["qwen3-coder"]));

    // Keep the legacy alias working for existing bridge clients.
    let alias = handle_bridge_request(ctx, "/codex-config-model", json!({})).await;
    assert_eq!(alias, catalog);
}

#[tokio::test]
async fn settings_get_includes_runtime_codex_app_version() {
    let ctx = BridgeContext::new(
        Arc::new(FakeSettings::with_codex_app_version("26.601.21317")),
        Arc::new(FakeRuntime::default()),
        Arc::new(FakeData::default()),
    );

    let result = handle_bridge_request(ctx, "/settings/get", json!({})).await;

    assert_eq!(result["codexAppVersion"], json!("26.601.21317"));
    assert_eq!(result["codexAppPluginEntryUnlock"], json!(true));
    assert_eq!(result["codexAppPluginMarketplaceUnlock"], json!(true));
    assert_eq!(result["codexAppForcePluginInstall"], json!(true));
}

#[tokio::test]
async fn settings_set_does_not_persist_runtime_codex_app_version() {
    let settings = Arc::new(FakeSettings::with_codex_app_version("26.601.21317"));
    let ctx = BridgeContext::new(
        settings.clone(),
        Arc::new(FakeRuntime::default()),
        Arc::new(FakeData::default()),
    );

    let result = handle_bridge_request(
        ctx,
        "/settings/set",
        json!({
            "codexAppVersion": "1.2.3",
            "codexAppPluginMarketplaceUnlock": false
        }),
    )
    .await;

    assert_eq!(result["codexAppVersion"], json!("26.601.21317"));
    assert_eq!(result["codexAppPluginMarketplaceUnlock"], json!(false));

    let persisted = settings.settings.lock().unwrap().clone();
    let persisted_value = serde_json::to_value(persisted).unwrap();
    assert!(persisted_value.get("codexAppVersion").is_none());
}

#[tokio::test]
async fn settings_ignore_legacy_codex_model_whitelist_flag() {
    let settings = Arc::new(FakeSettings::default());
    let ctx = BridgeContext::new(
        settings.clone(),
        Arc::new(FakeRuntime::default()),
        Arc::new(FakeData::default()),
    );

    let updated = handle_bridge_request(
        ctx.clone(),
        "/settings/set",
        json!({"codexAppModelWhitelistUnlock": false}),
    )
    .await;
    let loaded = handle_bridge_request(ctx, "/settings/get", json!({})).await;

    assert!(updated.get("codexAppModelWhitelistUnlock").is_none());
    assert!(loaded.get("codexAppModelWhitelistUnlock").is_none());
    let persisted = serde_json::to_value(settings.settings.lock().unwrap().clone()).unwrap();
    assert!(persisted.get("codexAppModelWhitelistUnlock").is_none());
}

#[tokio::test]
async fn bridge_context_core_with_app_dir_exposes_runtime_codex_app_version() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp
        .path()
        .join("OpenAI.Codex_26.601.21317.0_x64__abc")
        .join("app");
    std::fs::create_dir_all(&app_dir).unwrap();
    std::fs::write(app_dir.join("Codex.exe"), "").unwrap();
    let ctx = BridgeContext::core_with_data_and_app_dir(
        Arc::new(FakeRuntime::default()),
        Arc::new(FakeData::default()),
        app_dir,
    );

    let result = handle_bridge_request(ctx, "/settings/get", json!({})).await;

    assert_eq!(result["codexAppVersion"], json!("26.601.21317.0"));
}

#[tokio::test]
async fn upstream_worktree_routes_are_dispatched_to_runtime() {
    let ctx = test_context();

    assert_eq!(
        handle_bridge_request(ctx.clone(), "/upstream-worktree/status", json!({})).await,
        json!({"status": "ok", "feature": "upstream-worktree"})
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/upstream-worktree/defaults",
            json!({"repoPath": "/repo"}),
        )
        .await,
        json!({
            "status": "ok",
            "repoRoot": "/repo",
            "defaultRemote": "upstream",
            "defaultBaseBranch": "main",
        })
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/upstream-worktree/create",
            json!({"repoPath": "/repo", "branchName": "feature/demo"}),
        )
        .await,
        json!({
            "status": "ok",
            "repoRoot": "/repo",
            "branchName": "feature/demo",
            "worktreePath": "/repo-feature-demo",
        })
    );
    assert_eq!(
        handle_bridge_request(
            ctx,
            "/upstream-worktree/prepare",
            json!({"repoPath": "/repo", "remote": "upstream", "baseBranch": "main"}),
        )
        .await,
        json!({
            "status": "ok",
            "repoRoot": "/repo",
            "sourceRef": "upstream/main",
            "qualifiedSourceRef": "refs/remotes/upstream/main",
        })
    );
}

#[tokio::test]
async fn removed_memory_routes_are_unknown() {
    for path in [
        "/memory/status",
        "/memory/capture",
        "/memory/resolve-workspace",
        "/memory/query",
    ] {
        let result = handle_bridge_request(test_context(), path, json!({})).await;
        assert_eq!(result["status"], "failed", "{path}");
        assert_eq!(result["message"], "Unknown bridge path", "{path}");
    }
}

#[tokio::test]
async fn unknown_bridge_path_preserves_empty_session_id_shape() {
    let result = handle_bridge_request(
        test_context(),
        "/missing",
        json!({"session_id": "should-not-leak"}),
    )
    .await;

    assert_eq!(
        result,
        json!({
            "status": "failed",
            "session_id": "",
            "message": "Unknown bridge path"
        })
    );
}

#[tokio::test]
async fn settings_routes_use_settings_service() {
    let ctx = test_context();

    let updated = handle_bridge_request(
        ctx.clone(),
        "/settings/set",
        json!({"providerSyncEnabled": true, "codexAppSessionDelete": false, "codexAppServiceTierControls": true, "cliWrapperApiKeyEnv": ""}),
    )
    .await;
    let loaded = handle_bridge_request(ctx, "/settings/get", json!({})).await;

    assert_eq!(updated["providerSyncEnabled"], true);
    assert_eq!(updated["codexAppSessionDelete"], false);
    assert_eq!(updated["codexAppServiceTierControls"], true);
    assert_eq!(updated["cliWrapperApiKeyEnv"], "CUSTOM_OPENAI_API_KEY");
    assert_eq!(loaded, updated);
}

#[tokio::test]
async fn runtime_routes_keep_user_script_inventory_shape() {
    let ctx = test_context();

    let listed = handle_bridge_request(ctx.clone(), "/user-scripts/list", json!({})).await;
    let global = handle_bridge_request(
        ctx.clone(),
        "/user-scripts/set-enabled",
        json!({"enabled": false}),
    )
    .await;
    let script = handle_bridge_request(
        ctx.clone(),
        "/user-scripts/set-script-enabled",
        json!({"key": "user:a.js", "enabled": false}),
    )
    .await;
    let reloaded = handle_bridge_request(ctx, "/user-scripts/reload", json!({})).await;

    assert_eq!(listed["enabled"], true);
    assert_eq!(listed["scripts"][0]["key"], "builtin:demo.js");
    assert_eq!(global["enabled"], false);
    assert_eq!(script["scripts"][1]["enabled"], false);
    assert_eq!(reloaded["reloaded"], true);
    assert_eq!(reloaded["scripts"][0]["key"], "builtin:demo.js");
}

#[tokio::test]
async fn runtime_status_devtools_repair_and_ads_routes_are_dispatched() {
    let ctx = test_context();

    assert_eq!(
        handle_bridge_request(ctx.clone(), "/devtools/open", json!({})).await,
        json!({"status": "ok", "opened": true})
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/manager/open", json!({})).await,
        json!({"status": "ok", "opened": "manager"})
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/backend/status", json!({})).await,
        json!({"status": "ok", "message": "后端已连接", "version": claude_codex_pro_core::version::VERSION})
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/backend/repair", json!({})).await,
        json!({"status": "ok", "message": "后端已修复", "version": claude_codex_pro_core::version::VERSION})
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/claude-desktop/status", json!({})).await,
        json!({
            "status": "ok",
            "processCount": 1,
            "installKind": "msix",
            "cdpStatus": "blocked",
            "cdpBlocker": "CLAUDE_CDP_AUTH required",
            "supportedIntegration": "external_automation",
            "integrityStatus": "ok",
            "integrityMessage": "Audited 1 Claude Desktop executable path without modifying app files.",
            "executableAudits": [{
                "path": "C:\\Program Files\\WindowsApps\\Claude_1.0\\app\\Claude.exe",
                "exists": true,
                "fileSizeBytes": 1024,
                "modifiedUnixMs": 123456,
                "sha256": "abc123",
                "peFormat": "pe32_plus",
                "peMachine": "x64",
                "peSubsystem": "windows_gui",
                "peTimestampUnix": 1700000000,
                "peEntryPointRva": 4096,
                "peImageBase": 5368709120_u64,
                "peSectionCount": 1,
                "peCertificateTableBytes": 512,
                "peSections": [{
                    "name": ".text",
                    "virtualAddress": 4096,
                    "virtualSize": 8192,
                    "rawSize": 4096,
                    "rawSha256": "section123",
                    "characteristics": "0x60000020"
                }],
                "signatureStatus": "Valid",
                "signatureMessage": "Signature verified",
                "signerSubject": "CN=Anthropic",
                "signerIssuer": "CN=Trusted Root",
                "signerThumbprint": "thumbprint123",
                "signerSerialNumber": "serial123",
                "signerNotBefore": "2026-01-01T00:00:00.0000000Z",
                "signerNotAfter": "2027-01-01T00:00:00.0000000Z",
                "signerChainStatus": "",
                "productName": "Claude",
                "fileDescription": "Claude",
                "fileVersion": "1.12603.1.0",
                "originalFilename": "Claude.exe",
                "installKind": "msix",
                "trustBasis": "msix_protected_install_location_and_valid_authenticode",
                "integrityLevel": "executable_hash_authenticode_pe_header_section_audit",
                "riskLevel": "controlled",
                "verificationScope": "sha256_file_hash_authenticode_certificate_window_version_resource_pe_header_section_hashes_install_path",
                "mutationPolicy": "blocked_no_executable_asar_signature_or_integrity_metadata_changes",
                "patchEligible": false,
                "notes": ["Read-only audit only"]
            }]
        })
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/claude-desktop/integrity", json!({})).await,
        json!({
            "status": "ok",
            "message": "Audited 1 Claude Desktop executable path without modifying app files.",
            "policy": "read_only_audit_no_executable_or_asar_patch",
            "executableAudits": [{
                "path": "C:\\Program Files\\WindowsApps\\Claude_1.0\\app\\Claude.exe",
                "exists": true,
                "fileSizeBytes": 1024,
                "modifiedUnixMs": 123456,
                "sha256": "abc123",
                "peFormat": "pe32_plus",
                "peMachine": "x64",
                "peSubsystem": "windows_gui",
                "peTimestampUnix": 1700000000,
                "peEntryPointRva": 4096,
                "peImageBase": 5368709120_u64,
                "peSectionCount": 1,
                "peCertificateTableBytes": 512,
                "peSections": [{
                    "name": ".text",
                    "virtualAddress": 4096,
                    "virtualSize": 8192,
                    "rawSize": 4096,
                    "rawSha256": "section123",
                    "characteristics": "0x60000020"
                }],
                "signatureStatus": "Valid",
                "signatureMessage": "Signature verified",
                "signerSubject": "CN=Anthropic",
                "signerIssuer": "CN=Trusted Root",
                "signerThumbprint": "thumbprint123",
                "signerSerialNumber": "serial123",
                "signerNotBefore": "2026-01-01T00:00:00.0000000Z",
                "signerNotAfter": "2027-01-01T00:00:00.0000000Z",
                "signerChainStatus": "",
                "productName": "Claude",
                "fileDescription": "Claude",
                "fileVersion": "1.12603.1.0",
                "originalFilename": "Claude.exe",
                "installKind": "msix",
                "trustBasis": "msix_protected_install_location_and_valid_authenticode",
                "integrityLevel": "executable_hash_authenticode_pe_header_section_audit",
                "riskLevel": "controlled",
                "verificationScope": "sha256_file_hash_authenticode_certificate_window_version_resource_pe_header_section_hashes_install_path",
                "mutationPolicy": "blocked_no_executable_asar_signature_or_integrity_metadata_changes",
                "patchEligible": false,
                "notes": ["Read-only audit only"]
            }]
        })
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/claude-desktop/focus", json!({})).await,
        json!({
            "status": "ok",
            "message": "focused",
            "processId": 1234,
            "action": "focus",
            "foregroundVerified": true,
            "foregroundProcessId": 1234,
            "foregroundTitle": "Claude"
        })
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/claude-desktop/verify", json!({})).await,
        json!({
            "status": "ok",
            "message": "verified",
            "processId": 1234,
            "action": "verify_target",
            "foregroundVerified": true,
            "foregroundProcessId": 1234,
            "foregroundTitle": "Claude"
        })
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/claude-desktop/open", json!({})).await,
        json!({
            "status": "accepted",
            "message": "launch requested",
            "processId": null,
            "action": "open",
            "foregroundVerified": false,
            "foregroundProcessId": null,
            "foregroundTitle": null
        })
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/claude-desktop/new-chat", json!({})).await,
        json!({
            "status": "ok",
            "message": "new chat",
            "processId": 1234,
            "action": "new_chat",
            "foregroundVerified": true,
            "foregroundProcessId": 1234,
            "foregroundTitle": "Claude"
        })
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/claude-desktop/paste-draft",
            json!({"text": "hello Claude"}),
        )
        .await,
        json!({
            "status": "ok",
            "message": "draft pasted",
            "processId": 1234,
            "action": "paste_draft",
            "inputChars": 12,
            "autoSubmitted": false,
            "foregroundVerified": true,
            "foregroundProcessId": 1234,
            "foregroundTitle": "Claude"
        })
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/claude-desktop/submit",
            json!({"text": "hello Claude"}),
        )
        .await,
        json!({
            "status": "ok",
            "message": "submitted",
            "processId": 1234,
            "action": "paste_and_submit",
            "inputChars": 12,
            "autoSubmitted": true,
            "foregroundVerified": true,
            "foregroundProcessId": 1234,
            "foregroundTitle": "Claude"
        })
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/ads", json!({})).await,
        json!({"version": 1, "ads": [{"id": "runtime-ad"}]})
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/zed-remote/status", json!({})).await,
        json!({"status": "ok", "platformSupported": true, "zedAppFound": true, "zedCliFound": false})
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/zed-remote/resolve-host",
            json!({"hostId": "remote-ssh-codex-managed:remote"}),
        )
        .await,
        json!({"status": "ok", "ssh": {"user": "longnv", "host": "192.168.100.31", "port": null}})
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/zed-remote/fallback-request",
            json!({"hostId": "remote-ssh-codex-managed:remote"}),
        )
        .await,
        json!({
            "status": "ok",
            "request": {
                "hostId": "remote-ssh-codex-managed:remote",
                "ssh": {"user": "longnv", "host": "192.168.100.31", "port": null},
                "path": "/Users/longnv/bin/repo/sealos-skills",
            }
        })
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/zed-remote/open",
            json!({"ssh": {"host": "example.com"}, "path": "/home/app.py"}),
        )
        .await,
        json!({"status": "ok", "url": "ssh://example.com/home/app.py", "strategy": "addToFocusedWorkspace"})
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/zed-remote/projects", json!({})).await,
        json!({
            "status": "ok",
            "projects": [{
                "id": "zed-remote-project:test",
                "label": "sealos-skills",
                "hostId": "remote-ssh-codex-managed:remote",
                "ssh": {"user": "longnv", "host": "192.168.100.31", "port": null},
                "path": "/Users/longnv/bin/repo/sealos-skills",
                "url": "ssh://longnv@192.168.100.31/Users/longnv/bin/repo/sealos-skills",
                "source": "codexRemoteProject",
                "lastOpenedAtMs": null,
                "isCurrent": false
            }]
        })
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/zed-remote/remember-project",
            json!({"ssh": {"host": "example.com"}, "path": "/home/app.py"}),
        )
        .await,
        json!({"status": "ok", "remembered": true})
    );
    assert_eq!(
        handle_bridge_request(
            ctx,
            "/zed-remote/forget-project",
            json!({"id": "zed-remote-project:test"}),
        )
        .await,
        json!({"status": "ok", "removed": 1})
    );
}

#[tokio::test]
async fn data_routes_forward_payloads_to_data_service() {
    let ctx = test_context();

    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/delete",
            json!({"session_id": "s1", "title": "First"}),
        )
        .await["undo_token"],
        "undo-s1"
    );
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/undo", json!({"undo_token": "undo-s1"})).await,
        json!({
            "status": "undone",
            "session_id": "s1",
            "message": "undone",
            "undo_token": "undo-s1",
            "backup_path": null
        })
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/export-markdown",
            json!({"session_id": "s1", "title": "First"}),
        )
        .await["filename"],
        "First.md"
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/thread-usage-history",
            json!({"session_id": "s1", "title": "First"}),
        )
        .await,
        json!({
            "status": "ok",
            "session_id": "s1",
            "history": [
                {
                    "source": "rollout-history",
                    "conversation_id": "local:s1",
                    "turn_id": "turn-1",
                    "observed_at": "2026-06-02T05:00:00Z",
                    "usage": {
                        "inputTokens": 1200,
                        "outputTokens": 120,
                        "totalTokens": 1320,
                        "cachedTokens": 900,
                        "cacheReadTokens": 0,
                        "cacheCreationTokens": 0,
                        "contextUsed": 1320,
                        "contextLimit": 258400,
                        "hasBreakdown": true
                    }
                }
            ]
        })
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/archived-thread",
            json!({"title": "Archived"})
        )
        .await,
        json!({"session_id": "archived-1", "title": "Archived"})
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/move-thread-workspace",
            json!({"session_id": "s1", "title": "First", "target_cwd": "/new"}),
        )
        .await,
        json!({"status": "moved", "session_id": "s1", "target_cwd": "/new"})
    );
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/thread-sort-key",
            json!({"session_id": "s1", "title": "First"}),
        )
        .await,
        json!({"status": "ok", "session_id": "s1", "updated_at": 123})
    );
    assert_eq!(
        handle_bridge_request(
            ctx,
            "/thread-sort-keys",
            json!({"sessions": [{"session_id": "s1", "title": "First"}, null, {"session_id": "s2"}]}),
        )
        .await,
        json!({"status": "ok", "sort_keys": [{"session_id": "s1"}, {"session_id": "s2"}]})
    );
}

#[tokio::test]
async fn bridge_context_core_with_data_uses_injected_data_service() {
    let ctx = BridgeContext::core_with_data(
        Arc::new(CoreRuntimeService::new(9229, StatusStore::default())),
        Arc::new(FakeData::default()),
    );

    let result = handle_bridge_request(
        ctx,
        "/delete",
        json!({"session_id": "s1", "title": "First"}),
    )
    .await;

    assert_eq!(result["status"], "local_deleted");
    assert_eq!(result["undo_token"], "undo-s1");
    assert_ne!(
        result["message"],
        "Delete service is not wired in core launcher hooks"
    );
}

#[tokio::test]
async fn user_script_manager_scans_and_persists_inventory_shape() {
    let temp = tempfile::tempdir().unwrap();
    let builtin_dir = temp.path().join("builtin");
    let user_dir = temp.path().join("user");
    std::fs::create_dir_all(&builtin_dir).unwrap();
    std::fs::write(builtin_dir.join("demo.js"), "window.demo = true;").unwrap();
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::write(user_dir.join("a.js"), "window.a = true;").unwrap();
    std::fs::write(user_dir.join("ignore.txt"), "not js").unwrap();
    let manager = UserScriptManager::new(
        builtin_dir.clone(),
        user_dir.clone(),
        temp.path().join("user_scripts.json"),
    );

    let listed = manager.inventory().unwrap();
    manager.set_global_enabled(false).unwrap();
    let disabled = manager.inventory().unwrap();
    manager.set_script_enabled("user:a.js", false).unwrap();
    let script_disabled = manager.inventory().unwrap();
    manager.delete_user_script("user:a.js").unwrap();
    let deleted = manager.inventory().unwrap();

    assert_eq!(listed["enabled"], true);
    assert_eq!(
        listed["builtin_dir"].as_str().unwrap(),
        builtin_dir.to_string_lossy()
    );
    assert_eq!(
        listed["user_dir"].as_str().unwrap(),
        user_dir.to_string_lossy()
    );
    assert_eq!(listed["scripts"][0]["key"], "builtin:demo.js");
    assert_eq!(listed["scripts"][0]["source"], "builtin");
    assert_eq!(listed["scripts"][0]["enabled"], true);
    assert_eq!(listed["scripts"][0]["status"], "not_loaded");
    assert_eq!(listed["scripts"][0]["error"], "");
    assert_eq!(listed["scripts"][1]["key"], "user:a.js");
    assert_eq!(disabled["enabled"], false);
    assert_eq!(disabled["scripts"][0]["status"], "disabled");
    assert_eq!(script_disabled["scripts"][1]["enabled"], false);
    assert_eq!(deleted["scripts"].as_array().unwrap().len(), 1);
    assert!(!user_dir.join("a.js").exists());
    assert_eq!(
        serde_json::from_str::<Value>(
            &std::fs::read_to_string(temp.path().join("user_scripts.json")).unwrap()
        )
        .unwrap(),
        json!({"enabled": false, "scripts": {}})
    );
}

#[tokio::test]
async fn user_script_manager_deletes_market_script_metadata_and_rejects_builtin_delete() {
    let temp = tempfile::tempdir().unwrap();
    let builtin_dir = temp.path().join("builtin");
    let user_dir = temp.path().join("user");
    std::fs::create_dir_all(&builtin_dir).unwrap();
    std::fs::write(builtin_dir.join("demo.js"), "window.demo = true;").unwrap();
    std::fs::create_dir_all(&user_dir).unwrap();
    let manager = UserScriptManager::new(
        builtin_dir,
        user_dir.clone(),
        temp.path().join("user_scripts.json"),
    );
    let script = claude_codex_pro_core::script_market::MarketScript {
        id: "demo".to_string(),
        name: "Demo".to_string(),
        description: String::new(),
        version: "1.0.0".to_string(),
        author: String::new(),
        tags: Vec::new(),
        homepage: "https://example.com/demo".to_string(),
        script_url: "https://example.com/demo.js".to_string(),
        sha256: String::new(),
    };

    claude_codex_pro_core::script_market::install_market_script_content(
        &manager,
        &script,
        b"window.demo = true;",
    )
    .unwrap();
    manager
        .set_script_enabled("user:market-demo.js", false)
        .unwrap();

    let error = manager.delete_user_script("builtin:demo.js").unwrap_err();
    assert!(error.to_string().contains("only user scripts"));
    manager.delete_user_script("user:market-demo.js").unwrap();

    assert!(!user_dir.join("market-demo.js").exists());
    assert!(
        manager.inventory().unwrap()["scripts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|script| script["market_id"] != "demo")
    );
    let saved = serde_json::from_str::<Value>(
        &std::fs::read_to_string(temp.path().join("user_scripts.json")).unwrap(),
    )
    .unwrap();
    assert!(saved.get("market").is_none());
    assert_eq!(saved["scripts"], json!({}));
}

#[tokio::test]
async fn core_runtime_reload_evaluates_enabled_user_bundle_and_status_is_ok() {
    let temp = tempfile::tempdir().unwrap();
    let builtin_dir = temp.path().join("builtin");
    std::fs::create_dir_all(&builtin_dir).unwrap();
    std::fs::write(builtin_dir.join("demo.js"), "window.demo = true;").unwrap();
    let manager = UserScriptManager::new(
        builtin_dir,
        temp.path().join("user"),
        temp.path().join("user_scripts.json"),
    );
    let evaluated = Arc::new(Mutex::new(Vec::<String>::new()));
    let runtime = CoreRuntimeService::new(9229, StatusStore::default())
        .with_user_scripts(manager)
        .with_user_script_evaluator({
            let evaluated = evaluated.clone();
            Arc::new(move |websocket_url, script| {
                evaluated
                    .lock()
                    .unwrap()
                    .push(format!("{websocket_url}:{script}"));
                Ok(json!({"status": "ok"}))
            })
        })
        .with_websocket_url("ws://page");
    let ctx = BridgeContext::core_with_data(Arc::new(runtime), Arc::new(FakeData::default()));

    let status = handle_bridge_request(ctx.clone(), "/backend/status", json!({})).await;
    let repaired = handle_bridge_request(ctx.clone(), "/backend/repair", json!({})).await;
    let reloaded = handle_bridge_request(ctx, "/user-scripts/reload", json!({})).await;

    assert_eq!(
        status,
        json!({"status": "ok", "message": "后端已连接", "version": claude_codex_pro_core::version::VERSION})
    );
    assert_eq!(
        repaired,
        json!({"status": "ok", "message": "后端已连接", "version": claude_codex_pro_core::version::VERSION})
    );
    assert_eq!(reloaded["scripts"][0]["key"], "builtin:demo.js");
    let evaluated = evaluated.lock().unwrap();
    assert_eq!(evaluated.len(), 1);
    assert!(evaluated[0].starts_with("ws://page:"));
    assert!(evaluated[0].contains("window.demo = true;"));
}

#[tokio::test]
async fn core_runtime_open_devtools_uses_inspector_url_opener() {
    let opened = Arc::new(Mutex::new(Vec::<String>::new()));
    let runtime = CoreRuntimeService::new(9229, StatusStore::default())
        .with_devtools_opener({
            let opened = opened.clone();
            Arc::new(move |url| {
                opened.lock().unwrap().push(url.to_string());
                Ok(())
            })
        })
        .with_devtools_target_id("page-1");
    let ctx = BridgeContext::core_with_data(Arc::new(runtime), Arc::new(FakeData::default()));

    let result = handle_bridge_request(ctx, "/devtools/open", json!({})).await;

    assert_eq!(result["status"], "ok");
    assert_eq!(result["target_id"], "page-1");
    assert_eq!(
        opened.lock().unwrap().as_slice(),
        ["http://127.0.0.1:9229/devtools/inspector.html?ws=127.0.0.1:9229/devtools/page/page-1"]
    );
}

#[tokio::test]
async fn core_runtime_manager_route_attempts_to_open_manager_binary() {
    let ctx = BridgeContext::core(Arc::new(CoreRuntimeService::new(
        9229,
        StatusStore::default(),
    )));

    let result = handle_bridge_request(ctx, "/manager/open", json!({})).await;

    assert_ne!(result["message"], "管理工具启动未接入当前运行时");
}

#[tokio::test]
async fn bridge_backend_status_writes_diagnostic_log() {
    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("claude-codex-pro.log");
    claude_codex_pro_core::diagnostic_log::set_diagnostic_log_path_for_tests(Some(
        log_path.clone(),
    ));
    let ctx = BridgeContext::core(Arc::new(CoreRuntimeService::new(
        9229,
        StatusStore::default(),
    )));

    let result = handle_bridge_request(ctx, "/backend/status", json!({})).await;

    assert_eq!(result["status"], "ok");
    let contents = std::fs::read_to_string(&log_path).unwrap();
    assert!(contents.contains("bridge.request"));
    assert!(contents.contains("bridge.backend_status_ok"));
    assert!(contents.contains("/backend/status"));
    claude_codex_pro_core::diagnostic_log::set_diagnostic_log_path_for_tests(None);
}

#[test]
fn user_script_manager_tolerates_bad_config_fields_and_updates_atomically() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("user_scripts.json");
    std::fs::write(
        &config_path,
        r#"{"enabled":"not bool","scripts":{"user:a.js":false,"user:b.js":"bad"},"custom":true}"#,
    )
    .unwrap();
    let manager = UserScriptManager::new(
        temp.path().join("builtin"),
        temp.path().join("user"),
        config_path.clone(),
    );

    assert_eq!(manager.load_config().enabled, true);
    assert_eq!(manager.load_config().scripts.get("user:a.js"), Some(&false));
    assert!(!manager.load_config().scripts.contains_key("user:b.js"));

    manager.set_script_enabled("user:c.js", false).unwrap();
    let saved = serde_json::from_str::<Value>(&std::fs::read_to_string(config_path).unwrap())
        .expect("config should remain valid JSON");

    assert_eq!(saved["enabled"], true);
    assert_eq!(saved["scripts"]["user:a.js"], false);
    assert_eq!(saved["scripts"]["user:c.js"], false);
}

#[test]
fn script_market_manifest_filters_invalid_entries() {
    let raw = serde_json::json!({
        "version": 1,
        "updated_at": "2026-05-21T00:00:00Z",
        "scripts": [
            {
                "id": "demo",
                "name": "Demo",
                "description": "Useful demo",
                "version": "1.0.0",
                "author": "DamonZS",
                "tags": ["ui", 42],
                "homepage": "https://example.com/demo",
                "script_url": "https://example.com/demo.js",
                "sha256": ""
            },
            { "id": "", "name": "Bad", "version": "1", "script_url": "https://example.com/bad.js" },
            { "id": "missing-url", "name": "Bad", "version": "1" }
        ]
    });

    let manifest = claude_codex_pro_core::script_market::parse_market_manifest(raw).unwrap();

    assert_eq!(manifest.version, 1);
    assert_eq!(manifest.updated_at.as_deref(), Some("2026-05-21T00:00:00Z"));
    assert_eq!(manifest.scripts.len(), 1);
    assert_eq!(manifest.scripts[0].id, "demo");
    assert_eq!(manifest.scripts[0].tags, vec!["ui"]);
}

#[test]
fn user_script_inventory_includes_market_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let user_dir = temp.path().join("user");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::write(user_dir.join("market-demo.js"), "window.demo = true;").unwrap();
    let manager = UserScriptManager::new(
        temp.path().join("builtin"),
        user_dir,
        temp.path().join("user_scripts.json"),
    );

    manager
        .record_market_install(&claude_codex_pro_core::script_market::MarketScript {
            id: "demo".to_string(),
            name: "Demo".to_string(),
            description: "Useful demo".to_string(),
            version: "1.0.0".to_string(),
            author: "DamonZS".to_string(),
            tags: vec!["ui".to_string()],
            homepage: "https://example.com/demo".to_string(),
            script_url: "https://example.com/demo.js".to_string(),
            sha256: String::new(),
        })
        .unwrap();

    let inventory = manager.inventory().unwrap();

    assert_eq!(inventory["scripts"][0]["key"], "user:market-demo.js");
    assert_eq!(inventory["scripts"][0]["market_id"], "demo");
    assert_eq!(inventory["scripts"][0]["version"], "1.0.0");
    assert_eq!(inventory["scripts"][0]["installed"], true);
    assert_eq!(
        inventory["scripts"][0]["source_url"],
        "https://example.com/demo.js"
    );
    assert_eq!(
        inventory["scripts"][0]["homepage"],
        "https://example.com/demo"
    );
}

#[test]
fn install_market_script_writes_file_and_records_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let manager = UserScriptManager::new(
        temp.path().join("builtin"),
        temp.path().join("user"),
        temp.path().join("user_scripts.json"),
    );
    let script = claude_codex_pro_core::script_market::MarketScript {
        id: "demo".to_string(),
        name: "Demo".to_string(),
        description: String::new(),
        version: "1.0.0".to_string(),
        author: String::new(),
        tags: Vec::new(),
        homepage: "https://example.com/demo".to_string(),
        script_url: "https://example.com/demo.js".to_string(),
        sha256: String::new(),
    };

    claude_codex_pro_core::script_market::install_market_script_content(
        &manager,
        &script,
        b"window.demo = true;",
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(temp.path().join("user").join("market-demo.js")).unwrap(),
        "window.demo = true;"
    );
    let inventory = manager.inventory().unwrap();
    assert_eq!(inventory["scripts"][0]["market_id"], "demo");
}

#[test]
fn install_market_script_rejects_checksum_mismatch_and_preserves_existing_file() {
    let temp = tempfile::tempdir().unwrap();
    let user_dir = temp.path().join("user");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::write(user_dir.join("market-demo.js"), "old").unwrap();
    let manager = UserScriptManager::new(
        temp.path().join("builtin"),
        user_dir.clone(),
        temp.path().join("user_scripts.json"),
    );
    let script = claude_codex_pro_core::script_market::MarketScript {
        id: "demo".to_string(),
        name: "Demo".to_string(),
        description: String::new(),
        version: "1.0.0".to_string(),
        author: String::new(),
        tags: Vec::new(),
        homepage: String::new(),
        script_url: "https://example.com/demo.js".to_string(),
        sha256: "0".repeat(64),
    };

    let result = claude_codex_pro_core::script_market::install_market_script_content(
        &manager, &script, b"new",
    );

    assert!(result.is_err());
    assert_eq!(
        std::fs::read_to_string(user_dir.join("market-demo.js")).unwrap(),
        "old"
    );
}

#[test]
fn install_market_script_accepts_matching_checksum() {
    let temp = tempfile::tempdir().unwrap();
    let manager = UserScriptManager::new(
        temp.path().join("builtin"),
        temp.path().join("user"),
        temp.path().join("user_scripts.json"),
    );
    let content = b"new";
    let script = claude_codex_pro_core::script_market::MarketScript {
        id: "demo".to_string(),
        name: "Demo".to_string(),
        description: String::new(),
        version: "1.0.0".to_string(),
        author: String::new(),
        tags: Vec::new(),
        homepage: String::new(),
        script_url: "https://example.com/demo.js".to_string(),
        sha256: format!("sha256:{:x}", sha2::Sha256::digest(content)),
    };

    claude_codex_pro_core::script_market::install_market_script_content(&manager, &script, content)
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(temp.path().join("user").join("market-demo.js")).unwrap(),
        "new"
    );
}

#[tokio::test]
async fn launch_lifecycle_uses_hook_supplied_bridge_context_for_injection() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("Codex.app");
    std::fs::create_dir_all(&app_dir).unwrap();
    let events = Arc::new(Mutex::new(Vec::<String>::new()));
    let hooks = ContextHooks {
        events: events.clone(),
    };

    launch_and_inject_with_hooks(
        LaunchOptions {
            app_dir: Some(app_dir),
            debug_port: 9229,
            helper_port: 57321,
            status_store: StatusStore::new(temp.path().join("latest-status.json")),
        },
        &hooks,
    )
    .await
    .unwrap();

    assert_eq!(
        *events.lock().unwrap(),
        vec![
            "status:running_degraded",
            "bridge-context:9229",
            "inject-bridge:9229:57321",
            "watchdog:9229:57321",
            "status:running",
        ]
    );
}

async fn metadata_route_write(
    ctx: &BridgeContext,
    resource: &str,
    entity: Value,
    revision: u64,
) -> Value {
    let response = handle_bridge_request(
        ctx.clone(),
        "/multica/workspace/upsert",
        json!({
            "resource":resource, "entity":entity, "expectedRevision":revision,
        }),
    )
    .await;
    assert_eq!(response["status"], "ok", "{response}");
    response["entity"].clone()
}

#[tokio::test]
async fn metadata_bridge_resources_reorder_and_command_recovery_are_real_store_operations() {
    let (ctx, transport, dir) = multica_execution_test_context();
    let agent = metadata_route_write(&ctx, "agents", json!({"id":"agent","name":"Agent"}), 0).await;
    let workspace_id = agent["workspace_id"].as_str().unwrap();
    let property = metadata_route_write(
        &ctx,
        "properties",
        json!({"id":"cost","name":"Cost","type":"number"}),
        0,
    )
    .await;
    assert_eq!(property["archived"], false);
    metadata_route_write(
        &ctx,
        "issue_view_preferences",
        json!({"id":"prefs","scope_type":"my","prefs":{"hidden":[],"order":["builtin:all"]}}),
        0,
    )
    .await;
    metadata_route_write(&ctx,"quick_actions",json!({"id":"quick","name":"Review","prompt":"Review the issue","assignee_type":"agent","assignee_id":"agent"}),0).await;
    metadata_route_write(
        &ctx,
        "issues",
        json!({"id":"issue","properties":{"cost":3}}),
        0,
    )
    .await;
    metadata_route_write(
        &ctx,
        "reactions",
        json!({"id":"reaction","issue_id":"issue","emoji":"+1"}),
        0,
    )
    .await;
    metadata_route_write(&ctx,"issue_statuses",json!({"id":"ready","name":"Ready","key":"ready","category":"todo","color":"#123456","position":4,"is_system":false}),0).await;
    let payload = json!({"category":"todo","ids":["ready"],"expectedRevisions":{"ready":1},"commandId":"reorder","commandSignature":"a".repeat(64)});
    let result = handle_bridge_request(
        ctx.clone(),
        "/multica/workspace/reorder-statuses",
        payload.clone(),
    )
    .await;
    assert_eq!(result["status"], "ok", "{result}");
    assert_eq!(result["statuses"][0]["revision"], 2);
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/multica/workspace/reorder-statuses", payload).await,
        result
    );
    let receipt = handle_bridge_request(
        ctx.clone(),
        "/multica/workspace/command",
        json!({"commandId":"reorder","commandSignature":"a".repeat(64)}),
    )
    .await;
    assert_eq!(receipt["result"], result);
    let invalid = handle_bridge_request(ctx.clone(),"/multica/workspace/upsert",json!({"resource":"issues","entity":{"id":"issue","properties":{"cost":"bad"}},"expectedRevision":1})).await;
    assert_eq!(
        invalid["message"],
        "multica_workspace_property_value_invalid"
    );
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    let state = store.load(workspace_id).unwrap();
    assert_eq!(state.properties.len(), 1);
    assert_eq!(state.quick_actions.len(), 1);
    assert_eq!(
        state.issue_view_preferences[0]["user_id"],
        format!("{workspace_id}-user")
    );
    assert!(transport.calls().is_empty());
}

#[tokio::test]
async fn metadata_bridge_builder_uses_attached_transport_for_native_turn_and_transcript() {
    let (ctx, transport, _dir) = multica_execution_test_context();
    let created = handle_bridge_request(ctx.clone(),"/multica/builder",json!({"operation":"create","runtimeId":"codex-current-page","idempotencyKey":"builder-create"})).await;
    assert!(created["session_id"].is_string(), "{created}");
    let id = created["session_id"].clone();
    let saved = handle_bridge_request(ctx.clone(),"/multica/builder",json!({"operation":"save_draft","sessionId":id,"expectedRevision":1,"draft":{"name":"Reviewer","permission_scope":"private"}})).await;
    assert_eq!(saved["revision"], 2, "{saved}");
    let request = json!({"operation":"send","sessionId":id,"expectedRevision":2,"content":"Define a review agent","idempotencyKey":"builder-send"});
    let sent = handle_bridge_request(ctx.clone(), "/multica/builder", request.clone()).await;
    assert!(sent["native_thread_id"].is_string(), "{sent}");
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/multica/builder", request).await,
        sent
    );
    transport.push_response(CodexPageHostMethod::ThreadRead,Ok(json!({"thread":{"id":sent["native_thread_id"],"turns":[{"id":sent["native_turn_id"],"status":"completed","items":[{"id":"answer","type":"agentMessage","text":"Review changes carefully."}]}]}})));
    let messages = handle_bridge_request(
        ctx.clone(),
        "/multica/builder",
        json!({"operation":"messages","sessionId":id}),
    )
    .await;
    assert!(messages.is_array(), "{messages}");
    assert!(
        messages
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["content"] == "Review changes carefully.")
    );
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|call| call.method == CodexPageHostMethod::ThreadStart)
            .count(),
        1
    );
    let forbidden = handle_bridge_request(
        ctx,
        "/multica/builder",
        json!({"operation":"get","sessionId":id,"userId":"other"}),
    )
    .await;
    assert_eq!(forbidden["message"], "builder_request_invalid");
}

#[tokio::test]
async fn metadata_bridge_webhook_tick_recovers_received_delivery_and_dispatches_once() {
    let (ctx, transport, dir) = multica_execution_test_context();
    let agent = metadata_route_write(&ctx, "agents", json!({"id":"agent","name":"Agent"}), 0).await;
    let workspace_id = agent["workspace_id"].as_str().unwrap();
    let autopilot = metadata_route_write(&ctx,"autopilots",json!({"id":"pilot","title":"Hook","description":"Review updates","assignee_type":"agent","assignee_id":"agent","execution_mode":"run_only","triggers":[{"id":"trigger","kind":"webhook","enabled":true}]}),0).await;
    let provisioned = handle_bridge_request(
        ctx.clone(),
        "/multica/webhooks/provision",
        json!({"autopilotId":"pilot","triggerId":"trigger","commandId":"provision"}),
    )
    .await;
    let token = provisioned["webhook_token"]
        .as_str()
        .expect("one-time token");
    let store = MulticaWebhookStore::new(dir.path().join("webhooks.json"));
    let target = WebhookTarget::from_autopilot(workspace_id, &autopilot, "trigger").unwrap();
    let headers = std::collections::BTreeMap::from([
        ("Authorization".into(), format!("Bearer {token}")),
        ("Idempotency-Key".into(), "delivery".into()),
        ("Content-Type".into(), "application/json".into()),
    ]);
    let received = store.receive(&target, &headers, b"{}", 100).unwrap();
    assert!(received.delivery["autopilot_run_id"].is_null());
    let tick = handle_bridge_request(ctx.clone(), "/multica/autopilots/tick", json!({})).await;
    assert_eq!(tick["status"], "ok", "{tick}");
    assert_eq!(tick["runs"].as_array().unwrap().len(), 1);
    assert!(transport.calls().is_empty());
    let delivery = store
        .get(
            workspace_id,
            "pilot",
            received.delivery["id"].as_str().unwrap(),
        )
        .unwrap();
    assert_eq!(delivery["status"], "dispatched");
    let execution = MulticaExecutionStore::new(dir.path().join("multica-execution.json"));
    let run = execution
        .get_autopilot_run(delivery["autopilot_run_id"].as_str().unwrap())
        .unwrap();
    let binding = execution
        .get_execution(run.task_id.as_ref().unwrap())
        .unwrap();
    let dispatched = handle_bridge_request(ctx.clone(),"/multica/executions/dispatch",json!({"bindingId":binding.binding_id,"expectedRevision":binding.revision,"leaseToken":"lease"})).await;
    assert_eq!(dispatched["status"], "ok", "{dispatched}");
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|call| call.method == CodexPageHostMethod::ThreadStart)
            .count(),
        1
    );
    assert!(
        handle_bridge_request(ctx.clone(), "/multica/autopilots/tick", json!({})).await["runs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let list = handle_bridge_request(
        ctx.clone(),
        "/multica/webhooks/deliveries",
        json!({"autopilotId":"pilot","limit":10,"offset":0}),
    )
    .await;
    assert_eq!(list["total"], 1);
    assert!(!serde_json::to_string(&list).unwrap().contains(token));
    let replay = handle_bridge_request(
        ctx.clone(),
        "/multica/webhooks/replay",
        json!({"autopilotId":"pilot","deliveryId":delivery["id"],"commandId":"replay"}),
    )
    .await;
    assert_eq!(replay["replayed_from_delivery_id"], delivery["id"]);
    assert_ne!(replay["id"], delivery["id"]);
    let trigger = handle_bridge_request(
        ctx.clone(),
        "/multica/webhooks/trigger",
        json!({"autopilotId":"pilot","triggerId":"trigger"}),
    )
    .await;
    assert!(trigger["webhook_token"].is_null());
    let rotated = handle_bridge_request(ctx,"/multica/webhooks/rotate",json!({"autopilotId":"pilot","triggerId":"trigger","expectedRevision":1,"commandId":"rotate"})).await;
    assert_eq!(rotated["credential_revision"], 2);
}

#[tokio::test]
async fn metadata_bridge_agent_permission_is_checked_before_native_creation() {
    let (ctx, transport, dir) = multica_execution_test_context();
    let agent = metadata_route_write(
        &ctx,
        "agents",
        json!({"id":"agent","name":"Agent","permission_mode":"private"}),
        0,
    )
    .await;
    let workspace_id = agent["workspace_id"].as_str().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    let mut state = store.load(workspace_id).unwrap();
    state.agents[0]["owner_id"] = json!("other");
    store.save(&state).unwrap();
    let response = handle_bridge_request(ctx,"/multica/executions/create",json!({"workspaceId":workspace_id,"issueId":"issue","agentId":"agent","prompt":"work","idempotencyKey":"denied","bindings":{}})).await;
    assert_eq!(response["message"], "multica_workspace_agent_access_denied");
    assert!(transport.calls().is_empty());
}

#[tokio::test]
async fn metadata_bridge_native_domains_use_real_usage_and_idempotent_native_actions() {
    let (ctx, transport, dir) = multica_execution_test_context();
    let agent = metadata_route_write(&ctx, "agents", json!({"id":"agent","name":"Agent"}), 0).await;
    metadata_route_write(&ctx, "issues", json!({"id":"issue","title":"Review me"}), 0).await;
    metadata_route_write(&ctx,"quick_actions",json!({"id":"quick","name":"Review","prompt":"Review the issue","assignee_type":"agent","assignee_id":"agent"}),0).await;
    let usage = handle_bridge_request(ctx.clone(), "/multica/issues/limit-usage", json!({})).await;
    assert!(usage["usage"].is_null());
    let usage = handle_bridge_request(ctx.clone(), "/multica/autopilots/usage", json!({})).await;
    assert_eq!(usage["usage"]["action"], "off");
    assert!(usage["usage"]["used"].is_null());
    let preview = handle_bridge_request(
        ctx.clone(),
        "/multica/issues/preview-trigger",
        json!({"issueIds":["issue"],"isCreate":false,"assigneeType":"agent","assigneeId":"agent"}),
    )
    .await;
    assert_eq!(preview["total_count"], 1, "{preview}");
    assert!(transport.calls().is_empty());
    let rendered = handle_bridge_request(
        ctx.clone(),
        "/multica/quick-actions/render",
        json!({"issueId":"issue","quickActionId":"quick"}),
    )
    .await;
    assert!(
        rendered["content"]
            .as_str()
            .unwrap()
            .contains("Review the issue")
    );
    let payload = json!({"issueId":"issue","quickActionId":"quick","expectedIssueRevision":1,"expectedActionRevision":1,"commandId":"run-quick","commandSignature":"a".repeat(64)});
    let result =
        handle_bridge_request(ctx.clone(), "/multica/quick-actions/run", payload.clone()).await;
    assert_eq!(result["type"], "comment", "{result}");
    assert!(result.get("native_prompt").is_none());
    assert_eq!(result["trigger_outcomes"][0]["status"], "queued");
    assert_eq!(
        handle_bridge_request(ctx.clone(), "/multica/quick-actions/run", payload.clone()).await,
        result
    );
    let recovered = handle_bridge_request(
        ctx.clone(),
        "/multica/workspace/command",
        json!({"commandId":"run-quick","commandSignature":"a".repeat(64)}),
    )
    .await;
    assert_eq!(recovered["result"], result);
    let mut stale = payload;
    stale["commandId"] = json!("stale");
    stale["expectedIssueRevision"] = json!(0);
    assert_eq!(
        handle_bridge_request(ctx, "/multica/quick-actions/run", stale).await["message"],
        "multica_workspace_revision_conflict"
    );
    let state = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"))
        .load(agent["workspace_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(state.comments.len(), 1);
    assert_eq!(state.quick_actions[0]["use_count"], 1);
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
async fn metadata_bridge_environment_rejects_nonempty_write_without_changing_agent() {
    let (ctx, transport, _dir) = multica_execution_test_context();
    metadata_route_write(&ctx, "agents", json!({"id":"agent","name":"Agent"}), 0).await;
    let payload = json!({"operation":"set","agentId":"agent","customEnv":{"LANG":"en_US.UTF-8"},"expectedRevision":1,"commandId":"env","commandSignature":"b".repeat(64)});
    let result = handle_bridge_request(ctx.clone(), "/multica/agents/env", payload).await;
    assert_eq!(
        result["message"], "execution_agent_environment_unsupported",
        "{result}"
    );
    let read = handle_bridge_request(
        ctx.clone(),
        "/multica/agents/env",
        json!({"operation":"get","agentId":"agent"}),
    )
    .await;
    assert_eq!(read["custom_env"], json!({}));
    assert_eq!(read["revision"], 1);
    let empty = json!({"operation":"set","agentId":"agent","customEnv":{},"expectedRevision":1,"commandId":"clear-env","commandSignature":"c".repeat(64)});
    let cleared = handle_bridge_request(ctx.clone(), "/multica/agents/env", empty.clone()).await;
    assert_eq!(cleared["custom_env"], json!({}), "{cleared}");
    assert_eq!(cleared["revision"], 2);
    assert_eq!(
        handle_bridge_request(ctx, "/multica/agents/env", empty).await,
        cleared
    );
    assert!(transport.calls().is_empty());
}

#[tokio::test]
async fn metadata_bridge_new_paths_reject_extra_fields_and_respect_disabled_workspace() {
    let (ctx, _, _dir) = multica_execution_test_context();
    for (path, payload) in [
        (
            "/multica/webhooks/provision",
            json!({"autopilotId":"pilot","triggerId":"trigger","commandId":"provision","url":"https://example.com"}),
        ),
        (
            "/multica/workspace/reorder-statuses",
            json!({"category":"todo","ids":[],"expectedRevisions":{},"commandId":"reorder","commandSignature":"a".repeat(64),"owner":"admin"}),
        ),
        (
            "/multica/builder",
            json!({"operation":"create","runtimeId":"codex-current-page","idempotencyKey":"builder","headers":{}}),
        ),
    ] {
        let response = handle_bridge_request(ctx.clone(), path, payload).await;
        assert_eq!(response["status"], "failed");
        assert_ne!(response["message"], "Unknown bridge path");
    }
    let settings = FakeSettings::default();
    settings.settings.lock().unwrap().multica_workspace_enabled = false;
    let disabled = BridgeContext::new(
        Arc::new(settings),
        Arc::new(FakeRuntime::default()),
        Arc::new(FakeData::default()),
    )
    .without_diagnostics();
    for path in [
        "/multica/builder",
        "/multica/webhooks/provision",
        "/multica/workspace/reorder-statuses",
        "/multica/agents/env",
        "/multica/issues/limit-usage",
        "/multica/autopilots/usage",
        "/multica/issues/preview-trigger",
        "/multica/quick-actions/render",
        "/multica/quick-actions/run",
    ] {
        assert_eq!(
            handle_bridge_request(disabled.clone(), path, json!({})).await["message"],
            "multica_workspace_disabled"
        );
    }
}

fn test_context() -> BridgeContext {
    BridgeContext::new(
        Arc::new(FakeSettings::default()),
        Arc::new(FakeRuntime::default()),
        Arc::new(FakeData::default()),
    )
    .without_diagnostics()
}

fn multica_execution_test_context() -> (BridgeContext, FakeCodexPageHostTransport, tempfile::TempDir)
{
    let store_dir = tempfile::tempdir().unwrap();
    let transport = FakeCodexPageHostTransport::default();
    let client = CodexPageExecutionClient::new(
        transport.clone(),
        CodexRuntimeBinding {
            runtime_id: "codex-current-page".to_string(),
            provider: "codex".to_string(),
            app_server_version: None,
            declared_capabilities: Vec::new(),
        },
    )
    .unwrap();
    let runtime =
        CoreRuntimeService::new(9229, StatusStore::new(store_dir.path().join("status.json")))
            .with_codex_execution_service(Arc::new(client))
            .with_codex_page_transport(Arc::new(transport.clone()))
            .with_multica_workspace_store(LocalMulticaWorkspaceStore::new(
                store_dir.path().join("workspace.json"),
            ))
            .with_multica_execution_store(MulticaExecutionStore::new(
                store_dir.path().join("multica-execution.json"),
            ));
    let context = BridgeContext::new(
        Arc::new(FakeSettings::default()),
        Arc::new(runtime),
        Arc::new(FakeData::default()),
    )
    .without_diagnostics();
    (context, transport, store_dir)
}

#[derive(Default)]
struct FakeSettings {
    settings: Mutex<BackendSettings>,
    codex_app_version: Mutex<String>,
}

impl FakeSettings {
    fn with_codex_app_version(version: &str) -> Self {
        Self {
            settings: Mutex::new(BackendSettings::default()),
            codex_app_version: Mutex::new(version.to_string()),
        }
    }
}

#[async_trait]
impl BridgeSettingsService for FakeSettings {
    async fn get_settings(&self) -> anyhow::Result<BackendSettings> {
        Ok(self.settings.lock().unwrap().clone())
    }

    async fn set_settings(&self, payload: Value) -> anyhow::Result<BackendSettings> {
        let current = self.settings.lock().unwrap().clone();
        let mut raw = serde_json::to_value(current).unwrap();
        let raw = raw.as_object_mut().unwrap();
        if let Some(value) = payload.get("providerSyncEnabled").and_then(Value::as_bool) {
            raw.insert("providerSyncEnabled".to_string(), json!(value));
        }
        if let Some(value) = payload.get("enhancementsEnabled").and_then(Value::as_bool) {
            raw.insert("enhancementsEnabled".to_string(), json!(value));
        }
        for key in [
            "codexAppPluginEntryUnlock",
            "codexAppPluginMarketplaceUnlock",
            "codexAppForcePluginInstall",
            "codexAppSessionDelete",
            "codexAppMarkdownExport",
            "codexAppProjectMove",
            "codexAppConversationTimeline",
            "codexAppConversationView",
            "codexAppThreadScrollRestore",
            "codexAppZedRemoteOpen",
            "codexAppUpstreamWorktreeCreate",
            "codexAppNativeMenuPlacement",
            "codexAppServiceTierControls",
        ] {
            if let Some(value) = payload.get(key).and_then(Value::as_bool) {
                raw.insert(key.to_string(), json!(value));
            }
        }
        if let Some(value) = payload.get("launchMode").and_then(Value::as_str) {
            raw.insert("launchMode".to_string(), json!(value));
        }
        if let Some(value) = payload.get("relayBaseUrl").and_then(Value::as_str) {
            raw.insert("relayBaseUrl".to_string(), json!(value));
        }
        if let Some(value) = payload.get("relayApiKey").and_then(Value::as_str) {
            raw.insert("relayApiKey".to_string(), json!(value));
        }
        if let Some(value) = payload.get("cliWrapperApiKeyEnv").and_then(Value::as_str) {
            raw.insert(
                "cliWrapperApiKeyEnv".to_string(),
                json!(if value.is_empty() {
                    "CUSTOM_OPENAI_API_KEY"
                } else {
                    value
                }),
            );
        }
        let updated: BackendSettings = serde_json::from_value(Value::Object(raw.clone())).unwrap();
        *self.settings.lock().unwrap() = updated.clone();
        Ok(updated)
    }

    async fn codex_app_version(&self) -> anyhow::Result<String> {
        Ok(self.codex_app_version.lock().unwrap().clone())
    }
}

struct FakeRuntime {
    enabled: Mutex<bool>,
    script_enabled: Mutex<bool>,
    multica_calls: Mutex<Vec<String>>,
}

impl Default for FakeRuntime {
    fn default() -> Self {
        Self {
            enabled: Mutex::new(true),
            script_enabled: Mutex::new(true),
            multica_calls: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl BridgeRuntimeService for FakeRuntime {
    async fn user_script_inventory(&self) -> anyhow::Result<Value> {
        Ok(self.inventory(false))
    }

    async fn set_user_scripts_enabled(&self, enabled: bool) -> anyhow::Result<Value> {
        *self.enabled.lock().unwrap() = enabled;
        Ok(self.inventory(false))
    }

    async fn set_user_script_enabled(&self, key: String, enabled: bool) -> anyhow::Result<Value> {
        assert_eq!(key, "user:a.js");
        *self.script_enabled.lock().unwrap() = enabled;
        Ok(self.inventory(false))
    }

    async fn delete_user_script(&self, key: String) -> anyhow::Result<Value> {
        assert_eq!(key, "user:a.js");
        *self.script_enabled.lock().unwrap() = false;
        Ok(self.inventory(false))
    }

    async fn reload_user_scripts(&self) -> anyhow::Result<Value> {
        Ok(self.inventory(true))
    }

    async fn open_devtools(&self) -> anyhow::Result<Value> {
        Ok(json!({"status": "ok", "opened": true}))
    }

    async fn open_manager(&self) -> anyhow::Result<Value> {
        Ok(json!({"status": "ok", "opened": "manager"}))
    }

    async fn multica_workspace_bootstrap(&self) -> anyhow::Result<Value> {
        self.multica_calls
            .lock()
            .unwrap()
            .push("bootstrap".to_string());
        Ok(json!({"status": "ok", "modules": ["skills"]}))
    }

    async fn multica_autopilot_tick(&self) -> anyhow::Result<Value> {
        self.multica_calls.lock().unwrap().push("tick".into());
        Ok(json!({"status":"ok","runs":[],"diagnostics":[],"hasMore":false}))
    }

    async fn multica_workspace_query(&self, query: MulticaWorkspaceQuery) -> anyhow::Result<Value> {
        self.multica_calls.lock().unwrap().push(format!(
            "query:{}:{}:{}",
            serde_json::to_value(query.resource)?
                .as_str()
                .unwrap_or_default(),
            query.limit,
            query.offset
        ));
        Ok(json!({
            "resource": query.resource,
            "items": [],
            "total": 0,
            "limit": query.limit,
            "offset": query.offset
        }))
    }

    async fn backend_status(&self) -> anyhow::Result<Value> {
        Ok(
            json!({"status": "ok", "message": "后端已连接", "version": claude_codex_pro_core::version::VERSION}),
        )
    }

    async fn repair_backend(&self) -> anyhow::Result<Value> {
        Ok(
            json!({"status": "ok", "message": "后端已修复", "version": claude_codex_pro_core::version::VERSION}),
        )
    }

    async fn claude_desktop_status(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "ok",
            "processCount": 1,
            "installKind": "msix",
            "cdpStatus": "blocked",
            "cdpBlocker": "CLAUDE_CDP_AUTH required",
            "supportedIntegration": "external_automation",
            "integrityStatus": "ok",
            "integrityMessage": "Audited 1 Claude Desktop executable path without modifying app files.",
            "executableAudits": [{
                "path": "C:\\Program Files\\WindowsApps\\Claude_1.0\\app\\Claude.exe",
                "exists": true,
                "fileSizeBytes": 1024,
                "modifiedUnixMs": 123456,
                "sha256": "abc123",
                "peFormat": "pe32_plus",
                "peMachine": "x64",
                "peSubsystem": "windows_gui",
                "peTimestampUnix": 1700000000,
                "peEntryPointRva": 4096,
                "peImageBase": 5368709120_u64,
                "peSectionCount": 1,
                "peCertificateTableBytes": 512,
                "peSections": [{
                    "name": ".text",
                    "virtualAddress": 4096,
                    "virtualSize": 8192,
                    "rawSize": 4096,
                    "rawSha256": "section123",
                    "characteristics": "0x60000020"
                }],
                "signatureStatus": "Valid",
                "signatureMessage": "Signature verified",
                "signerSubject": "CN=Anthropic",
                "signerIssuer": "CN=Trusted Root",
                "signerThumbprint": "thumbprint123",
                "signerSerialNumber": "serial123",
                "signerNotBefore": "2026-01-01T00:00:00.0000000Z",
                "signerNotAfter": "2027-01-01T00:00:00.0000000Z",
                "signerChainStatus": "",
                "productName": "Claude",
                "fileDescription": "Claude",
                "fileVersion": "1.12603.1.0",
                "originalFilename": "Claude.exe",
                "installKind": "msix",
                "trustBasis": "msix_protected_install_location_and_valid_authenticode",
                "integrityLevel": "executable_hash_authenticode_pe_header_section_audit",
                "riskLevel": "controlled",
                "verificationScope": "sha256_file_hash_authenticode_certificate_window_version_resource_pe_header_section_hashes_install_path",
                "mutationPolicy": "blocked_no_executable_asar_signature_or_integrity_metadata_changes",
                "patchEligible": false,
                "notes": ["Read-only audit only"]
            }]
        }))
    }

    async fn claude_desktop_integrity(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "ok",
            "message": "Audited 1 Claude Desktop executable path without modifying app files.",
            "policy": "read_only_audit_no_executable_or_asar_patch",
            "executableAudits": [{
                "path": "C:\\Program Files\\WindowsApps\\Claude_1.0\\app\\Claude.exe",
                "exists": true,
                "fileSizeBytes": 1024,
                "modifiedUnixMs": 123456,
                "sha256": "abc123",
                "peFormat": "pe32_plus",
                "peMachine": "x64",
                "peSubsystem": "windows_gui",
                "peTimestampUnix": 1700000000,
                "peEntryPointRva": 4096,
                "peImageBase": 5368709120_u64,
                "peSectionCount": 1,
                "peCertificateTableBytes": 512,
                "peSections": [{
                    "name": ".text",
                    "virtualAddress": 4096,
                    "virtualSize": 8192,
                    "rawSize": 4096,
                    "rawSha256": "section123",
                    "characteristics": "0x60000020"
                }],
                "signatureStatus": "Valid",
                "signatureMessage": "Signature verified",
                "signerSubject": "CN=Anthropic",
                "signerIssuer": "CN=Trusted Root",
                "signerThumbprint": "thumbprint123",
                "signerSerialNumber": "serial123",
                "signerNotBefore": "2026-01-01T00:00:00.0000000Z",
                "signerNotAfter": "2027-01-01T00:00:00.0000000Z",
                "signerChainStatus": "",
                "productName": "Claude",
                "fileDescription": "Claude",
                "fileVersion": "1.12603.1.0",
                "originalFilename": "Claude.exe",
                "installKind": "msix",
                "trustBasis": "msix_protected_install_location_and_valid_authenticode",
                "integrityLevel": "executable_hash_authenticode_pe_header_section_audit",
                "riskLevel": "controlled",
                "verificationScope": "sha256_file_hash_authenticode_certificate_window_version_resource_pe_header_section_hashes_install_path",
                "mutationPolicy": "blocked_no_executable_asar_signature_or_integrity_metadata_changes",
                "patchEligible": false,
                "notes": ["Read-only audit only"]
            }]
        }))
    }

    async fn claude_desktop_focus(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "ok",
            "message": "focused",
            "processId": 1234,
            "action": "focus",
            "foregroundVerified": true,
            "foregroundProcessId": 1234,
            "foregroundTitle": "Claude"
        }))
    }

    async fn claude_desktop_verify(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "ok",
            "message": "verified",
            "processId": 1234,
            "action": "verify_target",
            "foregroundVerified": true,
            "foregroundProcessId": 1234,
            "foregroundTitle": "Claude"
        }))
    }

    async fn claude_desktop_open(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "accepted",
            "message": "launch requested",
            "processId": null,
            "action": "open",
            "foregroundVerified": false,
            "foregroundProcessId": null,
            "foregroundTitle": null
        }))
    }

    async fn claude_desktop_new_chat(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "ok",
            "message": "new chat",
            "processId": 1234,
            "action": "new_chat",
            "foregroundVerified": true,
            "foregroundProcessId": 1234,
            "foregroundTitle": "Claude"
        }))
    }

    async fn claude_desktop_paste_draft(&self, payload: Value) -> anyhow::Result<Value> {
        assert_eq!(payload["text"], json!("hello Claude"));
        Ok(json!({
            "status": "ok",
            "message": "draft pasted",
            "processId": 1234,
            "action": "paste_draft",
            "inputChars": 12,
            "autoSubmitted": false,
            "foregroundVerified": true,
            "foregroundProcessId": 1234,
            "foregroundTitle": "Claude"
        }))
    }

    async fn claude_desktop_submit(&self, payload: Value) -> anyhow::Result<Value> {
        assert_eq!(payload["text"], json!("hello Claude"));
        Ok(json!({
            "status": "ok",
            "message": "submitted",
            "processId": 1234,
            "action": "paste_and_submit",
            "inputChars": 12,
            "autoSubmitted": true,
            "foregroundVerified": true,
            "foregroundProcessId": 1234,
            "foregroundTitle": "Claude"
        }))
    }

    async fn codex_model_catalog(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "ok",
            "model": "qwen3-coder",
            "default_model": "qwen3-coder",
            "model_provider": "relay",
            "provider_name": "Relay",
            "models": ["qwen3-coder"],
            "sources": []
        }))
    }

    async fn ads(&self) -> anyhow::Result<Value> {
        Ok(json!({"version": 1, "ads": [{"id": "runtime-ad"}]}))
    }

    async fn zed_remote_status(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "ok",
            "platformSupported": true,
            "zedAppFound": true,
            "zedCliFound": false
        }))
    }

    async fn resolve_zed_remote_host(&self, payload: Value) -> anyhow::Result<Value> {
        assert_eq!(payload["hostId"], json!("remote-ssh-codex-managed:remote"));
        Ok(json!({
            "status": "ok",
            "ssh": {"user": "longnv", "host": "192.168.100.31", "port": null}
        }))
    }

    async fn fallback_zed_remote_request(&self, payload: Value) -> anyhow::Result<Value> {
        assert_eq!(payload["hostId"], json!("remote-ssh-codex-managed:remote"));
        Ok(json!({
            "status": "ok",
            "request": {
                "hostId": "remote-ssh-codex-managed:remote",
                "ssh": {"user": "longnv", "host": "192.168.100.31", "port": null},
                "path": "/Users/longnv/bin/repo/sealos-skills",
            }
        }))
    }

    async fn open_zed_remote(&self, payload: Value) -> anyhow::Result<Value> {
        assert_eq!(payload["path"], json!("/home/app.py"));
        Ok(
            json!({"status": "ok", "url": "ssh://example.com/home/app.py", "strategy": "addToFocusedWorkspace"}),
        )
    }

    async fn list_zed_remote_projects(&self, _payload: Value) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "ok",
            "projects": [{
                "id": "zed-remote-project:test",
                "label": "sealos-skills",
                "hostId": "remote-ssh-codex-managed:remote",
                "ssh": {"user": "longnv", "host": "192.168.100.31", "port": null},
                "path": "/Users/longnv/bin/repo/sealos-skills",
                "url": "ssh://longnv@192.168.100.31/Users/longnv/bin/repo/sealos-skills",
                "source": "codexRemoteProject",
                "lastOpenedAtMs": null,
                "isCurrent": false
            }]
        }))
    }

    async fn remember_zed_remote_project(&self, payload: Value) -> anyhow::Result<Value> {
        assert_eq!(payload["path"], json!("/home/app.py"));
        Ok(json!({"status": "ok", "remembered": true}))
    }

    async fn forget_zed_remote_project(&self, payload: Value) -> anyhow::Result<Value> {
        assert_eq!(payload["id"], json!("zed-remote-project:test"));
        Ok(json!({"status": "ok", "removed": 1}))
    }

    async fn upstream_worktree_status(&self) -> anyhow::Result<Value> {
        Ok(json!({"status": "ok", "feature": "upstream-worktree"}))
    }

    async fn upstream_worktree_defaults(&self, payload: Value) -> anyhow::Result<Value> {
        assert_eq!(payload["repoPath"], json!("/repo"));
        Ok(json!({
            "status": "ok",
            "repoRoot": "/repo",
            "defaultRemote": "upstream",
            "defaultBaseBranch": "main",
        }))
    }

    async fn upstream_worktree_prepare(&self, payload: Value) -> anyhow::Result<Value> {
        assert_eq!(payload["repoPath"], json!("/repo"));
        assert_eq!(payload["remote"], json!("upstream"));
        assert_eq!(payload["baseBranch"], json!("main"));
        Ok(json!({
            "status": "ok",
            "repoRoot": "/repo",
            "sourceRef": "upstream/main",
            "qualifiedSourceRef": "refs/remotes/upstream/main",
        }))
    }

    async fn upstream_worktree_create(&self, payload: Value) -> anyhow::Result<Value> {
        assert_eq!(payload["repoPath"], json!("/repo"));
        assert_eq!(payload["branchName"], json!("feature/demo"));
        Ok(json!({
            "status": "ok",
            "repoRoot": "/repo",
            "branchName": "feature/demo",
            "worktreePath": "/repo-feature-demo",
        }))
    }
}

impl FakeRuntime {
    fn inventory(&self, reloaded: bool) -> Value {
        json!({
            "enabled": *self.enabled.lock().unwrap(),
            "reloaded": reloaded,
            "scripts": [
                {"key": "builtin:demo.js", "name": "demo.js", "enabled": true},
                {"key": "user:a.js", "name": "a.js", "enabled": *self.script_enabled.lock().unwrap()}
            ]
        })
    }
}

struct FakeData;

impl Default for FakeData {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl BridgeDataService for FakeData {
    async fn session_availability(&self, session_ids: Vec<String>) -> anyhow::Result<Vec<String>> {
        Ok(session_ids)
    }

    async fn delete(&self, session: SessionRef) -> anyhow::Result<DeleteResult> {
        Ok(DeleteResult {
            status: DeleteStatus::LocalDeleted,
            session_id: session.session_id.clone(),
            message: format!("deleted {}", session.title),
            undo_token: Some(format!("undo-{}", session.session_id)),
            backup_path: None,
        })
    }

    async fn undo(&self, undo_token: String) -> anyhow::Result<DeleteResult> {
        Ok(DeleteResult {
            status: DeleteStatus::Undone,
            session_id: "s1".to_string(),
            message: "undone".to_string(),
            undo_token: Some(undo_token),
            backup_path: None,
        })
    }

    async fn export_markdown(&self, session: SessionRef) -> anyhow::Result<ExportResult> {
        Ok(ExportResult {
            status: ExportStatus::Exported,
            session_id: session.session_id,
            message: "exported".to_string(),
            filename: Some("First.md".to_string()),
            markdown: Some("# First\n".to_string()),
        })
    }

    async fn thread_usage_history(&self, session: SessionRef) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "ok",
            "session_id": session.session_id,
            "history": [
                {
                    "source": "rollout-history",
                    "conversation_id": "local:s1",
                    "turn_id": "turn-1",
                    "observed_at": "2026-06-02T05:00:00Z",
                    "usage": {
                        "inputTokens": 1200,
                        "outputTokens": 120,
                        "totalTokens": 1320,
                        "cachedTokens": 900,
                        "cacheReadTokens": 0,
                        "cacheCreationTokens": 0,
                        "contextUsed": 1320,
                        "contextLimit": 258400,
                        "hasBreakdown": true
                    }
                }
            ]
        }))
    }

    async fn find_archived_thread_by_title(
        &self,
        title: String,
    ) -> anyhow::Result<Option<SessionRef>> {
        Ok(Some(SessionRef {
            session_id: "archived-1".to_string(),
            title,
        }))
    }

    async fn move_thread_workspace(
        &self,
        session: SessionRef,
        target_cwd: String,
    ) -> anyhow::Result<Value> {
        Ok(json!({"status": "moved", "session_id": session.session_id, "target_cwd": target_cwd}))
    }

    async fn thread_sort_key(&self, session: SessionRef) -> anyhow::Result<Value> {
        Ok(json!({"status": "ok", "session_id": session.session_id, "updated_at": 123}))
    }

    async fn thread_sort_keys(&self, sessions: Vec<SessionRef>) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "ok",
            "sort_keys": sessions
                .into_iter()
                .map(|session| json!({"session_id": session.session_id}))
                .collect::<Vec<_>>()
        }))
    }
}

#[derive(Clone)]
struct ContextHooks {
    events: Arc<Mutex<Vec<String>>>,
}

impl ContextHooks {
    fn event(&self, event: impl Into<String>) {
        self.events.lock().unwrap().push(event.into());
    }
}

#[async_trait(?Send)]
impl LaunchHooks for ContextHooks {
    fn resolve_app_dir(
        &self,
        app_dir: Option<&std::path::Path>,
        _settings: &BackendSettings,
    ) -> anyhow::Result<std::path::PathBuf> {
        app_dir
            .map(std::path::Path::to_path_buf)
            .ok_or_else(|| anyhow::anyhow!("missing app dir"))
    }

    fn select_debug_port(&self, requested: u16) -> u16 {
        requested
    }

    fn select_helper_port(&self, requested: u16) -> u16 {
        requested
    }

    async fn load_settings(&self) -> anyhow::Result<BackendSettings> {
        Ok(BackendSettings::default())
    }

    async fn run_provider_sync(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn start_helper(&self, _helper_port: u16) -> anyhow::Result<()> {
        Ok(())
    }

    async fn launch_codex(
        &self,
        _app_dir: &std::path::Path,
        _debug_port: u16,
        _extra_args: &[String],
    ) -> anyhow::Result<CodexLaunch> {
        Ok(CodexLaunch::Process {
            command: vec!["codex".to_string()],
            wait_strategy: ProcessWaitStrategy::TrackedChild,
            macos_cleanup_policy: None,
        })
    }

    async fn bridge_context(
        &self,
        debug_port: u16,
        _app_dir: &std::path::Path,
    ) -> anyhow::Result<Option<BridgeContext>> {
        self.event(format!("bridge-context:{debug_port}"));
        Ok(Some(test_context()))
    }

    async fn inject(&self, _debug_port: u16, _helper_port: u16) -> anyhow::Result<()> {
        anyhow::bail!("legacy inject should not run when bridge context is supplied")
    }

    async fn inject_bridge(
        &self,
        debug_port: u16,
        helper_port: u16,
        _ctx: BridgeContext,
    ) -> anyhow::Result<()> {
        self.event(format!("inject-bridge:{debug_port}:{helper_port}"));
        Ok(())
    }

    async fn start_bridge_watchdog(&self, debug_port: u16, helper_port: u16) -> anyhow::Result<()> {
        self.event(format!("watchdog:{debug_port}:{helper_port}"));
        Ok(())
    }

    async fn write_status(&self, status: &str) {
        self.event(format!("status:{status}"));
    }

    async fn wait_for_codex_exit(&self, _launch: &CodexLaunch) -> anyhow::Result<()> {
        Ok(())
    }

    async fn shutdown_helper(&self, _helper_port: u16) {}

    async fn terminate_codex(&self, _launch: &CodexLaunch) {}
}

#[tokio::test]
async fn native_drag_enqueue_continues_the_existing_thread_once() {
    let (ctx, transport, _dir) = multica_execution_test_context();
    transport.push_response(CodexPageHostMethod::Initialize, Ok(json!({
        "provider":"codex", "pageHostProbe":{"methods":["thread/start","thread/read","turn/start","turn/interrupt","skills/list"],"skillInput":true}
    })));
    let thread = json!({"thread":{"id":"native-child","parentThreadId":"native-parent","turns":[{"id":"old-turn","status":"completed"}]}});
    transport.push_response(CodexPageHostMethod::ThreadRead, Ok(thread.clone()));
    let queued = handle_bridge_request(ctx.clone(), "/multica/native-executions/intent", json!({
        "workspaceId":"local-test", "threadId":"native-child", "intent":"enqueue", "idempotencyKey":"native-drag-1"
    })).await;
    assert_eq!(queued["status"], "ok", "{queued}");
    assert_eq!(queued["binding"]["state"], "binding_pending");
    assert_eq!(queued["binding"]["codexThreadId"], "native-child");
    assert!(transport.calls().iter().all(|r| !matches!(
        r.method,
        CodexPageHostMethod::TurnStart
            | CodexPageHostMethod::ThreadStart
            | CodexPageHostMethod::ThreadFork
    )));
    transport.push_response(CodexPageHostMethod::ThreadRead, Ok(thread));
    transport.push_response(
        CodexPageHostMethod::TurnStart,
        Ok(json!({"turn":{"id":"next-turn","status":"inProgress"}})),
    );
    let started = handle_bridge_request(ctx.clone(), "/multica/executions/dispatch", json!({
        "bindingId":queued["binding"]["bindingId"], "expectedRevision":queued["binding"]["revision"], "leaseToken":"native-worker-1"
    })).await;
    assert_eq!(started["status"], "ok", "{started}");
    assert_eq!(started["binding"]["codexThreadId"], "native-child");
    let calls = transport.calls();
    let turns = calls
        .iter()
        .filter(|r| r.method == CodexPageHostMethod::TurnStart)
        .collect::<Vec<_>>();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].params["threadId"], "native-child");
    assert_eq!(
        turns[0].params["input"][0]["text"],
        "继续完成当前任务，检查剩余工作并完成验证；若已全部完成，简要确认结果。"
    );
    let replay = handle_bridge_request(ctx, "/multica/native-executions/intent", json!({
        "workspaceId":"local-test", "threadId":"native-child", "intent":"enqueue", "idempotencyKey":"native-drag-1"
    })).await;
    assert_eq!(replay["status"], "ok", "{replay}");
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|r| r.method == CodexPageHostMethod::TurnStart)
            .count(),
        1
    );
}

fn native_thread_fixture(status: &str) -> Value {
    json!({"thread":{"id":"native-child","parentThreadId":"native-parent","turns":[{"id":"old-turn","status":status}]}})
}
fn native_host_ready(transport: &FakeCodexPageHostTransport) {
    transport.push_response(CodexPageHostMethod::Initialize, Ok(json!({
        "provider":"codex", "pageHostProbe":{"methods":["thread/start","thread/read","turn/start","turn/interrupt","skills/list"],"skillInput":true}
    })));
}

#[tokio::test]
async fn native_drag_continue_failed_and_completed_reuses_thread_and_polls_result() {
    for state in ["completed", "failed", "interrupted"] {
        let (ctx, transport, dir) = multica_execution_test_context();
        native_host_ready(&transport);
        for _ in 0..2 {
            transport.push_response(
                CodexPageHostMethod::ThreadRead,
                Ok(native_thread_fixture(state)),
            );
        }
        transport.push_response(
            CodexPageHostMethod::TurnStart,
            Ok(json!({"turn":{"id":"new-turn","status":"inProgress"}})),
        );
        let started = handle_bridge_request(ctx.clone(), "/multica/native-executions/intent", json!({"workspaceId":"local-test","threadId":"native-child","intent":"continue","idempotencyKey":"go-again"})).await;
        assert_eq!(started["status"], "ok", "{started}");
        assert_eq!(started["binding"]["state"], "dispatched");
        assert_eq!(started["binding"]["nativeResume"], true);
        assert_eq!(started["binding"]["codexThreadId"], "native-child");
        assert_eq!(started["binding"]["codexExecutionId"], "new-turn");
        transport.push_response(CodexPageHostMethod::ThreadRead, Ok(json!({"thread":{"id":"native-child","turns":[{"id":"new-turn","status":"failed"}]}})));
        let result = handle_bridge_request(
            ctx.clone(),
            "/multica/executions/status",
            json!({"bindingId":started["binding"]["bindingId"]}),
        )
        .await;
        assert_eq!(result["binding"]["state"], "failed", "{result}");
        let persisted = std::fs::read_to_string(dir.path().join("multica-execution.json")).unwrap();
        assert!(!persisted.contains("继续完成当前任务"));
        assert!(transport.calls().iter().all(|r| !matches!(
            r.method,
            CodexPageHostMethod::ThreadStart | CodexPageHostMethod::ThreadFork
        )));
        assert!(
            transport
                .calls()
                .iter()
                .filter(|r| r.method == CodexPageHostMethod::TurnStart)
                .all(|r| r.params["threadId"] == "native-child")
        );
    }
}

#[tokio::test]
async fn native_drag_concurrent_enqueue_reserves_one_binding_and_one_command() {
    let (ctx, transport, dir) = multica_execution_test_context();
    native_host_ready(&transport);
    for _ in 0..2 {
        transport.push_response(
            CodexPageHostMethod::ThreadRead,
            Ok(native_thread_fixture("completed")),
        );
    }
    let (a, b) = tokio::join!(
        handle_bridge_request(
            ctx.clone(),
            "/multica/native-executions/intent",
            json!({"workspaceId":"local-test","threadId":"native-child","intent":"enqueue","idempotencyKey":"drag-a"})
        ),
        handle_bridge_request(
            ctx,
            "/multica/native-executions/intent",
            json!({"workspaceId":"local-test","threadId":"native-child","intent":"enqueue","idempotencyKey":"drag-b"})
        )
    );
    assert_eq!(a["status"], "ok", "{a}");
    assert_eq!(b["status"], "ok", "{b}");
    assert_eq!(a["binding"]["bindingId"], b["binding"]["bindingId"]);
    assert!(a["binding"]["codexExecutionId"].is_null());
    let state: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("multica-execution.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(state["executionBindings"].as_array().unwrap().len(), 1);
    assert_eq!(state["executionCommands"].as_array().unwrap().len(), 1);
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|r| r.method == CodexPageHostMethod::TurnStart)
            .count(),
        0
    );
}

#[tokio::test]
async fn native_drag_running_and_unknown_never_start_an_extra_turn() {
    for status in ["inProgress", "unknown"] {
        let (ctx, transport, dir) = multica_execution_test_context();
        native_host_ready(&transport);
        transport.push_response(
            CodexPageHostMethod::ThreadRead,
            Ok(native_thread_fixture(status)),
        );
        let result = handle_bridge_request(ctx, "/multica/native-executions/intent", json!({"workspaceId":"local-test","threadId":"native-child","intent":"continue","idempotencyKey":"duplicate"})).await;
        assert_eq!(
            result["status"],
            if status == "inProgress" {
                "ok"
            } else {
                "failed"
            },
            "{result}"
        );
        if status == "inProgress" {
            assert_eq!(result["alreadyActive"], true);
        }
        assert_eq!(
            dir.path().join("multica-execution.json").exists(),
            status == "inProgress"
        );
        assert_eq!(
            transport
                .calls()
                .iter()
                .filter(|r| r.method == CodexPageHostMethod::TurnStart)
                .count(),
            0
        );
    }
}

#[tokio::test]
async fn native_drag_invalid_intents_and_custom_prompts_never_reach_native_host() {
    for payload in [
        json!({"workspaceId":"local-test","threadId":"native-child","intent":"done","idempotencyKey":"invalid"}),
        json!({"workspaceId":"local-test","threadId":"native-child","intent":"continue","idempotencyKey":"invalid","prompt":"replace task"}),
    ] {
        let (ctx, transport, _) = multica_execution_test_context();
        let result = handle_bridge_request(ctx, "/multica/native-executions/intent", payload).await;
        assert_eq!(result["status"], "failed");
        assert!(transport.calls().is_empty());
    }
}

#[tokio::test]
async fn native_drag_ambiguous_dispatch_is_not_automatically_retried() {
    let (ctx, transport, _) = multica_execution_test_context();
    native_host_ready(&transport);
    for _ in 0..2 {
        transport.push_response(
            CodexPageHostMethod::ThreadRead,
            Ok(native_thread_fixture("completed")),
        );
    }
    transport.push_response(
        CodexPageHostMethod::TurnStart,
        Err(anyhow::anyhow!("transport_timeout")),
    );
    let request = json!({"workspaceId":"local-test","threadId":"native-child","intent":"continue","idempotencyKey":"timeout-drag"});
    let failed = handle_bridge_request(
        ctx.clone(),
        "/multica/native-executions/intent",
        request.clone(),
    )
    .await;
    assert_eq!(failed["status"], "failed", "{failed}");
    transport.push_response(
        CodexPageHostMethod::ThreadRead,
        Ok(native_thread_fixture("completed")),
    );
    let replay =
        handle_bridge_request(ctx.clone(), "/multica/native-executions/intent", request).await;
    assert_eq!(
        replay["status"], "failed",
        "ambiguous continuation must not be acknowledged as a fresh queued success: {replay}"
    );
    assert_eq!(replay["message"], "native_dispatch_outcome_pending");
    let listed = handle_bridge_request(
        ctx.clone(),
        "/multica/executions/list",
        json!({"workspaceId":"local-test"}),
    )
    .await;
    assert_eq!(listed["items"][0]["state"], "reconciling");
    let retry = handle_bridge_request(ctx, "/multica/executions/dispatch", json!({"bindingId":listed["items"][0]["bindingId"],"expectedRevision":listed["items"][0]["revision"],"leaseToken":"worker-retry"})).await;
    assert_eq!(retry["status"], "failed");
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|r| r.method == CodexPageHostMethod::TurnStart)
            .count(),
        1
    );
}

#[tokio::test]
async fn native_drag_deduplicates_two_dispatchers_and_never_targets_parent() {
    let (ctx, transport, _) = multica_execution_test_context();
    native_host_ready(&transport);
    transport.push_response(
        CodexPageHostMethod::ThreadRead,
        Ok(native_thread_fixture("completed")),
    );
    let queued = handle_bridge_request(ctx.clone(), "/multica/native-executions/intent", json!({"workspaceId":"local-test","threadId":"native-child","intent":"enqueue","idempotencyKey":"queue-once"})).await;
    transport.push_response(
        CodexPageHostMethod::ThreadRead,
        Ok(native_thread_fixture("completed")),
    );
    transport.push_response(
        CodexPageHostMethod::TurnStart,
        Ok(json!({"turn":{"id":"new-turn","status":"inProgress"}})),
    );
    let payload = json!({"bindingId":queued["binding"]["bindingId"],"expectedRevision":queued["binding"]["revision"],"leaseToken":"worker-a"});
    let (a, b) = tokio::join!(
        handle_bridge_request(ctx.clone(), "/multica/executions/dispatch", payload.clone()),
        handle_bridge_request(ctx, "/multica/executions/dispatch", payload)
    );
    assert!(a["status"] == "ok" || b["status"] == "ok");
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|r| r.method == CodexPageHostMethod::TurnStart)
            .count(),
        1
    );
    assert!(
        transport
            .calls()
            .iter()
            .filter(|r| r.method == CodexPageHostMethod::TurnStart)
            .all(|r| r.params["threadId"] == "native-child")
    );
}

#[tokio::test]
async fn native_drag_checks_live_turns_even_when_the_last_array_entry_is_completed() {
    let (ctx, transport, _) = multica_execution_test_context();
    native_host_ready(&transport);
    transport.push_response(CodexPageHostMethod::ThreadRead, Ok(json!({"thread":{"id":"native-child","parentThreadId":"native-parent","turns":[{"id":"live","status":"inProgress"},{"id":"old","status":"completed"}]}})));
    let result = handle_bridge_request(ctx, "/multica/native-executions/intent", json!({"workspaceId":"local-test","threadId":"native-child","intent":"continue","idempotencyKey":"no-duplicate"})).await;
    assert_eq!(result["alreadyActive"], true, "{result}");
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|r| r.method == CodexPageHostMethod::TurnStart)
            .count(),
        0
    );
}

#[tokio::test]
async fn native_drag_does_not_execute_a_parent_or_cross_workspace_replay() {
    let (ctx, transport, _) = multica_execution_test_context();
    native_host_ready(&transport);
    transport.push_response(
        CodexPageHostMethod::ThreadRead,
        Ok(json!({"thread":{"id":"native-parent","turns":[{"id":"old","status":"completed"}]}})),
    );
    let result = handle_bridge_request(ctx.clone(), "/multica/native-executions/intent", json!({"workspaceId":"local-test","threadId":"native-parent","intent":"continue","idempotencyKey":"parent-no"})).await;
    assert_eq!(result["message"], "native_subagent_parent_unavailable");
    transport.push_response(
        CodexPageHostMethod::ThreadRead,
        Ok(native_thread_fixture("completed")),
    );
    let queued = handle_bridge_request(ctx.clone(), "/multica/native-executions/intent", json!({"workspaceId":"local-test","threadId":"native-child","intent":"enqueue","idempotencyKey":"local-queue"})).await;
    assert_eq!(queued["status"], "ok", "{queued}");
    let denied = handle_bridge_request(ctx, "/multica/native-executions/intent", json!({"workspaceId":"another-workspace","threadId":"native-child","intent":"continue","idempotencyKey":"cross-queue"})).await;
    assert_eq!(denied["message"], "workspace_mismatch");
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|r| r.method == CodexPageHostMethod::TurnStart)
            .count(),
        0
    );
}

#[tokio::test]
async fn native_drag_retry_reconciles_a_lost_response_without_starting_a_second_turn() {
    let (ctx, transport, _) = multica_execution_test_context();
    native_host_ready(&transport);
    for _ in 0..2 {
        transport.push_response(
            CodexPageHostMethod::ThreadRead,
            Ok(native_thread_fixture("completed")),
        );
    }
    transport.push_response(
        CodexPageHostMethod::TurnStart,
        Err(anyhow::anyhow!("transport_timeout")),
    );
    let request = json!({"workspaceId":"local-test","threadId":"native-child","intent":"continue","idempotencyKey":"lost-response"});
    assert_eq!(
        handle_bridge_request(
            ctx.clone(),
            "/multica/native-executions/intent",
            request.clone()
        )
        .await["status"],
        "failed"
    );
    transport.push_response(CodexPageHostMethod::ThreadRead, Ok(json!({"thread":{"id":"native-child","parentThreadId":"native-parent","turns":[{"id":"old-turn","status":"completed"},{"id":"actual-new-turn","status":"completed"}]}})));
    let recovered = handle_bridge_request(ctx, "/multica/native-executions/intent", request).await;
    assert_eq!(recovered["status"], "ok", "{recovered}");
    assert_eq!(recovered["binding"]["codexExecutionId"], "actual-new-turn");
    assert_eq!(recovered["binding"]["state"], "completed");
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|r| r.method == CodexPageHostMethod::TurnStart)
            .count(),
        1
    );
}

#[tokio::test]
async fn native_drag_queued_to_running_dispatches_for_same_or_new_intent_key() {
    for key in ["queue-key", "run-key"] {
        let (ctx, transport, _) = multica_execution_test_context();
        native_host_ready(&transport);
        transport.push_response(
            CodexPageHostMethod::ThreadRead,
            Ok(native_thread_fixture("completed")),
        );
        let queued = handle_bridge_request(ctx.clone(), "/multica/native-executions/intent", json!({"workspaceId":"local-test","threadId":"native-child","intent":"enqueue","idempotencyKey":"queue-key"})).await;
        assert_eq!(queued["binding"]["state"], "binding_pending", "{queued}");
        transport.push_response(
            CodexPageHostMethod::ThreadRead,
            Ok(native_thread_fixture("completed")),
        );
        transport.push_response(
            CodexPageHostMethod::TurnStart,
            Ok(json!({"turn":{"id":"new-turn","status":"inProgress"}})),
        );
        let request = json!({"workspaceId":"local-test","threadId":"native-child","intent":"continue","idempotencyKey":key});
        let started = handle_bridge_request(
            ctx.clone(),
            "/multica/native-executions/intent",
            request.clone(),
        )
        .await;
        assert_eq!(started["status"], "ok", "{started}");
        assert_eq!(started["binding"]["state"], "dispatched", "{started}");
        assert_eq!(
            started["binding"]["bindingId"],
            queued["binding"]["bindingId"]
        );
        assert_eq!(started["binding"]["codexExecutionId"], "new-turn");
        let calls_before_replay = transport.calls().len();
        assert_eq!(
            handle_bridge_request(
                ctx.clone(),
                "/multica/native-executions/intent",
                request.clone()
            )
            .await["status"],
            "ok"
        );
        assert_eq!(transport.calls().len(), calls_before_replay);
        transport.push_response(CodexPageHostMethod::ThreadRead, Ok(json!({"thread":{"id":"native-child","turns":[{"id":"new-turn","status":"completed"}]}})));
        let terminal = handle_bridge_request(
            ctx.clone(),
            "/multica/executions/status",
            json!({"bindingId":queued["binding"]["bindingId"]}),
        )
        .await;
        assert_eq!(terminal["binding"]["state"], "completed", "{terminal}");
        let late_replay =
            handle_bridge_request(ctx, "/multica/native-executions/intent", request).await;
        assert_eq!(
            late_replay["binding"]["state"], "completed",
            "{late_replay}"
        );
        assert_eq!(
            transport
                .calls()
                .iter()
                .filter(|r| r.method == CodexPageHostMethod::TurnStart)
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn native_drag_active_noop_key_stays_noop_after_the_turn_finishes() {
    let (ctx, transport, _) = multica_execution_test_context();
    native_host_ready(&transport);
    transport.push_response(
        CodexPageHostMethod::ThreadRead,
        Ok(native_thread_fixture("inProgress")),
    );
    let request = json!({"workspaceId":"local-test","threadId":"native-child","intent":"continue","idempotencyKey":"already-running"});
    let active = handle_bridge_request(
        ctx.clone(),
        "/multica/native-executions/intent",
        request.clone(),
    )
    .await;
    assert_eq!(active["alreadyActive"], true, "{active}");
    for (workspace, thread) in [
        ("another-workspace", "native-child"),
        ("local-test", "another-thread"),
    ] {
        let rejected = handle_bridge_request(ctx.clone(), "/multica/native-executions/intent", json!({"workspaceId":workspace,"threadId":thread,"intent":"continue","idempotencyKey":"already-running"})).await;
        assert_eq!(
            rejected["message"], "execution_command_idempotency_conflict",
            "{rejected}"
        );
    }
    for _ in 0..2 {
        transport.push_response(
            CodexPageHostMethod::ThreadRead,
            Ok(native_thread_fixture("completed")),
        );
    }
    let replay = handle_bridge_request(ctx, "/multica/native-executions/intent", request).await;
    assert_eq!(replay["status"], "ok", "{replay}");
    assert_eq!(
        transport
            .calls()
            .iter()
            .filter(|r| r.method == CodexPageHostMethod::TurnStart)
            .count(),
        0
    );
}

#[tokio::test]
async fn native_drag_enqueue_key_recovers_dispatch_timeout_with_only_thread_read() {
    for (native_state, binding_state) in [
        ("inProgress", "running"),
        ("failed", "failed"),
        ("interrupted", "cancelled"),
        ("unknown", "reconciling"),
    ] {
        let (ctx, transport, _) = multica_execution_test_context();
        native_host_ready(&transport);
        transport.push_response(
            CodexPageHostMethod::ThreadRead,
            Ok(native_thread_fixture("completed")),
        );
        let request = json!({"workspaceId":"local-test","threadId":"native-child","intent":"enqueue","idempotencyKey":"queued-timeout"});
        let queued = handle_bridge_request(
            ctx.clone(),
            "/multica/native-executions/intent",
            request.clone(),
        )
        .await;
        transport.push_response(
            CodexPageHostMethod::ThreadRead,
            Ok(native_thread_fixture("completed")),
        );
        transport.push_response(
            CodexPageHostMethod::TurnStart,
            Err(anyhow::anyhow!("transport_timeout")),
        );
        let failed = handle_bridge_request(ctx.clone(), "/multica/executions/dispatch", json!({"bindingId":queued["binding"]["bindingId"], "expectedRevision":queued["binding"]["revision"], "leaseToken":"queue-worker"})).await;
        assert_eq!(failed["status"], "failed");
        let calls_before = transport.calls().len();
        transport.push_response(CodexPageHostMethod::ThreadRead, Ok(json!({"thread":{"id":"native-child","parentThreadId":"native-parent","turns":[{"id":"actual-turn","status":native_state}]}})));
        let result =
            handle_bridge_request(ctx.clone(), "/multica/native-executions/intent", request).await;
        assert_eq!(
            result["status"],
            if native_state == "unknown" {
                "failed"
            } else {
                "ok"
            },
            "{result}"
        );
        let listed = handle_bridge_request(
            ctx,
            "/multica/executions/list",
            json!({"workspaceId":"local-test"}),
        )
        .await;
        assert_eq!(listed["items"][0]["state"], binding_state, "{listed}");
        assert!(
            transport.calls()[calls_before..]
                .iter()
                .all(|r| r.method == CodexPageHostMethod::ThreadRead)
        );
        assert_eq!(
            transport
                .calls()
                .iter()
                .filter(|r| r.method == CodexPageHostMethod::TurnStart)
                .count(),
            1
        );
    }
}
