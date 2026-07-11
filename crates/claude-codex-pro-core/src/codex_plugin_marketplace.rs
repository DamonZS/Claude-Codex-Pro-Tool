use std::collections::BTreeSet;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, Table};

pub const OPENAI_CURATED_MARKETPLACE: &str = "openai-curated";
pub const OPENAI_API_CURATED_MARKETPLACE: &str = "openai-api-curated";
pub const OPENAI_PLUGINS_ZIP_URL: &str =
    "https://codeload.github.com/openai/plugins/zip/refs/heads/main";
pub const HASHGRAPH_AWESOME_CODEX_MARKETPLACE: &str = "awesome-codex-plugins";
pub const HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SOURCE: &str =
    "https://github.com/hashgraph-online/awesome-codex-plugins.git";
pub const HASHGRAPH_AWESOME_CODEX_MARKETPLACE_REF: &str = "main";
const HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SPARSE_PATHS: [&str; 2] = [".agents/plugins", "plugins"];
pub const CODEX_SKILLS_ALTERNATIVE_MARKETPLACE: &str = "codex-skills-alternative";
pub const CODEX_SKILLS_ALTERNATIVE_MARKETPLACE_SOURCE: &str =
    "https://github.com/DKeken/codex-skills-alternative";
pub const CODEX_SKILLS_ALTERNATIVE_ZIP_URL: &str =
    "https://codeload.github.com/DKeken/codex-skills-alternative/zip/refs/heads/main";
const OPENAI_PLUGINS_DOWNLOAD_LIMIT_BYTES: usize = 128 * 1024 * 1024;
const OPENAI_PLUGINS_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(45);
const GIT_MARKETPLACE_SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(90);
const CODEX_SKILLS_ALTERNATIVE_DOWNLOAD_LIMIT_BYTES: usize = 32 * 1024 * 1024;
const CODEX_SKILLS_ALTERNATIVE_PLUGIN_NAME: &str = "codex-skills-alternative";
const OPENAI_CURATED_MARKETPLACE_ALIASES: [&str; 2] =
    [OPENAI_CURATED_MARKETPLACE, OPENAI_API_CURATED_MARKETPLACE];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexPluginMarketplaceStatus {
    pub codex_home: String,
    pub marketplace_root: Option<String>,
    pub config_registered: bool,
    pub needs_repair: bool,
    pub message: String,
    pub repositories: Vec<CodexPluginMarketplaceRepositoryStatus>,
    pub local_sources_ready: bool,
    pub runtime_confirmation: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexPluginMarketplaceRepositoryStatus {
    pub label: String,
    pub name: String,
    pub source_type: String,
    pub source: String,
    pub configured: bool,
}

pub fn status() -> CodexPluginMarketplaceStatus {
    status_from_home(&crate::relay_config::default_codex_home_dir())
}

pub fn status_from_home(home: &Path) -> CodexPluginMarketplaceStatus {
    let marketplace_root = local_openai_curated_marketplace_root(home).ok().flatten();
    let openai_config_registered = marketplace_root
        .as_deref()
        .map(|root| {
            OPENAI_CURATED_MARKETPLACE_ALIASES
                .iter()
                .all(|name| marketplace_config_points_to_root(home, name, root))
        })
        .unwrap_or(false);
    let hashgraph_config_registered = git_marketplace_config_registered(
        home,
        HASHGRAPH_AWESOME_CODEX_MARKETPLACE,
        HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SOURCE,
        HASHGRAPH_AWESOME_CODEX_MARKETPLACE_REF,
        &HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SPARSE_PATHS,
    );
    let product_design_marketplace_root =
        local_product_design_marketplace_root(home).ok().flatten();
    let product_design_config_registered = product_design_marketplace_root
        .as_deref()
        .map(|root| {
            marketplace_config_points_to_root(home, CODEX_SKILLS_ALTERNATIVE_MARKETPLACE, root)
        })
        .unwrap_or(false);
    let config_registered =
        openai_config_registered && hashgraph_config_registered && product_design_config_registered;
    let local_sources_ready = marketplace_root.is_some()
        && product_design_marketplace_root.is_some()
        && hashgraph_marketplace_local_snapshot_ready(home);
    let needs_repair = marketplace_root.is_none() || !config_registered || !local_sources_ready;
    let message = match (
        marketplace_root.is_some(),
        openai_config_registered,
        hashgraph_config_registered,
    ) {
        (true, true, true) => {
            "Codex 插件仓库配置已写入；应用是否已加载仍需重启 Codex 后确认。".to_string()
        }
        (true, false, true) => {
            "Codex OpenAI 插件仓库已下载，但尚未完整注册到 config.toml。".to_string()
        }
        (true, true, false) => "Codex OpenAI 插件仓库已注册，第三方插件仓库尚未注册。".to_string(),
        (true, false, false) => {
            "Codex OpenAI 插件仓库已下载，但官方与第三方仓库尚未完整注册。".to_string()
        }
        (false, _, true) => "Codex OpenAI 插件仓库尚未下载。".to_string(),
        (false, _, false) => "Codex OpenAI 与第三方插件仓库尚未完整配置。".to_string(),
    };
    let message = if !product_design_config_registered
        && openai_config_registered
        && hashgraph_config_registered
    {
        "Codex Product Design Skill 插件仓库尚未注册到 config.toml。".to_string()
    } else {
        message
    };
    let repositories = vec![
        CodexPluginMarketplaceRepositoryStatus {
            label: "OpenAI 官方仓库".to_string(),
            name: format!("{OPENAI_CURATED_MARKETPLACE} + {OPENAI_API_CURATED_MARKETPLACE}"),
            source_type: "local".to_string(),
            source: marketplace_root
                .as_ref()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| OPENAI_PLUGINS_ZIP_URL.to_string()),
            configured: openai_config_registered,
        },
        CodexPluginMarketplaceRepositoryStatus {
            label: "第三方插件仓库".to_string(),
            name: HASHGRAPH_AWESOME_CODEX_MARKETPLACE.to_string(),
            source_type: "git".to_string(),
            source: HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SOURCE.to_string(),
            configured: hashgraph_config_registered,
        },
        CodexPluginMarketplaceRepositoryStatus {
            label: "Product Design Skill 仓库".to_string(),
            name: CODEX_SKILLS_ALTERNATIVE_MARKETPLACE.to_string(),
            source_type: "local".to_string(),
            source: product_design_marketplace_root
                .as_ref()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| CODEX_SKILLS_ALTERNATIVE_MARKETPLACE_SOURCE.to_string()),
            configured: product_design_config_registered,
        },
    ];

    CodexPluginMarketplaceStatus {
        codex_home: home.to_string_lossy().to_string(),
        marketplace_root: marketplace_root.map(|path| path.to_string_lossy().to_string()),
        config_registered,
        needs_repair,
        message,
        repositories,
        local_sources_ready,
        runtime_confirmation: if config_registered && local_sources_ready {
            "待重启 Codex 确认应用可见".to_string()
        } else {
            "配置或本地来源尚未就绪".to_string()
        },
    }
}

fn hashgraph_marketplace_local_snapshot_ready(home: &Path) -> bool {
    [
        home.join("plugins")
            .join("cache")
            .join(HASHGRAPH_AWESOME_CODEX_MARKETPLACE),
        home.join(".tmp")
            .join("plugins")
            .join(HASHGRAPH_AWESOME_CODEX_MARKETPLACE),
    ]
    .iter()
    .any(|root| marketplace_snapshot_ready_at_root(root))
}

async fn ensure_git_marketplace_snapshot(
    home: &Path,
    name: &str,
    source: &str,
    reference: &str,
    sparse_paths: &[&str],
) -> anyhow::Result<bool> {
    validate_marketplace_cache_name(name)?;
    if source.trim().is_empty() {
        anyhow::bail!("marketplace {name} has no git source");
    }
    let destination = home.join("plugins").join("cache").join(name);
    if marketplace_snapshot_ready_at_root(&destination) {
        return Ok(false);
    }

    let temporary_root = home.join(".tmp");
    std::fs::create_dir_all(&temporary_root)?;
    let hooks = temporary_root.join("ccp-empty-git-hooks");
    std::fs::create_dir_all(&hooks)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let staging = temporary_root.join(format!("marketplace-{name}-{stamp}"));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }

    let mut clone = safe_git_command(&hooks);
    clone
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--single-branch")
        .arg("--branch")
        .arg(reference);
    if !sparse_paths.is_empty() {
        clone.arg("--filter=blob:none").arg("--sparse");
    }
    clone.arg("--").arg(source).arg(&staging);
    let status = tokio::time::timeout(GIT_MARKETPLACE_SNAPSHOT_TIMEOUT, clone.status())
        .await
        .map_err(|_| anyhow::anyhow!("git snapshot timed out for marketplace {name}"))?
        .with_context(|| format!("failed to start git for marketplace {name}"))?;
    if !status.success() {
        let _ = std::fs::remove_dir_all(&staging);
        anyhow::bail!("git snapshot failed for marketplace {name} ({status})");
    }

    if !sparse_paths.is_empty() {
        let mut paths = BTreeSet::new();
        paths.insert(".agents/plugins");
        for path in sparse_paths
            .iter()
            .map(|path| path.trim())
            .filter(|path| !path.is_empty())
        {
            paths.insert(path);
        }
        let mut sparse = safe_git_command(&hooks);
        sparse
            .arg("-C")
            .arg(&staging)
            .arg("sparse-checkout")
            .arg("set")
            .arg("--no-cone")
            .arg("--");
        for path in paths {
            sparse.arg(path);
        }
        let status = tokio::time::timeout(GIT_MARKETPLACE_SNAPSHOT_TIMEOUT, sparse.status())
            .await
            .map_err(|_| {
                anyhow::anyhow!("git sparse checkout timed out for marketplace {name}")
            })??;
        if !status.success() {
            let _ = std::fs::remove_dir_all(&staging);
            anyhow::bail!("git sparse checkout failed for marketplace {name} ({status})");
        }
    }

    if !marketplace_snapshot_ready_at_root(&staging) {
        let _ = std::fs::remove_dir_all(&staging);
        anyhow::bail!("marketplace {name} snapshot has no readable marketplace.json");
    }
    let git_metadata = staging.join(".git");
    if git_metadata.is_dir() {
        std::fs::remove_dir_all(&git_metadata)?;
    } else if git_metadata.exists() {
        std::fs::remove_file(&git_metadata)?;
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    replace_directory_with_backup_name(
        &staging,
        &destination,
        &format!("{name}.previous-claude-codex-pro"),
    )?;
    Ok(true)
}

fn safe_git_command(hooks: &Path) -> tokio::process::Command {
    let null_config = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut command = tokio::process::Command::new("git");
    #[cfg(windows)]
    command.creation_flags(crate::windows_create_no_window());
    command
        .arg("-c")
        .arg(format!("core.hooksPath={}", hooks.to_string_lossy()))
        .arg("-c")
        .arg("core.symlinks=false")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_config)
        .env("GIT_CONFIG_SYSTEM", null_config)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    command
}

fn validate_marketplace_cache_name(name: &str) -> anyhow::Result<()> {
    let valid = !name.is_empty()
        && name != "."
        && name != ".."
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        });
    if !valid {
        anyhow::bail!("invalid marketplace cache name")
    }
    Ok(())
}

fn marketplace_snapshot_ready_at_root(root: &Path) -> bool {
    let Some(manifest) = marketplace_manifest_in_root(root) else {
        return false;
    };
    let Ok(metadata) = std::fs::symlink_metadata(&manifest) else {
        return false;
    };
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return false;
    }
    std::fs::read_to_string(manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|marketplace| {
            marketplace
                .get("plugins")
                .and_then(serde_json::Value::as_array)
                .map(|plugins| !plugins.is_empty())
        })
        .unwrap_or(false)
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
    if local_product_design_marketplace_root(home)?.is_none() {
        initialize_product_design_marketplace_from_github(home).await?;
        initialized = true;
    }
    initialized |= ensure_git_marketplace_snapshot(
        home,
        HASHGRAPH_AWESOME_CODEX_MARKETPLACE,
        HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SOURCE,
        HASHGRAPH_AWESOME_CODEX_MARKETPLACE_REF,
        &HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SPARSE_PATHS,
    )
    .await?;
    let custom = crate::settings::SettingsStore::default()
        .load()
        .map(|settings| settings.codex_custom_marketplaces)
        .unwrap_or_default();
    for marketplace in &custom {
        if marketplace.source_type.trim().eq_ignore_ascii_case("git") {
            let reference = if marketplace.git_ref.trim().is_empty() {
                "main"
            } else {
                marketplace.git_ref.trim()
            };
            let sparse_paths = marketplace
                .sparse_paths
                .iter()
                .map(|path| path.trim())
                .filter(|path| !path.is_empty())
                .collect::<Vec<_>>();
            initialized |= ensure_git_marketplace_snapshot(
                home,
                marketplace.name.trim(),
                marketplace.source.trim(),
                reference,
                &sparse_paths,
            )
            .await
            .with_context(|| {
                format!(
                    "failed to prepare local snapshot for marketplace {}",
                    marketplace.name
                )
            })?;
        }
    }
    let mut configured = ensure_openai_curated_marketplace_config(home)?
        | ensure_hashgraph_awesome_codex_marketplace_config(home)?
        | ensure_product_design_skill_marketplace_config(home)?;
    // Also (re)apply any user-defined marketplaces. This is the missing write
    // path that made third-party repos never take effect: without it, a user's
    // custom marketplace was only ever persisted to settings and never landed in
    // config.toml. Failures are surfaced rather than silently swallowed.
    let (custom_changed, custom_errors) = apply_custom_marketplaces_from_home(home, &custom);
    if !custom_changed.is_empty() {
        configured = true;
    }
    if !custom_errors.is_empty() {
        anyhow::bail!("自定义插件仓库注册失败：{}", custom_errors.join("；"));
    }
    let next = status_from_home(home);
    Ok(CodexPluginMarketplaceRepair {
        codex_home: next.codex_home,
        marketplace_root: next.marketplace_root,
        initialized,
        configured,
        config_registered: next.config_registered,
        needs_repair: next.needs_repair,
        message: if next.needs_repair {
            "Codex 插件仓库修复后仍未通过状态检查。".to_string()
        } else if initialized || configured {
            "Codex OpenAI 与第三方插件仓库已注册。重启 Codex 后插件页会重新读取。".to_string()
        } else {
            "Codex 插件仓库已是最新状态。".to_string()
        },
    })
}

pub fn local_plugin_marketplaces() -> serde_json::Value {
    local_plugin_marketplaces_from_home(&crate::relay_config::default_codex_home_dir())
}

pub fn local_plugin_marketplaces_from_home(home: &Path) -> serde_json::Value {
    let installed_plugins = installed_plugins_from_config(home);
    let marketplaces = configured_local_marketplace_manifests(home)
        .into_iter()
        .filter_map(|(marketplace_name, path)| {
            let text = std::fs::read_to_string(&path).ok()?;
            let mut marketplace: serde_json::Value = serde_json::from_str(&text).ok()?;
            expand_local_plugin_marketplace(
                &mut marketplace,
                &path,
                home,
                &installed_plugins,
                &marketplace_name,
            );
            if let Some(object) = marketplace.as_object_mut() {
                object.insert(
                    "name".to_string(),
                    serde_json::Value::String(marketplace_name),
                );
                object.entry("path").or_insert_with(|| {
                    serde_json::Value::String(path.to_string_lossy().to_string())
                });
            }
            Some(marketplace)
        })
        .collect::<Vec<_>>();
    serde_json::Value::Array(marketplaces)
}

fn configured_local_marketplace_manifests(home: &Path) -> Vec<(String, PathBuf)> {
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();
    let openai_manifest = home
        .join(".tmp")
        .join("plugins")
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");
    if openai_manifest.is_file() {
        for name in OPENAI_CURATED_MARKETPLACE_ALIASES {
            let candidate = (name.to_string(), openai_manifest.clone());
            if seen.insert(candidate.clone()) {
                candidates.push(candidate);
            }
        }
    }

    let config = std::fs::read_to_string(home.join("config.toml")).unwrap_or_default();
    let document = config
        .trim_start_matches('\u{feff}')
        .parse::<DocumentMut>()
        .ok();
    let marketplaces = document
        .as_ref()
        .and_then(|document| document.get("marketplaces"))
        .and_then(Item::as_table);
    if let Some(marketplaces) = marketplaces {
        for (name, item) in marketplaces {
            let Some(table) = item.as_table() else {
                continue;
            };
            let source_type = table
                .get("source_type")
                .and_then(Item::as_str)
                .unwrap_or_default();
            let roots = match source_type {
                "local" => table
                    .get("source")
                    .and_then(Item::as_str)
                    .map(normalize_windows_extended_path)
                    .map(PathBuf::from)
                    .map(|path| {
                        if path.is_absolute() {
                            path
                        } else {
                            home.join(path)
                        }
                    })
                    .into_iter()
                    .collect::<Vec<_>>(),
                "git" => configured_git_marketplace_snapshot_roots(home, name),
                _ => Vec::new(),
            };
            for root in roots {
                if let Some(manifest) = marketplace_manifest_in_root(&root) {
                    let candidate = (name.to_string(), manifest);
                    if seen.insert(candidate.clone()) {
                        candidates.push(candidate);
                    }
                }
            }
        }
    }
    candidates
}

fn configured_git_marketplace_snapshot_roots(home: &Path, name: &str) -> Vec<PathBuf> {
    let mut roots = vec![
        home.join("plugins").join("cache").join(name),
        home.join("plugins")
            .join("cache")
            .join(format!("{name}-marketplace")),
        home.join(".tmp").join("plugins").join(name),
        home.join(".tmp").join("bundled-marketplaces").join(name),
    ];
    roots.sort();
    roots.dedup();
    roots
}

fn marketplace_manifest_in_root(root: &Path) -> Option<PathBuf> {
    if is_regular_file(root)
        && root.file_name().and_then(|name| name.to_str()) == Some("marketplace.json")
    {
        return Some(root.to_path_buf());
    }
    [
        root.join(".agents")
            .join("plugins")
            .join("marketplace.json"),
        root.join("marketplace.json"),
        root.join("plugins").join("marketplace.json"),
    ]
    .into_iter()
    .find(|path| is_regular_file(path))
}

fn is_regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_file() && !metadata.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn ensure_openai_curated_marketplace_config(home: &Path) -> anyhow::Result<bool> {
    let Some(marketplace_root) = local_openai_curated_marketplace_root(home)? else {
        return Ok(false);
    };
    let mut changed = false;
    for marketplace_name in OPENAI_CURATED_MARKETPLACE_ALIASES {
        changed |= ensure_marketplace_config(home, marketplace_name, &marketplace_root)?;
    }
    Ok(changed)
}

pub fn ensure_hashgraph_awesome_codex_marketplace_config(home: &Path) -> anyhow::Result<bool> {
    ensure_git_marketplace_config(
        home,
        HASHGRAPH_AWESOME_CODEX_MARKETPLACE,
        HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SOURCE,
        HASHGRAPH_AWESOME_CODEX_MARKETPLACE_REF,
        &HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SPARSE_PATHS,
    )
}

pub fn ensure_product_design_skill_marketplace_config(home: &Path) -> anyhow::Result<bool> {
    let Some(marketplace_root) = local_product_design_marketplace_root(home)? else {
        return Ok(false);
    };
    ensure_marketplace_config(
        home,
        CODEX_SKILLS_ALTERNATIVE_MARKETPLACE,
        &marketplace_root,
    )
}

/// Names reserved by the built-in marketplaces. A user repo may not reuse these
/// or it would silently overwrite / be overwritten by the built-in repair pass.
const RESERVED_MARKETPLACE_NAMES: [&str; 4] = [
    OPENAI_CURATED_MARKETPLACE,
    OPENAI_API_CURATED_MARKETPLACE,
    HASHGRAPH_AWESOME_CODEX_MARKETPLACE,
    CODEX_SKILLS_ALTERNATIVE_MARKETPLACE,
];

/// Write one user-defined marketplace into `config.toml`. This is the write
/// channel that previously did not exist: the built-in `ensure_*` helpers were
/// private and only ever wrote the three hard-coded repos.
pub fn ensure_custom_marketplace_config(
    home: &Path,
    marketplace: &crate::settings::CodexCustomMarketplace,
) -> anyhow::Result<bool> {
    let name = marketplace.name.trim();
    if name.is_empty() {
        anyhow::bail!("自定义插件仓库名称不能为空");
    }
    if RESERVED_MARKETPLACE_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(name))
    {
        anyhow::bail!("插件仓库名称 {name} 与内置仓库冲突，请改用其他名称");
    }
    let source = marketplace.source.trim();
    if source.is_empty() {
        anyhow::bail!("自定义插件仓库 {name} 的来源地址不能为空");
    }
    match marketplace.source_type.trim().to_ascii_lowercase().as_str() {
        "git" => {
            let reference = if marketplace.git_ref.trim().is_empty() {
                "main"
            } else {
                marketplace.git_ref.trim()
            };
            let sparse_paths = marketplace
                .sparse_paths
                .iter()
                .map(|path| path.trim())
                .filter(|path| !path.is_empty())
                .collect::<Vec<_>>();
            ensure_git_marketplace_config(home, name, source, reference, &sparse_paths)
        }
        "local" => ensure_marketplace_config(home, name, Path::new(source)),
        other => anyhow::bail!("不支持的插件仓库来源类型：{other}（仅支持 git 或 local）"),
    }
}

/// Apply every user-defined marketplace from settings to `config.toml`. Returns
/// the names that were newly written/changed and any per-repo errors so the
/// caller can surface them instead of silently swallowing failures.
pub fn apply_custom_marketplaces_from_home(
    home: &Path,
    marketplaces: &[crate::settings::CodexCustomMarketplace],
) -> (Vec<String>, Vec<String>) {
    let mut changed = Vec::new();
    let mut errors = Vec::new();
    for marketplace in marketplaces {
        match ensure_custom_marketplace_config(home, marketplace) {
            Ok(true) => changed.push(marketplace.name.trim().to_string()),
            Ok(false) => {}
            Err(error) => errors.push(format!("{}: {error}", marketplace.name.trim())),
        }
    }
    (changed, errors)
}

/// Drop a `[marketplaces.<name>]` section from `config.toml`. Refuses to touch
/// the built-in repos so a stray remove can never break the managed set.
/// Returns `true` when a section was actually removed.
pub fn remove_marketplace_config(home: &Path, name: &str) -> anyhow::Result<bool> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("插件仓库名称不能为空");
    }
    if RESERVED_MARKETPLACE_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(name))
    {
        anyhow::bail!("不能移除内置插件仓库 {name}");
    }
    let config_path = home.join("config.toml");
    let existing = match std::fs::read(&config_path) {
        Ok(bytes) => String::from_utf8(bytes)
            .with_context(|| format!("failed to read UTF-8 {}", config_path.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", config_path.display()));
        }
    };
    let without_bom = existing.trim_start_matches('\u{feff}');
    let mut doc = parse_toml_document(without_bom)?;
    let Some(marketplaces) = doc.get_mut("marketplaces").and_then(Item::as_table_mut) else {
        return Ok(false);
    };
    if marketplaces.remove(name).is_none() {
        return Ok(false);
    }
    if marketplaces.is_empty() {
        doc.as_table_mut().remove("marketplaces");
    }
    let updated = ensure_trailing_newline(doc.to_string());
    crate::settings::atomic_write(&config_path, updated.as_bytes())?;
    Ok(true)
}

fn local_openai_curated_marketplace_root(home: &Path) -> anyhow::Result<Option<PathBuf>> {
    local_openai_curated_marketplace_root_from_root(&home.join(".tmp").join("plugins"))
}

fn local_product_design_marketplace_root(home: &Path) -> anyhow::Result<Option<PathBuf>> {
    let root = home
        .join("plugins")
        .join("cache")
        .join("codex-skills-alternative-marketplace");
    if validate_product_design_marketplace_root(&root).is_ok() {
        Ok(Some(root))
    } else {
        Ok(None)
    }
}

async fn initialize_openai_curated_marketplace_from_github(home: &Path) -> anyhow::Result<()> {
    let bytes = download_openai_plugins_zip().await?;
    install_openai_plugins_zip(home, &bytes)
}

async fn initialize_product_design_marketplace_from_github(home: &Path) -> anyhow::Result<()> {
    let bytes = download_codex_skills_alternative_zip().await?;
    install_product_design_marketplace_zip(home, &bytes)
}

async fn download_openai_plugins_zip() -> anyhow::Result<Vec<u8>> {
    let client =
        crate::http_client::proxied_client(&format!("ClaudeCodexPro/{}", crate::version::VERSION))?;
    let bytes = client
        .get(OPENAI_PLUGINS_ZIP_URL)
        .header(reqwest::header::ACCEPT, "application/zip")
        .timeout(OPENAI_PLUGINS_DOWNLOAD_TIMEOUT)
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

async fn download_codex_skills_alternative_zip() -> anyhow::Result<Vec<u8>> {
    let client =
        crate::http_client::proxied_client(&format!("ClaudeCodexPro/{}", crate::version::VERSION))?;
    let bytes = client
        .get(CODEX_SKILLS_ALTERNATIVE_ZIP_URL)
        .header(reqwest::header::ACCEPT, "application/zip")
        .timeout(OPENAI_PLUGINS_DOWNLOAD_TIMEOUT)
        .send()
        .await
        .context("failed to download DKeken/codex-skills-alternative marketplace")?
        .error_for_status()
        .context("DKeken/codex-skills-alternative marketplace download returned an error status")?
        .bytes()
        .await
        .context("failed to read DKeken/codex-skills-alternative download body")?;
    if bytes.len() > CODEX_SKILLS_ALTERNATIVE_DOWNLOAD_LIMIT_BYTES {
        anyhow::bail!(
            "DKeken/codex-skills-alternative download is too large: {} bytes",
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

fn install_product_design_marketplace_zip(home: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let destination = home
        .join("plugins")
        .join("cache")
        .join("codex-skills-alternative-marketplace");
    let staging_parent = home.join(".tmp");
    std::fs::create_dir_all(&staging_parent)
        .with_context(|| format!("failed to create {}", staging_parent.display()))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let source_staging = staging_parent.join(format!("codex-skills-alternative-source-{stamp}"));
    let marketplace_staging =
        staging_parent.join(format!("codex-skills-alternative-marketplace-{stamp}"));
    for path in [&source_staging, &marketplace_staging] {
        if path.exists() {
            std::fs::remove_dir_all(path)
                .with_context(|| format!("failed to remove stale {}", path.display()))?;
        }
        std::fs::create_dir_all(path)
            .with_context(|| format!("failed to create {}", path.display()))?;
    }

    let result = extract_openai_plugins_zip(bytes, &source_staging)
        .and_then(|_| validate_codex_skills_alternative_source_root(&source_staging))
        .and_then(|_| {
            build_product_design_marketplace_snapshot(&source_staging, &marketplace_staging)
        })
        .and_then(|_| validate_product_design_marketplace_root(&marketplace_staging))
        .and_then(|_| {
            replace_directory_with_backup_name(
                &marketplace_staging,
                &destination,
                "codex-skills-alternative.previous-claude-codex-pro",
            )
        });
    let _ = std::fs::remove_dir_all(&source_staging);
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&marketplace_staging);
    }
    result
}

fn validate_codex_skills_alternative_source_root(root: &Path) -> anyhow::Result<()> {
    let manifest = root.join(".codex-plugin").join("plugin.json");
    if !manifest.is_file() {
        anyhow::bail!(
            "DKeken/codex-skills-alternative missing Codex plugin manifest: {}",
            manifest.display()
        );
    }
    let product_design = root.join("skills").join("product-design").join("SKILL.md");
    if !product_design.is_file() {
        anyhow::bail!(
            "DKeken/codex-skills-alternative missing product-design skill: {}",
            product_design.display()
        );
    }
    Ok(())
}

fn build_product_design_marketplace_snapshot(
    source: &Path,
    destination: &Path,
) -> anyhow::Result<()> {
    let plugin_root = destination
        .join("plugins")
        .join(CODEX_SKILLS_ALTERNATIVE_PLUGIN_NAME);
    std::fs::create_dir_all(destination.join(".agents").join("plugins"))
        .with_context(|| format!("failed to create {}", destination.display()))?;
    std::fs::create_dir_all(&plugin_root)
        .with_context(|| format!("failed to create {}", plugin_root.display()))?;
    copy_directory_recursive(
        &source.join(".codex-plugin"),
        &plugin_root.join(".codex-plugin"),
    )?;
    copy_directory_recursive(&source.join("skills"), &plugin_root.join("skills"))?;
    for file_name in ["README.md", "LICENSE"] {
        let source_file = source.join(file_name);
        if source_file.is_file() {
            std::fs::copy(&source_file, plugin_root.join(file_name)).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_file.display(),
                    plugin_root.join(file_name).display()
                )
            })?;
        }
    }
    let marketplace = serde_json::json!({
        "name": CODEX_SKILLS_ALTERNATIVE_MARKETPLACE,
        "interface": {
            "displayName": "Creative + Product Design Skills"
        },
        "plugins": [
            {
                "name": CODEX_SKILLS_ALTERNATIVE_PLUGIN_NAME,
                "source": {
                    "source": "local",
                    "path": "./plugins/codex-skills-alternative"
                },
                "policy": {
                    "installation": "AVAILABLE",
                    "authentication": "ON_INSTALL"
                },
                "category": "Design"
            }
        ]
    });
    let marketplace_path = destination
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");
    let text = serde_json::to_string_pretty(&marketplace)?;
    std::fs::write(&marketplace_path, ensure_trailing_newline(text))
        .with_context(|| format!("failed to write {}", marketplace_path.display()))?;
    Ok(())
}

fn validate_product_design_marketplace_root(root: &Path) -> anyhow::Result<()> {
    let marketplace_path = root
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");
    let text = std::fs::read_to_string(&marketplace_path)
        .with_context(|| format!("failed to read {}", marketplace_path.display()))?;
    let marketplace: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", marketplace_path.display()))?;
    if marketplace.get("name").and_then(serde_json::Value::as_str)
        != Some(CODEX_SKILLS_ALTERNATIVE_MARKETPLACE)
    {
        anyhow::bail!("Product Design Skill marketplace name mismatch");
    }
    let plugin = marketplace
        .get("plugins")
        .and_then(serde_json::Value::as_array)
        .and_then(|plugins| plugins.first())
        .ok_or_else(|| anyhow::anyhow!("Product Design Skill marketplace has no plugins"))?;
    let authentication = plugin
        .get("policy")
        .and_then(serde_json::Value::as_object)
        .and_then(|policy| policy.get("authentication"))
        .and_then(serde_json::Value::as_str);
    if authentication != Some("ON_INSTALL") {
        anyhow::bail!("Product Design Skill marketplace authentication policy must be ON_INSTALL");
    }
    let plugin_root = root
        .join("plugins")
        .join(CODEX_SKILLS_ALTERNATIVE_PLUGIN_NAME);
    let manifest = plugin_root.join(".codex-plugin").join("plugin.json");
    if !manifest.is_file() {
        anyhow::bail!(
            "Product Design Skill marketplace missing plugin manifest: {}",
            manifest.display()
        );
    }
    let product_design = plugin_root
        .join("skills")
        .join("product-design")
        .join("SKILL.md");
    if !product_design.is_file() {
        anyhow::bail!(
            "Product Design Skill marketplace missing product-design skill: {}",
            product_design.display()
        );
    }
    Ok(())
}

fn copy_directory_recursive(source: &Path, destination: &Path) -> anyhow::Result<()> {
    if !source.is_dir() {
        anyhow::bail!("source directory does not exist: {}", source.display());
    }
    if destination.exists() {
        std::fs::remove_dir_all(destination)
            .with_context(|| format!("failed to remove {}", destination.display()))?;
    }
    std::fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    for entry in
        std::fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read {}", source.display()))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to stat {}", source_path.display()))?;
        if file_type.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
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
            .ok_or_else(|| {
                anyhow::anyhow!("downloaded openai/plugins marketplace has an unnamed plugin")
            })?;
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
    replace_directory_with_backup_name(source, destination, "plugins.previous-claude-codex-pro")
}

fn replace_directory_with_backup_name(
    source: &Path,
    destination: &Path,
    backup_name: &str,
) -> anyhow::Result<()> {
    let backup = destination.with_file_name(backup_name);
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

fn ensure_git_marketplace_config(
    home: &Path,
    marketplace_name: &str,
    source: &str,
    reference: &str,
    sparse_paths: &[&str],
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
    marketplaces[marketplace_name]["source_type"] = toml_edit::value("git");
    marketplaces[marketplace_name]["source"] = toml_edit::value(source);
    marketplaces[marketplace_name]["ref"] = toml_edit::value(reference);
    let mut sparse = toml_edit::Array::default();
    for path in sparse_paths {
        sparse.push(*path);
    }
    marketplaces[marketplace_name]["sparse_paths"] = toml_edit::value(sparse);

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

fn git_marketplace_config_registered(
    home: &Path,
    marketplace_name: &str,
    source: &str,
    reference: &str,
    sparse_paths: &[&str],
) -> bool {
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
    let configured_source = table
        .get("source")
        .and_then(Item::as_str)
        .unwrap_or_default();
    let configured_ref = table.get("ref").and_then(Item::as_str).unwrap_or_default();
    let configured_sparse = table
        .get("sparse_paths")
        .and_then(Item::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(toml_edit::Value::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    source_type == "git"
        && configured_source == source
        && configured_ref == reference
        && configured_sparse == sparse_paths
}

fn expand_local_plugin_marketplace(
    marketplace: &mut serde_json::Value,
    marketplace_path: &Path,
    home: &Path,
    installed_plugins: &std::collections::BTreeSet<String>,
    marketplace_name_override: &str,
) {
    let original_marketplace_name = marketplace
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let marketplace_name = marketplace_name_override.to_string();
    let Some(plugins) = marketplace
        .get_mut("plugins")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    let marketplace_root = marketplace_root_for_manifest(marketplace_path)
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
        plugin_object.insert(
            "id".to_string(),
            serde_json::Value::String(format!("{plugin_name}@{marketplace_name}")),
        );
        plugin_object.insert(
            "pluginId".to_string(),
            serde_json::Value::String(format!("{plugin_name}@{marketplace_name}")),
        );
        plugin_object.insert(
            "marketplaceName".to_string(),
            serde_json::Value::String(marketplace_name.clone()),
        );
        plugin_object
            .entry("keywords".to_string())
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        plugin_object.insert(
            "installed".to_string(),
            serde_json::Value::Bool(plugin_installed_under_any_openai_curated_alias(
                installed_plugins,
                &plugin_name,
                &marketplace_name,
                &original_marketplace_name,
            )),
        );
    }
}

fn marketplace_root_for_manifest(marketplace_path: &Path) -> Option<PathBuf> {
    let parent = marketplace_path.parent()?;
    if parent.file_name().and_then(|name| name.to_str()) == Some("plugins") {
        let agents = parent.parent()?;
        if agents.file_name().and_then(|name| name.to_str()) == Some(".agents") {
            return agents.parent().map(Path::to_path_buf);
        }
    }
    Some(parent.to_path_buf())
}

fn plugin_installed_under_any_openai_curated_alias(
    installed_plugins: &BTreeSet<String>,
    plugin_name: &str,
    marketplace_name: &str,
    original_marketplace_name: &str,
) -> bool {
    let exact = installed_plugins.contains(&format!("{plugin_name}@{marketplace_name}"))
        || installed_plugins.contains(&format!("{plugin_name}@{original_marketplace_name}"));
    let is_openai_alias = OPENAI_CURATED_MARKETPLACE_ALIASES
        .iter()
        .any(|name| *name == marketplace_name || *name == original_marketplace_name);
    exact
        || (is_openai_alias
            && OPENAI_CURATED_MARKETPLACE_ALIASES
                .iter()
                .any(|name| installed_plugins.contains(&format!("{plugin_name}@{name}"))))
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
    plugin_marketplace_entry_path(root, trimmed).map(|path| path.to_string_lossy().to_string())
}

fn plugin_manifest(path: &Path) -> Option<serde_json::Map<String, serde_json::Value>> {
    if !is_regular_file(path) {
        return None;
    }
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

    fn write_product_design_marketplace(home: &Path) {
        let source = home.join("product-design-source");
        std::fs::create_dir_all(source.join(".codex-plugin")).unwrap();
        std::fs::create_dir_all(source.join("skills").join("product-design")).unwrap();
        std::fs::write(
            source.join(".codex-plugin").join("plugin.json"),
            r#"{"name":"codex-skills-alternative","version":"0.1.0","skills":"./skills/"}"#,
        )
        .unwrap();
        std::fs::write(
            source
                .join("skills")
                .join("product-design")
                .join("SKILL.md"),
            "---\nname: product-design\n---\n# Product Design\n",
        )
        .unwrap();
        std::fs::write(source.join("README.md"), "# codex-skills-alternative\n").unwrap();
        let destination = home
            .join("plugins")
            .join("cache")
            .join("codex-skills-alternative-marketplace");
        build_product_design_marketplace_snapshot(&source, &destination).unwrap();
        validate_product_design_marketplace_root(&destination).unwrap();
    }

    fn write_simple_marketplace(root: &Path, name: &str, plugin: &str) {
        let marketplace_path = root
            .join(".agents")
            .join("plugins")
            .join("marketplace.json");
        std::fs::create_dir_all(marketplace_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(root.join("plugins").join(plugin).join(".codex-plugin")).unwrap();
        std::fs::write(
            root.join("plugins")
                .join(plugin)
                .join(".codex-plugin")
                .join("plugin.json"),
            format!(r#"{{"name":"{plugin}","description":"{name} plugin"}}"#),
        )
        .unwrap();
        std::fs::write(
            marketplace_path,
            format!(
                r#"{{"name":"{name}","plugins":[{{"name":"{plugin}","source":{{"source":"local","path":"./plugins/{plugin}"}}}}]}}"#
            ),
        )
        .unwrap();
    }

    fn run_git(working_dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(working_dir)
            .status()
            .unwrap();
        assert!(status.success(), "git command failed: {args:?}");
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
    fn status_does_not_treat_an_empty_git_cache_directory_as_a_local_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        write_marketplace(temp.path());
        write_product_design_marketplace(temp.path());
        ensure_openai_curated_marketplace_config(temp.path()).unwrap();
        ensure_product_design_skill_marketplace_config(temp.path()).unwrap();
        ensure_hashgraph_awesome_codex_marketplace_config(temp.path()).unwrap();
        std::fs::create_dir_all(
            temp.path()
                .join("plugins")
                .join("cache")
                .join(HASHGRAPH_AWESOME_CODEX_MARKETPLACE),
        )
        .unwrap();

        let status = status_from_home(temp.path());

        assert!(status.config_registered);
        assert!(status.needs_repair);
        assert!(!status.local_sources_ready);
        assert_eq!(status.runtime_confirmation, "配置或本地来源尚未就绪");
    }

    #[test]
    fn ensure_config_registers_local_marketplace() {
        let temp = tempfile::tempdir().unwrap();
        write_marketplace(temp.path());
        write_product_design_marketplace(temp.path());

        let changed = ensure_openai_curated_marketplace_config(temp.path()).unwrap();
        let third_party_changed =
            ensure_hashgraph_awesome_codex_marketplace_config(temp.path()).unwrap();
        let product_design_changed =
            ensure_product_design_skill_marketplace_config(temp.path()).unwrap();

        assert!(changed);
        assert!(third_party_changed);
        assert!(product_design_changed);
        let status = status_from_home(temp.path());
        assert!(status.config_registered);
        assert!(status.needs_repair);
        assert!(!status.local_sources_ready);
        let config = std::fs::read_to_string(temp.path().join("config.toml")).unwrap();
        assert!(config.contains("[marketplaces.openai-curated]"));
        assert!(config.contains("[marketplaces.openai-api-curated]"));
        assert!(config.contains("[marketplaces.awesome-codex-plugins]"));
        assert!(config.contains("[marketplaces.codex-skills-alternative]"));
        assert!(config.contains("source_type = \"git\""));
        assert!(config.contains(
            "source = \"https://github.com/hashgraph-online/awesome-codex-plugins.git\""
        ));
        assert!(config.contains("source_type = \"local\""));
        assert!(config.contains("codex-skills-alternative-marketplace"));
        assert!(config.contains("ref = \"main\""));
        assert!(config.contains("sparse_paths = [\".agents/plugins\", \"plugins\"]"));
        assert_eq!(status.repositories.len(), 3);
        assert!(status.repositories.iter().any(|repository| {
            repository.name == HASHGRAPH_AWESOME_CODEX_MARKETPLACE && repository.configured
        }));
        assert!(status.repositories.iter().any(|repository| {
            repository.name == CODEX_SKILLS_ALTERNATIVE_MARKETPLACE && repository.configured
        }));
        assert!(!config.contains("[plugins."));
    }

    #[test]
    fn status_requires_both_openai_curated_aliases() {
        let temp = tempfile::tempdir().unwrap();
        write_marketplace(temp.path());
        let marketplace_root = local_openai_curated_marketplace_root(temp.path())
            .unwrap()
            .unwrap();
        ensure_marketplace_config(temp.path(), OPENAI_CURATED_MARKETPLACE, &marketplace_root)
            .unwrap();

        let status = status_from_home(temp.path());

        assert!(status.needs_repair);
        assert!(!status.config_registered);
    }

    #[test]
    fn status_requires_hashgraph_third_party_marketplace() {
        let temp = tempfile::tempdir().unwrap();
        write_marketplace(temp.path());
        ensure_openai_curated_marketplace_config(temp.path()).unwrap();

        let status = status_from_home(temp.path());

        assert!(status.needs_repair);
        assert!(!status.config_registered);
        assert!(status.repositories.iter().any(|repository| {
            repository.name == HASHGRAPH_AWESOME_CODEX_MARKETPLACE && !repository.configured
        }));
    }

    #[test]
    fn status_requires_product_design_skill_marketplace() {
        let temp = tempfile::tempdir().unwrap();
        write_marketplace(temp.path());
        ensure_openai_curated_marketplace_config(temp.path()).unwrap();
        ensure_hashgraph_awesome_codex_marketplace_config(temp.path()).unwrap();

        let status = status_from_home(temp.path());

        assert!(status.needs_repair);
        assert!(!status.config_registered);
        assert!(status.repositories.iter().any(|repository| {
            repository.name == CODEX_SKILLS_ALTERNATIVE_MARKETPLACE && !repository.configured
        }));
    }

    #[test]
    fn ensure_hashgraph_marketplace_config_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();

        let first = ensure_hashgraph_awesome_codex_marketplace_config(temp.path()).unwrap();
        let second = ensure_hashgraph_awesome_codex_marketplace_config(temp.path()).unwrap();

        assert!(first);
        assert!(!second);
        assert!(git_marketplace_config_registered(
            temp.path(),
            HASHGRAPH_AWESOME_CODEX_MARKETPLACE,
            HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SOURCE,
            HASHGRAPH_AWESOME_CODEX_MARKETPLACE_REF,
            &HASHGRAPH_AWESOME_CODEX_MARKETPLACE_SPARSE_PATHS,
        ));
        let config = std::fs::read_to_string(temp.path().join("config.toml")).unwrap();
        assert!(!config.contains("[plugins."));
    }

    #[test]
    fn product_design_skill_marketplace_snapshot_is_valid_and_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        write_product_design_marketplace(temp.path());

        let first = ensure_product_design_skill_marketplace_config(temp.path()).unwrap();
        let second = ensure_product_design_skill_marketplace_config(temp.path()).unwrap();

        assert!(first);
        assert!(!second);
        let root = local_product_design_marketplace_root(temp.path())
            .unwrap()
            .unwrap();
        assert!(
            root.join(".agents")
                .join("plugins")
                .join("marketplace.json")
                .is_file()
        );
        assert!(
            root.join("plugins")
                .join(CODEX_SKILLS_ALTERNATIVE_PLUGIN_NAME)
                .join(".codex-plugin")
                .join("plugin.json")
                .is_file()
        );
        assert!(
            root.join("plugins")
                .join(CODEX_SKILLS_ALTERNATIVE_PLUGIN_NAME)
                .join("skills")
                .join("product-design")
                .join("SKILL.md")
                .is_file()
        );
        let marketplace = std::fs::read_to_string(
            root.join(".agents")
                .join("plugins")
                .join("marketplace.json"),
        )
        .unwrap();
        assert!(marketplace.contains(r#""authentication": "ON_INSTALL""#));
        let config = std::fs::read_to_string(temp.path().join("config.toml")).unwrap();
        assert!(config.contains("[marketplaces.codex-skills-alternative]"));
        assert!(config.contains("source_type = \"local\""));
        assert!(config.contains("codex-skills-alternative-marketplace"));
        assert!(!config.contains("[plugins."));
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
        write_product_design_marketplace(temp.path());
        ensure_openai_curated_marketplace_config(temp.path()).unwrap();
        ensure_hashgraph_awesome_codex_marketplace_config(temp.path()).unwrap();
        ensure_product_design_skill_marketplace_config(temp.path()).unwrap();
        let status = status_from_home(temp.path());

        assert!(status.needs_repair);
        assert!(!status.local_sources_ready);
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
            writer
                .start_file("plugins-main/plugins/.keep", options)
                .unwrap();
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
        assert_eq!(marketplaces[0]["name"], OPENAI_CURATED_MARKETPLACE);
        assert_eq!(marketplaces[1]["name"], OPENAI_API_CURATED_MARKETPLACE);
        assert_eq!(
            marketplaces[1]["plugins"][0]["marketplaceName"],
            OPENAI_API_CURATED_MARKETPLACE
        );
        assert_eq!(
            marketplaces[1]["plugins"][0]["id"],
            format!("gmail@{OPENAI_API_CURATED_MARKETPLACE}")
        );
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
        std::fs::create_dir_all(
            root.join("plugins")
                .join("actual-dir")
                .join(".codex-plugin"),
        )
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

    #[test]
    fn local_marketplaces_mark_installed_for_either_official_alias() {
        let temp = tempfile::tempdir().unwrap();
        write_marketplace(temp.path());
        std::fs::write(
            temp.path().join("config.toml"),
            r#"[plugins."gmail@openai-api-curated"]
enabled = true
"#,
        )
        .unwrap();

        let marketplaces = local_plugin_marketplaces_from_home(temp.path());

        assert_eq!(marketplaces[0]["plugins"][0]["installed"], true);
        assert_eq!(marketplaces[1]["plugins"][0]["installed"], true);
    }

    #[test]
    fn plugin_asset_paths_reject_parent_directory_escape() {
        let root = Path::new("marketplace/plugins/demo");

        assert!(absolutize_plugin_asset_path("../../secret.txt", root).is_none());
        assert_eq!(
            absolutize_plugin_asset_path("./assets/logo.png", root),
            Some(
                root.join("assets")
                    .join("logo.png")
                    .to_string_lossy()
                    .to_string()
            )
        );
    }

    #[test]
    fn local_marketplaces_include_configured_snapshots_and_skip_missing_sources() {
        let temp = tempfile::tempdir().unwrap();
        write_marketplace(temp.path());

        write_product_design_marketplace(temp.path());
        ensure_product_design_skill_marketplace_config(temp.path()).unwrap();

        let third_party_root = temp
            .path()
            .join("plugins")
            .join("cache")
            .join(HASHGRAPH_AWESOME_CODEX_MARKETPLACE);
        write_simple_marketplace(
            &third_party_root,
            HASHGRAPH_AWESOME_CODEX_MARKETPLACE,
            "third-party-demo",
        );
        ensure_hashgraph_awesome_codex_marketplace_config(temp.path()).unwrap();

        let custom_local_root = temp.path().join("custom-local-marketplace");
        write_simple_marketplace(&custom_local_root, "custom-local", "custom-local-demo");
        ensure_custom_marketplace_config(
            temp.path(),
            &crate::settings::CodexCustomMarketplace {
                name: "custom-local".to_string(),
                source_type: "local".to_string(),
                source: custom_local_root.to_string_lossy().to_string(),
                git_ref: String::new(),
                sparse_paths: Vec::new(),
            },
        )
        .unwrap();

        let custom_git_root = temp.path().join("plugins").join("cache").join("custom-git");
        write_simple_marketplace(&custom_git_root, "custom-git", "custom-git-demo");
        ensure_custom_marketplace_config(
            temp.path(),
            &crate::settings::CodexCustomMarketplace {
                name: "custom-git".to_string(),
                source_type: "git".to_string(),
                source: "https://example.invalid/custom-git.git".to_string(),
                git_ref: "main".to_string(),
                sparse_paths: Vec::new(),
            },
        )
        .unwrap();

        ensure_custom_marketplace_config(
            temp.path(),
            &crate::settings::CodexCustomMarketplace {
                name: "missing-local".to_string(),
                source_type: "local".to_string(),
                source: temp
                    .path()
                    .join("does-not-exist")
                    .to_string_lossy()
                    .to_string(),
                git_ref: String::new(),
                sparse_paths: Vec::new(),
            },
        )
        .unwrap();

        let marketplaces = local_plugin_marketplaces_from_home(temp.path());
        let names = marketplaces
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|marketplace| marketplace.get("name").and_then(serde_json::Value::as_str))
            .collect::<BTreeSet<_>>();

        assert!(names.contains(OPENAI_CURATED_MARKETPLACE));
        assert!(names.contains(OPENAI_API_CURATED_MARKETPLACE));
        assert!(names.contains(CODEX_SKILLS_ALTERNATIVE_MARKETPLACE));
        assert!(names.contains(HASHGRAPH_AWESOME_CODEX_MARKETPLACE));
        assert!(names.contains("custom-local"));
        assert!(names.contains("custom-git"));
        assert!(!names.contains("missing-local"));
    }

    #[tokio::test]
    async fn git_marketplace_snapshot_is_created_without_running_repository_scripts() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let home = temp.path().join("codex-home");
        std::fs::create_dir_all(&source).unwrap();
        write_simple_marketplace(&source, "local-git", "local-git-demo");
        run_git(&source, &["init"]);
        run_git(&source, &["config", "user.email", "test@example.invalid"]);
        run_git(&source, &["config", "user.name", "CCP Test"]);
        run_git(&source, &["add", "."]);
        run_git(&source, &["commit", "-m", "fixture"]);
        run_git(&source, &["branch", "-M", "main"]);

        let first = ensure_git_marketplace_snapshot(
            &home,
            "local-git",
            &source.to_string_lossy(),
            "main",
            &[],
        )
        .await
        .unwrap();
        let second = ensure_git_marketplace_snapshot(
            &home,
            "local-git",
            &source.to_string_lossy(),
            "main",
            &[],
        )
        .await
        .unwrap();
        let snapshot = home.join("plugins").join("cache").join("local-git");

        assert!(first);
        assert!(!second);
        assert!(
            snapshot
                .join(".agents")
                .join("plugins")
                .join("marketplace.json")
                .is_file()
        );
        assert!(!snapshot.join(".git").exists());
    }

    #[test]
    fn ensure_custom_git_marketplace_writes_config_and_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let marketplace = crate::settings::CodexCustomMarketplace {
            name: "my-team-plugins".to_string(),
            source_type: "git".to_string(),
            source: "https://github.com/acme/codex-plugins.git".to_string(),
            git_ref: "release".to_string(),
            sparse_paths: vec!["plugins".to_string()],
        };

        let first = ensure_custom_marketplace_config(temp.path(), &marketplace).unwrap();
        let second = ensure_custom_marketplace_config(temp.path(), &marketplace).unwrap();

        assert!(first, "first write should change config.toml");
        assert!(!second, "re-applying an unchanged marketplace is a no-op");
        let config = std::fs::read_to_string(temp.path().join("config.toml")).unwrap();
        assert!(config.contains("[marketplaces.my-team-plugins]"));
        assert!(config.contains("source = \"https://github.com/acme/codex-plugins.git\""));
        assert!(config.contains("ref = \"release\""));
        assert!(config.contains("sparse_paths = [\"plugins\"]"));
    }

    #[test]
    fn ensure_custom_local_marketplace_defaults_ref_and_writes_path() {
        let temp = tempfile::tempdir().unwrap();
        let marketplace = crate::settings::CodexCustomMarketplace {
            name: "local-repo".to_string(),
            source_type: "local".to_string(),
            source: temp.path().join("repo").to_string_lossy().to_string(),
            git_ref: String::new(),
            sparse_paths: Vec::new(),
        };

        assert!(ensure_custom_marketplace_config(temp.path(), &marketplace).unwrap());
        let config = std::fs::read_to_string(temp.path().join("config.toml")).unwrap();
        assert!(config.contains("[marketplaces.local-repo]"));
        assert!(config.contains("source_type = \"local\""));
        // Local sources must not gain a git ref / sparse_paths block.
        assert!(!config.contains("ref = "));
        assert!(!config.contains("sparse_paths"));
    }

    #[test]
    fn ensure_custom_marketplace_rejects_reserved_names_and_blanks() {
        let temp = tempfile::tempdir().unwrap();
        // A name colliding with a built-in repo must be refused so a user repo
        // can never silently overwrite the OpenAI / third-party entries.
        let reserved = crate::settings::CodexCustomMarketplace {
            name: OPENAI_CURATED_MARKETPLACE.to_string(),
            source_type: "git".to_string(),
            source: "https://example.test/x.git".to_string(),
            git_ref: String::new(),
            sparse_paths: Vec::new(),
        };
        assert!(ensure_custom_marketplace_config(temp.path(), &reserved).is_err());

        let blank_source = crate::settings::CodexCustomMarketplace {
            name: "empty".to_string(),
            source_type: "git".to_string(),
            source: "   ".to_string(),
            git_ref: String::new(),
            sparse_paths: Vec::new(),
        };
        assert!(ensure_custom_marketplace_config(temp.path(), &blank_source).is_err());
        // Neither rejected write should have created a config.toml section.
        let config = std::fs::read_to_string(temp.path().join("config.toml")).unwrap_or_default();
        assert!(!config.contains("[marketplaces."));
    }

    #[test]
    fn apply_custom_marketplaces_reports_changed_and_errors_separately() {
        let temp = tempfile::tempdir().unwrap();
        let marketplaces = vec![
            crate::settings::CodexCustomMarketplace {
                name: "good-repo".to_string(),
                source_type: "git".to_string(),
                source: "https://github.com/acme/good.git".to_string(),
                git_ref: "main".to_string(),
                sparse_paths: Vec::new(),
            },
            crate::settings::CodexCustomMarketplace {
                name: HASHGRAPH_AWESOME_CODEX_MARKETPLACE.to_string(),
                source_type: "git".to_string(),
                source: "https://github.com/acme/collision.git".to_string(),
                git_ref: "main".to_string(),
                sparse_paths: Vec::new(),
            },
        ];

        let (changed, errors) = apply_custom_marketplaces_from_home(temp.path(), &marketplaces);

        assert_eq!(changed, vec!["good-repo".to_string()]);
        assert_eq!(
            errors.len(),
            1,
            "the reserved-name collision must be reported"
        );
        assert!(errors[0].contains(HASHGRAPH_AWESOME_CODEX_MARKETPLACE));
    }
}
