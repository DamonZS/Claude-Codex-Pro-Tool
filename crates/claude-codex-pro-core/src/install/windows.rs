use std::path::{Path, PathBuf};

use super::{
    InstallOptions, MANAGER_BINARY, MANAGER_NAME, SILENT_NAME, install_root_or_default,
    option_or_current_exe,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsEntrypointPlan {
    pub install_root: String,
    pub silent_shortcut: String,
    pub manager_shortcut: String,
    pub launcher_path: String,
    pub manager_path: String,
    pub icon_path: String,
    pub silent_icon_path: String,
    pub manager_icon_path: String,
    pub uninstall_key: String,
    pub remove_owned_data: bool,
}

pub fn build_windows_entrypoint_plan(options: &InstallOptions) -> WindowsEntrypointPlan {
    let install_root = install_root_or_default(options);
    let unified_path = options
        .manager_path
        .as_ref()
        .or(options.launcher_path.as_ref())
        .cloned()
        .unwrap_or_else(|| option_or_current_exe(&None, MANAGER_BINARY));
    let launcher_path = unified_path.clone();
    let manager_path = unified_path;
    let icon_path = default_icon_path();
    WindowsEntrypointPlan {
        silent_shortcut: install_root
            .join(format!("{SILENT_NAME}.lnk"))
            .to_string_lossy()
            .to_string(),
        manager_shortcut: install_root
            .join(format!("{MANAGER_NAME}.lnk"))
            .to_string_lossy()
            .to_string(),
        install_root: install_root.to_string_lossy().to_string(),
        launcher_path: launcher_path.to_string_lossy().to_string(),
        manager_path: manager_path.to_string_lossy().to_string(),
        icon_path: icon_path.to_string_lossy().to_string(),
        silent_icon_path: launcher_path.to_string_lossy().to_string(),
        manager_icon_path: manager_path.to_string_lossy().to_string(),
        uninstall_key: "ClaudeCodexPro".to_string(),
        remove_owned_data: options.remove_owned_data,
    }
}

fn default_icon_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .map(|path| path.join("claude-codex-pro.ico"))
        .unwrap_or_else(|| PathBuf::from("claude-codex-pro.ico"))
}

#[allow(dead_code)]
fn _entrypoint_names() -> (&'static str, &'static str) {
    (SILENT_NAME, MANAGER_NAME)
}
