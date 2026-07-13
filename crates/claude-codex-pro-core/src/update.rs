use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_REPOSITORY: &str = "DamonZS/Claude-Codex-Pro-Tool";
pub const DEFAULT_GITHUB_API_URL: &str =
    "https://api.github.com/repos/DamonZS/Claude-Codex-Pro-Tool/releases/latest";
pub const DEFAULT_LATEST_RELEASE_URL: &str =
    "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/latest";
pub const DEFAULT_LATEST_JSON_URL: &str =
    "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/latest/download/latest.json";

const UPDATE_CHECK_CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
const UPDATE_CHECK_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const UPDATE_CHECK_RETRY_CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const UPDATE_CHECK_RETRY_REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
const UPDATE_RELEASE_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const UPDATE_DOWNLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const UPDATE_DOWNLOAD_REQUEST_TIMEOUT: Duration = Duration::from_secs(30 * 60);

static UPDATE_RELEASE_CACHE: OnceLock<Mutex<Option<CachedRelease>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct CachedRelease {
    cached_at: Instant,
    release: Release,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadProgress {
    pub phase: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<u8>,
}

impl UpdateDownloadProgress {
    pub fn new(phase: impl Into<String>, downloaded_bytes: u64, total_bytes: Option<u64>) -> Self {
        let percent = total_bytes
            .filter(|total| *total > 0)
            .map(|total| ((downloaded_bytes.saturating_mul(100) / total).min(100)) as u8);
        Self {
            phase: phase.into(),
            downloaded_bytes,
            total_bytes,
            percent,
        }
    }
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

pub fn release_from_latest_redirect_url(value: &str) -> anyhow::Result<Release> {
    let url = url::Url::parse(value)
        .map_err(|error| anyhow::anyhow!("GitHub Release 重定向地址非法：{error}"))?;
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        anyhow::bail!("GitHub Release 重定向地址不可信");
    }

    let (owner, repository) = DEFAULT_REPOSITORY
        .split_once('/')
        .expect("DEFAULT_REPOSITORY must contain owner and repository");
    let segments = url
        .path_segments()
        .ok_or_else(|| anyhow::anyhow!("GitHub Release 重定向地址缺少路径"))?
        .collect::<Vec<_>>();
    let valid_path = segments.len() == 5
        && segments[0] == owner
        && segments[1] == repository
        && segments[2] == "releases"
        && segments[3] == "tag";
    if !valid_path {
        anyhow::bail!("GitHub Release 重定向地址不属于固定仓库");
    }

    let version = segments[4];
    if version.is_empty()
        || !version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        anyhow::bail!("GitHub Release Tag 非法：{version}");
    }
    parse_version_tag(version)?;

    let asset_version = version
        .strip_prefix('v')
        .or_else(|| version.strip_prefix('V'))
        .unwrap_or(version);
    let (asset_name, asset_url) = expected_platform_installer_suffix()
        .map(|suffix| {
            let name = format!("claude-codex-pro-{asset_version}{suffix}");
            let download_url = format!(
                "https://github.com/{DEFAULT_REPOSITORY}/releases/download/{version}/{name}"
            );
            (Some(name), Some(download_url))
        })
        .unwrap_or((None, None));
    if let (Some(name), Some(download_url)) = (&asset_name, &asset_url) {
        validate_update_asset(name, download_url)?;
    }

    Ok(Release {
        version: version.to_string(),
        url: url.to_string(),
        body: String::new(),
        asset_name,
        asset_url,
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
    let client = update_http_client(UPDATE_CHECK_CONNECT_TIMEOUT, UPDATE_CHECK_REQUEST_TIMEOUT)?;
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

pub async fn fetch_latest_github_release(api_url: &str) -> anyhow::Result<Release> {
    let client = update_http_client(UPDATE_CHECK_CONNECT_TIMEOUT, UPDATE_CHECK_REQUEST_TIMEOUT)?;
    let payload = client
        .get(api_url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    release_from_github_payload(&payload)
}

pub async fn fetch_latest_redirect_release(latest_release_url: &str) -> anyhow::Result<Release> {
    fetch_latest_redirect_release_with_route(
        latest_release_url,
        UPDATE_CHECK_CONNECT_TIMEOUT,
        UPDATE_CHECK_REQUEST_TIMEOUT,
        false,
    )
    .await
}

pub async fn fetch_latest_redirect_release_direct(
    latest_release_url: &str,
) -> anyhow::Result<Release> {
    fetch_latest_redirect_release_with_route(
        latest_release_url,
        UPDATE_CHECK_CONNECT_TIMEOUT,
        UPDATE_CHECK_REQUEST_TIMEOUT,
        true,
    )
    .await
}

async fn fetch_latest_redirect_release_with_route(
    latest_release_url: &str,
    connect_timeout: Duration,
    request_timeout: Duration,
    direct: bool,
) -> anyhow::Result<Release> {
    let client = if direct {
        update_direct_http_client_with_redirect(
            connect_timeout,
            request_timeout,
            reqwest::redirect::Policy::none(),
        )?
    } else {
        update_http_client_with_redirect(
            connect_timeout,
            request_timeout,
            reqwest::redirect::Policy::none(),
        )?
    };
    fetch_latest_redirect_release_with_client(client, latest_release_url).await
}

async fn fetch_latest_redirect_release_with_client(
    client: reqwest::Client,
    latest_release_url: &str,
) -> anyhow::Result<Release> {
    let response = client.get(latest_release_url).send().await?;
    if !response.status().is_redirection() {
        response.error_for_status()?;
        anyhow::bail!("GitHub latest Release 未返回 Tag 重定向");
    }
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or_else(|| anyhow::anyhow!("GitHub latest Release 重定向缺少 Location"))?
        .to_str()?;
    let redirect_url = url::Url::parse(latest_release_url)?.join(location)?;
    release_from_latest_redirect_url(redirect_url.as_str())
}

pub async fn fetch_current_release() -> anyhow::Result<Release> {
    if let Some(release) = cached_current_release() {
        return Ok(release);
    }

    let release = fetch_current_release_uncached().await?;
    cache_current_release(&release);
    Ok(release)
}

async fn fetch_current_release_uncached() -> anyhow::Result<Release> {
    let github_api = fetch_latest_github_release(DEFAULT_GITHUB_API_URL);
    let latest_redirect = fetch_latest_redirect_release(DEFAULT_LATEST_RELEASE_URL);
    let direct_redirect = fetch_latest_redirect_release_direct(DEFAULT_LATEST_RELEASE_URL);
    let latest_json = fetch_latest_release(DEFAULT_LATEST_JSON_URL);
    tokio::pin!(github_api);
    tokio::pin!(latest_redirect);
    tokio::pin!(direct_redirect);
    tokio::pin!(latest_json);

    let mut github_api_done = false;
    let mut latest_redirect_done = false;
    let mut direct_redirect_done = false;
    let mut latest_json_done = false;
    let mut errors = Vec::new();
    loop {
        tokio::select! {
            result = &mut github_api, if !github_api_done => {
                github_api_done = true;
                match result {
                    Ok(release) => return Ok(release),
                    Err(error) => errors.push(format!("API: {error}")),
                }
            },
            result = &mut latest_redirect, if !latest_redirect_done => {
                latest_redirect_done = true;
                match result {
                    Ok(release) => return Ok(release),
                    Err(error) => errors.push(format!("latest redirect: {error}")),
                }
            },
            result = &mut direct_redirect, if !direct_redirect_done => {
                direct_redirect_done = true;
                match result {
                    Ok(release) => return Ok(release),
                    Err(error) => errors.push(format!("direct latest redirect: {error}")),
                }
            },
            result = &mut latest_json, if !latest_json_done => {
                latest_json_done = true;
                match result {
                    Ok(release) => return Ok(release),
                    Err(error) => errors.push(format!("latest.json: {error}")),
                }
            },
            else => break,
        }
    }

    match retry_latest_redirect_release().await {
        Ok(release) => return Ok(release),
        Err(error) => errors.push(format!("latest redirect retry: {error}")),
    }
    anyhow::bail!("GitHub Release 更新源均不可用：{}", errors.join("; "))
}

async fn retry_latest_redirect_release() -> anyhow::Result<Release> {
    let proxied = fetch_latest_redirect_release_with_route(
        DEFAULT_LATEST_RELEASE_URL,
        UPDATE_CHECK_RETRY_CONNECT_TIMEOUT,
        UPDATE_CHECK_RETRY_REQUEST_TIMEOUT,
        false,
    );
    let direct = fetch_latest_redirect_release_with_route(
        DEFAULT_LATEST_RELEASE_URL,
        UPDATE_CHECK_RETRY_CONNECT_TIMEOUT,
        UPDATE_CHECK_RETRY_REQUEST_TIMEOUT,
        true,
    );
    tokio::pin!(proxied);
    tokio::pin!(direct);

    tokio::select! {
        result = &mut proxied => match result {
            Ok(release) => Ok(release),
            Err(proxied_error) => direct
                .await
                .map_err(|direct_error| anyhow::anyhow!(
                    "system proxy: {proxied_error}; direct: {direct_error}"
                )),
        },
        result = &mut direct => match result {
            Ok(release) => Ok(release),
            Err(direct_error) => proxied
                .await
                .map_err(|proxied_error| anyhow::anyhow!(
                    "direct: {direct_error}; system proxy: {proxied_error}"
                )),
        },
    }
}

fn cached_current_release() -> Option<Release> {
    let cache = UPDATE_RELEASE_CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()?;
    cache
        .as_ref()
        .filter(|cached| cached.cached_at.elapsed() <= UPDATE_RELEASE_CACHE_TTL)
        .map(|cached| cached.release.clone())
}

fn cache_current_release(release: &Release) {
    if let Ok(mut cache) = UPDATE_RELEASE_CACHE.get_or_init(|| Mutex::new(None)).lock() {
        *cache = Some(CachedRelease {
            cached_at: Instant::now(),
            release: release.clone(),
        });
    }
}

pub async fn check_for_update(current_version: &str) -> anyhow::Result<UpdateCheck> {
    let release = fetch_current_release().await?;
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
    perform_update_with_progress(release, download_dir, |_| {}).await
}

pub async fn perform_update_with_progress<F>(
    release: &Release,
    download_dir: &Path,
    mut on_progress: F,
) -> anyhow::Result<UpdateInstall>
where
    F: FnMut(UpdateDownloadProgress),
{
    let url = release
        .asset_url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("没有可下载的 Release asset"))?;
    let name = release
        .asset_name
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("没有可下载的 Release asset"))?;
    validate_update_asset(name, url)?;

    std::fs::create_dir_all(download_dir)?;
    let safe_name = safe_asset_name(name)?;
    let installer_path = download_dir.join(&safe_name);
    let partial_path = download_dir.join(format!("{safe_name}.part"));
    let _ = std::fs::remove_file(&partial_path);

    on_progress(UpdateDownloadProgress::new("connecting", 0, None));
    let response = fetch_update_download_response(url).await?;
    let total_bytes = response.content_length();
    on_progress(UpdateDownloadProgress::new("downloading", 0, total_bytes));

    let download_result = async {
        let mut file = std::fs::File::create(&partial_path)?;
        let mut stream = response.bytes_stream();
        let mut downloaded_bytes = 0_u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            downloaded_bytes = downloaded_bytes.saturating_add(chunk.len() as u64);
            on_progress(UpdateDownloadProgress::new(
                "downloading",
                downloaded_bytes,
                total_bytes,
            ));
        }
        file.flush()?;
        file.sync_all()?;
        Ok::<u64, anyhow::Error>(downloaded_bytes)
    }
    .await;

    let downloaded_bytes = match download_result {
        Ok(downloaded_bytes) => downloaded_bytes,
        Err(error) => {
            let _ = std::fs::remove_file(&partial_path);
            return Err(error);
        }
    };
    if total_bytes.is_some_and(|expected| expected != downloaded_bytes) {
        let _ = std::fs::remove_file(&partial_path);
        anyhow::bail!(
            "安装包下载不完整：应为 {} 字节，实际为 {} 字节",
            total_bytes.unwrap_or_default(),
            downloaded_bytes
        );
    }
    if let Err(error) = replace_downloaded_asset(&partial_path, &installer_path) {
        let _ = std::fs::remove_file(&partial_path);
        return Err(error);
    }
    on_progress(UpdateDownloadProgress::new(
        "launching",
        downloaded_bytes,
        total_bytes.or(Some(downloaded_bytes)),
    ));
    launch_installer(&installer_path)?;
    Ok(UpdateInstall {
        release: release.clone(),
        installer_path,
        launched: true,
    })
}

fn update_http_client(
    connect_timeout: Duration,
    request_timeout: Duration,
) -> anyhow::Result<reqwest::Client> {
    update_http_client_with_redirect(
        connect_timeout,
        request_timeout,
        reqwest::redirect::Policy::limited(10),
    )
}

fn update_http_client_with_redirect(
    connect_timeout: Duration,
    request_timeout: Duration,
    redirect_policy: reqwest::redirect::Policy,
) -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(format!("ClaudeCodexPro/{}", crate::version::VERSION))
        .connect_timeout(connect_timeout)
        .timeout(request_timeout)
        .redirect(redirect_policy)
        .build()?)
}

fn update_direct_http_client_with_redirect(
    connect_timeout: Duration,
    request_timeout: Duration,
    redirect_policy: reqwest::redirect::Policy,
) -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(format!("ClaudeCodexPro/{}", crate::version::VERSION))
        .connect_timeout(connect_timeout)
        .timeout(request_timeout)
        .redirect(redirect_policy)
        .no_proxy()
        .build()?)
}

async fn fetch_update_download_response(url: &str) -> anyhow::Result<reqwest::Response> {
    let proxied_client = update_http_client(
        UPDATE_DOWNLOAD_CONNECT_TIMEOUT,
        UPDATE_DOWNLOAD_REQUEST_TIMEOUT,
    )?;
    let direct_client = update_direct_http_client_with_redirect(
        UPDATE_DOWNLOAD_CONNECT_TIMEOUT,
        UPDATE_DOWNLOAD_REQUEST_TIMEOUT,
        reqwest::redirect::Policy::limited(10),
    )?;
    let proxied = send_update_download_request(proxied_client, url);
    let direct = send_update_download_request(direct_client, url);
    tokio::pin!(proxied);
    tokio::pin!(direct);

    let mut proxied_done = false;
    let mut direct_done = false;
    let mut errors = Vec::new();
    loop {
        tokio::select! {
            result = &mut proxied, if !proxied_done => {
                proxied_done = true;
                match result {
                    Ok(response) => return Ok(response),
                    Err(error) => errors.push(format!("system proxy: {error}")),
                }
            },
            result = &mut direct, if !direct_done => {
                direct_done = true;
                match result {
                    Ok(response) => return Ok(response),
                    Err(error) => errors.push(format!("direct: {error}")),
                }
            },
            else => break,
        }
    }
    anyhow::bail!("安装包下载连接均不可用：{}", errors.join("; "))
}

async fn send_update_download_request(
    client: reqwest::Client,
    url: &str,
) -> anyhow::Result<reqwest::Response> {
    Ok(client.get(url).send().await?.error_for_status()?)
}

#[cfg(windows)]
fn replace_downloaded_asset(partial_path: &Path, installer_path: &Path) -> anyhow::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    use windows::core::PCWSTR;

    let partial = partial_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let installer = installer_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    unsafe {
        MoveFileExW(
            PCWSTR(partial.as_ptr()),
            PCWSTR(installer.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| anyhow::anyhow!("替换已下载的安装包失败：{error}"))?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_downloaded_asset(partial_path: &Path, installer_path: &Path) -> anyhow::Result<()> {
    std::fs::rename(partial_path, installer_path)?;
    Ok(())
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
