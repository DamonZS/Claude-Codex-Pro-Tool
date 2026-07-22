//! Codex Responses API 与 OpenAI Chat Completions 的本地协议转换。
//!
//! Codex Chat 与 Responses 协议之间的转换实现。

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::settings::{BackendSettings, RelayProfile, RelayProtocol, SettingsStore};

pub const DEFAULT_PROTOCOL_PROXY_PORT: u16 = 57321;
pub const DEFAULT_CLAUDE_DESKTOP_PROXY_PORT: u16 = 57331;
pub const DEFAULT_CLAUDE_DESKTOP_UPSTREAM_BASE_URL: &str = "https://api.toporeduce.cn";
const CLAUDE_DESKTOP_SAFE_FABLE_MODEL: &str = "claude-fable-5";
const CLAUDE_DESKTOP_SAFE_SONNET_MODEL: &str = "claude-sonnet-4-6";
const CLAUDE_DESKTOP_SAFE_OPUS_MODEL: &str = "claude-opus-4-8";
const CLAUDE_DESKTOP_SAFE_HAIKU_MODEL: &str = "claude-haiku-4-5";
const CLAUDE_DESKTOP_DEFAULT_SONNET_MODEL: &str = "claude-opus-4-6";
const CLAUDE_DESKTOP_DEFAULT_OPUS_MODEL: &str = "claude-opus-4-8";
const CLAUDE_DESKTOP_DEFAULT_FABLE_MODEL: &str = "claude-Fable-5";
const CLAUDE_DESKTOP_DEFAULT_HAIKU_MODEL: &str = "claude-haiku-4-5";
const THINK_OPEN_TAG: &str = "<think>";
const THINK_CLOSE_TAG: &str = "</think>";
const EXTRA_CHAT_PASSTHROUGH_FIELDS: &[&str] = &[
    "frequency_penalty",
    "logit_bias",
    "logprobs",
    "metadata",
    "n",
    "presence_penalty",
    "response_format",
    "seed",
    "service_tier",
    "stop",
    "stream_options",
    "top_logprobs",
    "user",
];
const ERROR_BODY_PREVIEW_LIMIT: usize = 1024;
const MAX_ANTHROPIC_VERSION_HEADER_LEN: usize = 64;
const MAX_ANTHROPIC_BETA_HEADER_LEN: usize = 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaudeMessagesRequestMetadata {
    beta: bool,
    anthropic_version: Option<String>,
    anthropic_beta: Option<String>,
}

impl ClaudeMessagesRequestMetadata {
    pub fn beta(&self) -> bool {
        self.beta
    }

    pub fn has_anthropic_version(&self) -> bool {
        self.anthropic_version.is_some()
    }

    pub fn has_anthropic_beta(&self) -> bool {
        self.anthropic_beta.is_some()
    }

    pub fn from_inbound(raw_path: &str, anthropic_version: &str, anthropic_beta: &str) -> Self {
        let beta = raw_path
            .split_once('?')
            .map(|(_, query)| query)
            .map(|query| {
                url::form_urlencoded::parse(query.as_bytes()).any(|(name, value)| {
                    name.eq_ignore_ascii_case("beta") && value.eq_ignore_ascii_case("true")
                })
            })
            .unwrap_or(false);

        Self {
            beta,
            anthropic_version: sanitized_protocol_header(
                anthropic_version,
                MAX_ANTHROPIC_VERSION_HEADER_LEN,
            ),
            anthropic_beta: sanitized_protocol_header(
                anthropic_beta,
                MAX_ANTHROPIC_BETA_HEADER_LEN,
            ),
        }
    }
}

fn sanitized_protocol_header(value: &str, max_len: usize) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > max_len
        || reqwest::header::HeaderValue::from_str(value).is_err()
    {
        return None;
    }
    Some(value.to_string())
}

fn parsed_local_proxy_body_override(raw: &str) -> Option<Value> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let value = serde_json::from_str::<Value>(raw).ok()?;
    value.as_object()?;
    Some(value)
}

fn apply_local_proxy_body_override(target: &mut Value, override_body: &Value) -> Vec<String> {
    let Some(target_object) = target.as_object_mut() else {
        return Vec::new();
    };
    let Some(override_object) = override_body.as_object() else {
        return Vec::new();
    };

    let mut applied = Vec::new();
    for (key, value) in override_object {
        if key == "stream" {
            continue;
        }
        match target_object.get_mut(key) {
            Some(target_value) => {
                if merge_json_override_value(target_value, value) {
                    applied.push(key.clone());
                }
            }
            None => {
                target_object.insert(key.clone(), value.clone());
                applied.push(key.clone());
            }
        }
    }
    applied
}

fn merge_json_override_value(target: &mut Value, patch: &Value) -> bool {
    match (target, patch) {
        (Value::Object(target_object), Value::Object(patch_object)) => {
            let mut changed = false;
            for (key, value) in patch_object {
                match target_object.get_mut(key) {
                    Some(target_value) => {
                        changed |= merge_json_override_value(target_value, value);
                    }
                    None => {
                        target_object.insert(key.clone(), value.clone());
                        changed = true;
                    }
                }
            }
            changed
        }
        (target_value, patch_value) => {
            if target_value == patch_value {
                false
            } else {
                *target_value = patch_value.clone();
                true
            }
        }
    }
}

fn parsed_local_proxy_header_override(raw: &str) -> BTreeMap<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return BTreeMap::new();
    }
    let Ok(Value::Object(headers)) = serde_json::from_str::<Value>(raw) else {
        return BTreeMap::new();
    };

    headers
        .into_iter()
        .filter_map(|(name, value)| {
            let name = name.trim().to_ascii_lowercase();
            if name.is_empty() || is_protected_local_proxy_override_header(&name) {
                return None;
            }
            let value = match value {
                Value::String(value) => value,
                Value::Number(value) => value.to_string(),
                Value::Bool(value) => value.to_string(),
                _ => return None,
            };
            let value = value.trim().to_string();
            if value.is_empty()
                || reqwest::header::HeaderName::from_bytes(name.as_bytes()).is_err()
                || reqwest::header::HeaderValue::from_str(&value).is_err()
            {
                return None;
            }
            Some((name, value))
        })
        .collect()
}

fn is_protected_local_proxy_override_header(name: &str) -> bool {
    matches!(
        name,
        "host"
            | "content-length"
            | "transfer-encoding"
            | "connection"
            | "proxy-authorization"
            | "proxy-authenticate"
            | "te"
            | "trailer"
            | "upgrade"
            | "accept-encoding"
            | "content-type"
            | "authorization"
            | "x-api-key"
            | "x-goog-api-key"
            | "api-key"
            | "anthropic-api-key"
            | "chatgpt-account-id"
            | "session_id"
            | "x-client-request-id"
            | "x-codex-window-id"
            | "x-forwarded-host"
            | "x-forwarded-port"
            | "x-forwarded-proto"
            | "forwarded"
            | "cf-connecting-ip"
            | "cf-ipcountry"
            | "cf-ray"
            | "cf-visitor"
            | "true-client-ip"
            | "fastly-client-ip"
            | "x-azure-clientip"
            | "x-azure-fdid"
            | "x-azure-ref"
            | "akamai-origin-hop"
            | "x-akamai-config-log-detail"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatReasoningStyle {
    Default,
    DeepSeek,
    LowHigh,
    OpenRouter,
    Thinking,
    EnableThinking,
    ReasoningSplit,
}

#[derive(Debug, Clone, Default)]
struct CodexToolContext {
    custom_tools: BTreeMap<String, CodexCustomToolSpec>,
    function_tools: BTreeMap<String, CodexFunctionToolSpec>,
    has_custom_tools: bool,
    has_namespace_tools: bool,
}

#[derive(Debug, Clone)]
struct CodexCustomToolSpec {
    openai_name: String,
    kind: CodexCustomToolKind,
    proxy_action: Option<CodexPatchProxyAction>,
}

#[derive(Debug, Clone, Default)]
struct CodexFunctionToolSpec {
    namespace: String,
    name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexCustomToolKind {
    Raw,
    ApplyPatch,
    BuiltIn,
}

impl Default for CodexCustomToolKind {
    fn default() -> Self {
        Self::Raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexPatchProxyAction {
    AddFile,
    DeleteFile,
    UpdateFile,
    ReplaceFile,
    Batch,
}

impl CodexPatchProxyAction {
    fn suffix(self) -> &'static str {
        match self {
            Self::AddFile => "add_file",
            Self::DeleteFile => "delete_file",
            Self::UpdateFile => "update_file",
            Self::ReplaceFile => "replace_file",
            Self::Batch => "batch",
        }
    }
}

impl CodexToolContext {
    fn is_custom_tool_proxy(&self, upstream_name: &str) -> bool {
        self.custom_tools.contains_key(upstream_name)
    }

    fn original_custom_tool_name(&self, upstream_name: &str) -> String {
        self.custom_tools
            .get(upstream_name)
            .map(|spec| spec.openai_name.clone())
            .unwrap_or_else(|| upstream_name.to_string())
    }

    fn openai_name_for_function_tool(&self, upstream_name: &str) -> (String, String) {
        let Some(spec) = self.function_tools.get(upstream_name) else {
            return (upstream_name.to_string(), String::new());
        };
        let name = if spec.name.is_empty() {
            upstream_name.to_string()
        } else {
            spec.name.clone()
        };
        (name, spec.namespace.clone())
    }
}

pub fn local_responses_proxy_base_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/v1")
}

pub fn local_claude_desktop_proxy_base_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/claude-desktop")
}

pub fn responses_to_chat_completions(body: Value) -> anyhow::Result<Value> {
    let mut result = json!({});

    if let Some(model) = body.get("model") {
        result["model"] = model.clone();
    }

    let mut messages = Vec::new();
    if let Some(instructions) = body.get("instructions") {
        let text = instruction_text(instructions);
        if !text.is_empty() {
            messages.push(json!({ "role": "system", "content": text }));
        }
    }

    if let Some(input) = body.get("input") {
        append_responses_input(input, &mut messages);
    }
    normalize_chat_messages(&mut messages);
    let messages = collapse_system_messages_to_head(messages);
    result["messages"] = json!(messages);

    let model = body.get("model").and_then(Value::as_str).unwrap_or("");
    if let Some(value) = body.get("max_output_tokens") {
        if is_openai_o_series(model) {
            result["max_completion_tokens"] = value.clone();
        } else {
            result["max_tokens"] = value.clone();
        }
    }
    if let Some(value) = body.get("max_tokens") {
        result["max_tokens"] = value.clone();
    }
    if let Some(value) = body.get("max_completion_tokens") {
        result["max_completion_tokens"] = value.clone();
    }

    for key in ["temperature", "top_p", "stream"] {
        if let Some(value) = body.get(key) {
            result[key] = value.clone();
        }
    }
    if body.get("stream").and_then(Value::as_bool).unwrap_or(false) {
        let mut stream_options = body
            .get("stream_options")
            .cloned()
            .unwrap_or_else(|| json!({}));
        stream_options["include_usage"] = json!(true);
        result["stream_options"] = stream_options;
    }

    apply_chat_reasoning_options(&mut result, &body, model);

    let tool_context = build_codex_tool_context(body.get("tools"));
    let mut has_chat_tools = false;
    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        let converted = responses_tools_to_chat_tools(tools, &tool_context);
        if !converted.is_empty() {
            has_chat_tools = true;
            result["tools"] = json!(converted);
        }
    }

    if has_chat_tools {
        if let Some(tool_choice) = body
            .get("tool_choice")
            .and_then(|value| responses_tool_choice_to_chat(value, &tool_context))
        {
            result["tool_choice"] = tool_choice;
        }
        if let Some(value) = body.get("parallel_tool_calls") {
            result["parallel_tool_calls"] = value.clone();
        }
    }

    for key in EXTRA_CHAT_PASSTHROUGH_FIELDS {
        if *key == "stream_options" && result.get("stream_options").is_some() {
            continue;
        }
        if let Some(value) = body.get(*key) {
            result[*key] = value.clone();
        }
    }

    Ok(result)
}

pub fn chat_completion_to_response(body: Value) -> anyhow::Result<Value> {
    chat_completion_to_response_with_context(body, &CodexToolContext::default(), None)
}

pub fn chat_completion_to_response_with_request(
    body: Value,
    original_request: &Value,
) -> anyhow::Result<Value> {
    let context = build_codex_tool_context(original_request.get("tools"));
    chat_completion_to_response_with_context(body, &context, Some(original_request))
}

fn chat_completion_to_response_with_context(
    body: Value,
    tool_context: &CodexToolContext,
    original_request: Option<&Value>,
) -> anyhow::Result<Value> {
    let choices = body
        .get("choices")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("chat response missing choices"))?;
    let choice = choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("chat response choices is empty"))?;
    let message = choice
        .get("message")
        .ok_or_else(|| anyhow::anyhow!("chat response choice missing message"))?;

    let response_id = response_id_from_chat_id(body.get("id").and_then(Value::as_str));
    let mut output = Vec::new();
    if let Some(reasoning) = chat_reasoning_to_response_output_item(message, &response_id) {
        output.push(reasoning);
    }
    if let Some(message) = chat_message_to_response_output_item(message, &response_id) {
        output.push(message);
    }
    output.extend(chat_tool_calls_to_response_output_items(
        message,
        tool_context,
    ));

    let mut response = json!({
        "id": response_id,
        "object": "response",
        "created_at": body.get("created").and_then(Value::as_u64).unwrap_or(0),
        "status": response_status(choice.get("finish_reason").and_then(Value::as_str)),
        "model": body.get("model").and_then(Value::as_str).unwrap_or(""),
        "output": output,
        "usage": chat_usage_to_responses_usage(body.get("usage"))
    });

    if choice.get("finish_reason").and_then(Value::as_str) == Some("length") {
        response["incomplete_details"] = json!({ "reason": "max_output_tokens" });
    }
    copy_response_request_fields(&mut response, original_request);

    Ok(response)
}

pub struct ProxyHttpResponse {
    pub status: String,
    pub content_type: String,
    pub body: Vec<u8>,
}

pub struct UpstreamProxyResponse {
    pub status_code: u16,
    pub content_type: String,
    pub is_stream: bool,
    pub response: reqwest::Response,
}

impl UpstreamProxyResponse {
    pub fn status(&self) -> String {
        http_status_line(self.status_code)
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }
}

pub struct ChatSseToResponsesConverter {
    buffer: String,
    utf8_remainder: Vec<u8>,
    state: ChatSseState,
    failed: bool,
}

impl Default for ChatSseToResponsesConverter {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            utf8_remainder: Vec::new(),
            state: ChatSseState::default(),
            failed: false,
        }
    }
}

impl ChatSseToResponsesConverter {
    pub fn with_request(original_request: &Value) -> Self {
        Self {
            state: ChatSseState::with_request(original_request),
            ..Self::default()
        }
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) -> Vec<u8> {
        append_utf8_safe(&mut self.buffer, &mut self.utf8_remainder, bytes);
        let mut output = String::new();
        while let Some(block) = take_sse_block(&mut self.buffer) {
            if block.trim().is_empty() {
                continue;
            }
            self.handle_block(&block, &mut output);
            if self.failed {
                break;
            }
        }
        output.into_bytes()
    }

    pub fn finish(&mut self) -> Vec<u8> {
        if !self.utf8_remainder.is_empty() {
            self.buffer
                .push_str(&String::from_utf8_lossy(&self.utf8_remainder));
            self.utf8_remainder.clear();
        }

        let mut output = String::new();
        if !self.failed {
            self.state.finalize_into(&mut output);
        }
        output.into_bytes()
    }

    pub fn fail(&mut self, message: String, error_type: Option<String>) -> Vec<u8> {
        let mut output = String::new();
        self.state.failed_into(&mut output, message, error_type);
        self.failed = true;
        output.into_bytes()
    }

    fn handle_block(&mut self, block: &str, output: &mut String) {
        let mut event_name: Option<String> = None;
        let mut data_parts = Vec::new();
        for line in block.lines() {
            if let Some(event) = strip_sse_field(line, "event") {
                event_name = Some(event.trim().to_string());
            }
            if let Some(data) = strip_sse_field(line, "data") {
                data_parts.push(data.to_string());
            }
        }

        if data_parts.is_empty() {
            return;
        }
        let data = data_parts.join("\n");
        if data.trim() == "[DONE]" {
            self.state.finalize_into(output);
            return;
        }

        let Ok(chunk) = serde_json::from_str::<Value>(&data) else {
            return;
        };
        if event_name.as_deref() == Some("error") || chunk.get("error").is_some() {
            let (message, error_type) = extract_chat_sse_error(&chunk);
            self.state.failed_into(output, message, error_type);
            self.failed = true;
            return;
        }
        self.state.handle_chat_chunk_into(&chunk, output);
    }
}

pub fn is_responses_proxy_path(path: &str) -> bool {
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    matches!(
        path,
        "/responses"
            | "/v1/responses"
            | "/v1/v1/responses"
            | "/codex/v1/responses"
            | "/responses/compact"
            | "/v1/responses/compact"
            | "/v1/v1/responses/compact"
            | "/codex/v1/responses/compact"
    )
}

pub fn is_chat_completions_proxy_path(path: &str) -> bool {
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    matches!(
        path,
        "/chat/completions"
            | "/v1/chat/completions"
            | "/v1/v1/chat/completions"
            | "/codex/v1/chat/completions"
    )
}

pub fn is_models_proxy_path(path: &str) -> bool {
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    matches!(
        path,
        "/models" | "/v1/models" | "/v1/v1/models" | "/codex/v1/models"
    )
}

pub fn is_claude_desktop_models_proxy_path(path: &str) -> bool {
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    matches!(
        path,
        "/claude-desktop/models" | "/claude-desktop/v1/models" | "/claude-desktop/v1/v1/models"
    )
}

pub fn is_claude_desktop_messages_proxy_path(path: &str) -> bool {
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    matches!(
        path,
        "/claude-desktop/messages"
            | "/claude-desktop/v1/messages"
            | "/claude-desktop/v1/v1/messages"
    )
}

pub fn is_claude_desktop_count_tokens_proxy_path(path: &str) -> bool {
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    matches!(
        path,
        "/claude-desktop/messages/count_tokens"
            | "/claude-desktop/v1/messages/count_tokens"
            | "/claude-desktop/v1/v1/messages/count_tokens"
    )
}

pub fn is_claude_desktop_gateway_health_path(path: &str) -> bool {
    path.split_once('?').map_or(path, |(path, _)| path) == "/claude-desktop"
}

pub fn claude_desktop_safe_models() -> Vec<Value> {
    claude_desktop_safe_models_with_labels("")
}

pub fn claude_desktop_default_model_list() -> String {
    [
        CLAUDE_DESKTOP_DEFAULT_FABLE_MODEL,
        CLAUDE_DESKTOP_DEFAULT_HAIKU_MODEL,
        CLAUDE_DESKTOP_DEFAULT_OPUS_MODEL,
        CLAUDE_DESKTOP_DEFAULT_SONNET_MODEL,
    ]
    .join("\n")
}

pub fn claude_desktop_safe_models_with_labels(model_list: &str) -> Vec<Value> {
    let parsed_models = parse_claude_desktop_model_list(model_list);
    let default_models = parse_claude_desktop_model_list(&claude_desktop_default_model_list());
    // An explicitly supplied [1M] declaration is meaningful.  Do not collapse
    // it into the legacy all-1M default merely because the model names happen
    // to match the default role names.
    let uses_default_mapping = model_list.trim().is_empty()
        || (parsed_models
            .iter()
            .map(|entry| entry.model.as_str())
            .eq(default_models.iter().map(|entry| entry.model.as_str()))
            && parsed_models.iter().all(|entry| !entry.supports_1m));
    if uses_default_mapping {
        return vec![
            claude_desktop_model_entry(
                CLAUDE_DESKTOP_SAFE_FABLE_MODEL,
                Some(CLAUDE_DESKTOP_DEFAULT_FABLE_MODEL),
                true,
            ),
            claude_desktop_model_entry(
                CLAUDE_DESKTOP_SAFE_HAIKU_MODEL,
                Some(CLAUDE_DESKTOP_DEFAULT_HAIKU_MODEL),
                true,
            ),
            claude_desktop_model_entry(
                CLAUDE_DESKTOP_SAFE_OPUS_MODEL,
                Some(CLAUDE_DESKTOP_DEFAULT_OPUS_MODEL),
                true,
            ),
            claude_desktop_model_entry(
                CLAUDE_DESKTOP_SAFE_SONNET_MODEL,
                Some(CLAUDE_DESKTOP_DEFAULT_SONNET_MODEL),
                true,
            ),
        ];
    }
    let models = non_empty_model_entries_or_default(model_list);
    let fallback = models.first().cloned();
    let fable = pick_model_entry_by_keyword(&models, "fable")
        .or_else(|| models.first().cloned())
        .or_else(|| fallback.clone());
    let haiku = pick_model_entry_by_keyword(&models, "haiku")
        .or_else(|| models.get(1).cloned())
        .or_else(|| fallback.clone());
    let opus = pick_model_entry_by_keyword(&models, "opus")
        .or_else(|| models.get(2).cloned())
        .or_else(|| fallback.clone());
    let sonnet = pick_model_entry_by_keyword(&models, "sonnet")
        .or_else(|| models.get(3).cloned())
        .or_else(|| fallback.clone());

    let mut result = Vec::new();
    result.push(claude_desktop_model_entry(
        CLAUDE_DESKTOP_SAFE_FABLE_MODEL,
        fable.as_ref().map(|entry| entry.model.as_str()),
        fable.as_ref().is_some_and(|entry| entry.supports_1m),
    ));
    result.push(claude_desktop_model_entry(
        CLAUDE_DESKTOP_SAFE_HAIKU_MODEL,
        haiku.as_ref().map(|entry| entry.model.as_str()),
        haiku.as_ref().is_some_and(|entry| entry.supports_1m),
    ));
    result.push(claude_desktop_model_entry(
        CLAUDE_DESKTOP_SAFE_OPUS_MODEL,
        opus.as_ref().map(|entry| entry.model.as_str()),
        opus.as_ref().is_some_and(|entry| entry.supports_1m),
    ));
    result.push(claude_desktop_model_entry(
        CLAUDE_DESKTOP_SAFE_SONNET_MODEL,
        sonnet.as_ref().map(|entry| entry.model.as_str()),
        sonnet.as_ref().is_some_and(|entry| entry.supports_1m),
    ));
    result
}

/// Builds the model catalogue that Claude Desktop reads from a configured
/// supplier.  `None` preserves the historical provider-request behaviour;
/// supplier profiles always pass an explicit mode so their mapping and direct
/// lists cannot leak into one another.
pub fn claude_desktop_inference_models(
    model_list: &str,
    model_mapping_enabled: Option<bool>,
    model_mapping_json: &str,
) -> Vec<Value> {
    match model_mapping_enabled {
        None => claude_desktop_safe_models_with_labels(model_list),
        Some(false) => parse_claude_desktop_model_list(model_list)
            .into_iter()
            .map(|entry| claude_desktop_model_entry(&entry.model, None, entry.supports_1m))
            .collect(),
        Some(true) => claude_desktop_mapping_inference_models(model_mapping_json)
            .unwrap_or_else(|| claude_desktop_safe_models_with_labels("")),
    }
}

/// Mirrors the route whitelist enforced by Claude Desktop.  A profile with an
/// invalid route causes Claude Desktop to reject its complete model catalogue,
/// therefore direct mode never accepts an arbitrary upstream ID here.
pub fn is_claude_desktop_safe_model_id(model: &str) -> bool {
    let normalized = model.trim().to_ascii_lowercase();
    if normalized.contains("[1m]") {
        return false;
    }
    let Some(route_tail) = normalized
        .strip_prefix("anthropic/claude-")
        .or_else(|| normalized.strip_prefix("claude-"))
    else {
        return false;
    };
    ["sonnet-", "opus-", "haiku-", "fable-"]
        .into_iter()
        .any(|prefix| {
            route_tail
                .strip_prefix(prefix)
                .is_some_and(|version| !version.is_empty())
        })
}

fn claude_desktop_mapping_inference_models(model_mapping_json: &str) -> Option<Vec<Value>> {
    let entries = serde_json::from_str::<Value>(model_mapping_json.trim())
        .ok()?
        .as_array()?
        .iter()
        .filter_map(|entry| {
            let route_id = entry.get("routeId")?.as_str()?.trim();
            if route_id.is_empty() || !is_claude_desktop_safe_model_id(route_id) {
                return None;
            }
            let request_model = entry
                .get("requestModel")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|model| !model.is_empty());
            let display_name = entry
                .get("displayName")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .or(request_model);
            let supports_1m = entry
                .get("supports1m")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || request_model
                    .is_some_and(|model| model.ends_with("[1M]") || model.ends_with("[1m]"));
            Some(claude_desktop_model_entry(
                route_id,
                display_name.map(strip_one_m_suffix),
                supports_1m,
            ))
        })
        .collect::<Vec<_>>();
    (!entries.is_empty()).then_some(entries)
}

pub fn claude_desktop_models_response(model_list: &str) -> Value {
    claude_desktop_models_response_from_inference_models(claude_desktop_safe_models_with_labels(
        model_list,
    ))
}

fn claude_desktop_models_response_from_inference_models(inference_models: Vec<Value>) -> Value {
    let data = inference_models
        .into_iter()
        .filter_map(|item| {
            let object = item.as_object()?;
            let id = object.get("name")?.as_str()?.to_string();
            let mut model = json!({
                "type": "model",
                "id": id,
                "created_at": 0,
            });
            if object
                .get("supports1m")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                model["supports1m"] = json!(true);
            }
            if let Some(label) = object.get("labelOverride").and_then(Value::as_str) {
                model["display_name"] = json!(label);
            }
            Some(model)
        })
        .collect::<Vec<_>>();
    let first_id = data
        .first()
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let last_id = data
        .last()
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string);

    json!({
        "data": data,
        "has_more": false,
        "first_id": first_id,
        "last_id": last_id,
    })
}

fn load_proxy_settings(store: &SettingsStore) -> anyhow::Result<BackendSettings> {
    store
        .load_strict()
        .map_err(|error| anyhow::anyhow!("failed to load protocol proxy settings: {error:#}"))
}

pub async fn open_responses_proxy_request(body: &str) -> anyhow::Result<UpstreamProxyResponse> {
    let settings = load_proxy_settings(&SettingsStore::default())?;
    let relay = settings.active_relay_profile();
    open_responses_proxy_request_with_relay(body, &relay).await
}

async fn open_responses_proxy_request_with_relay(
    body: &str,
    relay: &RelayProfile,
) -> anyhow::Result<UpstreamProxyResponse> {
    if relay.protocol != RelayProtocol::ChatCompletions {
        anyhow::bail!("当前中转未启用 Chat Completions 协议代理");
    }
    if relay.base_url.trim().is_empty() {
        anyhow::bail!("Chat Completions 上游 Base URL 不能为空");
    }
    if relay.api_key.trim().is_empty() {
        anyhow::bail!("Chat Completions 上游 Key 不能为空");
    }

    let request_json: Value = serde_json::from_str(body)?;
    let is_stream = request_json
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let chat_request = responses_to_chat_completions(request_json.clone())?;
    let client = if is_stream {
        crate::http_client::streaming_proxy_client(&relay.user_agent)?
    } else {
        crate::http_client::proxied_client(&relay.user_agent)?
    };
    let upstream = client
        .post(chat_completions_url(&relay.base_url))
        .bearer_auth(relay.api_key.trim())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&chat_request)
        .send()
        .await?;
    let status_code = upstream.status().as_u16();
    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();

    Ok(UpstreamProxyResponse {
        status_code,
        is_stream: is_stream || content_type.contains("text/event-stream"),
        content_type,
        response: upstream,
    })
}

pub async fn open_models_proxy_request() -> anyhow::Result<UpstreamProxyResponse> {
    let settings = load_proxy_settings(&SettingsStore::default())?;
    let relay = settings.active_relay_profile();
    if relay.protocol != RelayProtocol::ChatCompletions {
        anyhow::bail!("当前中转未启用 Chat Completions 协议代理");
    }
    if relay.base_url.trim().is_empty() {
        anyhow::bail!("Chat Completions 上游 Base URL 不能为空");
    }
    if relay.api_key.trim().is_empty() {
        anyhow::bail!("Chat Completions 上游 Key 不能为空");
    }

    let client = crate::http_client::proxied_client(&relay.user_agent)?;
    let upstream = client
        .get(models_url(&relay.base_url))
        .bearer_auth(relay.api_key.trim())
        .send()
        .await?;
    let status_code = upstream.status().as_u16();
    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json; charset=utf-8")
        .to_string();

    Ok(UpstreamProxyResponse {
        status_code,
        is_stream: false,
        content_type,
        response: upstream,
    })
}

pub fn local_claude_desktop_models_proxy_response() -> anyhow::Result<ProxyHttpResponse> {
    let settings = load_proxy_settings(&SettingsStore::default())?;
    let body = serde_json::to_vec(&claude_desktop_models_response_for_settings(&settings))?;
    Ok(ProxyHttpResponse {
        status: "200 OK".to_string(),
        content_type: "application/json; charset=utf-8".to_string(),
        body,
    })
}

pub fn claude_desktop_models_response_for_settings(
    settings: &crate::settings::BackendSettings,
) -> Value {
    let relay = settings.active_relay_profile_for_target("claude-desktop");
    claude_desktop_models_response_from_inference_models(claude_desktop_inference_models(
        &relay.model_list,
        Some(relay.model_mapping_enabled),
        &relay.model_mapping_json,
    ))
}

pub fn claude_desktop_count_tokens_response(body: &str) -> anyhow::Result<Value> {
    let request: Value = serde_json::from_str(body)?;
    if !request.is_object() {
        anyhow::bail!("Claude Desktop count_tokens 请求必须是 JSON 对象");
    }
    Ok(json!({
        "input_tokens": estimate_json_tokens(&request)
    }))
}

pub fn local_claude_desktop_gateway_health_response() -> anyhow::Result<ProxyHttpResponse> {
    let settings = load_proxy_settings(&SettingsStore::default())?;
    let relay = settings.active_relay_profile_for_target("claude-desktop");
    let base_url = claude_desktop_resolved_upstream_base_url(&relay, &settings);
    let api_key = claude_desktop_upstream_api_key(&relay, &settings);
    let model_count = claude_desktop_inference_models(
        &relay.model_list,
        Some(relay.model_mapping_enabled),
        &relay.model_mapping_json,
    )
    .len();
    let ready = !base_url.trim().is_empty() && !api_key.trim().is_empty() && model_count > 0;
    let status = if ready {
        "200 OK"
    } else {
        "503 Service Unavailable"
    };
    let body = serde_json::to_vec(&json!({
        "status": if ready { "ok" } else { "failed" },
        "service": "claude-desktop-gateway",
        "model_count": model_count
    }))?;
    Ok(ProxyHttpResponse {
        status: status.to_string(),
        content_type: "application/json; charset=utf-8".to_string(),
        body,
    })
}

pub async fn open_claude_desktop_messages_proxy_request(
    body: &str,
) -> anyhow::Result<UpstreamProxyResponse> {
    open_claude_desktop_messages_proxy_request_with_metadata(
        body,
        &ClaudeMessagesRequestMetadata::default(),
    )
    .await
}

pub async fn open_claude_desktop_messages_proxy_request_with_metadata(
    body: &str,
    metadata: &ClaudeMessagesRequestMetadata,
) -> anyhow::Result<UpstreamProxyResponse> {
    let settings = load_proxy_settings(&SettingsStore::default())?;
    let relay = settings.active_relay_profile_for_target("claude-desktop");
    open_claude_desktop_messages_proxy_request_with_relay(body, &relay, &settings, metadata).await
}

async fn open_claude_desktop_messages_proxy_request_with_relay(
    body: &str,
    relay: &RelayProfile,
    settings: &crate::settings::BackendSettings,
    metadata: &ClaudeMessagesRequestMetadata,
) -> anyhow::Result<UpstreamProxyResponse> {
    let base_url = claude_desktop_resolved_upstream_base_url(&relay, &settings);
    if base_url.trim().is_empty() {
        anyhow::bail!("Claude Desktop 上游 Base URL 不能为空");
    }
    let api_key = crate::settings::relay_profile_resolved_api_key(relay);
    if api_key.trim().is_empty() {
        anyhow::bail!("当前 Claude Desktop 供应商 Key 不能为空");
    }

    let mut request_json: Value = serde_json::from_str(body)?;
    let original_model = {
        let request_object = request_json
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Claude Desktop 请求正文必须是 JSON 对象"))?;
        request_object
            .get("model")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Claude Desktop 请求缺少有效的 model 字段"))?
            .to_string()
    };
    let mapped_model = claude_desktop_model_mapping_for(&original_model, relay);
    let mapping_applied = mapped_model.is_some();
    let upstream_model = mapped_model.unwrap_or_else(|| original_model.clone());
    let applied_body_override_fields = parsed_local_proxy_body_override(&relay.body_override)
        .map(|override_body| apply_local_proxy_body_override(&mut request_json, &override_body))
        .unwrap_or_default();
    // The saved route mapping owns the upstream model. Reapply it after body
    // overrides so an unrelated override cannot silently undo the route.
    request_json["model"] = Value::String(upstream_model.clone());
    let is_stream = request_json
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let client = if is_stream {
        crate::http_client::streaming_proxy_client(&relay.user_agent)?
    } else {
        crate::http_client::proxied_client(&relay.user_agent)?
    };
    let upstream_url = claude_messages_url(&base_url);
    let request_body_fields = request_json
        .as_object()
        .map(|object| object.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let uses_anthropic_api_key = crate::settings::relay_profile_uses_anthropic_api_key(relay);
    let auth_header_mode = if uses_anthropic_api_key {
        "x-api-key"
    } else {
        "authorization"
    };
    let mut request = client
        .post(&upstream_url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&request_json);
    if metadata.beta {
        request = request.query(&[("beta", "true")]);
    }
    request = crate::http_client::apply_api_auth_headers(
        request,
        api_key.trim(),
        uses_anthropic_api_key,
        false,
    );
    request = request.header(
        "anthropic-version",
        metadata
            .anthropic_version
            .as_deref()
            .unwrap_or(crate::http_client::ANTHROPIC_VERSION),
    );
    if let Some(anthropic_beta) = metadata.anthropic_beta.as_deref() {
        request = request.header("anthropic-beta", anthropic_beta);
    }
    let applied_header_overrides = parsed_local_proxy_header_override(&relay.header_override);
    for (name, value) in &applied_header_overrides {
        request = request.header(name.as_str(), value.as_str());
    }
    let upstream = request.send().await?;
    let status_code = upstream.status().as_u16();
    let _ = crate::diagnostic_log::append_diagnostic_log("proxy.claude_desktop_upstream_route", {
        let mut detail = claude_desktop_route_diagnostic(
            &relay.id,
            &original_model,
            &upstream_model,
            status_code,
        );
        if let Some(object) = detail.as_object_mut() {
            object.insert(
                "target_app".to_string(),
                Value::String("claude-desktop".to_string()),
            );
            object.insert(
                "protocol".to_string(),
                Value::String(format!("{:?}", relay.protocol)),
            );
            object.insert(
                "upstream_path".to_string(),
                Value::String(sanitized_url_path(&upstream_url)),
            );
            object.insert(
                "auth_header_mode".to_string(),
                Value::String(auth_header_mode.to_string()),
            );
            object.insert("mapping_applied".to_string(), Value::Bool(mapping_applied));
            object.insert("has_anthropic_version".to_string(), Value::Bool(true));
            object.insert(
                "has_inbound_anthropic_version".to_string(),
                Value::Bool(metadata.has_anthropic_version()),
            );
            object.insert(
                "has_anthropic_beta".to_string(),
                Value::Bool(metadata.has_anthropic_beta()),
            );
            object.insert("beta_query".to_string(), Value::Bool(metadata.beta()));
            object.insert(
                "content_type".to_string(),
                Value::String("application/json".to_string()),
            );
            object.insert("is_stream".to_string(), Value::Bool(is_stream));
            object.insert(
                "request_body_fields".to_string(),
                Value::Array(
                    request_body_fields
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect(),
                ),
            );
            if !applied_header_overrides.is_empty() {
                object.insert(
                    "applied_custom_headers".to_string(),
                    Value::Array(
                        applied_header_overrides
                            .keys()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
            if !applied_body_override_fields.is_empty() {
                object.insert(
                    "applied_body_fields".to_string(),
                    Value::Array(
                        applied_body_override_fields
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
        }
        detail
    });
    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();

    Ok(UpstreamProxyResponse {
        status_code,
        is_stream: is_stream || content_type.contains("text/event-stream"),
        content_type,
        response: upstream,
    })
}

fn claude_desktop_route_diagnostic(
    profile_id: &str,
    original_model: &str,
    upstream_model: &str,
    http_status: u16,
) -> Value {
    json!({
        "profile_id": profile_id,
        "original_model": original_model,
        "upstream_model": upstream_model,
        "http_status": http_status
    })
}

fn sanitized_url_path(raw_url: &str) -> String {
    url::Url::parse(raw_url)
        .map(|url| url.path().to_string())
        .unwrap_or_else(|_| {
            raw_url
                .split_once('?')
                .map(|(path, _)| path)
                .unwrap_or(raw_url)
                .to_string()
        })
}

pub fn claude_desktop_resolved_upstream_base_url(
    relay: &crate::settings::RelayProfile,
    settings: &crate::settings::BackendSettings,
) -> String {
    provider_string_from_json_env(
        &relay.config_contents,
        &[
            "ANTHROPIC_BASE_URL",
            "CLAUDE_BASE_URL",
            "base_url",
            "baseUrl",
        ],
    )
    .or_else(|| provider_string_from_toml(&relay.config_contents, "base_url"))
    .or_else(|| non_empty_string(&relay.upstream_base_url))
    .or_else(|| {
        let value = non_empty_string(&relay.base_url)?;
        if value == local_claude_desktop_proxy_base_url(DEFAULT_CLAUDE_DESKTOP_PROXY_PORT) {
            None
        } else {
            Some(value)
        }
    })
    .or_else(|| non_empty_string(&settings.relay_base_url))
    .unwrap_or_else(|| DEFAULT_CLAUDE_DESKTOP_UPSTREAM_BASE_URL.to_string())
}

fn claude_desktop_upstream_api_key(
    relay: &crate::settings::RelayProfile,
    settings: &crate::settings::BackendSettings,
) -> String {
    non_empty_string(&crate::settings::relay_profile_resolved_api_key(relay))
        .or_else(|| non_empty_string(&settings.relay_api_key))
        .or_else(|| {
            [
                "ANTHROPIC_AUTH_TOKEN",
                "ANTHROPIC_API_KEY",
                "OPENAI_API_KEY",
            ]
            .into_iter()
            .find_map(|key| {
                std::env::var(key)
                    .ok()
                    .and_then(|value| non_empty_string(&value))
            })
        })
        .unwrap_or_default()
}

pub fn claude_desktop_model_mapping_for(
    original_model: &str,
    relay: &crate::settings::RelayProfile,
) -> Option<String> {
    if !relay.model_mapping_enabled {
        return None;
    }
    claude_desktop_model_mapping_json_for(original_model, relay)
}

fn claude_desktop_model_mapping_json_for(
    original_model: &str,
    relay: &crate::settings::RelayProfile,
) -> Option<String> {
    let raw = relay.model_mapping_json.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed: Value = serde_json::from_str(raw).ok()?;
    let requested = original_model.trim();
    if requested.is_empty() {
        return None;
    }
    let entries = parsed.as_array()?;
    entries
        .iter()
        .find(|entry| {
            entry
                .get("routeId")
                .and_then(Value::as_str)
                .map(str::trim)
                .is_some_and(|route_id| route_id == requested)
        })
        .and_then(|entry| entry.get("requestModel").and_then(Value::as_str))
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(strip_one_m_suffix)
        .map(ToOwned::to_owned)
}

#[derive(Clone)]
struct ClaudeDesktopModelListEntry {
    model: String,
    supports_1m: bool,
}

fn parse_claude_desktop_model_list(raw: &str) -> Vec<ClaudeDesktopModelListEntry> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| ClaudeDesktopModelListEntry {
            supports_1m: line.ends_with("[1M]") || line.ends_with("[1m]"),
            model: strip_one_m_suffix(line).trim().to_string(),
        })
        .filter(|entry| !entry.model.is_empty())
        .collect()
}

fn non_empty_model_entries_or_default(raw: &str) -> Vec<ClaudeDesktopModelListEntry> {
    let models = parse_claude_desktop_model_list(raw);
    if models.is_empty() {
        parse_claude_desktop_model_list(&claude_desktop_default_model_list())
    } else {
        models
    }
}

fn pick_model_entry_by_keyword(
    models: &[ClaudeDesktopModelListEntry],
    keyword: &str,
) -> Option<ClaudeDesktopModelListEntry> {
    models
        .iter()
        .find(|entry| entry.model.to_ascii_lowercase().contains(keyword))
        .cloned()
}

fn strip_one_m_suffix(model: &str) -> &str {
    let trimmed = model.trim();
    trimmed
        .strip_suffix("[1M]")
        .or_else(|| trimmed.strip_suffix("[1m]"))
        .map(str::trim)
        .unwrap_or(trimmed)
}

fn estimate_json_tokens(value: &Value) -> u64 {
    match value {
        Value::Null => 0,
        Value::Bool(_) | Value::Number(_) => 1,
        Value::String(text) => estimate_text_tokens(text),
        Value::Array(items) => items.iter().map(estimate_json_tokens).sum(),
        Value::Object(object) => object
            .iter()
            .map(|(key, value)| estimate_text_tokens(key) + estimate_json_tokens(value))
            .sum(),
    }
}

fn estimate_text_tokens(text: &str) -> u64 {
    let mut tokens = 0_u64;
    let mut ascii_run = 0_u64;
    let flush_ascii_run = |tokens: &mut u64, run: &mut u64| {
        if *run > 0 {
            *tokens += (*run).div_ceil(4);
            *run = 0;
        }
    };

    for character in text.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
            ascii_run += 1;
        } else if character.is_ascii_whitespace() {
            flush_ascii_run(&mut tokens, &mut ascii_run);
        } else {
            flush_ascii_run(&mut tokens, &mut ascii_run);
            tokens += 1;
        }
    }
    flush_ascii_run(&mut tokens, &mut ascii_run);
    tokens
}

fn claude_desktop_model_entry(
    name: &str,
    label_override: Option<&str>,
    supports_1m: bool,
) -> Value {
    let mut item = json!({ "name": name });
    if let Some(label) = label_override
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != name)
    {
        item["labelOverride"] = json!(label);
    }
    if supports_1m {
        item["supports1m"] = json!(true);
    }
    item
}

fn provider_string_from_toml(contents: &str, key: &str) -> Option<String> {
    let doc = contents.parse::<toml_edit::DocumentMut>().ok()?;
    doc.get("model_providers")
        .and_then(toml_edit::Item::as_table)
        .and_then(|providers| {
            providers.iter().find_map(|(_, item)| {
                item.get(key)
                    .and_then(toml_edit::Item::as_value)
                    .and_then(toml_edit::Value::as_str)
            })
        })
        .or_else(|| {
            doc.get(key)
                .and_then(toml_edit::Item::as_value)
                .and_then(toml_edit::Value::as_str)
        })
        .and_then(non_empty_string)
}

fn provider_string_from_json_env(contents: &str, keys: &[&str]) -> Option<String> {
    let value: Value = serde_json::from_str(contents.trim().trim_start_matches('\u{feff}')).ok()?;
    let objects = [
        value.as_object(),
        value.get("env").and_then(Value::as_object),
    ];
    for object in objects.into_iter().flatten() {
        for key in keys {
            if let Some(value) = object
                .get(*key)
                .and_then(Value::as_str)
                .and_then(non_empty_string)
            {
                return Some(value);
            }
        }
    }
    None
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub async fn open_chat_completions_proxy_request(
    body: &str,
) -> anyhow::Result<UpstreamProxyResponse> {
    let settings = load_proxy_settings(&SettingsStore::default())?;
    let relay = settings.active_relay_profile();
    if relay.protocol != RelayProtocol::ChatCompletions {
        anyhow::bail!("当前中转未启用 Chat Completions 协议代理");
    }
    if relay.base_url.trim().is_empty() {
        anyhow::bail!("Chat Completions 上游 Base URL 不能为空");
    }
    if relay.api_key.trim().is_empty() {
        anyhow::bail!("Chat Completions 上游 Key 不能为空");
    }

    let request_json: Value = serde_json::from_str(body)?;
    let is_stream = request_json
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let client = if is_stream {
        crate::http_client::streaming_proxy_client(&relay.user_agent)?
    } else {
        crate::http_client::proxied_client(&relay.user_agent)?
    };
    let upstream = client
        .post(chat_completions_url(&relay.base_url))
        .bearer_auth(relay.api_key.trim())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&request_json)
        .send()
        .await?;
    let status_code = upstream.status().as_u16();
    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();

    Ok(UpstreamProxyResponse {
        status_code,
        is_stream: is_stream || content_type.contains("text/event-stream"),
        content_type,
        response: upstream,
    })
}

pub async fn handle_responses_proxy_request(body: &str) -> anyhow::Result<ProxyHttpResponse> {
    let request_json: Value = serde_json::from_str(body)?;
    let upstream = open_responses_proxy_request(body).await?;
    let status_code = upstream.status_code;
    let upstream_content_type = upstream.content_type.clone();
    let is_stream = upstream.is_stream;
    let upstream_body = upstream.response.bytes().await?;

    if !(200..300).contains(&status_code) {
        let error =
            responses_error_from_upstream(status_code, &upstream_content_type, &upstream_body);
        return Ok(ProxyHttpResponse {
            status: http_status_line(status_code),
            content_type: "application/json; charset=utf-8".to_string(),
            body: serde_json::to_vec(&error)?,
        });
    }

    if is_stream {
        let text = String::from_utf8_lossy(&upstream_body);
        return Ok(ProxyHttpResponse {
            status: "200 OK".to_string(),
            content_type: "text/event-stream; charset=utf-8".to_string(),
            body: chat_sse_to_responses_sse_with_request(&text, &request_json).into_bytes(),
        });
    }

    let chat_json: Value = serde_json::from_slice(&upstream_body)?;
    let response_json = chat_completion_to_response_with_request(chat_json, &request_json)?;
    Ok(ProxyHttpResponse {
        status: "200 OK".to_string(),
        content_type: "application/json; charset=utf-8".to_string(),
        body: serde_json::to_vec(&response_json)?,
    })
}

pub fn chat_completions_url(base_url: &str) -> String {
    let skip_version_prefix = base_url.trim().ends_with('#');
    let base = base_url.trim().trim_end_matches('#').trim_end_matches('/');
    if base.to_ascii_lowercase().ends_with("/chat/completions") {
        return base.to_string();
    }
    let origin_only = base
        .split_once("://")
        .map_or(!base.contains('/'), |(_, rest)| !rest.contains('/'));
    let mut url = if skip_version_prefix || has_version_suffix(base) || !origin_only {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    };
    while url.contains("/v1/v1") {
        url = url.replace("/v1/v1", "/v1");
    }
    url
}

pub fn models_url(base_url: &str) -> String {
    let skip_version_prefix = base_url.trim().ends_with('#');
    let mut base = base_url
        .trim()
        .trim_end_matches('#')
        .trim_end_matches('/')
        .to_string();
    if base.to_ascii_lowercase().ends_with("/chat/completions") {
        base.truncate(base.len() - "/chat/completions".len());
    }
    if base.to_ascii_lowercase().ends_with("/models") {
        return base;
    }
    let origin_only = base
        .split_once("://")
        .map_or(!base.contains('/'), |(_, rest)| !rest.contains('/'));
    let mut url = if skip_version_prefix || has_version_suffix(&base) || !origin_only {
        format!("{base}/models")
    } else {
        format!("{base}/v1/models")
    };
    while url.contains("/v1/v1") {
        url = url.replace("/v1/v1", "/v1");
    }
    url
}

pub fn claude_messages_url(base_url: &str) -> String {
    let skip_version_prefix = base_url.trim().ends_with('#');
    let mut base = base_url
        .trim()
        .trim_end_matches('#')
        .trim_end_matches('/')
        .to_string();
    if base.to_ascii_lowercase().ends_with("/messages") {
        return base;
    }
    if base.to_ascii_lowercase().ends_with("/chat/completions") {
        base.truncate(base.len() - "/chat/completions".len());
    }
    let origin_only = base
        .split_once("://")
        .map_or(!base.contains('/'), |(_, rest)| !rest.contains('/'));
    let mut url = if skip_version_prefix || has_version_suffix(&base) || !origin_only {
        format!("{base}/messages")
    } else {
        format!("{base}/v1/messages")
    };
    while url.contains("/v1/v1") {
        url = url.replace("/v1/v1", "/v1");
    }
    url
}

fn has_version_suffix(base_url: &str) -> bool {
    let segment = base_url.rsplit('/').next().unwrap_or(base_url);
    let Some(rest) = segment.strip_prefix('v') else {
        return false;
    };
    rest.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

pub fn chat_sse_to_responses_sse(input: &str) -> String {
    let mut converter = ChatSseToResponsesConverter::default();
    let mut output = converter.push_bytes(input.as_bytes());
    output.extend(converter.finish());
    String::from_utf8(output).unwrap_or_default()
}

pub fn chat_sse_to_responses_sse_with_request(input: &str, original_request: &Value) -> String {
    let mut converter = ChatSseToResponsesConverter::with_request(original_request);
    let mut output = converter.push_bytes(input.as_bytes());
    output.extend(converter.finish());
    String::from_utf8(output).unwrap_or_default()
}

pub fn response_id_from_chat_id(id: Option<&str>) -> String {
    let id = id.unwrap_or("compat");
    if id.starts_with("resp_") {
        id.to_string()
    } else {
        format!("resp_{id}")
    }
}

fn push_sse(output: &mut String, event: &str, data: Value) {
    output.push_str("event: ");
    output.push_str(event);
    output.push_str("\ndata: ");
    output.push_str(&serde_json::to_string(&data).unwrap_or_default());
    output.push_str("\n\n");
}

#[derive(Debug, Default)]
struct TextItemState {
    output_index: Option<u32>,
    item_id: String,
    text: String,
    added: bool,
    done: bool,
}

#[derive(Debug, Default)]
struct ReasoningItemState {
    output_index: Option<u32>,
    item_id: String,
    text: String,
    added: bool,
    done: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum InlineThinkMode {
    #[default]
    Detecting,
    Reasoning,
    Text,
}

#[derive(Debug, Default)]
struct InlineThinkState {
    mode: InlineThinkMode,
    buffer: String,
}

#[derive(Debug, Default)]
struct ToolCallState {
    output_index: Option<u32>,
    item_id: String,
    call_id: String,
    name: String,
    arguments: String,
    added: bool,
    done: bool,
}

#[derive(Debug)]
struct ChatSseState {
    response_started: bool,
    completed: bool,
    response_id: String,
    model: String,
    created_at: u64,
    next_output_index: u32,
    text: TextItemState,
    reasoning: ReasoningItemState,
    inline_think: InlineThinkState,
    tools: BTreeMap<usize, ToolCallState>,
    output_items: Vec<(u32, Value)>,
    latest_usage: Option<Value>,
    finish_reason: Option<String>,
    tool_context: CodexToolContext,
    original_request: Option<Value>,
}

impl Default for ChatSseState {
    fn default() -> Self {
        Self {
            response_started: false,
            completed: false,
            response_id: "resp_compat".to_string(),
            model: String::new(),
            created_at: 0,
            next_output_index: 0,
            text: TextItemState::default(),
            reasoning: ReasoningItemState::default(),
            inline_think: InlineThinkState::default(),
            tools: BTreeMap::new(),
            output_items: Vec::new(),
            latest_usage: None,
            finish_reason: None,
            tool_context: CodexToolContext::default(),
            original_request: None,
        }
    }
}

impl ChatSseState {
    fn with_request(original_request: &Value) -> Self {
        Self {
            tool_context: build_codex_tool_context(original_request.get("tools")),
            original_request: Some(original_request.clone()),
            ..Self::default()
        }
    }

    fn handle_chat_chunk_into(&mut self, chunk: &Value, output: &mut String) {
        if let Some(id) = chunk.get("id").and_then(Value::as_str) {
            self.response_id = response_id_from_chat_id(Some(id));
        }
        if let Some(model) = chunk.get("model").and_then(Value::as_str) {
            if !model.is_empty() {
                self.model = model.to_string();
            }
        }
        if let Some(created) = chunk.get("created").and_then(Value::as_u64) {
            self.created_at = created;
        }
        self.ensure_response_started_into(output);

        if let Some(usage) = chunk.get("usage").filter(|value| !value.is_null()) {
            self.latest_usage = Some(chat_usage_to_responses_usage(Some(usage)));
        }

        let Some(choice) = chunk
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
        else {
            return;
        };

        if let Some(delta) = choice.get("delta") {
            if let Some(reasoning) = chat_delta_reasoning_text(delta) {
                self.push_reasoning_delta_into(&reasoning, output);
            }

            if let Some(content) = delta.get("content").and_then(Value::as_str) {
                if !content.is_empty() {
                    self.push_content_delta_into(content, output);
                }
            }

            if let Some(tool_calls) = delta.get("tool_calls").and_then(Value::as_array) {
                self.flush_inline_think_at_boundary_into(output);
                self.finalize_reasoning_into(output);
                for tool_call in tool_calls {
                    self.push_tool_call_delta_into(tool_call, output);
                }
            }
        }

        if let Some(finish_reason) = choice.get("finish_reason").and_then(Value::as_str) {
            self.finish_reason = Some(finish_reason.to_string());
        }
    }

    fn push_content_delta_into(&mut self, delta: &str, output: &mut String) {
        match self.inline_think.mode {
            InlineThinkMode::Text => {
                self.finalize_reasoning_into(output);
                self.push_text_delta_into(delta, output);
            }
            InlineThinkMode::Detecting => {
                self.inline_think.buffer.push_str(delta);
                match leading_think_prefix_decision(&self.inline_think.buffer) {
                    ThinkPrefixDecision::NeedMore => {}
                    ThinkPrefixDecision::Reasoning => {
                        self.inline_think.mode = InlineThinkMode::Reasoning;
                        self.drain_complete_inline_think_into(output);
                    }
                    ThinkPrefixDecision::Text => {
                        self.inline_think.mode = InlineThinkMode::Text;
                        let text = std::mem::take(&mut self.inline_think.buffer);
                        self.finalize_reasoning_into(output);
                        self.push_text_delta_into(&text, output);
                    }
                }
            }
            InlineThinkMode::Reasoning => {
                self.inline_think.buffer.push_str(delta);
                self.drain_complete_inline_think_into(output);
            }
        }
    }

    fn drain_complete_inline_think_into(&mut self, output: &mut String) {
        let Some((reasoning, answer)) = split_leading_think_block(&self.inline_think.buffer) else {
            return;
        };
        self.inline_think.mode = InlineThinkMode::Text;
        self.inline_think.buffer.clear();
        if !reasoning.is_empty() {
            self.push_reasoning_delta_into(&reasoning, output);
            self.finalize_reasoning_into(output);
        }
        if !answer.is_empty() {
            self.push_text_delta_into(&answer, output);
        }
    }

    fn flush_inline_think_at_boundary_into(&mut self, output: &mut String) {
        match self.inline_think.mode {
            InlineThinkMode::Text => {}
            InlineThinkMode::Detecting => {
                self.inline_think.mode = InlineThinkMode::Text;
                let text = std::mem::take(&mut self.inline_think.buffer);
                if !text.is_empty() {
                    self.finalize_reasoning_into(output);
                    self.push_text_delta_into(&text, output);
                }
            }
            InlineThinkMode::Reasoning => {
                let buffered = std::mem::take(&mut self.inline_think.buffer);
                self.inline_think.mode = InlineThinkMode::Text;
                if let Some((reasoning, answer)) = split_leading_think_block(&buffered) {
                    if !reasoning.is_empty() {
                        self.push_reasoning_delta_into(&reasoning, output);
                        self.finalize_reasoning_into(output);
                    }
                    if !answer.is_empty() {
                        self.push_text_delta_into(&answer, output);
                    }
                    return;
                }
                let reasoning = strip_leading_think_open_tag(&buffered).unwrap_or(buffered);
                if !reasoning.is_empty() {
                    self.push_reasoning_delta_into(&reasoning, output);
                    self.finalize_reasoning_into(output);
                }
            }
        }
    }

    fn ensure_response_started_into(&mut self, output: &mut String) {
        if self.response_started {
            return;
        }
        self.response_started = true;
        push_sse(
            output,
            "response.created",
            json!({
                "type": "response.created",
                "response": self.base_response("in_progress", Vec::new())
            }),
        );
        push_sse(
            output,
            "response.in_progress",
            json!({
                "type": "response.in_progress",
                "response": self.base_response("in_progress", Vec::new())
            }),
        );
    }

    fn push_reasoning_delta_into(&mut self, delta: &str, output: &mut String) {
        if !self.reasoning.added {
            let output_index = self.next_output_index();
            let item_id = format!("rs_{}", self.response_id);
            self.reasoning.output_index = Some(output_index);
            self.reasoning.item_id = item_id.clone();
            self.reasoning.added = true;

            push_sse(
                output,
                "response.output_item.added",
                json!({
                    "type": "response.output_item.added",
                    "output_index": output_index,
                    "item": {
                        "id": item_id,
                        "type": "reasoning",
                        "status": "in_progress",
                        "reasoning_content": "",
                        "summary": []
                    }
                }),
            );
            push_sse(
                output,
                "response.reasoning_summary_part.added",
                json!({
                    "type": "response.reasoning_summary_part.added",
                    "item_id": self.reasoning.item_id,
                    "output_index": output_index,
                    "summary_index": 0,
                    "part": { "type": "summary_text", "text": "" }
                }),
            );
        }

        self.reasoning.text.push_str(delta);
        let output_index = self.reasoning.output_index.unwrap_or(0);
        push_sse(
            output,
            "response.reasoning_summary_text.delta",
            json!({
                "type": "response.reasoning_summary_text.delta",
                "item_id": self.reasoning.item_id,
                "output_index": output_index,
                "summary_index": 0,
                "delta": delta
            }),
        );
    }

    fn push_text_delta_into(&mut self, delta: &str, output: &mut String) {
        if !self.text.added {
            let output_index = self.next_output_index();
            let item_id = format!("{}_msg", self.response_id);
            self.text.output_index = Some(output_index);
            self.text.item_id = item_id.clone();
            self.text.added = true;
            push_sse(
                output,
                "response.output_item.added",
                json!({
                    "type": "response.output_item.added",
                    "output_index": output_index,
                    "item": {
                        "id": item_id,
                        "type": "message",
                        "status": "in_progress",
                        "role": "assistant",
                        "content": []
                    }
                }),
            );
            push_sse(
                output,
                "response.content_part.added",
                json!({
                    "type": "response.content_part.added",
                    "item_id": self.text.item_id,
                    "output_index": output_index,
                    "content_index": 0,
                    "part": { "type": "output_text", "text": "", "annotations": [] }
                }),
            );
        }

        self.text.text.push_str(delta);
        let output_index = self.text.output_index.unwrap_or(0);
        push_sse(
            output,
            "response.output_text.delta",
            json!({
                "type": "response.output_text.delta",
                "item_id": self.text.item_id,
                "output_index": output_index,
                "content_index": 0,
                "delta": delta
            }),
        );
    }

    fn push_tool_call_delta_into(&mut self, tool_call: &Value, output: &mut String) {
        let chat_index = tool_call.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
        let id_delta = tool_call
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string);
        let function = tool_call.get("function").unwrap_or(&Value::Null);
        let name_delta = function
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string);
        let args_delta = function
            .get("arguments")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let mut should_add = false;
        let mut output_index = None;
        let mut item_id = String::new();
        let mut pending_arguments = String::new();

        {
            let state = self.tools.entry(chat_index).or_default();
            if let Some(id) = id_delta {
                state.call_id = id;
            }
            if let Some(name) = name_delta {
                if !name.is_empty() {
                    state.name = name;
                }
            }
            if !args_delta.is_empty() {
                state.arguments.push_str(&args_delta);
            }

            if !state.added && (!state.call_id.is_empty() || !state.name.is_empty()) {
                should_add = true;
                pending_arguments = state.arguments.clone();
            } else if state.added {
                output_index = state.output_index;
                item_id = state.item_id.clone();
            }
        }

        if should_add {
            let assigned = self.next_output_index();
            let state = self.tools.get_mut(&chat_index).expect("tool state exists");
            state.added = true;
            if state.call_id.is_empty() {
                state.call_id = format!("call_{chat_index}");
            }
            if state.name.is_empty() {
                state.name = "unknown_tool".to_string();
            }
            state.output_index = Some(assigned);
            state.item_id = format!("fc_{}", state.call_id);
            let added_item = tool_call_added_item(state, assigned, &self.tool_context);
            push_sse(output, "response.output_item.added", added_item);
            if !pending_arguments.is_empty() {
                push_tool_call_delta_sse(
                    output,
                    state,
                    assigned,
                    &pending_arguments,
                    &self.tool_context,
                );
            }
        } else if !args_delta.is_empty() {
            if let Some(output_index) = output_index {
                let state = ToolCallState {
                    output_index: Some(output_index),
                    item_id,
                    name: self
                        .tools
                        .get(&chat_index)
                        .map(|state| state.name.clone())
                        .unwrap_or_default(),
                    call_id: self
                        .tools
                        .get(&chat_index)
                        .map(|state| state.call_id.clone())
                        .unwrap_or_default(),
                    ..ToolCallState::default()
                };
                push_tool_call_delta_sse(
                    output,
                    &state,
                    output_index,
                    &args_delta,
                    &self.tool_context,
                );
            }
        }
    }

    fn finalize_into(&mut self, output: &mut String) {
        if self.completed {
            return;
        }
        self.ensure_response_started_into(output);
        self.flush_inline_think_at_boundary_into(output);
        self.finalize_reasoning_into(output);
        self.finalize_text_into(output);
        self.finalize_tools_into(output);

        let status = response_status(self.finish_reason.as_deref());
        let mut response = self.base_response(status, self.completed_output_items());
        if status == "incomplete" {
            response["incomplete_details"] = json!({ "reason": "max_output_tokens" });
        }
        copy_response_request_fields(&mut response, self.original_request.as_ref());
        push_sse(
            output,
            "response.completed",
            json!({
                "type": "response.completed",
                "response": response
            }),
        );
        output.push_str("data: [DONE]\n\n");
        self.completed = true;
    }

    fn finalize_reasoning_into(&mut self, output: &mut String) {
        if !self.reasoning.added || self.reasoning.done {
            return;
        }
        let output_index = self.reasoning.output_index.unwrap_or(0);
        let item = json!({
            "id": self.reasoning.item_id,
            "type": "reasoning",
            "reasoning_content": self.reasoning.text,
            "summary": [{ "type": "summary_text", "text": self.reasoning.text }]
        });
        self.output_items.push((output_index, item.clone()));
        self.reasoning.done = true;
        push_sse(
            output,
            "response.reasoning_summary_text.done",
            json!({
                "type": "response.reasoning_summary_text.done",
                "item_id": self.reasoning.item_id,
                "output_index": output_index,
                "summary_index": 0,
                "text": self.reasoning.text
            }),
        );
        push_sse(
            output,
            "response.reasoning_summary_part.done",
            json!({
                "type": "response.reasoning_summary_part.done",
                "item_id": self.reasoning.item_id,
                "output_index": output_index,
                "summary_index": 0,
                "part": { "type": "summary_text", "text": self.reasoning.text }
            }),
        );
        push_sse(
            output,
            "response.output_item.done",
            json!({
                "type": "response.output_item.done",
                "output_index": output_index,
                "item": item
            }),
        );
    }

    fn finalize_text_into(&mut self, output: &mut String) {
        if !self.text.added || self.text.done {
            return;
        }
        let output_index = self.text.output_index.unwrap_or(0);
        let item = json!({
            "id": self.text.item_id,
            "type": "message",
            "status": "completed",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": self.text.text, "annotations": [] }]
        });
        self.output_items.push((output_index, item.clone()));
        self.text.done = true;
        push_sse(
            output,
            "response.output_text.done",
            json!({
                "type": "response.output_text.done",
                "item_id": self.text.item_id,
                "output_index": output_index,
                "content_index": 0,
                "text": self.text.text
            }),
        );
        push_sse(
            output,
            "response.content_part.done",
            json!({
                "type": "response.content_part.done",
                "item_id": self.text.item_id,
                "output_index": output_index,
                "content_index": 0,
                "part": { "type": "output_text", "text": self.text.text, "annotations": [] }
            }),
        );
        push_sse(
            output,
            "response.output_item.done",
            json!({
                "type": "response.output_item.done",
                "output_index": output_index,
                "item": item
            }),
        );
    }

    fn finalize_tools_into(&mut self, output: &mut String) {
        let keys: Vec<usize> = self.tools.keys().copied().collect();
        for key in keys {
            if self.tools.get(&key).map(|state| state.done).unwrap_or(true) {
                continue;
            }
            if self
                .tools
                .get(&key)
                .map(|state| !state.added && !state.done)
                .unwrap_or(false)
            {
                let assigned = self.next_output_index();
                let state = self.tools.get_mut(&key).expect("tool state exists");
                state.added = true;
                if state.call_id.is_empty() {
                    state.call_id = format!("call_{key}");
                }
                if state.name.is_empty() {
                    state.name = "unknown_tool".to_string();
                }
                state.output_index = Some(assigned);
                state.item_id = format!("fc_{}", state.call_id);
                let added_item = tool_call_added_item(state, assigned, &self.tool_context);
                push_sse(output, "response.output_item.added", added_item);
            }

            let state = self.tools.get_mut(&key).expect("tool state exists");
            let output_index = state.output_index.unwrap_or(0);
            let item = tool_call_done_item(state, &self.tool_context);
            state.done = true;
            self.output_items.push((output_index, item.clone()));
            push_tool_call_done_sse(output, state, output_index, &self.tool_context);
            push_sse(
                output,
                "response.output_item.done",
                json!({
                    "type": "response.output_item.done",
                    "output_index": output_index,
                    "item": item
                }),
            );
        }
    }

    fn failed_into(&mut self, output: &mut String, message: String, error_type: Option<String>) {
        self.completed = true;
        let mut error = json!({ "message": message });
        if let Some(error_type) = error_type.filter(|value| !value.is_empty()) {
            error["type"] = json!(error_type);
        }
        let mut response = self.base_response("failed", self.completed_output_items());
        response["error"] = error;
        push_sse(
            output,
            "response.failed",
            json!({
                "type": "response.failed",
                "response": response
            }),
        );
    }

    fn completed_output_items(&self) -> Vec<Value> {
        let mut output_items = self.output_items.clone();
        output_items.sort_by_key(|(output_index, _)| *output_index);
        output_items.into_iter().map(|(_, item)| item).collect()
    }

    fn base_response(&self, status: &str, output: Vec<Value>) -> Value {
        json!({
            "id": self.response_id,
            "object": "response",
            "created_at": self.created_at,
            "status": status,
            "model": self.model,
            "output": output,
            "usage": self.latest_usage.clone().unwrap_or_else(default_responses_usage)
        })
    }

    fn next_output_index(&mut self) -> u32 {
        let index = self.next_output_index;
        self.next_output_index += 1;
        index
    }
}

fn take_sse_block(buffer: &mut String) -> Option<String> {
    let lf = buffer.find("\n\n").map(|index| (index, 2));
    let crlf = buffer.find("\r\n\r\n").map(|index| (index, 4));
    let (index, delimiter_len) = match (lf, crlf) {
        (Some(left), Some(right)) => {
            if left.0 <= right.0 {
                left
            } else {
                right
            }
        }
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => return None,
    };
    let block = buffer[..index].to_string();
    buffer.drain(..index + delimiter_len);
    Some(block)
}

fn append_utf8_safe(buffer: &mut String, remainder: &mut Vec<u8>, bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    let mut combined = Vec::new();
    if !remainder.is_empty() {
        combined.extend_from_slice(remainder);
        remainder.clear();
    }
    combined.extend_from_slice(bytes);

    match std::str::from_utf8(&combined) {
        Ok(text) => buffer.push_str(text),
        Err(error) => {
            let valid = error.valid_up_to();
            if valid > 0 {
                buffer.push_str(std::str::from_utf8(&combined[..valid]).unwrap_or_default());
            }
            if error.error_len().is_none() {
                remainder.extend_from_slice(&combined[valid..]);
            } else {
                buffer.push_str(&String::from_utf8_lossy(&combined[valid..]));
            }
        }
    }
}

fn strip_sse_field<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(field)?.strip_prefix(':')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

fn chat_delta_reasoning_text(delta: &Value) -> Option<String> {
    extract_reasoning_field_text(delta)
}

enum ThinkPrefixDecision {
    NeedMore,
    Reasoning,
    Text,
}

fn leading_think_prefix_decision(buffer: &str) -> ThinkPrefixDecision {
    let trimmed = buffer.trim_start();
    if trimmed.is_empty() {
        return ThinkPrefixDecision::NeedMore;
    }
    if trimmed.starts_with(THINK_OPEN_TAG) {
        return ThinkPrefixDecision::Reasoning;
    }
    if THINK_OPEN_TAG.starts_with(trimmed) {
        return ThinkPrefixDecision::NeedMore;
    }
    ThinkPrefixDecision::Text
}

fn extract_chat_sse_error(value: &Value) -> (String, Option<String>) {
    let error = value.get("error").unwrap_or(value);
    let message = error
        .as_str()
        .map(ToString::to_string)
        .or_else(|| {
            error
                .get("message")
                .or_else(|| error.get("detail"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| error.to_string());
    let error_type = error
        .get("type")
        .or_else(|| error.get("code"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    (message, error_type)
}

fn http_status_line(status: u16) -> String {
    match status {
        200 => "200 OK".to_string(),
        400 => "400 Bad Request".to_string(),
        401 => "401 Unauthorized".to_string(),
        403 => "403 Forbidden".to_string(),
        404 => "404 Not Found".to_string(),
        429 => "429 Too Many Requests".to_string(),
        500 => "500 Internal Server Error".to_string(),
        502 => "502 Bad Gateway".to_string(),
        503 => "503 Service Unavailable".to_string(),
        _ => format!("{status} Upstream"),
    }
}

pub fn responses_error_from_upstream(status_code: u16, content_type: &str, body: &[u8]) -> Value {
    let (message, error_type, code, param) = upstream_error_parts(status_code, content_type, body);
    let mut error = json!({
        "message": message,
        "type": error_type.unwrap_or_else(|| "upstream_error".to_string()),
    });
    if let Some(code) = code {
        error["code"] = json!(code);
    }
    if let Some(param) = param {
        error["param"] = json!(param);
    }
    json!({ "error": error })
}

fn upstream_error_parts(
    status_code: u16,
    content_type: &str,
    body: &[u8],
) -> (String, Option<String>, Option<String>, Option<String>) {
    if content_type.to_ascii_lowercase().contains("json") {
        if let Ok(value) = serde_json::from_slice::<Value>(body) {
            let error = value.get("error").unwrap_or(&value);
            let message = error
                .get("message")
                .or_else(|| error.get("detail"))
                .or_else(|| error.get("error"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .unwrap_or_else(|| truncate_error_preview(&value.to_string()));
            let error_type = error
                .get("type")
                .or_else(|| error.get("error_type"))
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let code = error.get("code").and_then(|value| {
                value
                    .as_str()
                    .map(ToString::to_string)
                    .or_else(|| value.as_i64().map(|number| number.to_string()))
            });
            let param = error
                .get("param")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            return (message, error_type, code, param);
        }
    }

    let preview = truncate_error_preview(&String::from_utf8_lossy(body));
    let message = if preview.trim().is_empty() {
        format!("Upstream returned HTTP {status_code}")
    } else {
        preview
    };
    (message, None, Some(status_code.to_string()), None)
}

fn truncate_error_preview(input: &str) -> String {
    input.chars().take(ERROR_BODY_PREVIEW_LIMIT).collect()
}

fn append_responses_input(input: &Value, messages: &mut Vec<Value>) {
    match input {
        Value::String(text) => messages.push(json!({ "role": "user", "content": text })),
        Value::Array(items) => {
            let mut pending_tool_calls = Vec::new();
            let mut pending_reasoning = Vec::new();
            let mut seen_tool_call_ids = BTreeSet::new();
            for item in items {
                append_responses_item(
                    item,
                    messages,
                    &mut pending_tool_calls,
                    &mut pending_reasoning,
                    &mut seen_tool_call_ids,
                );
            }
            flush_tool_calls(messages, &mut pending_tool_calls, &mut pending_reasoning);
            flush_reasoning(messages, &mut pending_reasoning);
        }
        Value::Object(_) => {
            let mut pending_tool_calls = Vec::new();
            let mut pending_reasoning = Vec::new();
            let mut seen_tool_call_ids = BTreeSet::new();
            append_responses_item(
                input,
                messages,
                &mut pending_tool_calls,
                &mut pending_reasoning,
                &mut seen_tool_call_ids,
            );
            flush_tool_calls(messages, &mut pending_tool_calls, &mut pending_reasoning);
            flush_reasoning(messages, &mut pending_reasoning);
        }
        _ => {}
    }
}

fn append_responses_item(
    item: &Value,
    messages: &mut Vec<Value>,
    pending_tool_calls: &mut Vec<Value>,
    pending_reasoning: &mut Vec<String>,
    seen_tool_call_ids: &mut BTreeSet<String>,
) {
    match item.get("type").and_then(Value::as_str) {
        Some("function_call") => {
            let name = responses_history_function_name(item);
            if name.is_empty() {
                return;
            }
            let call_id = item
                .get("call_id")
                .or_else(|| item.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if call_id.is_empty() {
                return;
            }
            seen_tool_call_ids.insert(call_id.to_string());
            pending_tool_calls.push(json!({
                "id": call_id,
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": responses_arguments_to_chat(item.get("arguments").unwrap_or(&json!({})))
                }
            }));
        }
        Some("function_call_output") => {
            let call_id = item.get("call_id").and_then(Value::as_str).unwrap_or("");
            if call_id.is_empty() {
                return;
            }
            if !seen_tool_call_ids.contains(call_id) {
                flush_tool_calls(messages, pending_tool_calls, pending_reasoning);
                flush_reasoning(messages, pending_reasoning);
                messages.push(orphan_tool_output_message(
                    call_id,
                    item.get("output").unwrap_or(&Value::Null),
                ));
                return;
            }
            flush_tool_calls(messages, pending_tool_calls, pending_reasoning);
            messages.push(json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": response_output_text(item.get("output").unwrap_or(&Value::Null))
            }));
        }
        Some("custom_tool_call") => {
            let name = item.get("name").and_then(Value::as_str).unwrap_or("");
            let input = item
                .get("input")
                .or_else(|| item.get("arguments"))
                .unwrap_or(&Value::Null);
            let (name, arguments) = build_custom_tool_call_history(name, input);
            let call_id = item
                .get("call_id")
                .or_else(|| item.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if call_id.is_empty() {
                return;
            }
            seen_tool_call_ids.insert(call_id.to_string());
            pending_tool_calls.push(json!({
                "id": call_id,
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": arguments
                }
            }));
        }
        Some("custom_tool_call_output") => {
            let call_id = item.get("call_id").and_then(Value::as_str).unwrap_or("");
            if call_id.is_empty() {
                return;
            }
            if !seen_tool_call_ids.contains(call_id) {
                flush_tool_calls(messages, pending_tool_calls, pending_reasoning);
                flush_reasoning(messages, pending_reasoning);
                messages.push(orphan_tool_output_message(
                    call_id,
                    item.get("output").unwrap_or(&Value::Null),
                ));
                return;
            }
            flush_tool_calls(messages, pending_tool_calls, pending_reasoning);
            messages.push(json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": response_output_text(item.get("output").unwrap_or(&Value::Null))
            }));
        }
        Some("tool_call") => {
            if let Some(tool_use) = item.get("tool_use") {
                let call_id = tool_use
                    .get("id")
                    .or_else(|| item.get("call_id"))
                    .or_else(|| item.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if call_id.is_empty() {
                    return;
                }
                seen_tool_call_ids.insert(call_id.to_string());
                pending_tool_calls.push(json!({
                    "id": call_id,
                    "type": "function",
                    "function": {
                        "name": tool_use.get("name").and_then(Value::as_str).unwrap_or(""),
                        "arguments": responses_arguments_to_chat(tool_use.get("input").unwrap_or(&json!({})))
                    }
                }));
            }
        }
        Some("tool_result") => {
            flush_tool_calls(messages, pending_tool_calls, pending_reasoning);
            let content = item.get("content").unwrap_or(&Value::Null);
            let call_id = content
                .get("tool_use_id")
                .or_else(|| item.get("tool_call_id"))
                .or_else(|| item.get("call_id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if call_id.is_empty() {
                return;
            }
            let output = content.get("content").unwrap_or(content);
            if !seen_tool_call_ids.contains(call_id) {
                flush_reasoning(messages, pending_reasoning);
                messages.push(orphan_tool_output_message(call_id, output));
                return;
            }
            messages.push(json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": response_output_text(output)
            }));
        }
        Some("reasoning") => {
            if let Some(text) = responses_reasoning_text(item) {
                if !text.is_empty() {
                    pending_reasoning.push(text);
                }
            }
        }
        _ => {
            flush_tool_calls(messages, pending_tool_calls, pending_reasoning);
            if item.get("role").is_some() || item.get("content").is_some() {
                let role = responses_role_to_chat_role(item.get("role").and_then(Value::as_str));
                let mut message = json!({
                    "role": role,
                    "content": responses_content_to_chat_content(
                        role,
                        item.get("content").unwrap_or(&Value::Null)
                        )
                });
                if role == "assistant" {
                    if !pending_reasoning.is_empty() && pending_tool_calls.is_empty() {
                        message["reasoning_content"] =
                            json!(std::mem::take(pending_reasoning).join("\n"));
                    }
                } else if !pending_reasoning.is_empty() {
                    flush_tool_calls(messages, pending_tool_calls, pending_reasoning);
                    flush_reasoning(messages, pending_reasoning);
                }
                messages.push(message);
            }
        }
    }
}

fn orphan_tool_output_message(call_id: &str, output: &Value) -> Value {
    json!({
        "role": "user",
        "content": format!(
            "Function call output ({call_id}): {}",
            response_output_text(output)
        )
    })
}

fn normalize_chat_messages(messages: &mut [Value]) {
    for message in messages {
        if message.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let has_content = match message.get("content") {
            Some(Value::Null) | None => false,
            Some(Value::String(_)) => true,
            Some(Value::Array(parts)) => !parts.is_empty(),
            Some(_) => true,
        };
        let has_tool_calls = message
            .get("tool_calls")
            .and_then(Value::as_array)
            .is_some_and(|tool_calls| !tool_calls.is_empty());
        if !has_content && !has_tool_calls {
            message["content"] = json!("");
        }
    }
}

fn collapse_system_messages_to_head(messages: Vec<Value>) -> Vec<Value> {
    let mut system_chunks = Vec::new();
    let mut rest = Vec::with_capacity(messages.len());

    for message in messages {
        if message.get("role").and_then(Value::as_str) == Some("system") {
            if let Some(text) = message.get("content").and_then(Value::as_str) {
                if !text.trim().is_empty() {
                    system_chunks.push(text.to_string());
                }
                continue;
            }
        }
        rest.push(message);
    }

    let mut output = Vec::with_capacity(rest.len() + usize::from(!system_chunks.is_empty()));
    if !system_chunks.is_empty() {
        output.push(json!({
            "role": "system",
            "content": system_chunks.join("\n\n")
        }));
    }
    output.extend(rest);
    output
}

fn responses_role_to_chat_role(role: Option<&str>) -> &'static str {
    match role {
        Some("developer") | Some("system") => "system",
        Some("assistant") => "assistant",
        Some("tool") => "tool",
        Some("latest_reminder") => "user",
        Some("user") | None => "user",
        Some(_) => "user",
    }
}

fn flush_tool_calls(
    messages: &mut Vec<Value>,
    pending_tool_calls: &mut Vec<Value>,
    pending_reasoning: &mut Vec<String>,
) {
    if pending_tool_calls.is_empty() {
        return;
    }

    if let Some(last) = messages.last_mut() {
        if last.get("role").and_then(Value::as_str) == Some("assistant") {
            merge_tool_calls_into_message(last, std::mem::take(pending_tool_calls));
            return;
        }
    }

    let mut message = json!({
        "role": "assistant",
        "content": "",
        "tool_calls": std::mem::take(pending_tool_calls)
    });
    if !pending_reasoning.is_empty() {
        message["reasoning_content"] = json!(std::mem::take(pending_reasoning).join("\n"));
    }
    messages.push(message);
}

fn flush_reasoning(messages: &mut Vec<Value>, pending_reasoning: &mut Vec<String>) {
    if pending_reasoning.is_empty() {
        return;
    }
    let reasoning = std::mem::take(pending_reasoning).join("\n");
    if let Some(last) = messages.last_mut() {
        if last.get("role").and_then(Value::as_str) == Some("assistant") {
            append_reasoning_to_assistant_message(last, &reasoning);
            return;
        }
    }
    messages.push(json!({
        "role": "assistant",
        "content": "",
        "reasoning_content": reasoning
    }));
}

fn append_reasoning_to_assistant_message(message: &mut Value, reasoning: &str) {
    if reasoning.is_empty() {
        return;
    }
    let existing = message
        .get("reasoning_content")
        .and_then(Value::as_str)
        .unwrap_or("");
    message["reasoning_content"] = if existing.is_empty() {
        json!(reasoning)
    } else {
        json!(format!("{existing}\n{reasoning}"))
    };
    if message.get("content").is_none() || message.get("content") == Some(&Value::Null) {
        message["content"] = json!("");
    }
}

fn merge_tool_calls_into_message(message: &mut Value, incoming: Vec<Value>) {
    let Some(object) = message.as_object_mut() else {
        return;
    };
    let existing = object
        .entry("tool_calls".to_string())
        .or_insert_with(|| json!([]));
    let Some(existing_array) = existing.as_array_mut() else {
        *existing = json!(incoming);
        return;
    };
    for tool_call in incoming {
        let id = tool_call.get("id").and_then(Value::as_str).unwrap_or("");
        if !id.is_empty()
            && existing_array
                .iter()
                .any(|item| item.get("id").and_then(Value::as_str) == Some(id))
        {
            continue;
        }
        existing_array.push(tool_call);
    }
    if message.get("content").is_none() || message.get("content") == Some(&Value::Null) {
        message["content"] = json!("");
    }
}

fn responses_reasoning_text(item: &Value) -> Option<String> {
    extract_reasoning_summary_text(item).or_else(|| extract_reasoning_field_text(item))
}

fn responses_content_to_chat_content(_role: &str, content: &Value) -> Value {
    if content.is_null() || content.is_string() {
        return content.clone();
    }

    let Some(parts) = content.as_array() else {
        return content.clone();
    };
    let mut chat_parts = Vec::new();
    let mut has_non_text_part = false;

    for part in parts {
        match part.get("type").and_then(Value::as_str).unwrap_or("") {
            "input_text" | "output_text" | "text" => {
                if let Some(value) = part.get("text").and_then(Value::as_str) {
                    if !value.is_empty() {
                        chat_parts.push(json!({ "type": "text", "text": value }));
                    }
                }
            }
            "refusal" => {
                if let Some(value) = part.get("refusal").and_then(Value::as_str) {
                    if !value.is_empty() {
                        chat_parts.push(json!({ "type": "text", "text": value }));
                    }
                }
            }
            "input_image" => {
                if let Some(image_url) = part.get("image_url") {
                    let image_url = if image_url.is_object() {
                        image_url.clone()
                    } else {
                        json!({ "url": image_url.as_str().unwrap_or_default() })
                    };
                    chat_parts.push(json!({ "type": "image_url", "image_url": image_url }));
                    has_non_text_part = true;
                }
            }
            _ => {}
        }
    }

    if !has_non_text_part {
        return Value::String(
            chat_parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }

    Value::Array(chat_parts)
}

fn responses_history_function_name(item: &Value) -> String {
    let name = item.get("name").and_then(Value::as_str).unwrap_or("");
    let namespace = item.get("namespace").and_then(Value::as_str).unwrap_or("");
    if name.is_empty() {
        String::new()
    } else if namespace.is_empty() {
        name.to_string()
    } else {
        flatten_namespace_tool_name(namespace, name)
    }
}

fn build_codex_tool_context(tools: Option<&Value>) -> CodexToolContext {
    let mut context = CodexToolContext::default();
    let Some(tools) = tools.and_then(Value::as_array) else {
        return context;
    };

    for tool in tools {
        if let Some(name) = tool.as_str().filter(|name| !name.is_empty()) {
            if let Some(action) = proxy_action_from_upstream_name(name) {
                context.custom_tools.insert(
                    name.to_string(),
                    CodexCustomToolSpec {
                        openai_name: "apply_patch".to_string(),
                        kind: CodexCustomToolKind::ApplyPatch,
                        proxy_action: Some(action),
                    },
                );
                context.has_custom_tools = true;
                continue;
            }
            context.custom_tools.insert(
                name.to_string(),
                CodexCustomToolSpec {
                    openai_name: name.to_string(),
                    kind: CodexCustomToolKind::Raw,
                    proxy_action: None,
                },
            );
            context.has_custom_tools = true;
            continue;
        }
        let tool_type = tool.get("type").and_then(Value::as_str).unwrap_or("");
        match tool_type {
            "custom" => {
                let Some(name) = tool
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|v| !v.is_empty())
                else {
                    continue;
                };
                let kind = detect_codex_custom_tool_kind(tool, name);
                context.custom_tools.insert(
                    name.to_string(),
                    CodexCustomToolSpec {
                        openai_name: name.to_string(),
                        kind,
                        proxy_action: None,
                    },
                );
                if kind == CodexCustomToolKind::ApplyPatch {
                    for action in [
                        CodexPatchProxyAction::AddFile,
                        CodexPatchProxyAction::DeleteFile,
                        CodexPatchProxyAction::UpdateFile,
                        CodexPatchProxyAction::ReplaceFile,
                        CodexPatchProxyAction::Batch,
                    ] {
                        let proxy_name = format!("{name}_{}", action.suffix());
                        context.custom_tools.insert(
                            proxy_name,
                            CodexCustomToolSpec {
                                openai_name: name.to_string(),
                                kind: CodexCustomToolKind::ApplyPatch,
                                proxy_action: Some(action),
                            },
                        );
                    }
                }
                context.has_custom_tools = true;
            }
            "function" => {
                if let Some(name) = tool
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|v| !v.is_empty())
                {
                    context.function_tools.insert(
                        name.to_string(),
                        CodexFunctionToolSpec {
                            name: name.to_string(),
                            namespace: String::new(),
                        },
                    );
                }
            }
            "namespace" => add_namespace_tools_to_context(&mut context, tool),
            "web_search" | "local_shell" | "computer_use" => {
                let name = tool
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|v| !v.is_empty())
                    .unwrap_or(tool_type);
                context.custom_tools.insert(
                    name.to_string(),
                    CodexCustomToolSpec {
                        openai_name: name.to_string(),
                        kind: CodexCustomToolKind::BuiltIn,
                        proxy_action: None,
                    },
                );
                context.has_custom_tools = true;
            }
            _ => {}
        }
    }

    context
}

fn add_namespace_tools_to_context(context: &mut CodexToolContext, namespace_tool: &Value) {
    let namespace = namespace_tool
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let Some(children) = namespace_tool.get("tools").and_then(Value::as_array) else {
        return;
    };
    for child in children {
        if child.get("type").and_then(Value::as_str) != Some("function") {
            continue;
        }
        let Some(name) = child
            .get("name")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        else {
            continue;
        };
        let flat = flatten_namespace_tool_name(namespace, name);
        if namespace.is_empty() {
            context.function_tools.insert(
                flat,
                CodexFunctionToolSpec {
                    namespace: namespace.to_string(),
                    name: name.to_string(),
                },
            );
        } else if context
            .function_tools
            .get(&flat)
            .is_none_or(|spec| !spec.namespace.is_empty())
        {
            context.function_tools.insert(
                flat,
                CodexFunctionToolSpec {
                    namespace: namespace.to_string(),
                    name: name.to_string(),
                },
            );
            context.has_namespace_tools = true;
        }
    }
}

fn responses_tools_to_chat_tools(tools: &[Value], context: &CodexToolContext) -> Vec<Value> {
    let mut converted = Vec::new();
    for tool in tools {
        if let Some(name) = tool.as_str().filter(|name| !name.is_empty()) {
            converted.push(generic_custom_proxy_tool(name, ""));
            continue;
        }
        match tool.get("type").and_then(Value::as_str).unwrap_or("") {
            "function" => {
                if let Some(tool) = responses_function_tool_to_chat_tool(tool) {
                    converted.push(tool);
                }
            }
            "custom" | "web_search" | "local_shell" | "computer_use" => {
                let tool_type = tool.get("type").and_then(Value::as_str).unwrap_or("");
                let name = tool
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|v| !v.is_empty())
                    .unwrap_or(tool_type);
                let description = tool
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if detect_codex_custom_tool_kind(tool, name) == CodexCustomToolKind::ApplyPatch {
                    converted.extend(apply_patch_proxy_tools(name, description));
                } else {
                    converted.push(generic_custom_proxy_tool(name, description));
                }
            }
            "namespace" => converted.extend(namespace_tool_to_chat_tools(tool, context)),
            _ => {}
        }
    }
    converted
}

fn detect_codex_custom_tool_kind(tool: &Value, name: &str) -> CodexCustomToolKind {
    if name == "apply_patch" {
        return CodexCustomToolKind::ApplyPatch;
    }
    if let Some(definition) = tool.pointer("/format/definition").and_then(Value::as_str) {
        if definition.contains("begin_patch")
            && definition.contains("end_patch")
            && definition.contains("add_hunk")
        {
            return CodexCustomToolKind::ApplyPatch;
        }
    }
    if matches!(
        tool.get("type").and_then(Value::as_str),
        Some("web_search" | "local_shell" | "computer_use")
    ) {
        CodexCustomToolKind::BuiltIn
    } else {
        CodexCustomToolKind::Raw
    }
}

fn responses_function_tool_to_chat_tool(tool: &Value) -> Option<Value> {
    if tool.get("type").and_then(Value::as_str) != Some("function") {
        return None;
    }
    if tool.get("function").is_some() {
        let mut chat_tool = tool.clone();
        if let Some(strict) = tool.get("strict").cloned() {
            if let Some(function) = chat_tool.get_mut("function").and_then(Value::as_object_mut) {
                function.entry("strict".to_string()).or_insert(strict);
            }
            if let Some(object) = chat_tool.as_object_mut() {
                object.remove("strict");
            }
        }
        if let Some(function) = chat_tool.get_mut("function").and_then(Value::as_object_mut) {
            let normalized =
                normalize_chat_tool_parameters(function.get("parameters").unwrap_or(&json!({})));
            function.insert("parameters".to_string(), normalized);
        }
        return Some(chat_tool);
    }
    let mut function = json!({
        "name": tool.get("name").and_then(Value::as_str).unwrap_or(""),
        "description": tool.get("description").cloned().unwrap_or(Value::Null),
        "parameters": normalize_chat_tool_parameters(tool.get("parameters").unwrap_or(&json!({})))
    });
    if let Some(strict) = tool.get("strict") {
        function["strict"] = strict.clone();
    }
    Some(json!({
        "type": "function",
        "function": function
    }))
}

fn namespace_tool_to_chat_tools(namespace_tool: &Value, context: &CodexToolContext) -> Vec<Value> {
    let namespace = namespace_tool
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let namespace_description = namespace_tool
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let Some(children) = namespace_tool.get("tools").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut converted = Vec::new();
    for child in children {
        if child.get("type").and_then(Value::as_str) != Some("function") {
            continue;
        }
        let Some(name) = child
            .get("name")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        else {
            continue;
        };
        let flat = flatten_namespace_tool_name(namespace, name);
        if namespace != ""
            && context
                .function_tools
                .get(&flat)
                .is_some_and(|spec| spec.namespace.is_empty())
        {
            continue;
        }
        let description = combine_namespace_description(
            namespace_description,
            child
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        let mut function = json!({
            "name": flat,
            "parameters": normalize_chat_tool_parameters(child.get("parameters").unwrap_or(&json!({})))
        });
        if !description.is_empty() {
            function["description"] = json!(description);
        }
        converted.push(json!({
            "type": "function",
            "function": function
        }));
    }
    converted
}

fn normalize_chat_tool_parameters(parameters: &Value) -> Value {
    let mut normalized = if parameters.is_object() {
        parameters.clone()
    } else {
        json!({})
    };
    if normalized.get("type").is_none() {
        normalized["type"] = json!("object");
    }
    if normalized.get("properties").is_none() {
        normalized["properties"] = json!({});
    }
    if normalized.get("required").is_none() {
        normalized["required"] = json!([]);
    }
    normalized
}

fn generic_custom_proxy_tool(name: &str, description: &str) -> Value {
    let description = if description.trim().is_empty() {
        format!("FREEFORM custom tool: {name}. Put only the tool input text here.")
    } else {
        format!(
            "{}\n\nThis is a FREEFORM tool. Do not wrap the input in JSON or markdown.",
            description.trim()
        )
    };
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Raw freeform input for this custom tool."
                    }
                },
                "required": ["input"]
            }
        }
    })
}

fn apply_patch_proxy_tools(name: &str, description: &str) -> Vec<Value> {
    vec![
        function_tool(
            &format!("{name}_add_file"),
            &patch_proxy_description(
                description,
                "add_file",
                "Create one new file by providing a target path and full file content.",
            ),
            apply_patch_add_file_schema(),
        ),
        function_tool(
            &format!("{name}_delete_file"),
            &patch_proxy_description(
                description,
                "delete_file",
                "Delete one file by providing a target path.",
            ),
            apply_patch_delete_file_schema(),
        ),
        function_tool(
            &format!("{name}_update_file"),
            &patch_proxy_description(
                description,
                "update_file",
                "Edit one existing file with structured hunks.",
            ),
            apply_patch_update_file_schema(),
        ),
        function_tool(
            &format!("{name}_replace_file"),
            &patch_proxy_description(
                description,
                "replace_file",
                "Replace one existing file by providing a target path and full new file content.",
            ),
            apply_patch_replace_file_schema(),
        ),
        function_tool(
            &format!("{name}_batch"),
            &patch_proxy_description(
                description,
                "batch",
                "Edit files by providing structured JSON patch operations.",
            ),
            apply_patch_batch_schema(),
        ),
    ]
}

fn function_tool(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": parameters
        }
    })
}

fn patch_proxy_description(description: &str, action: &str, default_description: &str) -> String {
    if description.trim().is_empty() {
        default_description.to_string()
    } else {
        format!("{} (proxy action: {action})", description.trim())
    }
}

fn apply_patch_add_file_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "path": { "type": "string", "description": "Target file path." },
            "content": { "type": "string", "description": "Full file content without patch '+' prefixes." }
        },
        "required": ["path", "content"]
    })
}

fn apply_patch_delete_file_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "path": { "type": "string", "description": "Target file path." }
        },
        "required": ["path"]
    })
}

fn apply_patch_update_file_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "path": { "type": "string", "description": "Target file path." },
            "move_to": { "type": "string", "description": "Optional destination path for move operations." },
            "hunks": apply_patch_hunks_schema()
        },
        "required": ["path", "hunks"]
    })
}

fn apply_patch_replace_file_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "path": { "type": "string", "description": "Target file path." },
            "content": { "type": "string", "description": "Full replacement content." }
        },
        "required": ["path", "content"]
    })
}

fn apply_patch_batch_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "operations": {
                "type": "array",
                "description": "Ordered list of file patch operations.",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "type": { "type": "string", "enum": ["add_file", "delete_file", "update_file", "replace_file"] },
                        "path": { "type": "string" },
                        "move_to": { "type": "string", "description": "Optional destination path for move operations (update_file only)." },
                        "content": { "type": "string", "description": "Full file content for add_file / replace_file." },
                        "hunks": apply_patch_hunks_schema()
                    },
                    "required": ["type", "path"]
                }
            }
        },
        "required": ["operations"]
    })
}

fn apply_patch_hunks_schema() -> Value {
    json!({
        "type": "array",
        "description": "Structured update hunks (required when type=update_file).",
        "items": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "context": { "type": "string", "description": "Optional @@ context header text." },
                "lines": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "op": { "type": "string", "enum": ["context", "add", "remove"] },
                            "text": { "type": "string" }
                        },
                        "required": ["op", "text"]
                    }
                }
            },
            "required": ["lines"]
        }
    })
}

fn proxy_action_from_upstream_name(name: &str) -> Option<CodexPatchProxyAction> {
    if name.ends_with("_add_file") {
        Some(CodexPatchProxyAction::AddFile)
    } else if name.ends_with("_delete_file") {
        Some(CodexPatchProxyAction::DeleteFile)
    } else if name.ends_with("_update_file") {
        Some(CodexPatchProxyAction::UpdateFile)
    } else if name.ends_with("_replace_file") {
        Some(CodexPatchProxyAction::ReplaceFile)
    } else if name.ends_with("_batch") {
        Some(CodexPatchProxyAction::Batch)
    } else {
        None
    }
}

fn combine_namespace_description(namespace_description: &str, child_description: &str) -> String {
    let namespace_description = namespace_description.trim();
    let child_description = child_description.trim();
    match (
        namespace_description.is_empty(),
        child_description.is_empty(),
    ) {
        (true, true) => String::new(),
        (true, false) => child_description.to_string(),
        (false, true) => namespace_description.to_string(),
        (false, false) => format!("{namespace_description}\n\n{child_description}"),
    }
}

fn flatten_namespace_tool_name(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        return name.to_string();
    }
    if name.is_empty() {
        return namespace.to_string();
    }
    if namespace.ends_with("__") || name.starts_with("__") {
        format!("{namespace}{name}")
    } else {
        format!("{namespace}__{name}")
    }
}

fn responses_tool_choice_to_chat(tool_choice: &Value, context: &CodexToolContext) -> Option<Value> {
    match tool_choice {
        Value::Object(object) if object.get("type").and_then(Value::as_str) == Some("function") => {
            if let Some(namespace) = object.get("namespace").and_then(Value::as_str) {
                let name = object.get("name").and_then(Value::as_str).unwrap_or("");
                return Some(json!({
                    "type": "function",
                    "function": {
                        "name": flatten_namespace_tool_name(namespace, name)
                    }
                }));
            }
            if let Some(function) = object.get("function").and_then(Value::as_object) {
                if let Some(namespace) = function.get("namespace").and_then(Value::as_str) {
                    let name = function.get("name").and_then(Value::as_str).unwrap_or("");
                    return Some(json!({
                        "type": "function",
                        "function": {
                            "name": flatten_namespace_tool_name(namespace, name)
                        }
                    }));
                }
            }
            Some(json!({
                "type": "function",
                "function": {
                    "name": object.get("name").and_then(Value::as_str).unwrap_or("")
                }
            }))
        }
        Value::Object(object) if object.get("type").and_then(Value::as_str) == Some("custom") => {
            let name = object.get("name").and_then(Value::as_str)?;
            let spec = context.custom_tools.get(name)?;
            let upstream_name = if spec.kind == CodexCustomToolKind::ApplyPatch {
                format!("{}_batch", spec.openai_name)
            } else {
                spec.openai_name.clone()
            };
            Some(json!({
                "type": "function",
                "function": { "name": upstream_name }
            }))
        }
        other => Some(other.clone()),
    }
}

fn chat_reasoning_to_response_output_item(message: &Value, response_id: &str) -> Option<Value> {
    let reasoning = chat_reasoning_text(message)?;
    if reasoning.is_empty() {
        return None;
    }
    Some(json!({
        "id": format!("rs_{response_id}"),
        "type": "reasoning",
        "reasoning_content": reasoning,
        "summary": [{ "type": "summary_text", "text": reasoning }]
    }))
}

fn chat_reasoning_text(message: &Value) -> Option<String> {
    if let Some(reasoning) = extract_reasoning_field_text(message) {
        return Some(reasoning);
    }

    if let Some(content) = message.get("content").and_then(Value::as_str) {
        if let Some((reasoning, _answer)) = split_leading_think_block(content) {
            if !reasoning.is_empty() {
                return Some(reasoning);
            }
        }
    }

    None
}

fn chat_message_to_response_output_item(message: &Value, response_id: &str) -> Option<Value> {
    let mut content = Vec::new();
    if let Some(text) = message.get("content").and_then(Value::as_str) {
        let text = split_leading_think_block(text)
            .map(|(_reasoning, answer)| answer)
            .unwrap_or_else(|| text.to_string());
        if !text.is_empty() {
            content.push(json!({ "type": "output_text", "text": text, "annotations": [] }));
        }
    } else if let Some(parts) = message.get("content").and_then(Value::as_array) {
        for part in parts {
            match part.get("type").and_then(Value::as_str).unwrap_or("") {
                "text" | "output_text" => {
                    if let Some(text) = part.get("text").and_then(Value::as_str) {
                        if !text.is_empty() {
                            content.push(
                                json!({ "type": "output_text", "text": text, "annotations": [] }),
                            );
                        }
                    }
                }
                "refusal" => {
                    if let Some(refusal) = part.get("refusal").and_then(Value::as_str) {
                        if !refusal.is_empty() {
                            content.push(json!({ "type": "refusal", "refusal": refusal }));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(refusal) = message.get("refusal").and_then(Value::as_str) {
        if !refusal.is_empty() {
            content.push(json!({ "type": "refusal", "refusal": refusal }));
        }
    }

    if content.is_empty() {
        return None;
    }

    Some(json!({
        "id": format!("{response_id}_msg"),
        "type": "message",
        "status": "completed",
        "role": "assistant",
        "content": content
    }))
}

fn chat_tool_calls_to_response_output_items(
    message: &Value,
    tool_context: &CodexToolContext,
) -> Vec<Value> {
    let mut output = Vec::new();
    if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
        for (index, tool_call) in tool_calls.iter().enumerate() {
            output.push(chat_tool_call_to_response_item(
                tool_call,
                index,
                tool_context,
            ));
        }
    } else if let Some(function_call) = message.get("function_call") {
        output.push(chat_legacy_function_call_to_response_item(
            function_call,
            tool_context,
        ));
    }
    output
}

fn chat_tool_call_to_response_item(
    tool_call: &Value,
    index: usize,
    tool_context: &CodexToolContext,
) -> Value {
    let call_id = tool_call
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("call_{index}"));
    let function = tool_call.get("function").unwrap_or(&Value::Null);
    let name = function.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = responses_arguments_to_chat(function.get("arguments").unwrap_or(&json!({})));
    response_tool_call_item(&call_id, name, &arguments, tool_context)
}

fn chat_legacy_function_call_to_response_item(
    function_call: &Value,
    tool_context: &CodexToolContext,
) -> Value {
    let call_id = function_call
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("call_0");
    let name = function_call
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let arguments =
        responses_arguments_to_chat(function_call.get("arguments").unwrap_or(&json!({})));
    response_tool_call_item(call_id, name, &arguments, tool_context)
}

fn tool_call_added_item(
    state: &ToolCallState,
    output_index: u32,
    tool_context: &CodexToolContext,
) -> Value {
    if tool_context.is_custom_tool_proxy(&state.name) {
        return json!({
            "type": "response.output_item.added",
            "output_index": output_index,
            "item": {
                "id": format!("ctc_{}", state.call_id),
                "type": "custom_tool_call",
                "status": "in_progress",
                "call_id": state.call_id,
                "name": tool_context.original_custom_tool_name(&state.name),
                "input": ""
            }
        });
    }
    let (display_name, namespace) = tool_context.openai_name_for_function_tool(&state.name);
    let mut item = json!({
        "type": "response.output_item.added",
        "output_index": output_index,
        "item": {
            "id": state.item_id,
            "type": "function_call",
            "status": "in_progress",
            "call_id": state.call_id,
            "name": display_name,
            "arguments": ""
        }
    });
    if !namespace.is_empty() {
        item["item"]["namespace"] = json!(namespace);
    }
    item
}

fn push_tool_call_delta_sse(
    output: &mut String,
    state: &ToolCallState,
    output_index: u32,
    delta: &str,
    tool_context: &CodexToolContext,
) {
    if tool_context.is_custom_tool_proxy(&state.name) {
        let _ = delta;
    } else {
        push_sse(
            output,
            "response.function_call_arguments.delta",
            json!({
                "type": "response.function_call_arguments.delta",
                "item_id": state.item_id,
                "output_index": output_index,
                "delta": delta
            }),
        );
    }
}

fn push_tool_call_done_sse(
    output: &mut String,
    state: &ToolCallState,
    output_index: u32,
    tool_context: &CodexToolContext,
) {
    if tool_context.is_custom_tool_proxy(&state.name) {
        push_sse(
            output,
            "response.custom_tool_call_input.delta",
            json!({
                "type": "response.custom_tool_call_input.delta",
                "item_id": format!("ctc_{}", state.call_id),
                "call_id": state.call_id,
                "output_index": output_index,
                "delta": reconstruct_custom_tool_call_input_with_context(
                    tool_context,
                    &state.name,
                    &state.arguments
                )
            }),
        );
        return;
    }
    push_sse(
        output,
        "response.function_call_arguments.done",
        json!({
            "type": "response.function_call_arguments.done",
            "item_id": state.item_id,
            "output_index": output_index,
            "arguments": state.arguments
        }),
    );
}

fn tool_call_done_item(state: &ToolCallState, tool_context: &CodexToolContext) -> Value {
    response_tool_call_item(&state.call_id, &state.name, &state.arguments, tool_context)
}

fn response_tool_call_item(
    call_id: &str,
    name: &str,
    arguments: &str,
    tool_context: &CodexToolContext,
) -> Value {
    if tool_context.is_custom_tool_proxy(name) {
        return json!({
            "id": format!("ctc_{call_id}"),
            "type": "custom_tool_call",
            "status": "completed",
            "call_id": call_id,
            "name": tool_context.original_custom_tool_name(name),
            "input": reconstruct_custom_tool_call_input_with_context(tool_context, name, arguments)
        });
    }
    let (display_name, namespace) = tool_context.openai_name_for_function_tool(name);
    let mut item = json!({
        "id": format!("fc_{call_id}"),
        "type": "function_call",
        "status": "completed",
        "call_id": call_id,
        "name": display_name,
        "arguments": arguments
    });
    if !namespace.is_empty() {
        item["namespace"] = json!(namespace);
    }
    item
}

fn split_leading_think_block(text: &str) -> Option<(String, String)> {
    let leading_ws_len = text.len() - text.trim_start().len();
    let after_ws = &text[leading_ws_len..];
    if !after_ws.starts_with(THINK_OPEN_TAG) {
        return None;
    }
    let body_start = leading_ws_len + THINK_OPEN_TAG.len();
    let close_relative = text[body_start..].find(THINK_CLOSE_TAG)?;
    let close_start = body_start + close_relative;
    let answer_start = close_start + THINK_CLOSE_TAG.len();
    Some((
        text[body_start..close_start].trim().to_string(),
        strip_think_answer_separator(&text[answer_start..]).to_string(),
    ))
}

fn strip_leading_think_open_tag(text: &str) -> Option<String> {
    let leading_ws_len = text.len() - text.trim_start().len();
    let after_ws = &text[leading_ws_len..];
    after_ws
        .strip_prefix(THINK_OPEN_TAG)
        .map(|value| value.trim().to_string())
}

fn strip_think_answer_separator(text: &str) -> &str {
    text.trim_start_matches(['\r', '\n', '\t', ' '])
}

fn extract_reasoning_field_text(value: &Value) -> Option<String> {
    for key in ["reasoning_content", "reasoning"] {
        if let Some(text) = value.get(key).and_then(Value::as_str) {
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }

    if let Some(reasoning) = value.get("reasoning") {
        for key in ["content", "text", "summary"] {
            if let Some(text) = reasoning.get(key).and_then(Value::as_str) {
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }
    }

    value
        .get("reasoning_details")
        .and_then(extract_reasoning_details_text)
}

fn extract_reasoning_details_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => (!text.is_empty()).then(|| text.to_string()),
        Value::Array(parts) => {
            let text = parts
                .iter()
                .filter_map(extract_reasoning_detail_part_text)
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            (!text.is_empty()).then_some(text)
        }
        Value::Object(_) => extract_reasoning_detail_part_text(value),
        _ => None,
    }
}

fn extract_reasoning_detail_part_text(value: &Value) -> Option<String> {
    for key in ["text", "content", "summary"] {
        if let Some(text) = value.get(key).and_then(Value::as_str) {
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }

    if let Some(parts) = value.get("parts").and_then(Value::as_array) {
        let text = parts
            .iter()
            .filter_map(extract_reasoning_detail_part_text)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        return (!text.is_empty()).then_some(text);
    }

    None
}

fn extract_reasoning_summary_text(value: &Value) -> Option<String> {
    for key in ["reasoning_content", "content", "text"] {
        if let Some(text) = value.get(key).and_then(Value::as_str) {
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }

    let summary = value.get("summary")?;
    if let Some(text) = summary.as_str() {
        return (!text.is_empty()).then(|| text.to_string());
    }

    let parts = summary.as_array()?;
    let text = parts
        .iter()
        .filter_map(|part| {
            part.get("text")
                .and_then(Value::as_str)
                .or_else(|| part.get("content").and_then(Value::as_str))
                .or_else(|| part.as_str())
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    (!text.is_empty()).then_some(text)
}

fn default_responses_usage() -> Value {
    json!({ "input_tokens": 0, "output_tokens": 0, "total_tokens": 0 })
}

fn chat_usage_to_responses_usage(usage: Option<&Value>) -> Value {
    let Some(usage) = usage.filter(|value| value.is_object() && !value.is_null()) else {
        return default_responses_usage();
    };
    let mut input_tokens = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .or_else(|| usage.get("promptTokenCount"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut input_tokens_include_cache = usage.get("prompt_tokens").is_some();
    let output_tokens = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .or_else(|| usage.get("candidatesTokenCount"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut cached_tokens = usage
        .pointer("/prompt_tokens_details/cached_tokens")
        .or_else(|| usage.pointer("/input_tokens_details/cached_tokens"))
        .or_else(|| usage.get("cachedContentTokenCount"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cache_creation = usage
        .get("cache_creation_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cache_creation_5m = usage
        .get("cache_creation_5m_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cache_creation_1h = usage
        .get("cache_creation_1h_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let has_claude_cache_fields = usage.get("cache_read_input_tokens").is_some()
        || usage.get("cache_creation_input_tokens").is_some()
        || usage.get("cache_creation_5m_input_tokens").is_some()
        || usage.get("cache_creation_1h_input_tokens").is_some();
    let has_cache_details = cached_tokens > 0
        || usage
            .pointer("/prompt_tokens_details/cached_tokens")
            .is_some()
        || usage
            .pointer("/input_tokens_details/cached_tokens")
            .is_some();

    if let Some(value) = usage.get("input_tokens").and_then(Value::as_u64) {
        input_tokens = value;
        input_tokens_include_cache = false;
    }
    if let Some(cache_read) = usage.get("cache_read_input_tokens").and_then(Value::as_u64) {
        cached_tokens = cache_read;
    }
    if let Some(prompt_tokens) = usage.get("promptTokenCount").and_then(Value::as_u64) {
        cached_tokens = usage
            .get("cachedContentTokenCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        input_tokens = prompt_tokens.saturating_sub(cached_tokens);
        input_tokens_include_cache = false;
    }

    let usage_input_tokens = if input_tokens_include_cache {
        input_tokens.saturating_sub(
            cached_tokens
                + effective_cache_creation_tokens(
                    cache_creation,
                    cache_creation_5m,
                    cache_creation_1h,
                ),
        )
    } else {
        input_tokens
    };
    let should_recalculate_total = usage.get("total_tokens").is_none()
        || cached_tokens > 0
        || effective_cache_creation_tokens(cache_creation, cache_creation_5m, cache_creation_1h)
            > 0
        || usage.get("promptTokenCount").is_some();
    let total_tokens = if should_recalculate_total {
        usage_input_tokens
            + output_tokens
            + cached_tokens
            + effective_cache_creation_tokens(cache_creation, cache_creation_5m, cache_creation_1h)
    } else {
        usage
            .get("total_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(usage_input_tokens + output_tokens)
    };
    let mut result = json!({
        "input_tokens": usage_input_tokens,
        "output_tokens": output_tokens,
        "total_tokens": total_tokens
    });

    if !has_claude_cache_fields && has_cache_details && cached_tokens > 0 {
        result["input_tokens_details"] = json!({ "cached_tokens": cached_tokens });
    }
    if let Some(details) = usage.get("completion_tokens_details") {
        result["output_tokens_details"] = details.clone();
    }
    if let Some(cache_read) = usage.get("cache_read_input_tokens") {
        result["cache_read_input_tokens"] = cache_read.clone();
    }
    if let Some(cache_creation) = usage.get("cache_creation_input_tokens") {
        result["cache_creation_input_tokens"] = cache_creation.clone();
    }
    if let Some(cache_creation) = usage.get("cache_creation_5m_input_tokens") {
        result["cache_creation_5m_input_tokens"] = cache_creation.clone();
    }
    if let Some(cache_creation) = usage.get("cache_creation_1h_input_tokens") {
        result["cache_creation_1h_input_tokens"] = cache_creation.clone();
    }
    let cache_ttl = match (cache_creation_5m > 0, cache_creation_1h > 0) {
        (true, true) => Some("mixed"),
        (true, false) => Some("5m"),
        (false, true) => Some("1h"),
        (false, false) => None,
    };
    if let Some(cache_ttl) = cache_ttl {
        result["cache_ttl"] = json!(cache_ttl);
    }
    result
}

fn effective_cache_creation_tokens(
    cache_creation: u64,
    cache_creation_5m: u64,
    cache_creation_1h: u64,
) -> u64 {
    if cache_creation > 0 {
        cache_creation
    } else {
        cache_creation_5m + cache_creation_1h
    }
}

fn response_status(finish_reason: Option<&str>) -> &'static str {
    match finish_reason {
        Some("length") => "incomplete",
        _ => "completed",
    }
}

fn response_output_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => canonical_json_string(other),
    }
}

fn build_custom_tool_call_history(name: &str, input: &Value) -> (String, String) {
    let input = response_output_text(input);
    if name == "apply_patch" || input.starts_with("*** Begin Patch") {
        let operations = parse_apply_patch_operations(&input);
        if operations.len() == 1 {
            let action = operations[0]
                .get("type")
                .and_then(Value::as_str)
                .and_then(single_apply_patch_action)
                .unwrap_or(CodexPatchProxyAction::Batch);
            return (
                format!("{name}_{}", action.suffix()),
                build_apply_patch_operation_arguments(&operations[0], action),
            );
        }
        return (
            format!("{name}_batch"),
            json!({ "operations": operations, "raw_patch": input }).to_string(),
        );
    }
    (name.to_string(), json!({ "input": input }).to_string())
}

fn reconstruct_custom_tool_call_input_with_context(
    tool_context: &CodexToolContext,
    upstream_name: &str,
    arguments: &str,
) -> String {
    if let Some(spec) = tool_context.custom_tools.get(upstream_name) {
        if spec.kind == CodexCustomToolKind::ApplyPatch {
            return reconstruct_apply_patch_input(spec.proxy_action, arguments);
        }
    }
    reconstruct_custom_tool_call_input(arguments)
}

fn reconstruct_custom_tool_call_input(arguments: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(arguments) else {
        return arguments.to_string();
    };
    value
        .get("input")
        .map(response_output_text)
        .unwrap_or_else(|| arguments.to_string())
}

fn reconstruct_apply_patch_input(action: Option<CodexPatchProxyAction>, arguments: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(arguments) else {
        return arguments.to_string();
    };
    if let Some(raw_patch) = value
        .get("raw_patch")
        .or_else(|| value.get("patch"))
        .or_else(|| value.get("input"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        return raw_patch.to_string();
    }

    let operations = match action.unwrap_or(CodexPatchProxyAction::Batch) {
        CodexPatchProxyAction::AddFile => vec![json!({
            "type": "add_file",
            "path": value.get("path").and_then(Value::as_str).unwrap_or(""),
            "content": value.get("content").and_then(Value::as_str).unwrap_or("")
        })],
        CodexPatchProxyAction::DeleteFile => vec![json!({
            "type": "delete_file",
            "path": value.get("path").and_then(Value::as_str).unwrap_or("")
        })],
        CodexPatchProxyAction::UpdateFile => vec![json!({
            "type": "update_file",
            "path": value.get("path").and_then(Value::as_str).unwrap_or(""),
            "move_to": value.get("move_to").and_then(Value::as_str).unwrap_or(""),
            "hunks": value.get("hunks").cloned().unwrap_or_else(|| json!([]))
        })],
        CodexPatchProxyAction::ReplaceFile => vec![json!({
            "type": "replace_file",
            "path": value.get("path").and_then(Value::as_str).unwrap_or(""),
            "content": value.get("content").and_then(Value::as_str).unwrap_or("")
        })],
        CodexPatchProxyAction::Batch => value
            .get("operations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    };

    build_apply_patch_text(&operations)
}

fn build_apply_patch_text(operations: &[Value]) -> String {
    let mut text = String::from("*** Begin Patch");
    for operation in operations {
        let op_type = operation.get("type").and_then(Value::as_str).unwrap_or("");
        let path = operation.get("path").and_then(Value::as_str).unwrap_or("");
        match op_type {
            "add_file" => {
                text.push_str(&format!("\n*** Add File: {path}"));
                for line in operation
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .lines()
                {
                    text.push_str("\n+");
                    text.push_str(line);
                }
            }
            "delete_file" => {
                text.push_str(&format!("\n*** Delete File: {path}"));
            }
            "update_file" => {
                text.push_str(&format!("\n*** Update File: {path}"));
                if let Some(move_to) = operation.get("move_to").and_then(Value::as_str) {
                    if !move_to.is_empty() {
                        text.push_str(&format!("\n*** Move to: {move_to}"));
                    }
                }
                if let Some(hunks) = operation.get("hunks").and_then(Value::as_array) {
                    for hunk in hunks {
                        let context = hunk.get("context").and_then(Value::as_str).unwrap_or("");
                        if context.is_empty() {
                            text.push_str("\n@@");
                        } else {
                            text.push_str(&format!("\n@@ {context}"));
                        }
                        if let Some(lines) = hunk.get("lines").and_then(Value::as_array) {
                            for line in lines {
                                text.push('\n');
                                text.push_str(line_op_prefix(
                                    line.get("op").and_then(Value::as_str).unwrap_or("context"),
                                ));
                                text.push_str(
                                    line.get("text").and_then(Value::as_str).unwrap_or(""),
                                );
                            }
                        }
                    }
                }
            }
            "replace_file" => {
                text.push_str(&format!("\n*** Delete File: {path}"));
                text.push_str(&format!("\n*** Add File: {path}"));
                for line in operation
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .lines()
                {
                    text.push_str("\n+");
                    text.push_str(line);
                }
            }
            _ => {}
        }
    }
    text.push_str("\n*** End Patch");
    text
}

fn line_op_prefix(op: &str) -> &'static str {
    match op {
        "add" => "+",
        "remove" | "delete" => "-",
        _ => " ",
    }
}

fn parse_apply_patch_operations(input: &str) -> Vec<Value> {
    let mut operations = Vec::new();
    let mut current: Option<serde_json::Map<String, Value>> = None;
    let mut content_lines: Vec<String> = Vec::new();
    let mut hunks: Vec<Value> = Vec::new();
    let mut current_hunk: Option<serde_json::Map<String, Value>> = None;
    let mut hunk_lines: Vec<Value> = Vec::new();

    let flush_hunk = |current_hunk: &mut Option<serde_json::Map<String, Value>>,
                      hunk_lines: &mut Vec<Value>,
                      hunks: &mut Vec<Value>| {
        if let Some(mut hunk) = current_hunk.take() {
            hunk.insert("lines".to_string(), json!(std::mem::take(hunk_lines)));
            hunks.push(Value::Object(hunk));
        }
    };
    let flush_operation = |current: &mut Option<serde_json::Map<String, Value>>,
                           content_lines: &mut Vec<String>,
                           hunks: &mut Vec<Value>,
                           operations: &mut Vec<Value>| {
        if let Some(mut operation) = current.take() {
            match operation.get("type").and_then(Value::as_str).unwrap_or("") {
                "add_file" | "replace_file" => {
                    operation.insert("content".to_string(), json!(content_lines.join("\n")));
                }
                "update_file" => {
                    operation.insert("hunks".to_string(), json!(std::mem::take(hunks)));
                }
                _ => {}
            }
            content_lines.clear();
            operations.push(Value::Object(operation));
        }
    };

    for raw_line in input.lines() {
        if raw_line == "*** Begin Patch" || raw_line == "*** End Patch" {
            continue;
        }
        if let Some(path) = raw_line.strip_prefix("*** Add File: ") {
            flush_hunk(&mut current_hunk, &mut hunk_lines, &mut hunks);
            flush_operation(
                &mut current,
                &mut content_lines,
                &mut hunks,
                &mut operations,
            );
            current = Some(serde_json::Map::from_iter([
                ("type".to_string(), json!("add_file")),
                ("path".to_string(), json!(path)),
            ]));
            continue;
        }
        if let Some(path) = raw_line.strip_prefix("*** Delete File: ") {
            flush_hunk(&mut current_hunk, &mut hunk_lines, &mut hunks);
            flush_operation(
                &mut current,
                &mut content_lines,
                &mut hunks,
                &mut operations,
            );
            current = Some(serde_json::Map::from_iter([
                ("type".to_string(), json!("delete_file")),
                ("path".to_string(), json!(path)),
            ]));
            continue;
        }
        if let Some(path) = raw_line.strip_prefix("*** Update File: ") {
            flush_hunk(&mut current_hunk, &mut hunk_lines, &mut hunks);
            flush_operation(
                &mut current,
                &mut content_lines,
                &mut hunks,
                &mut operations,
            );
            current = Some(serde_json::Map::from_iter([
                ("type".to_string(), json!("update_file")),
                ("path".to_string(), json!(path)),
            ]));
            continue;
        }
        if let Some(path) = raw_line.strip_prefix("*** Move to: ") {
            if let Some(operation) = current.as_mut() {
                operation.insert("move_to".to_string(), json!(path));
            }
            continue;
        }
        if raw_line.starts_with("@@") {
            flush_hunk(&mut current_hunk, &mut hunk_lines, &mut hunks);
            let context = raw_line.strip_prefix("@@").unwrap_or("").trim().to_string();
            current_hunk = Some(serde_json::Map::from_iter([(
                "context".to_string(),
                json!(context),
            )]));
            continue;
        }
        if let Some(operation) = current.as_ref() {
            match operation.get("type").and_then(Value::as_str).unwrap_or("") {
                "add_file" | "replace_file" => {
                    if let Some(line) = raw_line.strip_prefix('+') {
                        content_lines.push(line.to_string());
                    }
                }
                "update_file" => {
                    let (op, text) = match raw_line.chars().next() {
                        Some('+') => ("add", &raw_line[1..]),
                        Some('-') => ("remove", &raw_line[1..]),
                        Some(' ') => ("context", &raw_line[1..]),
                        _ => ("context", raw_line),
                    };
                    hunk_lines.push(json!({ "op": op, "text": text }));
                }
                _ => {}
            }
        }
    }

    flush_hunk(&mut current_hunk, &mut hunk_lines, &mut hunks);
    flush_operation(
        &mut current,
        &mut content_lines,
        &mut hunks,
        &mut operations,
    );
    operations
}

fn single_apply_patch_action(op_type: &str) -> Option<CodexPatchProxyAction> {
    match op_type {
        "add_file" => Some(CodexPatchProxyAction::AddFile),
        "delete_file" => Some(CodexPatchProxyAction::DeleteFile),
        "update_file" => Some(CodexPatchProxyAction::UpdateFile),
        "replace_file" => Some(CodexPatchProxyAction::ReplaceFile),
        _ => None,
    }
}

fn build_apply_patch_operation_arguments(
    operation: &Value,
    action: CodexPatchProxyAction,
) -> String {
    match action {
        CodexPatchProxyAction::AddFile | CodexPatchProxyAction::ReplaceFile => json!({
            "content": operation.get("content").and_then(Value::as_str).unwrap_or(""),
            "path": operation.get("path").and_then(Value::as_str).unwrap_or("")
        })
        .to_string(),
        CodexPatchProxyAction::DeleteFile => json!({
            "path": operation.get("path").and_then(Value::as_str).unwrap_or("")
        })
        .to_string(),
        CodexPatchProxyAction::UpdateFile => {
            let mut args = json!({
                "hunks": operation.get("hunks").cloned().unwrap_or_else(|| json!([])),
                "path": operation.get("path").and_then(Value::as_str).unwrap_or("")
            });
            if let Some(move_to) = operation.get("move_to").and_then(Value::as_str) {
                if !move_to.is_empty() {
                    args["move_to"] = json!(move_to);
                }
            }
            args.to_string()
        }
        CodexPatchProxyAction::Batch => json!({ "operations": [operation.clone()] }).to_string(),
    }
}

fn copy_response_request_fields(response: &mut Value, original_request: Option<&Value>) {
    let Some(original_request) = original_request else {
        return;
    };
    for key in [
        "instructions",
        "max_output_tokens",
        "parallel_tool_calls",
        "previous_response_id",
        "reasoning",
        "temperature",
        "tool_choice",
        "tools",
        "top_p",
        "metadata",
    ] {
        if let Some(value) = original_request.get(key) {
            response[key] = value.clone();
        }
    }
}

fn responses_arguments_to_chat(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => canonical_json_string(other),
    }
}

fn instruction_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| {
                part.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| part.as_str())
            })
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        other => other.as_str().unwrap_or_default().to_string(),
    }
}

fn canonical_json_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).unwrap_or_default(),
        Value::Array(values) => {
            let parts = values.iter().map(canonical_json_string).collect::<Vec<_>>();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| *key);
            let parts = entries
                .into_iter()
                .map(|(key, value)| {
                    let key = serde_json::to_string(key).unwrap_or_default();
                    format!("{key}:{}", canonical_json_string(value))
                })
                .collect::<Vec<_>>();
            format!("{{{}}}", parts.join(","))
        }
    }
}

fn apply_chat_reasoning_options(result: &mut Value, body: &Value, model: &str) {
    let Some(reasoning_enabled) = reasoning_requested(body) else {
        return;
    };
    let style = infer_chat_reasoning_style(model);

    match style {
        ChatReasoningStyle::Thinking => {
            result["thinking"] = json!({
                "type": if reasoning_enabled { "enabled" } else { "disabled" }
            });
        }
        ChatReasoningStyle::EnableThinking => {
            result["enable_thinking"] = json!(reasoning_enabled);
        }
        ChatReasoningStyle::ReasoningSplit => {
            result["reasoning_split"] = json!(reasoning_enabled);
        }
        _ => {}
    }

    if !reasoning_enabled {
        if style == ChatReasoningStyle::OpenRouter {
            result["reasoning"] = json!({ "effort": "none" });
        }
        return;
    }

    let Some(effort) = body.pointer("/reasoning/effort").and_then(Value::as_str) else {
        return;
    };
    let Some(mapped) = map_chat_reasoning_effort(effort, style) else {
        return;
    };

    match style {
        ChatReasoningStyle::OpenRouter => {
            result["reasoning"] = json!({ "effort": mapped });
        }
        ChatReasoningStyle::DeepSeek
        | ChatReasoningStyle::LowHigh
        | ChatReasoningStyle::Default
            if supports_reasoning_effort(model) =>
        {
            result["reasoning_effort"] = json!(mapped);
        }
        _ => {}
    }
}

fn reasoning_requested(body: &Value) -> Option<bool> {
    if let Some(effort) = body.pointer("/reasoning/effort").and_then(Value::as_str) {
        return Some(!matches!(
            effort.trim().to_ascii_lowercase().as_str(),
            "none" | "off" | "disabled"
        ));
    }

    body.get("reasoning").map(|value| !value.is_null())
}

fn infer_chat_reasoning_style(model: &str) -> ChatReasoningStyle {
    let model = model.to_ascii_lowercase();
    if model.contains("openrouter") || model.starts_with("openrouter/") {
        return ChatReasoningStyle::OpenRouter;
    }
    if model.contains("deepseek") {
        return ChatReasoningStyle::DeepSeek;
    }
    if model.contains("qwen") || model.contains("dashscope") || model.contains("bailian") {
        return ChatReasoningStyle::EnableThinking;
    }
    if model.contains("kimi")
        || model.contains("moonshot")
        || model.contains("glm")
        || model.contains("zhipu")
        || model.contains("z.ai")
        || model.contains("mimo")
    {
        return ChatReasoningStyle::Thinking;
    }
    if model.contains("minimax") {
        return ChatReasoningStyle::ReasoningSplit;
    }
    if model.contains("siliconflow") {
        return ChatReasoningStyle::EnableThinking;
    }
    if model.contains("stepfun") || model.contains("step-3.5-flash-2603") {
        return ChatReasoningStyle::LowHigh;
    }
    ChatReasoningStyle::Default
}

fn map_chat_reasoning_effort(effort: &str, style: ChatReasoningStyle) -> Option<&'static str> {
    let effort = effort.trim().to_ascii_lowercase();
    if matches!(effort.as_str(), "none" | "off" | "disabled") {
        return None;
    }

    match style {
        ChatReasoningStyle::DeepSeek => match effort.as_str() {
            "max" | "xhigh" => Some("max"),
            _ => Some("high"),
        },
        ChatReasoningStyle::LowHigh => match effort.as_str() {
            "minimal" | "low" => Some("low"),
            _ => Some("high"),
        },
        ChatReasoningStyle::OpenRouter => match effort.as_str() {
            "max" | "xhigh" => Some("xhigh"),
            "high" => Some("high"),
            "medium" => Some("medium"),
            "low" => Some("low"),
            "minimal" => Some("minimal"),
            _ => None,
        },
        _ => match effort.as_str() {
            "minimal" => Some("minimal"),
            "low" => Some("low"),
            "medium" => Some("medium"),
            "high" => Some("high"),
            "xhigh" => Some("xhigh"),
            "max" => Some("max"),
            _ => None,
        },
    }
}

fn supports_reasoning_effort(model: &str) -> bool {
    is_openai_o_series(model)
        || model
            .to_lowercase()
            .strip_prefix("gpt-")
            .and_then(|rest| rest.chars().next())
            .is_some_and(|ch| ch.is_ascii_digit() && ch >= '5')
        || infer_chat_reasoning_style(model) == ChatReasoningStyle::DeepSeek
        || infer_chat_reasoning_style(model) == ChatReasoningStyle::LowHigh
}

fn is_openai_o_series(model: &str) -> bool {
    model.len() > 1
        && model.starts_with('o')
        && model
            .as_bytes()
            .get(1)
            .is_some_and(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn proxy_settings_load_fails_closed_when_settings_json_is_corrupt() {
        let directory = tempfile::tempdir().expect("create temporary settings directory");
        let settings_path = directory.path().join("settings.json");
        std::fs::write(&settings_path, br#"{"relayProfiles":["#)
            .expect("write corrupt settings fixture");
        let store = SettingsStore::new(settings_path);

        let error = load_proxy_settings(&store).expect_err("corrupt settings must fail closed");

        assert!(
            error
                .to_string()
                .contains("failed to load protocol proxy settings"),
            "unexpected error: {error:#}"
        );
        assert!(
            format!("{error:#}").contains("解析设置文件失败"),
            "source error should remain available: {error:#}"
        );
    }

    #[test]
    fn proxy_settings_load_preserves_normal_settings_behavior() {
        let directory = tempfile::tempdir().expect("create temporary settings directory");
        let store = SettingsStore::new(directory.path().join("missing-settings.json"));

        let settings =
            load_proxy_settings(&store).expect("missing settings use established defaults");

        assert_eq!(settings, BackendSettings::default());
    }

    #[test]
    fn claude_desktop_route_diagnostic_contains_only_route_fields() {
        let detail = claude_desktop_route_diagnostic(
            "desktop-profile",
            "claude-haiku-4-5",
            "claude-haiku-4-5",
            503,
        );
        let object = detail
            .as_object()
            .expect("route diagnostic must be an object");
        let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
        keys.sort_unstable();

        assert_eq!(
            keys,
            [
                "http_status",
                "original_model",
                "profile_id",
                "upstream_model"
            ]
        );
        assert_eq!(object["http_status"], 503);
        assert_eq!(object["original_model"], "claude-haiku-4-5");
        assert_eq!(object["upstream_model"], "claude-haiku-4-5");
    }

    #[tokio::test]
    async fn responses_proxy_sends_one_upstream_request_with_original_model() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let address = listener.local_addr().expect("read mock upstream address");
        let server = thread::spawn(move || capture_upstream_requests(listener));
        let relay = RelayProfile {
            base_url: format!("http://{address}"),
            api_key: "test-key".to_string(),
            protocol: RelayProtocol::ChatCompletions,
            ..RelayProfile::default()
        };
        let body = json!({
            "model": "gpt-5.6-sol",
            "input": "verify one upstream request",
            "stream": false
        })
        .to_string();

        let response = open_responses_proxy_request_with_relay(&body, &relay)
            .await
            .expect("proxy request should succeed");
        assert_eq!(response.status_code, 200);
        drop(response);

        let (request_count, request) = server.join().expect("mock upstream should finish");
        let (headers, request_body) = request
            .split_once("\r\n\r\n")
            .expect("captured request should contain headers and body");
        let request_json: Value =
            serde_json::from_str(request_body).expect("captured body should be JSON");

        assert_eq!(
            request_count, 1,
            "one inbound request must not be duplicated"
        );
        assert!(headers.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert_eq!(request_json["model"], "gpt-5.6-sol");
        assert!(!request_body.contains("gpt-5.4"));
    }

    #[tokio::test]
    async fn claude_desktop_proxy_sends_anthropic_api_key_once() {
        assert_claude_desktop_proxy_auth_once(
            json!({"ANTHROPIC_API_KEY": "claude-test-key"}).to_string(),
            true,
        )
        .await;
    }

    #[tokio::test]
    async fn claude_desktop_proxy_sends_anthropic_auth_token_once() {
        assert_claude_desktop_proxy_auth_once(
            json!({"ANTHROPIC_AUTH_TOKEN": "claude-test-key"}).to_string(),
            false,
        )
        .await;
    }

    #[test]
    fn claude_messages_metadata_accepts_only_valid_protocol_values() {
        let metadata = ClaudeMessagesRequestMetadata::from_inbound(
            "/claude-desktop/v1/messages?other=1&BETA=true",
            "2024-10-22",
            "prompt-caching-2024-07-31,computer-use-2024-10-22",
        );
        assert!(metadata.beta);
        assert_eq!(metadata.anthropic_version.as_deref(), Some("2024-10-22"));
        assert_eq!(
            metadata.anthropic_beta.as_deref(),
            Some("prompt-caching-2024-07-31,computer-use-2024-10-22")
        );

        let invalid = ClaudeMessagesRequestMetadata::from_inbound(
            "/claude-desktop/v1/messages?beta=false",
            "2024-10-22\r\nx-injected: true",
            &"x".repeat(MAX_ANTHROPIC_BETA_HEADER_LEN + 1),
        );
        assert!(!invalid.beta);
        assert!(invalid.anthropic_version.is_none());
        assert!(invalid.anthropic_beta.is_none());
    }

    #[tokio::test]
    async fn claude_desktop_proxy_forwards_beta_protocol_metadata_once() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let address = listener.local_addr().expect("read mock upstream address");
        let server = thread::spawn(move || {
            capture_mock_upstream_requests(
                listener,
                vec![(200, r#"{"id":"msg-test","content":[]}"#)],
            )
        });
        let relay = RelayProfile {
            upstream_base_url: format!("http://{address}"),
            auth_contents: json!({"ANTHROPIC_AUTH_TOKEN": "claude-test-key"}).to_string(),
            target_app: "claude-desktop".to_string(),
            api_format: "Anthropic Messages".to_string(),
            ..RelayProfile::default()
        };
        let settings = crate::settings::BackendSettings::default();
        let metadata = ClaudeMessagesRequestMetadata::from_inbound(
            "/claude-desktop/v1/messages?beta=true",
            "2024-10-22",
            "computer-use-2024-10-22",
        );
        let body = json!({
            "model": "claude-opus-4-8",
            "max_tokens": 64,
            "messages": [{"role": "user", "content": "verify beta metadata"}],
            "stream": false
        })
        .to_string();

        let response = open_claude_desktop_messages_proxy_request_with_relay(
            &body, &relay, &settings, &metadata,
        )
        .await
        .expect("Claude Desktop proxy request should succeed");
        assert_eq!(response.status_code, 200);
        drop(response);

        let requests = server.join().expect("mock upstream should finish");
        assert_eq!(requests.len(), 1);
        let message_headers = requests[0]
            .split_once("\r\n\r\n")
            .map(|(headers, _)| headers.to_ascii_lowercase())
            .expect("captured message request should contain headers and body");
        assert!(message_headers.starts_with("post /v1/messages?beta=true http/1.1"));
        assert!(message_headers.contains("authorization: bearer claude-test-key"));
        assert!(!message_headers.contains("x-api-key:"));
        assert!(message_headers.contains("anthropic-version: 2024-10-22"));
        assert!(message_headers.contains("anthropic-beta: computer-use-2024-10-22"));
    }

    #[tokio::test]
    async fn claude_desktop_proxy_applies_saved_request_overrides_without_replacing_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let address = listener.local_addr().expect("read mock upstream address");
        let server = thread::spawn(move || {
            capture_mock_upstream_requests(
                listener,
                vec![(200, r#"{"id":"msg-test","content":[]}"#)],
            )
        });
        let relay = RelayProfile {
            upstream_base_url: format!("http://{address}"),
            auth_contents: json!({"ANTHROPIC_AUTH_TOKEN": "claude-test-key"}).to_string(),
            target_app: "claude-desktop".to_string(),
            api_format: "Anthropic Messages".to_string(),
            header_override: json!({
                "X-Provider": "cc-switch",
                "Authorization": "Bearer stale-override",
                "Content-Type": "text/plain"
            })
            .to_string(),
            body_override: json!({
                "temperature": 0.2,
                "stream": true,
                "metadata": {"source": "ccp"}
            })
            .to_string(),
            ..RelayProfile::default()
        };
        let body = json!({
            "model": "claude-opus-4-8",
            "max_tokens": 64,
            "messages": [{"role": "user", "content": "verify saved overrides"}],
            "metadata": {"existing": true},
            "stream": false
        })
        .to_string();

        let response = open_claude_desktop_messages_proxy_request_with_relay(
            &body,
            &relay,
            &crate::settings::BackendSettings::default(),
            &ClaudeMessagesRequestMetadata::default(),
        )
        .await
        .expect("Claude Desktop proxy request should succeed");
        assert_eq!(response.status_code, 200);
        drop(response);

        let requests = server.join().expect("mock upstream should finish");
        assert_eq!(requests.len(), 1);
        let (headers, request_body) = requests[0]
            .split_once("\r\n\r\n")
            .expect("captured message request should contain headers and body");
        let request_json: Value =
            serde_json::from_str(request_body).expect("captured body should be JSON");
        let headers = headers.to_ascii_lowercase();

        assert!(headers.contains("x-provider: cc-switch"));
        assert!(headers.contains("authorization: bearer claude-test-key"));
        assert!(!headers.contains("stale-override"));
        assert!(headers.contains("content-type: application/json"));
        assert_eq!(request_json["temperature"], json!(0.2));
        assert_eq!(request_json["metadata"]["existing"], json!(true));
        assert_eq!(request_json["metadata"]["source"], "ccp");
        assert_eq!(request_json["stream"], json!(false));
    }

    #[tokio::test]
    async fn claude_desktop_proxy_applies_saved_model_mapping() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let address = listener.local_addr().expect("read mock upstream address");
        let server = thread::spawn(move || {
            capture_mock_upstream_requests(
                listener,
                vec![(200, r#"{"id":"msg-test","content":[]}"#)],
            )
        });
        let relay = RelayProfile {
            upstream_base_url: format!("http://{address}"),
            auth_contents: json!({"ANTHROPIC_AUTH_TOKEN": "claude-test-key"}).to_string(),
            target_app: "claude-desktop".to_string(),
            api_format: "Anthropic Messages".to_string(),
            model_mapping_enabled: true,
            model_mapping_json: json!([{
                "routeId": "claude-haiku-4-5",
                "requestModel": "claude-opus-4-7"
            }])
            .to_string(),
            body_override: json!({"model": "stale-body-override"}).to_string(),
            ..RelayProfile::default()
        };
        let body = json!({
            "model": "claude-haiku-4-5",
            "max_tokens": 64,
            "messages": [{"role": "user", "content": "keep discovered model"}],
            "stream": false
        })
        .to_string();

        let response = open_claude_desktop_messages_proxy_request_with_relay(
            &body,
            &relay,
            &crate::settings::BackendSettings::default(),
            &ClaudeMessagesRequestMetadata::default(),
        )
        .await
        .expect("Claude Desktop proxy request should succeed");
        drop(response);

        let requests = server.join().expect("mock upstream should finish");
        assert_eq!(requests.len(), 1);
        let (headers, message_body) = requests[0]
            .split_once("\r\n\r\n")
            .expect("captured message request should contain a body");
        let message_json: Value =
            serde_json::from_str(message_body).expect("message body should be JSON");
        assert!(headers.starts_with("POST /v1/messages HTTP/1.1"));
        assert_eq!(message_json["model"], "claude-opus-4-7");
    }

    #[tokio::test]
    async fn claude_desktop_proxy_maps_all_claude_model_families() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let address = listener.local_addr().expect("read mock upstream address");
        let server = thread::spawn(move || {
            capture_mock_upstream_requests(
                listener,
                vec![
                    (200, r#"{"id":"msg-opus","content":[]}"#),
                    (200, r#"{"id":"msg-sonnet","content":[]}"#),
                    (200, r#"{"id":"msg-haiku","content":[]}"#),
                    (200, r#"{"id":"msg-fable","content":[]}"#),
                ],
            )
        });
        let relay = RelayProfile {
            upstream_base_url: format!("http://{address}"),
            api_key: "current-profile-key".to_string(),
            target_app: "claude-desktop".to_string(),
            api_format: "Anthropic Messages".to_string(),
            model_mapping_enabled: true,
            model_mapping_json: json!([
                {"routeId": "claude-opus-4-8", "requestModel": "mapped-opus"},
                {"routeId": "claude-sonnet-4-5", "requestModel": "mapped-sonnet"},
                {"routeId": "claude-haiku-4-5", "requestModel": "mapped-haiku"},
                {"routeId": "claude-fable-5", "requestModel": "mapped-fable"}
            ])
            .to_string(),
            ..RelayProfile::default()
        };
        let models = [
            ("claude-opus-4-8", "mapped-opus"),
            ("claude-sonnet-4-5", "mapped-sonnet"),
            ("claude-haiku-4-5", "mapped-haiku"),
            ("claude-fable-5", "mapped-fable"),
        ];

        for (model, _) in models {
            let body = json!({
                "model": model,
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "."}],
                "stream": false
            })
            .to_string();
            let response = open_claude_desktop_messages_proxy_request_with_relay(
                &body,
                &relay,
                &crate::settings::BackendSettings::default(),
                &ClaudeMessagesRequestMetadata::default(),
            )
            .await
            .expect("Claude Desktop proxy request should succeed");
            assert_eq!(response.status_code, 200);
        }

        let requests = server.join().expect("mock upstream should finish");
        assert_eq!(requests.len(), models.len());
        for (request, (_, expected_model)) in requests.iter().zip(models) {
            let (_, body) = request
                .split_once("\r\n\r\n")
                .expect("captured request should contain a body");
            let body: Value = serde_json::from_str(body).expect("request body should be JSON");
            assert_eq!(body["model"], expected_model);
        }
    }

    #[tokio::test]
    async fn claude_desktop_probe_shaped_request_applies_saved_mapping_once() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let address = listener.local_addr().expect("read mock upstream address");
        let server = thread::spawn(move || {
            capture_mock_upstream_requests(
                listener,
                vec![(200, r#"{"id":"msg-test","content":[]}"#)],
            )
        });
        let relay = RelayProfile {
            upstream_base_url: format!("http://{address}"),
            api_key: "current-profile-key".to_string(),
            target_app: "claude-desktop".to_string(),
            api_format: "Anthropic Messages".to_string(),
            model_mapping_enabled: true,
            model_mapping_json: json!([{
                "routeId": "claude-haiku-4-5",
                "requestModel": "claude-fable-5"
            }])
            .to_string(),
            ..RelayProfile::default()
        };
        let body = json!({
            "model": "claude-haiku-4-5",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "."}]
        })
        .to_string();

        let response = open_claude_desktop_messages_proxy_request_with_relay(
            &body,
            &relay,
            &crate::settings::BackendSettings::default(),
            &ClaudeMessagesRequestMetadata::default(),
        )
        .await
        .expect("probe-shaped request should be sent as a normal message");
        assert_eq!(response.status_code, 200);
        drop(response);

        let requests = server.join().expect("mock upstream should finish");
        assert_eq!(
            requests.len(),
            1,
            "request must not perform model discovery"
        );
        let (headers, message_body) = requests[0]
            .split_once("\r\n\r\n")
            .expect("captured request should contain a body");
        let headers = headers.to_ascii_lowercase();
        let message_json: Value =
            serde_json::from_str(message_body).expect("message body should be JSON");
        assert!(headers.starts_with("post /v1/messages http/1.1"));
        assert!(headers.contains("authorization: bearer current-profile-key"));
        assert_eq!(message_json["model"], "claude-fable-5");
    }

    #[tokio::test]
    async fn claude_desktop_proxy_preserves_model_when_mapping_is_disabled() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let address = listener.local_addr().expect("read mock upstream address");
        let server = thread::spawn(move || {
            capture_mock_upstream_requests(
                listener,
                vec![(200, r#"{"id":"msg-test","content":[]}"#)],
            )
        });
        let relay = RelayProfile {
            upstream_base_url: format!("http://{address}"),
            auth_contents: json!({"ANTHROPIC_AUTH_TOKEN": "claude-test-key"}).to_string(),
            target_app: "claude-desktop".to_string(),
            api_format: "Anthropic Messages".to_string(),
            model_mapping_enabled: false,
            model_mapping_json: json!([{
                "routeId": "claude-haiku-4-5",
                "requestModel": "claude-opus-4-7"
            }])
            .to_string(),
            ..RelayProfile::default()
        };
        let body = json!({
            "model": "claude-haiku-4-5",
            "max_tokens": 64,
            "messages": [{"role": "user", "content": "use current catalog"}],
            "stream": false
        })
        .to_string();

        let response = open_claude_desktop_messages_proxy_request_with_relay(
            &body,
            &relay,
            &crate::settings::BackendSettings::default(),
            &ClaudeMessagesRequestMetadata::default(),
        )
        .await
        .expect("Claude Desktop proxy request should succeed");
        drop(response);

        let requests = server.join().expect("mock upstream should finish");
        assert_eq!(requests.len(), 1);
        let (_, message_body) = requests[0]
            .split_once("\r\n\r\n")
            .expect("captured message request should contain a body");
        let message_json: Value =
            serde_json::from_str(message_body).expect("message body should be JSON");
        assert_eq!(message_json["model"], "claude-haiku-4-5");
    }

    #[tokio::test]
    async fn claude_desktop_proxy_preserves_unmatched_model_without_catalog_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let address = listener.local_addr().expect("read mock upstream address");
        let server = thread::spawn(move || {
            capture_mock_upstream_requests(
                listener,
                vec![(200, r#"{"id":"msg-test","content":[]}"#)],
            )
        });
        let relay = RelayProfile {
            upstream_base_url: format!("http://{address}"),
            auth_contents: json!({"ANTHROPIC_AUTH_TOKEN": "claude-test-key"}).to_string(),
            target_app: "claude-desktop".to_string(),
            api_format: "Anthropic Messages".to_string(),
            model_mapping_enabled: true,
            model_mapping_json: json!([{
                "routeId": "claude-opus-4-8",
                "requestModel": "upstream-opus"
            }])
            .to_string(),
            ..RelayProfile::default()
        };
        let body = json!({
            "model": "claude-haiku-4-5",
            "max_tokens": 64,
            "messages": [{"role": "user", "content": "do not post"}],
            "stream": false
        })
        .to_string();

        let response = open_claude_desktop_messages_proxy_request_with_relay(
            &body,
            &relay,
            &crate::settings::BackendSettings::default(),
            &ClaudeMessagesRequestMetadata::default(),
        )
        .await
        .expect("message forwarding must not depend on a model catalog endpoint");
        drop(response);
        let requests = server.join().expect("mock upstream should finish");
        assert_eq!(requests.len(), 1);
        let (headers, message_body) = requests[0]
            .split_once("\r\n\r\n")
            .expect("captured message request should contain a body");
        let message_json: Value =
            serde_json::from_str(message_body).expect("message body should be JSON");
        assert!(headers.starts_with("POST /v1/messages HTTP/1.1"));
        assert_eq!(message_json["model"], "claude-haiku-4-5");
    }

    #[tokio::test]
    async fn claude_desktop_proxy_rejects_invalid_model_before_connecting_upstream() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let address = listener.local_addr().expect("read mock upstream address");
        let relay = RelayProfile {
            upstream_base_url: format!("http://{address}"),
            auth_contents: json!({"ANTHROPIC_AUTH_TOKEN": "claude-test-key"}).to_string(),
            target_app: "claude-desktop".to_string(),
            api_format: "Anthropic Messages".to_string(),
            ..RelayProfile::default()
        };

        for invalid_body in ["[]", "{}", r#"{"model":""}"#, r#"{"model":42}"#] {
            assert!(
                open_claude_desktop_messages_proxy_request_with_relay(
                    invalid_body,
                    &relay,
                    &crate::settings::BackendSettings::default(),
                    &ClaudeMessagesRequestMetadata::default(),
                )
                .await
                .is_err(),
                "invalid model body must be rejected: {invalid_body}"
            );
        }

        listener
            .set_nonblocking(true)
            .expect("make mock listener nonblocking");
        let error = listener
            .accept()
            .expect_err("invalid requests must not connect upstream");
        assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
    }

    async fn assert_claude_desktop_proxy_auth_once(
        auth_contents: String,
        expects_anthropic_api_key: bool,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let address = listener.local_addr().expect("read mock upstream address");
        let server = thread::spawn(move || {
            capture_mock_upstream_requests(
                listener,
                vec![(200, r#"{"id":"msg-test","content":[]}"#)],
            )
        });
        let relay = RelayProfile {
            upstream_base_url: format!("http://{address}"),
            auth_contents,
            target_app: "claude-desktop".to_string(),
            api_format: "Anthropic Messages".to_string(),
            ..RelayProfile::default()
        };
        let settings = crate::settings::BackendSettings {
            relay_api_key: "stale-global-key".to_string(),
            ..crate::settings::BackendSettings::default()
        };
        let body = json!({
            "model": "claude-opus-4-8",
            "max_tokens": 64,
            "messages": [{"role": "user", "content": "verify anthropic auth"}],
            "stream": false
        })
        .to_string();

        let response = open_claude_desktop_messages_proxy_request_with_relay(
            &body,
            &relay,
            &settings,
            &ClaudeMessagesRequestMetadata::default(),
        )
        .await
        .expect("Claude Desktop proxy request should succeed");
        assert_eq!(response.status_code, 200);
        drop(response);

        let requests = server.join().expect("mock upstream should finish");
        assert_eq!(requests.len(), 1, "one message POST is required");
        let (headers, request_body) = requests[0]
            .split_once("\r\n\r\n")
            .expect("captured message request should contain headers and body");
        let request_json: Value =
            serde_json::from_str(request_body).expect("captured body should be JSON");
        let headers = headers.to_ascii_lowercase();

        assert!(headers.starts_with("post /v1/messages http/1.1"));
        assert_eq!(
            headers
                .matches("authorization: bearer claude-test-key")
                .count(),
            usize::from(!expects_anthropic_api_key)
        );
        assert_eq!(
            headers.matches("x-api-key: claude-test-key").count(),
            usize::from(expects_anthropic_api_key)
        );
        assert!(!headers.contains("stale-global-key"));
        assert!(headers.contains("anthropic-version: 2023-06-01"));
        assert_eq!(request_json["model"], "claude-opus-4-8");
    }

    fn capture_mock_upstream_requests(
        listener: TcpListener,
        responses: Vec<(u16, &'static str)>,
    ) -> Vec<String> {
        let mut requests = Vec::with_capacity(responses.len());
        for (status, response_body) in responses {
            let (mut stream, _) = listener.accept().expect("accept mock upstream request");
            requests.push(read_http_request(&mut stream));
            write_json_response(&mut stream, status, response_body);
        }

        listener
            .set_nonblocking(true)
            .expect("make mock listener nonblocking");
        let deadline = Instant::now() + Duration::from_millis(250);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut extra, _)) => {
                    requests.push(read_http_request(&mut extra));
                    write_json_response(
                        &mut extra,
                        500,
                        r#"{"error":{"message":"unexpected extra request"}}"#,
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("accept extra mock upstream request: {error}"),
            }
        }
        requests
    }

    fn write_json_response(stream: &mut TcpStream, status: u16, body: &str) {
        let reason = match status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Mock Status",
        };
        write!(
            stream,
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write mock response");
        stream.flush().expect("flush mock response");
    }

    fn capture_upstream_requests(listener: TcpListener) -> (usize, String) {
        let (mut stream, _) = listener.accept().expect("accept proxy request");
        let request = read_http_request(&mut stream);
        let response_body = r#"{"id":"chatcmpl-test","choices":[]}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        )
        .expect("write mock response");
        stream.flush().expect("flush mock response");
        drop(stream);

        listener
            .set_nonblocking(true)
            .expect("make mock listener nonblocking");
        let deadline = Instant::now() + Duration::from_millis(250);
        let mut request_count = 1;
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut duplicate, _)) => {
                    request_count += 1;
                    let _ = read_http_request(&mut duplicate);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("accept duplicate proxy request: {error}"),
            }
        }
        (request_count, request)
    }

    fn read_http_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set mock read timeout");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let read = stream.read(&mut buffer).expect("read proxy request");
            assert!(read > 0, "proxy request ended before its body was complete");
            request.extend_from_slice(&buffer[..read]);

            let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let header_end = header_end + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if request.len() >= header_end + content_length {
                request.truncate(header_end + content_length);
                return String::from_utf8(request).expect("proxy request should be UTF-8");
            }
        }
    }
}
