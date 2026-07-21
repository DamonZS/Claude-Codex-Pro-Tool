use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::settings::{RelayProfile, SettingsStore};
use serde_json::{Value, json};

const BASE_URL_ENV_KEYS: &[&str] = &[
    "CLAUDE_CODEX_PRO_OPENAI_BASE_URL",
    "CLAUDE_CODEX_PRO_BASE_URL",
    "OPENAI_BASE_URL",
    "OPENAI_API_BASE_URL",
    "OPENAI_API_BASE",
    "OPENAI_API_URL",
];
const API_KEY_ENV_KEYS: &[&str] = &[
    "CLAUDE_CODEX_PRO_OPENAI_API_KEY",
    "CLAUDE_CODEX_PRO_API_KEY",
    "OPENAI_API_KEY",
];

/// Base URL 可能指向 Anthropic/Claude 兼容协议的子路径。模型目录通常
/// 仍挂在供应商根路径下，因此在当前路径失败时只对这些已知后缀做根路径候选。
const KNOWN_MODEL_COMPAT_SUFFIXES: &[&str] = &[
    "/api/claudecode",
    "/api/anthropic",
    "/apps/anthropic",
    "/api/coding",
    "/claudecode",
    "/anthropic",
    "/step_plan",
    "/coding",
    "/claude",
];

#[derive(Debug, Clone)]
struct ModelSource {
    source_id: String,
    source_type: String,
    name: String,
    base_url: String,
    api_key: String,
    anthropic_api_key: bool,
    include_anthropic_version: bool,
}

const MODEL_PAYLOAD_CONTAINER_KEYS: &[&str] = &[
    "data", "models", "items", "result", "results", "payload", "response",
];
const MODEL_PAYLOAD_ID_KEYS: &[&str] = &[
    "id", "model", "name", "model_id", "modelId", "slug", "value",
];
const MODEL_PAYLOAD_ERROR_KEYS: &[&str] = &["error", "errors", "error_code", "errorCode"];
const MODEL_MAP_METADATA_KEYS: &[&str] = &[
    "object", "type", "total", "count", "page", "limit", "offset", "next", "previous", "has_more",
    "hasMore", "message", "code", "status", "success", "ok",
];
const MODEL_PAYLOAD_MAX_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModelPayloadResult {
    Models(Vec<String>),
    Empty,
    BusinessError,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RelayProfileCatalogModel {
    model: String,
    display_name: String,
    context_window: Option<u64>,
}

#[derive(Debug, Default)]
struct CodexConfig {
    root: HashMap<String, String>,
    profiles: HashMap<String, HashMap<String, String>>,
    model_providers: HashMap<String, HashMap<String, String>>,
}

pub async fn read_codex_model_catalog() -> Value {
    let home = codex_home_dir();
    let settings_path = crate::paths::default_settings_path();
    if settings_path.exists() {
        if let Ok(settings) = SettingsStore::new(settings_path).load() {
            return relay_profile_model_catalog_value(&home, &settings.active_relay_profile());
        }
    }
    let env = std::env::vars().collect::<HashMap<_, _>>();
    let client = match crate::http_client::proxied_client("ClaudeCodexPro/1.0") {
        Ok(client) => client,
        Err(error) => {
            return json!({
                "status": "failed",
                "path": home.join("config.toml").to_string_lossy(),
                "message": error.to_string(),
                "model": "",
                "model_provider": "",
                "provider_name": "",
                "default_model": "",
                "models": [],
                "model_descriptors": [],
                "sources": [],
                "responses_api": responses_api_status("unknown", "", "")
            });
        }
    };
    read_codex_model_catalog_from_home(&home, &env, client).await
}

fn relay_profile_model_catalog_value(home: &Path, profile: &RelayProfile) -> Value {
    let model_descriptors = relay_profile_catalog_models(profile);
    let (catalog_models, catalog_statuses) = models_from_default_model_catalog_json_files(home);
    let models = unique_strings(
        relay_profile_model_ids(profile)
            .into_iter()
            .chain(catalog_models)
            .collect(),
    );
    let model = profile.model.trim().to_string();
    let default_model = if models.iter().any(|item| item == &model) {
        model.clone()
    } else {
        models.first().cloned().unwrap_or_default()
    };
    let provider_name = if profile.name.trim().is_empty() {
        profile.id.trim()
    } else {
        profile.name.trim()
    };
    let mut sources = vec![json!({
        "id": format!("relay-profile:{}", profile.id),
        "type": "relay_profile_model_list",
        "name": provider_name,
        "base_url": profile.base_url.trim(),
        "status": "ok",
        "models": relay_profile_model_ids(profile).len(),
        "responses_api": responses_api_status("unknown", "", "")
    })];
    sources.extend(catalog_statuses);

    json!({
        "status": if models.is_empty() { "not_configured" } else { "ok" },
        "path": home.join("config.toml").to_string_lossy(),
        "model": model,
        "model_provider": profile.id.trim(),
        "provider_name": provider_name,
        "default_model": default_model,
        "models": models,
        "model_descriptors": model_descriptors.iter().map(|entry| {
            let mut descriptor = json!({
                "model": entry.model,
                "display_name": entry.display_name,
            });
            if let Some(context_window) = entry.context_window {
                descriptor["context_window"] = json!(context_window);
            }
            descriptor
        }).collect::<Vec<_>>(),
        "sources": sources,
        "responses_api": responses_api_status("unknown", "", "")
    })
}

fn relay_profile_model_ids(profile: &RelayProfile) -> Vec<String> {
    unique_strings(
        std::iter::once(profile.model.as_str())
            .chain(profile.model_list.split(['\r', '\n', ',']))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .chain(
                relay_profile_catalog_models(profile)
                    .into_iter()
                    .map(|entry| entry.model),
            )
            .collect(),
    )
}

fn relay_profile_catalog_models(profile: &RelayProfile) -> Vec<RelayProfileCatalogModel> {
    let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(&profile.codex_catalog_json) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    rows.into_iter()
        .filter_map(|row| {
            let model = row.get("model")?.as_str()?.trim();
            if model.is_empty() || !seen.insert(model.to_string()) {
                return None;
            }
            let display_name = row
                .get("displayName")
                .or_else(|| row.get("display_name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(model)
                .to_string();
            let context_window = row
                .get("contextWindow")
                .or_else(|| row.get("context_window"))
                .and_then(|value| {
                    value.as_u64().or_else(|| {
                        value
                            .as_str()
                            .map(|text| {
                                text.chars()
                                    .filter(char::is_ascii_digit)
                                    .collect::<String>()
                            })
                            .and_then(|text| text.parse::<u64>().ok())
                    })
                })
                .filter(|value| *value > 0);
            Some(RelayProfileCatalogModel {
                model: model.to_string(),
                display_name,
                context_window,
            })
        })
        .collect()
}

pub async fn read_codex_model_catalog_from_home(
    home: &Path,
    env: &HashMap<String, String>,
    client: reqwest::Client,
) -> Value {
    let config_path = home.join("config.toml");
    let auth_api_key = read_codex_auth_api_key(&home.join("auth.json"));
    let (config, effective, error) = load_codex_config(&config_path);
    let mut model = string_value(effective.get("model"));
    let mut model_provider = string_value(effective.get("model_provider"));
    let (resolved_provider, provider_config) =
        provider_config_for_model_provider(&config, &model_provider);
    if model_provider.is_empty() && !resolved_provider.is_empty() {
        model_provider = resolved_provider;
    }
    let provider_name = provider_config
        .as_ref()
        .and_then(|provider| provider.get("name"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| model_provider.clone());

    if let Some(error) = error.as_ref().filter(|error| *error != "missing") {
        return json!({
            "status": "failed",
            "path": config_path.to_string_lossy(),
            "message": error,
            "model": model,
            "model_provider": model_provider,
            "provider_name": provider_name,
            "default_model": "",
            "models": [],
            "sources": [],
            "responses_api": responses_api_status("unknown", "", "")
        });
    }

    let mut sources = model_sources_from_environment(env, &auth_api_key);
    if error.is_none() {
        if let Some(source) = model_source_from_config(&config, &effective, env, &auth_api_key) {
            if sources
                .iter()
                .all(|existing| trim_url(&existing.base_url) != trim_url(&source.base_url))
            {
                sources.push(source);
            }
        }
    }

    let mut source_statuses = Vec::new();
    let mut models = Vec::new();
    for source in sources.iter() {
        let (source_models, mut source_status) = fetch_models_from_source(&client, source).await;
        source_status["responses_api"] = responses_api_status("unknown", "", "");
        models.extend(source_models);
        source_statuses.push(source_status);
    }
    let (catalog_models, catalog_status) = models_from_config_model_catalog_json(home, &effective);
    models.extend(catalog_models);
    if let Some(status) = catalog_status {
        source_statuses.push(status);
    }

    models = unique_strings(models);
    if model.is_empty() {
        model = string_value(effective.get("default_model"));
    }
    let default_model = if models.iter().any(|item| item == &model) {
        model.clone()
    } else {
        models.first().cloned().unwrap_or_default()
    };
    let status = if !models.is_empty() {
        "ok"
    } else if !source_statuses.is_empty()
        && source_statuses
            .iter()
            .any(|source| source.get("status").and_then(Value::as_str) == Some("failed"))
    {
        "failed"
    } else if error.as_deref() == Some("missing") {
        "missing"
    } else {
        "not_configured"
    };
    let responses_api = preferred_responses_api_status(&source_statuses);

    json!({
        "status": status,
        "path": config_path.to_string_lossy(),
        "model": model,
        "model_provider": model_provider,
        "provider_name": provider_name,
        "default_model": default_model,
        "models": models,
        "sources": source_statuses,
        "responses_api": responses_api
    })
}

fn codex_home_dir() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(crate::relay_config::default_codex_home_dir)
}

fn load_codex_config(path: &Path) -> (CodexConfig, HashMap<String, String>, Option<String>) {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (
                CodexConfig::default(),
                HashMap::new(),
                Some("missing".to_string()),
            );
        }
        Err(error) => {
            return (
                CodexConfig::default(),
                HashMap::new(),
                Some(error.to_string()),
            );
        }
    };
    let config = parse_codex_config(&contents);
    let mut effective = config.root.clone();
    if let Some(profile) = config.root.get("profile") {
        if let Some(profile_values) = config.profiles.get(profile) {
            effective.extend(profile_values.clone());
        }
    }
    (config, effective, None)
}

fn parse_codex_config(contents: &str) -> CodexConfig {
    let mut config = CodexConfig::default();
    let mut section = ConfigSection::Root;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = ConfigSection::from_header(trimmed.trim_matches(&['[', ']'][..]));
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim().to_string();
        let value = unquote_toml_string(value);
        match &section {
            ConfigSection::Root => {
                config.root.insert(key, value);
            }
            ConfigSection::Profile(name) => {
                config
                    .profiles
                    .entry(name.clone())
                    .or_default()
                    .insert(key, value);
            }
            ConfigSection::ModelProvider(name) => {
                config
                    .model_providers
                    .entry(name.clone())
                    .or_default()
                    .insert(key, value);
            }
            ConfigSection::Other => {}
        }
    }
    config
}

#[derive(Debug, Clone)]
enum ConfigSection {
    Root,
    Profile(String),
    ModelProvider(String),
    Other,
}

impl ConfigSection {
    fn from_header(header: &str) -> Self {
        if let Some(name) = header.strip_prefix("profiles.") {
            return Self::Profile(name.trim_matches('"').to_string());
        }
        if let Some(name) = header.strip_prefix("model_providers.") {
            return Self::ModelProvider(name.trim_matches('"').to_string());
        }
        Self::Other
    }
}

fn read_codex_auth_api_key(path: &Path) -> String {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let Ok(payload) = serde_json::from_str::<Value>(&contents) else {
        return String::new();
    };
    for key in [
        "OPENAI_API_KEY",
        "api_key",
        "apikey",
        "access_token",
        "token",
    ] {
        let value = payload
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        if !value.is_empty() {
            return value.to_string();
        }
    }
    String::new()
}

fn provider_config_for_model_provider(
    config: &CodexConfig,
    model_provider: &str,
) -> (String, Option<HashMap<String, String>>) {
    if !model_provider.is_empty() {
        return (
            model_provider.to_string(),
            config.model_providers.get(model_provider).cloned(),
        );
    }
    if config.model_providers.len() == 1 {
        if let Some((name, provider)) = config.model_providers.iter().next() {
            return (name.clone(), Some(provider.clone()));
        }
    }
    (model_provider.to_string(), None)
}

fn model_sources_from_environment(
    env: &HashMap<String, String>,
    auth_api_key: &str,
) -> Vec<ModelSource> {
    let base_url = first_env_value(env, BASE_URL_ENV_KEYS);
    if base_url.is_empty() {
        return Vec::new();
    }
    let api_key = first_env_value(env, API_KEY_ENV_KEYS);
    vec![ModelSource {
        source_id: "env:openai-compatible".to_string(),
        source_type: "environment".to_string(),
        name: "Environment".to_string(),
        base_url,
        api_key: if api_key.is_empty() {
            auth_api_key.to_string()
        } else {
            api_key
        },
        anthropic_api_key: false,
        include_anthropic_version: false,
    }]
}

fn model_source_from_config(
    config: &CodexConfig,
    effective: &HashMap<String, String>,
    env: &HashMap<String, String>,
    auth_api_key: &str,
) -> Option<ModelSource> {
    let model_provider = string_value(effective.get("model_provider"));
    let (resolved_provider, provider_config) =
        provider_config_for_model_provider(config, &model_provider);
    let provider_config = provider_config?;
    let base_url = string_value(provider_config.get("base_url"));
    if base_url.is_empty() {
        return None;
    }
    let name = string_value(provider_config.get("name"));
    let api_key = provider_api_key(&provider_config, env, auth_api_key);
    Some(ModelSource {
        source_id: format!(
            "config:{}",
            if resolved_provider.is_empty() {
                &name
            } else {
                &resolved_provider
            }
        ),
        source_type: "config".to_string(),
        name: if name.is_empty() {
            resolved_provider
        } else {
            name
        },
        base_url,
        api_key,
        anthropic_api_key: false,
        include_anthropic_version: false,
    })
}

fn provider_api_key(
    provider_config: &HashMap<String, String>,
    env: &HashMap<String, String>,
    auth_api_key: &str,
) -> String {
    for key in [
        "experimental_bearer_token",
        "api_key",
        "apikey",
        "bearer_token",
        "token",
    ] {
        let value = string_value(provider_config.get(key));
        if !value.is_empty() {
            return value;
        }
    }
    for key in [
        "env_key",
        "api_key_env",
        "api_key_env_var",
        "key_env",
        "bearer_token_env",
    ] {
        let env_name = string_value(provider_config.get(key));
        if !env_name.is_empty() {
            let value = first_env_value(env, &[&env_name]);
            if !value.is_empty() {
                return value;
            }
        }
    }
    let env_key = first_env_value(env, API_KEY_ENV_KEYS);
    if env_key.is_empty() {
        auth_api_key.to_string()
    } else {
        env_key
    }
}

async fn fetch_models_from_source(
    client: &reqwest::Client,
    source: &ModelSource,
) -> (Vec<String>, Value) {
    let candidates = match build_models_url_candidates(&source.base_url) {
        Ok(candidates) if !candidates.is_empty() => candidates,
        Ok(_) => Vec::new(),
        Err(error) => {
            let mut safe_source = json!({
                "id": source.source_id,
                "type": source.source_type,
                "name": source.name,
                "base_url": safe_url_for_status(&source.base_url),
                "endpoint": "",
                "candidate_endpoints": [],
                "auth": if source.api_key.is_empty() { "missing" } else { "present" },
            });
            safe_source["status"] = json!("failed");
            safe_source["message"] = json!(error.to_string());
            safe_source["models"] = json!(0);
            return finish_model_discovery(Vec::new(), safe_source, Vec::new());
        }
    };
    let safe_candidates = candidates
        .iter()
        .map(|candidate| safe_url_for_status(candidate))
        .collect::<Vec<_>>();
    let mut safe_source = json!({
        "id": source.source_id,
        "type": source.source_type,
        "name": source.name,
        "base_url": safe_url_for_status(&source.base_url),
        "endpoint": safe_candidates.first().cloned().unwrap_or_default(),
        "candidate_endpoints": safe_candidates,
        "auth": if source.api_key.is_empty() { "missing" } else { "present" },
    });
    let mut attempts = Vec::new();
    if candidates.is_empty() {
        safe_source["status"] = json!("failed");
        safe_source["message"] = json!("Missing base URL");
        safe_source["models"] = json!(0);
        return finish_model_discovery(Vec::new(), safe_source, attempts);
    }

    for endpoint in candidates {
        let safe_endpoint = safe_url_for_status(&endpoint);
        let request = client
            .get(&endpoint)
            .header(reqwest::header::ACCEPT, "application/json");
        let request = crate::http_client::apply_api_auth_headers(
            request,
            &source.api_key,
            source.anthropic_api_key,
            source.include_anthropic_version,
        );

        match request.send().await {
            Ok(response) if response.status().is_success() => {
                let status_code = response.status().as_u16();
                match response.json::<Value>().await {
                    Ok(payload) => {
                        let (payload_result, payload_shape) = parse_model_payload(&payload);
                        match payload_result {
                            ModelPayloadResult::Models(models) => {
                                let models = unique_strings(models);
                                attempts.push(json!({
                                    "endpoint": safe_endpoint,
                                    "http_status": status_code,
                                    "models": models.len(),
                                    "action": "accepted",
                                    "payload_shape": payload_shape,
                                }));
                                safe_source["endpoint"] = json!(safe_url_for_status(&endpoint));
                                safe_source["status"] = json!("ok");
                                safe_source["models"] = json!(models.len());
                                return finish_model_discovery(models, safe_source, attempts);
                            }
                            ModelPayloadResult::Empty => {
                                attempts.push(json!({
                                    "endpoint": safe_endpoint,
                                    "http_status": status_code,
                                    "models": 0,
                                    "action": "empty_payload",
                                    "payload_shape": payload_shape,
                                }));
                                safe_source["endpoint"] = json!(safe_url_for_status(&endpoint));
                                safe_source["status"] = json!("ok");
                                safe_source["models"] = json!(0);
                                return finish_model_discovery(Vec::new(), safe_source, attempts);
                            }
                            ModelPayloadResult::BusinessError => {
                                attempts.push(json!({
                                    "endpoint": safe_endpoint,
                                    "http_status": status_code,
                                    "models": 0,
                                    "action": "business_error",
                                    "payload_shape": payload_shape,
                                }));
                                safe_source["endpoint"] = json!(safe_url_for_status(&endpoint));
                                return finish_model_discovery(
                                    Vec::new(),
                                    failed_source(
                                        safe_source,
                                        "模型目录返回业务错误；请检查当前 Key 与供应商分组"
                                            .to_string(),
                                    ),
                                    attempts,
                                );
                            }
                            ModelPayloadResult::Unknown => {
                                attempts.push(json!({
                                    "endpoint": safe_endpoint,
                                    "http_status": status_code,
                                    "models": 0,
                                    "action": "unknown_payload_shape",
                                    "payload_shape": payload_shape,
                                }));
                                safe_source["endpoint"] = json!(safe_url_for_status(&endpoint));
                                return finish_model_discovery(
                                    Vec::new(),
                                    failed_source(
                                        safe_source,
                                        "模型目录返回了未识别的 JSON 结构".to_string(),
                                    ),
                                    attempts,
                                );
                            }
                        }
                    }
                    Err(error) => {
                        attempts.push(json!({
                            "endpoint": safe_endpoint,
                            "http_status": status_code,
                            "models": 0,
                            "action": "parse_failed",
                        }));
                        safe_source["endpoint"] = json!(safe_url_for_status(&endpoint));
                        return finish_model_discovery(
                            Vec::new(),
                            failed_source(safe_source, format!("模型目录响应解析失败: {error}")),
                            attempts,
                        );
                    }
                }
            }
            Ok(response) => {
                let status_code = response.status().as_u16();
                let retryable = matches!(status_code, 404 | 405);
                attempts.push(json!({
                    "endpoint": safe_endpoint,
                    "http_status": status_code,
                    "models": 0,
                    "action": if retryable { "try_next" } else { "failed" },
                }));
                safe_source["endpoint"] = json!(safe_url_for_status(&endpoint));
                if retryable {
                    continue;
                }
                return finish_model_discovery(
                    Vec::new(),
                    failed_source(safe_source, format!("HTTP {status_code}")),
                    attempts,
                );
            }
            Err(error) => {
                attempts.push(json!({
                    "endpoint": safe_endpoint,
                    "http_status": Value::Null,
                    "models": 0,
                    "action": "failed",
                }));
                safe_source["endpoint"] = json!(safe_url_for_status(&endpoint));
                return finish_model_discovery(
                    Vec::new(),
                    failed_source(safe_source, model_request_error_message(&error)),
                    attempts,
                );
            }
        }
    }

    finish_model_discovery(
        Vec::new(),
        failed_source(
            safe_source,
            "所有模型目录候选均返回 HTTP 404/405".to_string(),
        ),
        attempts,
    )
}

fn finish_model_discovery(
    models: Vec<String>,
    mut source: Value,
    attempts: Vec<Value>,
) -> (Vec<String>, Value) {
    source["attempts"] = json!(attempts);
    let _ = crate::diagnostic_log::append_diagnostic_log(
        "model_catalog.discovery",
        json!({
            "source_id": source.get("id").and_then(Value::as_str).unwrap_or_default(),
            "candidate_endpoints": source
                .get("candidate_endpoints")
                .cloned()
                .unwrap_or_else(|| json!([])),
            "attempts": source
                .get("attempts")
                .cloned()
                .unwrap_or_else(|| json!([])),
            "endpoint": source.get("endpoint").cloned().unwrap_or_else(|| json!("")),
            "status": source.get("status").cloned().unwrap_or_else(|| json!("failed")),
            "models": models.len(),
            "message": source.get("message").cloned().unwrap_or_else(|| json!("")),
        }),
    );
    (models, source)
}

fn model_request_error_message(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "模型目录请求超时".to_string()
    } else if error.is_connect() {
        "模型目录连接失败".to_string()
    } else {
        "模型目录请求失败".to_string()
    }
}

fn failed_source(mut source: Value, message: String) -> Value {
    source["status"] = json!("failed");
    source["message"] = json!(message);
    source["models"] = json!(0);
    source["responses_api"] = responses_api_status("unknown", "", "");
    source
}

fn responses_api_status(status: &str, endpoint: &str, message: &str) -> Value {
    json!({
        "status": status,
        "endpoint": endpoint,
        "message": message
    })
}

pub async fn fetch_relay_profile_model_ids(
    profile: &RelayProfile,
) -> anyhow::Result<(Vec<String>, String)> {
    let (models, endpoint) = discover_relay_profile_model_ids(profile).await?;
    if !models.is_empty() {
        return Ok((models, endpoint));
    }

    anyhow::bail!("当前 Key 的标准模型目录 {endpoint} 没有返回可用模型")
}

fn relay_profile_model_source(profile: &RelayProfile) -> ModelSource {
    ModelSource {
        source_id: format!("relay-profile:{}", profile.id),
        source_type: "relay_profile".to_string(),
        name: if profile.name.trim().is_empty() {
            profile.id.clone()
        } else {
            profile.name.trim().to_string()
        },
        base_url: if profile.upstream_base_url.trim().is_empty() {
            profile.base_url.trim().to_string()
        } else {
            profile.upstream_base_url.trim().to_string()
        },
        api_key: crate::settings::relay_profile_resolved_api_key(profile),
        // Model discovery is an OpenAI-compatible catalogue request. Keep it
        // independent from the authentication mode used by real Messages traffic.
        anthropic_api_key: false,
        include_anthropic_version: false,
    }
}

/// Performs authenticated model discovery while preserving a successful empty
/// catalogue. Callers that have a narrowly scoped local fallback can therefore
/// distinguish HTTP/auth failures from an upstream that returned no models.
pub async fn discover_relay_profile_model_ids(
    profile: &RelayProfile,
) -> anyhow::Result<(Vec<String>, String)> {
    let source = relay_profile_model_source(profile);
    if source.base_url.is_empty() {
        anyhow::bail!("Base URL 不能为空");
    }
    let client = crate::http_client::proxied_client(&profile.user_agent)?;
    let (models, status) = fetch_models_from_source(&client, &source).await;
    let models_request_succeeded = status.get("status").and_then(Value::as_str) == Some("ok");
    if !models_request_succeeded {
        let message = status
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("上游没有返回可用模型");
        anyhow::bail!("{message}");
    }

    let endpoint = status
        .get("endpoint")
        .and_then(Value::as_str)
        .filter(|endpoint| !endpoint.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| models_endpoint(&source.base_url));
    Ok((models, endpoint))
}

fn preferred_responses_api_status(sources: &[Value]) -> Value {
    let statuses = sources
        .iter()
        .filter_map(|source| source.get("responses_api"))
        .collect::<Vec<_>>();
    for wanted in ["unsupported", "supported", "failed"] {
        if let Some(status) = statuses
            .iter()
            .find(|status| status.get("status").and_then(Value::as_str) == Some(wanted))
        {
            return (*status).clone();
        }
    }
    responses_api_status("unknown", "", "")
}

fn models_endpoint(base_url: &str) -> String {
    build_models_url_candidates(base_url)
        .ok()
        .and_then(|candidates| candidates.into_iter().next())
        .unwrap_or_default()
}

/// Construct deterministic model-directory candidates for a provider Base URL.
///
/// The first candidate follows the current path. A version segment such as
/// `/v4` uses `{base}/models` first, while known compatibility suffixes add
/// root-level `/v1/models` and `/models` fallbacks. Callers should only move to
/// the next candidate for HTTP 404 or 405.
pub fn build_models_url_candidates(base_url: &str) -> anyhow::Result<Vec<String>> {
    let raw = base_url.trim();
    if raw.is_empty() {
        anyhow::bail!("Base URL 不能为空");
    }

    let mut parsed = reqwest::Url::parse(raw).map_err(anyhow::Error::from)?;
    parsed.set_fragment(None);
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);

    let mut cleaned = parsed.path().trim_end_matches('/').to_string();
    let lower = cleaned.to_ascii_lowercase();
    for suffix in ["/chat/completions", "/messages", "/responses"] {
        if lower.ends_with(suffix) {
            cleaned.truncate(cleaned.len() - suffix.len());
            break;
        }
    }
    cleaned = cleaned.trim_end_matches('/').to_string();
    if cleaned.is_empty() && parsed.host_str().is_none() {
        anyhow::bail!("Base URL 不能为空");
    }
    if cleaned.to_ascii_lowercase().ends_with("/models") {
        parsed.set_path(&cleaned);
        return Ok(vec![parsed.to_string()]);
    }

    let candidate_url = |path: &str| {
        let mut candidate = parsed.clone();
        candidate.set_path(path);
        candidate.to_string()
    };
    let mut candidates = Vec::new();
    if ends_with_version_segment(&cleaned) {
        candidates.push(candidate_url(&format!("{cleaned}/models")));
        if !cleaned.to_ascii_lowercase().ends_with("/v1") {
            candidates.push(candidate_url(&format!("{cleaned}/v1/models")));
        }
    } else {
        candidates.push(candidate_url(&format!("{cleaned}/v1/models")));
    }

    if let Some(stripped) = strip_model_compat_suffix(&cleaned) {
        let root = stripped.trim_end_matches('/');
        candidates.push(candidate_url(&format!("{root}/v1/models")));
        candidates.push(candidate_url(&format!("{root}/models")));
    }

    let mut unique = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !unique.iter().any(|existing| existing == &candidate) {
            unique.push(candidate);
        }
    }
    Ok(unique)
}

fn strip_model_compat_suffix(base_url: &str) -> Option<&str> {
    KNOWN_MODEL_COMPAT_SUFFIXES
        .iter()
        .find_map(|suffix| base_url.strip_suffix(suffix))
}

fn ends_with_version_segment(url: &str) -> bool {
    let segment = url.rsplit('/').next().unwrap_or_default();
    let Some(digits) = segment.strip_prefix('v') else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn parse_model_payload(payload: &Value) -> (ModelPayloadResult, Value) {
    let result = match parse_model_payload_value(payload, 0, false) {
        ModelPayloadResult::Models(models) => ModelPayloadResult::Models(unique_strings(models)),
        other => other,
    };
    (result, model_payload_shape(payload, "$", 0))
}

fn parse_model_payload_value(
    value: &Value,
    depth: usize,
    scalar_is_model: bool,
) -> ModelPayloadResult {
    if depth >= MODEL_PAYLOAD_MAX_DEPTH {
        return ModelPayloadResult::Unknown;
    }

    match value {
        Value::String(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                return if scalar_is_model {
                    ModelPayloadResult::Empty
                } else {
                    ModelPayloadResult::Unknown
                };
            }
            if let Ok(decoded) = serde_json::from_str::<Value>(raw) {
                return parse_model_payload_value(&decoded, depth + 1, scalar_is_model);
            }
            if scalar_is_model {
                ModelPayloadResult::Models(vec![raw.to_string()])
            } else {
                ModelPayloadResult::Unknown
            }
        }
        Value::Array(items) => {
            if items.is_empty() {
                return ModelPayloadResult::Empty;
            }

            let mut models = Vec::new();
            let mut saw_empty = false;
            let mut saw_unknown = false;
            let mut saw_business_error = false;
            for item in items {
                match parse_model_payload_value(item, depth + 1, true) {
                    ModelPayloadResult::Models(nested) => models.extend(nested),
                    ModelPayloadResult::Empty => saw_empty = true,
                    ModelPayloadResult::BusinessError => saw_business_error = true,
                    ModelPayloadResult::Unknown => saw_unknown = true,
                }
            }
            if !models.is_empty() {
                ModelPayloadResult::Models(models)
            } else if saw_business_error {
                ModelPayloadResult::BusinessError
            } else if saw_empty && !saw_unknown {
                ModelPayloadResult::Empty
            } else {
                ModelPayloadResult::Unknown
            }
        }
        Value::Object(object) => {
            if object_has_business_error(object) {
                return ModelPayloadResult::BusinessError;
            }

            if let Some(model) = MODEL_PAYLOAD_ID_KEYS.iter().find_map(|key| {
                object
                    .get(*key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|model| !model.is_empty())
            }) {
                return ModelPayloadResult::Models(vec![model.to_string()]);
            }

            let mut found_container = false;
            let mut saw_empty = false;
            let mut saw_unknown = false;
            for key in MODEL_PAYLOAD_CONTAINER_KEYS {
                let Some(nested) = object.get(*key) else {
                    continue;
                };
                found_container = true;
                match parse_model_payload_value(nested, depth + 1, true) {
                    ModelPayloadResult::Models(models) => {
                        return ModelPayloadResult::Models(models);
                    }
                    ModelPayloadResult::BusinessError => {
                        return ModelPayloadResult::BusinessError;
                    }
                    ModelPayloadResult::Empty => saw_empty = true,
                    ModelPayloadResult::Unknown => saw_unknown = true,
                }
            }
            if found_container {
                return if saw_empty && !saw_unknown {
                    ModelPayloadResult::Empty
                } else {
                    ModelPayloadResult::Unknown
                };
            }

            if object.is_empty() {
                return ModelPayloadResult::Unknown;
            }

            let mapped_models = model_ids_from_object_map(object);
            if mapped_models.is_empty() {
                ModelPayloadResult::Unknown
            } else {
                ModelPayloadResult::Models(mapped_models)
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => ModelPayloadResult::Unknown,
    }
}

fn object_has_business_error(object: &serde_json::Map<String, Value>) -> bool {
    if object.get("success").and_then(Value::as_bool) == Some(false)
        || object.get("ok").and_then(Value::as_bool) == Some(false)
    {
        return true;
    }
    if object
        .get("status")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|status| {
            ["error", "failed", "failure"]
                .iter()
                .any(|expected| status.eq_ignore_ascii_case(expected))
        })
    {
        return true;
    }
    MODEL_PAYLOAD_ERROR_KEYS.iter().any(|key| {
        object
            .get(*key)
            .is_some_and(model_payload_error_value_is_present)
    })
}

fn model_payload_error_value_is_present(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.trim().is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
        Value::Bool(value) => *value,
        Value::Number(_) => true,
    }
}

fn model_ids_from_object_map(object: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut models = Vec::new();
    for (key, value) in object {
        if MODEL_PAYLOAD_CONTAINER_KEYS.contains(&key.as_str())
            || MODEL_PAYLOAD_ID_KEYS.contains(&key.as_str())
            || MODEL_PAYLOAD_ERROR_KEYS.contains(&key.as_str())
            || MODEL_MAP_METADATA_KEYS.contains(&key.as_str())
        {
            continue;
        }

        if let Some(model) = value.as_object().and_then(|descriptor| {
            MODEL_PAYLOAD_ID_KEYS.iter().find_map(|id_key| {
                descriptor
                    .get(*id_key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|model| !model.is_empty())
            })
        }) {
            models.push(model.to_string());
            continue;
        }

        if looks_like_model_id(key) && model_map_value_is_descriptor(value) {
            models.push(key.trim().to_string());
        }
    }
    unique_strings(models)
}

fn model_map_value_is_descriptor(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null
    )
}

fn looks_like_model_id(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 256
        || value
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte)))
    {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    value.bytes().any(|byte| b"-_.:/".contains(&byte))
        || [
            "claude",
            "anthropic",
            "opus",
            "sonnet",
            "haiku",
            "fable",
            "gpt",
            "o1",
            "o3",
            "o4",
            "gemini",
            "deepseek",
            "qwen",
            "llama",
            "mistral",
            "grok",
            "command",
        ]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn model_payload_shape(value: &Value, path: &str, depth: usize) -> Value {
    if depth >= MODEL_PAYLOAD_MAX_DEPTH {
        return json!({ "path": path, "kind": value_kind(value), "depth_limited": true });
    }

    match value {
        Value::Object(object) => {
            let fields = object.keys().take(32).cloned().collect::<Vec<_>>();
            let containers = MODEL_PAYLOAD_CONTAINER_KEYS
                .iter()
                .filter_map(|key| {
                    object.get(*key).map(|nested| {
                        model_payload_shape(nested, &format!("{path}.{key}"), depth + 1)
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "path": path,
                "kind": "object",
                "field_count": object.len(),
                "fields": fields,
                "containers": containers,
            })
        }
        Value::Array(items) => json!({
            "path": path,
            "kind": "array",
            "length": items.len(),
            "first": items
                .first()
                .map(|item| model_payload_shape(item, &format!("{path}[0]"), depth + 1))
                .unwrap_or(Value::Null),
        }),
        Value::String(raw) => {
            let decoded = serde_json::from_str::<Value>(raw.trim()).ok();
            json!({
                "path": path,
                "kind": "string",
                "length": raw.len(),
                "json_encoded": decoded.is_some(),
                "decoded": decoded
                    .as_ref()
                    .map(|decoded| model_payload_shape(decoded, path, depth + 1))
                    .unwrap_or(Value::Null),
            })
        }
        _ => json!({ "path": path, "kind": value_kind(value) }),
    }
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn models_from_config_model_catalog_json(
    home: &Path,
    effective: &HashMap<String, String>,
) -> (Vec<String>, Option<Value>) {
    let raw_path = string_value(effective.get("model_catalog_json"));
    if raw_path.is_empty() {
        return (Vec::new(), None);
    }
    let path = resolve_config_path(home, &raw_path);
    let safe_path = path.to_string_lossy().to_string();
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            return (
                Vec::new(),
                Some(json!({
                    "id": "config:model_catalog_json",
                    "type": "model_catalog_json",
                    "name": "Codex model catalog",
                    "path": safe_path,
                    "status": "failed",
                    "message": error.to_string(),
                    "models": 0,
                    "responses_api": responses_api_status("unknown", "", "")
                })),
            );
        }
    };
    let payload = match serde_json::from_str::<Value>(&contents) {
        Ok(payload) => payload,
        Err(error) => {
            return (
                Vec::new(),
                Some(json!({
                    "id": "config:model_catalog_json",
                    "type": "model_catalog_json",
                    "name": "Codex model catalog",
                    "path": safe_path,
                    "status": "failed",
                    "message": error.to_string(),
                    "models": 0,
                    "responses_api": responses_api_status("unknown", "", "")
                })),
            );
        }
    };
    let models = unique_strings(parse_model_catalog_json_models(&payload));
    let count = models.len();
    (
        models,
        Some(json!({
            "id": "config:model_catalog_json",
            "type": "model_catalog_json",
            "name": "Codex model catalog",
            "path": safe_path,
            "status": "ok",
            "models": count,
            "responses_api": responses_api_status("unknown", "", "")
        })),
    )
}

fn models_from_default_model_catalog_json_files(home: &Path) -> (Vec<String>, Vec<Value>) {
    let mut models = Vec::new();
    let mut statuses = Vec::new();
    for filename in ["model-catalog.gpt-5.6.json", "model-catalog.json"] {
        let path = home.join(filename);
        if !path.exists() {
            continue;
        }
        let (file_models, status) = models_from_model_catalog_json_file(&path);
        models.extend(file_models);
        statuses.push(status);
    }
    (unique_strings(models), statuses)
}

fn models_from_model_catalog_json_file(path: &Path) -> (Vec<String>, Value) {
    let safe_path = path.to_string_lossy().to_string();
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) => {
            return (
                Vec::new(),
                json!({
                    "id": format!("file:{}", safe_path),
                    "type": "model_catalog_json",
                    "name": "Codex local model catalog",
                    "path": safe_path,
                    "status": "failed",
                    "message": error.to_string(),
                    "models": 0,
                    "responses_api": responses_api_status("unknown", "", "")
                }),
            );
        }
    };
    let payload = match serde_json::from_str::<Value>(&contents) {
        Ok(payload) => payload,
        Err(error) => {
            return (
                Vec::new(),
                json!({
                    "id": format!("file:{}", safe_path),
                    "type": "model_catalog_json",
                    "name": "Codex local model catalog",
                    "path": safe_path,
                    "status": "failed",
                    "message": error.to_string(),
                    "models": 0,
                    "responses_api": responses_api_status("unknown", "", "")
                }),
            );
        }
    };
    let models = unique_strings(parse_model_catalog_json_models(&payload));
    let count = models.len();
    (
        models,
        json!({
            "id": format!("file:{}", safe_path),
            "type": "model_catalog_json",
            "name": "Codex local model catalog",
            "path": safe_path,
            "status": "ok",
            "models": count,
            "responses_api": responses_api_status("unknown", "", "")
        }),
    )
}

fn resolve_config_path(home: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        home.join(path)
    }
}

fn parse_model_catalog_json_models(payload: &Value) -> Vec<String> {
    let Some(models) = payload.get("models").and_then(Value::as_array) else {
        return Vec::new();
    };
    models
        .iter()
        .filter(|model| catalog_model_visible_in_api(model))
        .filter_map(|model| model.get("slug").and_then(Value::as_str))
        .map(str::trim)
        .filter(|slug| !slug.is_empty())
        .map(str::to_string)
        .collect()
}

fn catalog_model_visible_in_api(model: &Value) -> bool {
    let supported_in_api = model
        .get("supported_in_api")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if !supported_in_api {
        return false;
    }
    let visibility = model
        .get("visibility")
        .and_then(Value::as_str)
        .unwrap_or("list")
        .trim();
    visibility.eq_ignore_ascii_case("list")
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() || !seen.insert(value.to_string()) {
            continue;
        }
        result.push(value.to_string());
    }
    result
}

fn first_env_value(env: &HashMap<String, String>, names: &[&str]) -> String {
    names
        .iter()
        .filter_map(|name| env.get(*name))
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string()
}

fn safe_url_for_status(url: &str) -> String {
    let mut cleaned = url
        .split('?')
        .next()
        .unwrap_or_default()
        .split('#')
        .next()
        .unwrap_or_default()
        .to_string();
    if let Ok(parsed) = reqwest::Url::parse(&cleaned) {
        let host = parsed.host_str().unwrap_or_default();
        let authority = parsed
            .port()
            .map(|port| format!("{host}:{port}"))
            .unwrap_or_else(|| host.to_string());
        cleaned = format!("{}://{}{}", parsed.scheme(), authority, parsed.path());
    }
    cleaned
}

fn trim_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

fn string_value(value: Option<&String>) -> String {
    value
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

fn unquote_toml_string(value: &str) -> String {
    let value = value.trim();
    if let Ok(parsed) = toml::from_str::<toml::Value>(&format!("value = {value}")) {
        if let Some(value) = parsed.get("value").and_then(toml::Value::as_str) {
            return value.to_string();
        }
    }
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
        .to_string()
}
