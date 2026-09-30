use std::path::Path;

use super::{
    InstallOptions, MANAGER_BINARY, MANAGER_NAME, MacosAppBundle, SILENT_BINARY, SILENT_NAME,
    install_root_or_default, option_or_current_exe,
};

pub fn build_app_bundle(options: &InstallOptions, manager: bool) -> MacosAppBundle {
    let install_root = install_root_or_default(options);
    let display_name = if manager { MANAGER_NAME } else { SILENT_NAME };
    let executable_name = SILENT_BINARY;
    let binary = if manager {
        MANAGER_BINARY
    } else {
        SILENT_BINARY
    };
    let unified_path = options
        .manager_path
        .as_ref()
        .or(options.launcher_path.as_ref())
        .cloned();
    let target = option_or_current_exe(&unified_path, binary);
    let (target, binary_source, binary_target_name) =
        if is_bundle_executable_target(&target, executable_name) {
            let sidecar = target
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(binary);
            (sidecar, Some(target), Some(binary.to_string()))
        } else {
            (target, None, None)
        };
    MacosAppBundle {
        app_path: install_root.join(format!("{display_name}.app")),
        info_plist: info_plist(display_name, executable_name, ""),
        launch_script: format!("#!/bin/sh\nexec \"{}\"\n", target.to_string_lossy()),
        binary_source,
        binary_target_name,
    }
}

fn is_bundle_executable_target(target: &Path, executable_name: &str) -> bool {
    target.file_name().and_then(|name| name.to_str()) == Some(executable_name)
        && target
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            == Some("MacOS")
        && target
            .parent()
            .and_then(|parent| parent.parent())
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            == Some("Contents")
}

fn info_plist(display_name: &str, executable_name: &str, identifier_suffix: &str) -> String {
    let version = crate::version::VERSION;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>{display_name}</string>
  <key>CFBundleDisplayName</key>
  <string>{display_name}</string>
  <key>CFBundleIdentifier</key>
  <string>com.damonzs.claudecodexpro{identifier_suffix}</string>
  <key>CFBundleVersion</key>
  <string>{version}</string>
  <key>CFBundleShortVersionString</key>
  <string>{version}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleExecutable</key>
  <string>{executable_name}</string>
  <key>CFBundleIconFile</key>
  <string>claude-codex-pro.png</string>
  <key>LSUIElement</key>
  <false/>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
</dict>
</plist>"#
    )
}
