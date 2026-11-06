//! WebDAV 配置备份同步。
//!
//! 最小切片：把一份已经序列化好的配置备份字节流 PUT 到 WebDAV 目录，并
//! 按调用方给定的文件名 GET 回来。不做 PROPFIND，也不遍历目录——"最新一份"
//! 由调用方用 [`backup_file_name`] 生成的、按 UTC 时间自然排序的文件名决定。

use std::time::Duration;

use anyhow::{Context, anyhow, bail};

use crate::http_client::{proxied_client_for, url_target_is_loopback};

/// 同步功能标识，用于日志与错误文案。
pub const SYNC_FEATURE: &str = "webdav_sync";

/// Single-request timeout. WebDAV servers behind a NAS/relay can accept the
/// connection and never answer; 20s bounds the UI-visible wait without the
/// 60s default that ordinary API calls use.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// User agent for both PUT and GET; the shared client prepends nothing else.
const USER_AGENT: &str = "ClaudeCodexPro-WebdavSync";

/// Backups are opaque bytes (a JSON envelope), so no sniffing is wanted.
const BACKUP_CONTENT_TYPE: &str = "application/octet-stream";

/// WebDAV connection settings supplied by the caller (settings page / Tauri
/// command). No defaults are resolved here: the caller owns persistence.
pub struct WebdavConfig {
    /// Directory base, e.g. `https://dav.example.com/remote.php/dav/files/me/ccp`.
    /// A trailing `/` is accepted and normalized away.
    pub base_url: String,
    pub username: String,
    pub password: String,
}

/// Build the canonical backup name for a point in time: UTC, second
/// precision, `ccp-backup-YYYYMMDD-HHMMSS.json`.
///
/// UTC (not local time) keeps names sortable and stable across machines and
/// DST changes. Milliseconds are truncated so two backups inside the same
/// second collide on purpose: the newer file wins on the server, which is what
/// "keep the latest" means for this feature.
pub fn backup_file_name(now_ms: i64) -> String {
    match chrono::DateTime::from_timestamp_millis(now_ms) {
        Some(utc) => utc.format("ccp-backup-%Y%m%d-%H%M%S.json").to_string(),
        // Out-of-range millis cannot be a real timestamp; degrade to the raw
        // number (still path-safe) instead of panicking or inventing a time.
        None => format!("ccp-backup-{now_ms}.json"),
    }
}

/// Upload `bytes` as `file_name` and return the full remote URL on success.
///
/// `file_name` must be a bare file name (see [`validate_file_name`]); the
/// remote directory is `config.base_url`.
pub async fn upload_backup(
    config: &WebdavConfig,
    file_name: &str,
    bytes: &[u8],
) -> anyhow::Result<String> {
    let url = remote_url(config, file_name)?;
    let client = client_for(&url)?;
    let response = client
        .put(&url)
        .timeout(REQUEST_TIMEOUT)
        .header(reqwest::header::CONTENT_TYPE, BACKUP_CONTENT_TYPE)
        .basic_auth(&config.username, Some(&config.password))
        .body(bytes.to_vec())
        .send()
        .await
        .with_context(|| format!("上传备份 {file_name} 到 {url} 失败：无法完成网络请求"))?;
    let status = response.status();
    ensure_success("上传", file_name, status)?;
    Ok(url)
}

/// Download the backup stored as `file_name`.
///
/// Any 2xx body is returned verbatim; the caller owns parsing/validation.
pub async fn download_backup(config: &WebdavConfig, file_name: &str) -> anyhow::Result<Vec<u8>> {
    let url = remote_url(config, file_name)?;
    let client = client_for(&url)?;
    let response = client
        .get(&url)
        .timeout(REQUEST_TIMEOUT)
        .basic_auth(&config.username, Some(&config.password))
        .send()
        .await
        .with_context(|| format!("下载备份 {file_name} 自 {url} 失败：无法完成网络请求"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(status_error("下载", file_name, status));
    }
    let body = response
        .bytes()
        .await
        .with_context(|| format!("下载备份 {file_name} 失败：读取响应内容未完成"))?;
    Ok(body.to_vec())
}

/// Join the configured directory with a validated bare file name.
///
/// The name is validated *before* the join, so a caller-supplied `../` or `/`
/// can never escape the configured directory.
fn remote_url(config: &WebdavConfig, file_name: &str) -> anyhow::Result<String> {
    validate_file_name(file_name)?;
    let base = config.base_url.trim();
    if base.is_empty() {
        bail!("WebDAV 地址为空，无法定位备份文件");
    }
    Ok(format!("{}/{}", base.trim_end_matches('/'), file_name))
}

/// Build a client through the repo's shared constructor so ambient proxy
/// handling stays in one place.
///
/// `url_target_is_loopback` makes localhost WebDAV (a NAS/desktop daemon or a
/// test server) bypass the system proxy; note that `proxied_client_for` only
/// applies its client-wide timeout to the client, so the per-request
/// [`REQUEST_TIMEOUT`] above is what actually caps a stuck PUT/GET.
fn client_for(url: &str) -> anyhow::Result<reqwest::Client> {
    proxied_client_for(USER_AGENT, url_target_is_loopback(url))
        .with_context(|| format!("初始化 WebDAV HTTP 客户端失败（目标 {url}）"))
}

/// Only `[A-Za-z0-9._-]` is accepted, plus the name may not consist solely of
/// dots. That rejects `../x`, `a/b`, `a\b`, absolute paths, empty names and the
/// `.` / `..` path components themselves before any request is sent.
fn validate_file_name(file_name: &str) -> anyhow::Result<()> {
    let charset_ok = !file_name.is_empty()
        && file_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    let dots_only = !file_name.is_empty() && file_name.chars().all(|c| c == '.');
    if !charset_ok || dots_only {
        bail!(
            "备份文件名不合法：{file_name:?}；只允许字母、数字、点、下划线和连字符，且不能是 . 或 .."
        );
    }
    Ok(())
}

/// Map a non-2xx WebDAV response onto the Chinese, status-bearing messages the
/// UI surfaces. Kept in one place so upload and download cannot drift apart.
fn ensure_success(
    action: &str,
    file_name: &str,
    status: reqwest::StatusCode,
) -> anyhow::Result<()> {
    if status.is_success() {
        return Ok(());
    }
    Err(status_error(action, file_name, status))
}

fn status_error(action: &str, file_name: &str, status: reqwest::StatusCode) -> anyhow::Error {
    let code = status.as_u16();
    match code {
        401 | 403 => anyhow!(
            "{action}备份 {file_name} 失败：WebDAV 认证失败（HTTP {code}），请检查用户名和密码"
        ),
        404 => anyhow!("{action}备份 {file_name} 失败：远端文件不存在（HTTP 404）"),
        _ => anyhow!("{action}备份 {file_name} 失败：WebDAV 返回 HTTP {code}"),
    }
}
