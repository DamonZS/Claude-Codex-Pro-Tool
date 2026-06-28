use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

pub const CLAUDE_DESKTOP_PROVIDER_PROFILE_ID: &str = "00000000-0000-4000-8000-000000157210";
pub const CLAUDE_DESKTOP_PROVIDER_PROFILE_NAME: &str = "Claude Codex Pro";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopProviderRequest {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub model_list: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopProviderPreview {
    pub profile_id: String,
    pub profile_name: String,
    pub normal_config_path: String,
    pub threep_config_path: String,
    pub profile_path: String,
    pub meta_path: String,
    pub write_targets: Vec<String>,
    pub config_diff: String,
    pub redacted_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopProviderOutcome {
    pub configured: bool,
    pub normal_config_path: String,
    pub threep_config_path: String,
    pub profile_path: String,
    pub meta_path: String,
    pub backup_paths: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeDesktopProviderPaths {
    pub normal_config_path: PathBuf,
    pub threep_config_path: PathBuf,
    pub config_library_dir: PathBuf,
    pub profile_path: PathBuf,
    pub meta_path: PathBuf,
}

impl ClaudeDesktopProviderPaths {
    pub fn from_single_root(root: &Path) -> Self {
        let normal_config_path = root.join("Claude").join("claude_desktop_config.json");
        let threep_root = root.join("Claude-3p");
        let threep_config_path = threep_root.join("claude_desktop_config.json");
        let config_library_dir = threep_root.join("configLibrary");
        Self::from_explicit(normal_config_path, threep_config_path, config_library_dir)
    }

    pub fn from_explicit(
        normal_config_path: PathBuf,
        threep_config_path: PathBuf,
        config_library_dir: PathBuf,
    ) -> Self {
        let profile_path =
            config_library_dir.join(format!("{CLAUDE_DESKTOP_PROVIDER_PROFILE_ID}.json"));
        let meta_path = config_library_dir.join("_meta.json");
        Self {
            normal_config_path,
            threep_config_path,
            config_library_dir,
            profile_path,
            meta_path,
        }
    }
}

#[derive(Debug, Clone)]
struct FileSnapshot {
    path: PathBuf,
    content: Option<Vec<u8>>,
}

pub fn default_claude_desktop_provider_paths() -> ClaudeDesktopProviderPaths {
    let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    if cfg!(windows) {
        let appdata = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let local_appdata = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        return ClaudeDesktopProviderPaths::from_explicit(
            appdata.join("Claude").join("claude_desktop_config.json"),
            local_appdata
                .join("Claude-3p")
                .join("claude_desktop_config.json"),
            local_appdata.join("Claude-3p").join("configLibrary"),
        );
    }
    if cfg!(target_os = "macos") {
        let home = home.unwrap_or_else(|| PathBuf::from("."));
        return ClaudeDesktopProviderPaths::from_explicit(
            home.join("Library")
                .join("Application Support")
                .join("Claude")
                .join("claude_desktop_config.json"),
            home.join("Library")
                .join("Application Support")
                .join("Claude-3p")
                .join("claude_desktop_config.json"),
            home.join("Library")
                .join("Application Support")
                .join("Claude-3p")
                .join("configLibrary"),
        );
    }

    let home = home.unwrap_or_else(|| PathBuf::from("."));
    ClaudeDesktopProviderPaths::from_explicit(
        home.join(".config")
            .join("Claude")
            .join("claude_desktop_config.json"),
        home.join(".config")
            .join("Claude-3p")
            .join("claude_desktop_config.json"),
        home.join(".config").join("Claude-3p").join("configLibrary"),
    )
}

pub fn preview_claude_desktop_provider(
    request: &ClaudeDesktopProviderRequest,
) -> anyhow::Result<ClaudeDesktopProviderPreview> {
    preview_claude_desktop_provider_at_paths(&default_claude_desktop_provider_paths(), request)
}

pub fn preview_claude_desktop_provider_with_proxy_port(
    request: &ClaudeDesktopProviderRequest,
    proxy_port: u16,
) -> anyhow::Result<ClaudeDesktopProviderPreview> {
    preview_claude_desktop_provider_at_paths_with_proxy_port(
        &default_claude_desktop_provider_paths(),
        request,
        proxy_port,
    )
}

pub fn preview_claude_desktop_provider_at_paths(
    paths: &ClaudeDesktopProviderPaths,
    request: &ClaudeDesktopProviderRequest,
) -> anyhow::Result<ClaudeDesktopProviderPreview> {
    preview_claude_desktop_provider_at_paths_with_proxy_port(
        paths,
        request,
        crate::protocol_proxy::DEFAULT_CLAUDE_DESKTOP_PROXY_PORT,
    )
}

pub fn preview_claude_desktop_provider_at_paths_with_proxy_port(
    paths: &ClaudeDesktopProviderPaths,
    request: &ClaudeDesktopProviderRequest,
    proxy_port: u16,
) -> anyhow::Result<ClaudeDesktopProviderPreview> {
    validate_request(request)?;
    let profile_name = display_provider_name(request);
    let profile = build_gateway_profile(request, proxy_port);
    let redacted_profile = redact_profile(profile.clone());
    let redacted_profile_text =
        serde_json::to_string_pretty(&redacted_profile).context("serialize redacted profile")?;
    let config_diff = format!(
        "Claude Desktop normal config:\n  {} -> deploymentMode = 3p\n\nClaude Desktop 3P config:\n  {} -> deploymentMode = 3p\n\nProfile:\n  {} -> {}\n\n{}",
        paths.normal_config_path.display(),
        paths.threep_config_path.display(),
        paths.profile_path.display(),
        profile_name,
        redacted_profile_text
    );
    Ok(ClaudeDesktopProviderPreview {
        profile_id: CLAUDE_DESKTOP_PROVIDER_PROFILE_ID.to_string(),
        profile_name,
        normal_config_path: path_string(&paths.normal_config_path),
        threep_config_path: path_string(&paths.threep_config_path),
        profile_path: path_string(&paths.profile_path),
        meta_path: path_string(&paths.meta_path),
        write_targets: write_targets(paths),
        config_diff,
        redacted_profile: redacted_profile_text,
    })
}

pub fn apply_claude_desktop_provider(
    request: &ClaudeDesktopProviderRequest,
) -> anyhow::Result<ClaudeDesktopProviderOutcome> {
    apply_claude_desktop_provider_at_paths(&default_claude_desktop_provider_paths(), request)
}

pub fn apply_claude_desktop_provider_with_proxy_port(
    request: &ClaudeDesktopProviderRequest,
    proxy_port: u16,
) -> anyhow::Result<ClaudeDesktopProviderOutcome> {
    apply_claude_desktop_provider_at_paths_with_proxy_port(
        &default_claude_desktop_provider_paths(),
        request,
        proxy_port,
    )
}

pub fn apply_claude_desktop_provider_at_paths(
    paths: &ClaudeDesktopProviderPaths,
    request: &ClaudeDesktopProviderRequest,
) -> anyhow::Result<ClaudeDesktopProviderOutcome> {
    apply_claude_desktop_provider_at_paths_with_proxy_port(
        paths,
        request,
        crate::protocol_proxy::DEFAULT_CLAUDE_DESKTOP_PROXY_PORT,
    )
}

pub fn apply_claude_desktop_provider_at_paths_with_proxy_port(
    paths: &ClaudeDesktopProviderPaths,
    request: &ClaudeDesktopProviderRequest,
    proxy_port: u16,
) -> anyhow::Result<ClaudeDesktopProviderOutcome> {
    validate_request(request)?;
    let profile_name = display_provider_name(request);
    let profile = build_gateway_profile(request, proxy_port);
    let snapshots = snapshot_files(paths)?;
    let backup_paths = backup_existing_files(paths)?;
    let result = (|| {
        write_deployment_mode(&paths.normal_config_path, "3p")?;
        write_deployment_mode(&paths.threep_config_path, "3p")?;
        write_json(&paths.profile_path, &profile)?;
        write_meta(
            &paths.meta_path,
            Some(CLAUDE_DESKTOP_PROVIDER_PROFILE_ID),
            Some(&profile_name),
        )?;
        Ok::<(), anyhow::Error>(())
    })();

    if let Err(error) = result {
        restore_snapshots(&snapshots).with_context(|| {
            format!("Claude Desktop provider write failed and rollback failed: {error}")
        })?;
        return Err(error);
    }

    Ok(ClaudeDesktopProviderOutcome {
        configured: true,
        normal_config_path: path_string(&paths.normal_config_path),
        threep_config_path: path_string(&paths.threep_config_path),
        profile_path: path_string(&paths.profile_path),
        meta_path: path_string(&paths.meta_path),
        backup_paths,
        message: "Claude Desktop 开发模式供应商已写入。请完全退出并重启 Claude Desktop。"
            .to_string(),
    })
}

pub fn restore_claude_desktop_provider_official() -> anyhow::Result<ClaudeDesktopProviderOutcome> {
    restore_claude_desktop_provider_official_at_paths(&default_claude_desktop_provider_paths())
}

pub fn restore_claude_desktop_provider_official_at_paths(
    paths: &ClaudeDesktopProviderPaths,
) -> anyhow::Result<ClaudeDesktopProviderOutcome> {
    let snapshots = snapshot_files(paths)?;
    let backup_paths = backup_existing_files(paths)?;
    let result = (|| {
        write_deployment_mode(&paths.normal_config_path, "1p")?;
        write_deployment_mode(&paths.threep_config_path, "1p")?;
        remove_file_if_exists(&paths.profile_path)?;
        write_meta(&paths.meta_path, None, None)?;
        Ok::<(), anyhow::Error>(())
    })();

    if let Err(error) = result {
        restore_snapshots(&snapshots).with_context(|| {
            format!("Claude Desktop provider restore failed and rollback failed: {error}")
        })?;
        return Err(error);
    }

    Ok(ClaudeDesktopProviderOutcome {
        configured: false,
        normal_config_path: path_string(&paths.normal_config_path),
        threep_config_path: path_string(&paths.threep_config_path),
        profile_path: path_string(&paths.profile_path),
        meta_path: path_string(&paths.meta_path),
        backup_paths,
        message: "Claude Desktop 已切回官方部署模式。请完全退出并重启 Claude Desktop。".to_string(),
    })
}

fn validate_request(request: &ClaudeDesktopProviderRequest) -> anyhow::Result<()> {
    let base_url = request.base_url.trim();
    let parsed = Url::parse(base_url)
        .with_context(|| format!("Claude Desktop 供应商 Base URL 无效：{}", base_url))?;
    match parsed.scheme() {
        "https" => {}
        "http"
            if parsed
                .host_str()
                .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1")) => {}
        "http" => {
            bail!(
                "Claude Desktop 供应商 Base URL 仅允许 https://，或本机 http://localhost / 127.0.0.1 / [::1]。"
            )
        }
        _ => {
            bail!(
                "Claude Desktop 供应商 Base URL 仅允许 https://，或本机 http://localhost / 127.0.0.1 / [::1]。"
            )
        }
    }
    if request.api_key.trim().is_empty() {
        bail!("Claude Desktop 供应商 API Key 不能为空。");
    }
    Ok(())
}

fn build_gateway_profile(request: &ClaudeDesktopProviderRequest, proxy_port: u16) -> Value {
    let mut profile = json!({
        "coworkEgressAllowedHosts": ["*"],
        "disableDeploymentModeChooser": true,
        "inferenceGatewayApiKey": request.api_key.trim(),
        "inferenceGatewayAuthScheme": "bearer",
        "inferenceGatewayBaseUrl": crate::protocol_proxy::local_claude_desktop_proxy_base_url(
            proxy_port
        ),
        "inferenceProvider": "gateway"
    });

    profile["inferenceModels"] = Value::Array(
        crate::protocol_proxy::claude_desktop_safe_models_with_labels(&request.model_list),
    );
    profile
}

fn redact_profile(mut profile: Value) -> Value {
    if let Some(object) = profile.as_object_mut() {
        if object.contains_key("inferenceGatewayApiKey") {
            object.insert(
                "inferenceGatewayApiKey".to_string(),
                Value::String("***redacted***".to_string()),
            );
        }
    }
    profile
}

fn display_provider_name(request: &ClaudeDesktopProviderRequest) -> String {
    let name = request.name.trim();
    if name.is_empty() {
        CLAUDE_DESKTOP_PROVIDER_PROFILE_NAME.to_string()
    } else {
        name.to_string()
    }
}

fn write_targets(paths: &ClaudeDesktopProviderPaths) -> Vec<String> {
    [
        &paths.normal_config_path,
        &paths.threep_config_path,
        &paths.profile_path,
        &paths.meta_path,
    ]
    .into_iter()
    .map(|path| path_string(path))
    .collect()
}

fn write_deployment_mode(path: &Path, mode: &str) -> anyhow::Result<()> {
    let mut value = read_json_object_or_empty(path)?;
    value["deploymentMode"] = Value::String(mode.to_string());
    write_json(path, &value)
}

fn write_meta(
    path: &Path,
    applied_profile_id: Option<&str>,
    profile_name: Option<&str>,
) -> anyhow::Result<()> {
    let mut value = read_json_object_or_empty(path)?;
    let object = value.as_object_mut().expect("object was just normalized");
    let mut entries = object
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    entries.retain(|entry| {
        entry.get("id").and_then(Value::as_str) != Some(CLAUDE_DESKTOP_PROVIDER_PROFILE_ID)
    });

    if let Some(id) = applied_profile_id {
        entries.push(json!({
            "id": CLAUDE_DESKTOP_PROVIDER_PROFILE_ID,
            "name": profile_name
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .unwrap_or(CLAUDE_DESKTOP_PROVIDER_PROFILE_NAME)
        }));
        object.insert("appliedId".to_string(), Value::String(id.to_string()));
    } else if object
        .get("appliedId")
        .and_then(Value::as_str)
        .is_some_and(|id| id == CLAUDE_DESKTOP_PROVIDER_PROFILE_ID)
    {
        object.remove("appliedId");
    }

    object.insert("entries".to_string(), Value::Array(entries));
    write_json(path, &value)
}

fn read_json_object_or_empty(path: &Path) -> anyhow::Result<Value> {
    read_json_object_or_empty_recovering(path)
}

fn read_json_object_or_empty_recovering(path: &Path) -> anyhow::Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    if let Some(value) = recover_invalid_json_object(path)? {
        return Ok(value);
    }
    let value: Value = serde_json::from_str(strip_json_bom(&fs::read_to_string(path)?))
        .with_context(|| format!("读取 JSON 失败：{}", path.display()))?;
    if value.is_object() {
        Ok(value)
    } else {
        Ok(json!({}))
    }
}

fn write_json(path: &Path, value: &Value) -> anyhow::Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    crate::settings::atomic_write(path, text.as_bytes())?;
    let written = fs::read_to_string(path)
        .with_context(|| format!("read JSON file after write failed: {}", path.display()))?;
    serde_json::from_str::<Value>(strip_json_bom(&written))
        .with_context(|| format!("written JSON validation failed: {}", path.display()))?;
    Ok(())
}

fn recover_invalid_json_object(path: &Path) -> anyhow::Result<Option<Value>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read JSON file failed: {}", path.display()))?;
    if serde_json::from_str::<Value>(strip_json_bom(&raw)).is_ok() {
        return Ok(None);
    }
    backup_invalid_json_file(path)?;
    Ok(Some(json!({})))
}

fn strip_json_bom(raw: &str) -> &str {
    raw.strip_prefix('\u{feff}').unwrap_or(raw)
}

fn backup_invalid_json_file(path: &Path) -> anyhow::Result<()> {
    let backup_path = path.with_extension(format!(
        "{}.invalid.{}",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("json"),
        timestamp_millis()
    ));
    fs::copy(path, &backup_path).with_context(|| {
        format!(
            "backup invalid JSON {} to {} failed",
            path.display(),
            backup_path.display()
        )
    })?;
    Ok(())
}

fn snapshot_files(paths: &ClaudeDesktopProviderPaths) -> anyhow::Result<Vec<FileSnapshot>> {
    write_targets(paths)
        .into_iter()
        .map(PathBuf::from)
        .map(|path| {
            let content = if path.exists() {
                Some(fs::read(&path).with_context(|| format!("读取快照失败：{}", path.display()))?)
            } else {
                None
            };
            Ok(FileSnapshot { path, content })
        })
        .collect()
}

fn restore_snapshots(snapshots: &[FileSnapshot]) -> anyhow::Result<()> {
    for snapshot in snapshots {
        match &snapshot.content {
            Some(content) => crate::settings::atomic_write(&snapshot.path, content)?,
            None => remove_file_if_exists(&snapshot.path)?,
        }
    }
    Ok(())
}

fn backup_existing_files(paths: &ClaudeDesktopProviderPaths) -> anyhow::Result<Vec<String>> {
    let backup_dir = paths
        .threep_config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups")
        .join(format!("claude-codex-pro-provider-{}", timestamp_millis()));
    let mut backups = Vec::new();
    for path in [
        &paths.normal_config_path,
        &paths.threep_config_path,
        &paths.profile_path,
        &paths.meta_path,
    ] {
        if !path.exists() {
            continue;
        }
        let relative = backup_relative_name(path);
        let target = backup_dir.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(path, &target)
            .with_context(|| format!("备份 {} 到 {} 失败", path.display(), target.display()))?;
        backups.push(path_string(&target));
    }
    Ok(backups)
}

fn backup_relative_name(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config.json"));
    if path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        == Some("configLibrary")
    {
        PathBuf::from("configLibrary").join(file_name)
    } else if path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        == Some("Claude")
    {
        PathBuf::from("Claude").join(file_name)
    } else {
        PathBuf::from("Claude-3p").join(file_name)
    }
}

fn remove_file_if_exists(path: &Path) -> anyhow::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("删除 {} 失败", path.display())),
    }
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
