use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_REPOSITORY: &str = "DamonZS/Claude-Codex-Pro-Tool";
pub const DEFAULT_LATEST_JSON_URL: &str =
    "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/latest/download/latest.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub body: String,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateCheck {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_summary: String,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub update_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateInstall {
    pub release: Release,
    pub installer_path: PathBuf,
    pub launched: bool,
}

pub fn parse_version_tag(value: &str) -> anyhow::Result<Vec<u64>> {
    let normalized = value.trim().trim_start_matches(['v', 'V']);
    let mut digits = String::new();
    for ch in normalized.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            digits.push(ch);
        } else {
            break;
        }
    }
    if digits.is_empty() {
        anyhow::bail!("Invalid version tag: {value}");
    }
    digits
        .split('.')
        .map(|part| part.parse::<u64>().map_err(Into::into))
        .collect()
}

pub fn is_newer_version(candidate: &str, current: &str) -> anyhow::Result<bool> {
    let candidate_auto_release = parse_auto_release_counter(candidate);
    let current_auto_release = parse_auto_release_counter(current);
    match (candidate_auto_release, current_auto_release) {
        (Some(candidate), Some(current)) => return Ok(candidate > current),
        (Some(_), None) => return Ok(true),
        (None, Some(_)) => return Ok(false),
        (None, None) => {}
    }

    let mut left = parse_version_tag(candidate)?;
    let mut right = parse_version_tag(current)?;
    let len = left.len().max(right.len());
    left.resize(len, 0);
    right.resize(len, 0);
    Ok(left > right)
}

fn parse_auto_release_counter(value: &str) -> Option<u64> {
    let value = value.trim();
    let release = value
        .strip_prefix('V')
        .or_else(|| value.strip_prefix('v'))?;
    let (major, minor) = release.split_once('.')?;
    if minor.len() != 2 || !major.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    if !minor.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some(major.parse::<u64>().ok()? * 100 + minor.parse::<u64>().ok()?)
}

pub fn release_from_github_payload(payload: &Value) -> anyhow::Result<Release> {
    let version = payload
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("release payload missing tag_name"))?
        .to_string();
    let assets = payload
        .get("assets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|asset| {
            Some((
                asset.get("name")?.as_str()?.to_string(),
                asset.get("browser_download_url")?.as_str()?.to_string(),
            ))
        })
        .collect::<Vec<_>>();
    let selected = select_update_asset(&assets);
    Ok(Release {
        version,
        url: payload
            .get("html_url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        body: payload
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        asset_name: selected.as_ref().map(|asset| asset.name.clone()),
        asset_url: selected.map(|asset| asset.browser_download_url),
    })
}

pub fn release_from_latest_json_payload(payload: &Value) -> anyhow::Result<Release> {
    let version = payload
        .get("version")
        .or_else(|| payload.get("tag_name"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("latest.json missing version"))?
        .to_string();
    let assets = payload
        .get("assets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|asset| {
            let name = asset.get("name")?.as_str()?.to_string();
            let url = asset
                .get("url")
                .or_else(|| asset.get("browser_download_url"))?
                .as_str()?
                .to_string();
            Some((name, url))
        })
        .collect::<Vec<_>>();
    let selected = select_update_asset(&assets);
    Ok(Release {
        version,
        url: payload
            .get("url")
            .or_else(|| payload.get("html_url"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        body: payload
            .get("body")
            .or_else(|| payload.get("release_summary"))
            .or_else(|| payload.get("notes"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        asset_name: selected.as_ref().map(|asset| asset.name.clone()),
        asset_url: selected.map(|asset| asset.browser_download_url),
    })
}

pub fn select_update_asset(assets: &[(String, String)]) -> Option<ReleaseAsset> {
    for (name, url) in assets {
        if validate_update_asset(name, url).is_ok() {
            return Some(ReleaseAsset {
                name: name.clone(),
                browser_download_url: url.clone(),
            });
        }
    }
    None
}

pub async fn fetch_latest_release(latest_json_url: &str) -> anyhow::Result<Release> {
    let client =
        crate::http_client::proxied_client(&format!("ClaudeCodexPro/{}", crate::version::VERSION))?;
    let payload = client
        .get(latest_json_url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    release_from_latest_json_payload(&payload)
}

pub async fn check_for_update(current_version: &str) -> anyhow::Result<UpdateCheck> {
    let release = fetch_latest_release(DEFAULT_LATEST_JSON_URL).await?;
    let update_available = is_newer_version(&release.version, current_version)?;
    Ok(UpdateCheck {
        current_version: current_version.to_string(),
        latest_version: Some(release.version),
        release_summary: release.body,
        asset_name: release.asset_name,
        asset_url: release.asset_url,
        update_available,
    })
}

pub async fn perform_update(
    release: &Release,
    download_dir: &Path,
) -> anyhow::Result<UpdateInstall> {
    let url = release
        .asset_url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("没有可下载的 Release asset"))?;
    let name = release
        .asset_name
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("没有可下载的 Release asset"))?;
    validate_update_asset(name, url)?;
    let bytes =
        crate::http_client::proxied_client(&format!("ClaudeCodexPro/{}", crate::version::VERSION))?
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
    let installer_path = download_asset_to(release, &bytes, download_dir)?;
    launch_installer(&installer_path)?;
    Ok(UpdateInstall {
        release: release.clone(),
        installer_path,
        launched: true,
    })
}

pub fn download_asset_to(
    release: &Release,
    bytes: &[u8],
    download_dir: &Path,
) -> anyhow::Result<PathBuf> {
    let name = release
        .asset_name
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("没有可下载的 Release asset"))?;
    let safe = safe_asset_name(name)?;
    std::fs::create_dir_all(download_dir)?;
    let path = download_dir.join(safe);
    std::fs::write(&path, bytes)?;
    Ok(path)
}

pub fn safe_asset_name(name: &str) -> anyhow::Result<String> {
    if name.trim().is_empty() {
        anyhow::bail!("非法 Release asset 文件名: {name}");
    }
    let path = Path::new(name);
    if path.components().count() != 1 {
        anyhow::bail!("非法 Release asset 文件名: {name}");
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("非法 Release asset 文件名: {name}"))?;
    if file_name == "." || file_name == ".." {
        anyhow::bail!("非法 Release asset 文件名: {name}");
    }
    Ok(file_name.to_string())
}

pub fn validate_update_asset(name: &str, value: &str) -> anyhow::Result<()> {
    if !is_expected_platform_installer(name) {
        anyhow::bail!("Release asset 不匹配当前平台安装器：{name}");
    }

    let url = url::Url::parse(value)
        .map_err(|error| anyhow::anyhow!("Release asset URL 非法：{error}"))?;
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        anyhow::bail!("Release asset URL 必须使用 github.com HTTPS 下载地址");
    }

    let (owner, repository) = DEFAULT_REPOSITORY
        .split_once('/')
        .expect("DEFAULT_REPOSITORY must contain owner and repository");
    let segments = url
        .path_segments()
        .ok_or_else(|| anyhow::anyhow!("Release asset URL 缺少路径"))?
        .collect::<Vec<_>>();
    let valid_path = segments.len() == 6
        && segments[0] == owner
        && segments[1] == repository
        && segments[2] == "releases"
        && segments[3] == "download"
        && !segments[4].is_empty()
        && segments[5] == name;
    if !valid_path {
        anyhow::bail!("Release asset URL 不属于固定仓库下载路径");
    }
    Ok(())
}

fn is_expected_platform_installer(name: &str) -> bool {
    let Some(suffix) = expected_platform_installer_suffix() else {
        return false;
    };
    name.strip_prefix("claude-codex-pro-")
        .and_then(|value| value.strip_suffix(suffix))
        .is_some_and(|version| !version.is_empty())
}

fn expected_platform_installer_suffix() -> Option<&'static str> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        return Some("-windows-x64-setup.exe");
    }
    if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        return Some("-macos-x64.dmg");
    }
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Some("-macos-arm64.dmg");
    }
    None
}

pub fn launch_installer(path: &Path) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        crate::windows_open_path(path).map_err(|error| anyhow::anyhow!("启动安装包失败：{error}"))
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| anyhow::anyhow!("打开 DMG 失败：{error}"))
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = path;
        anyhow::bail!("当前平台不支持启动安装包")
    }
}
