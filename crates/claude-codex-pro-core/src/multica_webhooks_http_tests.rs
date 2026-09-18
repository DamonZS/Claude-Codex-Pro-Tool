use super::handle_helper_connection_with_webhooks;
use crate::multica_execution_store::MulticaExecutionStore;
use crate::multica_webhooks::{MulticaWebhookStore, WebhookTarget, http};
use crate::multica_workspace::{
    LocalMulticaWorkspaceStore, LocalWorkspaceEntityUpsert, MulticaWorkspaceResourceKey,
};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn fixture() -> (tempfile::TempDir, http::WebhookIngress, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let ingress = http::WebhookIngress {
        workspace: LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json")),
        executions: MulticaExecutionStore::new(dir.path().join("execution.json")),
        webhooks: MulticaWebhookStore::new(dir.path().join("webhooks.json")),
    };
    let workspace_id = crate::multica_workspace::local_workspace_id();
    let auto = json!({"id":"http-auto", "title":"Ingress test", "status":"active", "execution_mode":"run_only", "assignee_id":"agent-a",
        "triggers":[{"id":"http-trigger","kind":"webhook","enabled":true}]});
    ingress
        .workspace
        .upsert(
            &workspace_id,
            LocalWorkspaceEntityUpsert {
                resource: MulticaWorkspaceResourceKey::Autopilots,
                entity: auto.clone(),
                expected_revision: Some(0),
            },
            1,
        )
        .unwrap();
    let target = WebhookTarget::from_autopilot(&workspace_id, &auto, "http-trigger").unwrap();
    let dto = ingress
        .webhooks
        .provision(&target, "http-provision", 2)
        .unwrap();
    (
        dir,
        ingress,
        dto["webhook_token"].as_str().unwrap().into(),
        target.ingress_path(),
    )
}

fn request(path: &str, token: &str, body: &[u8], extra: &str) -> Vec<u8> {
    let mut bytes = format!("POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nIdempotency-Key: http-delivery\r\nContent-Length: {}\r\n{extra}\r\n",body.len()).into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

async fn exchange(ingress: http::WebhookIngress, bytes: Vec<u8>, fragmented: bool) -> String {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, peer) = listener.accept().await.unwrap();
        handle_helper_connection_with_webhooks(stream, Some(peer), ingress)
            .await
            .unwrap();
    });
    let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
    if fragmented {
        for chunk in bytes.chunks(17) {
            client.write_all(chunk).await.unwrap();
            tokio::task::yield_now().await;
        }
    } else {
        client.write_all(&bytes).await.unwrap();
    }
    client.shutdown().await.unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        client.read_to_end(&mut response),
    )
    .await
    .unwrap()
    .unwrap();
    server.await.unwrap();
    String::from_utf8(response).unwrap()
}

fn response_json(response: &str) -> Value {
    serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
}

#[tokio::test]
async fn published_helper_url_accepts_separate_one_time_bearer_credential() {
    let (dir, ingress, _, _) = fixture();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let store =
        MulticaWebhookStore::new(dir.path().join("webhooks.json")).with_helper_port(address.port());
    let workspace = crate::multica_workspace::local_workspace_id();
    let autopilots = ingress
        .workspace
        .list(&workspace, MulticaWorkspaceResourceKey::Autopilots)
        .unwrap();
    let target = WebhookTarget::from_autopilot(&workspace, &autopilots[0], "http-trigger").unwrap();
    let dto = store.rotate(&target, 1, "url-rotation", 3).unwrap();
    let url = reqwest::Url::parse(dto["webhook_url"].as_str().unwrap()).unwrap();
    assert_eq!(url.port(), Some(address.port()));
    let token = dto["webhook_token"].as_str().unwrap();
    assert!(!url.as_str().contains(token));
    assert!(url.query().is_none());
    let server = tokio::spawn(async move {
        let (stream, peer) = listener.accept().await.unwrap();
        handle_helper_connection_with_webhooks(stream, Some(peer), ingress)
            .await
            .unwrap();
    });
    let response = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .post(url)
        .bearer_auth(token)
        .header("Idempotency-Key", "copyable-url")
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    let body = response.text().await.unwrap();
    assert!(!body.contains(token));
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["status"],
        "queued"
    );
    server.await.unwrap();
    assert!(store.trigger(&target).unwrap()["webhook_token"].is_null());
    assert!(store.rotate(&target, 1, "url-rotation", 4).unwrap()["webhook_token"].is_null());
}

#[tokio::test]
async fn real_helper_http_queues_one_run_with_server_identity_and_no_host() {
    let (_dir, ingress, token, path) = fixture();
    let body = br#"{"workspace_id":"attacker-workspace","prompt":"never-execute-in-handler"}"#;
    let first = exchange(
        ingress.clone(),
        request(
            &path,
            &token,
            body,
            "X-Workspace-Id: attacker-workspace\r\n",
        ),
        true,
    )
    .await;
    assert!(first.starts_with("HTTP/1.1 202 Accepted"));
    assert!(!first.contains(&token));
    assert!(!first.contains("never-execute-in-handler"));
    assert!(!first.contains("Access-Control-Allow-Origin"));
    let json = response_json(&first);
    assert_eq!(json["status"], "queued");
    assert_eq!(json["duplicate"], false);
    assert_eq!(json.as_object().unwrap().len(), 3);
    for _ in 0..2 {
        let response = exchange(ingress.clone(), request(&path, &token, body, ""), false).await;
        assert_eq!(response_json(&response)["delivery_id"], json["delivery_id"]);
        assert_eq!(response_json(&response)["duplicate"], true);
    }
    let runs = ingress.executions.list_autopilot_runs("http-auto").unwrap();
    assert_eq!(runs.len(), 1);
    assert!(runs[0].task_id.is_none());
    assert_eq!(runs[0].status, "pending");
    let deliveries = ingress
        .webhooks
        .list(
            &crate::multica_workspace::local_workspace_id(),
            "http-auto",
            10,
            0,
        )
        .unwrap();
    assert_eq!(deliveries["total"], 1);
    assert_eq!(deliveries["deliveries"][0]["attempt_count"], 3);
    assert_eq!(
        deliveries["deliveries"][0]["workspace_id"],
        crate::multica_workspace::local_workspace_id()
    );
}

#[tokio::test]
async fn real_helper_http_rejects_wrong_missing_rotated_credentials_and_query_tokens() {
    let (_dir, ingress, token, path) = fixture();
    for bytes in [
        request(&path, &"0".repeat(64), b"{}", ""),
        format!("POST {path} HTTP/1.1\r\nContent-Length: 2\r\n\r\n{{}}").into_bytes(),
    ] {
        let response = exchange(ingress.clone(), bytes, false).await;
        assert!(response.starts_with("HTTP/1.1 401 Unauthorized"));
    }
    let response = exchange(
        ingress.clone(),
        request(&format!("{path}?token={token}"), &token, b"{}", ""),
        false,
    )
    .await;
    assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
    assert!(!response.contains(&token));
    let workspace_id = crate::multica_workspace::local_workspace_id();
    let auto = ingress
        .workspace
        .list(&workspace_id, MulticaWorkspaceResourceKey::Autopilots)
        .unwrap()
        .remove(0);
    let target = WebhookTarget::from_autopilot(&workspace_id, &auto, "http-trigger").unwrap();
    ingress
        .webhooks
        .rotate(&target, 1, "http-rotate", 3)
        .unwrap();
    let response = exchange(ingress.clone(), request(&path, &token, b"{}", ""), false).await;
    assert!(response.starts_with("HTTP/1.1 401 Unauthorized"));
    assert!(
        ingress
            .executions
            .list_autopilot_runs("http-auto")
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn real_helper_http_preserves_non_utf8_body_for_signature_verification() {
    use sha2::{Digest, Sha256};
    let (_dir, ingress, token, path) = fixture();
    let body = [0xff, 0xfe, 0x00, b'\n', b'\r'];
    let mut ipad = [0x36; 64];
    let mut opad = [0x5c; 64];
    for (index, byte) in token.bytes().enumerate() {
        ipad[index] ^= byte;
        opad[index] ^= byte;
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(body);
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner.finalize());
    let signature: String = outer
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let response = exchange(
        ingress.clone(),
        request(
            &path,
            &token,
            &body,
            &format!("X-Hub-Signature-256: sha256={signature}\r\n"),
        ),
        true,
    )
    .await;
    assert!(response.starts_with("HTTP/1.1 202 Accepted"));
    let detail = ingress
        .webhooks
        .get(
            &crate::multica_workspace::local_workspace_id(),
            "http-auto",
            response_json(&response)["delivery_id"].as_str().unwrap(),
        )
        .unwrap();
    assert_eq!(detail["signature_status"], "valid");
    assert!(detail["raw_body"].is_null());
}

#[tokio::test]
async fn real_helper_http_rejects_smuggling_oversize_truncation_and_invalid_methods() {
    let (_dir, ingress, token, path) = fixture();
    let cases = [
        (
            request(&path, &token, b"{}", "Content-Length: 2\r\n"),
            "400",
        ),
        (
            request(&path, &token, b"{}", "Transfer-Encoding: chunked\r\n"),
            "400",
        ),
        (
            request(&path, &token, b"{}", "Content-Encoding: gzip\r\n"),
            "400",
        ),
        (
            request(&path, &token, b"{}", "Authorization: Bearer extra\r\n"),
            "400",
        ),
        (
            format!("POST {path} HTTP/1.1\r\nContent-Length: 262145\r\n\r\n").into_bytes(),
            "413",
        ),
        (
            format!("POST {path} HTTP/1.1\r\nContent-Length: 20\r\n\r\nshort").into_bytes(),
            "400",
        ),
        (
            format!("POST {path} HTTP/1.1\r\nContent-Length: 2\r\n\r\n{{}}extra").into_bytes(),
            "400",
        ),
        (format!("POST {path} HTTP/1.1\r\n\r\n").into_bytes(), "411"),
        (
            format!("GET {path} HTTP/1.1\r\nContent-Length: 0\r\n\r\n").into_bytes(),
            "405",
        ),
        (
            format!("OPTIONS {path} HTTP/1.1\r\nContent-Length: 0\r\n\r\n").into_bytes(),
            "405",
        ),
        (request(&format!("{path}/extra"), &token, b"{}", ""), "400"),
        (
            request(
                &path,
                &token,
                b"{}",
                &format!("Padding: {}\r\n", "x".repeat(8200)),
            ),
            "413",
        ),
    ];
    for (bytes, status) in cases {
        let response = exchange(ingress.clone(), bytes, false).await;
        assert!(
            response.starts_with(&format!("HTTP/1.1 {status}")),
            "unexpected response: {response}"
        );
        assert!(!response.contains(&token));
    }
    assert!(
        ingress
            .executions
            .list_autopilot_runs("http-auto")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn webhook_dispatch_precedes_generic_request_logging() {
    let source = include_str!("launcher.rs");
    let handler = source
        .split("async fn handle_helper_connection_with_webhooks(")
        .nth(1)
        .unwrap();
    assert!(
        handler.find("if http::is_candidate").unwrap()
            < handler.find("append_diagnostic_log").unwrap()
    );
    assert!(
        handler.find("if http::is_candidate").unwrap()
            < handler.find("String::from_utf8_lossy").unwrap()
    );
    let http_source = include_str!("multica_webhooks_http.rs");
    assert!(!http_source.contains("append_diagnostic_log"));
    assert!(!http_source.contains("multica_autopilot_trigger"));
    assert!(!http_source.contains("CodexExecutionService"));
}

#[tokio::test]
async fn real_helper_http_rejects_bad_signature_and_current_disabled_trigger() {
    let (_dir, ingress, token, path) = fixture();
    let response = exchange(
        ingress.clone(),
        request(
            &path,
            &token,
            b"{}",
            "X-Hub-Signature-256: sha256=invalid\r\n",
        ),
        false,
    )
    .await;
    assert!(response.starts_with("HTTP/1.1 401 Unauthorized"));
    let workspace_id = crate::multica_workspace::local_workspace_id();
    let mut auto = ingress
        .workspace
        .list(&workspace_id, MulticaWorkspaceResourceKey::Autopilots)
        .unwrap()
        .remove(0);
    let revision = auto["revision"].as_u64().unwrap();
    auto["triggers"][0]["enabled"] = json!(false);
    ingress
        .workspace
        .upsert(
            &workspace_id,
            LocalWorkspaceEntityUpsert {
                resource: MulticaWorkspaceResourceKey::Autopilots,
                entity: auto,
                expected_revision: Some(revision),
            },
            3,
        )
        .unwrap();
    let bytes = String::from_utf8(request(&path, &token, b"{}", ""))
        .unwrap()
        .replace("http-delivery", "disabled-delivery")
        .into_bytes();
    let response = exchange(ingress.clone(), bytes, false).await;
    assert!(response.starts_with("HTTP/1.1 409 Conflict"));
    assert_eq!(response_json(&response)["code"], "webhook_trigger_inactive");
    assert!(
        ingress
            .executions
            .list_autopilot_runs("http-auto")
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn real_helper_http_incomplete_body_has_bounded_read_deadline() {
    let (_dir, ingress, _token, path) = fixture();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, peer) = listener.accept().await.unwrap();
        handle_helper_connection_with_webhooks(stream, Some(peer), ingress)
            .await
            .unwrap();
    });
    let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
    client
        .write_all(format!("POST {path} HTTP/1.1\r\nContent-Length: 2\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(12),
        client.read_to_end(&mut response),
    )
    .await
    .unwrap()
    .unwrap();
    server.await.unwrap();
    assert!(
        String::from_utf8(response)
            .unwrap()
            .starts_with("HTTP/1.1 408 Request Timeout")
    );
}
