use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const OFFICIAL_MARKETPLACE_URL: &str =
    "https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/.claude-plugin/marketplace.json";
pub const AWESOME_CLAUDE_CODE_CSV_URL: &str =
    "https://raw.githubusercontent.com/hesreallyhim/awesome-claude-code/main/THE_RESOURCES_TABLE.csv";
pub const GITHUB_MCP_REGISTRY_URL: &str = "https://github.com/mcp";
const OFFICIAL_MARKETPLACE_NAME: &str = "claude-plugins-official";

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
        InstallKind::ClaudePluginMarketplace => install_official_plugin(preview),
        InstallKind::McpServer => install_mcp_preview(preview),
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
        Err(error) => return Err(error).with_context(|| format!("读取插件中心安装记录失败：{}", path.display())),
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
                InstallKind::SkillBundle | InstallKind::McpServer => {
                    status_for(&format!("awesome:{id}"), installed, InstallStatus::NeedsReview)
                }
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
            message: "将调用 Claude Code CLI 安装官方插件；执行前可检查命令。".to_string(),
            item,
        },
        InstallKind::McpServer => PluginInstallPreview {
            command: Vec::new(),
            config_diff: mcp_config_preview(&item),
            can_install: item.source_id == "awesome" && item.homepage.contains("github.com"),
            action: "mcp_config_preview".to_string(),
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

fn install_official_plugin(preview: PluginInstallPreview) -> anyhow::Result<PluginInstallOutcome> {
    let command = official_install_command(&preview.item);
    ensure_claude_marketplace_added();
    let output = Command::new(&command[0])
        .args(&command[1..])
        .output()
        .with_context(|| "无法启动 claude CLI，请先安装 Claude Code CLI 或手动执行预览命令")?;
    let installed = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if installed {
        record_install(&preview.item, command.clone(), None)?;
    }
    Ok(PluginInstallOutcome {
        item: preview.item.clone(),
        preview,
        installed,
        message: if installed {
            "官方 Claude 插件已通过 claude CLI 安装。".to_string()
        } else {
            official_install_failure_message(&command, &stdout, &stderr)
        },
        stdout,
        stderr,
        backup_path: None,
    })
}

fn install_mcp_preview(preview: PluginInstallPreview) -> anyhow::Result<PluginInstallOutcome> {
    let backup_path = Some(write_plugin_hub_note(&preview)?);
    record_install(&preview.item, Vec::new(), backup_path.clone())?;
    Ok(PluginInstallOutcome {
        item: preview.item.clone(),
        preview,
        installed: true,
        message: "已保存 MCP 配置草案。社区 MCP 需要在工具与插件页确认 command/args 后启用。".to_string(),
        stdout: String::new(),
        stderr: String::new(),
        backup_path,
    })
}

fn ensure_claude_marketplace_added() {
    let _ = Command::new("claude")
        .args([
            "plugin",
            "marketplace",
            "add",
            "anthropics/claude-plugins-official",
        ])
        .output();
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

fn mcp_config_preview(item: &PluginCatalogItem) -> String {
    let id = safe_id(&item.name);
    format!(
        "[mcp_servers.{id}]\n# 来源：{}\n# 主页：{}\n# 社区条目需要确认实际启动命令后删除下面两行注释。\ncommand = \"npx\"\nargs = [\"-y\", \"<package-or-command>\"]\nenabled = false\n",
        item.source_label, item.homepage
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

fn save_installed_records(records: &BTreeMap<String, PluginHubInstallRecord>) -> anyhow::Result<()> {
    let path = installed_records_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let values = records.values().cloned().collect::<Vec<_>>();
    crate::settings::atomic_write(&path, serde_json::to_string_pretty(&values)?.as_bytes())
}

fn write_plugin_hub_note(preview: &PluginInstallPreview) -> anyhow::Result<String> {
    let dir = plugin_hub_dir().join("pending");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.toml", safe_id(&preview.item.name)));
    crate::settings::atomic_write(&path, preview.config_diff.as_bytes())?;
    Ok(path.to_string_lossy().to_string())
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
        let rows = parse_csv_records("ID,Name,Description\none,\"Two, too\",\"hello \"\"world\"\"\"\n");
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
            classify_awesome_item("x", "Project Scaffolding & MCP", "https://github.com/a/b", ""),
            InstallKind::McpServer
        );
        assert_eq!(
            classify_awesome_item("x", "Agent Skills", "https://github.com/a/b", ""),
            InstallKind::SkillBundle
        );
    }

    #[test]
    fn cli_failure_message_detects_missing_login_prompt() {
        let message = official_install_failure_message(
            &["claude".to_string(), "plugin".to_string(), "install".to_string(), "demo".to_string()],
            "未找到有效的登录配置，请先登录\n请选择登录方式:",
            "",
        );

        assert!(message.contains("需要先登录 Claude Code CLI"));
        assert!(message.contains("claude"));
    }
}
