use std::collections::BTreeSet;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, Table};

pub const OPENAI_CURATED_MARKETPLACE: &str = "openai-curated";
pub const OPENAI_PLUGINS_ZIP_URL: &str =
    "https://codeload.github.com/openai/plugins/zip/refs/heads/main";
const OPENAI_PLUGINS_DOWNLOAD_LIMIT_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexPluginMarketplaceStatus {
    pub codex_home: String,
    pub marketplace_root: Option<String>,
    pub config_registered: bool,
    pub needs_repair: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexPluginMarketplaceRepair {
    pub codex_home: String,
    pub marketplace_root: Option<String>,
    pub initialized: bool,
    pub configured: bool,
    pub config_registered: bool,
    pub needs_repair: bool,
    pub message: String,
}

pub fn status() -> CodexPluginMarketplaceStatus {
    status_from_home(&crate::relay_config::default_codex_home_dir())
}

pub fn status_from_home(home: &Path) -> CodexPluginMarketplaceStatus {
    let marketplace_root = local_openai_curated_marketplace_root(home).ok().flatten();
    let config_registered = marketplace_root
        .as_deref()
        .map(|root| marketplace_config_points_to_root(home, OPENAI_CURATED_MARKETPLACE, root))
        .unwrap_or(false);
    let needs_repair = marketplace_root.is_none() || !config_registered;
    let message = match (marketplace_root.is_some(), config_registered) {
        (true, true) => "Codex OpenAI 插件仓库已下载并注册到 config.toml。".to_string(),
        (true, false) => "Codex OpenAI 插件仓库已下载，但尚未注册到 config.toml。".to_string(),
        (false, _) => "Codex OpenAI 插件仓库尚未下载。".to_string(),
    };

    CodexPluginMarketplaceStatus {
        codex_home: home.to_string_lossy().to_string(),
        marketplace_root: marketplace_root.map(|path| path.to_string_lossy().to_string()),
        config_registered,
        needs_repair,
        message,
    }
}

pub async fn repair() -> anyhow::Result<CodexPluginMarketplaceRepair> {
    repair_from_home(&crate::relay_config::default_codex_home_dir()).await
}

pub async fn repair_from_home(home: &Path) -> anyhow::Result<CodexPluginMarketplaceRepair> {
    let mut initialized = false;
    if local_openai_curated_marketplace_root(home)?.is_none() {
        initialize_openai_curated_marketplace_from_github(home).await?;
        initialized = true;
    }
    let configured = ensure_openai_curated_marketplace_config(home)?;
    let next = status_from_home(home);
    Ok(CodexPluginMarketplaceRepair {
        codex_home: next.codex_home,
        marketplace_root: next.marketplace_root,
        initialized,
        configured,
        config_registered: next.config_registered,
        needs_repair: next.needs_repair,
        message: if next.needs_repair {
            "Codex OpenAI 插件仓库修复后仍未通过状态检查。".to_string()
        } else if initialized || configured {
            "Codex OpenAI 插件仓库已下载并注册。重启 Codex 后插件页会重新读取。".to_string()
        } else {
            "Codex OpenAI 插件仓库已是最新状态。".to_string()
        },
    })
}

pub fn local_plugin_marketplaces() -> serde_json::Value {
    local_plugin_marketplaces_from_home(&crate::relay_config::default_codex_home_dir())
}

pub fn local_plugin_marketplaces_from_home(home: &Path) -> serde_json::Value {
    let installed_plugins = installed_plugins_from_config(home);
    let candidates = [home
        .join(".tmp")
        .join("plugins")
        .join(".agents")
        .join("plugins")
        .join("marketplace.json")];
    let marketplaces = candidates
        .iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(path).ok()?;
            let mut marketplace: serde_json::Value = serde_json::from_str(&text).ok()?;
            expand_local_plugin_marketplace(&mut marketplace, path, home, &installed_plugins);
            if let Some(object) = marketplace.as_object_mut() {
                object.entry("path").or_insert_with(|| {
                    serde_json::Value::String(path.to_string_lossy().to_string())
                });
            }
            Some(marketplace)
        })
        .collect::<Vec<_>>();
    serde_json::Value::Array(marketplaces)
}

pub fn ensure_openai_curated_marketplace_config(home: &Path) -> anyhow::Result<bool> {
    let Some(marketplace_root) = local_openai_curated_marketplace_root(home)? else {
        return Ok(false);
    };
    ensure_marketplace_config(home, OPENAI_CURATED_MARKETPLACE, &marketplace_root)
}

fn local_openai_curated_marketplace_root(home: &Path) -> anyhow::Result<Option<PathBuf>> {
    local_openai_curated_marketplace_root_from_root(&home.join(".tmp").join("plugins"))
}

async fn initialize_openai_curated_marketplace_from_github(home: &Path) -> anyhow::Result<()> {
    let bytes = download_openai_plugins_zip().await?;
    install_openai_plugins_zip(home, &bytes)
}

async fn download_openai_plugins_zip() -> anyhow::Result<Vec<u8>> {
    let client =
        crate::http_client::proxied_client(&format!("ClaudeCodexPro/{}", crate::version::VERSION))?;
    let bytes = client
        .get(OPENAI_PLUGINS_ZIP_URL)
        .header(reqwest::header::ACCEPT, "application/zip")
        .send()
        .await
        .context("failed to download openai/plugins marketplace")?
        .error_for_status()
        .context("openai/plugins marketplace download returned an error status")?
        .bytes()
        .await
        .context("failed to read openai/plugins marketplace download body")?;
    if bytes.len() > OPENAI_PLUGINS_DOWNLOAD_LIMIT_BYTES {
        anyhow::bail!(
            "openai/plugins marketplace download is too large: {} bytes",
            bytes.len()
        );
    }
    Ok(bytes.to_vec())
}

fn install_openai_plugins_zip(home: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let destination = home.join(".tmp").join("plugins");
    let staging_parent = home.join(".tmp");
    std::fs::create_dir_all(&staging_parent)
        .with_context(|| format!("failed to create {}", staging_parent.display()))?;
    let staging = staging_parent.join(format!(
        "plugins-download-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)
            .with_context(|| format!("failed to remove stale {}", staging.display()))?;
    }
    std::fs::create_dir_all(&staging)
        .with_context(|| format!("failed to create {}", staging.display()))?;

    let result = extract_openai_plugins_zip(bytes, &staging)
        .and_then(|_| validate_openai_plugins_marketplace_root(&staging))
        .and_then(|_| replace_directory(&staging, &destination));
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    result
}

fn extract_openai_plugins_zip(bytes: &[u8], destination: &Path) -> anyhow::Result<()> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("failed to read openai/plugins zip")?;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .with_context(|| format!("failed to read zip entry {index}"))?;
        let Some(relative_path) = zip_entry_relative_path(file.name()) else {
            continue;
        };
        let output_path = destination.join(relative_path);
        if file.is_dir() {
            std::fs::create_dir_all(&output_path)
                .with_context(|| format!("failed to create {}", output_path.display()))?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .with_context(|| format!("failed to read zip entry {}", file.name()))?;
        std::fs::write(&output_path, contents)
            .with_context(|| format!("failed to write {}", output_path.display()))?;
    }
    Ok(())
}

fn zip_entry_relative_path(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    let mut components = path.components();
    match components.next()? {
        Component::Normal(_) => {}
        _ => return None,
    }
    let mut relative = PathBuf::new();
    for component in components {
        match component {
            Component::Normal(value) => relative.push(value),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!relative.as_os_str().is_empty()).then_some(relative)
}

fn validate_openai_plugins_marketplace_root(root: &Path) -> anyhow::Result<()> {
    let marketplace = local_openai_curated_marketplace_root_from_root(root)?
        .ok_or_else(|| anyhow::anyhow!("downloaded openai/plugins marketplace is invalid"))?;
    if marketplace != root {
        anyhow::bail!("downloaded openai/plugins marketplace root mismatch");
    }
    validate_openai_plugins_marketplace_entries(root)?;
    Ok(())
}

fn local_openai_curated_marketplace_root_from_root(root: &Path) -> anyhow::Result<Option<PathBuf>> {
    let marketplace_path = root
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");
    if !marketplace_path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&marketplace_path)
        .with_context(|| format!("failed to read {}", marketplace_path.display()))?;
    let marketplace: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", marketplace_path.display()))?;
    if marketplace.get("name").and_then(serde_json::Value::as_str)
        != Some(OPENAI_CURATED_MARKETPLACE)
    {
        return Ok(None);
    }
    let has_plugins = marketplace
        .get("plugins")
        .and_then(serde_json::Value::as_array)
        .map(|plugins| !plugins.is_empty())
        .unwrap_or(false);
    if !has_plugins || !root.join("plugins").is_dir() {
        return Ok(None);
    }
    Ok(Some(root.to_path_buf()))
}

fn validate_openai_plugins_marketplace_entries(root: &Path) -> anyhow::Result<()> {
    let marketplace_path = root
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");
    let text = std::fs::read_to_string(&marketplace_path)
        .with_context(|| format!("failed to read {}", marketplace_path.display()))?;
    let marketplace: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", marketplace_path.display()))?;
    let plugins = marketplace
        .get("plugins")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("downloaded openai/plugins marketplace has no plugins"))?;
    for plugin in plugins {
        let name = plugin
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                plugin
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|id| id.split('@').next())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .ok_or_else(|| anyhow::anyhow!("downloaded openai/plugins marketplace has an unnamed plugin"))?;
        let plugin_root = plugin_marketplace_entry_source_path(plugin)
            .and_then(|path| plugin_marketplace_entry_path(root, path))
            .unwrap_or_else(|| root.join("plugins").join(name));
        let manifest = plugin_root.join(".codex-plugin").join("plugin.json");
        if !manifest.is_file() {
            anyhow::bail!(
                "downloaded openai/plugins marketplace missing Codex plugin manifest for {name}: {}",
                manifest.display()
            );
        }
    }
    Ok(())
}

fn plugin_marketplace_entry_source_path(plugin: &serde_json::Value) -> Option<&str> {
    plugin
        .get("path")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            plugin
                .get("source")
                .and_then(serde_json::Value::as_object)
                .and_then(|source| source.get("path"))
                .and_then(serde_json::Value::as_str)
        })
}

fn plugin_marketplace_entry_path(root: &Path, value: &str) -> Option<PathBuf> {
    let trimmed = value.trim().strip_prefix("./").unwrap_or(value.trim());
    if trimmed.is_empty() || Path::new(trimmed).is_absolute() {
        return None;
    }
    let mut relative = PathBuf::new();
    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(value) => relative.push(value),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!relative.as_os_str().is_empty()).then(|| root.join(relative))
}

fn replace_directory(source: &Path, destination: &Path) -> anyhow::Result<()> {
    let backup = destination.with_file_name("plugins.previous-claude-codex-pro");
    if backup.exists() {
        std::fs::remove_dir_all(&backup)
            .with_context(|| format!("failed to remove {}", backup.display()))?;
    }
    if destination.exists() {
        std::fs::rename(destination, &backup).with_context(|| {
            format!(
                "failed to move {} to {}",
                destination.display(),
                backup.display()
            )
        })?;
    }
    match std::fs::rename(source, destination) {
        Ok(()) => {
            if backup.exists() {
                let _ = std::fs::remove_dir_all(&backup);
            }
            Ok(())
        }
        Err(error) => {
            if backup.exists() {
                let _ = std::fs::rename(&backup, destination);
            }
            Err(error).with_context(|| {
                format!(
                    "failed to move {} to {}",
                    source.display(),
                    destination.display()
                )
            })
        }
    }
}

fn ensure_marketplace_config(
    home: &Path,
    marketplace_name: &str,
    marketplace_root: &Path,
) -> anyhow::Result<bool> {
    let config_path = home.join("config.toml");
    let existing = match std::fs::read(&config_path) {
        Ok(bytes) => String::from_utf8(bytes)
            .with_context(|| format!("failed to read UTF-8 {}", config_path.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", config_path.display()));
        }
    };
    let without_bom = existing.trim_start_matches('\u{feff}');
    let mut doc = parse_toml_document(without_bom)?;
    let marketplaces = table_mut_or_insert(&mut doc, "marketplaces")?;
    if marketplaces
        .get(marketplace_name)
        .and_then(Item::as_table)
        .is_none()
    {
        marketplaces[marketplace_name] = toml_edit::table();
    }
    marketplaces[marketplace_name]["source_type"] = toml_edit::value("local");
    marketplaces[marketplace_name]["source"] =
        toml_edit::value(windows_extended_path(marketplace_root));

    let updated = ensure_trailing_newline(doc.to_string());
    if updated.as_bytes() == without_bom.as_bytes() {
        return Ok(false);
    }
    crate::settings::atomic_write(&config_path, updated.as_bytes())?;
    Ok(true)
}

fn marketplace_config_points_to_root(home: &Path, marketplace_name: &str, root: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(home.join("config.toml")) else {
        return false;
    };
    let Ok(doc) = text.trim_start_matches('\u{feff}').parse::<DocumentMut>() else {
        return false;
    };
    let Some(table) = doc
        .get("marketplaces")
        .and_then(Item::as_table)
        .and_then(|marketplaces| marketplaces.get(marketplace_name))
        .and_then(Item::as_table)
    else {
        return false;
    };
    let source_type = table
        .get("source_type")
        .and_then(Item::as_str)
        .unwrap_or_default();
    let source = table
        .get("source")
        .and_then(Item::as_str)
        .unwrap_or_default();
    source_type == "local" && normalize_windows_extended_path(source) == root.to_string_lossy()
}

fn expand_local_plugin_marketplace(
    marketplace: &mut serde_json::Value,
    marketplace_path: &Path,
    home: &Path,
    installed_plugins: &std::collections::BTreeSet<String>,
) {
    let marketplace_name = marketplace
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let Some(plugins) = marketplace
        .get_mut("plugins")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    let marketplace_root = marketplace_path
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".tmp").join("plugins"));
    for plugin in plugins {
        let source_path = plugin_marketplace_entry_source_path(plugin).map(str::to_string);
        let Some(plugin_object) = plugin.as_object_mut() else {
            continue;
        };
        let plugin_name = plugin_object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                plugin_object
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|id| id.split('@').next())
                    .map(str::to_string)
            })
            .unwrap_or_default();
        if plugin_name.is_empty() {
            continue;
        }
        let plugin_root = source_path
            .as_deref()
            .and_then(|path| plugin_marketplace_entry_path(&marketplace_root, path))
            .unwrap_or_else(|| marketplace_root.join("plugins").join(&plugin_name));
        let manifest_path = plugin_root.join(".codex-plugin").join("plugin.json");
        if let Some(manifest) = plugin_manifest(&manifest_path) {
            merge_plugin_manifest(plugin_object, manifest);
        }
        absolutize_plugin_icon_paths(plugin_object, &plugin_root);
        plugin_object
            .entry("name".to_string())
            .or_insert_with(|| serde_json::Value::String(plugin_name.clone()));
        plugin_object.entry("id".to_string()).or_insert_with(|| {
            serde_json::Value::String(format!("{plugin_name}@{marketplace_name}"))
        });
        plugin_object
            .entry("keywords".to_string())
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        plugin_object.insert(
            "installed".to_string(),
            serde_json::Value::Bool(
                installed_plugins.contains(&format!("{plugin_name}@{marketplace_name}")),
            ),
        );
    }
}

fn absolutize_plugin_icon_paths(
    plugin: &mut serde_json::Map<String, serde_json::Value>,
    plugin_root: &Path,
) {
    for key in ["composerIconPath", "logoPath"] {
        absolutize_string_field(plugin, key, plugin_root);
    }
    let Some(interface) = plugin
        .get_mut("interface")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    for key in ["composerIcon", "composerIconUrl", "logo", "logoUrl"] {
        absolutize_string_field(interface, key, plugin_root);
    }
}

fn absolutize_string_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    root: &Path,
) {
    let Some(value) = object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
    else {
        return;
    };
    let Some(path) = absolutize_plugin_asset_path(&value, root) else {
        return;
    };
    object.insert(key.to_string(), serde_json::Value::String(path));
}

fn absolutize_plugin_asset_path(value: &str, root: &Path) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("data:")
        || trimmed.starts_with("http:")
        || trimmed.starts_with("https:")
        || trimmed.starts_with("file:")
        || Path::new(trimmed).is_absolute()
    {
        return None;
    }
    let relative = trimmed.strip_prefix("./").unwrap_or(trimmed);
    Some(root.join(relative).to_string_lossy().to_string())
}

fn plugin_manifest(path: &Path) -> Option<serde_json::Map<String, serde_json::Value>> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()?
        .as_object()
        .cloned()
}

fn merge_plugin_manifest(
    plugin: &mut serde_json::Map<String, serde_json::Value>,
    manifest: serde_json::Map<String, serde_json::Value>,
) {
    for (key, value) in manifest {
        plugin.entry(key).or_insert(value);
    }
}

fn installed_plugins_from_config(home: &Path) -> BTreeSet<String> {
    let text = std::fs::read_to_string(home.join("config.toml")).unwrap_or_default();
    let doc = text.parse::<toml_edit::DocumentMut>().ok();
    let Some(plugins) = doc
        .as_ref()
        .and_then(|doc| doc.get("plugins"))
        .and_then(toml_edit::Item::as_table)
    else {
        return BTreeSet::new();
    };
    plugins
        .iter()
        .filter_map(|(id, item)| {
            let enabled = item
                .get("enabled")
                .and_then(toml_edit::Item::as_bool)
                .unwrap_or(false);
            enabled.then(|| id.to_string())
        })
        .collect()
}

fn normalize_windows_extended_path(value: &str) -> String {
    value.strip_prefix(r"\\?\").unwrap_or(value).to_string()
}

fn windows_extended_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if !cfg!(windows) || value.starts_with(r"\\?\") {
        value.into_owned()
    } else {
        format!(r"\\?\{value}")
    }
}

fn parse_toml_document(contents: &str) -> anyhow::Result<DocumentMut> {
    if contents.trim().is_empty() {
        Ok(DocumentMut::new())
    } else {
        contents
            .parse::<DocumentMut>()
            .with_context(|| "config.toml TOML parse failed")
    }
}

fn table_mut_or_insert<'a>(doc: &'a mut DocumentMut, key: &str) -> anyhow::Result<&'a mut Table> {
    if !doc.as_table().contains_key(key) {
        doc[key] = toml_edit::table();
    }
    if doc.get(key).and_then(Item::as_table).is_none() {
        doc[key] = toml_edit::table();
    }
    doc.get_mut(key)
        .and_then(Item::as_table_mut)
        .ok_or_else(|| anyhow::anyhow!("{key} must be a TOML table"))
}

fn ensure_trailing_newline(mut contents: String) -> String {
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_marketplace(home: &Path) {
        let root = home.join(".tmp").join("plugins");
        std::fs::create_dir_all(root.join(".agents").join("plugins")).unwrap();
        std::fs::create_dir_all(root.join("plugins").join("gmail")).unwrap();
        std::fs::write(
            root.join(".agents")
                .join("plugins")
                .join("marketplace.json"),
            r#"{"name":"openai-curated","plugins":[{"name":"gmail","source":{"source":"local","path":"./plugins/gmail"}}]}"#,
        )
        .unwrap();
    }

    #[test]
    fn status_detects_missing_snapshot() {
        let temp = tempfile::tempdir().unwrap();

        let status = status_from_home(temp.path());

        assert!(status.needs_repair);
        assert!(status.marketplace_root.is_none());
        assert!(!status.config_registered);
    }

    #[test]
    fn ensure_config_registers_local_marketplace() {
        let temp = tempfile::tempdir().unwrap();
        write_marketplace(temp.path());

        let changed = ensure_openai_curated_marketplace_config(temp.path()).unwrap();

        assert!(changed);
        let status = status_from_home(temp.path());
        assert!(!status.needs_repair);
        assert!(status.config_registered);
    }

    #[test]
    fn zip_entry_relative_path_strips_archive_root_and_rejects_escape() {
        assert_eq!(
            zip_entry_relative_path("plugins-main/plugins/gmail/file.txt"),
            Some(PathBuf::from("plugins").join("gmail").join("file.txt"))
        );
        assert_eq!(zip_entry_relative_path("plugins-main/../evil.txt"), None);
        assert_eq!(zip_entry_relative_path("../evil.txt"), None);
    }

    #[test]
    fn install_zip_installs_valid_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let mut bytes = Cursor::new(Vec::<u8>::new());
        {
            let mut writer = zip::ZipWriter::new(&mut bytes);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer
                .start_file("plugins-main/.agents/plugins/marketplace.json", options)
                .unwrap();
            std::io::Write::write_all(
                &mut writer,
                br#"{"name":"openai-curated","plugins":[{"name":"gmail","source":{"source":"local","path":"./plugins/gmail"}}]}"#,
            )
            .unwrap();
            writer
                .start_file(
                    "plugins-main/plugins/gmail/.codex-plugin/plugin.json",
                    options,
                )
                .unwrap();
            std::io::Write::write_all(&mut writer, br#"{"name":"gmail"}"#).unwrap();
            writer.finish().unwrap();
        }

        install_openai_plugins_zip(temp.path(), bytes.get_ref()).unwrap();
        ensure_openai_curated_marketplace_config(temp.path()).unwrap();
        let status = status_from_home(temp.path());

        assert!(!status.needs_repair);
        assert!(
            temp.path()
                .join(".tmp/plugins/.agents/plugins/marketplace.json")
                .is_file()
        );
    }

    #[test]
    fn install_zip_rejects_marketplace_without_plugin_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let mut bytes = Cursor::new(Vec::<u8>::new());
        {
            let mut writer = zip::ZipWriter::new(&mut bytes);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer
                .start_file("plugins-main/.agents/plugins/marketplace.json", options)
                .unwrap();
            std::io::Write::write_all(
                &mut writer,
                br#"{"name":"openai-curated","plugins":[{"name":"gmail","source":{"source":"local","path":"./plugins/gmail"}}]}"#,
            )
            .unwrap();
            writer.start_file("plugins-main/plugins/.keep", options).unwrap();
            std::io::Write::write_all(&mut writer, b"").unwrap();
            writer.finish().unwrap();
        }

        let error = install_openai_plugins_zip(temp.path(), bytes.get_ref())
            .expect_err("incomplete marketplace should be rejected");

        assert!(
            error
                .to_string()
                .contains("missing Codex plugin manifest for gmail")
        );
        assert!(
            !temp
                .path()
                .join(".tmp/plugins/.agents/plugins/marketplace.json")
                .exists()
        );
    }

    #[test]
    fn local_marketplaces_expand_plugin_manifest() {
        let temp = tempfile::tempdir().unwrap();
        write_marketplace(temp.path());
        std::fs::create_dir_all(temp.path().join(".tmp/plugins/plugins/gmail/.codex-plugin"))
            .unwrap();
        std::fs::write(
            temp.path()
                .join(".tmp/plugins/plugins/gmail/.codex-plugin/plugin.json"),
            r#"{"description":"Gmail plugin","logoPath":"./logo.png"}"#,
        )
        .unwrap();

        let marketplaces = local_plugin_marketplaces_from_home(temp.path());

        assert_eq!(marketplaces[0]["plugins"][0]["description"], "Gmail plugin");
        assert!(
            marketplaces[0]["plugins"][0]["logoPath"]
                .as_str()
                .unwrap()
                .contains("plugins")
        );
    }

    #[test]
    fn local_marketplaces_use_nested_source_path_for_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".tmp").join("plugins");
        std::fs::create_dir_all(root.join(".agents").join("plugins")).unwrap();
        std::fs::create_dir_all(root.join("plugins").join("actual-dir").join(".codex-plugin"))
            .unwrap();
        std::fs::write(
            root.join(".agents")
                .join("plugins")
                .join("marketplace.json"),
            r#"{"name":"openai-curated","plugins":[{"name":"demo","source":{"source":"local","path":"./plugins/actual-dir"}}]}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("plugins")
                .join("actual-dir")
                .join(".codex-plugin")
                .join("plugin.json"),
            r#"{"description":"Nested source path plugin"}"#,
        )
        .unwrap();

        let marketplaces = local_plugin_marketplaces_from_home(temp.path());

        assert_eq!(
            marketplaces[0]["plugins"][0]["description"],
            "Nested source path plugin"
        );
    }
}
