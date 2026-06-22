#[cfg(windows)]
#[test]
fn manager_binary_uses_windows_gui_subsystem_in_debug_and_release() {
    let main_rs = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"))
        .expect("read manager main.rs");

    assert!(
        main_rs.contains("#![cfg_attr(windows, windows_subsystem = \"windows\")]"),
        "manager binary should not allocate a console window on Windows"
    );
}

#[test]
fn manager_release_binary_uses_embedded_frontend_assets() {
    let cargo_toml = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read manager Cargo.toml");

    assert!(
        cargo_toml.contains("custom-protocol"),
        "release manager binary should use Tauri custom protocol instead of devUrl localhost"
    );
}

#[test]
fn manager_uses_single_instance_guard_before_starting_tauri() {
    let lib_rs = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("read manager lib.rs");

    assert!(lib_rs.contains("acquire_single_instance_guard()"));
    assert!(lib_rs.contains("MANAGER_GUARD_PORT"));
    assert!(lib_rs.contains("manager.guard_conflict_parallel_fallback"));
    assert!(lib_rs.contains("CCP_MANAGER_ALLOW_PARALLEL"));
}

#[test]
fn launcher_binary_embeds_codex_icon_resource() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let launcher_build = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap()
        .join("claude-codex-pro-launcher/build.rs");
    let build_rs = std::fs::read_to_string(&launcher_build).expect("read launcher build.rs");

    assert!(build_rs.contains("WindowsResource"));
    assert!(build_rs.contains("icons/icon.ico"));
}

#[test]
fn manager_runs_as_invoker_while_installer_requests_administrator_privileges() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manager_build =
        std::fs::read_to_string(manifest_dir.join("build.rs")).expect("read manager build.rs");
    let windows_manifest = std::fs::read_to_string(manifest_dir.join("windows-app-manifest.xml"))
        .expect("read windows app manifest");
    let launcher_build = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap()
        .join("claude-codex-pro-launcher/build.rs");
    let launcher_build = std::fs::read_to_string(&launcher_build).expect("read launcher build.rs");
    let windows_installer = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .unwrap()
        .join("scripts/installer/windows/ClaudeCodexPro.nsi");
    let windows_installer =
        std::fs::read_to_string(&windows_installer).expect("read windows installer");

    assert!(manager_build.contains("windows-app-manifest.xml"));
    assert!(launcher_build.contains("windows-app-manifest.xml"));
    assert!(windows_manifest.contains("asInvoker"));
    assert!(!windows_manifest.contains("requireAdministrator"));
    assert!(windows_manifest.contains("Microsoft.Windows.Common-Controls"));
    assert!(windows_installer.contains("RequestExecutionLevel admin"));
}

#[test]
fn manager_launch_button_spawns_silent_launcher_binary() {
    let commands_rs =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands.rs"))
            .expect("read manager commands.rs");

    assert!(commands_rs.contains("SILENT_BINARY"));
    assert!(commands_rs.contains("std::process::Command::new"));
    assert!(!commands_rs.contains("launch_and_inject_with_hooks(options"));
}

#[test]
fn macos_packager_hides_silent_launcher_but_not_manager() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let packager = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .unwrap()
        .join("scripts/installer/macos/package-dmg.sh");
    let script = std::fs::read_to_string(&packager).expect("read macOS packager");

    assert!(script.contains("<key>LSUIElement</key>"));
    assert!(script.contains("ARCH=\"${2:-$(uname -m)}\""));
    assert!(script.contains("BINARY_DIR=\"${BINARY_DIR:-$ROOT/target/release}\""));
    assert!(script.contains("claude-codex-pro-${VERSION}-macos-${ARCH}.dmg"));
    assert!(!script.contains("claude-codex-pro-plus-${VERSION}-macos-${ARCH}.dmg"));
    assert!(script.contains(
        "create_app \"Claude Codex Pro\" \"ClaudeCodexPro\" \"$BINARY_DIR/claude-codex-pro\" \"com.damonzs.claudecodexpro\" \"true\""
    ));
    assert!(script.contains(
        "create_app \"Claude Codex Pro 管理工具\" \"ClaudeCodexProManager\" \"$BINARY_DIR/claude-codex-pro-manager\" \"com.damonzs.claudecodexpro.manager\" \"false\""
    ));
}

#[test]
fn github_release_workflow_builds_separate_macos_x64_and_arm64_dmgs() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .unwrap()
        .join(".github/workflows/release-assets.yml");
    let workflow = std::fs::read_to_string(&workflow).expect("read release assets workflow");

    assert!(workflow.contains("macos-15-intel"));
    assert!(workflow.contains("x86_64-apple-darwin"));
    assert!(workflow.contains("macos-14"));
    assert!(workflow.contains("aarch64-apple-darwin"));
    assert!(workflow.contains("working-directory: apps/claude-codex-pro-manager"));
    assert!(workflow.contains("package-dmg.sh \"$VERSION\" \"${{ matrix.arch }}\""));
    assert!(workflow.contains("target/${{ matrix.target }}/release"));
    assert!(workflow.contains("Copy-Item target/release/claude-codex-pro.exe"));
    assert!(workflow.contains("Copy-Item target/release/claude-codex-pro-manager.exe"));
}

#[test]
fn github_release_workflow_uploads_static_latest_json() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .unwrap()
        .join(".github/workflows/release-assets.yml");
    let workflow = std::fs::read_to_string(&workflow).expect("read release assets workflow");

    assert!(workflow.contains("latest-json:"));
    assert!(workflow.contains("latest.json"));
    assert!(workflow.contains("gh release upload \"$TAG\" latest.json --clobber"));
}

#[test]
fn github_auto_release_workflow_builds_installers_with_v0_tags() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .unwrap();
    let workflow =
        std::fs::read_to_string(repo_root.join(".github/workflows/auto-release-installers.yml"))
            .expect("read auto release workflow");
    let release_assets =
        std::fs::read_to_string(repo_root.join(".github/workflows/release-assets.yml"))
            .expect("read release assets workflow");
    let version_script =
        std::fs::read_to_string(repo_root.join("scripts/release/next-release-tag.js"))
            .expect("read release tag script");
    let version_rs =
        std::fs::read_to_string(repo_root.join("crates/claude-codex-pro-core/src/version.rs"))
            .expect("read version module");

    assert!(workflow.contains("branches: [main]"));
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("node scripts/release/next-release-tag.js"));
    assert!(workflow.contains("git tag -a \"$TAG\" -m \"Release $TAG\""));
    assert!(workflow.contains("gh release create \"$TAG\""));
    assert!(workflow.contains("CLAUDE_CODEX_PRO_RELEASE_VERSION"));
    assert!(workflow.contains("npm run check"));
    assert!(workflow.contains("cargo test --workspace"));
    assert!(workflow.contains("Copy-Item target/release/claude-codex-pro.exe"));
    assert!(workflow.contains("Copy-Item target/release/claude-codex-pro-manager.exe"));
    assert!(workflow.contains("macos-15-intel"));
    assert!(workflow.contains("x86_64-apple-darwin"));
    assert!(workflow.contains("macos-14"));
    assert!(workflow.contains("aarch64-apple-darwin"));
    assert!(workflow.contains("package-dmg.sh \"$VERSION\" \"${{ matrix.arch }}\""));
    assert!(workflow.contains("gh release upload \"$TAG\" dist/macos/*.dmg --clobber"));
    assert!(workflow.contains("gh release upload \"$TAG\" latest.json --clobber"));
    assert!(workflow.contains("gh api --method PATCH \"repos/$REPO/releases/$release_id\""));
    assert!(workflow.contains("-F draft=false"));
    assert!(workflow.contains("-f make_latest=true"));
    assert!(workflow.contains("version: tag"));

    assert!(release_assets.contains("auto-release-installers-managed"));
    assert!(release_assets.contains("if: ${{ !contains(github.event.release.body"));
    assert!(release_assets.contains("version: tag"));

    assert!(version_script.contains("RELEASE_TAG_PATTERN = /^[vV](\\d+)\\.(\\d{2})$/"));
    assert!(version_script.contains("assert.equal(nextReleaseTag([]), \"V0.01\")"));
    assert!(version_script.contains("assert.equal(nextReleaseTag([\"V0.99\"]), \"V1.00\")"));

    assert!(version_rs.contains("CLAUDE_CODEX_PRO_RELEASE_VERSION"));
    assert!(version_rs.contains("env!(\"CARGO_PKG_VERSION\")"));
}

#[test]
fn ops_console_exposes_separate_claude_codex_and_plugin_actions() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    assert!(app_tsx.contains("重启 Codex"));
    assert!(app_tsx.contains("启动 Claude"));
    assert!(app_tsx.contains("Claude 中文窗口"));
    assert!(app_tsx.contains("open_claude_chinese_window"));
    assert!(app_tsx.contains("goPromptOptimizer"));
    assert!(commands_rs.contains("pub async fn open_claude_chinese_window"));
    assert!(commands_rs.contains("pub async fn open_plugin_hub_window"));
    assert!(commands_rs.contains("pub async fn open_prompt_optimizer_window"));
    assert!(commands_rs.contains("tauri::WebviewUrl::External"));
    assert!(commands_rs.contains("https://claude.ai/new"));
    assert!(commands_rs.contains("https://prompt.always200.com"));
    assert!(commands_rs.contains("__CLAUDE_CODEX_PRO_INITIAL_ROUTE"));
    assert!(commands_rs.contains("window.eval(script)"));
    assert!(commands_rs.contains("find_running_codex_app_dir"));
}

#[test]
fn plugin_hub_is_first_class_ops_console_route() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    assert!(app_tsx.contains("id: \"tools\""));
    assert!(app_tsx.contains("label: \"工具与插件\""));
    assert!(app_tsx.contains("function PluginHubScreen"));
    assert!(app_tsx.contains("function ToolsAndPluginsScreen"));
    assert!(app_tsx.contains("claude-codex-pro-navigate"));
    assert!(commands_rs.contains("route_main_window_to_plugin_hub"));
    assert!(commands_rs.contains("main_window_route_script(\"tools\")"));
    assert!(app_tsx.contains("refresh_plugin_hub_catalog"));
    assert!(app_tsx.contains("preview_plugin_hub_install"));
    assert!(app_tsx.contains("install_plugin_hub_item"));
    assert!(app_tsx.contains("uninstall_plugin_hub_item"));
    assert!(app_tsx.contains("claude_desktop_mcp"));
    assert!(app_tsx.contains("Claude Desktop MCP"));
    assert!(app_tsx.contains("Claude Code 插件"));
    assert!(
        app_tsx.contains(
            "Claude 插件、Codex 插件仓库、MCP Registry 与 awesome-claude-code 社区资源。"
        )
    );
    assert!(styles.contains(".plugin-layout"));
    assert!(styles.contains(".plugin-list"));
    assert!(styles.contains(".preview-box"));
    assert!(styles.contains(".risk-box"));
    assert!(!commands_rs.contains("WebviewWindowBuilder::new(&app, \"plugin-hub\""));
    assert!(!commands_rs.contains("WebviewWindowBuilder::new(&handle, \"plugin-hub\""));
}

#[test]
fn tools_and_plugins_route_contains_plugin_catalog_and_session_repair_tools() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    assert!(!app_tsx.contains("id: \"context\""));
    assert!(!app_tsx.contains("id: \"pluginHub\""));
    assert!(!app_tsx.contains("id: \"maintenance\""));
    assert!(app_tsx.contains("function ToolsAndPluginsScreen"));
    assert!(app_tsx.contains("Codex 插件仓库"));
    assert!(app_tsx.contains("https://github.com/openai/plugins"));
    assert!(app_tsx.contains("Codex 会话管理"));
    assert!(app_tsx.contains("Claude 会话诊断"));
    assert!(app_tsx.contains("历史会话修复"));
    assert!(app_tsx.contains("list_local_sessions"));
    assert!(app_tsx.contains("delete_local_session"));
    assert!(app_tsx.contains("sync_providers_now"));
    assert!(commands_rs.contains("list_local_sessions"));
    assert!(commands_rs.contains("sync_providers_now"));
}

#[test]
fn prompt_optimizer_is_integrated_as_internal_launcher() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");
    let prompt_screen = app_tsx
        .split("function PromptOptimizerScreen")
        .nth(1)
        .and_then(|rest| rest.split("function ScriptsScreen").next())
        .expect("prompt optimizer screen source");

    assert!(app_tsx.contains("id: \"promptOptimizer\""));
    assert!(app_tsx.contains("function PromptOptimizerScreen"));
    assert!(app_tsx.contains("linshenkx/prompt-optimizer"));
    assert!(app_tsx.contains("http://localhost:8081/mcp"));
    assert!(app_tsx.contains("isPromptOptimizerStandaloneWindow"));
    assert!(app_tsx.contains("prompt-optimizer-window-shell"));
    assert!(app_tsx.contains("if (isPromptOptimizerStandaloneWindow)"));
    assert!(app_tsx.contains("goPromptOptimizer"));
    assert!(
        !app_tsx.contains("call<PromptOptimizerWindowResult>(\"open_prompt_optimizer_window\")")
    );
    assert!(app_tsx.contains("浏览器打开在线版"));
    assert!(!prompt_screen.contains("打开控制窗口"));
    assert!(!prompt_screen.contains("openPromptOptimizerWindow"));
    assert!(!prompt_screen.contains("GitHub"));
    assert!(!prompt_screen.contains("安全边界"));
    assert!(app_tsx.contains("routeDocumentTitle"));
    assert!(app_tsx.contains("return \"提示词优化器\""));
    assert!(styles.contains(".prompt-optimizer-hero"));
    assert!(styles.contains(".prompt-optimizer-window-shell"));
    assert!(styles.contains(".prompt-usecase-list"));
    assert!(commands_rs.contains("internal_launcher_external_browser"));
    assert!(commands_rs.contains("ops_console_initial_route_script(\"promptOptimizer\")"));
    assert!(commands_rs.contains("prompt_optimizer_window_background_task"));
    assert!(commands_rs.contains("tauri::WebviewUrl::App(\"/\".into())"));
    assert!(app_tsx.contains("__CLAUDE_CODEX_PRO_INITIAL_ROUTE"));
    assert!(!commands_rs.contains(
        "tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::External(url))"
    ));
    assert!(commands_rs.contains("PromptOptimizerWindowPayload"));
}

#[test]
fn manager_window_and_ops_console_layout_stay_usable() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");
    let lib_rs =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read manager lib.rs");
    let tauri_conf =
        std::fs::read_to_string(manifest_dir.join("tauri.conf.json")).expect("read tauri config");

    assert!(app_tsx.contains("ops-shell"));
    assert!(app_tsx.contains("ops-rail"));
    assert!(app_tsx.contains("ops-commandbar"));
    assert!(app_tsx.contains("relay-banner"));
    assert!(app_tsx.contains("ops-primary-command"));
    assert!(styles.contains(".ops-shell"));
    assert!(styles.contains("grid-template-columns: 78px minmax(0, 1fr)"));
    assert!(styles.contains("height: 100vh;"));
    assert!(styles.contains(".ops-workspace"));
    assert!(styles.contains("min-height: 0;"));
    assert!(styles.contains(".ops-screen"));
    assert!(styles.contains("overflow-y: auto;"));
    assert!(styles.contains("padding-bottom: 32px;"));
    assert!(styles.contains(".ops-commandbar"));
    assert!(styles.contains(".relay-banner"));
    assert!(styles.contains(".status-tile"));
    assert!(styles.contains(".toast-wrap"));
    assert!(app_tsx.contains("notifyIfNeedsAttention"));
    assert!(!app_tsx.contains("role=\"dialog\""));
    assert!(!app_tsx.contains("aria-modal"));
    assert!(!styles.contains("notice-backdrop"));
    assert!(!styles.contains("notice-card"));
    assert!(lib_rs.contains(".inner_size(1180.0, 820.0)"));
    assert!(lib_rs.contains(".min_inner_size(960.0, 720.0)"));
    assert!(tauri_conf.contains("\"width\": 1180"));
    assert!(tauri_conf.contains("\"height\": 820"));
    assert!(tauri_conf.contains("\"minWidth\": 960"));
    assert!(tauri_conf.contains("\"minHeight\": 720"));
}

#[test]
fn settings_and_tools_route_keep_full_ops_controls() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");

    assert!(app_tsx.contains("function ToolsAndPluginsScreen"));
    assert!(app_tsx.contains("function MaintenanceToolsPanel"));
    assert!(app_tsx.contains("label: \"工具与插件\""));
    assert!(app_tsx.contains("安装入口"));
    assert!(app_tsx.contains("修复入口"));
    assert!(app_tsx.contains("修复后端"));
    assert!(app_tsx.contains("Watcher 自动接管"));
    assert!(app_tsx.contains("启动 Claude"));
    assert!(app_tsx.contains("Claude 中文窗口"));
    assert!(app_tsx.contains("重启 Codex"));
    assert!(app_tsx.contains("load_watcher_state"));
    assert!(app_tsx.contains("install_entrypoints"));
    assert!(app_tsx.contains("uninstall_entrypoints"));
    assert!(app_tsx.contains("install_watcher"));
    assert!(app_tsx.contains("disable_watcher"));

    assert!(app_tsx.contains("function SettingsScreen"));
    assert!(app_tsx.contains("设置文件位置"));
    assert!(app_tsx.contains("Codex 增强矩阵"));
    assert!(app_tsx.contains("Claude 中文包装窗口"));
    assert!(app_tsx.contains("CLI Wrapper"));
    assert!(app_tsx.contains("Codex 启动参数"));
    assert!(app_tsx.contains("安全边界"));
    assert!(app_tsx.contains("reset_settings"));
    assert!(app_tsx.contains("reset_image_overlay_settings"));
    assert!(app_tsx.contains("saveSettings"));
    assert!(app_tsx.contains("ops-form-field"));
    assert!(app_tsx.contains("ToggleSwitch"));
    assert!(app_tsx.contains("ops-toggle-line"));
    assert!(app_tsx.contains("ops-textarea"));

    assert!(styles.contains(".ops-two-column"));
    assert!(styles.contains(".ops-wide-column"));
    assert!(styles.contains(".ops-setting-card"));
    assert!(styles.contains(".ops-status-list"));
    assert!(styles.contains(".ops-danger-zone"));
    assert!(styles.contains(".ops-form-field"));
    assert!(styles.contains(".toggle-switch"));
    assert!(styles.contains(".toggle-switch-thumb"));
    assert!(styles.contains(".ops-textarea"));
}

#[test]
fn vite_build_uses_relative_assets_for_tauri_custom_protocol() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let vite_config = manifest_dir.parent().unwrap().join("vite.config.ts");
    let vite_config = std::fs::read_to_string(&vite_config).expect("read manager vite config");
    let app_tsx = std::fs::read_to_string(manifest_dir.parent().unwrap().join("src/App.tsx"))
        .expect("read manager App.tsx");

    assert!(vite_config.contains("base: \"./\""));
    assert!(app_tsx.contains("OPS_THEME_STORAGE_KEY"));
    assert!(app_tsx.contains("claude-codex-pro-ops-theme"));
}
