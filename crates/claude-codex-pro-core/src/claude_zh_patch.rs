use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const PATCH_MARKER: &str = "claude-codex-pro-zh-cn-patch";
const LANGUAGE_MARKER: &str = "claude-codex-pro-zh-cn-language";
const TEXT_MARKER: &str = "claude-codex-pro-zh-cn-text-v6";
const LEGACY_TEXT_MARKERS: &[&str] = &[
    "claude-codex-pro-zh-cn-text",
    "claude-codex-pro-zh-cn-text-v2",
    "claude-codex-pro-zh-cn-text-v3",
    "claude-codex-pro-zh-cn-text-v4",
    "claude-codex-pro-zh-cn-text-v5",
];
const BACKUP_DIR_NAME: &str = "Claude-zh-CN-official-backup";
const OFFICIAL_LANGUAGE_LOCALES: &[&str] = &[
    "en-US", "de-DE", "fr-FR", "ko-KR", "ja-JP", "es-419", "es-ES", "it-IT", "hi-IN", "pt-BR",
    "id-ID",
];
const DESKTOP_I18N_URL: &str = "https://raw.githubusercontent.com/Jyy1529/claude-desktop_win-zh_cn/master/resources/desktop-zh-CN.json";
const FRONTEND_I18N_URL: &str = "https://raw.githubusercontent.com/Jyy1529/claude-desktop_win-zh_cn/master/resources/frontend-zh-CN.json";
const STATSIG_I18N_URL: &str = "https://raw.githubusercontent.com/Jyy1529/claude-desktop_win-zh_cn/master/resources/statsig-zh-CN.json";
const EMBEDDED_DESKTOP_I18N: &str = include_str!("../../../assets/claude-zh/desktop-zh-CN.json");
const EMBEDDED_FRONTEND_I18N: &str = include_str!("../../../assets/claude-zh/frontend-zh-CN.json");
const EMBEDDED_STATSIG_I18N: &str = include_str!("../../../assets/claude-zh/statsig-zh-CN.json");
const EMBEDDED_CHUNK_PATCHES: &str = include_str!("../../../assets/claude-zh/chunk-patches.json");
const REMOTE_I18N_FETCH_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeZhPatchStatus {
    pub status: String,
    pub message: String,
    pub install_root: Option<String>,
    pub app_root: Option<String>,
    pub install_kind: String,
    pub locale_config_path: String,
    pub backup_dir: String,
    pub resources_present: bool,
    pub frontend_i18n_present: bool,
    pub statsig_i18n_present: bool,
    pub chunk_patch_present: bool,
    pub language_whitelist_patched: bool,
    pub locale_configured: bool,
    pub writable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeZhPatchOutcome {
    pub status: ClaudeZhPatchStatus,
    pub changed_files: Vec<String>,
    pub backup_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeZhPatchPaths {
    pub install_root: PathBuf,
    pub app_root: PathBuf,
    pub locale_config_path: PathBuf,
    pub backup_dir: PathBuf,
    pub install_kind: String,
}

impl ClaudeZhPatchPaths {
    pub fn patch_needs_elevation(&self) -> bool {
        self.install_kind == "msix" && !resource_tree_writable_no_create(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteI18nResources {
    desktop: String,
    frontend: String,
    statsig: String,
}

pub fn detect_status() -> ClaudeZhPatchStatus {
    match detect_paths() {
        Some(paths) => status_for_paths(&paths),
        None => {
            let locale_config_path = default_locale_config_path();
            let backup_dir = default_backup_dir();
            ClaudeZhPatchStatus {
                status: "not_found".to_string(),
                message: "未找到 Claude Desktop 安装目录。".to_string(),
                install_root: None,
                app_root: None,
                install_kind: "unknown".to_string(),
                locale_config_path: locale_config_path.to_string_lossy().to_string(),
                backup_dir: backup_dir.to_string_lossy().to_string(),
                resources_present: false,
                frontend_i18n_present: false,
                statsig_i18n_present: false,
                chunk_patch_present: false,
                language_whitelist_patched: false,
                locale_configured: locale_configured(&locale_config_path),
                writable: false,
            }
        }
    }
}

pub fn status_for_install_root(install_root: &Path) -> ClaudeZhPatchStatus {
    match paths_from_install_root(install_root.to_path_buf()) {
        Some(paths) => status_for_paths(&paths),
        None => {
            let locale_config_path = default_locale_config_path();
            let backup_dir = default_backup_dir();
            ClaudeZhPatchStatus {
                status: "not_found".to_string(),
                message: format!(
                    "所选目录不是可识别的 Claude Desktop 安装目录：{}",
                    install_root.display()
                ),
                install_root: Some(install_root.to_string_lossy().to_string()),
                app_root: None,
                install_kind: "unknown".to_string(),
                locale_config_path: locale_config_path.to_string_lossy().to_string(),
                backup_dir: backup_dir.to_string_lossy().to_string(),
                resources_present: false,
                frontend_i18n_present: false,
                statsig_i18n_present: false,
                chunk_patch_present: false,
                language_whitelist_patched: false,
                locale_configured: locale_configured(&locale_config_path),
                writable: false,
            }
        }
    }
}

pub fn install_patch() -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = detect_paths().ok_or_else(|| anyhow::anyhow!("未找到 Claude Desktop 安装目录"))?;
    install_patch_at(&paths)
}

pub fn detected_patch_needs_elevation() -> bool {
    detect_paths()
        .map(|paths| paths.patch_needs_elevation())
        .unwrap_or(false)
}

pub fn install_root_patch_needs_elevation(install_root: &Path) -> bool {
    paths_from_install_root(install_root.to_path_buf())
        .map(|paths| paths.patch_needs_elevation())
        .unwrap_or(false)
}

pub async fn install_patch_with_remote_resources() -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = detect_paths().ok_or_else(|| anyhow::anyhow!("未找到 Claude Desktop 安装目录"))?;
    let resources = i18n_resources_for_install().await;
    install_patch_at_with_resources(&paths, Some(&resources))
}

pub async fn install_patch_with_remote_resources_elevated() -> anyhow::Result<ClaudeZhPatchOutcome>
{
    install_patch_with_remote_resources_elevated_for_user(None).await
}

pub async fn install_patch_with_remote_resources_elevated_for_user(
    target_user_sid: Option<&str>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = detect_paths().ok_or_else(|| anyhow::anyhow!("未找到 Claude Desktop 安装目录"))?;
    let resources = i18n_resources_for_install().await;
    install_patch_at_with_resources_elevated_for_user(&paths, Some(&resources), target_user_sid)
}

pub async fn install_patch_with_remote_resources_elevated_for_user_dirs(
    target_user_sid: Option<&str>,
    appdata: Option<&Path>,
    local_appdata: Option<&Path>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = detect_paths_for_user_dirs(appdata, local_appdata)
        .ok_or_else(|| anyhow::anyhow!("未找到 Claude Desktop 安装目录"))?;
    let resources = i18n_resources_for_install().await;
    install_patch_at_with_resources_elevated_for_user(&paths, Some(&resources), target_user_sid)
}

pub async fn install_patch_with_remote_resources_elevated_for_user_dirs_at_install_root(
    install_root: &Path,
    target_user_sid: Option<&str>,
    appdata: Option<&Path>,
    local_appdata: Option<&Path>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = paths_from_install_root(install_root.to_path_buf())
        .ok_or_else(|| {
            anyhow::anyhow!("未找到 Claude Desktop 安装目录：{}", install_root.display())
        })?
        .with_user_data_dirs(appdata, local_appdata);
    let resources = i18n_resources_for_install().await;
    install_patch_at_with_resources_elevated_for_user(&paths, Some(&resources), target_user_sid)
}

pub async fn install_patch_at_install_root_with_remote_resources(
    install_root: &Path,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = paths_from_install_root(install_root.to_path_buf()).ok_or_else(|| {
        anyhow::anyhow!(
            "所选目录不是可识别的 Claude Desktop 安装目录：{}",
            install_root.display()
        )
    })?;
    let resources = i18n_resources_for_install().await;
    install_patch_at_with_resources(&paths, Some(&resources))
}

async fn i18n_resources_for_install() -> RemoteI18nResources {
    match tokio::time::timeout(REMOTE_I18N_FETCH_TIMEOUT, fetch_remote_i18n_resources()).await {
        Ok(Ok(resources)) => resources,
        _ => embedded_i18n_resources(),
    }
}

pub async fn fetch_remote_i18n_resources() -> anyhow::Result<RemoteI18nResources> {
    let client = crate::http_client::proxied_client("ClaudeCodexPro/ClaudeZhPatch")?;
    let (desktop, frontend, statsig) = tokio::try_join!(
        fetch_i18n_json(&client, DESKTOP_I18N_URL),
        fetch_i18n_json(&client, FRONTEND_I18N_URL),
        fetch_i18n_json(&client, STATSIG_I18N_URL),
    )?;
    Ok(RemoteI18nResources {
        desktop,
        frontend,
        statsig,
    })
}

pub fn embedded_i18n_resources() -> RemoteI18nResources {
    RemoteI18nResources {
        desktop: EMBEDDED_DESKTOP_I18N.to_string(),
        frontend: EMBEDDED_FRONTEND_I18N.to_string(),
        statsig: EMBEDDED_STATSIG_I18N.to_string(),
    }
}

async fn fetch_i18n_json(client: &reqwest::Client, url: &str) -> anyhow::Result<String> {
    let text = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json,text/plain,*/*")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let value: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("远程汉化资源不是合法 JSON：{url}"))?;
    if !value
        .as_object()
        .map(|object| !object.is_empty())
        .unwrap_or(false)
    {
        anyhow::bail!("远程汉化资源为空：{url}");
    }
    Ok(text)
}

pub fn restore_patch() -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = detect_paths().ok_or_else(|| anyhow::anyhow!("未找到 Claude Desktop 安装目录"))?;
    restore_patch_at(&paths)
}

pub fn restore_patch_elevated_for_user(
    target_user_sid: Option<&str>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = detect_paths().ok_or_else(|| anyhow::anyhow!("未找到 Claude Desktop 安装目录"))?;
    restore_patch_at_elevated_for_user(&paths, target_user_sid)
}

pub fn restore_patch_elevated_for_user_dirs(
    target_user_sid: Option<&str>,
    appdata: Option<&Path>,
    local_appdata: Option<&Path>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = detect_paths_for_user_dirs(appdata, local_appdata)
        .ok_or_else(|| anyhow::anyhow!("未找到 Claude Desktop 安装目录"))?;
    restore_patch_at_elevated_for_user(&paths, target_user_sid)
}

pub fn restore_patch_elevated_for_user_dirs_at_install_root(
    install_root: &Path,
    target_user_sid: Option<&str>,
    appdata: Option<&Path>,
    local_appdata: Option<&Path>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = paths_from_install_root(install_root.to_path_buf())
        .ok_or_else(|| {
            anyhow::anyhow!("未找到 Claude Desktop 安装目录：{}", install_root.display())
        })?
        .with_user_data_dirs(appdata, local_appdata);
    restore_patch_at_elevated_for_user(&paths, target_user_sid)
}

pub fn install_patch_at(paths: &ClaudeZhPatchPaths) -> anyhow::Result<ClaudeZhPatchOutcome> {
    install_patch_at_with_resources(paths, None)
}

pub fn install_patch_at_with_resources(
    paths: &ClaudeZhPatchPaths,
    resources: Option<&RemoteI18nResources>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    install_patch_at_with_resources_impl(paths, resources, true, None)
}

pub fn install_patch_at_with_resources_elevated(
    paths: &ClaudeZhPatchPaths,
    resources: Option<&RemoteI18nResources>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    install_patch_at_with_resources_elevated_for_user(paths, resources, None)
}

pub fn install_patch_at_with_resources_elevated_for_user(
    paths: &ClaudeZhPatchPaths,
    resources: Option<&RemoteI18nResources>,
    target_user_sid: Option<&str>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    prepare_elevated_patch_access(paths, target_user_sid)?;
    ensure_patch_writable(paths)?;
    install_patch_at_with_resources_impl(paths, resources, false, target_user_sid)
}

fn install_patch_at_with_resources_impl(
    paths: &ClaudeZhPatchPaths,
    resources: Option<&RemoteI18nResources>,
    check_writable: bool,
    elevated_target_user_sid: Option<&str>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    let mut changed_files = Vec::new();
    if check_writable {
        ensure_patch_writable(paths)?;
    }
    std::fs::create_dir_all(&paths.backup_dir).with_context(|| {
        format!(
            "创建 Claude 中文补丁备份目录失败：{}",
            paths.backup_dir.display()
        )
    })?;

    let root_i18n = paths.app_root.join("resources").join("zh-CN.json");
    backup_file(&root_i18n, paths)?;
    let fallback_desktop_i18n = desktop_i18n_json();
    let desktop_i18n = resources
        .map(|resources| resources.desktop.as_str())
        .unwrap_or(fallback_desktop_i18n.as_str());
    write_patch_file_for_install(
        &root_i18n,
        desktop_i18n.as_bytes(),
        elevated_target_user_sid,
    )?;
    changed_files.push(root_i18n.to_string_lossy().to_string());

    let frontend_i18n = paths
        .app_root
        .join("resources")
        .join("ion-dist")
        .join("i18n")
        .join("zh-CN.json");
    backup_file(&frontend_i18n, paths)?;
    let fallback_frontend_i18n = frontend_i18n_json();
    let frontend_i18n_contents = resources
        .map(|resources| resources.frontend.as_str())
        .unwrap_or(fallback_frontend_i18n.as_str());
    write_patch_file_for_install(
        &frontend_i18n,
        frontend_i18n_contents.as_bytes(),
        elevated_target_user_sid,
    )?;
    changed_files.push(frontend_i18n.to_string_lossy().to_string());

    let statsig_i18n = paths
        .app_root
        .join("resources")
        .join("ion-dist")
        .join("i18n")
        .join("statsig")
        .join("zh-CN.json");
    backup_file(&statsig_i18n, paths)?;
    let fallback_statsig_i18n = statsig_i18n_json();
    let statsig_i18n_contents = resources
        .map(|resources| resources.statsig.as_str())
        .unwrap_or(fallback_statsig_i18n.as_str());
    write_patch_file_for_install(
        &statsig_i18n,
        statsig_i18n_contents.as_bytes(),
        elevated_target_user_sid,
    )?;
    changed_files.push(statsig_i18n.to_string_lossy().to_string());

    let chunks = find_patchable_chunks(&paths.app_root)?;
    let runtime_patch_chunk = select_runtime_patch_chunk(&chunks);
    for chunk in chunks {
        let before = std::fs::read_to_string(&chunk).unwrap_or_default();
        backup_file(&chunk, paths)?;
        patch_chunk(
            &chunk,
            runtime_patch_chunk.as_deref() == Some(chunk.as_path()),
        )?;
        if let Err(error) = validate_patched_javascript_chunk(&chunk) {
            restore_official_backup_file(&chunk, paths).with_context(|| {
                format!(
                    "Claude 汉化 JS 校验失败后恢复官方备份也失败：{}",
                    chunk.display()
                )
            })?;
            return Err(error).with_context(|| {
                format!(
                    "Claude 汉化 JS 校验失败，已恢复官方备份：{}",
                    chunk.display()
                )
            });
        }
        let after = std::fs::read_to_string(&chunk).unwrap_or_default();
        if before != after {
            changed_files.push(chunk.to_string_lossy().to_string());
        }
    }
    clear_claude_renderer_cache(paths, &mut changed_files);

    // Best effort: if Claude was relaunched during the long chunk patch phase,
    // close it again immediately before writing locale so it cannot flush en-US back.
    let _ = crate::claude_desktop::close_claude_desktop_for_patch();

    backup_file(&paths.locale_config_path, paths)?;
    write_locale_config(&paths.locale_config_path)?;
    if !locale_configured(&paths.locale_config_path) {
        write_locale_config(&paths.locale_config_path)?;
    }
    changed_files.push(paths.locale_config_path.to_string_lossy().to_string());

    let outcome = ClaudeZhPatchOutcome {
        status: status_for_paths(paths),
        backup_dir: paths.backup_dir.to_string_lossy().to_string(),
        changed_files,
    };
    ensure_install_complete(&outcome)?;
    Ok(outcome)
}

pub fn restore_patch_at(paths: &ClaudeZhPatchPaths) -> anyhow::Result<ClaudeZhPatchOutcome> {
    let mut changed_files = Vec::new();
    if paths.backup_dir.exists() {
        for entry in std::fs::read_dir(&paths.backup_dir)? {
            let entry = entry?;
            let from = entry.path();
            if !from.is_file() {
                continue;
            }
            let Some(name) = from.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let to = decode_backup_name(paths, name);
            let bytes = std::fs::read(&from)
                .with_context(|| format!("读取 Claude 汉化备份失败：{}", from.display()))?;
            write_patch_file(&to, &bytes)
                .with_context(|| format!("恢复 Claude 官方备份失败：{}", to.display()))?;
            changed_files.push(to.to_string_lossy().to_string());
        }
    }
    remove_zh_cn_artifacts(paths, &mut changed_files)?;
    remove_locale_config(paths, &mut changed_files)?;
    scrub_zh_cn_from_chunks(paths, &mut changed_files)?;
    let outcome = ClaudeZhPatchOutcome {
        status: status_for_paths(paths),
        backup_dir: paths.backup_dir.to_string_lossy().to_string(),
        changed_files,
    };
    ensure_restore_complete(&outcome)?;
    Ok(outcome)
}

fn ensure_install_complete(outcome: &ClaudeZhPatchOutcome) -> anyhow::Result<()> {
    if outcome.status.status == "ok" {
        return Ok(());
    }
    anyhow::bail!(
        "{} resources={} frontend={} statsig={} locale={} chunk={} language={}",
        outcome.status.message,
        outcome.status.resources_present,
        outcome.status.frontend_i18n_present,
        outcome.status.statsig_i18n_present,
        outcome.status.locale_configured,
        outcome.status.chunk_patch_present,
        outcome.status.language_whitelist_patched,
    )
}

fn ensure_restore_complete(outcome: &ClaudeZhPatchOutcome) -> anyhow::Result<()> {
    if outcome.status.status == "not_installed" {
        return Ok(());
    }
    anyhow::bail!(
        "Claude 官方文件恢复后仍检测到汉化残留：status={} resources={} frontend={} statsig={} locale={} chunk={} language={}",
        outcome.status.status,
        outcome.status.resources_present,
        outcome.status.frontend_i18n_present,
        outcome.status.statsig_i18n_present,
        outcome.status.locale_configured,
        outcome.status.chunk_patch_present,
        outcome.status.language_whitelist_patched,
    )
}

pub fn restore_patch_at_elevated_for_user(
    paths: &ClaudeZhPatchPaths,
    target_user_sid: Option<&str>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    prepare_elevated_patch_access(paths, target_user_sid)?;
    ensure_patch_writable(paths)?;
    restore_patch_at(paths)
}

fn remove_zh_cn_artifacts(
    paths: &ClaudeZhPatchPaths,
    changed_files: &mut Vec<String>,
) -> anyhow::Result<()> {
    for path in [
        paths.app_root.join("resources").join("zh-CN.json"),
        paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("i18n")
            .join("zh-CN.json"),
        paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("i18n")
            .join("statsig")
            .join("zh-CN.json"),
    ] {
        clear_atomic_temp_file(&path);
        clear_atomic_backup_file(&path);
        if path.exists() {
            clear_readonly_bit(&path);
            std::fs::remove_file(&path)
                .with_context(|| format!("删除 Claude zh-CN 汉化资源失败：{}", path.display()))?;
            changed_files.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

fn remove_locale_config(
    paths: &ClaudeZhPatchPaths,
    changed_files: &mut Vec<String>,
) -> anyhow::Result<()> {
    if !paths.locale_config_path.exists() {
        return Ok(());
    }
    let mut config = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(
        &paths.locale_config_path,
    )?)
    .unwrap_or_else(|_| json!({}));
    let Some(object) = config.as_object_mut() else {
        return Ok(());
    };
    let removed_locale = object.remove("locale").is_some();
    let removed_font = object.remove("claudeZhCnFont").is_some();
    if removed_locale || removed_font {
        write_patch_file(
            &paths.locale_config_path,
            serde_json::to_string_pretty(&config)?.as_bytes(),
        )?;
        changed_files.push(paths.locale_config_path.to_string_lossy().to_string());
    }
    Ok(())
}

fn scrub_zh_cn_from_chunks(
    paths: &ClaudeZhPatchPaths,
    changed_files: &mut Vec<String>,
) -> anyhow::Result<()> {
    for chunk in find_patchable_chunks(&paths.app_root)? {
        let before = std::fs::read_to_string(&chunk).unwrap_or_default();
        let mut after = before.replace(",\"zh-CN\"", "").replace(",'zh-CN'", "");
        after = remove_marker_line(after, LANGUAGE_MARKER);
        after = remove_marker_line(after, TEXT_MARKER);
        for marker in LEGACY_TEXT_MARKERS {
            after = remove_marker_line(after, marker);
        }
        after = remove_runtime_patch_script(after);
        if after != before {
            write_patch_file(&chunk, after.as_bytes())?;
            changed_files.push(chunk.to_string_lossy().to_string());
        }
    }
    Ok(())
}

fn remove_marker_line(text: String, marker: &str) -> String {
    text.lines()
        .filter(|line| !line.contains(marker))
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_runtime_patch_script(text: String) -> String {
    let Some(marker_index) = text.find(PATCH_MARKER) else {
        return text;
    };
    let Some(script_start) = text[..marker_index].rfind(";(() => {") else {
        return text;
    };
    text[..script_start].trim_end().to_string()
}

fn remove_legacy_runtime_residue(mut text: String) -> String {
    text = remove_runtime_patch_script(text);
    remove_legacy_language_patch_lines(text)
}

fn remove_legacy_language_patch_lines(text: String) -> String {
    let mut output = Vec::new();
    let mut skip_next_language_iife = false;
    for line in text.lines() {
        let is_language_marker = line.contains(LANGUAGE_MARKER);
        let is_legacy_text_marker = LEGACY_TEXT_MARKERS
            .iter()
            .any(|marker| line.contains(marker));
        let is_language_iife =
            line.contains("__CLAUDE_CODEX_PRO_ZH_CN_LANGUAGE__") && line.contains(LANGUAGE_MARKER);
        if is_language_marker
            || is_legacy_text_marker
            || (skip_next_language_iife && is_language_iife)
        {
            skip_next_language_iife = is_legacy_text_marker;
            continue;
        }
        skip_next_language_iife = false;
        output.push(line);
    }
    output.join("\n")
}

pub fn status_for_paths(paths: &ClaudeZhPatchPaths) -> ClaudeZhPatchStatus {
    let root_i18n = paths.app_root.join("resources").join("zh-CN.json");
    let frontend_i18n = paths
        .app_root
        .join("resources")
        .join("ion-dist")
        .join("i18n")
        .join("zh-CN.json");
    let statsig_i18n = paths
        .app_root
        .join("resources")
        .join("ion-dist")
        .join("i18n")
        .join("statsig")
        .join("zh-CN.json");
    let chunk_texts = find_patchable_chunks(&paths.app_root)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .collect::<Vec<_>>();
    let ui_chunk_texts = chunk_texts
        .iter()
        .filter(|text| chunk_needs_or_has_ui_patch(text))
        .collect::<Vec<_>>();
    let chunk_patch_present = !ui_chunk_texts.is_empty()
        && ui_chunk_texts.iter().all(|text| text.contains(TEXT_MARKER))
        && ui_chunk_texts
            .iter()
            .all(|text| !text.contains(PATCH_MARKER));
    let language_whitelist_patched = ui_chunk_texts
        .iter()
        .any(|text| text.contains(LANGUAGE_MARKER) || has_zh_cn_language_support(text));
    let writable = resource_tree_writable_no_create(paths);
    let locale_configured = locale_configured(&paths.locale_config_path);
    let resources_present = valid_i18n_resource_with_min_keys(&root_i18n, 100);
    let frontend_i18n_present = valid_i18n_resource_with_min_keys(&frontend_i18n, 1_000);
    let statsig_i18n_present = valid_i18n_resource_with_min_keys(&statsig_i18n, 10);
    let ready = resources_present
        && frontend_i18n_present
        && statsig_i18n_present
        && locale_configured
        && chunk_patch_present
        && language_whitelist_patched;
    ClaudeZhPatchStatus {
        status: if ready { "ok" } else { "not_installed" }.to_string(),
        message: if ready {
            "Claude Desktop 本机中文补丁已安装。"
        } else {
            "Claude Desktop 本机中文补丁未完整安装。"
        }
        .to_string(),
        install_root: Some(paths.install_root.to_string_lossy().to_string()),
        app_root: Some(paths.app_root.to_string_lossy().to_string()),
        install_kind: paths.install_kind.clone(),
        locale_config_path: paths.locale_config_path.to_string_lossy().to_string(),
        backup_dir: paths.backup_dir.to_string_lossy().to_string(),
        resources_present,
        frontend_i18n_present,
        statsig_i18n_present,
        chunk_patch_present,
        language_whitelist_patched,
        locale_configured,
        writable,
    }
}

fn detect_paths() -> Option<ClaudeZhPatchPaths> {
    candidate_install_roots()
        .into_iter()
        .filter(|path| path.exists())
        .filter_map(paths_from_install_root)
        .next()
}

fn detect_paths_for_user_dirs(
    appdata: Option<&Path>,
    local_appdata: Option<&Path>,
) -> Option<ClaudeZhPatchPaths> {
    detect_paths().map(|paths| paths.with_user_data_dirs(appdata, local_appdata))
}

impl ClaudeZhPatchPaths {
    fn with_user_data_dirs(mut self, appdata: Option<&Path>, local_appdata: Option<&Path>) -> Self {
        if let Some(local_appdata) = local_appdata {
            self.locale_config_path = local_appdata.join("Claude-3p").join("config.json");
        } else if let Some(appdata) = appdata {
            self.locale_config_path = appdata.join("Claude-3p").join("config.json");
        }
        if let Some(local_appdata) = local_appdata {
            self.backup_dir = local_appdata.join(BACKUP_DIR_NAME);
        }
        self
    }
}

fn candidate_install_roots() -> Vec<PathBuf> {
    let mut candidates = running_claude_install_roots();
    for path in appx_claude_install_roots() {
        push_unique_path(&mut candidates, path);
    }
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        push_unique_path(&mut candidates, local_app_data.join("AnthropicClaude"));
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles").map(PathBuf::from) {
        let windows_apps = program_files.join("WindowsApps");
        if let Ok(entries) = std::fs::read_dir(&windows_apps) {
            let mut matches = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.to_ascii_lowercase().starts_with("claude_"))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();
            matches.sort();
            matches.reverse();
            for path in matches {
                push_unique_path(&mut candidates, path);
            }
        }
    }
    candidates
}

#[cfg(windows)]
fn appx_claude_install_roots() -> Vec<PathBuf> {
    let script = "$packages = @(); $packages += Get-AppxPackage -Name Claude -ErrorAction SilentlyContinue; try { $packages += Get-AppxPackage -AllUsers -Name Claude -ErrorAction Stop } catch { }; $packages | Where-Object { $_.InstallLocation } | Sort-Object Version -Descending | Select-Object -ExpandProperty InstallLocation -Unique";
    let mut command = std::process::Command::new("powershell.exe");
    command.args(["-NoProfile", "-Command", script]);
    command.stdin(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::windows_create_no_window());
    }
    let Ok(output) = command.output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let mut roots = Vec::new();
    for path in String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
    {
        push_unique_path(&mut roots, path);
    }
    roots
}

#[cfg(not(windows))]
fn appx_claude_install_roots() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(windows)]
fn running_claude_install_roots() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for process in crate::windows_integration::enumerate_processes()
        .into_iter()
        .filter(|process| process.exe_file.eq_ignore_ascii_case("claude.exe"))
    {
        if let Some(path) = process
            .executable_path
            .as_deref()
            .and_then(install_root_from_executable_path)
        {
            push_unique_path(&mut candidates, path);
        }
    }
    candidates
}

#[cfg(not(windows))]
fn running_claude_install_roots() -> Vec<PathBuf> {
    Vec::new()
}

fn install_root_from_executable_path(executable_path: &Path) -> Option<PathBuf> {
    let app_root = executable_path.parent()?;
    if !app_root.join("resources").exists() {
        return None;
    }
    if app_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("app"))
        .unwrap_or(false)
    {
        return app_root.parent().map(Path::to_path_buf);
    }
    Some(app_root.to_path_buf())
}

fn push_unique_path(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if !candidates.iter().any(|existing| existing == &path) {
        candidates.push(path);
    }
}

fn paths_from_install_root(install_root: PathBuf) -> Option<ClaudeZhPatchPaths> {
    let app_root = if install_root.join("app").join("resources").exists() {
        install_root.join("app")
    } else if install_root.join("resources").exists() {
        install_root.clone()
    } else {
        return None;
    };
    let install_kind = if install_root
        .to_string_lossy()
        .to_ascii_lowercase()
        .contains("\\windowsapps\\")
    {
        "msix"
    } else {
        "desktop"
    }
    .to_string();
    Some(ClaudeZhPatchPaths {
        install_root,
        app_root,
        locale_config_path: default_locale_config_path(),
        backup_dir: default_backup_dir(),
        install_kind,
    })
}

fn default_locale_config_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Claude-3p")
        .join("config.json")
}

fn default_backup_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(BACKUP_DIR_NAME)
}

fn write_locale_config(path: &Path) -> anyhow::Result<()> {
    let mut config = if path.exists() {
        serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(path)?)
            .unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    if !config.is_object() {
        config = json!({});
    }
    config["locale"] = json!("zh-CN");
    crate::settings::atomic_write(path, serde_json::to_string_pretty(&config)?.as_bytes())
}

fn ensure_patch_writable(paths: &ClaudeZhPatchPaths) -> anyhow::Result<()> {
    if resource_tree_writable_or_create(paths) {
        return Ok(());
    }
    anyhow::bail!(
        "Claude Desktop 安装目录已找到，但资源目录不可写：{}。汉化需要在该目录写入 zh-CN.json、前端 i18n 文件并修改语言白名单；如果这是 Microsoft Store/MSIX 版本，请允许弹出的管理员授权后重试，或安装可写入的桌面版 Claude 后再执行 Claude 一键汉化。",
        paths.app_root.join("resources").display()
    );
}

fn prepare_elevated_patch_access(
    paths: &ClaudeZhPatchPaths,
    target_user_sid: Option<&str>,
) -> anyhow::Result<()> {
    if paths.install_kind != "msix" || resource_tree_writable_no_create(paths) {
        return Ok(());
    }
    if !is_real_windows_apps_path(&paths.install_root) {
        return Ok(());
    }
    let mut access_warnings = Vec::new();
    for target in patch_access_targets(paths) {
        access_warnings.extend(grant_current_user_write_access(&target, target_user_sid)?);
    }
    if resource_tree_writable_or_create(paths) {
        return Ok(());
    }
    let warning_suffix = if access_warnings.is_empty() {
        String::new()
    } else {
        format!(" 授权诊断：{}", access_warnings.join("；"))
    };
    anyhow::bail!(
        "Claude Desktop WindowsApps 资源目录仍不可写：{}。已尝试管理员授权 takeown/icacls，请确认已在 UAC 中允许授权，或安装可写入的桌面版 Claude 后再重试。{}",
        paths.app_root.join("resources").display(),
        warning_suffix
    );
}

fn is_real_windows_apps_path(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    lower.contains("\\program files\\windowsapps\\")
        || lower.contains("/program files/windowsapps/")
}

#[cfg(windows)]
fn grant_current_user_write_access(
    path: &Path,
    target_user_sid: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let principal = target_user_sid
        .and_then(windows_sid_principal)
        .unwrap_or_else(current_windows_principal);
    let path_text = path.to_string_lossy().to_string();
    let takeown_args = vec![
        "/F".to_string(),
        path_text.clone(),
        "/R".to_string(),
        "/D".to_string(),
        "Y".to_string(),
    ];
    let user_grant_args = vec![
        path_text.clone(),
        "/grant".to_string(),
        format!("{principal}:(OI)(CI)F"),
        "/T".to_string(),
        "/C".to_string(),
    ];
    let users_grant_args = vec![
        path_text.clone(),
        "/grant".to_string(),
        "*S-1-5-32-545:(OI)(CI)M".to_string(),
        "/T".to_string(),
        "/C".to_string(),
    ];
    let admins_grant_args = vec![
        path_text,
        "/grant".to_string(),
        "*S-1-5-32-544:(OI)(CI)F".to_string(),
        "/T".to_string(),
        "/C".to_string(),
    ];

    let mut warnings = Vec::new();
    if let Err(error) = run_hidden_windows_command("takeown.exe", &takeown_args) {
        warnings.push(format!("takeown {} 失败：{}", path.display(), error));
    }
    let user_grant = run_hidden_windows_command("icacls.exe", &user_grant_args)
        .map_err(|error| error.to_string());
    let users_grant = run_hidden_windows_command("icacls.exe", &users_grant_args)
        .map_err(|error| error.to_string());
    let admins_grant = run_hidden_windows_command("icacls.exe", &admins_grant_args)
        .map_err(|error| error.to_string());
    if let Err(error) = &user_grant {
        warnings.push(format!(
            "icacls 当前用户 {} 授权失败：{}",
            path.display(),
            error
        ));
    }
    if let Err(error) = &users_grant {
        warnings.push(format!(
            "icacls Users 组 {} 授权失败：{}",
            path.display(),
            error
        ));
    }
    if let Err(error) = &admins_grant {
        warnings.push(format!(
            "icacls Administrators 组 {} 授权失败：{}",
            path.display(),
            error
        ));
    }
    if user_grant.is_err() && users_grant.is_err() && admins_grant.is_err() {
        anyhow::bail!(
            "WindowsApps 写入授权失败：{}：{}",
            path.display(),
            warnings.join("；")
        );
    }
    Ok(warnings)
}

#[cfg(windows)]
fn windows_sid_principal(value: &str) -> Option<String> {
    let sid = value.trim();
    let valid = sid.starts_with("S-")
        && sid
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch == '-' || ch == 'S');
    if valid { Some(format!("*{sid}")) } else { None }
}

#[cfg(windows)]
fn current_windows_principal() -> String {
    match (
        std::env::var("USERDOMAIN")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        std::env::var("USERNAME")
            .ok()
            .filter(|value| !value.trim().is_empty()),
    ) {
        (Some(domain), Some(user)) => format!("{domain}\\{user}"),
        (_, Some(user)) => user,
        _ => "%USERNAME%".to_string(),
    }
}

#[cfg(windows)]
fn run_hidden_windows_command(program: &str, args: &[String]) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(program);
    command.args(args);
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::windows_create_no_window());
    }
    let output = command
        .output()
        .with_context(|| format!("无法启动 {program}"))?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "{} 退出码 {:?} stdout={} stderr={}",
            program,
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
}

#[cfg(not(windows))]
fn grant_current_user_write_access(
    _path: &Path,
    _target_user_sid: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    Ok(Vec::new())
}

fn resource_tree_writable_no_create(paths: &ClaudeZhPatchPaths) -> bool {
    let resources = paths.app_root.join("resources");
    if !resources.is_dir() {
        return false;
    }
    for dir in patch_target_dirs(paths) {
        if !dir.is_dir() {
            return false;
        }
        if !probe_writable_dir(&dir) {
            return false;
        }
    }
    true
}

fn resource_tree_writable_or_create(paths: &ClaudeZhPatchPaths) -> bool {
    let resources = paths.app_root.join("resources");
    if !resources.is_dir() {
        return false;
    }
    for dir in patch_target_dirs(paths) {
        if !dir.exists() && std::fs::create_dir_all(&dir).is_err() {
            return false;
        }
        if !probe_writable_dir(&dir) {
            return false;
        }
    }
    true
}

fn patch_target_dirs(paths: &ClaudeZhPatchPaths) -> Vec<PathBuf> {
    let resources = paths.app_root.join("resources");
    let i18n = resources.join("ion-dist").join("i18n");
    let mut dirs = vec![resources.clone(), i18n.clone(), i18n.join("statsig")];
    if let Ok(chunks) = find_patchable_chunks(&paths.app_root) {
        for chunk in chunks {
            if let Some(parent) = chunk.parent() {
                push_unique_path(&mut dirs, parent.to_path_buf());
            }
        }
    }
    dirs
}

fn patch_target_files(paths: &ClaudeZhPatchPaths) -> Vec<PathBuf> {
    let resources = paths.app_root.join("resources");
    let i18n = resources.join("ion-dist").join("i18n");
    let mut files = vec![
        resources.join("zh-CN.json"),
        i18n.join("zh-CN.json"),
        i18n.join("statsig").join("zh-CN.json"),
    ];
    if let Ok(chunks) = find_patchable_chunks(&paths.app_root) {
        for chunk in chunks {
            push_unique_path(&mut files, chunk);
        }
    }
    files
}

fn patch_access_targets(paths: &ClaudeZhPatchPaths) -> Vec<PathBuf> {
    let mut targets = patch_target_dirs(paths);
    for file in patch_target_files(paths) {
        push_unique_path(&mut targets, file);
    }
    targets
}

fn probe_writable_dir(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(format!(
        ".claude-codex-pro-zh-probe-{}.tmp",
        std::process::id()
    ));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(mut file) => {
            let _ = std::io::Write::write_all(&mut file, b"probe");
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn locale_configured(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| {
            value
                .get("locale")
                .and_then(|locale| locale.as_str())
                .map(|locale| locale == "zh-CN")
        })
        .unwrap_or(false)
}

fn find_patchable_chunks(app_root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let assets = app_root.join("resources").join("ion-dist").join("assets");
    if !assets.exists() {
        return Ok(Vec::new());
    }
    let mut chunks = Vec::new();
    collect_patchable_chunks_recursive(&assets, &mut chunks)?;
    chunks.sort();
    Ok(chunks)
}

fn collect_patchable_chunks_recursive(dir: &Path, chunks: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_patchable_chunks_recursive(&path, chunks)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("js") {
            continue;
        }
        chunks.push(path);
    }
    Ok(())
}

fn patch_chunk(path: &Path, include_runtime_patch: bool) -> anyhow::Result<()> {
    let mut text = std::fs::read_to_string(path)?;
    if has_unsafe_window_runtime_patch(&text)
        || (!include_runtime_patch && text.contains(PATCH_MARKER))
        || text.contains(LANGUAGE_MARKER)
    {
        text = remove_legacy_runtime_residue(text);
    }
    if text.contains(PATCH_MARKER)
        && has_zh_cn_language_support(&text)
        && text.contains(TEXT_MARKER)
        && !has_unsafe_window_runtime_patch(&text)
    {
        return Ok(());
    }
    let mut patched = ensure_language_support(text);
    patched = replace_hardcoded_text(patched);
    write_patch_file(path, patched.as_bytes())
}

fn select_runtime_patch_chunk(chunks: &[PathBuf]) -> Option<PathBuf> {
    chunks
        .iter()
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("index-"))
                .unwrap_or(false)
        })
        .or_else(|| chunks.iter().find(|path| !is_worker_chunk(path)))
        .or_else(|| chunks.first())
        .cloned()
}

fn is_worker_chunk(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase().contains("worker"))
        .unwrap_or(false)
}

fn clear_claude_renderer_cache(paths: &ClaudeZhPatchPaths, changed_files: &mut Vec<String>) {
    let Some(profile_root) = paths.locale_config_path.parent() else {
        return;
    };
    for name in [
        "Cache",
        "Code Cache",
        "GPUCache",
        "DawnGraphiteCache",
        "DawnWebGPUCache",
    ] {
        let path = profile_root.join(name);
        if !path.exists() {
            continue;
        }
        if std::fs::remove_dir_all(&path).is_ok() {
            changed_files.push(path.to_string_lossy().to_string());
        }
    }
}

fn validate_patched_javascript_chunk(path: &Path) -> anyhow::Result<()> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("read Claude JS chunk metadata {}", path.display()))?;
    if metadata.len() == 0 {
        anyhow::bail!("Claude JS chunk is empty: {}", path.display());
    }
    let mut command = Command::new("node");
    command.arg("--check").arg(path);
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::windows_create_no_window());
    }
    let status = command.status();
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => anyhow::bail!(
            "node --check failed for {} with exit status {}",
            path.display(),
            status
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("run node --check for Claude JS chunk {}", path.display())),
    }
}

fn write_patch_file(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    clear_atomic_temp_file(path);
    match replace_patch_file(path, contents) {
        Ok(()) => Ok(()),
        Err(first_error) => {
            clear_atomic_temp_file(path);
            clear_readonly_bit(path);
            replace_patch_file(path, contents)
                .with_context(|| {
                    format!(
                        "写入 Claude 汉化文件失败：{}；首次错误：{}",
                        path.display(),
                        first_error
                    )
                })
                .or_else(|atomic_error| {
                    clear_atomic_temp_file(path);
                    clear_readonly_bit(path);
                    std::fs::write(path, contents).with_context(|| {
                        format!(
                            "直接覆盖 Claude 汉化文件失败：{}；原子写错误：{}",
                            path.display(),
                            atomic_error
                        )
                    })
                })
        }
    }
}

fn write_patch_file_for_install(
    path: &Path,
    contents: &[u8],
    elevated_target_user_sid: Option<&str>,
) -> anyhow::Result<()> {
    match write_patch_file(path, contents) {
        Ok(()) => Ok(()),
        Err(first_error) if elevated_target_user_sid.is_some() => {
            retry_write_patch_file_after_elevated_access(path, contents, elevated_target_user_sid)
                .with_context(|| {
                    format!(
                        "管理员授权后写入 Claude 汉化文件仍失败：{}；首次错误：{}",
                        path.display(),
                        first_error
                    )
                })
        }
        Err(error) => Err(error),
    }
}

fn retry_write_patch_file_after_elevated_access(
    path: &Path,
    contents: &[u8],
    target_user_sid: Option<&str>,
) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        let mut warnings = Vec::new();
        if let Some(parent) = path.parent() {
            warnings.extend(grant_current_user_write_access(parent, target_user_sid)?);
        }
        if path.exists() {
            warnings.extend(grant_current_user_write_access(path, target_user_sid)?);
        }
        clear_atomic_temp_file(path);
        clear_atomic_backup_file(path);
        clear_readonly_bit(path);
        write_patch_file(path, contents).map_err(|error| {
            anyhow::anyhow!(
                "{}；补授权诊断：{}",
                error,
                if warnings.is_empty() {
                    "无额外输出".to_string()
                } else {
                    warnings.join("；")
                }
            )
        })
    }
    #[cfg(not(windows))]
    {
        let _ = target_user_sid;
        write_patch_file(path, contents)
    }
}

fn replace_patch_file(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建 Claude 汉化目录失败：{}", parent.display()))?;
    }
    let temp_path = unique_atomic_temp_path_for(path);
    let backup_path = atomic_backup_path_for(path);
    clear_atomic_temp_file(path);
    clear_atomic_backup_file(path);
    std::fs::write(&temp_path, contents).with_context(|| {
        format!(
            "写入 Claude 汉化唯一临时文件失败：{}；目标文件：{}；父目录：{}",
            temp_path.display(),
            path.display(),
            path.parent()
                .map(|parent| parent.display().to_string())
                .unwrap_or_default()
        )
    })?;
    let had_existing_target = path.exists();
    if had_existing_target {
        clear_readonly_bit(path);
        std::fs::rename(path, &backup_path).with_context(|| {
            format!(
                "暂存旧 Claude 汉化文件失败：{} -> {}",
                path.display(),
                backup_path.display()
            )
        })?;
    }
    match std::fs::rename(&temp_path, path) {
        Ok(()) => {
            if had_existing_target {
                let _ = std::fs::remove_file(&backup_path);
            }
            Ok(())
        }
        Err(error) => {
            if had_existing_target && backup_path.exists() && !path.exists() {
                let _ = std::fs::rename(&backup_path, path);
            }
            anyhow::bail!(
                "替换 Claude 汉化文件失败：{} <- {}：{}",
                path.display(),
                temp_path.display(),
                error
            )
        }
    }
}

fn clear_atomic_temp_file(path: &Path) {
    let temp_path = atomic_temp_path_for(path);
    if !temp_path.exists() {
        return;
    }
    clear_readonly_bit(&temp_path);
    let _ = std::fs::remove_file(temp_path);
}

fn clear_atomic_backup_file(path: &Path) {
    let backup_path = atomic_backup_path_for(path);
    if !backup_path.exists() {
        return;
    }
    clear_readonly_bit(&backup_path);
    let _ = std::fs::remove_file(backup_path);
}

fn atomic_temp_path_for(path: &Path) -> PathBuf {
    sidecar_path_for(path, "tmp")
}

fn unique_atomic_temp_path_for(path: &Path) -> PathBuf {
    let suffix = format!(
        "tmp.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    );
    sidecar_path_for(path, &suffix)
}

fn atomic_backup_path_for(path: &Path) -> PathBuf {
    sidecar_path_for(path, "ccp-bak")
}

fn sidecar_path_for(path: &Path, suffix: &str) -> PathBuf {
    let mut temp_path = path.to_path_buf();
    let extension = path.extension().and_then(|value| value.to_str());
    temp_path.set_extension(match extension {
        Some(extension) => format!("{extension}.{suffix}"),
        None => suffix.to_string(),
    });
    temp_path
}

fn clear_readonly_bit(path: &Path) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    let mut permissions = metadata.permissions();
    if !permissions.readonly() {
        return;
    }
    permissions.set_readonly(false);
    let _ = std::fs::set_permissions(path, permissions);
}

fn ensure_language_support(text: String) -> String {
    if has_zh_cn_language_support(&text) {
        return text;
    }
    let (patched, changed) = patch_locale_arrays(&text);
    if changed {
        return format!("{patched}\n/* {LANGUAGE_MARKER} */");
    }
    let mut patched = text;
    for needle in [
        r#""en-US","fr-FR""#,
        r#""en-US","de-DE""#,
        r#""en-US","ja-JP""#,
        r#"'en-US','fr-FR'"#,
        r#"'en-US','de-DE'"#,
        r#"'en-US','ja-JP'"#,
    ] {
        if patched.contains(needle) {
            let replacement = if needle.starts_with('"') {
                needle.replacen(r#""en-US","#, r#""en-US","zh-CN","#, 1)
            } else {
                needle.replacen("'en-US',", "'en-US','zh-CN',", 1)
            };
            patched = patched.replacen(needle, &replacement, 1);
            return format!("{patched}\n/* {LANGUAGE_MARKER} */");
        }
    }
    patched
}

fn patch_locale_arrays(text: &str) -> (String, bool) {
    let bytes = text.as_bytes();
    let mut output = String::with_capacity(text.len() + 16);
    let mut cursor = 0;
    let mut changed = false;
    while cursor < bytes.len() {
        if bytes[cursor] != b'[' {
            output.push(bytes[cursor] as char);
            cursor += 1;
            continue;
        }
        if let Some((end, locales)) = parse_string_array_at(text, cursor) {
            let array_text = &text[cursor..end];
            if should_patch_locale_array(&locales)
                && !locales.iter().any(|locale| locale == "zh-CN")
            {
                output.push_str(&array_text[..array_text.len() - 1]);
                output.push_str(",\"zh-CN\"]");
                changed = true;
            } else {
                output.push_str(array_text);
            }
            cursor = end;
            continue;
        }
        output.push('[');
        cursor += 1;
    }
    (output, changed)
}

fn parse_string_array_at(text: &str, start: usize) -> Option<(usize, Vec<String>)> {
    let bytes = text.as_bytes();
    let mut index = start;
    if *bytes.get(index)? != b'[' {
        return None;
    }
    index += 1;
    let mut values = Vec::new();
    loop {
        skip_ascii_whitespace(bytes, &mut index);
        if *bytes.get(index)? == b']' {
            index += 1;
            return (!values.is_empty()).then_some((index, values));
        }
        if *bytes.get(index)? != b'"' {
            return None;
        }
        let (next, value) = parse_json_string(text, index)?;
        if !looks_like_locale(&value) {
            return None;
        }
        values.push(value);
        index = next;
        skip_ascii_whitespace(bytes, &mut index);
        match *bytes.get(index)? {
            b',' => index += 1,
            b']' => {
                index += 1;
                return Some((index, values));
            }
            _ => return None,
        }
    }
}

fn parse_json_string(text: &str, start: usize) -> Option<(usize, String)> {
    let bytes = text.as_bytes();
    if *bytes.get(start)? != b'"' {
        return None;
    }
    let mut index = start + 1;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if byte == b'"' {
            let raw = &text[start..=index];
            let value = serde_json::from_str::<String>(raw).ok()?;
            return Some((index + 1, value));
        }
        index += 1;
    }
    None
}

fn skip_ascii_whitespace(bytes: &[u8], index: &mut usize) {
    while matches!(bytes.get(*index), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        *index += 1;
    }
}

fn should_patch_locale_array(locales: &[String]) -> bool {
    if locales.first().map(String::as_str) != Some("en-US") {
        return false;
    }
    if locales.len() >= 2 && locales.len() <= 5 {
        return true;
    }
    let hits = locales
        .iter()
        .filter(|locale| OFFICIAL_LANGUAGE_LOCALES.contains(&locale.as_str()))
        .count();
    hits >= 6
        && ["de-DE", "fr-FR", "ja-JP", "ko-KR"]
            .iter()
            .all(|locale| locales.iter().any(|candidate| candidate == locale))
}

fn looks_like_locale(value: &str) -> bool {
    let mut parts = value.split('-');
    let Some(language) = parts.next() else {
        return false;
    };
    if !(2..=3).contains(&language.len()) || !language.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return false;
    }
    parts.all(|part| {
        (2..=4).contains(&part.len()) && part.chars().all(|ch| ch.is_ascii_alphanumeric())
    })
}

fn has_zh_cn_language_support(text: &str) -> bool {
    text.contains("\"zh-CN\"") || text.contains("'zh-CN'") || text.contains(LANGUAGE_MARKER)
}

fn has_unsafe_window_runtime_patch(text: &str) -> bool {
    text.contains(";window.__CLAUDE_CODEX_PRO_ZH_CN_LANGUAGE__")
        || text.contains("window.__CLAUDE_CODEX_PRO_ZH_CN_PATCH__")
}

fn chunk_needs_or_has_ui_patch(text: &str) -> bool {
    text.contains(PATCH_MARKER)
        || text.contains(TEXT_MARKER)
        || LEGACY_TEXT_MARKERS
            .iter()
            .any(|marker| text.contains(marker))
        || patch_locale_arrays(text).1
        || embedded_chunk_patch_pairs()
            .into_iter()
            .any(|(needle, _)| text.contains(&needle))
        || zh_text_pairs().iter().any(|(english, _)| {
            text.contains(&format!("\"{english}\"")) || text.contains(&format!("'{english}'"))
        })
        || zh_raw_patch_pairs()
            .iter()
            .any(|(needle, _)| text.contains(needle))
}

fn replace_hardcoded_text(mut text: String) -> String {
    for marker in LEGACY_TEXT_MARKERS {
        text = remove_marker_line(text, marker);
    }
    text = restore_static_text_replacements(text);
    if text.contains(TEXT_MARKER) {
        return text;
    }
    format!("{text}\n/* {TEXT_MARKER} */")
}

fn restore_static_text_replacements(mut text: String) -> String {
    for (english, chinese) in zh_raw_patch_pairs() {
        text = text.replace(chinese, english);
    }
    for (english, chinese) in embedded_chunk_patch_pairs() {
        text = text.replace(&chinese, &english);
    }
    for (english, chinese) in zh_text_pairs() {
        text = text.replace(&format!("\"{chinese}\""), &format!("\"{english}\""));
        text = text.replace(&format!("'{chinese}'"), &format!("'{english}'"));
    }
    text
}

fn embedded_chunk_patch_pairs() -> Vec<(String, String)> {
    serde_json::from_str::<std::collections::BTreeMap<String, Vec<(String, String)>>>(
        EMBEDDED_CHUNK_PATCHES,
    )
    .map(|groups| groups.into_values().flatten().collect())
    .unwrap_or_default()
}

fn zh_text_pairs() -> &'static [(&'static str, &'static str)] {
    &[
        ("Settings", "设置"),
        ("Inference configuration", "推理配置"),
        ("New chat", "新建对话"),
        ("New session", "新建会话"),
        ("Customize", "自定义"),
        ("Artifacts", "制品"),
        ("Live artifacts", "实时制品"),
        ("Account", "账号"),
        ("Appearance", "外观"),
        ("Privacy", "隐私"),
        ("Profile", "个人资料"),
        ("Notifications", "通知"),
        ("Billing", "账单"),
        ("Usage", "用量"),
        ("General", "通用"),
        ("Advanced", "高级"),
        ("Model", "模型"),
        ("Thinking", "思考"),
        ("Temperature", "温度"),
        ("System prompt", "系统提示词"),
        ("Memory", "记忆"),
        ("Projects", "项目"),
        ("Project", "项目"),
        ("Recents", "最近"),
        ("Recent", "最近"),
        ("Search", "搜索"),
        ("Help", "帮助"),
        ("Log out", "退出登录"),
        ("Sign in", "登录"),
        ("Continue", "继续"),
        ("Cancel", "取消"),
        ("Save", "保存"),
        ("Delete", "删除"),
        ("Rename", "重命名"),
        ("Export", "导出"),
        ("Import", "导入"),
        ("Retry", "重试"),
        ("Copy", "复制"),
        ("Edit", "编辑"),
        ("Done", "完成"),
        ("Close", "关闭"),
        ("Cowork", "协作"),
        ("Code", "代码"),
        ("New task", "新建任务"),
        ("Scheduled", "已安排"),
        ("Gateway", "第三方"),
        ("What's new", "新功能"),
        ("Configure third-party inference", "配置第三方推理"),
        ("Your provider setup needs a fix", "你的供应商配置需要修复"),
        (
            "Some required fields are missing or malformed. Open Setup to finish configuring it.",
            "部分必填字段缺失或格式不正确。打开设置向导完成配置。",
        ),
        ("Details", "详情"),
        ("Open Setup", "打开设置向导"),
        (
            "Let's knock something off your list",
            "让我们处理清单上的一件事",
        ),
        ("Learn how to use Cowork safely.", "了解如何安全使用协作。"),
        ("Connection", "连接方式"),
        (
            "Choose where Claude Desktop sends inference requests.",
            "选择 Claude Desktop 将推理请求发送到哪里。",
        ),
        ("Gateway credentials", "第三方凭据"),
        ("Gateway Credentials", "第三方凭据"),
        ("GATEWAY CREDENTIALS", "第三方凭据"),
        ("Gateway base URL", "第三方 URL"),
        (
            "Full URL of the inference gateway endpoint.",
            "推理第三方端点的完整 URL。",
        ),
        ("Custom inference headers", "自定义推理请求头"),
        (
            "Extra HTTP headers sent on every inference request to the configured provider.",
            "每次推理请求都会发送到已配置供应商的额外 HTTP 请求头。",
        ),
        (
            "For tenant routing, org IDs, Bedrock Guardrails, etc.",
            "用于租户路由、组织 ID、Bedrock Guardrails 等。",
        ),
        ("Add header", "添加请求头"),
        ("Credential kind", "凭据类型"),
        (
            "Selects the credential source. When set, only that source is used (no fallback).",
            "选择凭据来源。设置后只使用该来源（不回退）。",
        ),
        ("Models", "模型"),
        ("Model discovery", "模型发现"),
        ("Apply Changes", "应用更改"),
        ("Apply changes", "应用更改"),
        ("Workspace restrictions", "工作区限制"),
        ("Connectors & extensions", "连接器与扩展"),
        ("Telemetry & updates", "遥测与更新"),
        ("Usage limits", "使用限制"),
        ("Plugins & skills", "插件与技能"),
        ("Egress Requirements", "出口要求"),
        ("Source", "来源"),
    ]
}

fn zh_raw_patch_pairs() -> &'static [(&'static str, &'static str)] {
    &[
        ("label:\"Cowork\"", "label:\"协作\""),
        ("label:\"Code\"", "label:\"代码\""),
        ("label:\"Scheduled\"", "label:\"已安排\""),
        ("gateway:\"Gateway\"", "gateway:\"第三方\""),
        ("gateway:\"自定义\"", "gateway:\"第三方\""),
        ("\"Gateway base URL\"", "\"第三方 URL\""),
        ("\"Gateway API key\"", "\"第三方 API Key\""),
        ("\"Gateway auth scheme\"", "\"第三方认证方式\""),
        ("\"Gateway extra headers\"", "\"第三方额外请求头\""),
        ("\"Inference provider\"", "\"推理供应商\""),
        ("\"Sandbox & workspace\"", "\"沙盒与工作区\""),
        ("\"Add header\"", "\"添加请求头\""),
        ("\"Apply Changes\"", "\"应用更改\""),
        (
            "defaultMessage:\"Configure third-party inference\"",
            "defaultMessage:\"配置第三方推理\"",
        ),
        ("defaultMessage:\"What's new\"", "defaultMessage:\"新功能\""),
    ]
}

fn desktop_i18n_json() -> String {
    EMBEDDED_DESKTOP_I18N.to_string()
}

fn frontend_i18n_json() -> String {
    EMBEDDED_FRONTEND_I18N.to_string()
}

fn statsig_i18n_json() -> String {
    EMBEDDED_STATSIG_I18N.to_string()
}

fn backup_file(path: &Path, paths: &ClaudeZhPatchPaths) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let backup = paths.backup_dir.join(encode_backup_name(paths, path));
    if backup.exists() {
        return Ok(());
    }
    if let Some(parent) = backup.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(path, backup)?;
    Ok(())
}

fn restore_official_backup_file(path: &Path, paths: &ClaudeZhPatchPaths) -> anyhow::Result<()> {
    let backup = paths.backup_dir.join(encode_backup_name(paths, path));
    if !backup.exists() {
        anyhow::bail!("Claude 官方备份不存在：{}", backup.display());
    }
    let bytes = std::fs::read(&backup)
        .with_context(|| format!("读取 Claude 官方备份失败：{}", backup.display()))?;
    write_patch_file(path, &bytes)
        .with_context(|| format!("恢复 Claude 官方备份失败：{}", path.display()))
}

fn encode_backup_name(paths: &ClaudeZhPatchPaths, path: &Path) -> String {
    let base = if path.starts_with(&paths.app_root) {
        path.strip_prefix(&paths.app_root).unwrap_or(path)
    } else {
        path
    };
    base.to_string_lossy()
        .trim_start_matches(['\\', '/'])
        .replace(['\\', '/', ':'], "__")
}

fn decode_backup_name(paths: &ClaudeZhPatchPaths, name: &str) -> PathBuf {
    if name.ends_with("config.json") {
        return paths.locale_config_path.clone();
    }
    let rel = name.replace("__", std::path::MAIN_SEPARATOR_STR);
    paths.app_root.join(rel)
}

#[cfg(test)]
fn valid_i18n_resource(path: &Path) -> bool {
    valid_i18n_resource_with_min_keys(path, 1)
}

fn valid_i18n_resource_with_min_keys(path: &Path, min_keys: usize) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.as_object().map(|object| object.len() >= min_keys))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_paths(root: &Path) -> ClaudeZhPatchPaths {
        ClaudeZhPatchPaths {
            install_root: root.join("AnthropicClaude"),
            app_root: root.join("AnthropicClaude"),
            locale_config_path: root.join("Claude-3p").join("config.json"),
            backup_dir: root.join("backup"),
            install_kind: "desktop".to_string(),
        }
    }

    fn create_sample_install(paths: &ClaudeZhPatchPaths) {
        let i18n = paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("i18n");
        let statsig = i18n.join("statsig");
        let assets = paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("assets");
        let assets_v1 = assets.join("v1");
        std::fs::create_dir_all(&statsig).unwrap();
        std::fs::create_dir_all(&assets_v1).unwrap();
        std::fs::write(
            paths.app_root.join("resources").join("zh-CN.json"),
            "{\"locale\":\"zh-CN\"}",
        )
        .unwrap();
        std::fs::write(
            assets.join("index-demo.js"),
            "const locales=[\"en-US\",\"fr-FR\"]; console.log('Settings'); console.log(\"Inference configuration\"); console.log(\"Configure third-party inference\"); console.log(\"Gateway base URL\"); console.log(\"Open Setup\"); console.log(\"What's new\"); const tab={label:\"Cowork\"}; e.jsx(yt,{defaultMessage:\"Configure third-party inference\",id:\"on79ZcGd72\"}); e.jsx(Sa,{defaultMessage:\"What's new\",id:\"G5/5ebU+Uo\"});",
        )
        .unwrap();
        std::fs::write(
            assets_v1.join("index-real.js"),
            "const locales=[\"en-US\",\"de-DE\"]; console.log('Customize');",
        )
        .unwrap();
        std::fs::write(
            assets_v1.join("c71860c77-demo.js"),
            "e.jsx(yt,{defaultMessage:\"Configure third-party inference\",id:\"on79ZcGd72\"});",
        )
        .unwrap();
        std::fs::write(
            assets_v1.join("c5610fbe3-demo.js"),
            "e.jsx(Sa,{defaultMessage:\"What's new\",id:\"G5/5ebU+Uo\"});",
        )
        .unwrap();
    }

    #[test]
    fn install_patch_writes_resources_locale_and_safe_chunk_markers() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);
        let cache_dir = paths
            .locale_config_path
            .parent()
            .unwrap()
            .join("Code Cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let outcome = install_patch_at(&paths).unwrap();

        assert_eq!(outcome.status.status, "ok");
        assert!(outcome.status.resources_present);
        assert!(outcome.status.frontend_i18n_present);
        assert!(outcome.status.statsig_i18n_present);
        assert!(outcome.status.locale_configured);
        assert!(outcome.status.chunk_patch_present);
        assert!(outcome.status.language_whitelist_patched);
        assert!(!cache_dir.exists());
        assert!(
            std::fs::read_to_string(&paths.locale_config_path)
                .unwrap()
                .contains("zh-CN")
        );
        let patched_chunk = std::fs::read_to_string(
            paths
                .app_root
                .join("resources")
                .join("ion-dist")
                .join("assets")
                .join("index-demo.js"),
        )
        .unwrap();
        assert!(!patched_chunk.contains(PATCH_MARKER));
        assert!(patched_chunk.contains(TEXT_MARKER));
        assert!(patched_chunk.contains("\"zh-CN\""));
        assert!(patched_chunk.contains("'Settings'"));
        assert!(patched_chunk.contains("\"Inference configuration\""));
        assert!(patched_chunk.contains("\"Configure third-party inference\""));
        assert!(patched_chunk.contains("\"Gateway base URL\""));
        assert!(patched_chunk.contains("\"Open Setup\""));
        assert!(patched_chunk.contains("\"What's new\""));
        assert!(patched_chunk.contains("label:\"Cowork\""));
        assert!(patched_chunk.contains("defaultMessage:\"Configure third-party inference\""));
        assert!(patched_chunk.contains("defaultMessage:\"What's new\""));
        let nested_chunk = std::fs::read_to_string(
            paths
                .app_root
                .join("resources")
                .join("ion-dist")
                .join("assets")
                .join("v1")
                .join("index-real.js"),
        )
        .unwrap();
        assert!(!nested_chunk.contains(PATCH_MARKER));
        assert!(nested_chunk.contains(TEXT_MARKER));
        assert!(nested_chunk.contains("\"zh-CN\""));
        assert!(nested_chunk.contains("'Customize'"));
        let lazy_provider_chunk = std::fs::read_to_string(
            paths
                .app_root
                .join("resources")
                .join("ion-dist")
                .join("assets")
                .join("v1")
                .join("c71860c77-demo.js"),
        )
        .unwrap();
        assert!(!lazy_provider_chunk.contains(PATCH_MARKER));
        assert!(lazy_provider_chunk.contains(TEXT_MARKER));
        assert!(lazy_provider_chunk.contains("defaultMessage:\"Configure third-party inference\""));
        let lazy_whats_new_chunk = std::fs::read_to_string(
            paths
                .app_root
                .join("resources")
                .join("ion-dist")
                .join("assets")
                .join("v1")
                .join("c5610fbe3-demo.js"),
        )
        .unwrap();
        assert!(!lazy_whats_new_chunk.contains(PATCH_MARKER));
        assert!(lazy_whats_new_chunk.contains(TEXT_MARKER));
        assert!(lazy_whats_new_chunk.contains("defaultMessage:\"What's new\""));

        let frontend_i18n = std::fs::read_to_string(
            paths
                .app_root
                .join("resources")
                .join("ion-dist")
                .join("i18n")
                .join("zh-CN.json"),
        )
        .unwrap();
        assert_eq!(frontend_i18n, EMBEDDED_FRONTEND_I18N);
        assert!(frontend_i18n.len() > 1_000_000);
    }

    #[test]
    fn install_patch_rejects_incomplete_final_status() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);
        std::fs::remove_dir_all(
            paths
                .app_root
                .join("resources")
                .join("ion-dist")
                .join("assets"),
        )
        .unwrap();

        let error = install_patch_at(&paths).unwrap_err().to_string();

        assert!(error.contains("Claude Desktop 本机中文补丁未完整安装"));
    }

    #[test]
    fn status_rejects_legacy_runtime_patch_marker() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);
        install_patch_at(&paths).unwrap();
        let runtime_chunk = paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("assets")
            .join("index-demo.js");
        let runtime_chunk_text = format!(
            "{}\n{}",
            std::fs::read_to_string(&runtime_chunk).unwrap(),
            format!("/* {PATCH_MARKER} */")
        );
        std::fs::write(&runtime_chunk, runtime_chunk_text).unwrap();

        let status = status_for_paths(&paths);

        assert_eq!(status.status, "not_installed");
        assert!(!status.chunk_patch_present);
        assert!(status.resources_present);
        assert!(status.frontend_i18n_present);
        assert!(status.statsig_i18n_present);
        assert!(status.locale_configured);
    }

    #[test]
    fn status_rejects_runtime_chunk_missing_text_marker() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);
        install_patch_at(&paths).unwrap();
        let runtime_chunk = paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("assets")
            .join("index-demo.js");
        let runtime_chunk_text = std::fs::read_to_string(&runtime_chunk)
            .unwrap()
            .replace(TEXT_MARKER, "missing-claude-zh-text-marker");
        std::fs::write(&runtime_chunk, runtime_chunk_text).unwrap();

        let status = status_for_paths(&paths);

        assert_eq!(status.status, "not_installed");
        assert!(!status.chunk_patch_present);
    }

    #[test]
    fn restore_patch_restores_backed_up_files() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);
        let original = paths.app_root.join("resources").join("zh-CN.json");
        let chunk = paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("assets")
            .join("index-demo.js");
        let official_chunk = std::fs::read_to_string(&chunk).unwrap();
        std::fs::write(&original, "{\"original\":true}").unwrap();
        std::fs::create_dir_all(paths.locale_config_path.parent().unwrap()).unwrap();
        std::fs::write(&paths.locale_config_path, "{\"locale\":\"en-US\"}").unwrap();

        install_patch_at(&paths).unwrap();
        std::fs::write(atomic_temp_path_for(&original), b"stale-temp").unwrap();
        std::fs::write(atomic_backup_path_for(&original), b"stale-backup").unwrap();
        assert_ne!(std::fs::read_to_string(&chunk).unwrap(), official_chunk);
        restore_patch_at(&paths).unwrap();

        assert!(!original.exists());
        assert!(!atomic_temp_path_for(&original).exists());
        assert!(!atomic_backup_path_for(&original).exists());
        assert_eq!(
            std::fs::read_to_string(&paths.locale_config_path).unwrap(),
            "{}"
        );
        let patched_chunk = std::fs::read_to_string(&chunk).unwrap();
        assert_eq!(patched_chunk, official_chunk);
        assert!(!atomic_temp_path_for(&chunk).exists());
        assert!(!patched_chunk.contains("\"zh-CN\""));
        assert!(!patched_chunk.contains(PATCH_MARKER));
        assert!(!patched_chunk.contains(TEXT_MARKER));
        assert_eq!(status_for_paths(&paths).status, "not_installed");
    }

    #[test]
    fn paths_from_install_root_supports_app_subdirectory() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("Claude_1.0.0.0_x64__abcd");
        std::fs::create_dir_all(install_root.join("app").join("resources")).unwrap();

        let paths = paths_from_install_root(install_root.clone()).unwrap();

        assert_eq!(paths.install_root, install_root);
        assert!(paths.app_root.ends_with("app"));
    }

    #[test]
    fn paths_from_appx_install_location_supports_package_root() {
        let temp = tempfile::tempdir().unwrap();
        let appx_install_location = temp.path().join("Claude_1.0.0.0_x64__abcd");
        std::fs::create_dir_all(appx_install_location.join("app").join("resources")).unwrap();

        let paths = paths_from_install_root(appx_install_location.clone()).unwrap();

        assert_eq!(paths.install_root, appx_install_location);
        assert_eq!(paths.app_root, paths.install_root.join("app"));
    }

    #[test]
    fn paths_can_be_rebased_to_original_user_data_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("Claude_1.0.0.0_x64__abcd");
        std::fs::create_dir_all(install_root.join("app").join("resources")).unwrap();
        let appdata = temp.path().join("original-appdata");
        let local_appdata = temp.path().join("original-localappdata");

        let paths = paths_from_install_root(install_root)
            .unwrap()
            .with_user_data_dirs(Some(&appdata), Some(&local_appdata));

        assert_eq!(
            paths.locale_config_path,
            local_appdata.join("Claude-3p").join("config.json")
        );
        assert_eq!(paths.backup_dir, local_appdata.join(BACKUP_DIR_NAME));
    }

    #[test]
    fn install_root_paths_can_be_rebased_to_original_user_data_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("Claude_1.0.0.0_x64__abcd");
        std::fs::create_dir_all(install_root.join("app").join("resources")).unwrap();
        let appdata = temp.path().join("original-appdata");
        let local_appdata = temp.path().join("original-localappdata");

        let paths = paths_from_install_root(install_root.clone())
            .unwrap()
            .with_user_data_dirs(Some(&appdata), Some(&local_appdata));

        assert_eq!(paths.install_root, install_root);
        assert_eq!(paths.app_root, paths.install_root.join("app"));
        assert_eq!(
            paths.locale_config_path,
            local_appdata.join("Claude-3p").join("config.json")
        );
        assert_eq!(paths.backup_dir, local_appdata.join(BACKUP_DIR_NAME));
    }

    #[test]
    fn install_root_from_executable_path_supports_running_msix_app_path() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("Claude_1.0.0.0_x64__abcd");
        let app_root = install_root.join("app");
        std::fs::create_dir_all(app_root.join("resources")).unwrap();

        let detected = install_root_from_executable_path(&app_root.join("Claude.exe")).unwrap();

        assert_eq!(detected, install_root);
    }

    #[test]
    fn msix_unwritable_paths_require_elevation() {
        let temp = tempfile::tempdir().unwrap();
        let paths = ClaudeZhPatchPaths {
            install_root: temp
                .path()
                .join("WindowsApps")
                .join("Claude_1.0.0.0_x64__abcd"),
            app_root: temp
                .path()
                .join("WindowsApps")
                .join("Claude_1.0.0.0_x64__abcd")
                .join("app"),
            locale_config_path: temp.path().join("Claude-3p").join("config.json"),
            backup_dir: temp.path().join("backup"),
            install_kind: "msix".to_string(),
        };

        assert!(paths.patch_needs_elevation());
    }

    #[test]
    fn patch_target_dirs_include_all_resource_write_locations() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);
        let statsig_dir = paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("i18n")
            .join("statsig");
        std::fs::remove_dir_all(&statsig_dir).unwrap();

        let dirs = patch_target_dirs(&paths);

        assert!(dirs.contains(&paths.app_root.join("resources")));
        assert!(
            dirs.contains(
                &paths
                    .app_root
                    .join("resources")
                    .join("ion-dist")
                    .join("i18n")
            )
        );
        assert!(dirs.contains(&statsig_dir));
        assert!(
            dirs.contains(
                &paths
                    .app_root
                    .join("resources")
                    .join("ion-dist")
                    .join("assets")
            )
        );
        assert!(resource_tree_writable_or_create(&paths));
        assert!(statsig_dir.is_dir());
    }

    #[test]
    fn patch_target_files_include_all_zh_cn_resource_files() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);

        let files = patch_target_files(&paths);

        assert!(files.contains(&paths.app_root.join("resources").join("zh-CN.json")));
        assert!(
            files.contains(
                &paths
                    .app_root
                    .join("resources")
                    .join("ion-dist")
                    .join("i18n")
                    .join("zh-CN.json")
            )
        );
        assert!(
            files.contains(
                &paths
                    .app_root
                    .join("resources")
                    .join("ion-dist")
                    .join("i18n")
                    .join("statsig")
                    .join("zh-CN.json")
            )
        );
    }

    #[test]
    fn patch_access_targets_include_dirs_and_files() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);

        let targets = patch_access_targets(&paths);

        assert!(targets.contains(&paths.app_root.join("resources")));
        assert!(targets.contains(&paths.app_root.join("resources").join("zh-CN.json")));
        assert!(
            targets.contains(
                &paths
                    .app_root
                    .join("resources")
                    .join("ion-dist")
                    .join("i18n")
                    .join("zh-CN.json")
            )
        );
    }

    #[test]
    fn patch_needs_elevation_does_not_create_missing_statsig_dir() {
        let temp = tempfile::tempdir().unwrap();
        let mut paths = sample_paths(temp.path());
        paths.install_kind = "msix".to_string();
        create_sample_install(&paths);
        let statsig_dir = paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("i18n")
            .join("statsig");
        std::fs::remove_dir_all(&statsig_dir).unwrap();

        assert!(paths.patch_needs_elevation());
        assert!(!statsig_dir.exists());
    }

    #[test]
    fn status_for_paths_does_not_create_missing_statsig_dir() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);
        let statsig_dir = paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("i18n")
            .join("statsig");
        std::fs::remove_dir_all(&statsig_dir).unwrap();

        let status = status_for_paths(&paths);

        assert!(!status.writable);
        assert!(!statsig_dir.exists());
    }

    #[test]
    fn write_patch_file_removes_stale_atomic_temp_file() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("zh-CN.json");
        let temp_file = atomic_temp_path_for(&target);
        std::fs::write(&temp_file, b"stale").unwrap();

        write_patch_file(&target, b"{\"ok\":true}").unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"{\"ok\":true}");
        assert!(!temp_file.exists());
    }

    #[test]
    fn write_patch_file_uses_unique_temp_and_leaves_no_temp_sidecars() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("zh-CN.json");
        std::fs::write(atomic_temp_path_for(&target), b"stale").unwrap();

        write_patch_file(&target, b"new").unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        let sidecars = std::fs::read_dir(temp.path())
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(sidecars, vec!["zh-CN.json".to_string()]);
    }

    #[test]
    fn write_patch_file_replaces_existing_target_file() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("zh-CN.json");
        std::fs::write(&target, b"old").unwrap();
        let backup_file = atomic_backup_path_for(&target);

        write_patch_file(&target, b"new").unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(!atomic_temp_path_for(&target).exists());
        assert!(!backup_file.exists());
    }

    #[test]
    fn write_patch_file_clears_stale_backup_file() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("zh-CN.json");
        std::fs::write(&target, b"old").unwrap();
        std::fs::write(atomic_backup_path_for(&target), b"stale-backup").unwrap();

        write_patch_file(&target, b"new").unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(!atomic_backup_path_for(&target).exists());
    }

    #[test]
    fn write_patch_file_falls_back_to_direct_write_when_atomic_temp_path_is_blocked() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("zh-CN.json");
        std::fs::create_dir(atomic_temp_path_for(&target)).unwrap();

        write_patch_file(&target, b"new").unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(atomic_temp_path_for(&target).is_dir());
    }

    #[test]
    fn javascript_chunk_validation_rejects_broken_chunks_when_node_is_available() {
        if Command::new("node").arg("--version").output().is_err() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("broken.js");
        std::fs::write(&target, "const broken = {").unwrap();

        let error = validate_patched_javascript_chunk(&target)
            .expect_err("broken JavaScript should fail node --check");

        assert!(error.to_string().contains("node --check failed"));
    }

    #[test]
    fn javascript_chunk_validation_rejects_empty_chunks() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("empty.js");
        std::fs::write(&target, "").unwrap();

        let error = validate_patched_javascript_chunk(&target)
            .expect_err("empty JavaScript chunk should fail validation");

        assert!(error.to_string().contains("Claude JS chunk is empty"));
    }

    #[test]
    fn patch_locale_arrays_inserts_zh_cn_into_official_locale_list() {
        let source = r#"const locales=["en-US","de-DE","fr-FR","ko-KR","ja-JP","es-419","es-ES","it-IT","hi-IN","pt-BR","id-ID"];"#;

        let (patched, changed) = patch_locale_arrays(source);

        assert!(changed);
        assert!(patched.contains(r#""id-ID","zh-CN"]"#));
    }

    #[test]
    fn patch_locale_arrays_ignores_non_locale_arrays() {
        let source = r#"const labels=["en-US","not a locale","Settings"];"#;

        let (patched, changed) = patch_locale_arrays(source);

        assert!(!changed);
        assert_eq!(patched, source);
    }

    #[test]
    fn ensure_language_support_does_not_inject_script_when_no_locale_array_exists() {
        let source = r#"const countries=["AT","BE","FR","DE"]; console.log("Settings");"#;

        let patched = ensure_language_support(source.to_string());

        assert!(!patched.contains(LANGUAGE_MARKER));
        assert!(!patched.contains("__CLAUDE_CODEX_PRO_ZH_CN_LANGUAGE__"));
        assert!(!patched.contains(";window.__CLAUDE_CODEX_PRO_ZH_CN_LANGUAGE__"));
        assert!(patched.contains(r#"["AT","BE","FR","DE"]"#));
    }

    #[test]
    fn patch_chunk_replaces_legacy_window_runtime_patch() {
        let temp = tempfile::tempdir().unwrap();
        let chunk = temp.path().join("worker.js");
        std::fs::write(
            &chunk,
            format!(
                "console.log(\"Settings\");\n;window.__CLAUDE_CODEX_PRO_ZH_CN_LANGUAGE__ = \"{LANGUAGE_MARKER}\";\n;(() => {{\n  if (window.__CLAUDE_CODEX_PRO_ZH_CN_PATCH__) return;\n  window.__CLAUDE_CODEX_PRO_ZH_CN_PATCH__ = \"{PATCH_MARKER}\";\n}})();\n/* {TEXT_MARKER} */"
            ),
        )
        .unwrap();

        patch_chunk(&chunk, true).unwrap();

        let patched = std::fs::read_to_string(&chunk).unwrap();
        assert!(!patched.contains("typeof window === \"undefined\""));
        assert!(!patched.contains("g.__CLAUDE_CODEX_PRO_ZH_CN_PATCH__"));
        assert!(!patched.contains(PATCH_MARKER));
        assert!(!patched.contains(";window.__CLAUDE_CODEX_PRO_ZH_CN_LANGUAGE__"));
        assert!(!patched.contains("window.__CLAUDE_CODEX_PRO_ZH_CN_PATCH__"));
    }

    #[test]
    fn patch_chunk_removes_legacy_language_residue_from_non_runtime_chunks() {
        let temp = tempfile::tempdir().unwrap();
        let chunk = temp.path().join("worker.js");
        std::fs::write(
            &chunk,
            format!(
                "console.log(\"Settings\");\n/* claude-codex-pro-zh-cn-text-v3 */\n;(() => {{ if (typeof globalThis !== \"undefined\" && typeof window !== \"undefined\") globalThis.__CLAUDE_CODEX_PRO_ZH_CN_LANGUAGE__ = \"{LANGUAGE_MARKER}\"; }})();"
            ),
        )
        .unwrap();

        patch_chunk(&chunk, false).unwrap();

        let patched = std::fs::read_to_string(&chunk).unwrap();
        assert!(patched.contains("console.log(\"Settings\")"));
        assert!(patched.contains(TEXT_MARKER));
        assert!(!patched.contains("claude-codex-pro-zh-cn-text-v3"));
        assert!(!patched.contains(LANGUAGE_MARKER));
        assert!(!patched.contains("__CLAUDE_CODEX_PRO_ZH_CN_LANGUAGE__"));
    }

    #[test]
    fn replace_hardcoded_text_repatches_legacy_text_marker_with_new_provider_labels() {
        let source = format!(
            "{}\n/* claude-codex-pro-zh-cn-text */",
            r#"console.log("配置第三方推理"); console.log("第三方 URL"); const tab={label:"协作"};"#
        );

        let patched = replace_hardcoded_text(source);

        assert!(patched.contains(TEXT_MARKER));
        assert!(!patched.contains("/* claude-codex-pro-zh-cn-text */"));
        assert!(patched.contains("\"Configure third-party inference\""));
        assert!(patched.contains("\"Gateway base URL\""));
        assert!(patched.contains("label:\"Cowork\""));
    }

    #[test]
    fn replace_hardcoded_text_repatches_v2_text_marker_with_current_labels() {
        let source = format!(
            "{}\n/* claude-codex-pro-zh-cn-text-v2 */",
            r#"console.log("配置第三方推理"); console.log("第三方 URL"); console.log("新功能");"#
        );

        let patched = replace_hardcoded_text(source);

        assert!(patched.contains(TEXT_MARKER));
        assert!(!patched.contains("claude-codex-pro-zh-cn-text-v2"));
        assert!(patched.contains("\"Configure third-party inference\""));
        assert!(patched.contains("\"Gateway base URL\""));
        assert!(patched.contains("\"What's new\""));
    }

    #[test]
    fn replace_hardcoded_text_preserves_bundle_code() {
        let pairs = embedded_chunk_patch_pairs();
        assert_eq!(pairs.len(), 301);

        let source = r#"const settings=[
          "Configure third-party inference",
          "Gateway base URL",
          "Plugins & skills",
          "Open Setup",
          "New task",
          "Cowork",
          label:"Scheduled",
          title:"Scheduled tasks",subheader,
          message:"Scheduled tasks only run while your computer is awake."
        ];"#;

        let patched = replace_hardcoded_text(source.to_string());

        assert!(patched.contains(TEXT_MARKER));
        assert!(patched.contains("\"Configure third-party inference\""));
        assert!(patched.contains("\"Gateway base URL\""));
        assert!(patched.contains("\"Plugins & skills\""));
        assert!(patched.contains("\"Open Setup\""));
        assert!(patched.contains("\"New task\""));
        assert!(patched.contains("\"Cowork\""));
        assert!(patched.contains("label:\"Scheduled\""));
        assert!(patched.contains("title:\"Scheduled tasks\",subheader"));
        assert!(
            patched.contains("message:\"Scheduled tasks only run while your computer is awake.\"")
        );
    }

    #[test]
    fn elevated_access_preparation_is_limited_to_real_windowsapps_paths() {
        assert!(is_real_windows_apps_path(Path::new(
            r"C:\Program Files\WindowsApps\Claude_1.0.0.0_x64__abcd"
        )));
        assert!(!is_real_windows_apps_path(Path::new(
            r"C:\Users\Damon\AppData\Local\Temp\WindowsApps\Claude_1.0.0.0_x64__abcd"
        )));
        assert!(!is_real_windows_apps_path(Path::new(
            r"C:\Users\Damon\AppData\Local\AnthropicClaude"
        )));
    }

    #[test]
    fn windows_sid_principal_accepts_only_sid_syntax() {
        #[cfg(windows)]
        {
            assert_eq!(
                windows_sid_principal("S-1-5-21-1-2-3-1001").as_deref(),
                Some("*S-1-5-21-1-2-3-1001")
            );
            assert_eq!(windows_sid_principal("DAMON\\Damon"), None);
            assert_eq!(windows_sid_principal("& whoami"), None);
        }
    }

    #[test]
    fn elevated_install_skips_preflight_writable_probe() {
        let temp = tempfile::tempdir().unwrap();
        let paths = ClaudeZhPatchPaths {
            install_root: temp
                .path()
                .join("WindowsApps")
                .join("Claude_1.0.0.0_x64__abcd"),
            app_root: temp
                .path()
                .join("WindowsApps")
                .join("Claude_1.0.0.0_x64__abcd")
                .join("app"),
            locale_config_path: temp.path().join("Claude-3p").join("config.json"),
            backup_dir: temp.path().join("backup"),
            install_kind: "msix".to_string(),
        };
        create_sample_install(&paths);

        let outcome = install_patch_at_with_resources_elevated(&paths, None).unwrap();

        assert!(
            outcome
                .changed_files
                .iter()
                .any(|path| path.ends_with("resources\\zh-CN.json")
                    || path.ends_with("resources/zh-CN.json"))
        );
        let desktop_i18n =
            std::fs::read_to_string(paths.app_root.join("resources").join("zh-CN.json")).unwrap();
        assert_eq!(desktop_i18n, EMBEDDED_DESKTOP_I18N);
        assert!(
            serde_json::from_str::<serde_json::Value>(&desktop_i18n)
                .unwrap()
                .as_object()
                .unwrap()
                .len()
                > 100
        );
        assert!(
            std::fs::read_to_string(&paths.locale_config_path)
                .unwrap()
                .contains("zh-CN")
        );
    }

    #[test]
    fn install_patch_prefers_remote_resources_when_available() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);
        let embedded = embedded_i18n_resources();
        let resources = RemoteI18nResources {
            desktop: embedded
                .desktop
                .replace("\"Settings\"", "\"RemoteSettings\""),
            frontend: embedded
                .frontend
                .replace("\"Settings\"", "\"RemoteSettings\""),
            statsig: embedded.statsig.replace("\"reason\"", "\"remoteReason\""),
        };

        install_patch_at_with_resources(&paths, Some(&resources)).unwrap();

        assert_eq!(
            std::fs::read_to_string(paths.app_root.join("resources").join("zh-CN.json")).unwrap(),
            resources.desktop
        );
        assert_eq!(
            std::fs::read_to_string(
                paths
                    .app_root
                    .join("resources")
                    .join("ion-dist")
                    .join("i18n")
                    .join("zh-CN.json")
            )
            .unwrap(),
            resources.frontend
        );
        assert_eq!(
            std::fs::read_to_string(
                paths
                    .app_root
                    .join("resources")
                    .join("ion-dist")
                    .join("i18n")
                    .join("statsig")
                    .join("zh-CN.json")
            )
            .unwrap(),
            resources.statsig
        );
    }

    #[test]
    fn embedded_i18n_resources_are_full_reference_resources() {
        let resources = embedded_i18n_resources();

        assert!(resources.desktop.len() > 20_000);
        assert!(resources.frontend.len() > 1_000_000);
        assert!(resources.statsig.len() > 3_000);
        assert!(
            serde_json::from_str::<serde_json::Value>(&resources.desktop)
                .unwrap()
                .as_object()
                .unwrap()
                .len()
                > 100
        );
        assert!(
            serde_json::from_str::<serde_json::Value>(&resources.frontend)
                .unwrap()
                .as_object()
                .unwrap()
                .len()
                > 1_000
        );
        assert!(
            serde_json::from_str::<serde_json::Value>(&resources.statsig)
                .unwrap()
                .as_object()
                .unwrap()
                .len()
                > 10
        );
    }

    #[test]
    fn valid_i18n_resource_rejects_empty_or_invalid_json() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("zh-CN.json");

        std::fs::write(&path, "{}").unwrap();
        assert!(!valid_i18n_resource(&path));

        std::fs::write(&path, "not json").unwrap();
        assert!(!valid_i18n_resource(&path));

        std::fs::write(&path, "{\"Settings\":\"设置\"}").unwrap();
        assert!(valid_i18n_resource(&path));
        assert!(!valid_i18n_resource_with_min_keys(&path, 2));

        std::fs::write(&path, EMBEDDED_FRONTEND_I18N).unwrap();
        assert!(valid_i18n_resource_with_min_keys(&path, 1_000));
    }
}
