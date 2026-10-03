//! Distillation model client, ported from AITracker
//! `src/modules/ai-orchestration/model-profile.server.ts` (`requestBody`,
//! `chatUrl`/`modelRequestUrl`, `chatHeaders`, `parseChatCompletion`,
//! `createProfileBackedProvider` retry policy). Copyright (C) 2026 AITracker
//! contributors, used with permission.
//!
//! The API key is only placed in request headers; it never appears in errors.

use regex::Regex;
use serde_json::{Value, json};
use std::sync::OnceLock;
use std::time::Duration;

pub const MAX_OUTPUT_TOKENS: u32 = 8192;
const MAX_TRANSIENT_RETRIES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelProtocol {
    /// OpenAI Chat Completions.
    OpenAi,
    /// OpenAI Responses API.
    Responses,
    /// Anthropic Messages API.
    Anthropic,
}

impl ModelProtocol {
    pub fn vendor(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::Responses => "Responses",
            Self::Anthropic => "Anthropic",
        }
    }
}

#[derive(Clone)]
pub struct ModelTarget {
    pub protocol: ModelProtocol,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// The supplier's custom User-Agent; empty means the CCP default.
    pub user_agent: String,
}

/// Some relay gateways drop the TLS connection (no close_notify) for requests
/// without a User-Agent, which surfaced as `ai.provider-network` while the
/// same supplier worked everywhere else in CCP. reqwest sends none by default.
fn distill_user_agent(custom: &str) -> String {
    let custom = custom.trim();
    if custom.is_empty() {
        format!("ClaudeCodexPro/{}", env!("CARGO_PKG_VERSION"))
    } else {
        custom.to_string()
    }
}

impl std::fmt::Debug for ModelTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelTarget")
            .field("protocol", &self.protocol)
            .field("base_url", &self.base_url)
            .field("api_key", &"[REDACTED]")
            .field("model", &self.model)
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ModelCallOptions {
    pub timeout: Duration,
    pub retry_delay: Duration,
}

impl Default for ModelCallOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            retry_delay: Duration::from_millis(350),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCallError {
    /// Stable AITracker error code, e.g. `ai.provider-auth`.
    pub code: &'static str,
    /// Sanitized attribution such as `http-error:401` or `reasoning-only`.
    pub detail: Option<String>,
}

impl ModelCallError {
    fn new(code: &'static str, detail: Option<String>) -> Self {
        Self { code, detail }
    }

    pub fn cancelled() -> Self {
        Self::new("ai.cancelled", None)
    }

    /// User-facing Chinese reason that always includes the stable code.
    pub fn user_message(&self) -> String {
        let reason = match self.code {
            "ai.provider-auth" => "模型服务鉴权失败，请检查供应商 API Key 与权限",
            "ai.provider-rate-limited" => "模型服务限流，请稍后重试",
            "ai.provider-unavailable" => "模型服务暂不可用，请稍后重试",
            "ai.provider-http-client" => "模型服务拒绝了请求，请检查 Base URL、模型 ID 与协议",
            "ai.provider-invalid-response" => match self.detail.as_deref() {
                Some("reasoning-only") => {
                    "模型只返回了推理内容、没有正文（输出预算可能被推理耗尽），请更换模型后重试"
                }
                Some("empty-content") => "模型返回了空内容",
                Some("not-json") => "模型服务返回的不是有效 JSON",
                _ => "模型响应无法解析",
            },
            "ai.provider-network" => "无法连接模型服务，请检查网络与 Base URL",
            "ai.provider-timeout" => {
                "模型服务响应超时（输入较大时等待时间会相应延长，仍超时请减少素材或更换模型）"
            }
            "ai.profile-unavailable" => "所选供应商缺少 Base URL 或 API Key",
            "ai.cancelled" => "任务已取消",
            _ => "模型调用失败",
        };
        match &self.detail {
            Some(detail) if detail.starts_with("http-error:") => {
                format!("{reason}（{}，{detail}）", self.code)
            }
            _ => format!("{reason}（{}）", self.code),
        }
    }
}

impl std::fmt::Display for ModelCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.user_message())
    }
}

impl std::error::Error for ModelCallError {}

fn http_failure_code(status: u16) -> &'static str {
    match status {
        401 | 403 => "ai.provider-auth",
        429 => "ai.provider-rate-limited",
        500.. => "ai.provider-unavailable",
        _ => "ai.provider-http-client",
    }
}

fn is_transient_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

fn version_suffix() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/v\d+$").expect("version regex"))
}

/// Port of `chatUrl` / `modelRequestUrl`.
pub fn request_url(protocol: ModelProtocol, endpoint: &str) -> String {
    let base = endpoint.trim().trim_end_matches('/');
    if base.ends_with("/chat/completions")
        || base.ends_with("/messages")
        || base.ends_with("/responses")
    {
        return base.to_string();
    }
    match protocol {
        ModelProtocol::Responses => format!("{base}/responses"),
        ModelProtocol::Anthropic if version_suffix().is_match(base) => format!("{base}/messages"),
        ModelProtocol::Anthropic => format!("{base}/v1/messages"),
        ModelProtocol::OpenAi => format!("{base}/chat/completions"),
    }
}

/// Port of `requestBody` (no temperature, no streaming).
pub fn request_body(protocol: ModelProtocol, model: &str, prompt: &str, input: &str) -> Value {
    match protocol {
        ModelProtocol::Responses => {
            let mut body = json!({ "model": model, "max_output_tokens": MAX_OUTPUT_TOKENS });
            if !prompt.is_empty() {
                body["instructions"] = json!(prompt);
            }
            body["input"] = json!(input);
            body
        }
        ModelProtocol::Anthropic => {
            let mut body = json!({ "model": model, "max_tokens": MAX_OUTPUT_TOKENS });
            if !prompt.is_empty() {
                body["system"] = json!(prompt);
            }
            body["messages"] = json!([{ "role": "user", "content": input }]);
            body
        }
        ModelProtocol::OpenAi => {
            let mut messages = Vec::new();
            if !prompt.is_empty() {
                messages.push(json!({ "role": "system", "content": prompt }));
            }
            messages.push(json!({ "role": "user", "content": input }));
            json!({ "model": model, "max_tokens": MAX_OUTPUT_TOKENS, "messages": messages })
        }
    }
}

/// Port of `chatHeaders` with each protocol's default auth scheme.
pub fn request_headers(protocol: ModelProtocol, api_key: &str) -> Vec<(&'static str, String)> {
    match protocol {
        ModelProtocol::Anthropic => vec![
            ("content-type", "application/json".into()),
            ("x-api-key", api_key.to_string()),
            ("anthropic-version", "2023-06-01".into()),
        ],
        _ => vec![
            ("content-type", "application/json".into()),
            ("authorization", format!("Bearer {api_key}")),
        ],
    }
}

/// Response classification, as in `ChatResponseClassification`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseKind {
    Ok,
    NotJson,
    EmptyContent,
    ReasoningOnly,
}

impl ResponseKind {
    fn detail(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NotJson => "not-json",
            Self::EmptyContent => "empty-content",
            Self::ReasoningOnly => "reasoning-only",
        }
    }
}

fn text_from_content(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Array(parts)) => parts
            .iter()
            .map(|part| match part {
                Value::String(text) => text.as_str(),
                Value::Object(record) => record
                    .get("text")
                    .and_then(Value::as_str)
                    .or_else(|| record.get("content").and_then(Value::as_str))
                    .unwrap_or(""),
                _ => "",
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string(),
        _ => String::new(),
    }
}

fn classify(text: &str, reasoning: &str) -> ResponseKind {
    if !text.is_empty() {
        ResponseKind::Ok
    } else if !reasoning.is_empty() {
        ResponseKind::ReasoningOnly
    } else {
        ResponseKind::EmptyContent
    }
}

/// Port of `parseChatCompletion`.
pub fn parse_completion(protocol: ModelProtocol, raw: &str) -> (String, ResponseKind) {
    let Ok(json) = serde_json::from_str::<Value>(raw) else {
        return (String::new(), ResponseKind::NotJson);
    };
    if protocol == ModelProtocol::Anthropic {
        let blocks = json.get("content").and_then(Value::as_array);
        let text = text_from_content(json.get("content").filter(|value| value.is_array()));
        let thinking = blocks
            .into_iter()
            .flatten()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("thinking"))
            .filter_map(|block| block.get("thinking").and_then(Value::as_str))
            .map(|value| Value::String(value.to_string()))
            .collect::<Vec<_>>();
        let reasoning = text_from_content(Some(&Value::Array(thinking)));
        let kind = classify(&text, &reasoning);
        return (text, kind);
    }
    if let Some(output) = json.get("output").and_then(Value::as_array) {
        let message_parts = output
            .iter()
            .filter(|item| match item.get("type") {
                None => true,
                Some(kind) => kind.as_str() == Some("message"),
            })
            .flat_map(|item| {
                item.get("content")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
            })
            .cloned()
            .collect::<Vec<_>>();
        let reasoning_parts = output
            .iter()
            .filter(|item| item.get("type").and_then(Value::as_str) == Some("reasoning"))
            .flat_map(|item| {
                let content = item
                    .get("content")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten();
                let summary = item
                    .get("summary")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten();
                content.chain(summary)
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut text = text_from_content(Some(&Value::Array(message_parts)));
        if text.is_empty() {
            text = text_from_content(json.get("output_text"));
        }
        let reasoning = text_from_content(Some(&Value::Array(reasoning_parts)));
        let kind = classify(&text, &reasoning);
        return (text, kind);
    }
    let choice = json.pointer("/choices/0");
    let mut text = text_from_content(choice.and_then(|choice| choice.pointer("/message/content")));
    if text.is_empty() {
        text = text_from_content(choice.and_then(|choice| choice.get("text")));
    }
    if text.is_empty() {
        text = text_from_content(json.get("output_text"));
    }
    let reasoning =
        text_from_content(choice.and_then(|choice| choice.pointer("/message/reasoning_content")));
    let kind = classify(&text, &reasoning);
    (text, kind)
}

/// One real model call with AITracker's transient-retry policy: up to 2
/// retries (3 attempts), 350 ms apart, on network errors or
/// 408/425/429/500/502/503/504. Timeouts are terminal.
pub async fn call_model(
    target: &ModelTarget,
    system: &str,
    input: &str,
    options: ModelCallOptions,
    is_cancelled: &(dyn Fn() -> bool + Send + Sync),
) -> Result<String, ModelCallError> {
    if target.base_url.trim().is_empty() || target.api_key.trim().is_empty() {
        return Err(ModelCallError::new("ai.profile-unavailable", None));
    }
    let client = reqwest::Client::builder()
        .user_agent(distill_user_agent(&target.user_agent))
        .timeout(options.timeout)
        .build()
        .map_err(|_| ModelCallError::new("ai.provider-network", None))?;
    let url = request_url(target.protocol, &target.base_url);
    let body = request_body(target.protocol, &target.model, system.trim(), input.trim());
    let body = serde_json::to_vec(&body)
        .map_err(|_| ModelCallError::new("ai.provider-invalid-response", None))?;
    let mut last: Option<(u16, String)> = None;
    for attempt in 0..=MAX_TRANSIENT_RETRIES {
        if is_cancelled() {
            return Err(ModelCallError::new("ai.cancelled", None));
        }
        let mut request = client.post(&url).body(body.clone());
        for (name, value) in request_headers(target.protocol, &target.api_key) {
            request = request.header(name, value);
        }
        match request.send().await {
            Err(error) => {
                if error.is_timeout() {
                    return Err(ModelCallError::new("ai.provider-timeout", None));
                }
                if attempt == MAX_TRANSIENT_RETRIES {
                    return Err(ModelCallError::new("ai.provider-network", None));
                }
            }
            Ok(response) => {
                let status = response.status().as_u16();
                let success = response.status().is_success();
                if success || !is_transient_status(status) || attempt == MAX_TRANSIENT_RETRIES {
                    let raw = match response.text().await {
                        Ok(raw) => raw,
                        Err(error) if error.is_timeout() => {
                            return Err(ModelCallError::new("ai.provider-timeout", None));
                        }
                        Err(_) => String::new(),
                    };
                    last = Some((status, raw));
                    break;
                }
            }
        }
        tokio::time::sleep(options.retry_delay).await;
    }
    let Some((status, raw)) = last else {
        return Err(ModelCallError::new("ai.provider-invalid-response", None));
    };
    if !(200..300).contains(&status) {
        return Err(ModelCallError::new(
            http_failure_code(status),
            Some(format!("http-error:{status}")),
        ));
    }
    let (text, kind) = parse_completion(target.protocol, &raw);
    if kind != ResponseKind::Ok {
        return Err(ModelCallError::new(
            "ai.provider-invalid-response",
            Some(kind.detail().into()),
        ));
    }
    Ok(text)
}

#[cfg(test)]
pub mod test_support {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    #[derive(Debug, Clone)]
    pub struct CapturedRequest {
        pub request_line: String,
        pub headers: String,
        pub body: String,
    }

    /// Raw TCP mock: serves each `(status, body)` to one connection in order
    /// and reports the captured requests.
    pub fn spawn_mock(
        responses: Vec<(u16, String)>,
    ) -> (
        String,
        mpsc::Receiver<CapturedRequest>,
        std::thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let (sender, receiver) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 8192];
                loop {
                    let read = stream.read(&mut buffer).unwrap_or(0);
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                    let Some(split) = request.windows(4).position(|w| w == b"\r\n\r\n") else {
                        continue;
                    };
                    let headers = String::from_utf8_lossy(&request[..split]).to_string();
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= split + 4 + length {
                        break;
                    }
                }
                let split = request
                    .windows(4)
                    .position(|w| w == b"\r\n\r\n")
                    .unwrap_or(request.len());
                let head = String::from_utf8_lossy(&request[..split]).to_string();
                let body_text =
                    String::from_utf8_lossy(&request[(split + 4).min(request.len())..]).to_string();
                let (request_line, headers) = head.split_once("\r\n").unwrap_or((&head, ""));
                let _ = sender.send(CapturedRequest {
                    request_line: request_line.to_string(),
                    headers: headers.to_ascii_lowercase(),
                    body: body_text,
                });
                let reason = if status == 200 { "OK" } else { "ERR" };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        (base, receiver, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::spawn_mock;
    use super::*;

    #[test]
    fn custom_supplier_user_agent_is_sent() {
        let (base, captured, server) = spawn_mock(vec![(
            200,
            r#"{"choices":[{"message":{"content":"ok"}}]}"#.into(),
        )]);
        let custom = ModelTarget {
            user_agent: "claude-cli/2.1.161 (external, cli)".into(),
            ..target(ModelProtocol::OpenAi, format!("{base}/v1"))
        };
        assert_eq!(run(&custom).unwrap(), "ok");
        server.join().unwrap();
        let request = captured.recv().unwrap();
        assert!(
            request
                .headers
                .contains("user-agent: claude-cli/2.1.161 (external, cli)")
        );
    }

    fn target(protocol: ModelProtocol, base_url: String) -> ModelTarget {
        ModelTarget {
            protocol,
            base_url,
            api_key: "test-key".into(),
            model: "fixture-model".into(),
            user_agent: String::new(),
        }
    }

    fn fast() -> ModelCallOptions {
        ModelCallOptions {
            timeout: Duration::from_secs(10),
            retry_delay: Duration::from_millis(10),
        }
    }

    fn never() -> impl Fn() -> bool + Send + Sync {
        || false
    }

    #[test]
    fn distill_model_url_rules_match_aitracker() {
        assert_eq!(
            request_url(ModelProtocol::OpenAi, "https://x/v1/"),
            "https://x/v1/chat/completions"
        );
        assert_eq!(
            request_url(ModelProtocol::Responses, "https://x/v1"),
            "https://x/v1/responses"
        );
        assert_eq!(
            request_url(ModelProtocol::Anthropic, "https://x/v1"),
            "https://x/v1/messages"
        );
        assert_eq!(
            request_url(ModelProtocol::Anthropic, "https://x/api"),
            "https://x/api/v1/messages"
        );
        assert_eq!(
            request_url(ModelProtocol::Anthropic, "https://x/chat/completions"),
            "https://x/chat/completions"
        );
    }

    #[test]
    fn distill_model_parser_handles_all_protocol_shapes() {
        let (text, kind) = parse_completion(
            ModelProtocol::Anthropic,
            r#"{"content":[{"type":"thinking","thinking":"hmm"},{"type":"text","text":"answer"}]}"#,
        );
        assert_eq!((text.as_str(), kind), ("answer", ResponseKind::Ok));
        let (_, kind) = parse_completion(
            ModelProtocol::Anthropic,
            r#"{"content":[{"type":"thinking","thinking":"hmm"}]}"#,
        );
        assert_eq!(kind, ResponseKind::ReasoningOnly);
        let (text, _) = parse_completion(
            ModelProtocol::OpenAi,
            r#"{"output":[{"type":"reasoning","summary":[{"text":"r"}]},{"type":"message","content":[{"type":"output_text","text":"out"}]}]}"#,
        );
        assert_eq!(text, "out");
        let (text, _) = parse_completion(
            ModelProtocol::OpenAi,
            r#"{"choices":[{"message":{"content":[{"type":"text","text":"a"},{"type":"text","text":"b"}]}}]}"#,
        );
        assert_eq!(text, "a\nb");
        let (_, kind) = parse_completion(
            ModelProtocol::OpenAi,
            r#"{"choices":[{"message":{"content":"","reasoning_content":"thinking"}}]}"#,
        );
        assert_eq!(kind, ResponseKind::ReasoningOnly);
        assert_eq!(
            parse_completion(ModelProtocol::OpenAi, "<html>").1,
            ResponseKind::NotJson
        );
        assert_eq!(
            parse_completion(ModelProtocol::OpenAi, "{}").1,
            ResponseKind::EmptyContent
        );
    }

    fn run(target: &ModelTarget) -> Result<String, ModelCallError> {
        let cancel = never();
        tauri::async_runtime::block_on(call_model(target, " system ", " input ", fast(), &cancel))
    }

    #[test]
    fn distill_model_openai_chat_request_shape() {
        let (base, captured, server) = spawn_mock(vec![(
            200,
            r#"{"choices":[{"message":{"content":"chat out"}}]}"#.into(),
        )]);
        let output = run(&target(ModelProtocol::OpenAi, format!("{base}/v1/"))).unwrap();
        server.join().unwrap();
        assert_eq!(output, "chat out");
        let request = captured.recv().unwrap();
        assert_eq!(request.request_line, "POST /v1/chat/completions HTTP/1.1");
        assert!(request.headers.contains("authorization: bearer test-key"));
        // Relay gateways may drop requests without a User-Agent.
        assert!(
            request.headers.contains("user-agent: claudecodexpro/"),
            "distill requests must carry a User-Agent: {}",
            request.headers
        );
        assert!(request.headers.contains("content-type: application/json"));
        let body: Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["max_tokens"], 8192);
        assert!(body.get("temperature").is_none());
        assert!(body.get("stream").is_none());
        assert_eq!(
            body["messages"][0],
            json!({"role":"system","content":"system"})
        );
        assert_eq!(
            body["messages"][1],
            json!({"role":"user","content":"input"})
        );
    }

    #[test]
    fn distill_model_responses_request_shape() {
        let (base, captured, server) = spawn_mock(vec![(
            200,
            r#"{"output":[{"type":"message","content":[{"type":"output_text","text":"resp out"}]}]}"#
                .into(),
        )]);
        let output = run(&target(ModelProtocol::Responses, base)).unwrap();
        server.join().unwrap();
        assert_eq!(output, "resp out");
        let request = captured.recv().unwrap();
        assert_eq!(request.request_line, "POST /responses HTTP/1.1");
        assert!(request.headers.contains("authorization: bearer test-key"));
        let body: Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["max_output_tokens"], 8192);
        assert_eq!(body["instructions"], "system");
        assert_eq!(body["input"], "input");
        assert!(body.get("temperature").is_none());
    }

    #[test]
    fn distill_model_anthropic_request_shape() {
        let (base, captured, server) = spawn_mock(vec![(
            200,
            r#"{"content":[{"type":"text","text":"claude out"}],"stop_reason":"end_turn"}"#.into(),
        )]);
        let output = run(&target(ModelProtocol::Anthropic, base)).unwrap();
        server.join().unwrap();
        assert_eq!(output, "claude out");
        let request = captured.recv().unwrap();
        assert_eq!(request.request_line, "POST /v1/messages HTTP/1.1");
        assert!(request.headers.contains("x-api-key: test-key"));
        assert!(request.headers.contains("anthropic-version: 2023-06-01"));
        assert!(!request.headers.contains("authorization:"));
        let body: Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["max_tokens"], 8192);
        assert_eq!(body["system"], "system");
        assert_eq!(body["messages"], json!([{"role":"user","content":"input"}]));
        assert!(body.get("temperature").is_none());
    }

    #[test]
    fn distill_model_retries_after_429_then_succeeds() {
        let (base, captured, server) = spawn_mock(vec![
            (429, r#"{"error":"slow down"}"#.into()),
            (
                200,
                r#"{"choices":[{"message":{"content":"after retry"}}]}"#.into(),
            ),
        ]);
        let output = run(&target(ModelProtocol::OpenAi, base)).unwrap();
        server.join().unwrap();
        assert_eq!(output, "after retry");
        assert_eq!(captured.try_iter().count(), 2);
    }

    #[test]
    fn distill_model_maps_http_and_reasoning_errors() {
        let (base, _, server) = spawn_mock(vec![(401, "{}".into())]);
        let error = run(&target(ModelProtocol::OpenAi, base)).unwrap_err();
        server.join().unwrap();
        assert_eq!(error.code, "ai.provider-auth");
        assert!(error.user_message().contains("ai.provider-auth"));
        assert!(!error.user_message().contains("test-key"));

        let (base, _, server) = spawn_mock(vec![(
            200,
            r#"{"choices":[{"message":{"content":"","reasoning_content":"only thoughts"}}]}"#
                .into(),
        )]);
        let error = run(&target(ModelProtocol::OpenAi, base)).unwrap_err();
        server.join().unwrap();
        assert_eq!(error.code, "ai.provider-invalid-response");
        assert_eq!(error.detail.as_deref(), Some("reasoning-only"));

        let (base, captured, server) = spawn_mock(vec![(404, "{}".into())]);
        let error = run(&target(ModelProtocol::OpenAi, base)).unwrap_err();
        server.join().unwrap();
        assert_eq!(error.code, "ai.provider-http-client");
        assert_eq!(captured.try_iter().count(), 1, "404 is not retried");

        let error = run(&ModelTarget {
            api_key: String::new(),
            ..target(ModelProtocol::OpenAi, "http://127.0.0.1:1".into())
        })
        .unwrap_err();
        assert_eq!(error.code, "ai.profile-unavailable");
    }
}
