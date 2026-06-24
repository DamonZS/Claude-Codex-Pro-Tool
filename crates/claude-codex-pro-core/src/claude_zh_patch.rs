use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

const PATCH_MARKER: &str = "claude-codex-pro-zh-cn-patch";
const LANGUAGE_MARKER: &str = "claude-codex-pro-zh-cn-language";
const TEXT_MARKER: &str = "claude-codex-pro-zh-cn-text";
const BACKUP_DIR_NAME: &str = "Claude-zh-CN-official-backup";
const DESKTOP_I18N_URL: &str = "https://raw.githubusercontent.com/Jyy1529/claude-desktop_win-zh_cn/master/resources/desktop-zh-CN.json";
const FRONTEND_I18N_URL: &str = "https://raw.githubusercontent.com/Jyy1529/claude-desktop_win-zh_cn/master/resources/frontend-zh-CN.json";
const STATSIG_I18N_URL: &str = "https://raw.githubusercontent.com/Jyy1529/claude-desktop_win-zh_cn/master/resources/statsig-zh-CN.json";

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

pub fn install_patch() -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = detect_paths().ok_or_else(|| anyhow::anyhow!("未找到 Claude Desktop 安装目录"))?;
    install_patch_at(&paths)
}

pub async fn install_patch_with_remote_resources() -> anyhow::Result<ClaudeZhPatchOutcome> {
    let paths = detect_paths().ok_or_else(|| anyhow::anyhow!("未找到 Claude Desktop 安装目录"))?;
    let resources = fetch_remote_i18n_resources().await.ok();
    install_patch_at_with_resources(&paths, resources.as_ref())
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

pub fn install_patch_at(paths: &ClaudeZhPatchPaths) -> anyhow::Result<ClaudeZhPatchOutcome> {
    install_patch_at_with_resources(paths, None)
}

pub fn install_patch_at_with_resources(
    paths: &ClaudeZhPatchPaths,
    resources: Option<&RemoteI18nResources>,
) -> anyhow::Result<ClaudeZhPatchOutcome> {
    let mut changed_files = Vec::new();
    ensure_patch_writable(paths)?;
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
    crate::settings::atomic_write(&root_i18n, desktop_i18n.as_bytes())?;
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
    crate::settings::atomic_write(&frontend_i18n, frontend_i18n_contents.as_bytes())?;
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
    crate::settings::atomic_write(&statsig_i18n, statsig_i18n_contents.as_bytes())?;
    changed_files.push(statsig_i18n.to_string_lossy().to_string());

    backup_file(&paths.locale_config_path, paths)?;
    write_locale_config(&paths.locale_config_path)?;
    changed_files.push(paths.locale_config_path.to_string_lossy().to_string());

    for chunk in find_patchable_chunks(&paths.app_root)? {
        let before = std::fs::read_to_string(&chunk).unwrap_or_default();
        backup_file(&chunk, paths)?;
        patch_chunk(&chunk)?;
        let after = std::fs::read_to_string(&chunk).unwrap_or_default();
        if before != after {
            changed_files.push(chunk.to_string_lossy().to_string());
        }
    }

    Ok(ClaudeZhPatchOutcome {
        status: status_for_paths(paths),
        backup_dir: paths.backup_dir.to_string_lossy().to_string(),
        changed_files,
    })
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
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&from, &to)?;
            changed_files.push(to.to_string_lossy().to_string());
        }
    }
    Ok(ClaudeZhPatchOutcome {
        status: status_for_paths(paths),
        backup_dir: paths.backup_dir.to_string_lossy().to_string(),
        changed_files,
    })
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
    let chunk_patch_present = chunk_texts.iter().any(|text| text.contains(PATCH_MARKER));
    let language_whitelist_patched = chunk_texts
        .iter()
        .any(|text| text.contains(LANGUAGE_MARKER) || has_zh_cn_language_support(text));
    let writable = resource_tree_writable(paths);
    let locale_configured = locale_configured(&paths.locale_config_path);
    let resources_present = valid_i18n_resource(&root_i18n);
    let frontend_i18n_present = valid_i18n_resource(&frontend_i18n);
    let statsig_i18n_present = valid_i18n_resource(&statsig_i18n);
    let ready = resources_present
        && frontend_i18n_present
        && statsig_i18n_present
        && locale_configured
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

fn candidate_install_roots() -> Vec<PathBuf> {
    let mut candidates = running_claude_install_roots();
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
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
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
    if resource_tree_writable(paths) {
        return Ok(());
    }
    anyhow::bail!(
        "Claude Desktop 安装目录已找到，但资源目录不可写：{}。如果这是 Microsoft Store/MSIX 版本，WindowsApps 受系统保护，不能直接写入汉化资源；请使用“Claude 中文窗口”，或安装可写入的桌面版后再执行本机汉化。",
        paths.app_root.join("resources").display()
    );
}

fn resource_tree_writable(paths: &ClaudeZhPatchPaths) -> bool {
    let resources = paths.app_root.join("resources");
    if !resources.is_dir() {
        return false;
    }
    let probe = resources.join(format!(
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
    let mut chunks = std::fs::read_dir(assets)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("js"))
        .collect::<Vec<_>>();
    chunks.sort();
    Ok(chunks
        .into_iter()
        .filter(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            name.starts_with("index-") || name.starts_with("assets-") || name.starts_with("main-")
        })
        .collect())
}

fn patch_chunk(path: &Path) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(path)?;
    if text.contains(PATCH_MARKER)
        && has_zh_cn_language_support(&text)
        && text.contains(TEXT_MARKER)
    {
        return Ok(());
    }
    let mut patched = ensure_language_support(text);
    patched = replace_hardcoded_text(patched);
    if !patched.contains(PATCH_MARKER) {
        patched = format!("{patched}\n{}", runtime_patch_script());
    }
    crate::settings::atomic_write(path, patched.as_bytes())
}

fn ensure_language_support(text: String) -> String {
    if has_zh_cn_language_support(&text) {
        return text;
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
    format!("{patched}\n;window.__CLAUDE_CODEX_PRO_ZH_CN_LANGUAGE__ = \"{LANGUAGE_MARKER}\";")
}

fn has_zh_cn_language_support(text: &str) -> bool {
    text.contains("\"zh-CN\"") || text.contains("'zh-CN'") || text.contains(LANGUAGE_MARKER)
}

fn replace_hardcoded_text(mut text: String) -> String {
    if text.contains(TEXT_MARKER) {
        return text;
    }
    for (english, chinese) in zh_text_pairs() {
        let double_quoted = format!("\"{english}\"");
        let single_quoted = format!("'{english}'");
        if text.contains(&double_quoted) {
            text = text.replace(&double_quoted, &format!("\"{chinese}\""));
        }
        if text.contains(&single_quoted) {
            text = text.replace(&single_quoted, &format!("'{chinese}'"));
        }
    }
    format!("{text}\n/* {TEXT_MARKER} */")
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
        ("Recents", "最近"),
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
    ]
}

fn runtime_patch_script() -> String {
    let dict = serde_json::to_string(zh_text_pairs()).unwrap_or_else(|_| "[]".to_string());
    r#";(() => {
  if (window.__CLAUDE_CODEX_PRO_ZH_CN_PATCH__) return;
  window.__CLAUDE_CODEX_PRO_ZH_CN_PATCH__ = "claude-codex-pro-zh-cn-patch";
  const dict = new Map(__CLAUDE_CODEX_PRO_ZH_CN_DICT__);
  const apply = (root = document.body) => {
    if (!root) return;
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    let node;
    while ((node = walker.nextNode())) {
      const value = node.nodeValue && node.nodeValue.trim();
      if (value && dict.has(value)) node.nodeValue = node.nodeValue.replace(value, dict.get(value));
    }
    root.querySelectorAll?.("[aria-label],[title],[placeholder]").forEach((el) => {
      for (const attr of ["aria-label", "title", "placeholder"]) {
        const value = el.getAttribute(attr);
        if (value && dict.has(value.trim())) el.setAttribute(attr, dict.get(value.trim()));
      }
    });
  };
  apply();
  new MutationObserver((changes) => {
    for (const change of changes) change.addedNodes.forEach((node) => node.nodeType === 1 && apply(node));
  }).observe(document.documentElement, { childList: true, subtree: true });
})();"#
    .replace("__CLAUDE_CODEX_PRO_ZH_CN_DICT__", &dict)
}

fn desktop_i18n_json() -> String {
    serde_json::to_string_pretty(&json!({
        "locale": "zh-CN",
        "language": "简体中文",
        "app_name": "Claude",
        "settings": "设置"
    }))
    .unwrap()
}

fn frontend_i18n_json() -> String {
    let mut map = serde_json::Map::new();
    map.insert("locale".to_string(), json!("zh-CN"));
    for (english, chinese) in zh_text_pairs() {
        map.insert((*english).to_string(), json!(chinese));
    }
    serde_json::to_string_pretty(&serde_json::Value::Object(map)).unwrap()
}

fn statsig_i18n_json() -> String {
    serde_json::to_string_pretty(&json!({
        "locale": "zh-CN"
    }))
    .unwrap()
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

fn valid_i18n_resource(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.as_object().map(|object| !object.is_empty()))
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
        let assets = paths
            .app_root
            .join("resources")
            .join("ion-dist")
            .join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(
            paths.app_root.join("resources").join("zh-CN.json"),
            "{\"locale\":\"zh-CN\"}",
        )
        .unwrap();
        std::fs::write(
            assets.join("index-demo.js"),
            "const locales=[\"en-US\",\"fr-FR\"]; console.log('Settings'); console.log(\"Inference configuration\");",
        )
        .unwrap();
    }

    #[test]
    fn install_patch_writes_resources_locale_and_runtime_marker() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);

        let outcome = install_patch_at(&paths).unwrap();

        assert_eq!(outcome.status.status, "ok");
        assert!(outcome.status.resources_present);
        assert!(outcome.status.frontend_i18n_present);
        assert!(outcome.status.statsig_i18n_present);
        assert!(outcome.status.locale_configured);
        assert!(outcome.status.chunk_patch_present);
        assert!(outcome.status.language_whitelist_patched);
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
        assert!(patched_chunk.contains(PATCH_MARKER));
        assert!(patched_chunk.contains(TEXT_MARKER));
        assert!(patched_chunk.contains("\"zh-CN\""));
        assert!(patched_chunk.contains("'设置'"));
        assert!(patched_chunk.contains("\"推理配置\""));

        let frontend_i18n = std::fs::read_to_string(
            paths
                .app_root
                .join("resources")
                .join("ion-dist")
                .join("i18n")
                .join("zh-CN.json"),
        )
        .unwrap();
        assert!(frontend_i18n.contains("Inference configuration"));
        assert!(frontend_i18n.contains("推理配置"));
    }

    #[test]
    fn restore_patch_restores_backed_up_files() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);
        let original = paths.app_root.join("resources").join("zh-CN.json");
        std::fs::write(&original, "{\"original\":true}").unwrap();
        std::fs::create_dir_all(paths.locale_config_path.parent().unwrap()).unwrap();
        std::fs::write(&paths.locale_config_path, "{\"locale\":\"en-US\"}").unwrap();

        install_patch_at(&paths).unwrap();
        restore_patch_at(&paths).unwrap();

        assert_eq!(
            std::fs::read_to_string(original).unwrap(),
            "{\"original\":true}"
        );
        assert_eq!(
            std::fs::read_to_string(&paths.locale_config_path).unwrap(),
            "{\"locale\":\"en-US\"}"
        );
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
    fn install_root_from_executable_path_supports_running_msix_app_path() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("Claude_1.0.0.0_x64__abcd");
        let app_root = install_root.join("app");
        std::fs::create_dir_all(app_root.join("resources")).unwrap();

        let detected = install_root_from_executable_path(&app_root.join("Claude.exe")).unwrap();

        assert_eq!(detected, install_root);
    }

    #[test]
    fn install_patch_prefers_remote_resources_when_available() {
        let temp = tempfile::tempdir().unwrap();
        let paths = sample_paths(temp.path());
        create_sample_install(&paths);
        let resources = RemoteI18nResources {
            desktop: "{\"remoteDesktop\":\"桌面\"}".to_string(),
            frontend: "{\"remoteFrontend\":\"前端\"}".to_string(),
            statsig: "{\"remoteStatsig\":\"统计\"}".to_string(),
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
    fn valid_i18n_resource_rejects_empty_or_invalid_json() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("zh-CN.json");

        std::fs::write(&path, "{}").unwrap();
        assert!(!valid_i18n_resource(&path));

        std::fs::write(&path, "not json").unwrap();
        assert!(!valid_i18n_resource(&path));

        std::fs::write(&path, "{\"Settings\":\"设置\"}").unwrap();
        assert!(valid_i18n_resource(&path));
    }
}
