use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use toml_edit::DocumentMut;

const MAX_SCAN_DEPTH: usize = 12;
const MAX_SCAN_ENTRIES: usize = 20_000;
const MAX_METADATA_BYTES: usize = 64 * 1024;
const MAX_CONFIG_BYTES: usize = 8 * 1024 * 1024;
const DISABLED_DIR: &str = ".ccp-disabled";
const DISABLED_CLAUDE_MCP_SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedToolInventoryRoots {
    pub codex_home: PathBuf,
    pub claude_home: PathBuf,
    pub claude_config_paths: Vec<PathBuf>,
}

impl Default for UnifiedToolInventoryRoots {
    fn default() -> Self {
        let home = directories::BaseDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let codex_home = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex"));
        let claude_home = home.join(".claude");
        let mut claude_config_paths = vec![
            home.join(".claude.json"),
            home.join(".mcp.json"),
            claude_home_path(&home).join("settings.json"),
            claude_home_path(&home).join("mcp.json"),
        ];

        if let Some(app_data) = std::env::var_os("APPDATA") {
            claude_config_paths.push(
                PathBuf::from(app_data)
                    .join("Claude")
                    .join("claude_desktop_config.json"),
            );
        }
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let local_app_data = PathBuf::from(local_app_data);
            claude_config_paths.push(
                local_app_data
                    .join("Claude-3p")
                    .join("claude_desktop_config.json"),
            );
            let packages = local_app_data.join("Packages");
            if let Ok(entries) = std::fs::read_dir(&packages) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("Claude_") {
                        claude_config_paths.push(
                            entry
                                .path()
                                .join("LocalCache")
                                .join("Roaming")
                                .join("Claude-3p")
                                .join("claude_desktop_config.json"),
                        );
                    }
                }
            }
        }
        dedupe_paths(&mut claude_config_paths);
        Self {
            codex_home,
            claude_home,
            claude_config_paths,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedToolAppState {
    pub enabled: bool,
    pub available: bool,
    pub toggle_supported: bool,
    pub source_path: String,
    #[serde(skip_serializing)]
    pub config_body: String,
    #[serde(skip_serializing)]
    pub config_id: String,
    #[serde(skip_serializing)]
    pub restore_body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisabledClaudeMcpSnapshot {
    version: u32,
    id: String,
    entries: Vec<DisabledClaudeMcpEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisabledClaudeMcpEntry {
    config_path: String,
    project: Option<String>,
    server_id: String,
    server: Value,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedToolAsset {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub source: String,
    pub claude: UnifiedToolAppState,
    pub codex: UnifiedToolAppState,
    #[serde(skip_serializing)]
    pub discovery_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedToolInventoryCounts {
    pub total: usize,
    pub raw_discoveries: usize,
    pub deduplicated: usize,
    pub mcp: usize,
    pub skills: usize,
    pub plugins: usize,
    pub codex_enabled: usize,
    pub claude_enabled: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedToolInventory {
    pub assets: Vec<UnifiedToolAsset>,
    pub counts: UnifiedToolInventoryCounts,
    pub scanned_sources: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedToolToggleRequest {
    pub id: String,
    pub kind: String,
    pub app: String,
    pub enabled: bool,
}

pub fn scan_unified_tool_inventory(
    roots: &UnifiedToolInventoryRoots,
) -> anyhow::Result<UnifiedToolInventory> {
    let mut assets = BTreeMap::<String, UnifiedToolAsset>::new();
    let mut scanned_sources = BTreeSet::<String>::new();
    let mut diagnostics = Vec::new();

    scan_codex_config(
        &roots.codex_home.join("config.toml"),
        &mut assets,
        &mut scanned_sources,
        &mut diagnostics,
    );
    scan_claude_mcp_configs(
        &roots.claude_config_paths,
        &mut assets,
        &mut scanned_sources,
        &mut diagnostics,
    );
    scan_disabled_claude_mcp_snapshots(roots, &mut assets, &mut scanned_sources, &mut diagnostics);

    scan_skill_root(
        &roots.codex_home.join("skills"),
        AppTarget::Codex,
        true,
        &mut assets,
        &mut scanned_sources,
        &mut diagnostics,
    );
    scan_skill_root(
        &roots.codex_home.join("skills").join(DISABLED_DIR),
        AppTarget::Codex,
        false,
        &mut assets,
        &mut scanned_sources,
        &mut diagnostics,
    );
    scan_skill_root(
        &roots.claude_home.join("skills"),
        AppTarget::Claude,
        true,
        &mut assets,
        &mut scanned_sources,
        &mut diagnostics,
    );
    scan_skill_root(
        &roots.claude_home.join("skills").join(DISABLED_DIR),
        AppTarget::Claude,
        false,
        &mut assets,
        &mut scanned_sources,
        &mut diagnostics,
    );
    let mut claude_desktop_roots = roots
        .claude_config_paths
        .iter()
        .filter_map(|path| path.parent())
        .filter(|parent| parent.file_name().and_then(|name| name.to_str()) == Some("Claude-3p"))
        .map(Path::to_path_buf)
        .collect::<Vec<_>>();
    dedupe_paths(&mut claude_desktop_roots);
    for root in &claude_desktop_roots {
        scan_skill_root(
            root,
            AppTarget::Claude,
            true,
            &mut assets,
            &mut scanned_sources,
            &mut diagnostics,
        );
    }

    let mut codex_plugin_roots = vec![
        roots.codex_home.join("plugins").join("cache"),
        roots.codex_home.join(".tmp").join("plugins"),
        roots.codex_home.join(".tmp").join("bundled-marketplaces"),
    ];
    codex_plugin_roots.extend(codex_local_marketplace_roots(&roots.codex_home));
    dedupe_paths(&mut codex_plugin_roots);
    for root in codex_plugin_roots {
        scan_plugin_cache(
            &root,
            ".codex-plugin",
            AppTarget::Codex,
            &mut assets,
            &mut scanned_sources,
            &mut diagnostics,
        );
    }
    scan_plugin_cache(
        &roots.claude_home.join("plugins").join("cache"),
        ".claude-plugin",
        AppTarget::Claude,
        &mut assets,
        &mut scanned_sources,
        &mut diagnostics,
    );
    scan_plugin_cache(
        &roots.claude_home.join("plugins").join("marketplaces"),
        ".claude-plugin",
        AppTarget::Claude,
        &mut assets,
        &mut scanned_sources,
        &mut diagnostics,
    );
    for root in &claude_desktop_roots {
        scan_plugin_cache(
            root,
            ".claude-plugin",
            AppTarget::Claude,
            &mut assets,
            &mut scanned_sources,
            &mut diagnostics,
        );
    }
    scan_claude_plugin_state(roots, &mut assets, &mut scanned_sources, &mut diagnostics);

    let raw_discoveries = assets
        .values()
        .map(|asset| asset.discovery_count)
        .sum::<usize>();
    let deduplicated = raw_discoveries.saturating_sub(assets.len());
    let mut assets = assets.into_values().collect::<Vec<_>>();
    for asset in &mut assets {
        if matches!(asset.kind.as_str(), "mcp" | "skill") {
            let any_available = asset.codex.available || asset.claude.available;
            asset.codex.available |= any_available;
            asset.claude.available |= any_available;
            asset.codex.toggle_supported |= any_available;
            asset.claude.toggle_supported |= any_available;
        }
        asset.source = joined_sources(&asset.codex.source_path, &asset.claude.source_path);
    }
    assets.sort_by(|left, right| {
        kind_order(&left.kind)
            .cmp(&kind_order(&right.kind))
            .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });

    let counts = UnifiedToolInventoryCounts {
        total: assets.len(),
        raw_discoveries,
        deduplicated,
        mcp: assets.iter().filter(|asset| asset.kind == "mcp").count(),
        skills: assets.iter().filter(|asset| asset.kind == "skill").count(),
        plugins: assets.iter().filter(|asset| asset.kind == "plugin").count(),
        codex_enabled: assets.iter().filter(|asset| asset.codex.enabled).count(),
        claude_enabled: assets.iter().filter(|asset| asset.claude.enabled).count(),
    };

    Ok(UnifiedToolInventory {
        assets,
        counts,
        scanned_sources: scanned_sources.into_iter().collect(),
        diagnostics,
    })
}

pub fn set_unified_tool_asset_enabled(
    roots: &UnifiedToolInventoryRoots,
    request: &UnifiedToolToggleRequest,
) -> anyhow::Result<UnifiedToolInventory> {
    let current = scan_unified_tool_inventory(roots)?;
    let asset = current
        .assets
        .iter()
        .find(|asset| asset.kind == request.kind && asset.id == request.id)
        .ok_or_else(|| anyhow::anyhow!("未找到工具或插件：{}:{}", request.kind, request.id))?;
    match (request.kind.as_str(), request.app.as_str()) {
        ("mcp", "codex") | ("plugin", "codex") => {
            set_codex_config_asset_enabled(roots, asset, request.enabled)?;
        }
        ("mcp", "claude") => set_claude_mcp_enabled(roots, asset, request.enabled)?,
        ("plugin", "claude") => set_claude_plugin_enabled(roots, asset, request.enabled)?,
        ("skill", "codex") => set_skill_enabled(roots, asset, AppTarget::Codex, request.enabled)?,
        ("skill", "claude") => set_skill_enabled(roots, asset, AppTarget::Claude, request.enabled)?,
        (_, app) if app != "claude" && app != "codex" => {
            anyhow::bail!("未知目标应用：{app}")
        }
        _ => anyhow::bail!("{} 不支持切换到 {}", request.kind, request.app),
    }
    scan_unified_tool_inventory(roots)
}

#[derive(Clone, Copy)]
enum AppTarget {
    Claude,
    Codex,
}

fn scan_codex_config(
    config_path: &Path,
    assets: &mut BTreeMap<String, UnifiedToolAsset>,
    scanned_sources: &mut BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    if !config_path.exists() {
        return;
    }
    scanned_sources.insert(display_path(config_path));
    let config = match std::fs::read_to_string(config_path) {
        Ok(config) => config,
        Err(error) => {
            diagnostics.push(format!(
                "读取 Codex 配置失败（{}）：{error}",
                display_path(config_path)
            ));
            return;
        }
    };
    let entries = match crate::relay_config::list_context_entries_from_common_config(&config) {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(format!(
                "解析 Codex 工具配置失败（{}）：{error}",
                display_path(config_path)
            ));
            return;
        }
    };
    for entry in entries.mcp_servers {
        merge_asset_state(
            assets,
            "mcp",
            &entry.id,
            &entry.title,
            "Codex MCP 配置",
            AppTarget::Codex,
            entry.enabled,
            true,
            true,
            config_path,
        );
        if let Some(asset) =
            assets.get_mut(&format!("mcp:{}", normalized_asset_id("mcp", &entry.id)))
        {
            asset.codex.config_body = entry.toml_body;
            asset.codex.config_id = entry.id;
        }
    }
    for entry in entries.plugins {
        merge_asset_state(
            assets,
            "plugin",
            &entry.id,
            &entry.title,
            "Codex 插件配置",
            AppTarget::Codex,
            entry.enabled,
            true,
            true,
            config_path,
        );
        if let Some(asset) = assets.get_mut(&format!(
            "plugin:{}",
            normalized_asset_id("plugin", &entry.id)
        )) {
            asset.codex.config_body = entry.toml_body;
            asset.codex.config_id = entry.id;
        }
    }
}

fn set_codex_config_asset_enabled(
    roots: &UnifiedToolInventoryRoots,
    asset: &UnifiedToolAsset,
    enabled: bool,
) -> anyhow::Result<()> {
    let path = roots.codex_home.join("config.toml");
    let existing = read_optional_text(&path)?;
    let kind = asset.kind.as_str();
    let current = crate::relay_config::list_context_entries_from_common_config(&existing)?;
    let entry = match kind {
        "mcp" => current
            .mcp_servers
            .iter()
            .find(|entry| entry.id == asset.codex.config_id || entry.id == asset.id),
        "plugin" => current
            .plugins
            .iter()
            .find(|entry| normalized_asset_id("plugin", &entry.id) == asset.id),
        _ => None,
    };
    let (entry_id, body) = if let Some(entry) = entry {
        (
            entry.id.clone(),
            set_toml_enabled(&entry.toml_body, enabled)?,
        )
    } else if enabled && kind == "mcp" && !asset.claude.config_body.trim().is_empty() {
        (
            asset.id.clone(),
            claude_mcp_json_to_codex_toml(&asset.claude.config_body)?,
        )
    } else if enabled && kind == "plugin" && !asset.codex.config_id.trim().is_empty() {
        (
            asset.codex.config_id.clone(),
            "enabled = true\n".to_string(),
        )
    } else {
        anyhow::bail!("Codex 配置中未找到可切换条目：{}", asset.id);
    };
    let updated = crate::relay_config::upsert_context_entry_in_common_config(
        &existing, kind, &entry_id, &body,
    )?;
    backup_file(&path)?;
    crate::settings::atomic_write(&path, updated.as_bytes())
}

fn set_claude_mcp_enabled(
    roots: &UnifiedToolInventoryRoots,
    asset: &UnifiedToolAsset,
    enabled: bool,
) -> anyhow::Result<()> {
    if !enabled {
        return remove_claude_mcp_from_all_configs(roots, &asset.id);
    }
    if !asset.claude.restore_body.trim().is_empty() {
        let snapshot: DisabledClaudeMcpSnapshot = serde_json::from_str(&asset.claude.restore_body)
            .context("盘古受管 Claude MCP 快照无法解析")?;
        return restore_claude_mcp_snapshot(roots, &snapshot);
    }
    let source = path_from_state(&asset.claude).or_else(|_| path_from_state(&asset.codex))?;
    let server = if !asset.claude.config_body.trim().is_empty() {
        serde_json::from_str(&asset.claude.config_body)?
    } else if enabled && !asset.codex.config_body.trim().is_empty() {
        codex_mcp_toml_to_claude_json(&asset.codex.config_body)?
    } else {
        let source_json = read_json(&source)?;
        source_json
            .get("mcpServers")
            .and_then(Value::as_object)
            .and_then(|servers| servers.get(&asset.id))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Claude 配置中未找到 MCP：{}", asset.id))?
    };
    let target = primary_claude_config_path(roots).unwrap_or(source);
    let mut target_json = read_json_object_or_empty(&target)?;
    let root = target_json
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude 配置根节点必须是对象"))?;
    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude mcpServers 必须是对象"))?;
    let mut server = server;
    if let Some(object) = server.as_object_mut() {
        object.insert("enabled".to_string(), Value::Bool(true));
    }
    servers.insert(asset.id.clone(), server);
    write_json_atomic(&target, &target_json)
}

fn remove_claude_mcp_from_all_configs(
    roots: &UnifiedToolInventoryRoots,
    normalized_id: &str,
) -> anyhow::Result<()> {
    let mut updates = Vec::new();
    let mut snapshot_entries = Vec::new();
    let mut seen = BTreeSet::new();
    let mut removed = 0usize;
    for path in &roots.claude_config_paths {
        if !path.exists() || !seen.insert(path.clone()) {
            continue;
        }
        let original = std::fs::read(path)?;
        if original.len() > MAX_CONFIG_BYTES {
            anyhow::bail!(
                "JSON 配置超过 {} MiB 安全上限",
                MAX_CONFIG_BYTES / 1024 / 1024
            );
        }
        let mut json: Value = serde_json::from_slice(&original)?;
        collect_matching_claude_mcp_entries(path, &json, normalized_id, &mut snapshot_entries);
        let removed_here = remove_claude_mcp_from_json(&mut json, normalized_id);
        if removed_here > 0 {
            removed += removed_here;
            updates.push((path.clone(), Some(original), json));
        }
    }
    if removed == 0 {
        anyhow::bail!("Claude 配置中未找到 MCP：{normalized_id}");
    }
    let snapshot_path = disabled_claude_mcp_snapshot_path(roots, normalized_id);
    validate_snapshot_file_location(&snapshot_path)?;
    if std::fs::metadata(&snapshot_path)
        .map(|metadata| metadata.len() as usize > MAX_CONFIG_BYTES)
        .unwrap_or(false)
    {
        anyhow::bail!("Claude MCP 停用快照超过安全上限");
    }
    let original_snapshot = read_optional_bytes(&snapshot_path)?;
    if let Some(bytes) = original_snapshot.as_deref() {
        let existing: DisabledClaudeMcpSnapshot =
            serde_json::from_slice(bytes).with_context(|| {
                format!(
                    "解析受管 Claude MCP 快照失败：{}",
                    display_path(&snapshot_path)
                )
            })?;
        validate_disabled_claude_mcp_snapshot(roots, &existing)?;
        snapshot_entries.extend(existing.entries);
    }
    dedupe_disabled_claude_mcp_entries(&mut snapshot_entries);
    let snapshot = DisabledClaudeMcpSnapshot {
        version: DISABLED_CLAUDE_MCP_SNAPSHOT_VERSION,
        id: normalized_id.to_string(),
        entries: snapshot_entries,
    };
    write_disabled_claude_mcp_snapshot(&snapshot_path, &snapshot)?;

    if let Err(error) =
        apply_json_updates_with_rollback(&updates, write_json_atomic, restore_optional_file)
    {
        let snapshot_restore = restore_optional_file(&snapshot_path, original_snapshot.as_deref());
        return match snapshot_restore {
            Ok(()) => Err(error),
            Err(restore_error) => {
                Err(error).context(format!("恢复受管 Claude MCP 快照失败：{restore_error}"))
            }
        };
    }
    Ok(())
}

fn apply_json_updates_with_rollback(
    updates: &[(PathBuf, Option<Vec<u8>>, Value)],
    mut write_update: impl FnMut(&Path, &Value) -> anyhow::Result<()>,
    mut restore_original: impl FnMut(&Path, Option<&[u8]>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    for (index, (path, _, json)) in updates.iter().enumerate() {
        if let Err(write_error) = write_update(path, json) {
            let mut rollback_errors = Vec::new();
            for (rollback_path, original, _) in updates[..=index].iter().rev() {
                if let Err(error) = restore_original(rollback_path, original.as_deref()) {
                    rollback_errors.push(format!("{}: {error}", display_path(rollback_path)));
                }
            }
            if rollback_errors.is_empty() {
                return Err(write_error).context("Claude MCP 多配置更新失败，已恢复原配置");
            }
            anyhow::bail!(
                "Claude MCP 多配置更新失败：{write_error}；恢复失败：{}",
                rollback_errors.join("；")
            );
        }
    }
    Ok(())
}

fn collect_matching_claude_mcp_entries(
    config_path: &Path,
    json: &Value,
    normalized_id: &str,
    output: &mut Vec<DisabledClaudeMcpEntry>,
) {
    if let Some(servers) = json.get("mcpServers").and_then(Value::as_object) {
        collect_matching_claude_mcp_servers(config_path, None, servers, normalized_id, output);
    }
    if let Some(projects) = json.get("projects").and_then(Value::as_object) {
        for (project_id, project) in projects {
            if let Some(servers) = project.get("mcpServers").and_then(Value::as_object) {
                collect_matching_claude_mcp_servers(
                    config_path,
                    Some(project_id),
                    servers,
                    normalized_id,
                    output,
                );
            }
        }
    }
}

fn collect_matching_claude_mcp_servers(
    config_path: &Path,
    project: Option<&str>,
    servers: &serde_json::Map<String, Value>,
    normalized_id: &str,
    output: &mut Vec<DisabledClaudeMcpEntry>,
) {
    for (server_id, server) in servers {
        if normalized_asset_id("mcp", server_id) == normalized_id {
            output.push(DisabledClaudeMcpEntry {
                config_path: display_path(config_path),
                project: project.map(str::to_string),
                server_id: server_id.clone(),
                server: server.clone(),
            });
        }
    }
}

fn restore_claude_mcp_snapshot(
    roots: &UnifiedToolInventoryRoots,
    snapshot: &DisabledClaudeMcpSnapshot,
) -> anyhow::Result<()> {
    validate_disabled_claude_mcp_snapshot(roots, snapshot)?;
    let mut grouped = BTreeMap::<PathBuf, Vec<&DisabledClaudeMcpEntry>>::new();
    for entry in &snapshot.entries {
        grouped
            .entry(PathBuf::from(&entry.config_path))
            .or_default()
            .push(entry);
    }

    let mut updates = Vec::new();
    for (path, entries) in grouped {
        let original = read_optional_bytes(&path)?;
        if original
            .as_ref()
            .is_some_and(|bytes| bytes.len() > MAX_CONFIG_BYTES)
        {
            anyhow::bail!(
                "JSON 配置超过 {} MiB 安全上限",
                MAX_CONFIG_BYTES / 1024 / 1024
            );
        }
        let mut json = match original.as_deref() {
            Some(bytes) => serde_json::from_slice(bytes)
                .with_context(|| format!("解析 Claude 配置失败：{}", display_path(&path)))?,
            None => serde_json::json!({}),
        };
        for entry in entries {
            insert_claude_mcp_snapshot_entry(&mut json, entry)?;
        }
        updates.push((path, original, json));
    }

    apply_json_updates_with_rollback(&updates, write_json_atomic, restore_optional_file)?;
    let snapshot_path = disabled_claude_mcp_snapshot_path(roots, &snapshot.id);
    if let Err(remove_error) = remove_file_if_exists(&snapshot_path) {
        let rollback_errors = rollback_json_updates(&updates);
        if rollback_errors.is_empty() {
            return Err(remove_error).context("删除受管 Claude MCP 快照失败，已恢复关闭状态");
        }
        anyhow::bail!(
            "删除受管 Claude MCP 快照失败：{remove_error}；恢复关闭状态失败：{}",
            rollback_errors.join("；")
        );
    }
    Ok(())
}

fn insert_claude_mcp_snapshot_entry(
    json: &mut Value,
    entry: &DisabledClaudeMcpEntry,
) -> anyhow::Result<()> {
    let root = json
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude 配置根节点必须是对象"))?;
    let servers = if let Some(project_id) = entry.project.as_deref() {
        let projects = root
            .entry("projects")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("Claude projects 必须是对象"))?;
        let project = projects
            .entry(project_id)
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("Claude project 配置必须是对象"))?;
        project
            .entry("mcpServers")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("Claude project mcpServers 必须是对象"))?
    } else {
        root.entry("mcpServers")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("Claude mcpServers 必须是对象"))?
    };
    if let Some(current) = servers.get(&entry.server_id) {
        if current != &entry.server {
            anyhow::bail!(
                "同名 MCP 配置冲突：{}（{}）",
                entry.server_id,
                entry.project.as_deref().unwrap_or("全局")
            );
        }
        return Ok(());
    }
    servers.insert(entry.server_id.clone(), entry.server.clone());
    Ok(())
}

fn rollback_json_updates(updates: &[(PathBuf, Option<Vec<u8>>, Value)]) -> Vec<String> {
    let mut errors = Vec::new();
    for (path, original, _) in updates.iter().rev() {
        if let Err(error) = restore_optional_file(path, original.as_deref()) {
            errors.push(format!("{}: {error}", display_path(path)));
        }
    }
    errors
}

fn remove_claude_mcp_from_json(json: &mut Value, normalized_id: &str) -> usize {
    let Some(root) = json.as_object_mut() else {
        return 0;
    };
    let mut removed = root
        .get_mut("mcpServers")
        .and_then(Value::as_object_mut)
        .map(|servers| remove_matching_claude_mcp_servers(servers, normalized_id))
        .unwrap_or_default();
    if let Some(projects) = root.get_mut("projects").and_then(Value::as_object_mut) {
        for project in projects.values_mut() {
            if let Some(servers) = project.get_mut("mcpServers").and_then(Value::as_object_mut) {
                removed += remove_matching_claude_mcp_servers(servers, normalized_id);
            }
        }
    }
    removed
}

fn remove_matching_claude_mcp_servers(
    servers: &mut serde_json::Map<String, Value>,
    normalized_id: &str,
) -> usize {
    let before = servers.len();
    servers.retain(|id, _| normalized_asset_id("mcp", id) != normalized_id);
    before.saturating_sub(servers.len())
}

fn set_claude_plugin_enabled(
    roots: &UnifiedToolInventoryRoots,
    asset: &UnifiedToolAsset,
    enabled: bool,
) -> anyhow::Result<()> {
    let path = roots.claude_home.join("settings.json");
    let mut json = read_json_object_or_empty(&path)?;
    let root = json
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude settings.json 根节点必须是对象"))?;
    let plugins = root
        .entry("enabledPlugins")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude enabledPlugins 必须是对象"))?;
    let existing_key = plugins
        .keys()
        .find(|id| normalized_asset_id("plugin", id) == asset.id)
        .cloned();
    let key = existing_key.unwrap_or_else(|| {
        if asset.claude.config_id.trim().is_empty() {
            asset.id.clone()
        } else {
            asset.claude.config_id.clone()
        }
    });
    plugins.insert(key, Value::Bool(enabled));
    write_json_atomic(&path, &json)
}

fn codex_local_marketplace_roots(codex_home: &Path) -> Vec<PathBuf> {
    let config = match std::fs::read_to_string(codex_home.join("config.toml")) {
        Ok(config) => config,
        Err(_) => return Vec::new(),
    };
    let document = match config.parse::<DocumentMut>() {
        Ok(document) => document,
        Err(_) => return Vec::new(),
    };
    let Some(marketplaces) = document
        .get("marketplaces")
        .and_then(|item| item.as_table())
    else {
        return Vec::new();
    };
    marketplaces
        .iter()
        .filter_map(|(_, item)| item.as_table())
        .filter(|table| table.get("source_type").and_then(|item| item.as_str()) == Some("local"))
        .filter_map(|table| table.get("source").and_then(|item| item.as_str()))
        .map(strip_windows_extended_path)
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                codex_home.join(path)
            }
        })
        .collect()
}

fn strip_windows_extended_path(path: &str) -> &str {
    path.strip_prefix(r"\\?\").unwrap_or(path)
}

fn plugin_marketplace_name(scan_root: &Path, manifest_path: &Path) -> Option<String> {
    for marketplace_manifest in [
        scan_root
            .join(".agents")
            .join("plugins")
            .join("marketplace.json"),
        scan_root.join("marketplace.json"),
        scan_root.join("plugins").join("marketplace.json"),
    ] {
        let regular_file = std::fs::symlink_metadata(&marketplace_manifest)
            .map(|metadata| metadata.file_type().is_file() && !metadata.file_type().is_symlink())
            .unwrap_or(false);
        if regular_file {
            if let Ok(marketplace) = read_json(&marketplace_manifest) {
                if let Some(name) = marketplace
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                {
                    return Some(name.to_string());
                }
            }
        }
    }
    let candidate = manifest_path
        .strip_prefix(scan_root)
        .ok()
        .and_then(|relative| relative.components().next())
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .filter(|name| !matches!(name.as_str(), "plugins" | ".agents" | ".codex-plugin"));
    candidate
}

fn set_skill_enabled(
    roots: &UnifiedToolInventoryRoots,
    asset: &UnifiedToolAsset,
    target: AppTarget,
    enabled: bool,
) -> anyhow::Result<()> {
    let (target_root, target_state) = match target {
        AppTarget::Codex => (&roots.codex_home, &asset.codex),
        AppTarget::Claude => (&roots.claude_home, &asset.claude),
    };
    let skill_root = target_root.join("skills");
    let other_state = match target {
        AppTarget::Codex => &asset.claude,
        AppTarget::Claude => &asset.codex,
    };
    let relative = safe_skill_relative_path(&target_state.config_id)
        .or_else(|| safe_skill_relative_path(&other_state.config_id))
        .unwrap_or_else(|| PathBuf::from(&asset.id));
    let active = skill_root.join(&relative);
    let disabled = skill_root.join(DISABLED_DIR).join(&relative);
    if enabled {
        if active.join("SKILL.md").exists() {
            return Ok(());
        }
        if disabled.join("SKILL.md").exists() {
            return move_directory(&disabled, &active);
        }
        let source = path_from_state(target_state).or_else(|_| path_from_state(other_state))?;
        return copy_directory(&source, &active);
    }
    let source = path_from_state(target_state)?;
    if !source.join("SKILL.md").exists() {
        anyhow::bail!("Skill 来源无效：{}", display_path(&source));
    }
    move_directory(&source, &disabled)
}

fn safe_skill_relative_path(value: &str) -> Option<PathBuf> {
    let path = Path::new(value.trim());
    if value.trim().is_empty() || path.is_absolute() {
        return None;
    }
    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(value) => relative.push(value),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    (!relative.as_os_str().is_empty()).then_some(relative)
}

fn scan_claude_mcp_configs(
    paths: &[PathBuf],
    assets: &mut BTreeMap<String, UnifiedToolAsset>,
    scanned_sources: &mut BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    for path in paths {
        if !path.exists() {
            continue;
        }
        scanned_sources.insert(display_path(path));
        let json = match read_json(path) {
            Ok(json) => json,
            Err(error) => {
                diagnostics.push(format!(
                    "解析 Claude 配置失败（{}）：{error}",
                    display_path(path)
                ));
                continue;
            }
        };
        for (id, server) in claude_mcp_servers(&json) {
            let enabled = server
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            merge_asset_state(
                assets,
                "mcp",
                id,
                id,
                "Claude MCP 配置",
                AppTarget::Claude,
                enabled,
                true,
                true,
                path,
            );
            if let Some(asset) = assets.get_mut(&format!("mcp:{}", normalized_asset_id("mcp", id)))
            {
                asset.claude.config_body = serde_json::to_string(server).unwrap_or_default();
                asset.claude.config_id = id.to_string();
            }
        }
    }
}

fn scan_disabled_claude_mcp_snapshots(
    roots: &UnifiedToolInventoryRoots,
    assets: &mut BTreeMap<String, UnifiedToolAsset>,
    scanned_sources: &mut BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    let root = disabled_claude_mcp_snapshot_root(roots);
    let metadata = match std::fs::symlink_metadata(&root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            diagnostics.push(format!(
                "读取 Claude MCP 停用快照目录失败（{}）：{error}",
                display_path(&root)
            ));
            return;
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        diagnostics.push(format!(
            "忽略不安全的 Claude MCP 停用快照目录：{}",
            display_path(&root)
        ));
        return;
    }
    scanned_sources.insert(display_path(&root));
    let entries = match std::fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(format!(
                "扫描 Claude MCP 停用快照失败（{}）：{error}",
                display_path(&root)
            ));
            return;
        }
    };

    for entry in entries.take(MAX_SCAN_ENTRIES) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                diagnostics.push(format!("读取 Claude MCP 停用快照项失败：{error}"));
                continue;
            }
        };
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata)
                if metadata.file_type().is_file() && !metadata.file_type().is_symlink() =>
            {
                metadata
            }
            Ok(_) => {
                diagnostics.push(format!(
                    "忽略不安全的 Claude MCP 停用快照：{}",
                    display_path(&path)
                ));
                continue;
            }
            Err(error) => {
                diagnostics.push(format!(
                    "读取 Claude MCP 停用快照元数据失败（{}）：{error}",
                    display_path(&path)
                ));
                continue;
            }
        };
        if metadata.len() as usize > MAX_CONFIG_BYTES {
            diagnostics.push(format!(
                "忽略超过安全上限的 Claude MCP 停用快照：{}",
                display_path(&path)
            ));
            continue;
        }
        let snapshot = match std::fs::read(&path)
            .with_context(|| format!("读取 {} 失败", display_path(&path)))
            .and_then(|bytes| {
                serde_json::from_slice::<DisabledClaudeMcpSnapshot>(&bytes)
                    .context("快照 JSON 无法解析")
            }) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                diagnostics.push(format!(
                    "忽略损坏的 Claude MCP 停用快照（{}）：{error}",
                    display_path(&path)
                ));
                continue;
            }
        };
        if let Err(error) = validate_disabled_claude_mcp_snapshot(roots, &snapshot) {
            diagnostics.push(format!(
                "忽略无效的 Claude MCP 停用快照（{}）：{error}",
                display_path(&path)
            ));
            continue;
        }
        if path.file_name() != disabled_claude_mcp_snapshot_path(roots, &snapshot.id).file_name() {
            diagnostics.push(format!(
                "忽略名称不匹配的 Claude MCP 停用快照：{}",
                display_path(&path)
            ));
            continue;
        }

        merge_asset_state(
            assets,
            "mcp",
            &snapshot.id,
            &snapshot.id,
            "Claude MCP 受管停用快照",
            AppTarget::Claude,
            false,
            true,
            true,
            &path,
        );
        if let Some(asset) =
            assets.get_mut(&format!("mcp:{}", normalized_asset_id("mcp", &snapshot.id)))
        {
            asset.claude.config_id = snapshot.id.clone();
            asset.claude.restore_body = serde_json::to_string(&snapshot).unwrap_or_default();
        }
    }
}

fn disabled_claude_mcp_snapshot_root(roots: &UnifiedToolInventoryRoots) -> PathBuf {
    roots.claude_home.join(DISABLED_DIR).join("mcp")
}

fn disabled_claude_mcp_snapshot_path(
    roots: &UnifiedToolInventoryRoots,
    normalized_id: &str,
) -> PathBuf {
    let mut safe = String::new();
    let mut previous_dash = false;
    for ch in normalized_id.chars() {
        let next = if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            previous_dash = false;
            Some(ch.to_ascii_lowercase())
        } else if !safe.is_empty() && !previous_dash {
            previous_dash = true;
            Some('-')
        } else {
            None
        };
        if let Some(next) = next {
            safe.push(next);
        }
        if safe.len() >= 48 {
            break;
        }
    }
    let safe = safe.trim_matches('-');
    let safe = if safe.is_empty() { "mcp" } else { safe };
    disabled_claude_mcp_snapshot_root(roots).join(format!(
        "{safe}-{:016x}.json",
        stable_fnv1a64(normalized_id.as_bytes())
    ))
}

fn stable_fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn write_disabled_claude_mcp_snapshot(
    path: &Path,
    snapshot: &DisabledClaudeMcpSnapshot,
) -> anyhow::Result<()> {
    validate_snapshot_file_location(path)?;
    let bytes = serde_json::to_vec_pretty(snapshot)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        anyhow::bail!("Claude MCP 停用快照超过安全上限");
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Claude MCP 停用快照路径无父目录"))?;
    if let Ok(metadata) = std::fs::symlink_metadata(parent) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            anyhow::bail!("Claude MCP 停用快照目录不安全：{}", display_path(parent));
        }
    } else {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("创建 Claude MCP 停用快照目录失败：{}", display_path(parent))
        })?;
    }
    crate::settings::atomic_write(path, &bytes)
}

fn validate_snapshot_file_location(path: &Path) -> anyhow::Result<()> {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            anyhow::bail!("Claude MCP 停用快照文件不安全：{}", display_path(path));
        }
    }
    Ok(())
}

fn validate_disabled_claude_mcp_snapshot(
    roots: &UnifiedToolInventoryRoots,
    snapshot: &DisabledClaudeMcpSnapshot,
) -> anyhow::Result<()> {
    if snapshot.version != DISABLED_CLAUDE_MCP_SNAPSHOT_VERSION {
        anyhow::bail!("不支持的 Claude MCP 停用快照版本：{}", snapshot.version);
    }
    let normalized_id = normalized_asset_id("mcp", &snapshot.id);
    if normalized_id.is_empty() || normalized_id != snapshot.id {
        anyhow::bail!("Claude MCP 停用快照 ID 无效");
    }
    if snapshot.entries.is_empty() || snapshot.entries.len() > MAX_SCAN_ENTRIES {
        anyhow::bail!("Claude MCP 停用快照条目数量无效");
    }
    for entry in &snapshot.entries {
        let path = PathBuf::from(&entry.config_path);
        if !roots
            .claude_config_paths
            .iter()
            .any(|allowed| paths_equivalent(allowed, &path))
        {
            anyhow::bail!("Claude MCP 停用快照包含未授权配置路径");
        }
        if normalized_asset_id("mcp", &entry.server_id) != snapshot.id {
            anyhow::bail!("Claude MCP 停用快照包含不匹配的 server ID");
        }
        if !entry.server.is_object() {
            anyhow::bail!("Claude MCP 停用快照 server 配置必须是对象");
        }
        if entry
            .project
            .as_ref()
            .is_some_and(|project| project.is_empty())
        {
            anyhow::bail!("Claude MCP 停用快照 project 标识不能为空");
        }
    }
    Ok(())
}

fn dedupe_disabled_claude_mcp_entries(entries: &mut Vec<DisabledClaudeMcpEntry>) {
    let mut seen = BTreeSet::new();
    entries.retain(|entry| {
        seen.insert((
            normalized_path_key(Path::new(&entry.config_path)),
            entry.project.clone(),
            entry.server_id.clone(),
        ))
    });
    entries.sort_by(|left, right| {
        normalized_path_key(Path::new(&left.config_path))
            .cmp(&normalized_path_key(Path::new(&right.config_path)))
            .then_with(|| left.project.cmp(&right.project))
            .then_with(|| left.server_id.cmp(&right.server_id))
    });
}

fn paths_equivalent(left: &Path, right: &Path) -> bool {
    normalized_path_key(left) == normalized_path_key(right)
}

fn normalized_path_key(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        value.trim_start_matches("//?/").to_ascii_lowercase()
    } else {
        value.to_string()
    }
}

fn claude_mcp_servers(json: &Value) -> Vec<(&str, &Value)> {
    let mut servers = Vec::new();
    if let Some(root) = json.get("mcpServers").and_then(Value::as_object) {
        servers.extend(root.iter().map(|(id, value)| (id.as_str(), value)));
    }
    if let Some(projects) = json.get("projects").and_then(Value::as_object) {
        for project in projects.values() {
            if let Some(project_servers) = project.get("mcpServers").and_then(Value::as_object) {
                servers.extend(
                    project_servers
                        .iter()
                        .map(|(id, value)| (id.as_str(), value)),
                );
            }
        }
    }
    servers
}

fn scan_skill_root(
    root: &Path,
    target: AppTarget,
    enabled: bool,
    assets: &mut BTreeMap<String, UnifiedToolAsset>,
    scanned_sources: &mut BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    if !root.exists() {
        return;
    }
    scanned_sources.insert(display_path(root));
    let mut visited = 0usize;
    walk_files(root, MAX_SCAN_DEPTH, &mut visited, &mut |path| {
        if path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
            return;
        }
        if enabled
            && path
                .components()
                .any(|part| part.as_os_str() == DISABLED_DIR)
        {
            return;
        }
        let metadata = read_limited_text(path).unwrap_or_default();
        let parent = path.parent().unwrap_or(root);
        let fallback = parent
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("skill");
        let title = frontmatter_value(&metadata, "name").unwrap_or_else(|| fallback.to_string());
        let summary =
            frontmatter_value(&metadata, "description").unwrap_or_else(|| "本地 Skill".to_string());
        merge_asset_state(
            assets, "skill", &title, &title, &summary, target, enabled, true, true, parent,
        );
        if let Ok(relative) = parent.strip_prefix(root) {
            if !relative.as_os_str().is_empty() {
                if let Some(asset) =
                    assets.get_mut(&format!("skill:{}", normalized_asset_id("skill", &title)))
                {
                    let state = match target {
                        AppTarget::Claude => &mut asset.claude,
                        AppTarget::Codex => &mut asset.codex,
                    };
                    if state.config_id.is_empty() || enabled {
                        state.config_id = relative.to_string_lossy().to_string();
                    }
                }
            }
        }
    });
    if visited >= MAX_SCAN_ENTRIES {
        diagnostics.push(format!("Skill 扫描达到数量上限：{}", display_path(root)));
    }
}

fn scan_plugin_cache(
    root: &Path,
    manifest_parent: &str,
    target: AppTarget,
    assets: &mut BTreeMap<String, UnifiedToolAsset>,
    scanned_sources: &mut BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    if !root.exists() {
        return;
    }
    scanned_sources.insert(display_path(root));
    let mut visited = 0usize;
    walk_files(root, MAX_SCAN_DEPTH, &mut visited, &mut |path| {
        if path.file_name().and_then(|name| name.to_str()) != Some("plugin.json")
            || path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                != Some(manifest_parent)
        {
            return;
        }
        let Ok(json) = read_json(path) else {
            return;
        };
        let Some(name) = json.get("name").and_then(Value::as_str) else {
            return;
        };
        let summary = json
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("本地插件缓存");
        let plugin_root = path.parent().and_then(Path::parent).unwrap_or(path);
        merge_asset_state(
            assets,
            "plugin",
            name,
            name,
            summary,
            target,
            false,
            true,
            false,
            plugin_root,
        );
        let config_id =
            plugin_marketplace_name(root, path).map(|marketplace| format!("{name}@{marketplace}"));
        if let Some(asset) =
            assets.get_mut(&format!("plugin:{}", normalized_asset_id("plugin", name)))
        {
            let state = match target {
                AppTarget::Claude => &mut asset.claude,
                AppTarget::Codex => &mut asset.codex,
            };
            if state.config_id.is_empty() {
                if let Some(config_id) = config_id {
                    state.config_id = config_id;
                    state.toggle_supported = true;
                }
            } else {
                state.toggle_supported = true;
            }
        }
    });
    if visited >= MAX_SCAN_ENTRIES {
        diagnostics.push(format!("插件扫描达到数量上限：{}", display_path(root)));
    }
}

fn scan_claude_plugin_state(
    roots: &UnifiedToolInventoryRoots,
    assets: &mut BTreeMap<String, UnifiedToolAsset>,
    scanned_sources: &mut BTreeSet<String>,
    diagnostics: &mut Vec<String>,
) {
    let settings_path = roots.claude_home.join("settings.json");
    if settings_path.exists() {
        scanned_sources.insert(display_path(&settings_path));
        match read_json(&settings_path) {
            Ok(json) => {
                if let Some(plugins) = json.get("enabledPlugins").and_then(Value::as_object) {
                    for (id, enabled) in plugins {
                        merge_asset_state(
                            assets,
                            "plugin",
                            id,
                            id,
                            "Claude 插件配置",
                            AppTarget::Claude,
                            enabled.as_bool().unwrap_or(false),
                            true,
                            true,
                            &settings_path,
                        );
                        if let Some(asset) =
                            assets.get_mut(&format!("plugin:{}", normalized_asset_id("plugin", id)))
                        {
                            asset.claude.config_id = id.to_string();
                        }
                    }
                }
            }
            Err(error) => diagnostics.push(format!(
                "解析 Claude 插件设置失败（{}）：{error}",
                display_path(&settings_path)
            )),
        }
    }

    let installed_path = roots
        .claude_home
        .join("plugins")
        .join("installed_plugins.json");
    if !installed_path.exists() {
        return;
    }
    scanned_sources.insert(display_path(&installed_path));
    match read_json(&installed_path) {
        Ok(json) => {
            if let Some(plugins) = json.get("plugins").and_then(Value::as_object) {
                for (id, _) in plugins {
                    merge_asset_state(
                        assets,
                        "plugin",
                        id,
                        id,
                        "Claude 已安装插件",
                        AppTarget::Claude,
                        false,
                        true,
                        true,
                        &installed_path,
                    );
                    if let Some(asset) =
                        assets.get_mut(&format!("plugin:{}", normalized_asset_id("plugin", id)))
                    {
                        asset.claude.config_id = id.to_string();
                    }
                }
            }
        }
        Err(error) => diagnostics.push(format!(
            "解析 Claude 已安装插件失败（{}）：{error}",
            display_path(&installed_path)
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn merge_asset_state(
    assets: &mut BTreeMap<String, UnifiedToolAsset>,
    kind: &str,
    raw_id: &str,
    title: &str,
    summary: &str,
    target: AppTarget,
    enabled: bool,
    available: bool,
    toggle_supported: bool,
    source_path: &Path,
) {
    let id = normalized_asset_id(kind, raw_id);
    if id.is_empty() {
        return;
    }
    let key = format!("{kind}:{id}");
    let asset = assets.entry(key).or_insert_with(|| UnifiedToolAsset {
        id: id.clone(),
        kind: kind.to_string(),
        title: clean_title(title, raw_id),
        summary: summary.trim().to_string(),
        ..UnifiedToolAsset::default()
    });
    asset.discovery_count = asset.discovery_count.saturating_add(1);
    if asset.summary.is_empty() && !summary.trim().is_empty() {
        asset.summary = summary.trim().to_string();
    }
    let state = match target {
        AppTarget::Claude => &mut asset.claude,
        AppTarget::Codex => &mut asset.codex,
    };
    state.enabled |= enabled;
    state.available |= available;
    state.toggle_supported |= toggle_supported;
    if state.source_path.is_empty() || enabled {
        state.source_path = display_path(source_path);
    }
}

fn normalized_asset_id(kind: &str, raw: &str) -> String {
    let raw = if kind == "plugin" {
        raw.split('@').next().unwrap_or(raw)
    } else {
        raw
    };
    let mut output = String::new();
    let mut previous_dash = false;
    for ch in raw.trim().to_lowercase().chars() {
        if ch.is_alphanumeric() || matches!(ch, '_' | '.' | ':') {
            output.push(ch);
            previous_dash = false;
        } else if !output.is_empty() && !previous_dash {
            output.push('-');
            previous_dash = true;
        }
    }
    output.trim_matches('-').to_string()
}

fn clean_title(title: &str, fallback: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        fallback.trim().to_string()
    } else {
        title.to_string()
    }
}

fn frontmatter_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines().take(80) {
        let line = line.trim();
        let Some(value) = line
            .strip_prefix(key)
            .and_then(|rest| rest.strip_prefix(':'))
        else {
            continue;
        };
        let value = value.trim().trim_matches(['\'', '"']);
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn read_limited_text(path: &Path) -> anyhow::Result<String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_METADATA_BYTES as u64)
        .read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn read_json(path: &Path) -> anyhow::Result<Value> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_CONFIG_BYTES as u64 {
        anyhow::bail!(
            "JSON 配置超过 {} MiB 安全上限",
            MAX_CONFIG_BYTES / 1024 / 1024
        );
    }
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn read_optional_text(path: &Path) -> anyhow::Result<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

fn read_optional_bytes(path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("读取 {} 失败", display_path(path))),
    }
}

fn read_json_object_or_empty(path: &Path) -> anyhow::Result<Value> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let value = read_json(path)?;
    if !value.is_object() {
        anyhow::bail!("JSON 根节点必须是对象：{}", display_path(path));
    }
    Ok(value)
}

fn write_json_atomic(path: &Path, value: &Value) -> anyhow::Result<()> {
    backup_file(path)?;
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    crate::settings::atomic_write(path, &bytes)
}

fn restore_optional_file(path: &Path, original: Option<&[u8]>) -> anyhow::Result<()> {
    match original {
        Some(bytes) => crate::settings::atomic_write(path, bytes),
        None => remove_file_if_exists(path),
    }
}

fn remove_file_if_exists(path: &Path) -> anyhow::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("删除 {} 失败", display_path(path))),
    }
}

fn backup_file(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    let backup = path.with_file_name(format!("{name}.ccp.{stamp}.bak"));
    std::fs::copy(path, backup)?;
    Ok(())
}

fn set_toml_enabled(body: &str, enabled: bool) -> anyhow::Result<String> {
    let mut document = if body.trim().is_empty() {
        DocumentMut::new()
    } else {
        body.parse::<DocumentMut>()?
    };
    document["enabled"] = toml_edit::value(enabled);
    document.as_table_mut().remove("disabled");
    let mut output = document.to_string();
    if !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

fn claude_mcp_json_to_codex_toml(body: &str) -> anyhow::Result<String> {
    let json: Value = serde_json::from_str(body)?;
    let object = json
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Claude MCP 配置必须是对象"))?;
    let mut document = DocumentMut::new();
    for key in ["command", "url", "cwd"] {
        if let Some(value) = object.get(key).and_then(Value::as_str) {
            document[key] = toml_edit::value(value);
        }
    }
    if let Some(args) = object.get("args").and_then(Value::as_array) {
        let mut array = toml_edit::Array::new();
        for value in args.iter().filter_map(Value::as_str) {
            array.push(value);
        }
        document["args"] = toml_edit::value(array);
    }
    if let Some(env) = object.get("env").and_then(Value::as_object) {
        let mut table = toml_edit::Table::new();
        for (key, value) in env {
            if let Some(value) = value.as_str() {
                table[key] = toml_edit::value(value);
            }
        }
        if !table.is_empty() {
            document["env"] = toml_edit::Item::Table(table);
        }
    }
    document["enabled"] = toml_edit::value(true);
    Ok(document.to_string())
}

fn codex_mcp_toml_to_claude_json(body: &str) -> anyhow::Result<Value> {
    let document = body.parse::<DocumentMut>()?;
    let mut json = serde_json::Map::new();
    for key in ["command", "url", "cwd"] {
        if let Some(value) = document.get(key).and_then(|value| value.as_str()) {
            json.insert(key.to_string(), Value::String(value.to_string()));
        }
    }
    if let Some(args) = document.get("args").and_then(|value| value.as_array()) {
        json.insert(
            "args".to_string(),
            Value::Array(
                args.iter()
                    .filter_map(|value| value.as_str())
                    .map(|value| Value::String(value.to_string()))
                    .collect(),
            ),
        );
    }
    if let Some(env) = document.get("env").and_then(|value| value.as_table()) {
        let values = env
            .iter()
            .filter_map(|(key, value)| {
                value
                    .as_str()
                    .map(|value| (key.to_string(), Value::String(value.to_string())))
            })
            .collect::<serde_json::Map<_, _>>();
        if !values.is_empty() {
            json.insert("env".to_string(), Value::Object(values));
        }
    }
    json.insert("enabled".to_string(), Value::Bool(true));
    Ok(Value::Object(json))
}

fn primary_claude_config_path(roots: &UnifiedToolInventoryRoots) -> Option<PathBuf> {
    roots
        .claude_config_paths
        .iter()
        .find(|path| {
            path.file_name().and_then(|name| name.to_str()) == Some(".claude.json")
                || path.to_string_lossy().ends_with(".claude.json")
        })
        .cloned()
        .or_else(|| roots.claude_config_paths.first().cloned())
}

fn path_from_state(state: &UnifiedToolAppState) -> anyhow::Result<PathBuf> {
    if state.source_path.trim().is_empty() {
        anyhow::bail!("该应用没有可审查来源")
    }
    Ok(PathBuf::from(&state.source_path))
}

fn move_directory(source: &Path, destination: &Path) -> anyhow::Result<()> {
    if !source.exists() {
        anyhow::bail!("目录不存在：{}", display_path(source));
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if destination.exists() {
        anyhow::bail!("目标目录已存在：{}", display_path(destination));
    }
    match std::fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_directory(source, destination)?;
            std::fs::remove_dir_all(source)?;
            Ok(())
        }
    }
}

fn copy_directory(source: &Path, destination: &Path) -> anyhow::Result<()> {
    if !source.is_dir() {
        anyhow::bail!("来源不是目录：{}", display_path(source));
    }
    std::fs::create_dir_all(destination)?;
    let mut visited = 0usize;
    copy_directory_inner(source, destination, 0, &mut visited)
}

fn copy_directory_inner(
    source: &Path,
    destination: &Path,
    depth: usize,
    visited: &mut usize,
) -> anyhow::Result<()> {
    if depth > MAX_SCAN_DEPTH || *visited >= MAX_SCAN_ENTRIES {
        anyhow::bail!("Skill 目录超过安全扫描上限")
    }
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        *visited += 1;
        if *visited >= MAX_SCAN_ENTRIES {
            anyhow::bail!("Skill 目录超过安全数量上限")
        }
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_directory_inner(&entry.path(), &target, depth + 1, visited)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn walk_files(
    root: &Path,
    max_depth: usize,
    visited: &mut usize,
    callback: &mut impl FnMut(&Path),
) {
    fn visit(
        path: &Path,
        depth: usize,
        max_depth: usize,
        visited: &mut usize,
        callback: &mut impl FnMut(&Path),
    ) {
        if depth > max_depth || *visited >= MAX_SCAN_ENTRIES {
            return;
        }
        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            if *visited >= MAX_SCAN_ENTRIES {
                return;
            }
            *visited += 1;
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                visit(&path, depth + 1, max_depth, visited, callback);
            } else if file_type.is_file() {
                callback(&path);
            }
        }
    }

    visit(root, 0, max_depth, visited, callback);
}

fn dedupe_paths(paths: &mut Vec<PathBuf>) {
    let mut seen = BTreeSet::new();
    paths.retain(|path| seen.insert(path.to_string_lossy().to_lowercase()));
}

fn claude_home_path(home: &Path) -> PathBuf {
    home.join(".claude")
}

fn joined_sources(codex: &str, claude: &str) -> String {
    match (codex.is_empty(), claude.is_empty()) {
        (false, false) if codex != claude => format!("Codex: {codex}；Claude: {claude}"),
        (false, _) => codex.to_string(),
        (_, false) => claude.to_string(),
        _ => String::new(),
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn kind_order(kind: &str) -> u8 {
    match kind {
        "mcp" => 0,
        "skill" => 1,
        "plugin" => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn json_update_transaction_restores_prior_files_when_a_later_write_fails() {
        let first = PathBuf::from("first.json");
        let second = PathBuf::from("second.json");
        let third = PathBuf::from("third.json");
        let updates = vec![
            (
                first.clone(),
                Some(b"first-original".to_vec()),
                serde_json::json!({"changed": 1}),
            ),
            (second.clone(), None, serde_json::json!({"changed": 2})),
            (
                third.clone(),
                Some(b"third-original".to_vec()),
                serde_json::json!({"changed": 3}),
            ),
        ];
        let files = RefCell::new(BTreeMap::from([
            (first.clone(), b"first-original".to_vec()),
            (third.clone(), b"third-original".to_vec()),
        ]));
        let writes = Cell::new(0usize);

        let result = apply_json_updates_with_rollback(
            &updates,
            |path, value| {
                let attempt = writes.get();
                writes.set(attempt + 1);
                if attempt == 2 {
                    anyhow::bail!("injected third write failure");
                }
                files
                    .borrow_mut()
                    .insert(path.to_path_buf(), serde_json::to_vec(value).unwrap());
                Ok(())
            },
            |path, original| {
                if let Some(original) = original {
                    files
                        .borrow_mut()
                        .insert(path.to_path_buf(), original.to_vec());
                } else {
                    files.borrow_mut().remove(path);
                }
                Ok(())
            },
        );

        assert!(result.is_err());
        assert_eq!(files.borrow().get(&first).unwrap(), b"first-original");
        assert!(!files.borrow().contains_key(&second));
        assert_eq!(files.borrow().get(&third).unwrap(), b"third-original");
    }
}
