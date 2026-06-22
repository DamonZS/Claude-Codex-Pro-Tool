use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const OFFICIAL_MARKETPLACE_URL: &str = "https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/.claude-plugin/marketplace.json";
pub const AWESOME_CLAUDE_CODE_CSV_URL: &str = "https://raw.githubusercontent.com/hesreallyhim/awesome-claude-code/main/THE_RESOURCES_TABLE.csv";
pub const GITHUB_MCP_REGISTRY_URL: &str = "https://github.com/mcp";
pub const CODEX_PLUGIN_REPOSITORY_URL: &str = "https://github.com/openai/plugins";
pub const CODEX_PLUGIN_DOCUMENTATION_URL: &str = "https://developers.openai.com/codex/plugins";
pub const PONYTAIL_REPOSITORY_URL: &str = "https://github.com/DietrichGebert/ponytail";
const OFFICIAL_MARKETPLACE_NAME: &str = "claude-plugins-official";
const PONYTAIL_MARKETPLACE: &str = "DietrichGebert/ponytail";
const PONYTAIL_PLUGIN_REF: &str = "ponytail@ponytail";

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
        InstallKind::ClaudeCodePlugin | InstallKind::CodexPlugin | InstallKind::CopilotPlugin => {
            install_cli_plugin(preview)
        }
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

pub fn uninstall_item(id: &str) -> anyhow::Result<Vec<PluginHubInstallRecord>> {
    let mut records = load_installed_records().unwrap_or_default();
    records.remove(id);
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
    let records = serde_json::from_str::<Vec<PluginHubInstallRecord>>(&text)
        .with_context(|| format!("解析插件中心安装记录失败：{}", path.display()))?;
    Ok(records
        .into_iter()
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
            let command = vec![
                "claude".to_string(),
                "plugin".to_string(),
                "install".to_string(),
                format!("{name}@{OFFICIAL_MARKETPLACE_NAME}"),
            ];
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
                config_preview: String::new(),
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
            config_preview: "codex plugin marketplace add DietrichGebert/ponytail\n\n然后打开 Codex，进入 /plugins 选择 Ponytail marketplace 安装，并在 /hooks 中审查和信任 hooks。".to_string(),
            risk: "Codex CLI 目前只保证添加 marketplace；具体插件安装与 hooks 信任仍在 Codex 交互界面完成，避免后台静默信任第三方 hooks。".to_string(),
            requirements: vec![
                "codex CLI".to_string(),
                "Node.js on PATH".to_string(),
                "Codex /plugins 手动确认".to_string(),
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
    } else if haystack.contains("plugin") {
        InstallKind::ClaudePluginMarketplace
    } else {
        InstallKind::ResourceLink
    }
}

fn preview_for_item(item: PluginCatalogItem) -> PluginInstallPreview {
    match item.install_kind {
        InstallKind::ClaudePluginMarketplace => PluginInstallPreview {
            command: official_install_command(&item),
            config_diff: String::new(),
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
        InstallKind::ClaudeCodePlugin | InstallKind::CodexPlugin | InstallKind::CopilotPlugin => {
            PluginInstallPreview {
                command: item.install_command.clone(),
                config_diff: item.config_preview.clone(),
                can_install: true,
                action: "external_cli_plugin".to_string(),
                message: "Run the previewed CLI install steps. If the target CLI is missing or not logged in, the error output is returned.".to_string(),
                item,
            }
        }
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

fn official_install_command(item: &PluginCatalogItem) -> Vec<String> {
    if !item.install_command.is_empty() {
        return item.install_command.clone();
    }
    vec![
        "claude".to_string(),
        "plugin".to_string(),
        "install".to_string(),
        format!("{}@{OFFICIAL_MARKETPLACE_NAME}", item.name),
    ]
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
        InstallKind::CodexPlugin => Ok(vec![ponytail_codex_install_command()]),
        InstallKind::CopilotPlugin => Ok(vec![
            ponytail_copilot_install_command(),
            vec![
                "copilot".to_string(),
                "plugin".to_string(),
                "install".to_string(),
                PONYTAIL_PLUGIN_REF.to_string(),
            ],
        ]),
        _ => anyhow::bail!("unsupported CLI plugin install kind"),
    }
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

fn install_cli_plugin(preview: PluginInstallPreview) -> anyhow::Result<PluginInstallOutcome> {
    let plan = cli_plugin_install_plan(preview.item.install_kind)?;
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut recorded_command = preview.command.clone();
    for command in plan {
        if recorded_command.is_empty() {
            recorded_command = command.clone();
        }
        let (out, err) = run_command(&command)?;
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

fn install_official_claude_plugin(
    preview: PluginInstallPreview,
) -> anyhow::Result<PluginInstallOutcome> {
    let command = official_install_command(&preview.item);
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
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        anyhow::bail!(
            "{}",
            official_install_failure_message(&command, &stdout, &stderr)
        );
    }
    record_install(&preview.item, command, None)?;
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
    let config_path = claude_desktop_config_path();
    let backup_path = backup_claude_desktop_config(&config_path)?;
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
    upsert_claude_desktop_mcp_server(&config_path, &server_name, server_config)?;
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
    record_install(&preview.item, Vec::new(), backup_path.clone())?;
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

fn claude_desktop_config_path() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Claude")
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
        assert_eq!(items[0].install_command[3], "demo@claude-plugins-official");
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

        assert_eq!(items.len(), 5);
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
            ids["ponytail:codex-skills"],
            InstallKind::ManagedSkillBundle
        );
        assert!(items.iter().all(|item| item.source_id == "ponytail"));
    }

    #[test]
    fn ponytail_codex_preview_adds_marketplace_without_trusting_hooks() {
        let item = ponytail_catalog_items(&BTreeMap::new())
            .into_iter()
            .find(|item| item.id == "ponytail:codex-plugin")
            .expect("ponytail codex plugin item");
        let preview = preview_for_item(item);

        assert!(preview.can_install);
        assert_eq!(preview.action, "external_cli_plugin");
        assert_eq!(
            preview.command,
            vec![
                "codex",
                "plugin",
                "marketplace",
                "add",
                "DietrichGebert/ponytail"
            ]
        );
        assert!(preview.config_diff.contains("/hooks"));
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
            install_command: vec![
                "claude".to_string(),
                "plugin".to_string(),
                "install".to_string(),
                "demo@claude-plugins-official".to_string(),
            ],
            config_preview: String::new(),
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
                "install",
                "demo@claude-plugins-official"
            ]
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
