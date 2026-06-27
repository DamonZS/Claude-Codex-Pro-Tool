use crate::claude_desktop_provider::ClaudeDesktopProviderRequest;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use url::Url;

pub const OFFICIAL_MARKETPLACE_URL: &str = "https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/.claude-plugin/marketplace.json";
pub const AWESOME_CLAUDE_CODE_CSV_URL: &str = "https://raw.githubusercontent.com/hesreallyhim/awesome-claude-code/main/THE_RESOURCES_TABLE.csv";
pub const GITHUB_MCP_REGISTRY_URL: &str = "https://github.com/mcp";
pub const CODEX_PLUGIN_REPOSITORY_URL: &str = "https://github.com/openai/plugins";
pub const CODEX_PLUGIN_DOCUMENTATION_URL: &str = "https://developers.openai.com/codex/plugins";
pub const PONYTAIL_REPOSITORY_URL: &str = "https://github.com/DietrichGebert/ponytail";
const OFFICIAL_MARKETPLACE_NAME: &str = "claude-plugins-official";
const OFFICIAL_MARKETPLACE_REPOSITORY: &str = "anthropics/claude-plugins-official";
const PONYTAIL_MARKETPLACE: &str = "DietrichGebert/ponytail";
const PONYTAIL_PLUGIN_REF: &str = "ponytail@ponytail";
const PONYTAIL_CODEX_ID: &str = "ponytail:codex-plugin";
const PONYTAIL_CODEX_PLUGIN_ID: &str = "ponytail@ponytail";
const PONYTAIL_CODEX_HOOKS_RELATIVE_PATH: &str = "hooks/claude-codex-hooks.json";
const PONYTAIL_CLAUDE_DESKTOP_ORG_ID: &str = "ponytail:claude-desktop-org-plugin";
const PONYTAIL_ORG_PLUGIN_DIR_NAME: &str = "ponytail";
const PONYTAIL_CLAUDE_DESKTOP_MARKETPLACE_DEEP_LINK: &str = "claude://claude.ai/customize/plugins/new?marketplace=DietrichGebert%2Fponytail&plugin=ponytail";
const CLAUDE_DESKTOP_DEV_PROFILE_ID: &str = "00000000-0000-4000-8000-000000157210";
const CLAUDE_DESKTOP_DEV_PROFILE_NAME: &str = "Claude Codex Pro";
const CLAUDE_DESKTOP_DEFAULT_MODEL_LIST: &str =
    "claude-sonnet-4-6\nclaude-opus-4-8 [1m]\nclaude-haiku-4-5";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHubCatalog {
    pub updated_at: String,
    pub sources: Vec<CatalogSource>,
    pub items: Vec<PluginCatalogItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSource {
    pub id: String,
    pub label: String,
    pub url: String,
    pub status: String,
    pub message: String,
    pub item_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCatalogItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_id: String,
    pub source_label: String,
    pub source_url: String,
    pub category: String,
    pub author: String,
    pub homepage: String,
    pub license: String,
    pub tags: Vec<String>,
    pub install_kind: InstallKind,
    pub install_status: InstallStatus,
    pub install_command: Vec<String>,
    pub config_preview: String,
    pub risk: String,
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallKind {
    ClaudePluginMarketplace,
    ClaudeDesktopMcp,
    ClaudeDesktopOrgPlugin,
    ClaudeCodePlugin,
    CodexPlugin,
    CopilotPlugin,
    ManagedSkillBundle,
    McpServer,
    SkillBundle,
    ResourceLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallStatus {
    NotInstalled,
    Installed,
    NeedsReview,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallPreview {
    pub item: PluginCatalogItem,
    pub can_install: bool,
    pub action: String,
    pub command: Vec<String>,
    pub config_diff: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallOutcome {
    pub item: PluginCatalogItem,
    pub preview: PluginInstallPreview,
    pub installed: bool,
    #[serde(rename = "installMessage")]
    pub message: String,
    pub stdout: String,
    pub stderr: String,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHubInstallRecord {
    pub id: String,
    pub name: String,
    pub install_kind: InstallKind,
    pub installed_at: String,
    pub command: Vec<String>,
    pub source_url: String,
    pub backup_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub managed_paths: Vec<String>,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexHookTrustEntry {
    pub key: String,
    pub event_name: String,
    pub matcher: Option<String>,
    pub command: String,
    pub status_message: Option<String>,
    pub current_hash: String,
    pub trusted: bool,
    pub source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexHookTrustPreview {
    pub config_path: String,
    pub hooks: Vec<CodexHookTrustEntry>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpbPackageOutcome {
    pub mcpb_path: String,
    pub manifest_path: String,
    pub opened: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopOrgPluginStatus {
    pub supported: bool,
    pub org_plugins_dir: String,
    pub config_library_dir: String,
    pub profile_meta_path: String,
    pub ponytail_plugin_dir: String,
    pub ponytail_installed: bool,
    pub writable: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopOrgPluginOutcome {
    pub installed: bool,
    pub org_plugins_dir: String,
    pub plugin_dir: String,
    pub manifest_path: String,
    pub plugin_json_path: String,
    pub copied_skills: Vec<String>,
    pub backup_path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopLocalBundleOutcome {
    pub dev_mode: ClaudeDesktopDevModeOutcome,
    pub codex_mcp: PluginInstallOutcome,
    pub ponytail_mcp: PluginInstallOutcome,
    pub organization_plugin: ClaudeDesktopOrgPluginOutcome,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopMarketplaceStatus {
    pub supported: bool,
    pub marketplace: String,
    pub plugin: String,
    pub deep_link: String,
    pub can_auto_write: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopMarketplaceOutcome {
    pub opened: bool,
    pub deep_link: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopDevModeStatus {
    pub supported: bool,
    pub configured: bool,
    pub normal_config_path: String,
    pub threep_config_path: String,
    pub config_library_dir: String,
    pub profile_meta_path: String,
    pub applied_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopDevModeOutcome {
    pub configured: bool,
    pub normal_config_path: String,
    pub threep_config_path: String,
    pub profile_path: String,
    pub profile_meta_path: String,
    pub backup_paths: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopMcpEntry {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub json_body: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopMcpEntries {
    pub config_path: String,
    pub entries: Vec<ClaudeDesktopMcpEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClaudeDesktopDevModeProfile {
    name: String,
    base_url: String,
    api_key: String,
    model_list: String,
}

pub async fn fetch_catalog() -> PluginHubCatalog {
    let mut sources = Vec::new();
    let mut items = Vec::new();
    let installed = load_installed_records().unwrap_or_default();

    match fetch_official_marketplace_items(&installed).await {
        Ok(mut source_items) => {
            sources.push(ok_source(
                "official",
                "Claude 官方插件市场",
                OFFICIAL_MARKETPLACE_URL,
                source_items.len(),
            ));
            items.append(&mut source_items);
        }
        Err(error) => sources.push(failed_source(
            "official",
            "Claude 官方插件市场",
            OFFICIAL_MARKETPLACE_URL,
            error,
        )),
    }

    let mut desktop_items = builtin_claude_desktop_items(&installed);
    sources.push(ok_source(
        "desktop",
        "Claude Desktop MCP",
        "file://claude_desktop_config.json",
        desktop_items.len(),
    ));
    items.append(&mut desktop_items);

    let mut codex_plugin_items = codex_plugin_repository_items(&installed);
    sources.push(ok_source(
        "codex-plugins",
        "OpenAI Codex Plugins",
        CODEX_PLUGIN_REPOSITORY_URL,
        codex_plugin_items.len(),
    ));
    items.append(&mut codex_plugin_items);

    let mut ponytail_items = ponytail_catalog_items(&installed);
    sources.push(ok_source(
        "ponytail",
        "Ponytail 多工具插件",
        PONYTAIL_REPOSITORY_URL,
        ponytail_items.len(),
    ));
    items.append(&mut ponytail_items);

    match fetch_awesome_items(&installed).await {
        Ok(mut source_items) => {
            sources.push(ok_source(
                "awesome",
                "Awesome Claude Code",
                AWESOME_CLAUDE_CODE_CSV_URL,
                source_items.len(),
            ));
            items.append(&mut source_items);
        }
        Err(error) => sources.push(failed_source(
            "awesome",
            "Awesome Claude Code",
            AWESOME_CLAUDE_CODE_CSV_URL,
            error,
        )),
    }

    let mut mcp_items = github_mcp_registry_items(&installed);
    sources.push(ok_source(
        "github-mcp",
        "GitHub MCP Registry",
        GITHUB_MCP_REGISTRY_URL,
        mcp_items.len(),
    ));
    items.append(&mut mcp_items);

    items.sort_by(|a, b| {
        a.install_status
            .status_rank()
            .cmp(&b.install_status.status_rank())
            .then_with(|| a.source_id.cmp(&b.source_id))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    PluginHubCatalog {
        updated_at: current_unix_timestamp_string(),
        sources,
        items,
    }
}

pub async fn preview_install(id: &str) -> anyhow::Result<PluginInstallPreview> {
    let catalog = fetch_catalog().await;
    let item = catalog
        .items
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| anyhow::anyhow!("未找到插件中心条目：{id}"))?;
    Ok(preview_for_item(item))
}

pub async fn install_item(id: &str) -> anyhow::Result<PluginInstallOutcome> {
    let preview = preview_install(id).await?;
    if !preview.can_install {
        anyhow::bail!("{}", preview.message);
    }

    match preview.item.install_kind {
        InstallKind::ClaudeDesktopMcp => install_claude_desktop_mcp(preview),
        InstallKind::ClaudeDesktopOrgPlugin => install_claude_desktop_org_plugin(preview),
        InstallKind::CodexPlugin => install_codex_plugin(preview),
        InstallKind::ClaudeCodePlugin | InstallKind::CopilotPlugin => install_cli_plugin(preview),
        InstallKind::ManagedSkillBundle => install_managed_skill_bundle(preview),
        InstallKind::McpServer => {
            anyhow::bail!(
                "Community MCP items require confirmed command/args before writing Claude Desktop config"
            )
        }
        InstallKind::ClaudePluginMarketplace => install_official_claude_plugin(preview),
        InstallKind::SkillBundle | InstallKind::ResourceLink => {
            anyhow::bail!("该社区资源需要人工审查后安装")
        }
    }
}

pub async fn install_ponytail_claude_desktop_local_bundle()
-> anyhow::Result<ClaudeDesktopLocalBundleOutcome> {
    let dev_mode = configure_claude_desktop_dev_mode(None)?;
    if !dev_mode.configured {
        anyhow::bail!("{}", dev_mode.message);
    }

    let catalog = fetch_catalog().await;
    let install_local_mcp = |id: &str| -> anyhow::Result<PluginInstallOutcome> {
        let item = catalog
            .items
            .iter()
            .find(|item| item.id == id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("plugin hub item not found: {id}"))?;
        install_claude_desktop_mcp(preview_for_item(item))
    };

    let codex_mcp = install_local_mcp("desktop:claude-codex-pro-codex")?;
    let ponytail_mcp = install_local_mcp("ponytail:claude-desktop-mcp")?;
    let organization_plugin = install_ponytail_claude_desktop_org_plugin()?;

    Ok(ClaudeDesktopLocalBundleOutcome {
        dev_mode,
        codex_mcp,
        ponytail_mcp,
        organization_plugin,
        message: "Claude Desktop development mode, MCP config, Ponytail MCP, and local organization plugin skills were written locally. No Claude CLI login or official plugin marketplace install was used. Fully restart Claude Desktop.".to_string(),
    })
}

pub fn uninstall_item(id: &str) -> anyhow::Result<Vec<PluginHubInstallRecord>> {
    let mut records = load_installed_records().unwrap_or_default();
    if let Some(record) = records.get(id).cloned() {
        uninstall_record_artifacts(&record)?;
        records.remove(id);
    }
    save_installed_records(&records)?;
    Ok(records.into_values().collect())
}

pub fn load_installed_records() -> anyhow::Result<BTreeMap<String, PluginHubInstallRecord>> {
    let path = installed_records_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("读取插件中心安装记录失败：{}", path.display()));
        }
    };
    installed_records_from_text(&text)
        .with_context(|| format!("解析插件中心安装记录失败：{}", path.display()))
}

fn installed_records_from_text(
    text: &str,
) -> anyhow::Result<BTreeMap<String, PluginHubInstallRecord>> {
    let records = serde_json::from_str::<Vec<PluginHubInstallRecord>>(text)?;
    Ok(records
        .into_iter()
        .filter(|record| record.install_kind != InstallKind::CodexPlugin || record.verified)
        .map(|record| (record.id.clone(), record))
        .collect())
}

async fn fetch_official_marketplace_items(
    installed: &BTreeMap<String, PluginHubInstallRecord>,
) -> anyhow::Result<Vec<PluginCatalogItem>> {
    let client = crate::http_client::proxied_client("ClaudeCodexPro/PluginHub")?;
    let raw = client
        .get(OFFICIAL_MARKETPLACE_URL)
        .send()
        .await
        .context("请求官方 Claude 插件市场失败")?
        .error_for_status()
        .context("官方 Claude 插件市场返回错误状态")?
        .json::<Value>()
        .await
        .context("解析官方 Claude 插件市场 JSON 失败")?;
    Ok(parse_official_marketplace(raw, installed))
}

fn parse_official_marketplace(
    raw: Value,
    installed: &BTreeMap<String, PluginHubInstallRecord>,
) -> Vec<PluginCatalogItem> {
    raw.get("plugins")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|plugin| {
            let name = value_string(plugin, "name");
            if name.is_empty() {
                return None;
            }
            let id = format!("official:{name}");
            let plugin_name = name.clone();
            let category = value_string(plugin, "category");
            let homepage = value_string(plugin, "homepage");
            let author = plugin
                .get("author")
                .and_then(|author| author.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            let tags = plugin
                .get("tags")
                .and_then(Value::as_array)
                .map(|items| string_array(items))
                .unwrap_or_default();
            let command = official_marketplace_add_command();
            Some(PluginCatalogItem {
                id: id.clone(),
                name,
                description: value_string(plugin, "description"),
                source_id: "official".to_string(),
                source_label: "Claude 官方插件市场".to_string(),
                source_url: OFFICIAL_MARKETPLACE_URL.to_string(),
                category: if category.is_empty() {
                    "claude-plugin".to_string()
                } else {
                    category
                },
                author,
                homepage,
                license: String::new(),
                tags,
                install_kind: InstallKind::ClaudePluginMarketplace,
                install_status: status_for(&id, installed, InstallStatus::NotInstalled),
                install_command: command,
                config_preview: official_plugin_plan_text(&plugin_name),
                risk: "官方市场条目，安装前仍会显示 CLI 命令。".to_string(),
                requirements: vec!["claude CLI".to_string(), "网络访问".to_string()],
            })
        })
        .collect()
}

async fn fetch_awesome_items(
    installed: &BTreeMap<String, PluginHubInstallRecord>,
) -> anyhow::Result<Vec<PluginCatalogItem>> {
    let client = crate::http_client::proxied_client("ClaudeCodexPro/PluginHub")?;
    let text = client
        .get(AWESOME_CLAUDE_CODE_CSV_URL)
        .send()
        .await
        .context("请求 awesome-claude-code CSV 失败")?
        .error_for_status()
        .context("awesome-claude-code CSV 返回错误状态")?
        .text()
        .await
        .context("读取 awesome-claude-code CSV 失败")?;
    Ok(parse_awesome_csv(&text, installed))
}

fn parse_awesome_csv(
    text: &str,
    installed: &BTreeMap<String, PluginHubInstallRecord>,
) -> Vec<PluginCatalogItem> {
    parse_csv_records(text)
        .into_iter()
        .skip(1)
        .filter_map(|row| {
            let id = row.first().cloned().unwrap_or_default();
            let name = row.get(1).cloned().unwrap_or_default();
            let category = row.get(2).cloned().unwrap_or_default();
            let link = row.get(4).cloned().unwrap_or_default();
            if id.is_empty() || name.is_empty() || link.is_empty() {
                return None;
            }
            let description = row.get(13).cloned().unwrap_or_default();
            let license = row.get(12).cloned().unwrap_or_default();
            let source_id = "awesome".to_string();
            let install_kind = classify_awesome_item(&id, &category, &link, &description);
            let install_status = match install_kind {
                InstallKind::SkillBundle | InstallKind::McpServer => status_for(
                    &format!("awesome:{id}"),
                    installed,
                    InstallStatus::NeedsReview,
                ),
                _ => InstallStatus::Unsupported,
            };
            Some(PluginCatalogItem {
                id: format!("awesome:{id}"),
                name,
                description,
                source_id,
                source_label: "Awesome Claude Code".to_string(),
                source_url: AWESOME_CLAUDE_CODE_CSV_URL.to_string(),
                category,
                author: row.get(6).cloned().unwrap_or_default(),
                homepage: link,
                license,
                tags: vec!["community".to_string()],
                install_kind,
                install_status,
                install_command: Vec::new(),
                config_preview: String::new(),
                risk: "社区资源默认只展示，安装前需要人工审查仓库结构。".to_string(),
                requirements: vec!["人工审查".to_string()],
            })
        })
        .take(240)
        .collect()
}

fn builtin_claude_desktop_items(
    installed: &BTreeMap<String, PluginHubInstallRecord>,
) -> Vec<PluginCatalogItem> {
    let id = "desktop:claude-codex-pro-codex";
    vec![PluginCatalogItem {
        id: id.to_string(),
        name: "Claude Code / Codex MCP".to_string(),
        description: "将 Claude Codex Pro 的 Codex 能力注册到 Claude Desktop 的 MCP 服务器列表。安装后重启 Claude Desktop，在桌面端工具/插件里使用。".to_string(),
        source_id: "desktop".to_string(),
        source_label: "Claude Desktop MCP".to_string(),
        source_url: "file://claude_desktop_config.json".to_string(),
        category: "codex".to_string(),
        author: "Claude Codex Pro".to_string(),
        homepage: "https://github.com/DamonZS/Claude-Codex-Pro-Tool".to_string(),
        license: "MIT".to_string(),
        tags: vec!["codex".to_string(), "mcp".to_string(), "claude-desktop".to_string()],
        install_kind: InstallKind::ClaudeDesktopMcp,
        install_status: status_for(id, installed, InstallStatus::NotInstalled),
        install_command: desktop_mcp_command(),
        config_preview: claude_desktop_mcp_config_preview(&desktop_mcp_server_name(), &desktop_mcp_command()),
        risk: "写入 Claude Desktop 的 MCP 配置文件；安装前会备份原 claude_desktop_config.json，安装后需要重启 Claude Desktop。".to_string(),
        requirements: vec![
            "Claude Desktop".to_string(),
            "本机 MCP sidecar".to_string(),
            "重启 Claude Desktop".to_string(),
        ],
    }]
}

fn github_mcp_registry_items(
    installed: &BTreeMap<String, PluginHubInstallRecord>,
) -> Vec<PluginCatalogItem> {
    let id = "github-mcp:registry";
    vec![PluginCatalogItem {
        id: id.to_string(),
        name: "GitHub MCP Registry".to_string(),
        description: "GitHub 官方 MCP Registry 入口，可继续浏览并选择具体 MCP 服务器。第一版不自动执行未知服务器安装脚本。".to_string(),
        source_id: "github-mcp".to_string(),
        source_label: "GitHub MCP Registry".to_string(),
        source_url: GITHUB_MCP_REGISTRY_URL.to_string(),
        category: "mcp".to_string(),
        author: "GitHub".to_string(),
        homepage: GITHUB_MCP_REGISTRY_URL.to_string(),
        license: String::new(),
        tags: vec!["mcp".to_string(), "registry".to_string()],
        install_kind: InstallKind::ResourceLink,
        install_status: status_for(id, installed, InstallStatus::Unsupported),
        install_command: Vec::new(),
        config_preview: String::new(),
        risk: "注册表入口只做发现，不自动写入本地 MCP 配置。".to_string(),
        requirements: vec!["浏览器".to_string()],
    }]
}

fn codex_plugin_repository_items(
    installed: &BTreeMap<String, PluginHubInstallRecord>,
) -> Vec<PluginCatalogItem> {
    let id = "codex-plugins:openai";
    vec![PluginCatalogItem {
        id: id.to_string(),
        name: "OpenAI Codex Plugins".to_string(),
        description: "OpenAI 维护的 Codex 插件示例仓库，包含可把 skills、MCP servers、hooks、commands、apps 等能力打包为 Codex plugin 的目录结构。".to_string(),
        source_id: "codex-plugins".to_string(),
        source_label: "OpenAI Codex Plugins".to_string(),
        source_url: CODEX_PLUGIN_REPOSITORY_URL.to_string(),
        category: "codex-plugin".to_string(),
        author: "OpenAI".to_string(),
        homepage: CODEX_PLUGIN_REPOSITORY_URL.to_string(),
        license: String::new(),
        tags: vec![
            "codex".to_string(),
            "plugin".to_string(),
            "skills".to_string(),
            "mcp".to_string(),
        ],
        install_kind: InstallKind::ResourceLink,
        install_status: status_for(id, installed, InstallStatus::Unsupported),
        install_command: Vec::new(),
        config_preview: String::new(),
        risk: "Codex 插件仓库作为资源入口展示；安装前需要人工审查 plugin.json、skills、commands、MCP 和 hooks。".to_string(),
        requirements: vec![
            "人工审查".to_string(),
            "Codex CLI / Codex App".to_string(),
            CODEX_PLUGIN_DOCUMENTATION_URL.to_string(),
        ],
    }]
}

fn ponytail_catalog_items(
    installed: &BTreeMap<String, PluginHubInstallRecord>,
) -> Vec<PluginCatalogItem> {
    let source_id = "ponytail";
    let source_label = "Ponytail 多工具插件";
    let base_description = "Ponytail lazy senior dev 模式：优先 YAGNI、标准库、平台原生能力和最小正确实现，同时保留安全、校验和可访问性边界。";
    let base_tags = || {
        vec![
            "ponytail".to_string(),
            "yagni".to_string(),
            "skills".to_string(),
            "hooks".to_string(),
        ]
    };

    vec![
        PluginCatalogItem {
            id: "ponytail:claude-code-plugin".to_string(),
            name: "Ponytail for Claude Code".to_string(),
            description: format!("{base_description} 安装到 Claude Code 插件市场。"),
            source_id: source_id.to_string(),
            source_label: source_label.to_string(),
            source_url: PONYTAIL_REPOSITORY_URL.to_string(),
            category: "claude-code-plugin".to_string(),
            author: "Dietrich Gebert".to_string(),
            homepage: PONYTAIL_REPOSITORY_URL.to_string(),
            license: "MIT".to_string(),
            tags: {
                let mut tags = base_tags();
                tags.push("claude-code".to_string());
                tags
            },
            install_kind: InstallKind::ClaudeCodePlugin,
            install_status: status_for(
                "ponytail:claude-code-plugin",
                installed,
                InstallStatus::NotInstalled,
            ),
            install_command: ponytail_claude_code_install_command(),
            config_preview: ponytail_cli_plan_text("Claude Code"),
            risk: "会调用 Claude Code CLI 添加 Ponytail marketplace 并安装插件；插件包含 lifecycle hooks，安装后请在 Claude Code 中审查并信任 hooks。".to_string(),
            requirements: vec![
                "claude CLI".to_string(),
                "Node.js on PATH".to_string(),
                "网络访问 GitHub".to_string(),
            ],
        },
        PluginCatalogItem {
            id: "ponytail:codex-plugin".to_string(),
            name: "Ponytail for Codex".to_string(),
            description: format!("{base_description} 添加到 Codex 插件 marketplace，之后在 Codex 的 /plugins 中安装并信任 hooks。"),
            source_id: source_id.to_string(),
            source_label: source_label.to_string(),
            source_url: PONYTAIL_REPOSITORY_URL.to_string(),
            category: "codex-plugin".to_string(),
            author: "Dietrich Gebert".to_string(),
            homepage: PONYTAIL_REPOSITORY_URL.to_string(),
            license: "MIT".to_string(),
            tags: {
                let mut tags = base_tags();
                tags.push("codex".to_string());
                tags.push("plugin".to_string());
                tags
            },
            install_kind: InstallKind::CodexPlugin,
            install_status: status_for(
                "ponytail:codex-plugin",
                installed,
                InstallStatus::NotInstalled,
            ),
            install_command: ponytail_codex_install_command(),
            config_preview: "codex plugin marketplace add DietrichGebert/ponytail --json\ncodex plugin list --available --json\ncodex plugin add ponytail@ponytail --json\n\nHooks are reviewed and trusted only through the separate Review hooks / Trust hooks actions.".to_string(),
            risk: "Runs Codex CLI marketplace add/list/add. The install is recorded only after CLI success; third-party hooks are never trusted silently.".to_string(),
            requirements: vec![
                "codex CLI".to_string(),
                "Node.js on PATH".to_string(),
                "Separate hook review".to_string(),
            ],
        },
        PluginCatalogItem {
            id: "ponytail:copilot-plugin".to_string(),
            name: "Ponytail for GitHub Copilot CLI".to_string(),
            description: format!("{base_description} 安装到 GitHub Copilot CLI 插件系统。"),
            source_id: source_id.to_string(),
            source_label: source_label.to_string(),
            source_url: PONYTAIL_REPOSITORY_URL.to_string(),
            category: "copilot-plugin".to_string(),
            author: "Dietrich Gebert".to_string(),
            homepage: PONYTAIL_REPOSITORY_URL.to_string(),
            license: "MIT".to_string(),
            tags: {
                let mut tags = base_tags();
                tags.push("copilot".to_string());
                tags
            },
            install_kind: InstallKind::CopilotPlugin,
            install_status: status_for(
                "ponytail:copilot-plugin",
                installed,
                InstallStatus::NotInstalled,
            ),
            install_command: ponytail_copilot_install_command(),
            config_preview: ponytail_copilot_plan_text(),
            risk: "会调用 GitHub Copilot CLI 添加 marketplace 并安装插件；如 CLI 未登录或未安装，会返回可读错误。".to_string(),
            requirements: vec![
                "copilot CLI".to_string(),
                "Node.js on PATH".to_string(),
                "网络访问 GitHub".to_string(),
            ],
        },
        PluginCatalogItem {
            id: "ponytail:claude-desktop-mcp".to_string(),
            name: "Ponytail MCP for Claude Desktop".to_string(),
            description: format!("{base_description} 将 Ponytail MCP server 写入 Claude Desktop 的 mcpServers 配置。"),
            source_id: source_id.to_string(),
            source_label: source_label.to_string(),
            source_url: PONYTAIL_REPOSITORY_URL.to_string(),
            category: "claude-desktop-mcp".to_string(),
            author: "Dietrich Gebert".to_string(),
            homepage: PONYTAIL_REPOSITORY_URL.to_string(),
            license: "MIT".to_string(),
            tags: {
                let mut tags = base_tags();
                tags.push("mcp".to_string());
                tags.push("claude-desktop".to_string());
                tags
            },
            install_kind: InstallKind::ClaudeDesktopMcp,
            install_status: status_for(
                "ponytail:claude-desktop-mcp",
                installed,
                InstallStatus::NotInstalled,
            ),
            install_command: ponytail_mcp_command_for_preview(),
            config_preview: claude_desktop_mcp_config_preview(
                &ponytail_mcp_server_name(),
                &ponytail_mcp_command_for_preview(),
            ),
            risk: "安装前会克隆/更新 Ponytail 到本地托管缓存，并备份 claude_desktop_config.json 后写入 MCP 配置；需要重启 Claude Desktop。".to_string(),
            requirements: vec![
                "Git".to_string(),
                "Node.js".to_string(),
                "Claude Desktop".to_string(),
                "npm install 会在托管缓存中安装 MCP 依赖".to_string(),
            ],
        },
        PluginCatalogItem {
            id: PONYTAIL_CLAUDE_DESKTOP_ORG_ID.to_string(),
            name: "Ponytail Organization Plugin for Claude Desktop".to_string(),
            description: format!(
                "{base_description} 安装为 Claude Desktop 开发模式可读取的组织插件目录。"
            ),
            source_id: source_id.to_string(),
            source_label: source_label.to_string(),
            source_url: PONYTAIL_REPOSITORY_URL.to_string(),
            category: "claude-desktop-org-plugin".to_string(),
            author: "Dietrich Gebert".to_string(),
            homepage: PONYTAIL_REPOSITORY_URL.to_string(),
            license: "MIT".to_string(),
            tags: {
                let mut tags = base_tags();
                tags.push("claude-desktop".to_string());
                tags.push("organization-plugin".to_string());
                tags
            },
            install_kind: InstallKind::ClaudeDesktopOrgPlugin,
            install_status: status_for(
                PONYTAIL_CLAUDE_DESKTOP_ORG_ID,
                installed,
                InstallStatus::NotInstalled,
            ),
            install_command: Vec::new(),
            config_preview: claude_desktop_org_plugin_preview_text(),
            risk: "会写入 Claude Desktop 组织插件目录；Windows 默认路径在 Program Files 下，普通权限不可写时会失败并提示以管理员运行。只复制 skills 和插件元数据，不静默信任 hooks。".to_string(),
            requirements: vec![
                "Claude Desktop 3P / 开发模式".to_string(),
                "Git".to_string(),
                "可写入组织插件目录".to_string(),
                "本地写入 MCP/skills/组织插件目录，不调用 Claude CLI 登录".to_string(),
                "完全重启 Claude Desktop 后生效".to_string(),
            ],
        },
        PluginCatalogItem {
            id: "ponytail:codex-skills".to_string(),
            name: "Ponytail Skills for Codex".to_string(),
            description: format!("{base_description} 将 Ponytail 的 skills 复制到当前 Codex 技能目录。"),
            source_id: source_id.to_string(),
            source_label: source_label.to_string(),
            source_url: PONYTAIL_REPOSITORY_URL.to_string(),
            category: "codex-skills".to_string(),
            author: "Dietrich Gebert".to_string(),
            homepage: PONYTAIL_REPOSITORY_URL.to_string(),
            license: "MIT".to_string(),
            tags: {
                let mut tags = base_tags();
                tags.push("codex".to_string());
                tags.push("skill-bundle".to_string());
                tags
            },
            install_kind: InstallKind::ManagedSkillBundle,
            install_status: status_for(
                "ponytail:codex-skills",
                installed,
                InstallStatus::NotInstalled,
            ),
            install_command: Vec::new(),
            config_preview: format!(
                "源：{}\\skills\\*\n目标：{}\\skills\\*",
                ponytail_repo_dir().display(),
                codex_home_dir().display()
            ),
            risk: "会复制 Ponytail skills 到 Codex 用户技能目录；若同名技能已存在，会先备份到 plugin-hub/backups 后覆盖。不会自动信任 hooks。".to_string(),
            requirements: vec![
                "Git".to_string(),
                "Codex skills 目录".to_string(),
                "人工选择技能使用".to_string(),
            ],
        },
    ]
}

fn classify_awesome_item(id: &str, category: &str, link: &str, description: &str) -> InstallKind {
    let haystack = format!("{id} {category} {link} {description}").to_lowercase();
    if haystack.contains("mcp") {
        InstallKind::McpServer
    } else if haystack.contains("skill") || category.to_lowercase().contains("agent skills") {
        InstallKind::SkillBundle
    } else {
        InstallKind::ResourceLink
    }
}

fn preview_for_item(item: PluginCatalogItem) -> PluginInstallPreview {
    match item.install_kind {
        InstallKind::ClaudePluginMarketplace => PluginInstallPreview {
            command: official_install_command(&item),
            config_diff: if item.config_preview.trim().is_empty() {
                official_plugin_plan_text(&item.name)
            } else {
                item.config_preview.clone()
            },
            can_install: true,
            action: "claude_plugin_cli".to_string(),
            message: "Install through Claude Code CLI after previewing the command.".to_string(),
            item,
        },
        InstallKind::ClaudeDesktopMcp => PluginInstallPreview {
            command: claude_desktop_command_for_item(&item),
            config_diff: claude_desktop_mcp_config_preview(
                &claude_desktop_server_name_for_item(&item),
                &claude_desktop_command_for_item(&item),
            ),
            can_install: true,
            action: "claude_desktop_mcp_config".to_string(),
            message:
                "将写入 Claude Desktop 的 claude_desktop_config.json；重启 Claude Desktop 后生效。"
                    .to_string(),
            item,
        },
        InstallKind::ClaudeDesktopOrgPlugin => PluginInstallPreview {
            command: Vec::new(),
            config_diff: claude_desktop_org_plugin_preview_text(),
            can_install: true,
            action: "claude_desktop_org_plugin".to_string(),
            message: "Writes the reviewed plugin directory and skills into Claude Desktop development-mode local folders. No Claude CLI login is required; restart Claude Desktop after install.".to_string(),
            item,
        },
        InstallKind::ClaudeCodePlugin | InstallKind::CopilotPlugin => PluginInstallPreview {
            command: item.install_command.clone(),
            config_diff: item.config_preview.clone(),
            can_install: true,
            action: "external_cli_plugin".to_string(),
            message: "Run the previewed CLI install steps. If the target CLI is missing or not logged in, the error output is returned.".to_string(),
            item,
        },
        InstallKind::CodexPlugin => PluginInstallPreview {
            command: item.install_command.clone(),
            config_diff: item.config_preview.clone(),
            can_install: true,
            action: "codex_cli_plugin".to_string(),
            message: "Runs Codex CLI marketplace add, verifies Ponytail is available, then installs ponytail@ponytail. Hooks remain untrusted until you review and trust them separately.".to_string(),
            item,
        },
        InstallKind::ManagedSkillBundle => PluginInstallPreview {
            command: Vec::new(),
            config_diff: item.config_preview.clone(),
            can_install: true,
            action: "managed_skill_bundle".to_string(),
            message: "Clone or update Ponytail, then copy verified SKILL.md directories into Codex skills.".to_string(),
            item,
        },
        InstallKind::McpServer => PluginInstallPreview {
            command: Vec::new(),
            config_diff: claude_desktop_mcp_config_preview(
                &safe_id(&item.name),
                &[
                    "npx".to_string(),
                    "-y".to_string(),
                    "<package-or-command>".to_string(),
                ],
            ),
            can_install: false,
            action: "claude_desktop_mcp_config".to_string(),
            message: if item.source_id == "awesome" {
                "已生成 MCP 配置草案；社区条目需要确认 command/args 后再启用。".to_string()
            } else {
                "该 MCP 条目只支持浏览，不自动写入配置。".to_string()
            },
            item,
        },
        InstallKind::SkillBundle => PluginInstallPreview {
            command: Vec::new(),
            config_diff: String::new(),
            can_install: false,
            action: "manual_review".to_string(),
            message: "Skill bundle 需要确认仓库内 SKILL.md 结构后安装。".to_string(),
            item,
        },
        InstallKind::ResourceLink => PluginInstallPreview {
            command: Vec::new(),
            config_diff: String::new(),
            can_install: false,
            action: "open_link".to_string(),
            message: "该条目作为资源链接展示，不支持自动安装。".to_string(),
            item,
        },
    }
}

fn official_install_failure_message(command: &[String], stdout: &str, stderr: &str) -> String {
    let combined = format!("{}\n{}", stdout, stderr);
    let command_text = command.join(" ");
    let login_hint = [
        "未找到有效的登录配置",
        "请先登录",
        "No valid login configuration",
        "please login",
        "login first",
    ];
    if login_hint.iter().any(|needle| combined.contains(needle)) {
        return format!(
            "需要先登录 Claude Code CLI 后才能安装官方插件。请先运行 `claude` 完成登录，然后重试。\n命令：{command_text}"
        );
    }

    let output = combined.trim();
    if output.is_empty() {
        format!("claude CLI 返回失败。\n命令：{command_text}")
    } else {
        format!("claude CLI 返回失败。\n命令：{command_text}\n输出：{output}")
    }
}

fn official_marketplace_add_command() -> Vec<String> {
    vec![
        "claude".to_string(),
        "plugin".to_string(),
        "marketplace".to_string(),
        "add".to_string(),
        OFFICIAL_MARKETPLACE_REPOSITORY.to_string(),
    ]
}

fn official_plugin_install_command(plugin_name: &str) -> Vec<String> {
    vec![
        "claude".to_string(),
        "plugin".to_string(),
        "install".to_string(),
        format!("{plugin_name}@{OFFICIAL_MARKETPLACE_NAME}"),
    ]
}

fn official_install_command(item: &PluginCatalogItem) -> Vec<String> {
    if !item.install_command.is_empty() {
        return item.install_command.clone();
    }
    official_marketplace_add_command()
}

fn official_plugin_install_plan(item: &PluginCatalogItem) -> Vec<Vec<String>> {
    vec![
        official_install_command(item),
        official_plugin_install_command(&item.name),
    ]
}

fn official_plugin_plan_text(plugin_name: &str) -> String {
    format!(
        "claude plugin marketplace add {OFFICIAL_MARKETPLACE_REPOSITORY}\nclaude plugin install {plugin_name}@{OFFICIAL_MARKETPLACE_NAME}"
    )
}

fn ponytail_claude_code_install_command() -> Vec<String> {
    vec![
        "claude".to_string(),
        "plugin".to_string(),
        "marketplace".to_string(),
        "add".to_string(),
        PONYTAIL_MARKETPLACE.to_string(),
    ]
}

fn ponytail_codex_install_command() -> Vec<String> {
    vec![
        "codex".to_string(),
        "plugin".to_string(),
        "marketplace".to_string(),
        "add".to_string(),
        PONYTAIL_MARKETPLACE.to_string(),
        "--json".to_string(),
    ]
}

fn ponytail_copilot_install_command() -> Vec<String> {
    vec![
        "copilot".to_string(),
        "plugin".to_string(),
        "marketplace".to_string(),
        "add".to_string(),
        PONYTAIL_MARKETPLACE.to_string(),
    ]
}

fn ponytail_cli_plan_text(tool: &str) -> String {
    format!(
        "{tool}:\n/plugin marketplace add {PONYTAIL_MARKETPLACE}\n/plugin install {PONYTAIL_PLUGIN_REF}\n\nThe backend runs marketplace add and plugin install where the CLI supports it. Review Ponytail hooks inside the target tool after install."
    )
}

fn ponytail_copilot_plan_text() -> String {
    format!(
        "copilot plugin marketplace add {PONYTAIL_MARKETPLACE}\ncopilot plugin install {PONYTAIL_PLUGIN_REF}"
    )
}

fn cli_plugin_install_plan(kind: InstallKind) -> anyhow::Result<Vec<Vec<String>>> {
    match kind {
        InstallKind::ClaudeCodePlugin => Ok(vec![
            ponytail_claude_code_install_command(),
            vec![
                "claude".to_string(),
                "plugin".to_string(),
                "install".to_string(),
                PONYTAIL_PLUGIN_REF.to_string(),
            ],
        ]),
        InstallKind::CopilotPlugin => Ok(vec![
            ponytail_copilot_install_command(),
            vec![
                "copilot".to_string(),
                "plugin".to_string(),
                "install".to_string(),
                PONYTAIL_PLUGIN_REF.to_string(),
            ],
        ]),
        InstallKind::CodexPlugin => Ok(vec![
            ponytail_codex_install_command(),
            vec![
                "codex".to_string(),
                "plugin".to_string(),
                "list".to_string(),
                "--available".to_string(),
                "--json".to_string(),
            ],
            vec![
                "codex".to_string(),
                "plugin".to_string(),
                "add".to_string(),
                PONYTAIL_PLUGIN_REF.to_string(),
                "--json".to_string(),
            ],
        ]),
        _ => anyhow::bail!("unsupported CLI plugin install kind"),
    }
}

fn uninstall_record_artifacts(record: &PluginHubInstallRecord) -> anyhow::Result<()> {
    match record.install_kind {
        InstallKind::ClaudeDesktopMcp => {
            let server_name = claude_desktop_server_name_for_record(record);
            for config_path in claude_desktop_normal_config_paths() {
                remove_claude_desktop_mcp_server(&config_path, &server_name)?;
            }
        }
        InstallKind::ClaudeDesktopOrgPlugin => {
            remove_claude_desktop_org_plugin(record)?;
        }
        InstallKind::ManagedSkillBundle => {
            remove_managed_skill_paths(record)?;
        }
        _ => {}
    }
    Ok(())
}

fn remove_managed_skill_paths(record: &PluginHubInstallRecord) -> anyhow::Result<()> {
    let paths = managed_skill_paths_for_record(record)?;
    remove_managed_skill_paths_under(record, paths, codex_home_dir().join("skills"))
}

fn remove_claude_desktop_org_plugin(record: &PluginHubInstallRecord) -> anyhow::Result<()> {
    let allowed_root = claude_desktop_org_plugins_dir();
    let plugin_dir = record
        .managed_paths
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| allowed_root.join(PONYTAIL_ORG_PLUGIN_DIR_NAME));
    let absolute_allowed_root = std::path::absolute(&allowed_root).unwrap_or(allowed_root);
    let absolute_plugin_dir =
        std::path::absolute(&plugin_dir).unwrap_or_else(|_| plugin_dir.clone());
    if !absolute_plugin_dir.starts_with(&absolute_allowed_root) {
        anyhow::bail!(
            "Refusing to remove Claude Desktop organization plugin outside org plugin directory: {}",
            plugin_dir.display()
        );
    }
    remove_path_if_exists(&plugin_dir)?;
    if let Some(backup_root) = record.backup_path.as_ref().map(PathBuf::from) {
        if backup_root.exists() {
            copy_dir_recursive(&backup_root, &plugin_dir)?;
        }
    }
    Ok(())
}

fn remove_managed_skill_paths_under(
    record: &PluginHubInstallRecord,
    paths: Vec<PathBuf>,
    allowed_root: PathBuf,
) -> anyhow::Result<()> {
    let allowed_root = std::path::absolute(&allowed_root).unwrap_or(allowed_root);
    for destination in paths {
        let absolute_destination =
            std::path::absolute(&destination).unwrap_or_else(|_| destination.clone());
        if !absolute_destination.starts_with(&allowed_root) {
            anyhow::bail!(
                "Refusing to remove managed skill outside Codex skills directory: {}",
                destination.display()
            );
        }
        let backup = record.backup_path.as_ref().and_then(|root| {
            destination
                .file_name()
                .map(|name| PathBuf::from(root).join(name))
        });
        remove_path_if_exists(&destination)?;
        if let Some(backup) = backup {
            if backup.exists() {
                copy_dir_recursive(&backup, &destination)?;
            }
        }
    }
    Ok(())
}

fn managed_skill_paths_for_record(record: &PluginHubInstallRecord) -> anyhow::Result<Vec<PathBuf>> {
    if !record.managed_paths.is_empty() {
        return Ok(record.managed_paths.iter().map(PathBuf::from).collect());
    }
    if record.id != "ponytail:codex-skills" {
        return Ok(Vec::new());
    }
    let source_skills = ponytail_repo_dir().join("skills");
    if !source_skills.is_dir() {
        anyhow::bail!(
            "Cannot locate managed Ponytail skills for uninstall: {}",
            source_skills.display()
        );
    }
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(source_skills)? {
        let entry = entry?;
        let source = entry.path();
        if source.is_dir() && source.join("SKILL.md").is_file() {
            paths.push(codex_home_dir().join("skills").join(entry.file_name()));
        }
    }
    Ok(paths)
}

fn remove_path_if_exists(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn run_command(command: &[String]) -> anyhow::Result<(String, String)> {
    let executable = command
        .first()
        .ok_or_else(|| anyhow::anyhow!("plugin install command is empty"))?;
    let output = Command::new(executable)
        .args(command.iter().skip(1))
        .output()
        .with_context(|| format!("cannot run command: {}", command.join(" ")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        let combined = format!("{}\n{}", stdout, stderr);
        anyhow::bail!("command failed: {}\n{}", command.join(" "), combined.trim());
    }
    Ok((stdout, stderr))
}

fn install_claude_desktop_org_plugin(
    preview: PluginInstallPreview,
) -> anyhow::Result<PluginInstallOutcome> {
    let outcome = install_ponytail_claude_desktop_org_plugin()?;
    record_install_with_managed_paths(
        &preview.item,
        Vec::new(),
        outcome.backup_path.clone(),
        vec![outcome.plugin_dir.clone()],
    )?;
    Ok(PluginInstallOutcome {
        item: preview.item.clone(),
        preview,
        installed: outcome.installed,
        message: outcome.message,
        stdout: outcome.copied_skills.join("\n"),
        stderr: String::new(),
        backup_path: outcome.backup_path,
    })
}

fn install_cli_plugin(preview: PluginInstallPreview) -> anyhow::Result<PluginInstallOutcome> {
    let plan = cli_plugin_install_plan(preview.item.install_kind)?;
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut recorded_command = preview.command.clone();
    for command in plan {
        if recorded_command.is_empty() {
            recorded_command = command.clone();
        }
        let (out, err) = run_command(&command).map_err(|error| {
            anyhow::anyhow!(
                "{}",
                cli_plugin_install_failure_message(&preview.item.install_kind, &command, error)
            )
        })?;
        if !out.is_empty() {
            stdout.push_str(&out);
        }
        if !err.is_empty() {
            stderr.push_str(&err);
        }
    }
    record_install(&preview.item, recorded_command, None)?;
    Ok(PluginInstallOutcome {
        item: preview.item.clone(),
        preview,
        installed: true,
        message: "Ponytail install command completed. Review and trust hooks in the target tool when prompted.".to_string(),
        stdout,
        stderr,
        backup_path: None,
    })
}

fn cli_plugin_install_failure_message(
    kind: &InstallKind,
    command: &[String],
    error: anyhow::Error,
) -> String {
    let raw = error.to_string();
    let command_text = command.join(" ");
    let login_hint = [
        "未找到有效的登录配置",
        "请先登录",
        "No valid login configuration",
        "please login",
        "login first",
    ];
    if matches!(kind, InstallKind::ClaudeCodePlugin)
        && login_hint.iter().any(|needle| raw.contains(needle))
    {
        return format!(
            "Claude Code CLI is not logged in. Run `claude` and complete login, then retry.\nCommand: {command_text}\nOutput: {raw}"
        );
    }
    format!("CLI plugin install failed.\nCommand: {command_text}\nOutput: {raw}")
}

fn install_codex_plugin(preview: PluginInstallPreview) -> anyhow::Result<PluginInstallOutcome> {
    let plan = cli_plugin_install_plan(InstallKind::CodexPlugin)?;
    let (marketplace_stdout, marketplace_stderr) = run_command(&plan[0])
        .with_context(|| "Codex CLI marketplace add failed; plugin was not marked installed")?;
    let (list_stdout, list_stderr) = run_command(&plan[1]).with_context(
        || "Codex CLI available plugin list failed; plugin was not marked installed",
    )?;
    ensure_codex_available_list_contains_ponytail(&list_stdout)?;
    let (install_stdout, install_stderr) = run_command(&plan[2])
        .with_context(|| "Codex CLI plugin add failed; plugin was not marked installed")?;
    let installed_path = parse_codex_plugin_add_installed_path(&install_stdout);
    let mut managed_paths = Vec::new();
    if let Some(path) = installed_path {
        managed_paths.push(path);
    }
    record_install_with_managed_paths(&preview.item, plan[2].clone(), None, managed_paths.clone())?;
    let stdout = [marketplace_stdout, list_stdout, install_stdout].concat();
    let stderr = [marketplace_stderr, list_stderr, install_stderr].concat();
    Ok(PluginInstallOutcome {
        item: preview.item.clone(),
        preview,
        installed: true,
        message: "Ponytail installed in Codex via CLI. Review hooks before trusting them; this step did not silently trust third-party hooks.".to_string(),
        stdout,
        stderr,
        backup_path: None,
    })
}

fn ensure_codex_available_list_contains_ponytail(text: &str) -> anyhow::Result<()> {
    let raw: Value = serde_json::from_str(text).context("parse codex plugin list --json output")?;
    let entries = raw
        .get("available")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(
            raw.get("installed")
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        );
    let found = entries.into_iter().any(codex_plugin_entry_is_ponytail);
    if found {
        Ok(())
    } else {
        anyhow::bail!(
            "Codex marketplace was added, but ponytail@ponytail was not found in `codex plugin list --available --json`; install was not recorded."
        )
    }
}

fn codex_plugin_entry_is_ponytail(entry: &Value) -> bool {
    ["pluginId", "name", "marketplaceName", "id"]
        .into_iter()
        .filter_map(|key| entry.get(key).and_then(Value::as_str))
        .any(|value| {
            let value = value.trim().to_ascii_lowercase();
            value == "ponytail@ponytail" || value == "ponytail"
        })
}

fn parse_codex_plugin_add_installed_path(text: &str) -> Option<String> {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|raw| {
            raw.get("installedPath")
                .and_then(Value::as_str)
                .map(str::trim)
                .map(str::to_string)
        })
        .filter(|value| !value.is_empty())
}

pub fn preview_ponytail_codex_hooks() -> anyhow::Result<CodexHookTrustPreview> {
    let hooks = discover_ponytail_codex_hooks()?;
    let config_path = codex_config_path();
    let pending = hooks.iter().filter(|hook| !hook.trusted).count();
    Ok(CodexHookTrustPreview {
        config_path: config_path.to_string_lossy().to_string(),
        hooks,
        message: if pending == 0 {
            "All discovered Ponytail Codex hooks are already trusted.".to_string()
        } else {
            format!(
                "Discovered {pending} untrusted Ponytail Codex hook(s). Review them before trusting."
            )
        },
    })
}

pub fn trust_ponytail_codex_hooks() -> anyhow::Result<CodexHookTrustPreview> {
    let hooks = discover_ponytail_codex_hooks()?;
    let pending = hooks
        .iter()
        .filter(|hook| !hook.trusted)
        .cloned()
        .collect::<Vec<_>>();
    if pending.is_empty() {
        return preview_ponytail_codex_hooks();
    }
    let config_path = codex_config_path();
    upsert_codex_hook_trust_state(&config_path, &pending)?;
    preview_ponytail_codex_hooks()
}

fn discover_ponytail_codex_hooks() -> anyhow::Result<Vec<CodexHookTrustEntry>> {
    let plugin_root = ponytail_codex_plugin_root()?;
    let hooks_path = plugin_root.join(PONYTAIL_CODEX_HOOKS_RELATIVE_PATH);
    if !hooks_path.is_file() {
        anyhow::bail!(
            "Ponytail Codex hooks file not found at {}. Install Ponytail for Codex first.",
            hooks_path.display()
        );
    }
    let text = std::fs::read_to_string(&hooks_path)
        .with_context(|| format!("read {}", hooks_path.display()))?;
    let raw: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", hooks_path.display()))?;
    let states = read_codex_hook_states(&codex_config_path())?;
    let mut entries = Vec::new();
    let hooks_object = raw
        .get("hooks")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Ponytail hooks JSON must contain a hooks object"))?;
    for (event_name, groups) in hooks_object {
        let Some(groups) = groups.as_array() else {
            continue;
        };
        for (group_index, group) in groups.iter().enumerate() {
            let matcher =
                matcher_for_hash(event_name, group.get("matcher").and_then(Value::as_str));
            let Some(hooks) = group.get("hooks").and_then(Value::as_array) else {
                continue;
            };
            for (handler_index, handler) in hooks.iter().enumerate() {
                if handler.get("type").and_then(Value::as_str) != Some("command") {
                    continue;
                }
                let command = effective_hook_command(handler);
                if command.trim().is_empty() {
                    continue;
                }
                let timeout_sec = handler
                    .get("timeout")
                    .and_then(Value::as_u64)
                    .unwrap_or(600)
                    .max(1);
                let status_message = handler
                    .get("statusMessage")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let key = format!(
                    "{}:{}:{}:{}:{}",
                    PONYTAIL_CODEX_PLUGIN_ID,
                    PONYTAIL_CODEX_HOOKS_RELATIVE_PATH,
                    hook_event_key_label(event_name),
                    group_index,
                    handler_index
                );
                let current_hash = command_hook_hash_for_value(
                    hook_event_key_label(event_name),
                    matcher,
                    &effective_hook_command(handler),
                    timeout_sec,
                    status_message.as_deref(),
                )?;
                let trusted = states.get(&key) == Some(&current_hash);
                entries.push(CodexHookTrustEntry {
                    key,
                    event_name: hook_event_key_label(event_name).to_string(),
                    matcher: matcher.map(str::to_string),
                    command,
                    status_message,
                    current_hash,
                    trusted,
                    source_path: hooks_path.to_string_lossy().to_string(),
                });
            }
        }
    }
    if entries.is_empty() {
        anyhow::bail!(
            "No command hooks were discovered in {}",
            hooks_path.display()
        );
    }
    Ok(entries)
}

fn ponytail_codex_plugin_root() -> anyhow::Result<PathBuf> {
    if let Some(record) = load_installed_records()?.get(PONYTAIL_CODEX_ID) {
        if let Some(path) = record.managed_paths.first() {
            let path = PathBuf::from(path);
            if path.join(PONYTAIL_CODEX_HOOKS_RELATIVE_PATH).is_file() {
                return Ok(path);
            }
        }
    }
    let (stdout, _) = run_command(&[
        "codex".to_string(),
        "plugin".to_string(),
        "list".to_string(),
        "--json".to_string(),
    ])
    .context("cannot locate installed Ponytail plugin; `codex plugin list --json` failed")?;
    parse_codex_installed_plugin_path(&stdout).ok_or_else(|| {
        anyhow::anyhow!(
            "Ponytail is not installed in Codex or Codex CLI did not return an installed path. Install Ponytail for Codex first."
        )
    })
}

fn parse_codex_installed_plugin_path(text: &str) -> Option<PathBuf> {
    let raw = serde_json::from_str::<Value>(text).ok()?;
    let installed = raw.get("installed").and_then(Value::as_array)?;
    installed
        .iter()
        .filter(|entry| codex_plugin_entry_is_ponytail(entry))
        .find_map(|entry| entry.get("installedPath").and_then(Value::as_str))
        .map(PathBuf::from)
}

fn codex_config_path() -> PathBuf {
    codex_home_dir().join("config.toml")
}

fn read_codex_hook_states(config_path: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let text = match std::fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(error).with_context(|| format!("read {}", config_path.display())),
    };
    let value = text
        .parse::<toml::Value>()
        .unwrap_or_else(|_| toml::Value::Table(Default::default()));
    let mut states = BTreeMap::new();
    if let Some(table) = value
        .get("hooks")
        .and_then(|hooks| hooks.get("state"))
        .and_then(toml::Value::as_table)
    {
        for (key, state) in table {
            if let Some(hash) = state.get("trusted_hash").and_then(toml::Value::as_str) {
                states.insert(key.clone(), hash.to_string());
            }
        }
    }
    Ok(states)
}

fn upsert_codex_hook_trust_state(
    config_path: &Path,
    hooks: &[CodexHookTrustEntry],
) -> anyhow::Result<()> {
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existing = std::fs::read_to_string(config_path).unwrap_or_default();
    let mut doc = existing
        .parse::<toml_edit::DocumentMut>()
        .unwrap_or_else(|_| toml_edit::DocumentMut::new());
    let root = doc.as_table_mut();
    if !root.contains_key("hooks") || !root["hooks"].is_table() {
        root.insert("hooks", toml_edit::Item::Table(toml_edit::Table::new()));
    }
    let hooks_table = root["hooks"]
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Codex hooks config is not a TOML table"))?;
    if !hooks_table.contains_key("state") || !hooks_table["state"].is_table() {
        hooks_table.insert("state", toml_edit::Item::Table(toml_edit::Table::new()));
    }
    let state_table = hooks_table["state"]
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Codex hooks.state config is not a TOML table"))?;
    for hook in hooks {
        let mut hook_state = toml_edit::Table::new();
        hook_state.insert("trusted_hash", toml_edit::value(hook.current_hash.clone()));
        state_table.insert(&hook.key, toml_edit::Item::Table(hook_state));
    }
    crate::settings::atomic_write(config_path, doc.to_string().as_bytes())
}

fn hook_event_key_label(event_name: &str) -> &'static str {
    match event_name {
        "PreToolUse" | "pre_tool_use" => "pre_tool_use",
        "PermissionRequest" | "permission_request" => "permission_request",
        "PostToolUse" | "post_tool_use" => "post_tool_use",
        "PreCompact" | "pre_compact" => "pre_compact",
        "PostCompact" | "post_compact" => "post_compact",
        "SessionStart" | "session_start" => "session_start",
        "UserPromptSubmit" | "user_prompt_submit" => "user_prompt_submit",
        "SubagentStart" | "subagent_start" => "subagent_start",
        "SubagentStop" | "subagent_stop" => "subagent_stop",
        "Stop" | "stop" => "stop",
        _ => "unknown",
    }
}

fn matcher_for_hash<'a>(event_name: &str, matcher: Option<&'a str>) -> Option<&'a str> {
    match hook_event_key_label(event_name) {
        "user_prompt_submit" | "stop" => None,
        _ => matcher,
    }
}

fn effective_hook_command(handler: &Value) -> String {
    if cfg!(windows) {
        handler
            .get("commandWindows")
            .or_else(|| handler.get("command_windows"))
            .and_then(Value::as_str)
            .or_else(|| handler.get("command").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string()
    } else {
        handler
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }
}

fn command_hook_hash_for_value(
    event_name: &str,
    matcher: Option<&str>,
    command: &str,
    timeout_sec: u64,
    status_message: Option<&str>,
) -> anyhow::Result<String> {
    let mut handler = serde_json::Map::new();
    handler.insert("type".to_string(), json!("command"));
    handler.insert("command".to_string(), json!(command));
    handler.insert("timeout".to_string(), json!(timeout_sec));
    handler.insert("async".to_string(), json!(false));
    if let Some(status_message) = status_message {
        handler.insert("statusMessage".to_string(), json!(status_message));
    }

    let mut group = serde_json::Map::new();
    if let Some(matcher) = matcher {
        group.insert("matcher".to_string(), json!(matcher));
    }
    group.insert(
        "hooks".to_string(),
        Value::Array(vec![Value::Object(handler)]),
    );

    let mut identity = serde_json::Map::new();
    identity.insert("event_name".to_string(), json!(event_name));
    identity.extend(group);
    let toml_value = toml::Value::try_from(Value::Object(identity))?;
    Ok(version_for_toml_value(&toml_value))
}

fn version_for_toml_value(value: &toml::Value) -> String {
    let json = serde_json::to_value(value).unwrap_or(Value::Null);
    let canonical = canonical_json_value(&json);
    let serialized = serde_json::to_vec(&canonical).unwrap_or_default();
    let hash = Sha256::digest(serialized);
    format!("sha256:{hash:x}")
}

fn canonical_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = serde_json::Map::new();
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                if let Some(value) = map.get(&key) {
                    sorted.insert(key, canonical_json_value(value));
                }
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_json_value).collect()),
        other => other.clone(),
    }
}

fn install_official_claude_plugin(
    preview: PluginInstallPreview,
) -> anyhow::Result<PluginInstallOutcome> {
    let plan = official_plugin_install_plan(&preview.item);
    let mut stdout = String::new();
    let mut stderr = String::new();
    for command in &plan {
        let executable = command
            .first()
            .ok_or_else(|| anyhow::anyhow!("Claude plugin install command is empty"))?;
        let args = command.iter().skip(1).collect::<Vec<_>>();
        let output = Command::new(executable)
            .args(args)
            .output()
            .with_context(|| {
                format!(
                    "Claude Code CLI unavailable; cannot run plugin install command: {}",
                    command.join(" ")
                )
            })?;
        let command_stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let command_stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            anyhow::bail!(
                "{}",
                official_install_failure_message(command, &command_stdout, &command_stderr)
            );
        }
        stdout.push_str(&command_stdout);
        stderr.push_str(&command_stderr);
    }
    let record_command = plan
        .last()
        .cloned()
        .unwrap_or_else(|| official_plugin_install_command(&preview.item.name));
    record_install(&preview.item, record_command, None)?;
    Ok(PluginInstallOutcome {
        item: preview.item.clone(),
        preview,
        installed: true,
        message: "Installed official Claude plugin through Claude Code CLI.".to_string(),
        stdout,
        stderr,
        backup_path: None,
    })
}

fn install_claude_desktop_mcp(
    preview: PluginInstallPreview,
) -> anyhow::Result<PluginInstallOutcome> {
    let config_paths = claude_desktop_normal_config_paths();
    let mut backup_paths = Vec::new();
    for config_path in &config_paths {
        if let Some(path) = backup_claude_desktop_config(config_path)? {
            backup_paths.push(path);
        }
    }
    let backup_path = backup_paths.first().cloned();
    let server_name = claude_desktop_server_name_for_item(&preview.item);
    let command = if preview.item.id == "ponytail:claude-desktop-mcp" {
        ensure_ponytail_mcp_ready()?
    } else {
        claude_desktop_command_for_item(&preview.item)
    };
    let server_config = json!({
        "command": command.first().cloned().unwrap_or_default(),
        "args": command.iter().skip(1).cloned().collect::<Vec<_>>(),
        "env": {},
    });
    for config_path in &config_paths {
        upsert_claude_desktop_mcp_server(config_path, &server_name, server_config.clone())?;
    }
    record_install(&preview.item, command, backup_path.clone())?;
    Ok(PluginInstallOutcome {
        item: preview.item.clone(),
        preview,
        installed: true,
        message: "已写入 Claude Desktop MCP 配置；请重启 Claude Desktop 以生效。".to_string(),
        stdout: String::new(),
        stderr: String::new(),
        backup_path,
    })
}

fn claude_desktop_server_name_for_item(item: &PluginCatalogItem) -> String {
    if item.id == "ponytail:claude-desktop-mcp" {
        ponytail_mcp_server_name()
    } else if matches!(item.install_kind, InstallKind::ClaudeDesktopMcp) {
        desktop_mcp_server_name()
    } else {
        safe_id(&item.name)
    }
}

fn claude_desktop_command_for_item(item: &PluginCatalogItem) -> Vec<String> {
    if item.id == "ponytail:claude-desktop-mcp" {
        ponytail_mcp_command_for_preview()
    } else if matches!(item.install_kind, InstallKind::ClaudeDesktopMcp) {
        desktop_mcp_command()
    } else {
        vec![
            "npx".to_string(),
            "-y".to_string(),
            "<package-or-command>".to_string(),
        ]
    }
}

fn desktop_mcp_server_name() -> String {
    "claude-codex-pro-codex".to_string()
}

fn desktop_mcp_command() -> Vec<String> {
    vec!["claude".to_string(), "mcp".to_string(), "serve".to_string()]
}

fn ponytail_mcp_server_name() -> String {
    "ponytail".to_string()
}

fn ponytail_mcp_command_for_preview() -> Vec<String> {
    vec![
        "node".to_string(),
        ponytail_repo_dir()
            .join("ponytail-mcp")
            .join("index.js")
            .to_string_lossy()
            .to_string(),
    ]
}

fn ponytail_repo_dir() -> PathBuf {
    plugin_hub_dir().join("repos").join("ponytail")
}

fn ensure_ponytail_repo() -> anyhow::Result<PathBuf> {
    let repo_dir = ponytail_repo_dir();
    if repo_dir.join(".git").is_dir() {
        run_command(&[
            "git".to_string(),
            "-C".to_string(),
            repo_dir.to_string_lossy().to_string(),
            "pull".to_string(),
            "--ff-only".to_string(),
        ])?;
    } else {
        if repo_dir.exists() {
            anyhow::bail!(
                "Ponytail managed directory exists but is not a Git repository: {}",
                repo_dir.display()
            );
        }
        if let Some(parent) = repo_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }
        run_command(&[
            "git".to_string(),
            "clone".to_string(),
            "--depth".to_string(),
            "1".to_string(),
            PONYTAIL_REPOSITORY_URL.to_string(),
            repo_dir.to_string_lossy().to_string(),
        ])?;
    }
    Ok(repo_dir)
}

fn ensure_ponytail_mcp_ready() -> anyhow::Result<Vec<String>> {
    let repo_dir = ensure_ponytail_repo()?;
    let mcp_dir = repo_dir.join("ponytail-mcp");
    let index = mcp_dir.join("index.js");
    if !index.is_file() {
        anyhow::bail!("Ponytail MCP entry not found: {}", index.display());
    }
    if !mcp_dir.join("node_modules").is_dir() {
        run_command(&[
            "npm".to_string(),
            "install".to_string(),
            "--omit=dev".to_string(),
            "--prefix".to_string(),
            mcp_dir.to_string_lossy().to_string(),
        ])?;
    }
    Ok(vec![
        "node".to_string(),
        index.to_string_lossy().to_string(),
    ])
}

pub fn generate_and_open_ponytail_mcpb() -> anyhow::Result<McpbPackageOutcome> {
    let repo_dir = ensure_ponytail_repo()?;
    let mcp_dir = repo_dir.join("ponytail-mcp");
    let index = mcp_dir.join("index.js");
    if !index.is_file() {
        anyhow::bail!("Ponytail MCP entry not found: {}", index.display());
    }
    if !mcp_dir.join("node_modules").is_dir() {
        run_command(&[
            "npm".to_string(),
            "install".to_string(),
            "--omit=dev".to_string(),
            "--prefix".to_string(),
            mcp_dir.to_string_lossy().to_string(),
        ])?;
    }

    let package_root = plugin_hub_dir()
        .join("mcpb")
        .join(format!("ponytail-{}", current_unix_timestamp_string()));
    let server_dir = package_root.join("server");
    std::fs::create_dir_all(&server_dir)?;
    copy_dir_recursive(&mcp_dir, &server_dir)?;
    write_ponytail_mcpb_manifest(&package_root)?;
    write_ponytail_mcpb_package_json(&package_root)?;
    let mcpb_path = package_root.with_extension("mcpb");
    pack_mcpb_directory(&package_root, &mcpb_path)?;
    open_path_with_system(&mcpb_path)?;
    Ok(McpbPackageOutcome {
        mcpb_path: mcpb_path.to_string_lossy().to_string(),
        manifest_path: package_root
            .join("manifest.json")
            .to_string_lossy()
            .to_string(),
        opened: true,
        message: "Ponytail MCPB package generated and opened. Complete installation in Claude Desktop's official confirmation dialog.".to_string(),
    })
}

pub fn load_claude_desktop_org_plugin_status() -> ClaudeDesktopOrgPluginStatus {
    let org_plugins_dir = claude_desktop_org_plugins_dir();
    let config_library_dir = claude_desktop_threep_config_library_dir();
    let profile_meta_path = config_library_dir.join("_meta.json");
    let ponytail_plugin_dir = org_plugins_dir.join(PONYTAIL_ORG_PLUGIN_DIR_NAME);
    let supported = matches!(
        current_platform(),
        DesktopPlatform::Windows | DesktopPlatform::Macos
    );
    let writable = directory_is_writable(&org_plugins_dir);
    let ponytail_installed = ponytail_plugin_dir
        .join(".claude-plugin")
        .join("plugin.json")
        .is_file()
        && ponytail_plugin_dir.join("manifest.json").is_file()
        && ponytail_plugin_dir.join("skills").is_dir();
    let message = if !supported {
        "Claude Desktop organization plugins are currently supported on Windows and macOS."
            .to_string()
    } else if ponytail_installed {
        "Ponytail organization plugin is installed. Fully restart Claude Desktop to reload organization plugins.".to_string()
    } else if !org_plugins_dir.is_dir() {
        format!(
            "Organization plugin directory does not exist yet: {}. Open directory or install Ponytail to create it if permissions allow.",
            org_plugins_dir.display()
        )
    } else if !writable {
        format!(
            "Organization plugin directory is not writable: {}. Run the manager as administrator or adjust folder permissions.",
            org_plugins_dir.display()
        )
    } else {
        "Organization plugin directory is ready.".to_string()
    };

    ClaudeDesktopOrgPluginStatus {
        supported,
        org_plugins_dir: org_plugins_dir.to_string_lossy().to_string(),
        config_library_dir: config_library_dir.to_string_lossy().to_string(),
        profile_meta_path: profile_meta_path.to_string_lossy().to_string(),
        ponytail_plugin_dir: ponytail_plugin_dir.to_string_lossy().to_string(),
        ponytail_installed,
        writable,
        message,
    }
}

pub fn load_claude_desktop_marketplace_status() -> ClaudeDesktopMarketplaceStatus {
    let supported = matches!(
        current_platform(),
        DesktopPlatform::Windows | DesktopPlatform::Macos
    );
    let message = if supported {
        "Claude Desktop plugin repositories are managed by Claude's official account/organization UI. This opens the Ponytail marketplace add page; Claude Desktop still asks you to confirm.".to_string()
    } else {
        "Claude Desktop marketplace deep links are currently supported on Windows and macOS."
            .to_string()
    };

    ClaudeDesktopMarketplaceStatus {
        supported,
        marketplace: PONYTAIL_MARKETPLACE.to_string(),
        plugin: "ponytail".to_string(),
        deep_link: PONYTAIL_CLAUDE_DESKTOP_MARKETPLACE_DEEP_LINK.to_string(),
        can_auto_write: false,
        message,
    }
}

pub fn open_ponytail_claude_desktop_marketplace_setup()
-> anyhow::Result<ClaudeDesktopMarketplaceOutcome> {
    let status = load_claude_desktop_marketplace_status();
    if !status.supported {
        anyhow::bail!("{}", status.message);
    }
    open_uri_with_system(&status.deep_link)?;
    Ok(ClaudeDesktopMarketplaceOutcome {
        opened: true,
        deep_link: status.deep_link,
        message: "Opened Claude Desktop's official Ponytail plugin repository setup page. Complete marketplace add/install inside Claude Desktop.".to_string(),
    })
}

pub fn load_claude_desktop_dev_mode_status() -> ClaudeDesktopDevModeStatus {
    let normal_config_path = claude_desktop_config_path();
    let normal_config_paths = claude_desktop_normal_config_paths();
    let threep_config_path = claude_desktop_threep_config_path();
    let config_library_dir = claude_desktop_threep_config_library_dir();
    let profile_meta_path = config_library_dir.join("_meta.json");
    let profile_path = claude_desktop_dev_mode_profile_path(&config_library_dir);
    let supported = matches!(
        current_platform(),
        DesktopPlatform::Windows | DesktopPlatform::Macos
    );
    let applied_id = read_claude_desktop_meta_applied_id(&profile_meta_path);
    let configured = claude_desktop_dev_mode_is_configured(
        &normal_config_paths,
        &threep_config_path,
        &profile_path,
        &profile_meta_path,
    );
    let message = if !supported {
        "Claude Desktop development mode config is currently supported on Windows and macOS."
            .to_string()
    } else if configured {
        "Claude Desktop development mode is configured. Restart Claude Desktop to reload plugin and config state.".to_string()
    } else {
        "Claude Desktop development mode is not fully configured. The one-click action writes deploymentMode=3p and initializes Claude-3p/configLibrary metadata.".to_string()
    };

    ClaudeDesktopDevModeStatus {
        supported,
        configured,
        normal_config_path: normal_config_path.to_string_lossy().to_string(),
        threep_config_path: threep_config_path.to_string_lossy().to_string(),
        config_library_dir: config_library_dir.to_string_lossy().to_string(),
        profile_meta_path: profile_meta_path.to_string_lossy().to_string(),
        applied_id,
        message,
    }
}

pub fn list_claude_desktop_mcp_entries() -> anyhow::Result<ClaudeDesktopMcpEntries> {
    let config_path = claude_desktop_config_path();
    let config = if config_path.exists() {
        serde_json::from_str::<Value>(&std::fs::read_to_string(&config_path)?)
            .unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    let mut entries = Vec::new();
    if let Some(servers) = config.get("mcpServers").and_then(Value::as_object) {
        for (id, server) in servers {
            let enabled = server
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let command = server
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let args = server
                .get("args")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            let summary = if command.is_empty() {
                "未配置 command".to_string()
            } else if args.is_empty() {
                command.clone()
            } else {
                format!("{command} {args}")
            };
            let json_body = serde_json::to_string_pretty(server).unwrap_or_else(|_| "{}".to_string());
            entries.push(ClaudeDesktopMcpEntry {
                id: id.to_string(),
                title: id.to_string(),
                summary,
                json_body,
                enabled,
            });
        }
    }
    Ok(ClaudeDesktopMcpEntries {
        config_path: config_path.to_string_lossy().to_string(),
        entries,
    })
}

pub fn upsert_claude_desktop_mcp_entry(
    id: &str,
    json_body: &str,
) -> anyhow::Result<ClaudeDesktopMcpEntries> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        anyhow::bail!("MCP ID cannot be empty");
    }
    let server_config: Value = serde_json::from_str(json_body)
        .with_context(|| format!("parse Claude Desktop MCP JSON for {trimmed}"))?;
    if !server_config.is_object() {
        anyhow::bail!("Claude Desktop MCP config must be a JSON object");
    }
    let config_path = claude_desktop_config_path();
    let _ = backup_claude_desktop_config(&config_path)?;
    upsert_claude_desktop_mcp_server(&config_path, trimmed, server_config)?;
    list_claude_desktop_mcp_entries()
}

pub fn delete_claude_desktop_mcp_entry(id: &str) -> anyhow::Result<ClaudeDesktopMcpEntries> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        anyhow::bail!("MCP ID cannot be empty");
    }
    let config_path = claude_desktop_config_path();
    let _ = backup_claude_desktop_config(&config_path)?;
    remove_claude_desktop_mcp_server(&config_path, trimmed)?;
    list_claude_desktop_mcp_entries()
}

pub fn configure_claude_desktop_dev_mode(
    provider_request: Option<&ClaudeDesktopProviderRequest>,
) -> anyhow::Result<ClaudeDesktopDevModeOutcome> {
    let status = load_claude_desktop_dev_mode_status();
    if !status.supported {
        anyhow::bail!("{}", status.message);
    }
    let provider = resolve_claude_desktop_dev_mode_profile(provider_request)?;

    let normal_config_path = PathBuf::from(&status.normal_config_path);
    let normal_config_paths = claude_desktop_normal_config_paths();
    let threep_config_path = PathBuf::from(&status.threep_config_path);
    let profile_meta_path = PathBuf::from(&status.profile_meta_path);
    let profile_path = claude_desktop_dev_mode_profile_path(
        profile_meta_path.parent().unwrap_or_else(|| Path::new(".")),
    );
    let mut backup_paths = Vec::new();
    for path in &normal_config_paths {
        if let Some(path) = backup_claude_desktop_config(path)? {
            backup_paths.push(path);
        }
    }
    if normal_config_path != threep_config_path {
        if let Some(path) = backup_claude_desktop_config(&threep_config_path)? {
            backup_paths.push(path);
        }
    }
    if let Some(path) = backup_claude_desktop_config(&profile_meta_path)? {
        backup_paths.push(path);
    }
    if let Some(path) = backup_claude_desktop_config(&profile_path)? {
        backup_paths.push(path);
    }

    for path in &normal_config_paths {
        write_claude_desktop_deployment_mode(path, "3p")?;
    }
    write_claude_desktop_deployment_mode(&threep_config_path, "3p")?;
    if let Some(provider) = provider.as_ref() {
        write_claude_desktop_dev_mode_profile(&profile_path, provider)?;
    } else if profile_path.exists() {
        std::fs::remove_file(&profile_path).with_context(|| {
            format!(
                "remove stale Claude Desktop gateway profile {}",
                profile_path.display()
            )
        })?;
    }
    write_claude_desktop_dev_mode_meta(&profile_meta_path)?;

    let next = load_claude_desktop_dev_mode_status();
    Ok(ClaudeDesktopDevModeOutcome {
        configured: next.configured,
        normal_config_path: next.normal_config_path,
        threep_config_path: next.threep_config_path,
        profile_path: profile_path.to_string_lossy().to_string(),
        profile_meta_path: next.profile_meta_path,
        backup_paths,
        message: match provider {
            Some(provider) => format!(
                "Claude Desktop 开发模式已写入 {} gateway profile。请完全退出并重启 Claude Desktop。",
                provider.name
            ),
            None if next.configured => "Claude Desktop 开发模式外壳已开启。当前还没有写入供应商 URL 和 Key，后续补全供应商后即可继续写入完整 gateway profile。".to_string(),
            None => "Claude Desktop 开发模式文件已写入，但状态校验还没有看到预期的 3P 元数据。".to_string(),
        },
    })
}

pub fn open_claude_desktop_org_plugins_dir() -> anyhow::Result<ClaudeDesktopOrgPluginStatus> {
    let status = load_claude_desktop_org_plugin_status();
    let path = PathBuf::from(&status.org_plugins_dir);
    std::fs::create_dir_all(&path).with_context(|| {
        format!(
            "create Claude Desktop organization plugin dir {}",
            path.display()
        )
    })?;
    open_path_with_system(&path)?;
    Ok(load_claude_desktop_org_plugin_status())
}

pub fn install_ponytail_claude_desktop_org_plugin() -> anyhow::Result<ClaudeDesktopOrgPluginOutcome>
{
    let dev_status = load_claude_desktop_dev_mode_status();
    if !dev_status.configured {
        anyhow::bail!(
            "Claude Desktop development mode is not configured. Run one-click development mode first, then install the organization plugin."
        );
    }

    let org_plugins_dir = claude_desktop_org_plugins_dir();
    std::fs::create_dir_all(&org_plugins_dir).with_context(|| {
        format!(
            "create Claude Desktop organization plugin dir {}",
            org_plugins_dir.display()
        )
    })?;
    if !directory_is_writable(&org_plugins_dir) {
        anyhow::bail!(
            "Claude Desktop organization plugin directory is not writable: {}. Run the manager as administrator or adjust folder permissions.",
            org_plugins_dir.display()
        );
    }

    let repo_dir = ensure_ponytail_repo()?;
    let source_skills = repo_dir.join("skills");
    if !source_skills.is_dir() {
        anyhow::bail!(
            "Ponytail skills directory not found: {}",
            source_skills.display()
        );
    }

    let plugin_dir = org_plugins_dir.join(PONYTAIL_ORG_PLUGIN_DIR_NAME);
    let backup_path = if plugin_dir.exists() {
        let backup = plugin_hub_backup_dir()
            .join("claude-desktop-org-plugin")
            .join(PONYTAIL_ORG_PLUGIN_DIR_NAME);
        if let Some(parent) = backup.parent() {
            std::fs::create_dir_all(parent)?;
        }
        copy_dir_recursive(&plugin_dir, &backup)?;
        std::fs::remove_dir_all(&plugin_dir)?;
        Some(backup.to_string_lossy().to_string())
    } else {
        None
    };

    let skills_dir = plugin_dir.join("skills");
    std::fs::create_dir_all(plugin_dir.join(".claude-plugin"))?;
    std::fs::create_dir_all(&skills_dir)?;
    let copied_skills = copy_skill_dirs(&source_skills, &skills_dir)?;
    if copied_skills.is_empty() {
        anyhow::bail!(
            "No Ponytail skill directories with SKILL.md were found in {}",
            source_skills.display()
        );
    }
    let plugin_json_path = plugin_dir.join(".claude-plugin").join("plugin.json");
    let manifest_path = plugin_dir.join("manifest.json");
    write_ponytail_org_plugin_json(&plugin_json_path)?;
    write_ponytail_org_plugin_manifest(&manifest_path, &copied_skills)?;

    Ok(ClaudeDesktopOrgPluginOutcome {
        installed: true,
        org_plugins_dir: org_plugins_dir.to_string_lossy().to_string(),
        plugin_dir: plugin_dir.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        plugin_json_path: plugin_json_path.to_string_lossy().to_string(),
        copied_skills,
        backup_path,
        message: "Ponytail organization plugin and skills were written locally for Claude Desktop development mode. No Claude CLI login was used. Fully restart Claude Desktop, then check Plugins & skills.".to_string(),
    })
}

fn write_ponytail_mcpb_manifest(package_root: &Path) -> anyhow::Result<()> {
    let manifest = json!({
        "manifest_version": "0.3",
        "name": "ponytail",
        "display_name": "Ponytail MCP",
        "version": "0.1.0",
        "description": "Ponytail lazy senior developer instructions as a Claude Desktop MCP extension.",
        "long_description": "Provides Ponytail's YAGNI, standard-library-first, smallest-correct-implementation guidance through an MCP server.",
        "author": {
            "name": "Dietrich Gebert",
            "url": "https://github.com/DietrichGebert"
        },
        "repository": {
            "type": "git",
            "url": PONYTAIL_REPOSITORY_URL
        },
        "homepage": PONYTAIL_REPOSITORY_URL,
        "server": {
            "type": "node",
            "entry_point": "server/index.js",
            "mcp_config": {
                "command": "node",
                "args": ["${__dirname}/server/index.js"]
            }
        },
        "tools": [
            {
                "name": "ponytail",
                "description": "Return Ponytail lazy senior developer guidance."
            }
        ],
        "keywords": ["ponytail", "yagni", "mcp", "claude-desktop"],
        "license": "MIT",
        "compatibility": {
            "claude_desktop": ">=0.10.0",
            "platforms": ["darwin", "win32", "linux"],
            "runtimes": {
                "node": ">=18.0.0"
            }
        },
        "privacy_policies": []
    });
    crate::settings::atomic_write(
        &package_root.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?.as_bytes(),
    )
}

fn write_ponytail_mcpb_package_json(package_root: &Path) -> anyhow::Result<()> {
    let package = json!({
        "type": "module",
        "private": true
    });
    crate::settings::atomic_write(
        &package_root.join("package.json"),
        serde_json::to_string_pretty(&package)?.as_bytes(),
    )
}

fn pack_mcpb_directory(source_dir: &Path, output_path: &Path) -> anyhow::Result<()> {
    if output_path.exists() {
        std::fs::remove_file(output_path)?;
    }
    #[cfg(windows)]
    {
        let zip_path = output_path.with_extension("zip");
        if zip_path.exists() {
            std::fs::remove_file(&zip_path)?;
        }
        let command = vec![
            "powershell".to_string(),
            "-NoProfile".to_string(),
            "-Command".to_string(),
            format!(
                "Set-Location -LiteralPath '{}'; Compress-Archive -Path * -DestinationPath '{}' -Force",
                source_dir.display(),
                zip_path.display()
            ),
        ];
        run_command(&command)?;
        std::fs::rename(zip_path, output_path)?;
    }
    #[cfg(not(windows))]
    {
        let command = vec![
            "zip".to_string(),
            "-r".to_string(),
            output_path.to_string_lossy().to_string(),
            ".".to_string(),
        ];
        let executable = command.first().cloned().unwrap_or_default();
        let output = Command::new(executable)
            .args(command.iter().skip(1))
            .current_dir(source_dir)
            .output()
            .with_context(|| format!("cannot run command: {}", command.join(" ")))?;
        if !output.status.success() {
            anyhow::bail!(
                "command failed: {}\n{}{}",
                command.join(" "),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    Ok(())
}

fn open_path_with_system(path: &Path) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .with_context(|| format!("open {}", path.display()))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .with_context(|| format!("open {}", path.display()))?;
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .with_context(|| format!("open {}", path.display()))?;
    }
    Ok(())
}

fn open_uri_with_system(target: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        crate::windows_open_url(target).with_context(|| format!("open {target}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(target)
            .spawn()
            .with_context(|| format!("open {target}"))?;
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .with_context(|| format!("open {target}"))?;
    }
    Ok(())
}

fn codex_home_dir() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|dirs| dirs.home_dir().join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}

fn plugin_hub_backup_dir() -> PathBuf {
    plugin_hub_dir()
        .join("backups")
        .join(current_unix_timestamp_string())
}

fn install_managed_skill_bundle(
    preview: PluginInstallPreview,
) -> anyhow::Result<PluginInstallOutcome> {
    let repo_dir = ensure_ponytail_repo()?;
    let source_skills = repo_dir.join("skills");
    if !source_skills.is_dir() {
        anyhow::bail!(
            "Ponytail skills directory not found: {}",
            source_skills.display()
        );
    }
    let codex_skills = codex_home_dir().join("skills");
    std::fs::create_dir_all(&codex_skills)?;
    let backup_root = plugin_hub_backup_dir().join("codex-skills");
    let mut copied = Vec::new();
    let mut backed_up = Vec::new();
    for entry in std::fs::read_dir(&source_skills)? {
        let entry = entry?;
        let source = entry.path();
        if !source.is_dir() || !source.join("SKILL.md").is_file() {
            continue;
        }
        let name = entry.file_name();
        let destination = codex_skills.join(&name);
        if destination.exists() {
            std::fs::create_dir_all(&backup_root)?;
            let backup = backup_root.join(&name);
            copy_dir_recursive(&destination, &backup)?;
            backed_up.push(backup.to_string_lossy().to_string());
            if destination.is_dir() {
                std::fs::remove_dir_all(&destination)?;
            } else {
                std::fs::remove_file(&destination)?;
            }
        }
        copy_dir_recursive(&source, &destination)?;
        copied.push(destination.to_string_lossy().to_string());
    }
    if copied.is_empty() {
        anyhow::bail!("No Ponytail skill directories with SKILL.md were found");
    }
    let backup_path = if backed_up.is_empty() {
        None
    } else {
        Some(backup_root.to_string_lossy().to_string())
    };
    record_install_with_managed_paths(
        &preview.item,
        Vec::new(),
        backup_path.clone(),
        copied.clone(),
    )?;
    Ok(PluginInstallOutcome {
        item: preview.item.clone(),
        preview,
        installed: true,
        message: format!(
            "Installed {} Ponytail skills into Codex skills.",
            copied.len()
        ),
        stdout: copied.join("\n"),
        stderr: String::new(),
        backup_path,
    })
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "copy {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn copy_skill_dirs(source_skills: &Path, destination_skills: &Path) -> anyhow::Result<Vec<String>> {
    let mut copied = Vec::new();
    for entry in std::fs::read_dir(source_skills)? {
        let entry = entry?;
        let source = entry.path();
        if !source.is_dir() || !source.join("SKILL.md").is_file() {
            continue;
        }
        let destination = destination_skills.join(entry.file_name());
        copy_dir_recursive(&source, &destination)?;
        copied.push(destination.to_string_lossy().to_string());
    }
    copied.sort();
    Ok(copied)
}

fn write_ponytail_org_plugin_json(path: &Path) -> anyhow::Result<()> {
    let plugin = json!({
        "name": "ponytail",
        "version": "1.0.0",
        "description": "Ponytail skills for Claude Desktop organization plugins"
    });
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::settings::atomic_write(path, serde_json::to_string_pretty(&plugin)?.as_bytes())
}

fn write_ponytail_org_plugin_manifest(path: &Path, copied_skills: &[String]) -> anyhow::Result<()> {
    let skill_entries = copied_skills
        .iter()
        .filter_map(|skill_path| Path::new(skill_path).file_name())
        .map(|name| {
            let name = name.to_string_lossy();
            json!({
                "skillId": name,
                "name": name,
                "description": "Ponytail lazy senior developer guidance.",
                "creatorType": "organization",
                "updatedAt": null,
                "enabled": true
            })
        })
        .collect::<Vec<_>>();
    let manifest = json!({
        "lastUpdated": current_unix_timestamp_string().parse::<u64>().unwrap_or(0) * 1000,
        "skills": skill_entries
    });
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::settings::atomic_write(path, serde_json::to_string_pretty(&manifest)?.as_bytes())
}

fn claude_desktop_org_plugin_preview_text() -> String {
    format!(
        "Source: {}\\skills\\*\nTarget: {}\\{}\\skills\\*\nWrites: .claude-plugin\\plugin.json and manifest.json\nRestart Claude Desktop after install.",
        ponytail_repo_dir().display(),
        claude_desktop_org_plugins_dir().display(),
        PONYTAIL_ORG_PLUGIN_DIR_NAME
    )
}

fn directory_is_writable(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let probe = path.join(format!(
        ".claude-codex-pro-write-test-{}",
        current_unix_timestamp_string()
    ));
    match std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

fn claude_desktop_config_path() -> PathBuf {
    claude_desktop_config_path_for_platform(
        current_platform(),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()),
    )
}

fn claude_desktop_normal_config_paths() -> Vec<PathBuf> {
    let primary = claude_desktop_config_path();
    let mut paths = vec![primary];

    if cfg!(windows) {
        if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
            push_unique_path(
                &mut paths,
                local_appdata
                    .join("Claude")
                    .join("claude_desktop_config.json"),
            );
            if let Ok(packages) = std::fs::read_dir(local_appdata.join("Packages")) {
                for entry in packages.filter_map(Result::ok) {
                    let path = entry.path();
                    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                        continue;
                    };
                    if name.starts_with("Claude_") {
                        push_unique_path(
                            &mut paths,
                            path.join("LocalCache")
                                .join("Roaming")
                                .join("Claude")
                                .join("claude_desktop_config.json"),
                        );
                    }
                }
            }
        }
        if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
            push_unique_path(
                &mut paths,
                appdata.join("Claude").join("claude_desktop_config.json"),
            );
        }
    }

    paths
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn claude_desktop_threep_config_path() -> PathBuf {
    claude_desktop_threep_config_path_for_platform(
        current_platform(),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()),
    )
}

fn claude_desktop_org_plugins_dir() -> PathBuf {
    claude_desktop_org_plugins_dir_for_platform(
        current_platform(),
        std::env::var_os("ProgramFiles").map(PathBuf::from),
        directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()),
    )
}

fn claude_desktop_org_plugins_dir_for_platform(
    platform: DesktopPlatform,
    program_files: Option<PathBuf>,
    home: Option<PathBuf>,
) -> PathBuf {
    match platform {
        DesktopPlatform::Windows => program_files
            .unwrap_or_else(|| PathBuf::from("C:\\Program Files"))
            .join("Claude")
            .join("org-plugins"),
        DesktopPlatform::Macos => PathBuf::from("/Library/Application Support/Claude/org-plugins"),
        DesktopPlatform::Other => home
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("Claude")
            .join("org-plugins"),
    }
}

fn claude_desktop_threep_config_library_dir() -> PathBuf {
    claude_desktop_threep_config_library_dir_for_platform(
        current_platform(),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()),
    )
}

fn claude_desktop_threep_config_path_for_platform(
    platform: DesktopPlatform,
    local_appdata: Option<PathBuf>,
    home: Option<PathBuf>,
) -> PathBuf {
    claude_desktop_threep_config_root_for_platform(platform, local_appdata, home)
        .join("claude_desktop_config.json")
}

fn claude_desktop_threep_config_library_dir_for_platform(
    platform: DesktopPlatform,
    local_appdata: Option<PathBuf>,
    home: Option<PathBuf>,
) -> PathBuf {
    claude_desktop_threep_config_root_for_platform(platform, local_appdata, home)
        .join("configLibrary")
}

fn claude_desktop_threep_config_root_for_platform(
    platform: DesktopPlatform,
    local_appdata: Option<PathBuf>,
    home: Option<PathBuf>,
) -> PathBuf {
    match platform {
        DesktopPlatform::Windows => local_appdata
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Claude-3p"),
        DesktopPlatform::Macos => home
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Application Support")
            .join("Claude-3p"),
        DesktopPlatform::Other => home
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("Claude-3p"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DesktopPlatform {
    Windows,
    Macos,
    Other,
}

fn current_platform() -> DesktopPlatform {
    if cfg!(windows) {
        DesktopPlatform::Windows
    } else if cfg!(target_os = "macos") {
        DesktopPlatform::Macos
    } else {
        DesktopPlatform::Other
    }
}

fn claude_desktop_config_path_for_platform(
    platform: DesktopPlatform,
    local_appdata: Option<PathBuf>,
    home: Option<PathBuf>,
) -> PathBuf {
    match platform {
        DesktopPlatform::Windows => local_appdata
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Claude"),
        DesktopPlatform::Macos => home
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Application Support")
            .join("Claude"),
        DesktopPlatform::Other => home
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("Claude"),
    }
    .join("claude_desktop_config.json")
}

fn backup_claude_desktop_config(path: &PathBuf) -> anyhow::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let backup_path = path.with_extension("json.bak");
    std::fs::copy(path, &backup_path)?;
    Ok(Some(backup_path.to_string_lossy().to_string()))
}

fn read_deployment_mode(path: &Path) -> Option<String> {
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    raw.get("deploymentMode")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn read_claude_desktop_meta_applied_id(path: &Path) -> Option<String> {
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    raw.get("appliedId")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn write_claude_desktop_deployment_mode(path: &Path, mode: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut config = if path.exists() {
        serde_json::from_str::<Value>(&std::fs::read_to_string(path)?).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    let root = config
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude Desktop config must be a JSON object"))?;
    root.insert("deploymentMode".to_string(), json!(mode));
    crate::settings::atomic_write(path, serde_json::to_string_pretty(&config)?.as_bytes())
}

fn claude_desktop_dev_mode_profile_path(config_library_dir: &Path) -> PathBuf {
    config_library_dir.join(format!("{CLAUDE_DESKTOP_DEV_PROFILE_ID}.json"))
}

fn claude_desktop_dev_mode_is_configured(
    normal_config_paths: &[PathBuf],
    threep_config_path: &Path,
    profile_path: &Path,
    profile_meta_path: &Path,
) -> bool {
    !normal_config_paths.is_empty()
        && normal_config_paths
            .iter()
            .all(|path| read_deployment_mode(path).as_deref() == Some("3p"))
        && read_deployment_mode(threep_config_path).as_deref() == Some("3p")
        && read_claude_desktop_meta_applied_id(profile_meta_path).as_deref()
            == Some(CLAUDE_DESKTOP_DEV_PROFILE_ID)
        && profile_path.is_file()
}

fn resolve_claude_desktop_dev_mode_profile(
    provider_request: Option<&ClaudeDesktopProviderRequest>,
) -> anyhow::Result<Option<ClaudeDesktopDevModeProfile>> {
    if let Some(request) = provider_request {
        let base_url = request.base_url.trim().to_string();
        let api_key = request.api_key.trim().to_string();
        if base_url.is_empty() {
            return Ok(None);
        }
        validate_claude_desktop_gateway_url(&base_url)?;
        return Ok(Some(ClaudeDesktopDevModeProfile {
            name: if request.name.trim().is_empty() {
                CLAUDE_DESKTOP_DEV_PROFILE_NAME.to_string()
            } else {
                request.name.trim().to_string()
            },
            base_url,
            api_key,
            model_list: if request.model_list.trim().is_empty() {
                CLAUDE_DESKTOP_DEFAULT_MODEL_LIST.to_string()
            } else {
                request.model_list.clone()
            },
        }));
    }
    let settings = crate::settings::SettingsStore::default()
        .load()
        .context("读取 Claude Desktop 开发模式供应商设置失败")?;
    let relay = settings.active_relay_profile();
    let base_url = relay_profile_base_url_for_claude_desktop(&relay, &settings);
    let api_key = relay_profile_api_key_for_claude_desktop(&relay, &settings);
    if base_url.trim().is_empty() || api_key.trim().is_empty() {
        return Ok(None);
    }
    validate_claude_desktop_gateway_url(&base_url)?;
    Ok(Some(ClaudeDesktopDevModeProfile {
        name: if relay.name.trim().is_empty() {
            CLAUDE_DESKTOP_DEV_PROFILE_NAME.to_string()
        } else {
            relay.name.trim().to_string()
        },
        base_url,
        api_key,
        model_list: if relay.model_list.trim().is_empty() {
            CLAUDE_DESKTOP_DEFAULT_MODEL_LIST.to_string()
        } else {
            relay.model_list.clone()
        },
    }))
}

fn relay_profile_base_url_for_claude_desktop(
    relay: &crate::settings::RelayProfile,
    settings: &crate::settings::BackendSettings,
) -> String {
    let provider_base_url = provider_string_from_toml(&relay.config_contents, "base_url")
        .filter(|value| !value.trim().is_empty());
    provider_base_url
        .or_else(|| {
            if relay.upstream_base_url.trim().is_empty() {
                None
            } else {
                Some(relay.upstream_base_url.trim().to_string())
            }
        })
        .or_else(|| {
            if relay.base_url.trim().is_empty() {
                None
            } else {
                Some(relay.base_url.trim().to_string())
            }
        })
        .or_else(|| {
            if settings.relay_base_url.trim().is_empty() {
                None
            } else {
                Some(settings.relay_base_url.trim().to_string())
            }
        })
        .unwrap_or_default()
}

fn relay_profile_api_key_for_claude_desktop(
    relay: &crate::settings::RelayProfile,
    settings: &crate::settings::BackendSettings,
) -> String {
    experimental_bearer_token_from_toml(&relay.config_contents)
        .or_else(|| openai_api_key_from_json(&relay.auth_contents))
        .or_else(|| {
            if relay.api_key.trim().is_empty() {
                None
            } else {
                Some(relay.api_key.trim().to_string())
            }
        })
        .or_else(|| {
            if settings.relay_api_key.trim().is_empty() {
                None
            } else {
                Some(settings.relay_api_key.trim().to_string())
            }
        })
        .unwrap_or_default()
}

fn provider_string_from_toml(contents: &str, key: &str) -> Option<String> {
    let doc = contents.parse::<toml_edit::DocumentMut>().ok()?;
    let value = doc
        .get("model_providers")
        .and_then(toml_edit::Item::as_table)
        .and_then(|providers| {
            providers.iter().find_map(|(_, item)| {
                item.get(key)
                    .and_then(toml_edit::Item::as_value)
                    .and_then(toml_edit::Value::as_str)
            })
        })
        .or_else(|| {
            doc.get(key)
                .and_then(toml_edit::Item::as_value)
                .and_then(toml_edit::Value::as_str)
        })?;
    Some(value.trim().to_string()).filter(|value| !value.is_empty())
}

fn experimental_bearer_token_from_toml(contents: &str) -> Option<String> {
    provider_string_from_toml(contents, "experimental_bearer_token")
}

fn openai_api_key_from_json(contents: &str) -> Option<String> {
    let auth: Value = serde_json::from_str(contents).ok()?;
    auth.get("OPENAI_API_KEY")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn validate_claude_desktop_gateway_url(base_url: &str) -> anyhow::Result<()> {
    let parsed = Url::parse(base_url.trim())
        .with_context(|| format!("Claude Desktop 供应商 Base URL 无效：{}", base_url.trim()))?;
    match parsed.scheme() {
        "https" => Ok(()),
        "http"
            if parsed
                .host_str()
                .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1")) =>
        {
            Ok(())
        }
        _ => anyhow::bail!(
            "Claude Desktop 供应商 Base URL 仅允许 https://，或本机 http://localhost / 127.0.0.1 / [::1]。"
        ),
    }
}

fn write_claude_desktop_dev_mode_profile(
    path: &Path,
    provider: &ClaudeDesktopDevModeProfile,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    validate_claude_desktop_gateway_url(&provider.base_url)?;
    let mut profile = json!({
        "coworkEgressAllowedHosts": ["*"],
        "disableDeploymentModeChooser": true,
        "inferenceGatewayApiKey": provider.api_key.trim(),
        "inferenceGatewayAuthScheme": "bearer",
        "inferenceGatewayBaseUrl": provider.base_url.trim().trim_end_matches('/'),
        "inferenceProvider": "gateway"
    });
    let models = parse_claude_desktop_model_list(&provider.model_list);
    if !models.is_empty() {
        profile["inferenceModels"] = Value::Array(models);
    }
    crate::settings::atomic_write(path, serde_json::to_string_pretty(&profile)?.as_bytes())
}

fn parse_claude_desktop_model_list(raw: &str) -> Vec<Value> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let supports_1m = line.to_ascii_lowercase().contains("[1m]");
            let name = line
                .replace("[1M]", "")
                .replace("[1m]", "")
                .trim()
                .to_string();
            if supports_1m {
                json!({ "name": name, "supports1m": true })
            } else {
                json!({ "name": name })
            }
        })
        .collect()
}

fn write_claude_desktop_dev_mode_meta(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut raw = if path.exists() {
        serde_json::from_str::<Value>(&std::fs::read_to_string(path)?).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    if !raw.is_object() {
        raw = json!({});
    }
    let root = raw
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude Desktop profile metadata must be a JSON object"))?;
    let mut entries = root
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    entries.retain(|entry| {
        entry.get("id").and_then(Value::as_str) != Some(CLAUDE_DESKTOP_DEV_PROFILE_ID)
    });
    entries.push(json!({
        "id": CLAUDE_DESKTOP_DEV_PROFILE_ID,
        "name": CLAUDE_DESKTOP_DEV_PROFILE_NAME
    }));
    root.insert(
        "appliedId".to_string(),
        json!(CLAUDE_DESKTOP_DEV_PROFILE_ID),
    );
    root.insert("entries".to_string(), Value::Array(entries));
    crate::settings::atomic_write(path, serde_json::to_string_pretty(&raw)?.as_bytes())
}

fn upsert_claude_desktop_mcp_server(
    config_path: &PathBuf,
    server_name: &str,
    server_config: serde_json::Value,
) -> anyhow::Result<()> {
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut config = if config_path.exists() {
        serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(config_path)?)
            .unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    let root = config
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude Desktop config must be a JSON object"))?;
    let mcp_servers = root.entry("mcpServers").or_insert_with(|| json!({}));
    let servers = mcp_servers
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude Desktop mcpServers must be a JSON object"))?;
    servers.insert(server_name.to_string(), server_config);
    crate::settings::atomic_write(
        config_path,
        serde_json::to_string_pretty(&config)?.as_bytes(),
    )?;
    Ok(())
}

fn remove_claude_desktop_mcp_server(
    config_path: &PathBuf,
    server_name: &str,
) -> anyhow::Result<()> {
    if !config_path.exists() {
        return Ok(());
    }
    let mut config =
        serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(config_path)?)
            .unwrap_or_else(|_| json!({}));
    if let Some(servers) = config
        .get_mut("mcpServers")
        .and_then(serde_json::Value::as_object_mut)
    {
        servers.remove(server_name);
    }
    crate::settings::atomic_write(
        config_path,
        serde_json::to_string_pretty(&config)?.as_bytes(),
    )?;
    Ok(())
}

fn claude_desktop_server_name_for_record(record: &PluginHubInstallRecord) -> String {
    match record.id.as_str() {
        "ponytail:claude-desktop-mcp" => ponytail_mcp_server_name(),
        "desktop:claude-codex-pro-codex" => desktop_mcp_server_name(),
        _ => safe_id(&record.name),
    }
}

fn claude_desktop_mcp_config_preview(server_name: &str, command: &[String]) -> String {
    let command_name = command
        .first()
        .cloned()
        .unwrap_or_else(|| "claude".to_string());
    let args = serde_json::to_string(&command.iter().skip(1).collect::<Vec<_>>())
        .unwrap_or_else(|_| "[]".to_string());
    format!(
        "{{\n  \"mcpServers\": {{\n    \"{server_name}\": {{\n      \"command\": \"{command_name}\",\n      \"args\": {args}\n    }}\n  }}\n}}"
    )
}

fn record_install(
    item: &PluginCatalogItem,
    command: Vec<String>,
    backup_path: Option<String>,
) -> anyhow::Result<()> {
    record_install_with_managed_paths(item, command, backup_path, Vec::new())
}

fn record_install_with_managed_paths(
    item: &PluginCatalogItem,
    command: Vec<String>,
    backup_path: Option<String>,
    managed_paths: Vec<String>,
) -> anyhow::Result<()> {
    let mut records = load_installed_records().unwrap_or_default();
    records.insert(
        item.id.clone(),
        PluginHubInstallRecord {
            id: item.id.clone(),
            name: item.name.clone(),
            install_kind: item.install_kind,
            installed_at: current_unix_timestamp_string(),
            command,
            source_url: item.homepage.clone(),
            backup_path,
            managed_paths,
            verified: true,
        },
    );
    save_installed_records(&records)
}

fn save_installed_records(
    records: &BTreeMap<String, PluginHubInstallRecord>,
) -> anyhow::Result<()> {
    let path = installed_records_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let values = records.values().cloned().collect::<Vec<_>>();
    crate::settings::atomic_write(&path, serde_json::to_string_pretty(&values)?.as_bytes())
}

fn status_for(
    id: &str,
    installed: &BTreeMap<String, PluginHubInstallRecord>,
    fallback: InstallStatus,
) -> InstallStatus {
    if installed.contains_key(id) {
        InstallStatus::Installed
    } else {
        fallback
    }
}

fn ok_source(id: &str, label: &str, url: &str, item_count: usize) -> CatalogSource {
    CatalogSource {
        id: id.to_string(),
        label: label.to_string(),
        url: url.to_string(),
        status: "ok".to_string(),
        message: "已加载".to_string(),
        item_count,
    }
}

fn failed_source(id: &str, label: &str, url: &str, error: anyhow::Error) -> CatalogSource {
    CatalogSource {
        id: id.to_string(),
        label: label.to_string(),
        url: url.to_string(),
        status: "failed".to_string(),
        message: error.to_string(),
        item_count: 0,
    }
}

fn installed_records_path() -> PathBuf {
    plugin_hub_dir().join("installed.json")
}

fn plugin_hub_dir() -> PathBuf {
    crate::paths::default_app_state_dir().join("plugin-hub")
}

fn value_string(raw: &Value, key: &str) -> String {
    raw.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn string_array(items: &[Value]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    items
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter_map(|value| {
            if seen.insert(value.to_lowercase()) {
                Some(value.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn parse_csv_records(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                let _ = chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                row.push(field.trim().to_string());
                field.clear();
            }
            '\n' if !in_quotes => {
                row.push(field.trim().to_string());
                field.clear();
                if row.iter().any(|value| !value.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            '\r' if !in_quotes => {}
            _ => field.push(ch),
        }
    }

    if !field.is_empty() || !row.is_empty() {
        row.push(field.trim().to_string());
        if row.iter().any(|value| !value.is_empty()) {
            rows.push(row);
        }
    }

    rows
}

fn safe_id(value: &str) -> String {
    let mut result = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | '.') {
            result.push(ch);
        } else if ch.is_whitespace() {
            result.push('-');
        }
    }
    let trimmed = result.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "plugin-hub-item".to_string()
    } else {
        trimmed
    }
}

fn current_unix_timestamp_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

impl InstallStatus {
    fn status_rank(self) -> u8 {
        match self {
            InstallStatus::NotInstalled => 0,
            InstallStatus::NeedsReview => 1,
            InstallStatus::Installed => 2,
            InstallStatus::Unsupported => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_test_plugin_outcome(id: &str) -> PluginInstallOutcome {
        let item = PluginCatalogItem {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            source_id: String::new(),
            source_label: String::new(),
            source_url: String::new(),
            category: String::new(),
            author: String::new(),
            homepage: String::new(),
            license: String::new(),
            tags: Vec::new(),
            install_kind: InstallKind::ClaudeDesktopMcp,
            install_status: InstallStatus::NotInstalled,
            install_command: Vec::new(),
            config_preview: String::new(),
            risk: String::new(),
            requirements: Vec::new(),
        };
        PluginInstallOutcome {
            item: item.clone(),
            preview: preview_for_item(item),
            installed: true,
            message: String::new(),
            stdout: String::new(),
            stderr: String::new(),
            backup_path: None,
        }
    }

    #[test]
    fn csv_parser_handles_quotes() {
        let rows =
            parse_csv_records("ID,Name,Description\none,\"Two, too\",\"hello \"\"world\"\"\"\n");
        assert_eq!(rows[1][1], "Two, too");
        assert_eq!(rows[1][2], "hello \"world\"");
    }

    #[test]
    fn official_marketplace_parser_reads_plugins() {
        let raw = serde_json::json!({
            "plugins": [{
                "name": "demo",
                "description": "Demo plugin",
                "category": "development",
                "author": { "name": "Anthropic" },
                "homepage": "https://example.com",
                "tags": ["mcp"]
            }]
        });
        let items = parse_official_marketplace(raw, &BTreeMap::new());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].install_kind, InstallKind::ClaudePluginMarketplace);
        assert_eq!(items[0].install_command, official_marketplace_add_command());
        assert!(
            items[0]
                .config_preview
                .contains("claude plugin install demo@claude-plugins-official")
        );
    }

    #[test]
    fn awesome_classifier_detects_mcp_and_skills() {
        assert_eq!(
            classify_awesome_item(
                "x",
                "Project Scaffolding & MCP",
                "https://github.com/a/b",
                ""
            ),
            InstallKind::McpServer
        );
        assert_eq!(
            classify_awesome_item("x", "Agent Skills", "https://github.com/a/b", ""),
            InstallKind::SkillBundle
        );
    }

    #[test]
    fn awesome_classifier_does_not_make_community_plugins_auto_installable() {
        assert_eq!(
            classify_awesome_item(
                "x",
                "Plugins",
                "https://github.com/example/community-plugin",
                "Community Claude plugin"
            ),
            InstallKind::ResourceLink
        );
    }

    #[test]
    fn builtin_catalog_exposes_codex_desktop_mcp_item() {
        let items = builtin_claude_desktop_items(&BTreeMap::new());
        let item = items
            .iter()
            .find(|item| item.id == "desktop:claude-codex-pro-codex")
            .expect("codex desktop MCP item");

        assert_eq!(item.install_kind, InstallKind::ClaudeDesktopMcp);
        assert_eq!(item.install_command, vec!["claude", "mcp", "serve"]);
        assert!(item.tags.contains(&"codex".to_string()));
    }

    #[test]
    fn codex_plugin_repository_item_is_exposed_as_reviewable_resource() {
        let items = codex_plugin_repository_items(&BTreeMap::new());
        let item = items
            .iter()
            .find(|item| item.id == "codex-plugins:openai")
            .expect("codex plugin repository item");

        assert_eq!(item.name, "OpenAI Codex Plugins");
        assert_eq!(item.homepage, "https://github.com/openai/plugins");
        assert_eq!(item.install_kind, InstallKind::ResourceLink);
        assert_eq!(item.install_status, InstallStatus::Unsupported);
        assert!(item.tags.contains(&"codex".to_string()));
        assert!(item.description.contains("Codex"));
    }

    #[test]
    fn ponytail_catalog_exposes_installable_targets() {
        let items = ponytail_catalog_items(&BTreeMap::new());
        let ids = items
            .iter()
            .map(|item| (item.id.as_str(), item.install_kind))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(items.len(), 6);
        assert_eq!(
            ids["ponytail:claude-code-plugin"],
            InstallKind::ClaudeCodePlugin
        );
        assert_eq!(ids["ponytail:codex-plugin"], InstallKind::CodexPlugin);
        assert_eq!(ids["ponytail:copilot-plugin"], InstallKind::CopilotPlugin);
        assert_eq!(
            ids["ponytail:claude-desktop-mcp"],
            InstallKind::ClaudeDesktopMcp
        );
        assert_eq!(
            ids[PONYTAIL_CLAUDE_DESKTOP_ORG_ID],
            InstallKind::ClaudeDesktopOrgPlugin
        );
        assert_eq!(
            ids["ponytail:codex-skills"],
            InstallKind::ManagedSkillBundle
        );
        assert!(items.iter().all(|item| item.source_id == "ponytail"));
    }

    #[test]
    fn claude_desktop_org_plugin_uses_local_dev_mode_install_not_claude_cli_login() {
        let item = ponytail_catalog_items(&BTreeMap::new())
            .into_iter()
            .find(|item| item.id == PONYTAIL_CLAUDE_DESKTOP_ORG_ID)
            .expect("ponytail claude desktop organization plugin item");
        let preview = preview_for_item(item.clone());
        let combined = format!(
            "{}\n{}\n{}\n{}",
            item.requirements.join("\n"),
            item.risk,
            preview.message,
            preview.config_diff
        )
        .to_lowercase();

        assert_eq!(item.install_kind, InstallKind::ClaudeDesktopOrgPlugin);
        assert_eq!(preview.action, "claude_desktop_org_plugin");
        assert!(preview.command.is_empty());
        assert!(combined.contains("development-mode local folders"));
        assert!(combined.contains("no claude cli login"));
        assert!(!combined.contains("claude plugin install"));
        assert!(!combined.contains("claude plugin marketplace add"));
        assert!(!combined.contains("official plugin"));
    }

    #[test]
    fn claude_desktop_local_bundle_message_stays_local_and_login_free() {
        let outcome = ClaudeDesktopLocalBundleOutcome {
            dev_mode: ClaudeDesktopDevModeOutcome {
                configured: true,
                normal_config_path: String::new(),
                threep_config_path: String::new(),
                profile_path: String::new(),
                profile_meta_path: String::new(),
                backup_paths: Vec::new(),
                message: String::new(),
            },
            codex_mcp: empty_test_plugin_outcome("desktop:claude-codex-pro-codex"),
            ponytail_mcp: empty_test_plugin_outcome("ponytail:claude-desktop-mcp"),
            organization_plugin: ClaudeDesktopOrgPluginOutcome {
                installed: true,
                org_plugins_dir: String::new(),
                plugin_dir: String::new(),
                manifest_path: String::new(),
                plugin_json_path: String::new(),
                copied_skills: Vec::new(),
                backup_path: None,
                message: String::new(),
            },
            message: "Claude Desktop development mode, MCP config, Ponytail MCP, and local organization plugin skills were written locally. No Claude CLI login or official plugin marketplace install was used. Fully restart Claude Desktop.".to_string(),
        };
        let message = outcome.message.to_lowercase();

        assert!(message.contains("written locally"));
        assert!(message.contains("no claude cli login"));
        assert!(!message.contains("claude plugin install"));
        assert!(!message.contains("claude plugin marketplace add"));
    }

    #[test]
    fn ponytail_codex_preview_adds_marketplace_without_trusting_hooks() {
        let item = ponytail_catalog_items(&BTreeMap::new())
            .into_iter()
            .find(|item| item.id == "ponytail:codex-plugin")
            .expect("ponytail codex plugin item");
        let preview = preview_for_item(item);

        assert!(preview.can_install);
        assert_eq!(preview.action, "codex_cli_plugin");
        assert_eq!(
            preview.command,
            vec![
                "codex",
                "plugin",
                "marketplace",
                "add",
                "DietrichGebert/ponytail",
                "--json"
            ]
        );
        assert!(
            preview
                .config_diff
                .contains("codex plugin add ponytail@ponytail --json")
        );
        assert!(
            preview.message.contains("does not silently trust")
                || preview.message.contains("Hooks remain untrusted")
        );
        let plan = cli_plugin_install_plan(InstallKind::CodexPlugin).unwrap();
        assert_eq!(plan.len(), 3);
        assert_eq!(plan[0], ponytail_codex_install_command());
        assert_eq!(
            plan[1],
            vec!["codex", "plugin", "list", "--available", "--json"]
        );
        assert_eq!(
            plan[2],
            vec!["codex", "plugin", "add", "ponytail@ponytail", "--json"]
        );
    }

    #[test]
    fn codex_available_list_parser_finds_ponytail() {
        ensure_codex_available_list_contains_ponytail(
            r#"{"available":[{"pluginId":"ponytail@ponytail","name":"Ponytail","marketplaceName":"ponytail"}],"installed":[]}"#,
        )
        .unwrap();
        assert!(
            ensure_codex_available_list_contains_ponytail(
                r#"{"available":[{"pluginId":"other@demo","name":"Other"}],"installed":[]}"#,
            )
            .is_err()
        );
    }

    #[test]
    fn codex_plugin_add_parser_reads_installed_path() {
        assert_eq!(
            parse_codex_plugin_add_installed_path(
                r#"{"pluginId":"ponytail@ponytail","installedPath":"C:\\Users\\Damon\\.codex\\plugins\\cache\\ponytail"}"#
            ),
            Some("C:\\Users\\Damon\\.codex\\plugins\\cache\\ponytail".to_string())
        );
    }

    #[test]
    fn ponytail_desktop_mcp_preview_targets_ponytail_server() {
        let item = ponytail_catalog_items(&BTreeMap::new())
            .into_iter()
            .find(|item| item.id == "ponytail:claude-desktop-mcp")
            .expect("ponytail mcp item");
        let preview = preview_for_item(item);

        assert!(preview.can_install);
        assert_eq!(preview.action, "claude_desktop_mcp_config");
        assert!(preview.config_diff.contains("\"ponytail\""));
        assert!(preview.config_diff.contains("ponytail-mcp"));
        assert!(preview.config_diff.contains("index.js"));
    }

    #[test]
    fn codex_desktop_mcp_preview_targets_claude_desktop_config() {
        let item = builtin_claude_desktop_items(&BTreeMap::new()).remove(0);
        let preview = preview_for_item(item);

        assert!(preview.can_install);
        assert_eq!(preview.action, "claude_desktop_mcp_config");
        assert!(preview.config_diff.contains("\"mcpServers\""));
        assert!(preview.config_diff.contains("\"claude-codex-pro-codex\""));
        assert!(preview.config_diff.contains("\"claude\""));
        assert!(preview.config_diff.contains("\"mcp\""));
        assert!(preview.config_diff.contains("\"serve\""));
    }

    #[test]
    fn official_plugin_preview_is_installable_through_claude_cli() {
        let item = PluginCatalogItem {
            id: "official:demo".to_string(),
            name: "demo".to_string(),
            description: String::new(),
            source_id: "official".to_string(),
            source_label: "Claude official plugins".to_string(),
            source_url: OFFICIAL_MARKETPLACE_URL.to_string(),
            category: "claude-plugin".to_string(),
            author: "Anthropic".to_string(),
            homepage: String::new(),
            license: String::new(),
            tags: Vec::new(),
            install_kind: InstallKind::ClaudePluginMarketplace,
            install_status: InstallStatus::NotInstalled,
            install_command: official_marketplace_add_command(),
            config_preview: official_plugin_plan_text("demo"),
            risk: String::new(),
            requirements: vec!["claude CLI".to_string()],
        };

        let preview = preview_for_item(item);

        assert!(preview.can_install);
        assert_eq!(preview.action, "claude_plugin_cli");
        assert_eq!(
            preview.command,
            vec![
                "claude",
                "plugin",
                "marketplace",
                "add",
                "anthropics/claude-plugins-official"
            ]
        );
        assert!(
            preview
                .config_diff
                .contains("claude plugin install demo@claude-plugins-official")
        );
        assert_eq!(
            official_plugin_install_plan(&preview.item),
            vec![
                vec![
                    "claude",
                    "plugin",
                    "marketplace",
                    "add",
                    "anthropics/claude-plugins-official"
                ],
                vec![
                    "claude",
                    "plugin",
                    "install",
                    "demo@claude-plugins-official"
                ],
            ]
        );
    }

    #[test]
    fn official_marketplace_parser_previews_add_then_install() {
        let raw = serde_json::json!({
            "plugins": [{
                "name": "demo",
                "description": "Demo plugin"
            }]
        });

        let items = parse_official_marketplace(raw, &BTreeMap::new());

        assert_eq!(
            items[0].install_command,
            vec![
                "claude",
                "plugin",
                "marketplace",
                "add",
                "anthropics/claude-plugins-official"
            ]
        );
        assert!(
            items[0]
                .config_preview
                .contains("claude plugin install demo@claude-plugins-official")
        );
    }
    #[test]
    fn community_mcp_preview_does_not_allow_placeholder_install() {
        let item = PluginCatalogItem {
            id: "awesome:demo-mcp".to_string(),
            name: "Demo MCP".to_string(),
            description: "Community MCP".to_string(),
            source_id: "awesome".to_string(),
            source_label: "Awesome Claude Code".to_string(),
            source_url: AWESOME_CLAUDE_CODE_CSV_URL.to_string(),
            category: "MCP".to_string(),
            author: "Community".to_string(),
            homepage: "https://github.com/example/demo-mcp".to_string(),
            license: String::new(),
            tags: vec!["mcp".to_string()],
            install_kind: InstallKind::McpServer,
            install_status: InstallStatus::NeedsReview,
            install_command: Vec::new(),
            config_preview: String::new(),
            risk: String::new(),
            requirements: Vec::new(),
        };

        let preview = preview_for_item(item);

        assert!(!preview.can_install);
        assert_eq!(preview.action, "claude_desktop_mcp_config");
        assert!(preview.config_diff.contains("<package-or-command>"));
        assert!(preview.message.contains("command/args"));
    }

    #[test]
    fn claude_desktop_config_upsert_preserves_existing_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("claude_desktop_config.json");
        std::fs::write(
            &path,
            r#"{"windowBounds":{"width":1200},"mcpServers":{"existing":{"command":"node","args":["server.js"]}}}"#,
        )
        .unwrap();

        upsert_claude_desktop_mcp_server(
            &path,
            "claude-codex-pro-codex",
            json!({"command": "claude", "args": ["mcp", "serve"], "env": {}}),
        )
        .unwrap();

        let parsed: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(parsed["windowBounds"]["width"], 1200);
        assert_eq!(parsed["mcpServers"]["existing"]["command"], "node");
        assert_eq!(
            parsed["mcpServers"]["claude-codex-pro-codex"]["command"],
            "claude"
        );
        assert_eq!(
            parsed["mcpServers"]["claude-codex-pro-codex"]["args"],
            json!(["mcp", "serve"])
        );
    }

    #[test]
    fn claude_desktop_config_path_uses_platform_locations() {
        assert_eq!(
            claude_desktop_config_path_for_platform(
                DesktopPlatform::Windows,
                Some(PathBuf::from("local")),
                Some(PathBuf::from("home")),
            ),
            PathBuf::from("local")
                .join("Claude")
                .join("claude_desktop_config.json")
        );
        assert_eq!(
            claude_desktop_config_path_for_platform(
                DesktopPlatform::Macos,
                Some(PathBuf::from("appdata")),
                Some(PathBuf::from("home")),
            ),
            PathBuf::from("home")
                .join("Library")
                .join("Application Support")
                .join("Claude")
                .join("claude_desktop_config.json")
        );
        assert_eq!(
            claude_desktop_config_path_for_platform(
                DesktopPlatform::Other,
                None,
                Some(PathBuf::from("home")),
            ),
            PathBuf::from("home")
                .join(".config")
                .join("Claude")
                .join("claude_desktop_config.json")
        );
    }

    #[test]
    fn claude_desktop_org_plugin_paths_use_platform_locations() {
        assert_eq!(
            claude_desktop_org_plugins_dir_for_platform(
                DesktopPlatform::Windows,
                Some(PathBuf::from("C:\\Program Files")),
                Some(PathBuf::from("home")),
            ),
            PathBuf::from("C:\\Program Files")
                .join("Claude")
                .join("org-plugins")
        );
        assert_eq!(
            claude_desktop_threep_config_library_dir_for_platform(
                DesktopPlatform::Windows,
                Some(PathBuf::from("local")),
                Some(PathBuf::from("home")),
            ),
            PathBuf::from("local")
                .join("Claude-3p")
                .join("configLibrary")
        );
        assert_eq!(
            claude_desktop_threep_config_library_dir_for_platform(
                DesktopPlatform::Macos,
                None,
                Some(PathBuf::from("home")),
            ),
            PathBuf::from("home")
                .join("Library")
                .join("Application Support")
                .join("Claude-3p")
                .join("configLibrary")
        );
    }

    #[test]
    fn claude_desktop_dev_mode_paths_use_platform_locations() {
        assert_eq!(
            claude_desktop_threep_config_path_for_platform(
                DesktopPlatform::Windows,
                Some(PathBuf::from("local")),
                Some(PathBuf::from("home")),
            ),
            PathBuf::from("local")
                .join("Claude-3p")
                .join("claude_desktop_config.json")
        );
        assert_eq!(
            claude_desktop_threep_config_path_for_platform(
                DesktopPlatform::Macos,
                None,
                Some(PathBuf::from("home")),
            ),
            PathBuf::from("home")
                .join("Library")
                .join("Application Support")
                .join("Claude-3p")
                .join("claude_desktop_config.json")
        );
    }

    #[test]
    fn claude_desktop_dev_mode_writers_preserve_existing_json() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("claude_desktop_config.json");
        let meta_path = dir.path().join("configLibrary").join("_meta.json");
        std::fs::create_dir_all(meta_path.parent().unwrap()).unwrap();
        std::fs::write(
            &config_path,
            r#"{"windowBounds":{"width":1200},"mcpServers":{"existing":{"command":"node"}}}"#,
        )
        .unwrap();
        std::fs::write(
            &meta_path,
            json!({
                "entries": [
                    {"id": "existing-profile", "name": "Existing"},
                    {"id": CLAUDE_DESKTOP_DEV_PROFILE_ID, "name": "Old Name"}
                ],
                "other": true
            })
            .to_string(),
        )
        .unwrap();

        write_claude_desktop_deployment_mode(&config_path, "3p").unwrap();
        write_claude_desktop_dev_mode_meta(&meta_path).unwrap();

        let config: Value =
            serde_json::from_str(&std::fs::read_to_string(config_path).unwrap()).unwrap();
        let meta: Value =
            serde_json::from_str(&std::fs::read_to_string(meta_path).unwrap()).unwrap();
        assert_eq!(config["deploymentMode"], "3p");
        assert_eq!(config["windowBounds"]["width"], 1200);
        assert_eq!(config["mcpServers"]["existing"]["command"], "node");
        assert_eq!(meta["appliedId"], CLAUDE_DESKTOP_DEV_PROFILE_ID);
        assert_eq!(meta["other"], true);
        let entries = meta["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .any(|entry| entry["id"] == "existing-profile")
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry["id"] == CLAUDE_DESKTOP_DEV_PROFILE_ID
                    && entry["name"] == CLAUDE_DESKTOP_DEV_PROFILE_NAME)
        );
    }

    #[test]
    fn claude_desktop_dev_mode_profile_writes_gateway_provider_shape() {
        let dir = tempfile::tempdir().unwrap();
        let profile_path = dir
            .path()
            .join("configLibrary")
            .join(format!("{CLAUDE_DESKTOP_DEV_PROFILE_ID}.json"));

        write_claude_desktop_dev_mode_profile(
            &profile_path,
            &ClaudeDesktopDevModeProfile {
                name: "TopoReduce".to_string(),
                base_url: "https://api.toporeduce.cn/v1/".to_string(),
                api_key: "sk-test-secret".to_string(),
                model_list: "claude-sonnet-4-6\nclaude-opus-4-8 [1m]".to_string(),
            },
        )
        .unwrap();

        let profile: Value =
            serde_json::from_str(&std::fs::read_to_string(profile_path).unwrap()).unwrap();
        assert_eq!(profile["inferenceProvider"], "gateway");
        assert_eq!(
            profile["inferenceGatewayBaseUrl"],
            "https://api.toporeduce.cn/v1"
        );
        assert_eq!(profile["inferenceGatewayApiKey"], "sk-test-secret");
        assert_eq!(profile["inferenceGatewayAuthScheme"], "bearer");
        assert_eq!(profile["disableDeploymentModeChooser"], true);
        assert_eq!(profile["coworkEgressAllowedHosts"], json!(["*"]));
        assert_eq!(profile["inferenceModels"][0]["name"], "claude-sonnet-4-6");
        assert_eq!(profile["inferenceModels"][1]["name"], "claude-opus-4-8");
        assert_eq!(profile["inferenceModels"][1]["supports1m"], true);
    }

    #[test]
    fn claude_desktop_dev_mode_profile_allows_empty_api_key_for_first_open() {
        let dir = tempfile::tempdir().unwrap();
        let profile_path = dir
            .path()
            .join("configLibrary")
            .join(format!("{CLAUDE_DESKTOP_DEV_PROFILE_ID}.json"));

        write_claude_desktop_dev_mode_profile(
            &profile_path,
            &ClaudeDesktopDevModeProfile {
                name: "TopoReduce".to_string(),
                base_url: "https://api.toporeduce.cn".to_string(),
                api_key: String::new(),
                model_list: String::new(),
            },
        )
        .unwrap();

        let profile: Value =
            serde_json::from_str(&std::fs::read_to_string(profile_path).unwrap()).unwrap();
        assert_eq!(profile["inferenceProvider"], "gateway");
        assert_eq!(
            profile["inferenceGatewayBaseUrl"],
            "https://api.toporeduce.cn"
        );
        assert_eq!(profile["inferenceGatewayApiKey"], "");
        assert_eq!(profile["inferenceGatewayAuthScheme"], "bearer");
    }

    #[test]
    fn claude_desktop_dev_mode_requires_profile_file() {
        let dir = tempfile::tempdir().unwrap();
        let normal_config_path = dir.path().join("Claude").join("claude_desktop_config.json");
        let threep_config_path = dir
            .path()
            .join("Claude-3p")
            .join("claude_desktop_config.json");
        let profile_path = dir
            .path()
            .join("Claude-3p")
            .join("configLibrary")
            .join(format!("{CLAUDE_DESKTOP_DEV_PROFILE_ID}.json"));
        let meta_path = dir
            .path()
            .join("Claude-3p")
            .join("configLibrary")
            .join("_meta.json");
        write_claude_desktop_deployment_mode(&normal_config_path, "3p").unwrap();
        write_claude_desktop_deployment_mode(&threep_config_path, "3p").unwrap();
        write_claude_desktop_dev_mode_meta(&meta_path).unwrap();

        assert!(!claude_desktop_dev_mode_is_configured(
            std::slice::from_ref(&normal_config_path),
            &threep_config_path,
            &profile_path,
            &meta_path
        ));

        write_claude_desktop_dev_mode_profile(
            &profile_path,
            &ClaudeDesktopDevModeProfile {
                name: "TopoReduce".to_string(),
                base_url: "https://api.toporeduce.cn".to_string(),
                api_key: "sk-test-secret".to_string(),
                model_list: String::new(),
            },
        )
        .unwrap();
        assert!(claude_desktop_dev_mode_is_configured(
            std::slice::from_ref(&normal_config_path),
            &threep_config_path,
            &profile_path,
            &meta_path
        ));
    }

    #[test]
    fn claude_desktop_dev_mode_requires_all_normal_config_paths() {
        let dir = tempfile::tempdir().unwrap();
        let normal_a = dir.path().join("Claude").join("claude_desktop_config.json");
        let normal_b = dir
            .path()
            .join("Packages")
            .join("Claude_test")
            .join("LocalCache")
            .join("Roaming")
            .join("Claude")
            .join("claude_desktop_config.json");
        let threep_config_path = dir
            .path()
            .join("Claude-3p")
            .join("claude_desktop_config.json");
        let profile_path = dir
            .path()
            .join("Claude-3p")
            .join("configLibrary")
            .join(format!("{CLAUDE_DESKTOP_DEV_PROFILE_ID}.json"));
        let meta_path = dir
            .path()
            .join("Claude-3p")
            .join("configLibrary")
            .join("_meta.json");

        write_claude_desktop_deployment_mode(&normal_a, "3p").unwrap();
        write_claude_desktop_deployment_mode(&threep_config_path, "3p").unwrap();
        write_claude_desktop_dev_mode_meta(&meta_path).unwrap();
        write_claude_desktop_dev_mode_profile(
            &profile_path,
            &ClaudeDesktopDevModeProfile {
                name: "TopoReduce".to_string(),
                base_url: "https://api.toporeduce.cn".to_string(),
                api_key: String::new(),
                model_list: String::new(),
            },
        )
        .unwrap();

        assert!(!claude_desktop_dev_mode_is_configured(
            &[normal_a.clone(), normal_b.clone()],
            &threep_config_path,
            &profile_path,
            &meta_path
        ));

        write_claude_desktop_deployment_mode(&normal_b, "3p").unwrap();
        assert!(claude_desktop_dev_mode_is_configured(
            &[normal_a, normal_b],
            &threep_config_path,
            &profile_path,
            &meta_path
        ));
    }

    #[test]
    fn claude_desktop_marketplace_status_uses_official_deep_link_without_auto_write() {
        let status = load_claude_desktop_marketplace_status();

        assert_eq!(status.marketplace, PONYTAIL_MARKETPLACE);
        assert_eq!(status.plugin, "ponytail");
        assert!(!status.can_auto_write);
        assert!(
            status
                .deep_link
                .starts_with("claude://claude.ai/customize/plugins/new?")
        );
        assert!(
            status
                .deep_link
                .contains("marketplace=DietrichGebert%2Fponytail")
        );
        assert!(status.deep_link.contains("plugin=ponytail"));
    }

    #[test]
    fn missing_org_plugin_directory_is_not_reported_writable() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing-org-plugins");

        assert!(!directory_is_writable(&missing));
        assert!(!missing.exists());
    }

    #[test]
    fn ponytail_org_plugin_manifest_matches_claude_desktop_shape() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_json = dir.path().join(".claude-plugin").join("plugin.json");
        let manifest = dir.path().join("manifest.json");
        let copied = vec![
            dir.path()
                .join("skills")
                .join("ponytail")
                .to_string_lossy()
                .to_string(),
        ];

        write_ponytail_org_plugin_json(&plugin_json).unwrap();
        write_ponytail_org_plugin_manifest(&manifest, &copied).unwrap();

        let plugin_raw: Value =
            serde_json::from_str(&std::fs::read_to_string(plugin_json).unwrap()).unwrap();
        let manifest_raw: Value =
            serde_json::from_str(&std::fs::read_to_string(manifest).unwrap()).unwrap();

        assert_eq!(plugin_raw["name"], "ponytail");
        assert_eq!(plugin_raw["version"], "1.0.0");
        assert_eq!(manifest_raw["skills"][0]["skillId"], "ponytail");
        assert_eq!(manifest_raw["skills"][0]["creatorType"], "organization");
        assert_eq!(manifest_raw["skills"][0]["enabled"], true);
    }

    #[test]
    fn copy_skill_dirs_only_copies_valid_skill_directories() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(source.join("valid")).unwrap();
        std::fs::create_dir_all(source.join("ignored")).unwrap();
        std::fs::write(source.join("valid").join("SKILL.md"), "skill").unwrap();
        std::fs::write(source.join("ignored").join("README.md"), "nope").unwrap();

        let copied = copy_skill_dirs(&source, &dest).unwrap();

        assert_eq!(copied.len(), 1);
        assert!(dest.join("valid").join("SKILL.md").is_file());
        assert!(!dest.join("ignored").exists());
    }

    #[test]
    fn claude_desktop_config_remove_preserves_existing_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("claude_desktop_config.json");
        std::fs::write(
            &path,
            r#"{"windowBounds":{"width":1200},"mcpServers":{"existing":{"command":"node"},"claude-codex-pro-codex":{"command":"claude"}}}"#,
        )
        .unwrap();

        remove_claude_desktop_mcp_server(&path, "claude-codex-pro-codex").unwrap();

        let parsed: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(parsed["windowBounds"]["width"], 1200);
        assert_eq!(parsed["mcpServers"]["existing"]["command"], "node");
        assert!(parsed["mcpServers"]["claude-codex-pro-codex"].is_null());
    }

    #[test]
    fn managed_skill_uninstall_removes_copied_paths_and_restores_backups() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        let backup_root = dir.path().join("backup");
        let restored = skills_dir.join("restored");
        let removed = skills_dir.join("removed");
        let restored_backup = backup_root.join("restored");
        std::fs::create_dir_all(&restored).unwrap();
        std::fs::create_dir_all(&removed).unwrap();
        std::fs::create_dir_all(&restored_backup).unwrap();
        std::fs::write(restored.join("SKILL.md"), "installed").unwrap();
        std::fs::write(removed.join("SKILL.md"), "installed").unwrap();
        std::fs::write(restored_backup.join("SKILL.md"), "original").unwrap();
        let record = PluginHubInstallRecord {
            id: "ponytail:codex-skills".to_string(),
            name: "Ponytail Skills for Codex".to_string(),
            install_kind: InstallKind::ManagedSkillBundle,
            installed_at: "1".to_string(),
            command: Vec::new(),
            source_url: PONYTAIL_REPOSITORY_URL.to_string(),
            backup_path: Some(backup_root.to_string_lossy().to_string()),
            managed_paths: vec![
                restored.to_string_lossy().to_string(),
                removed.to_string_lossy().to_string(),
            ],
            verified: true,
        };

        remove_managed_skill_paths_under(
            &record,
            record.managed_paths.iter().map(PathBuf::from).collect(),
            skills_dir.clone(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(restored.join("SKILL.md")).unwrap(),
            "original"
        );
        assert!(!removed.exists());
    }

    #[test]
    fn managed_skill_uninstall_rejects_paths_outside_codex_skills() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside-skill");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("SKILL.md"), "installed").unwrap();
        let record = PluginHubInstallRecord {
            id: "ponytail:codex-skills".to_string(),
            name: "Ponytail Skills for Codex".to_string(),
            install_kind: InstallKind::ManagedSkillBundle,
            installed_at: "1".to_string(),
            command: Vec::new(),
            source_url: PONYTAIL_REPOSITORY_URL.to_string(),
            backup_path: None,
            managed_paths: vec![outside.to_string_lossy().to_string()],
            verified: true,
        };

        let error = remove_managed_skill_paths_under(
            &record,
            vec![outside.clone()],
            dir.path().join("codex").join("skills"),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("outside Codex skills directory"));
        assert!(outside.exists());
    }

    #[test]
    fn claude_desktop_org_plugin_uninstall_rejects_paths_outside_org_dir() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside-plugin");
        std::fs::create_dir_all(&outside).unwrap();
        let record = PluginHubInstallRecord {
            id: PONYTAIL_CLAUDE_DESKTOP_ORG_ID.to_string(),
            name: "Ponytail Organization Plugin for Claude Desktop".to_string(),
            install_kind: InstallKind::ClaudeDesktopOrgPlugin,
            installed_at: "1".to_string(),
            command: Vec::new(),
            source_url: PONYTAIL_REPOSITORY_URL.to_string(),
            backup_path: None,
            managed_paths: vec![outside.to_string_lossy().to_string()],
            verified: true,
        };

        let error = remove_claude_desktop_org_plugin(&record)
            .unwrap_err()
            .to_string();

        assert!(error.contains("outside org plugin directory"));
        assert!(outside.exists());
    }

    #[test]
    fn legacy_install_records_default_empty_managed_paths() {
        let records: Vec<PluginHubInstallRecord> = serde_json::from_str(
            r#"[{"id":"x","name":"X","installKind":"managed_skill_bundle","installedAt":"1","command":[],"sourceUrl":"https://example.test","backupPath":null}]"#,
        )
        .unwrap();

        assert!(records[0].managed_paths.is_empty());
    }

    #[test]
    fn installed_records_ignore_legacy_codex_plugin_false_installs() {
        let records = installed_records_from_text(
            r#"[{"id":"ponytail:codex-plugin","name":"Ponytail for Codex","installKind":"codex_plugin","installedAt":"1","command":["codex","plugin","marketplace","add","DietrichGebert/ponytail"],"sourceUrl":"https://github.com/DietrichGebert/ponytail","backupPath":null}]"#,
        )
        .unwrap();

        assert!(!records.contains_key("ponytail:codex-plugin"));
    }

    #[test]
    fn installed_records_keep_verified_codex_plugin_installs() {
        let records = installed_records_from_text(
            r#"[{"id":"ponytail:codex-plugin","name":"Ponytail for Codex","installKind":"codex_plugin","installedAt":"1","command":["codex","plugin","add","ponytail@ponytail","--json"],"sourceUrl":"https://github.com/DietrichGebert/ponytail","backupPath":null,"verified":true}]"#,
        )
        .unwrap();

        assert!(records.contains_key("ponytail:codex-plugin"));
    }

    #[test]
    fn codex_hook_hash_uses_normalized_identity() {
        let hash = command_hook_hash_for_value(
            "session_start",
            Some("startup|resume|clear|compact"),
            "if (Get-Command node -ErrorAction SilentlyContinue) { node \"$env:CLAUDE_PLUGIN_ROOT\\hooks\\ponytail-activate.js\" }",
            5,
            Some("Loading ponytail mode..."),
        )
        .unwrap();

        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), "sha256:".len() + 64);
    }

    #[test]
    fn codex_hook_trust_state_writer_preserves_existing_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "model = \"gpt-5\"\n").unwrap();
        let hook = CodexHookTrustEntry {
            key: "ponytail@ponytail:hooks/claude-codex-hooks.json:session_start:0:0".to_string(),
            event_name: "session_start".to_string(),
            matcher: None,
            command: "node hook.js".to_string(),
            status_message: None,
            current_hash: "sha256:abc".to_string(),
            trusted: false,
            source_path: "hooks/claude-codex-hooks.json".to_string(),
        };

        upsert_codex_hook_trust_state(&path, &[hook]).unwrap();
        let text = std::fs::read_to_string(path).unwrap();

        assert!(text.contains("model = \"gpt-5\""));
        assert!(text.contains("[hooks.state."));
        assert!(text.contains("trusted_hash = \"sha256:abc\""));
    }

    #[test]
    fn mcpb_manifest_contains_ponytail_server_entry() {
        let dir = tempfile::tempdir().unwrap();
        write_ponytail_mcpb_manifest(dir.path()).unwrap();
        let raw: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("manifest.json")).unwrap(),
        )
        .unwrap();

        assert_eq!(raw["name"], "ponytail");
        assert_eq!(raw["server"]["type"], "node");
        assert_eq!(raw["server"]["entry_point"], "server/index.js");
        assert_eq!(
            raw["server"]["mcp_config"]["args"][0],
            "${__dirname}/server/index.js"
        );
    }

    #[test]
    fn cli_failure_message_detects_missing_login_prompt() {
        let message = official_install_failure_message(
            &[
                "claude".to_string(),
                "plugin".to_string(),
                "install".to_string(),
                "demo".to_string(),
            ],
            "未找到有效的登录配置，请先登录\n请选择登录方式:",
            "",
        );

        assert!(message.contains("需要先登录 Claude Code CLI"));
        assert!(message.contains("claude"));
    }
}
