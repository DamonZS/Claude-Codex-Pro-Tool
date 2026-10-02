use crate::plugin_hub::{
    backup_claude_desktop_config, claude_desktop_normal_config_paths,
    read_existing_json_object_or_empty, remove_claude_desktop_mcp_server,
    upsert_claude_desktop_mcp_server,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub const COMPUTER_USE_SERVER_NAME: &str = "claude-codex-pro-computer-use";
pub const COMPUTER_USE_MCP_ARG: &str = "--mcp-computer-use";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputerUseStatus {
    pub enabled: bool,
    pub supported: bool,
    pub platform: String,
    pub executable_path: String,
    pub config_paths: Vec<String>,
    pub registered_paths: Vec<String>,
}

fn server_config(executable: &Path) -> Value {
    json!({
        "command": executable.to_string_lossy(),
        "args": [COMPUTER_USE_MCP_ARG],
        "env": {}
    })
}

fn is_registered(path: &Path) -> bool {
    path.exists()
        && read_existing_json_object_or_empty(path)
            .ok()
            .and_then(|config| {
                config
                    .get("mcpServers")
                    .and_then(|servers| servers.get(COMPUTER_USE_SERVER_NAME))
                    .cloned()
            })
            .is_some()
}

/// Write the MCP entry into each config. `backup` is off only in tests.
pub fn register_computer_use_at(
    paths: &[PathBuf],
    executable: &Path,
    backup: bool,
) -> anyhow::Result<()> {
    // Validate every config first so a broken one aborts before any write.
    for path in paths {
        read_existing_json_object_or_empty(path)?;
    }
    for path in paths {
        if backup {
            backup_claude_desktop_config(path)?;
        }
        upsert_claude_desktop_mcp_server(
            path,
            COMPUTER_USE_SERVER_NAME,
            server_config(executable),
        )?;
    }
    Ok(())
}

pub fn unregister_computer_use_at(paths: &[PathBuf], backup: bool) -> anyhow::Result<()> {
    for path in paths.iter().filter(|path| is_registered(path)) {
        if backup {
            backup_claude_desktop_config(path)?;
        }
        remove_claude_desktop_mcp_server(path, COMPUTER_USE_SERVER_NAME)?;
    }
    Ok(())
}

pub fn computer_use_status_for_paths(
    enabled: bool,
    paths: &[PathBuf],
    executable: &Path,
) -> ComputerUseStatus {
    ComputerUseStatus {
        enabled,
        supported: super::platform::SUPPORTED,
        platform: std::env::consts::OS.to_string(),
        executable_path: executable.to_string_lossy().to_string(),
        config_paths: paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
        registered_paths: paths
            .iter()
            .filter(|path| is_registered(path))
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    }
}

fn settings_store() -> crate::settings::SettingsStore {
    crate::settings::SettingsStore::new(crate::paths::default_settings_path())
}

pub fn computer_use_status() -> anyhow::Result<ComputerUseStatus> {
    let enabled = settings_store().load()?.claude_desktop_computer_use_enabled;
    let executable = std::env::current_exe()?;
    Ok(computer_use_status_for_paths(
        enabled,
        &claude_desktop_normal_config_paths(),
        &executable,
    ))
}

/// Toggle the CCP setting and (un)register the MCP server in Claude Desktop.
pub fn set_computer_use_enabled(enabled: bool) -> anyhow::Result<ComputerUseStatus> {
    if enabled && !super::platform::SUPPORTED {
        anyhow::bail!("当前平台不支持 Computer Use（仅支持 Windows 与 macOS）");
    }
    let paths = claude_desktop_normal_config_paths();
    let executable = std::env::current_exe()?;
    if enabled {
        register_computer_use_at(&paths, &executable, true)?;
    } else {
        unregister_computer_use_at(&paths, true)?;
    }
    settings_store()
        .update_boolean_preserving_profiles("claudeDesktopComputerUseEnabled", enabled)?;
    Ok(computer_use_status_for_paths(enabled, &paths, &executable))
}
