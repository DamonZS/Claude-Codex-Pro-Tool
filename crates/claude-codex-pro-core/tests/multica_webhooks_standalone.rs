use claude_codex_pro_core::multica_webhooks::{MulticaWebhookStore, WebhookTarget};
use serde_json::json;

#[test]
fn multica_webhooks_public_contract_never_restores_provisioned_plaintext() {
    let dir = tempfile::tempdir().unwrap();
    let store = MulticaWebhookStore::new(dir.path().join("webhooks.json"));
    let target = WebhookTarget::from_autopilot("workspace-a",&json!({
        "id":"auto-a", "status":"active", "triggers":[{"id":"trigger-a","kind":"webhook","enabled":true}]
    }),"trigger-a").unwrap();
    assert!(store.provision(&target, "create-a", 1).unwrap()["webhook_token"].is_string());
    assert!(store.trigger(&target).unwrap()["webhook_token"].is_null());
    assert!(store.provision(&target, "create-a", 2).unwrap()["webhook_token"].is_null());
    assert_eq!(
        target.ingress_path(),
        "/multica/webhooks/ingress/auto-a/trigger-a"
    );
}
