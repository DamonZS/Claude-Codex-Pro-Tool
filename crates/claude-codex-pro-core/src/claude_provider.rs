use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use serde::Serialize;
use serde_json::{Map, Value};
use url::Url;

use crate::settings::{RelayProfile, atomic_write, relay_profile_resolved_api_key};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeProviderOutcome {
    pub settings_path: String,
    pub backup_path: Option<String>,
    pub merged_env_keys: Vec<String>,
}

pub fn default_claude_settings_path() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().join(".claude").join("settings.json"))
        .unwrap_or_else(|| PathBuf::from(".claude").join("settings.json"))
}

pub fn apply_claude_provider(profile: &RelayProfile) -> anyhow::Result<ClaudeProviderOutcome> {
    apply_claude_provider_at_path(&default_claude_settings_path(), profile)
}

/// 走本机模型代理时用这个：命令行这只 Claude 的地址指向本机代理，真实上游仍留在供应商档案的
/// JSON env 里，由代理自己解析 —— 这正是 Claude Desktop 那条路一直在用的约定。
pub fn apply_claude_provider_via_proxy(
    profile: &RelayProfile,
    proxy_base_url: &str,
) -> anyhow::Result<ClaudeProviderOutcome> {
    apply_claude_provider_at_path_using(
        &default_claude_settings_path(),
        profile,
        Some(proxy_base_url),
    )
}

pub fn apply_claude_provider_at_path(
    settings_path: &Path,
    profile: &RelayProfile,
) -> anyhow::Result<ClaudeProviderOutcome> {
    apply_claude_provider_at_path_using(settings_path, profile, None)
}

fn apply_claude_provider_at_path_using(
    settings_path: &Path,
    profile: &RelayProfile,
    proxy_base_url: Option<&str>,
) -> anyhow::Result<ClaudeProviderOutcome> {
    let mut desired_env = profile_env(profile)?;
    let base_url = match proxy_base_url.map(str::trim).filter(|url| !url.is_empty()) {
        Some(url) => url.to_string(),
        None => resolve_base_url(profile, &desired_env)?,
    };
    validate_base_url(&base_url)?;
    let api_key = relay_profile_resolved_api_key(profile);
    if api_key.trim().is_empty() {
        bail!("Claude 供应商 API Key 不能为空。");
    }

    desired_env.insert("ANTHROPIC_BASE_URL".to_string(), Value::String(base_url));
    let auth_field = preferred_auth_field(profile, &desired_env);
    desired_env.remove("ANTHROPIC_AUTH_TOKEN");
    desired_env.remove("ANTHROPIC_API_KEY");
    desired_env.insert(auth_field.to_string(), Value::String(api_key));
    if !desired_env.contains_key("ANTHROPIC_MODEL") && !profile.model.trim().is_empty() {
        desired_env.insert(
            "ANTHROPIC_MODEL".to_string(),
            Value::String(profile.model.trim().to_string()),
        );
    }

    let mut settings = read_settings_object(settings_path)?;
    let env = settings
        .entry("env".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let env = env
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude settings.json 的 env 必须是 JSON 对象"))?;
    env.remove("ANTHROPIC_AUTH_TOKEN");
    env.remove("ANTHROPIC_API_KEY");
    let mut merged_env_keys = Vec::new();
    for (key, value) in desired_env {
        if value.as_str().is_some_and(|value| value.trim().is_empty()) {
            continue;
        }
        merged_env_keys.push(key.clone());
        env.insert(key, value);
    }
    merged_env_keys.sort();
    merged_env_keys.dedup();

    let backup_path = backup_existing_settings(settings_path)?;
    let bytes = serde_json::to_vec_pretty(&Value::Object(settings))?;
    atomic_write(settings_path, &bytes).with_context(|| {
        format!(
            "写入 Claude settings.json 失败：{}",
            settings_path.display()
        )
    })?;

    Ok(ClaudeProviderOutcome {
        settings_path: settings_path.to_string_lossy().to_string(),
        backup_path: backup_path.map(|path| path.to_string_lossy().to_string()),
        merged_env_keys,
    })
}

fn profile_env(profile: &RelayProfile) -> anyhow::Result<Map<String, Value>> {
    let contents = profile
        .config_contents
        .trim()
        .trim_start_matches('\u{feff}');
    if contents.is_empty() || !contents.starts_with('{') {
        return Ok(Map::new());
    }
    let value: Value = serde_json::from_str(contents)
        .with_context(|| "Claude 供应商 configContents JSON 解析失败")?;
    Ok(value
        .get("env")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default())
}

fn resolve_base_url(profile: &RelayProfile, env: &Map<String, Value>) -> anyhow::Result<String> {
    for value in [
        env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str),
        env.get("CLAUDE_BASE_URL").and_then(Value::as_str),
        Some(profile.upstream_base_url.as_str()),
        Some(profile.base_url.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        let value = value.trim();
        if !value.is_empty() {
            return Ok(value.to_string());
        }
    }
    bail!("Claude 供应商 Base URL 不能为空。")
}

fn validate_base_url(base_url: &str) -> anyhow::Result<()> {
    let parsed =
        Url::parse(base_url).with_context(|| format!("Claude 供应商 Base URL 无效：{base_url}"))?;
    let localhost = parsed
        .host_str()
        .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
    if parsed.scheme() == "https" || (parsed.scheme() == "http" && localhost) {
        return Ok(());
    }
    bail!("Claude 供应商 Base URL 仅允许 https://，或本机 http:// 地址。")
}

fn preferred_auth_field<'a>(profile: &'a RelayProfile, env: &Map<String, Value>) -> &'a str {
    match profile.auth_field.trim() {
        "ANTHROPIC_API_KEY" => "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN" => "ANTHROPIC_AUTH_TOKEN",
        _ if env.contains_key("ANTHROPIC_API_KEY") && !env.contains_key("ANTHROPIC_AUTH_TOKEN") => {
            "ANTHROPIC_API_KEY"
        }
        _ => "ANTHROPIC_AUTH_TOKEN",
    }
}

fn read_settings_object(path: &Path) -> anyhow::Result<Map<String, Value>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("读取 Claude settings.json 失败：{}", path.display()));
        }
    };
    let text = String::from_utf8(bytes)
        .with_context(|| format!("Claude settings.json 不是 UTF-8：{}", path.display()))?;
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .with_context(|| format!("Claude settings.json JSON 解析失败：{}", path.display()))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Claude settings.json 必须是 JSON 对象"))
}

fn backup_existing_settings(path: &Path) -> anyhow::Result<Option<PathBuf>> {
    if !path.is_file() {
        return Ok(None);
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("settings.json");
    let backup_path = path.with_file_name(format!("{file_name}.bak.{timestamp}"));
    fs::copy(path, &backup_path).with_context(|| {
        format!(
            "备份 Claude settings.json 失败：{} -> {}",
            path.display(),
            backup_path.display()
        )
    })?;
    Ok(Some(backup_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn profile() -> RelayProfile {
        let mut profile = RelayProfile::default();
        profile.name = "My Claude".to_string();
        profile.target_app = "claude".to_string();
        profile.base_url = "https://api.example.com".to_string();
        profile.api_key = "sk-test".to_string();
        profile.api_key_explicit = true;
        profile
    }

    fn scratch(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("ccp-claude-provider-{name}-{stamp}.json"))
    }

    fn written_base_url(path: &Path) -> String {
        let text = std::fs::read_to_string(path).expect("写入后的 settings.json 应该存在");
        let value: Value = serde_json::from_str(&text).expect("写入后的 settings.json 应该能解析");
        value
            .get("env")
            .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn without_a_proxy_the_address_is_the_supplier() {
        let path = scratch("direct");
        apply_claude_provider_at_path(&path, &profile()).expect("直连写入应该成功");
        assert_eq!(written_base_url(&path), "https://api.example.com");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn with_a_proxy_the_address_is_the_proxy() {
        let path = scratch("proxy");
        apply_claude_provider_at_path_using(
            &path,
            &profile(),
            Some("http://127.0.0.1:57331/claude-desktop"),
        )
        .expect("走代理写入应该成功");
        assert_eq!(
            written_base_url(&path),
            "http://127.0.0.1:57331/claude-desktop"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_empty_proxy_falls_back_to_the_supplier() {
        let path = scratch("blank-proxy");
        apply_claude_provider_at_path_using(&path, &profile(), Some("   "))
            .expect("空白代理地址应退回直连");
        assert_eq!(written_base_url(&path), "https://api.example.com");
        let _ = std::fs::remove_file(&path);
    }
}
