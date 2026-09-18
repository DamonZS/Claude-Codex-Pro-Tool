//! Local webhook ingress for the existing helper, never a separate listener.
//!
//! Integration: resolve `WebhookTarget` from the authoritative autopilot for every
//! operation. Provision/rotate return a one-time `webhook_token`; normal reads do
//! not. POST `ingress_path()` with `Authorization: Bearer <token>` and either
//! `Idempotency-Key`, `X-Webhook-Id`, or `X-GitHub-Delivery`. Optional
//! `X-Hub-Signature-256: sha256=<hex>` is HMAC-SHA256(token, exact request bytes).
//! Pass bounded raw bytes/headers to `receive`, then `dispatch` accepted deliveries
//! through the existing BridgeRuntimeService. `pending` supports crash recovery.
//! Management routes must retain the helper's normal authentication; only this
//! exact ingress path uses the per-trigger credential. No credential in a URL.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use anyhow::{anyhow, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::multica_execution_store::MulticaExecutionStore;
use crate::routes::{BridgeRuntimeService, MulticaAutopilotTriggerRequest};

#[path = "multica_webhooks_http.rs"]
pub(crate) mod http;

pub const MAX_WEBHOOK_BODY_BYTES: usize = 256 * 1024;
pub const MAX_WEBHOOK_HEADER_BYTES: usize = 8192;
const MAX_DELIVERIES: usize = 256;
const MAX_RECEIPTS: usize = 4096;
const MAX_CREDENTIALS: usize = 512;
const MAX_STORE_BYTES: usize = 8 * 1024 * 1024;
const MAX_AUDIT_BODY_BYTES: usize = 8192;

#[derive(Clone)]
pub struct WebhookTarget {
    workspace_id: String,
    autopilot_id: String,
    trigger_id: String,
    active: bool,
    trigger: Value,
}

impl WebhookTarget {
    pub fn from_autopilot(
        workspace_id: &str,
        autopilot: &Value,
        trigger_id: &str,
    ) -> anyhow::Result<Self> {
        validate_id(workspace_id)?;
        validate_id(trigger_id)?;
        let autopilot_id = autopilot["id"].as_str().unwrap_or_default();
        validate_id(autopilot_id)?;
        if autopilot
            .get("workspace_id")
            .and_then(Value::as_str)
            .is_some_and(|id| id != workspace_id)
        {
            bail!("webhook_target_not_found");
        }
        let trigger = autopilot["triggers"]
            .as_array()
            .and_then(|items| items.iter().find(|item| item["id"] == trigger_id))
            .filter(|item| matches!(item["kind"].as_str(), Some("webhook" | "api")))
            .ok_or_else(|| anyhow!("webhook_target_not_found"))?
            .clone();
        validate_filters(&trigger["event_filters"])?;
        Ok(Self {
            workspace_id: workspace_id.into(),
            autopilot_id: autopilot_id.into(),
            trigger_id: trigger_id.into(),
            active: autopilot["status"].as_str().unwrap_or("active") == "active"
                && trigger["enabled"].as_bool() == Some(true),
            trigger,
        })
    }

    pub fn ingress_path(&self) -> String {
        format!(
            "/multica/webhooks/ingress/{}/{}",
            self.autopilot_id, self.trigger_id
        )
    }

    fn matches(&self, credential: &Credential) -> bool {
        credential.workspace_id == self.workspace_id
            && credential.autopilot_id == self.autopilot_id
            && credential.trigger_id == self.trigger_id
    }

    fn owns(&self, delivery: &Value) -> bool {
        delivery["workspace_id"] == self.workspace_id
            && delivery["autopilot_id"] == self.autopilot_id
            && delivery["trigger_id"] == self.trigger_id
    }

    fn dto(&self, token: Option<&str>, revision: u64, now_ms: u64) -> Value {
        // Copy only upstream public fields, never arbitrary stored metadata.
        let mut dto = json!({
            "id":self.trigger_id, "autopilot_id":self.autopilot_id,
            "kind":self.trigger["kind"], "enabled":self.trigger["enabled"],
            "cron_expression":null, "timezone":null, "next_run_at":null,
            "webhook_token":token, "webhook_path":self.ingress_path(),
            "webhook_url":null, "credential_revision":revision,
            "label":self.trigger["label"], "event_filters":self.trigger["event_filters"],
            "last_fired_at":self.trigger["last_fired_at"],
            "created_at":timestamp(self.trigger["created_at_ms"].as_u64().unwrap_or(now_ms)),
            "updated_at":timestamp(now_ms),
        });
        if let Some(created) = self.trigger["created_at"].as_str() {
            dto["created_at"] = json!(created);
        }
        dto
    }
}

#[derive(Serialize, Deserialize)]
struct Credential {
    workspace_id: String,
    autopilot_id: String,
    trigger_id: String,
    token_digest: [u8; 32],
    revision: u64,
    updated_at_ms: u64,
}

#[derive(Serialize, Deserialize)]
struct Receipt {
    key: String,
    signature: String,
    delivery_id: Option<String>,
    credential_revision: Option<u64>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    credentials: Vec<Credential>,
    deliveries: Vec<Value>,
    receipts: Vec<Receipt>,
}

#[derive(Clone)]
pub struct MulticaWebhookStore {
    path: PathBuf,
    helper_port: Option<u16>,
}

pub struct WebhookReceipt {
    pub delivery: Value,
    pub http_status: u16,
    pub duplicate: bool,
    pub should_dispatch: bool,
}

impl Default for MulticaWebhookStore {
    fn default() -> Self {
        Self::new(crate::paths::default_multica_state_dir().join("webhooks.json"))
    }
}

impl MulticaWebhookStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            helper_port: None,
        }
    }

    pub fn with_helper_port(mut self, port: u16) -> Self {
        self.helper_port = (port != 0).then_some(port);
        self
    }

    fn trigger_dto(
        &self,
        target: &WebhookTarget,
        token: Option<&str>,
        revision: u64,
        now_ms: u64,
    ) -> Value {
        let mut dto = target.dto(token, revision, now_ms);
        if let Some(port) = self.helper_port {
            dto["webhook_url"] = json!(format!("http://127.0.0.1:{port}{}", target.ingress_path()));
        }
        dto
    }

    pub fn provision(
        &self,
        target: &WebhookTarget,
        command_id: &str,
        now_ms: u64,
    ) -> anyhow::Result<Value> {
        self.change_credential(target, command_id, None, now_ms)
    }

    pub fn rotate(
        &self,
        target: &WebhookTarget,
        expected_revision: u64,
        command_id: &str,
        now_ms: u64,
    ) -> anyhow::Result<Value> {
        self.change_credential(target, command_id, Some(expected_revision), now_ms)
    }

    fn change_credential(
        &self,
        target: &WebhookTarget,
        command_id: &str,
        expected: Option<u64>,
        now_ms: u64,
    ) -> anyhow::Result<Value> {
        self.change_credential_with_signature(target, command_id, expected, None, now_ms)
    }

    /// Original request signatures survive refresh even when the adapter reads
    /// a newer CAS revision. Only a receipt match skips the first-write CAS.
    pub fn rotate_with_signature(
        &self,
        target: &WebhookTarget,
        expected_revision: u64,
        command_id: &str,
        command_signature: &str,
        now_ms: u64,
    ) -> anyhow::Result<Value> {
        if command_signature.len() != 64
            || !command_signature
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            bail!("webhook_command_signature_invalid");
        }
        self.change_credential_with_signature(
            target,
            command_id,
            Some(expected_revision),
            Some(command_signature),
            now_ms,
        )
    }

    fn change_credential_with_signature(
        &self,
        target: &WebhookTarget,
        command_id: &str,
        expected: Option<u64>,
        command_signature: Option<&str>,
        now_ms: u64,
    ) -> anyhow::Result<Value> {
        validate_id(command_id)?;
        let key = digest_parts(&["credential-command", command_id]);
        let signature = if let Some(original) = command_signature {
            digest_parts(&[
                "rotate-original-request",
                &target.workspace_id,
                &target.autopilot_id,
                &target.trigger_id,
                original,
            ])
        } else {
            digest_parts(&[
                &target.workspace_id,
                &target.autopilot_id,
                &target.trigger_id,
                &expected
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "provision".into()),
            ])
        };
        self.update(|state| {
            if let Some(receipt) = state.receipts.iter().find(|receipt| receipt.key == key) {
                if receipt.signature != signature {
                    bail!("webhook_command_conflict");
                }
                let credential = state
                    .credentials
                    .iter()
                    .find(|entry| target.matches(entry))
                    .ok_or_else(|| anyhow!("webhook_credential_revoked"))?;
                // Plaintext is deliberately not recoverable from a persisted receipt.
                let mut dto =
                    self.trigger_dto(target, None, credential.revision, credential.updated_at_ms);
                dto["credential_replay"] = json!(true);
                return Ok(dto);
            }
            ensure_receipt_capacity(state)?;
            let index = state
                .credentials
                .iter()
                .position(|entry| target.matches(entry));
            let revision = match (index, expected) {
                (None, None) => {
                    if state.credentials.len() >= MAX_CREDENTIALS {
                        bail!("webhook_credentials_full");
                    }
                    1
                }
                (Some(i), Some(revision)) if revision == state.credentials[i].revision => revision
                    .checked_add(1)
                    .ok_or_else(|| anyhow!("webhook_revision_conflict"))?,
                _ => bail!("webhook_revision_conflict"),
            };
            // UUID v4 uses the OS-backed RNG; two independent values supply 244 random bits.
            let token = Zeroizing::new(format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ));
            let credential = Credential {
                workspace_id: target.workspace_id.clone(),
                autopilot_id: target.autopilot_id.clone(),
                trigger_id: target.trigger_id.clone(),
                token_digest: Sha256::digest(token.as_bytes()).into(),
                revision,
                updated_at_ms: now_ms,
            };
            if let Some(i) = index {
                state.credentials[i] = credential;
            } else {
                state.credentials.push(credential);
            }
            state.receipts.push(Receipt {
                key,
                signature,
                delivery_id: None,
                credential_revision: Some(revision),
            });
            Ok(self.trigger_dto(target, Some(&token), revision, now_ms))
        })
    }

    pub fn trigger(&self, target: &WebhookTarget) -> anyhow::Result<Value> {
        let state = self.load()?;
        let credential = state
            .credentials
            .iter()
            .find(|entry| target.matches(entry))
            .ok_or_else(|| anyhow!("webhook_credential_not_found"))?;
        Ok(self.trigger_dto(target, None, credential.revision, credential.updated_at_ms))
    }

    pub fn revoke(&self, target: &WebhookTarget, expected_revision: u64) -> anyhow::Result<()> {
        self.update(|state| {
            if let Some(index) = state
                .credentials
                .iter()
                .position(|entry| target.matches(entry))
            {
                if state.credentials[index].revision != expected_revision {
                    bail!("webhook_revision_conflict");
                }
                state.credentials.remove(index);
            }
            Ok(())
        })
    }

    pub fn receive(
        &self,
        target: &WebhookTarget,
        headers: &BTreeMap<String, String>,
        raw_body: &[u8],
        now_ms: u64,
    ) -> anyhow::Result<WebhookReceipt> {
        if raw_body.len() > MAX_WEBHOOK_BODY_BYTES {
            bail!("webhook_body_too_large");
        }
        let headers = normalize_headers(headers)?;
        let authorization = headers
            .get("authorization")
            .and_then(|s| s.strip_prefix("Bearer "));
        let other = headers.get("x-webhook-token").map(String::as_str);
        if authorization.is_some() && other.is_some() && authorization != other {
            bail!("webhook_credential_invalid");
        }
        let token = Zeroizing::new(authorization.or(other).unwrap_or_default().to_owned());
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("webhook_credential_invalid");
        }
        let body: Option<Value> = serde_json::from_slice(raw_body).ok();
        let event = headers
            .get("x-github-event")
            .or_else(|| headers.get("x-webhook-event"))
            .map(String::as_str)
            .or_else(|| body.as_ref().and_then(|b| b["event"].as_str()))
            .unwrap_or("webhook");
        validate_label(event)?;
        let dedupe_header = ["idempotency-key", "x-webhook-id", "x-github-delivery"]
            .into_iter()
            .find_map(|name| headers.get(name).map(|value| (name, value.as_str())));
        if let Some((_, key)) = dedupe_header {
            validate_label(key)?;
        }
        let body_digest = hex(&Sha256::digest(raw_body));
        let dedupe = digest_parts(&[
            "delivery",
            &target.workspace_id,
            &target.autopilot_id,
            &target.trigger_id,
            dedupe_header.map(|pair| pair.1).unwrap_or(&body_digest),
        ]);
        let signature = digest_parts(&[&body_digest, event]);
        self.update(|state| {
            let credential = state.credentials.iter().find(|entry| target.matches(entry))
                .ok_or_else(|| anyhow!("webhook_credential_invalid"))?;
            if !constant_eq(&credential.token_digest, &Sha256::digest(token.as_bytes())) { bail!("webhook_credential_invalid"); }
            let signature_status = match headers.get("x-hub-signature-256") {
                None => "not_required",
                Some(value) if verify_signature(&token, raw_body, value) => "valid",
                Some(_) => "invalid",
            };
            // Authentication is rechecked before returning even a prior receipt.
            if let Some(receipt) = state.receipts.iter().find(|receipt| receipt.key == dedupe) {
                if receipt.signature != signature { bail!("webhook_idempotency_conflict"); }
                if signature_status == "invalid" { bail!("webhook_signature_invalid"); }
                let id = receipt.delivery_id.as_deref().unwrap_or_default();
                let delivery = state.deliveries.iter_mut().find(|row| row["id"] == id)
                    .ok_or_else(|| anyhow!("webhook_delivery_expired"))?;
                delivery["attempt_count"] = json!(delivery["attempt_count"].as_u64().unwrap_or(0).saturating_add(1));
                delivery["last_attempt_at"] = json!(timestamp(now_ms));
                return Ok(receipt_for(delivery.clone(), true, target.active));
            }
            ensure_receipt_capacity(state)?;
            let filtered = !matches_event(&target.trigger["event_filters"], event,
                body.as_ref().and_then(|b| b["action"].as_str()));
            let (status, response, reason) = if signature_status == "invalid" {
                ("rejected", 401, Some("webhook_signature_invalid"))
            } else if !target.active {
                ("ignored", 409, Some("webhook_trigger_inactive"))
            } else if filtered {
                ("ignored", 202, Some("webhook_event_filtered"))
            } else { ("queued", 202, None) };
            let mut selected = serde_json::Map::new();
            for name in ["content-type", "x-github-event", "x-webhook-event"] {
                if let Some(value) = headers.get(name) { selected.insert(name.into(), json!(redact_text(value, &token))); }
            }
            let id = format!("delivery-{dedupe}");
            let at = timestamp(now_ms);
            let delivery = json!({
                "id":id, "workspace_id":target.workspace_id, "autopilot_id":target.autopilot_id,
                "trigger_id":target.trigger_id, "provider":if headers.contains_key("x-github-event") {"github"} else {"generic"},
                "event":redact_text(event, &token), "dedupe_key":dedupe, "dedupe_source":dedupe_header.map(|p| p.0).unwrap_or("body_sha256"),
                "signature_status":signature_status, "status":status, "attempt_count":1,
                "dispatch_attempts":0, "available_at":at, "content_type":headers.get("content-type").map(|v| redact_text(v,&token)),
                "response_status":response, "autopilot_run_id":null, "replayed_from_delivery_id":null,
                "error":reason, "reason_code":reason, "replay_idempotency_key":null,
                "received_at":at, "last_attempt_at":at, "created_at":at,
                "selected_headers":selected, "raw_body":if signature_status == "invalid" { None } else { audit_body(body.clone(), &token) }, "response_body":null,
            });
            insert_delivery(state, delivery.clone())?;
            state.receipts.push(Receipt { key:dedupe, signature, delivery_id:Some(id), credential_revision:None });
            Ok(receipt_for(delivery, false, target.active))
        })
    }

    /// Helper ingress only reserves the existing engine's durable occurrence.
    /// It performs no Host calls. The normal autopilot tick materializes this run.
    pub fn receive_and_enqueue(
        &self,
        executions: &MulticaExecutionStore,
        target: &WebhookTarget,
        headers: &BTreeMap<String, String>,
        raw_body: &[u8],
        now_ms: u64,
    ) -> anyhow::Result<WebhookReceipt> {
        let mut receipt = self.receive(target, headers, raw_body, now_ms)?;
        if receipt.should_dispatch {
            let id = receipt.delivery["id"]
                .as_str()
                .ok_or_else(|| anyhow!("webhook_store_invalid"))?;
            receipt.delivery = self.enqueue(executions, target, id, now_ms)?;
            receipt.http_status =
                receipt.delivery["response_status"].as_u64().unwrap_or(202) as u16;
        }
        Ok(receipt)
    }

    /// Call for `pending()` rows before/after the normal background autopilot tick.
    /// Repeating the reservation recovers either side of a two-store crash window.
    pub fn enqueue(
        &self,
        executions: &MulticaExecutionStore,
        target: &WebhookTarget,
        delivery_id: &str,
        now_ms: u64,
    ) -> anyhow::Result<Value> {
        if !target.active {
            bail!("webhook_trigger_inactive");
        }
        self.update(|state| {
            if !state.credentials.iter().any(|entry| target.matches(entry)) {
                bail!("webhook_credential_not_found");
            }
            let row = state
                .deliveries
                .iter_mut()
                .find(|row| target.owns(row) && row["id"] == delivery_id)
                .ok_or_else(|| anyhow!("webhook_delivery_not_found"))?;
            if !matches!(row["status"].as_str(), Some("queued" | "failed")) {
                return Ok(row.clone());
            }
            let run = executions.reserve_autopilot_occurrence(
                target.autopilot_id.clone(),
                Some(target.trigger_id.clone()),
                target.trigger["kind"].as_str().unwrap_or("webhook").into(),
                Some(delivery_id.into()),
                now_ms,
            )?;
            row["autopilot_run_id"] = json!(run.id);
            if run.status == "failed" && run.task_id.is_none() {
                row["status"] = json!("failed");
                row["error"] = json!(
                    run.failure_reason
                        .as_deref()
                        .or(run.reason_code.as_deref())
                        .unwrap_or("webhook_dispatch_failed")
                );
                row["reason_code"] = json!(
                    run.reason_code
                        .as_deref()
                        .unwrap_or("webhook_dispatch_failed")
                );
                row["response_status"] = json!(503);
                return Ok(row.clone());
            }
            row["error"] = Value::Null;
            row["reason_code"] = Value::Null;
            row["response_status"] = json!(202);
            row["status"] = json!(if run.task_id.is_some() || run.status != "pending" {
                // Tick handed off this occurrence; recovery is not another attempt.
                row["dispatch_attempts"] =
                    json!(row["dispatch_attempts"].as_u64().unwrap_or(0).max(1));
                "dispatched"
            } else {
                "queued"
            });
            Ok(row.clone())
        })
    }

    pub fn list(
        &self,
        workspace_id: &str,
        autopilot_id: &str,
        limit: usize,
        offset: usize,
    ) -> anyhow::Result<Value> {
        validate_id(workspace_id)?;
        validate_id(autopilot_id)?;
        if limit == 0 || limit > 100 || offset > MAX_DELIVERIES {
            bail!("webhook_pagination_invalid");
        }
        let rows: Vec<_> = self
            .load()?
            .deliveries
            .into_iter()
            .rev()
            .filter(|row| {
                row["workspace_id"] == workspace_id && row["autopilot_id"] == autopilot_id
            })
            .collect();
        let total = rows.len();
        let deliveries: Vec<_> = rows
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|mut row| {
                for key in ["raw_body", "selected_headers", "response_body"] {
                    row.as_object_mut().unwrap().remove(key);
                }
                row
            })
            .collect();
        Ok(json!({"deliveries":deliveries, "total":total}))
    }

    pub fn get(
        &self,
        workspace_id: &str,
        autopilot_id: &str,
        delivery_id: &str,
    ) -> anyhow::Result<Value> {
        validate_id(workspace_id)?;
        validate_id(autopilot_id)?;
        validate_id(delivery_id)?;
        self.load()?
            .deliveries
            .into_iter()
            .find(|row| {
                row["workspace_id"] == workspace_id
                    && row["autopilot_id"] == autopilot_id
                    && row["id"] == delivery_id
            })
            .ok_or_else(|| anyhow!("webhook_delivery_not_found"))
    }

    pub fn replay(
        &self,
        target: &WebhookTarget,
        delivery_id: &str,
        command_id: &str,
        now_ms: u64,
    ) -> anyhow::Result<Value> {
        validate_id(delivery_id)?;
        validate_id(command_id)?;
        if !target.active {
            bail!("webhook_trigger_inactive");
        }
        let key = digest_parts(&["replay", command_id]);
        let signature = digest_parts(&[
            &target.workspace_id,
            &target.autopilot_id,
            &target.trigger_id,
            delivery_id,
        ]);
        self.update(|state| {
            if !state.credentials.iter().any(|entry| target.matches(entry)) {
                bail!("webhook_credential_not_found");
            }
            if let Some(receipt) = state.receipts.iter().find(|receipt| receipt.key == key) {
                if receipt.signature != signature {
                    bail!("webhook_command_conflict");
                }
                return state
                    .deliveries
                    .iter()
                    .find(|row| row["id"].as_str() == receipt.delivery_id.as_deref())
                    .cloned()
                    .ok_or_else(|| anyhow!("webhook_delivery_expired"));
            }
            let mut row = state
                .deliveries
                .iter()
                .find(|row| target.owns(row) && row["id"] == delivery_id)
                .cloned()
                .ok_or_else(|| anyhow!("webhook_delivery_not_found"))?;
            if row["status"] == "rejected"
                || !matches!(
                    row["signature_status"].as_str(),
                    Some("valid" | "not_required")
                )
            {
                bail!("webhook_replay_rejected");
            }
            ensure_receipt_capacity(state)?;
            let id = format!("delivery-{key}");
            row["id"] = json!(id);
            row["replayed_from_delivery_id"] = json!(delivery_id);
            row["replay_idempotency_key"] = json!(key);
            row["status"] = json!("queued");
            row["attempt_count"] = json!(1);
            row["dispatch_attempts"] = json!(0);
            row["response_status"] = json!(202);
            for field in ["autopilot_run_id", "error", "reason_code", "response_body"] {
                row[field] = Value::Null;
            }
            for field in [
                "available_at",
                "received_at",
                "last_attempt_at",
                "created_at",
            ] {
                row[field] = json!(timestamp(now_ms));
            }
            insert_delivery(state, row.clone())?;
            state.receipts.push(Receipt {
                key,
                signature,
                delivery_id: Some(id),
                credential_revision: None,
            });
            Ok(row)
        })
    }

    /// The existing engine owns run reservation, capacity and native Host dispatch.
    /// A crash after its commit is retried with the same occurrence, not a new run.
    pub async fn dispatch(
        &self,
        runtime: &dyn BridgeRuntimeService,
        target: &WebhookTarget,
        delivery_id: &str,
        now_ms: u64,
    ) -> anyhow::Result<Value> {
        validate_id(delivery_id)?;
        if !target.active {
            bail!("webhook_trigger_inactive");
        }
        let row = self.update(|state| {
            if !state.credentials.iter().any(|entry| target.matches(entry)) {
                bail!("webhook_credential_not_found");
            }
            let row = state
                .deliveries
                .iter_mut()
                .find(|row| target.owns(row) && row["id"] == delivery_id)
                .ok_or_else(|| anyhow!("webhook_delivery_not_found"))?;
            if matches!(row["status"].as_str(), Some("queued" | "failed")) {
                row["status"] = json!("queued");
                row["dispatch_attempts"] = json!(
                    row["dispatch_attempts"]
                        .as_u64()
                        .unwrap_or(0)
                        .saturating_add(1)
                );
                row["last_attempt_at"] = json!(timestamp(now_ms));
            }
            Ok(row.clone())
        })?;
        if !matches!(row["status"].as_str(), Some("queued" | "failed")) {
            return Ok(row);
        }
        let result = runtime
            .multica_autopilot_trigger(MulticaAutopilotTriggerRequest {
                autopilot_id: target.autopilot_id.clone(),
                trigger_id: Some(target.trigger_id.clone()),
                source: target.trigger["kind"].as_str().unwrap_or("webhook").into(),
                occurrence_id: Some(delivery_id.into()),
            })
            .await;
        self.update(|state| {
            let row = state
                .deliveries
                .iter_mut()
                .find(|row| target.owns(row) && row["id"] == delivery_id)
                .ok_or_else(|| anyhow!("webhook_delivery_not_found"))?;
            // A slower failed concurrent caller must not undo a committed dispatch.
            if row["status"] == "dispatched" {
                return Ok(row.clone());
            }
            match result {
                Ok(value)
                    if value["run"]["id"]
                        .as_str()
                        .is_some_and(|id| validate_id(id).is_ok()) =>
                {
                    row["status"] = json!("dispatched");
                    row["autopilot_run_id"] = value["run"]["id"].clone();
                    row["error"] = Value::Null;
                    row["reason_code"] = Value::Null;
                    row["response_status"] = json!(202);
                }
                _ => {
                    row["status"] = json!("failed");
                    row["error"] = json!("webhook_dispatch_failed");
                    row["reason_code"] = json!("webhook_dispatch_failed");
                    row["response_status"] = json!(503);
                }
            }
            Ok(row.clone())
        })
    }

    pub fn pending(&self, limit: usize) -> anyhow::Result<Vec<Value>> {
        if limit == 0 || limit > 100 {
            bail!("webhook_pagination_invalid");
        }
        Ok(self.load()?.deliveries.into_iter().filter(|row| row["status"] == "queued").take(limit).map(|row|
            json!({"id":row["id"], "workspace_id":row["workspace_id"], "autopilot_id":row["autopilot_id"], "trigger_id":row["trigger_id"]})
        ).collect())
    }

    fn load(&self) -> anyhow::Result<State> {
        let file = match fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(State::default());
            }
            Err(_) => bail!("webhook_store_read_failed"),
        };
        let mut bytes = Vec::new();
        file.take((MAX_STORE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| anyhow!("webhook_store_read_failed"))?;
        if bytes.len() > MAX_STORE_BYTES {
            bail!("webhook_store_too_large");
        }
        let state: State =
            serde_json::from_slice(&bytes).map_err(|_| anyhow!("webhook_store_invalid"))?;
        if state.credentials.len() > MAX_CREDENTIALS
            || state.deliveries.len() > MAX_DELIVERIES
            || state.receipts.len() > MAX_RECEIPTS
            || state.deliveries.iter().any(|row| !row.is_object())
        {
            bail!("webhook_store_invalid");
        }
        Ok(state)
    }

    fn update<T>(
        &self,
        operation: impl FnOnce(&mut State) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        static PROCESS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _process = PROCESS_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| anyhow!("webhook_store_lock_failed"))?;
        if let Some(parent) = self.path.parent() {
            crate::settings::create_private_dir_all(parent)
                .map_err(|_| anyhow!("webhook_store_lock_failed"))?;
        }
        let lock_path = self.path.with_extension("json.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|_| anyhow!("webhook_store_lock_failed"))?;
        file.lock_exclusive()
            .map_err(|_| anyhow!("webhook_store_lock_failed"))?;
        let mut state = self.load()?;
        let result = operation(&mut state)?;
        let bytes = serde_json::to_vec(&state).map_err(|_| anyhow!("webhook_store_invalid"))?;
        if bytes.len() > MAX_STORE_BYTES {
            bail!("webhook_store_too_large");
        }
        crate::settings::atomic_write(&self.path, &bytes)
            .map_err(|_| anyhow!("webhook_store_write_failed"))?;
        Ok(result)
    }
}

fn receipt_for(delivery: Value, duplicate: bool, active: bool) -> WebhookReceipt {
    let should_dispatch =
        active && matches!(delivery["status"].as_str(), Some("queued" | "failed"));
    WebhookReceipt {
        http_status: delivery["response_status"].as_u64().unwrap_or(202) as u16,
        delivery,
        duplicate,
        should_dispatch,
    }
}

fn insert_delivery(state: &mut State, delivery: Value) -> anyhow::Result<()> {
    if state.deliveries.len() >= MAX_DELIVERIES {
        let index = state
            .deliveries
            .iter()
            .position(|row| row["status"] != "queued")
            .ok_or_else(|| anyhow!("webhook_delivery_queue_full"))?;
        state.deliveries.remove(index);
    }
    state.deliveries.push(delivery);
    Ok(())
}

fn ensure_receipt_capacity(state: &State) -> anyhow::Result<()> {
    // Fail closed instead of silently forgetting a replay key and executing again.
    if state.receipts.len() >= MAX_RECEIPTS {
        bail!("webhook_receipts_full");
    }
    Ok(())
}

pub fn validate_filters(filters: &Value) -> anyhow::Result<()> {
    if filters.is_null() {
        return Ok(());
    }
    let filters = filters
        .as_array()
        .filter(|items| items.len() <= 32)
        .ok_or_else(|| anyhow!("webhook_filters_invalid"))?;
    for filter in filters {
        let object = filter
            .as_object()
            .ok_or_else(|| anyhow!("webhook_filters_invalid"))?;
        if object
            .keys()
            .any(|key| !matches!(key.as_str(), "event" | "actions"))
        {
            bail!("webhook_filters_invalid");
        }
        validate_label(filter["event"].as_str().unwrap_or_default())?;
        if let Some(actions) = filter.get("actions") {
            for action in actions
                .as_array()
                .filter(|items| items.len() <= 32)
                .ok_or_else(|| anyhow!("webhook_filters_invalid"))?
            {
                validate_label(action.as_str().unwrap_or_default())?;
            }
        }
    }
    Ok(())
}

fn matches_event(filters: &Value, event: &str, action: Option<&str>) -> bool {
    let Some(filters) = filters.as_array().filter(|items| !items.is_empty()) else {
        return true;
    };
    filters.iter().any(|filter| {
        filter["event"] == event
            && filter["actions"].as_array().is_none_or(|actions| {
                actions.is_empty() || actions.iter().any(|value| value.as_str() == action)
            })
    })
}

fn normalize_headers(
    headers: &BTreeMap<String, String>,
) -> anyhow::Result<BTreeMap<String, String>> {
    if headers.len() > 32
        || headers
            .iter()
            .map(|(key, value)| key.len() + value.len())
            .sum::<usize>()
            > MAX_WEBHOOK_HEADER_BYTES
    {
        bail!("webhook_headers_too_large");
    }
    let mut normalized = BTreeMap::new();
    for (name, value) in headers {
        if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || value.chars().any(char::is_control)
            || value.len() > 2048
            || normalized
                .insert(name.to_ascii_lowercase(), value.clone())
                .is_some()
        {
            bail!("webhook_headers_invalid");
        }
    }
    Ok(normalized)
}

fn audit_body(body: Option<Value>, token: &str) -> Option<String> {
    let mut body = body?;
    fn redact(value: &mut Value, token: &str, depth: usize) {
        if depth > 12 {
            *value = json!("[redacted]");
            return;
        }
        match value {
            Value::Object(object) => {
                for (key, value) in object.iter_mut() {
                    let normalized = key.to_ascii_lowercase();
                    if [
                        "token",
                        "secret",
                        "password",
                        "authorization",
                        "cookie",
                        "api_key",
                        "apikey",
                        "credential",
                        "private_key",
                        "prompt",
                        "instructions",
                    ]
                    .iter()
                    .any(|part| normalized.contains(part))
                    {
                        *value = json!("[redacted]");
                    } else {
                        redact(value, token, depth + 1);
                    }
                }
            }
            Value::Array(items) => {
                for value in items {
                    redact(value, token, depth + 1);
                }
            }
            Value::String(text) => *text = redact_text(text, token),
            _ => {}
        }
    }
    redact(&mut body, token, 0);
    let text = serde_json::to_string(&body)
        .ok()?
        .replace(token, "[redacted]");
    Some(if text.len() > MAX_AUDIT_BODY_BYTES {
        "{\"audit\":\"body_omitted_size_limit\"}".into()
    } else {
        text
    })
}

fn redact_text(text: &str, token: &str) -> String {
    let lower = text.to_ascii_lowercase();
    if lower.contains("bearer ")
        || lower.contains("-----begin ")
        || ((lower.starts_with("http://") || lower.starts_with("https://"))
            && (text.contains('?') || text.contains('@')))
    {
        "[redacted]".into()
    } else {
        text.replace(token, "[redacted]")
    }
}

fn validate_id(value: &str) -> anyhow::Result<()> {
    if value.is_empty()
        || value.len() > 240
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
    {
        bail!("webhook_id_invalid");
    }
    Ok(())
}

fn validate_label(value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        bail!("webhook_label_invalid");
    }
    Ok(())
}

fn timestamp(now_ms: u64) -> String {
    i64::try_from(now_ms)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn digest_parts(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    hex(&hash.finalize())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn constant_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |difference, (a, b)| difference | (a ^ b))
            == 0
}

fn hmac_sha256(key: &[u8], body: &[u8]) -> [u8; 32] {
    // RFC 2104, SHA-256 block size 64. Keep key pads out of ordinary debug output.
    let mut pad = Zeroizing::new([0u8; 64]);
    if key.len() > 64 {
        pad[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        pad[..key.len()].copy_from_slice(key);
    }
    for byte in pad.iter_mut() {
        *byte ^= 0x36;
    }
    let mut inner = Sha256::new();
    inner.update(*pad);
    inner.update(body);
    for byte in pad.iter_mut() {
        *byte ^= 0x36 ^ 0x5c;
    }
    let mut outer = Sha256::new();
    outer.update(*pad);
    outer.update(inner.finalize());
    outer.finalize().into()
}

fn verify_signature(token: &str, body: &[u8], signature: &str) -> bool {
    let Some(signature) = signature.strip_prefix("sha256=") else {
        return false;
    };
    constant_eq(
        hex(&hmac_sha256(token.as_bytes(), body)).as_bytes(),
        signature.as_bytes(),
    )
}

#[cfg(test)]
#[path = "multica_webhooks_tests.rs"]
mod tests;
