//! Strict byte-oriented receiver for the existing loopback helper.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

use super::{MAX_WEBHOOK_BODY_BYTES, MAX_WEBHOOK_HEADER_BYTES, MulticaWebhookStore, WebhookTarget};
use crate::multica_execution_store::MulticaExecutionStore;
use crate::multica_workspace::{LocalMulticaWorkspaceStore, MulticaWorkspaceResourceKey};

const PREFIX: &str = "/multica/webhooks/ingress/";

#[derive(Debug, thiserror::Error)]
#[error("{code}")]
pub(crate) struct IngressError {
    pub status: &'static str,
    pub code: &'static str,
}

fn invalid() -> IngressError {
    IngressError {
        status: "400 Bad Request",
        code: "webhook_http_invalid",
    }
}

pub(crate) fn too_large() -> IngressError {
    IngressError {
        status: "413 Payload Too Large",
        code: "webhook_request_too_large",
    }
}

pub(crate) fn incomplete() -> IngressError {
    IngressError {
        status: "400 Bad Request",
        code: "webhook_http_incomplete",
    }
}

pub(crate) fn timeout() -> IngressError {
    IngressError {
        status: "408 Request Timeout",
        code: "webhook_request_timeout",
    }
}

/// Broad classification keeps malformed webhook URLs out of generic request logs.
/// Acceptance still requires the exact origin-form path in `parse_head`.
pub(crate) fn is_candidate(bytes: &[u8]) -> bool {
    let line = bytes
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    line.windows(b"/multica/webhooks".len())
        .any(|part| part == b"/multica/webhooks")
}

pub(crate) struct IngressHead {
    autopilot_id: String,
    trigger_id: String,
    headers: BTreeMap<String, String>,
    pub content_length: usize,
}

pub(crate) fn parse_head(bytes: &[u8]) -> Result<IngressHead, IngressError> {
    if bytes.len() > MAX_WEBHOOK_HEADER_BYTES {
        return Err(too_large());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let mut lines = text.split("\r\n");
    let mut request = lines.next().ok_or_else(invalid)?.split(' ');
    let method = request.next().ok_or_else(invalid)?;
    let path = request.next().ok_or_else(invalid)?;
    let version = request.next().ok_or_else(invalid)?;
    if request.next().is_some() || !matches!(version, "HTTP/1.1" | "HTTP/1.0") {
        return Err(invalid());
    }
    let suffix = path.strip_prefix(PREFIX).ok_or_else(invalid)?;
    let (autopilot_id, trigger_id) = suffix.split_once('/').ok_or_else(invalid)?;
    // No query, fragments, percent-decoding, extra segments, or URL credentials.
    for id in [autopilot_id, trigger_id] {
        if id.is_empty()
            || id.len() > 240
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
        {
            return Err(invalid());
        }
    }
    if method != "POST" {
        return Err(IngressError {
            status: "405 Method Not Allowed",
            code: "webhook_method_invalid",
        });
    }
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or_else(invalid)?;
        if name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || value.bytes().any(|b| b.is_ascii_control() && b != b'\t')
        {
            return Err(invalid());
        }
        let name = name.to_ascii_lowercase();
        let value = value.trim_matches([' ', '\t']);
        if headers.len() >= 32
            || value.len() > 2048
            || headers.insert(name, value.to_owned()).is_some()
        {
            return Err(invalid());
        }
    }
    if headers.contains_key("transfer-encoding")
        || headers.contains_key("content-encoding")
        || headers.contains_key("expect")
    {
        return Err(invalid());
    }
    let length = headers.get("content-length").ok_or(IngressError {
        status: "411 Length Required",
        code: "webhook_content_length_required",
    })?;
    if length.is_empty() || !length.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    let content_length = length.parse::<usize>().map_err(|_| too_large())?;
    if content_length > MAX_WEBHOOK_BODY_BYTES {
        return Err(too_large());
    }
    Ok(IngressHead {
        autopilot_id: autopilot_id.into(),
        trigger_id: trigger_id.into(),
        headers,
        content_length,
    })
}

#[derive(Clone, Default)]
pub(crate) struct WebhookIngress {
    pub workspace: LocalMulticaWorkspaceStore,
    pub executions: MulticaExecutionStore,
    pub webhooks: MulticaWebhookStore,
}

impl WebhookIngress {
    pub(crate) fn receive(&self, bytes: &[u8]) -> Result<Value, IngressError> {
        let end = bytes
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .ok_or_else(incomplete)?;
        let head = parse_head(&bytes[..end])?;
        let body = &bytes[end + 4..];
        if body.len() != head.content_length {
            return Err(incomplete());
        }
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let autopilot = self
            .workspace
            .list(&workspace_id, MulticaWorkspaceResourceKey::Autopilots)
            .map_err(map_error)?
            .into_iter()
            .find(|row| row["id"] == head.autopilot_id)
            .ok_or(IngressError {
                status: "404 Not Found",
                code: "webhook_target_not_found",
            })?;
        let target = WebhookTarget::from_autopilot(&workspace_id, &autopilot, &head.trigger_id)
            .map_err(map_error)?;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| IngressError {
                status: "503 Service Unavailable",
                code: "webhook_clock_unavailable",
            })?
            .as_millis() as u64;
        let receipt = self
            .webhooks
            .receive_and_enqueue(&self.executions, &target, &head.headers, body, now_ms)
            .map_err(map_error)?;
        if receipt.http_status != 202 {
            return Err(match receipt.http_status {
                401 => IngressError {
                    status: "401 Unauthorized",
                    code: "webhook_signature_invalid",
                },
                409 => IngressError {
                    status: "409 Conflict",
                    code: "webhook_trigger_inactive",
                },
                _ => IngressError {
                    status: "503 Service Unavailable",
                    code: "webhook_ingress_failed",
                },
            });
        }
        Ok(
            json!({"delivery_id":receipt.delivery["id"], "status":receipt.delivery["status"], "duplicate":receipt.duplicate}),
        )
    }
}

fn map_error(error: anyhow::Error) -> IngressError {
    // Never forward error chains, raw requests, credentials or filesystem paths.
    match error.to_string().as_str() {
        "webhook_credential_invalid" => IngressError {
            status: "401 Unauthorized",
            code: "webhook_credential_invalid",
        },
        "webhook_signature_invalid" => IngressError {
            status: "401 Unauthorized",
            code: "webhook_signature_invalid",
        },
        "webhook_target_not_found" => IngressError {
            status: "404 Not Found",
            code: "webhook_target_not_found",
        },
        "webhook_idempotency_conflict" => IngressError {
            status: "409 Conflict",
            code: "webhook_idempotency_conflict",
        },
        "webhook_delivery_expired" => IngressError {
            status: "409 Conflict",
            code: "webhook_delivery_expired",
        },
        "webhook_trigger_inactive" => IngressError {
            status: "409 Conflict",
            code: "webhook_trigger_inactive",
        },
        "webhook_body_too_large" | "webhook_headers_too_large" => too_large(),
        "webhook_headers_invalid" | "webhook_label_invalid" | "webhook_id_invalid" => invalid(),
        _ => IngressError {
            status: "503 Service Unavailable",
            code: "webhook_ingress_unavailable",
        },
    }
}

pub(crate) async fn write_response(
    stream: &mut tokio::net::TcpStream,
    result: Result<Value, IngressError>,
) -> anyhow::Result<()> {
    let (status, value) = match result {
        Ok(value) => ("202 Accepted", value),
        Err(error) => (error.status, json!({"status":"failed", "code":error.code})),
    };
    let body = serde_json::to_vec(&value)?;
    // This is an external delivery endpoint, not a browser API. No permissive CORS.
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(&body).await?;
    stream.shutdown().await?;
    Ok(())
}
