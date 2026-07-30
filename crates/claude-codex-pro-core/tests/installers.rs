use claude_codex_pro_core::install::{
    InstallOptions, MCP_BINARY, SILENT_BINARY, app_bundle_names, build_macos_app_bundle,
    build_windows_entrypoint_plan, companion_binary_path_from_exe, default_install_root_strategy,
    is_macos_app_translocation_path, macos_bundle_companion_path_from_exe, shortcut_names,
};

#[test]
fn windows_entrypoint_plan_uses_one_unified_entrypoint() {
    let options = InstallOptions {
        install_root: Some("C:/Users/A/Desktop".into()),
        launcher_path: Some("C:/Tools/claude-codex-pro.exe".into()),
        manager_path: Some("C:/Tools/claude-codex-pro-manager.exe".into()),
        remove_owned_data: false,
    };

    let plan = build_windows_entrypoint_plan(&options);

    assert!(plan.silent_shortcut.ends_with("Claude Codex Pro.lnk"));
    assert!(plan.manager_shortcut.ends_with("Claude Codex Pro.lnk"));
    assert_eq!(plan.launcher_path, "C:/Tools/claude-codex-pro-manager.exe");
    assert_eq!(plan.manager_path, "C:/Tools/claude-codex-pro-manager.exe");
    assert_eq!(
        plan.silent_icon_path,
        "C:/Tools/claude-codex-pro-manager.exe"
    );
    assert_eq!(
        plan.manager_icon_path,
        "C:/Tools/claude-codex-pro-manager.exe"
    );
    assert_eq!(plan.uninstall_key, "ClaudeCodexPro");
}

#[test]
fn windows_entrypoint_plan_can_request_owned_data_removal_without_shell_script() {
    let options = InstallOptions {
        install_root: Some("C:/Users/A/Desktop".into()),
        launcher_path: None,
        manager_path: None,
        remove_owned_data: true,
    };

    let plan = build_windows_entrypoint_plan(&options);

    assert!(plan.silent_shortcut.ends_with("Claude Codex Pro.lnk"));
    assert!(plan.manager_shortcut.ends_with("Claude Codex Pro.lnk"));
    assert!(plan.remove_owned_data);
}

#[test]
fn macos_bundle_metadata_uses_one_unified_app() {
    let options = InstallOptions {
        install_root: Some("/Applications".into()),
        launcher_path: Some("/opt/Claude Code Pro/claude-codex-pro".into()),
        manager_path: Some("/opt/Claude Code Pro/claude-codex-pro-manager".into()),
        remove_owned_data: false,
    };

    let silent = build_macos_app_bundle(&options, false);
    let manager = build_macos_app_bundle(&options, true);

    assert!(silent.app_path.ends_with("Claude Codex Pro.app"));
    assert!(manager.app_path.ends_with("Claude Codex Pro.app"));
    assert!(
        silent
            .info_plist
            .contains("<string>Claude Codex Pro</string>")
    );
    assert!(
        manager
            .info_plist
            .contains("<string>Claude Codex Pro</string>")
    );
    assert!(silent.launch_script.contains("claude-codex-pro-manager"));
    assert!(manager.launch_script.contains("claude-codex-pro-manager"));
}

#[test]
fn installer_exports_one_entrypoint_name_for_both_legacy_fields() {
    assert_eq!(
        shortcut_names(),
        ("Claude Codex Pro.lnk", "Claude Codex Pro.lnk")
    );
    assert_eq!(
        app_bundle_names(),
        ("Claude Codex Pro.app", "Claude Codex Pro.app")
    );
}

#[test]
fn companion_binary_path_resolves_runtime_inside_unified_bundle() {
    let manager_exe =
        std::path::Path::new("/Applications/Claude Codex Pro.app/Contents/MacOS/claude-codex-pro");

    let companion = companion_binary_path_from_exe(manager_exe, SILENT_BINARY);

    assert_eq!(
        companion,
        std::path::PathBuf::from(
            "/Applications/Claude Codex Pro.app/Contents/MacOS/claude-codex-pro"
        )
    );

    assert_eq!(
        macos_bundle_companion_path_from_exe(manager_exe, MCP_BINARY),
        Some(std::path::PathBuf::from(
            "/Applications/Claude Codex Pro.app/Contents/MacOS/claude-codex-pro-mcp"
        ))
    );
}

#[test]
fn macos_bundle_companion_rejects_app_translocation() {
    let manager_exe = std::path::Path::new(
        "/private/var/folders/zz/AppTranslocation/ABC/d/Claude Codex Pro.app/Contents/MacOS/claude-codex-pro",
    );

    assert!(is_macos_app_translocation_path(manager_exe));
    assert_eq!(
        macos_bundle_companion_path_from_exe(manager_exe, SILENT_BINARY),
        None
    );
}

#[test]
fn macos_bundle_does_not_wrap_the_bundle_executable_in_itself() {
    let options = InstallOptions {
        install_root: Some("/Applications".into()),
        launcher_path: Some(
            "/Applications/Claude Codex Pro.app/Contents/MacOS/claude-codex-pro".into(),
        ),
        manager_path: Some(
            "/Applications/Claude Codex Pro.app/Contents/MacOS/claude-codex-pro".into(),
        ),
        remove_owned_data: false,
    };

    let silent = build_macos_app_bundle(&options, false);
    let manager = build_macos_app_bundle(&options, true);

    assert_eq!(
        silent.binary_target_name.as_deref(),
        Some("claude-codex-pro")
    );
    assert_eq!(
        manager.binary_target_name.as_deref(),
        Some("claude-codex-pro")
    );
    assert_eq!(silent.binary_source, options.manager_path.clone());
    assert_eq!(manager.binary_source, options.manager_path);
}

#[test]
fn windows_default_install_root_uses_known_folder_before_userprofile_desktop() {
    let strategy = default_install_root_strategy();

    if cfg!(windows) {
        assert_eq!(strategy, "windows-known-folder");
    } else if cfg!(target_os = "macos") {
        assert_eq!(strategy, "macos-applications");
    } else {
        assert_eq!(strategy, "user-dirs-desktop");
    }
}
