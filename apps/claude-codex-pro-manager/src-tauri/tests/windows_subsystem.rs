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
fn claude_chinese_window_can_call_manager_backend_commands() {
    let capability =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/capabilities/default.json"))
            .expect("read default capability");

    assert!(capability.contains("\"main\""));
    assert!(capability.contains("\"claude-chinese\""));
    assert!(capability.contains("\"core:default\""));
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
    assert!(commands_rs.contains("resolve_silent_launcher_path()"));
    assert!(commands_rs.contains("target\").join(\"debug\").join(&launcher_name)"));
    assert!(commands_rs.contains("target\").join(\"release\").join(&launcher_name)"));
    assert!(commands_rs.contains("找不到静默启动器"));
    assert!(commands_rs.contains("std::process::Command::new"));
    assert!(!commands_rs.contains("launch_and_inject_with_hooks(options"));
}

#[test]
fn codex_launch_and_injected_status_do_not_auto_open_manager() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands.rs"))
            .expect("read manager commands.rs");
    let repo_root = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let launcher_main =
        std::fs::read_to_string(repo_root.join("apps/claude-codex-pro-launcher/src/main.rs"))
            .expect("read launcher main");
    let codex_inject = std::fs::read_to_string(repo_root.join("assets/inject/renderer-inject.js"))
        .expect("read renderer inject");

    let launch_command = commands_rs
        .split("fn spawn_silent_launcher(request: &LaunchRequest)")
        .nth(1)
        .and_then(|rest| rest.split("pub fn resolve_silent_launcher_path").next())
        .expect("spawn_silent_launcher source");
    assert!(launch_command.contains("std::process::Command::new(&launcher)"));
    assert!(!launch_command.contains("claude-codex-pro-manager"));
    assert!(!launch_command.contains("--show-update"));

    assert!(launcher_main.contains("async fn open_manager(&self)"));
    assert!(!codex_inject.contains("data-codex-open-manager"));
    assert!(!codex_inject.contains("function openManagerFromCodex"));
    assert!(codex_inject.contains("data-codex-memory-manager"));
    assert!(codex_inject.contains("void postJson(\"/manager/open\", {});"));
}

#[test]
fn watcher_install_uses_resolved_existing_launcher_path() {
    let commands_rs =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands.rs"))
            .expect("read manager commands.rs");
    let watcher_section = commands_rs
        .split("pub fn install_watcher()")
        .nth(1)
        .expect("install_watcher should exist")
        .split("pub fn uninstall_watcher()")
        .next()
        .expect("install_watcher section should end before uninstall_watcher");

    assert!(watcher_section.contains("resolve_silent_launcher_path()"));
    assert!(!watcher_section.contains("companion_binary_path("));
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
    assert!(workflow.contains("gh release edit \"$TAG\" --repo \"$REPO\" --draft=false --latest"));
    assert!(workflow.contains("cleanup-failed-draft:"));
    assert!(workflow.contains("if: ${{ failure() }}"));
    assert!(workflow.contains("--json databaseId,isDraft"));
    assert!(workflow.contains("data.isDraft ? \"true\" : \"false\""));
    assert!(workflow.contains("gh api --method DELETE \"repos/$REPO/releases/$release_id\""));
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

    assert!(app_tsx.contains("启动/重启Codex"));
    assert!(app_tsx.contains("启动/重启Claude"));
    assert!(app_tsx.contains("Claude 一键汉化"));
    assert!(app_tsx.contains("install_claude_zh_patch"));
    assert!(app_tsx.contains("onClick={() => void actions.installClaudeZhPatch()}"));
    assert!(!app_tsx.contains("onClick={() => void actions.openClaudeChinese()}"));
    assert!(!app_tsx.contains("包装 WebView"));
    assert!(app_tsx.contains("PromptOptimizerCard"));
    assert!(commands_rs.contains("pub async fn open_claude_chinese_window"));
    assert!(commands_rs.contains("pub async fn open_plugin_hub_window"));
    assert!(commands_rs.contains("pub async fn open_prompt_optimizer_window"));
    assert!(commands_rs.contains("tauri::WebviewUrl::External"));
    assert!(commands_rs.contains("https://claude.ai/new"));
    assert!(commands_rs.contains("https://prompt.always200.com"));
    assert!(commands_rs.contains("main_window_route_script(\"tools\")"));
    assert!(commands_rs.contains("claude-codex-pro-navigate"));
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
    assert!(app_tsx.contains("id: \"sessions\""));
    assert!(app_tsx.contains("label: \"会话管理\""));
    assert!(app_tsx.contains("function PluginHubScreen"));
    assert!(app_tsx.contains("function ToolsAndPluginsScreen"));
    assert!(app_tsx.contains("function SessionManagementScreen"));
    assert!(app_tsx.contains("claude-codex-pro-navigate"));
    assert!(commands_rs.contains("route_main_window_to_plugin_hub"));
    assert!(commands_rs.contains("main_window_route_script(\"tools\")"));
    assert!(app_tsx.contains("refresh_plugin_hub_catalog"));
    assert!(app_tsx.contains("preview_plugin_hub_install"));
    assert!(app_tsx.contains("install_plugin_hub_item"));
    assert!(app_tsx.contains("uninstall_plugin_hub_item"));
    assert!(app_tsx.contains("claude_desktop_mcp"));
    assert!(app_tsx.contains("ponytail"));
    assert!(app_tsx.contains("codex_plugin"));
    assert!(app_tsx.contains("managed_skill_bundle"));
    assert!(app_tsx.contains("Claude Desktop MCP"));
    assert!(app_tsx.contains("Claude Code 插件"));
    assert!(
        app_tsx.contains(
            "Claude 插件、Codex 插件仓库、MCP Registry 与 awesome-claude-code 社区资源。"
        )
    );
    assert!(styles.contains(".plugin-layout"));
    assert!(styles.contains(".ops-tools-columns"));
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
    let tools_section = app_tsx
        .split("function ToolsAndPluginsScreen")
        .nth(1)
        .and_then(|rest| rest.split("function SessionManagementScreen").next())
        .expect("tools screen source");

    assert!(tools_section.contains("Codex 插件仓库"));
    assert!(tools_section.contains("https://github.com/openai/plugins"));
    assert!(tools_section.contains("工具与插件配置"));
    assert!(tools_section.contains("ClaudeDesktopOrgPluginPanel"));
    assert!(tools_section.contains("PromptOptimizerCard"));
    assert!(!tools_section.contains("Codex 会话管理"));
    assert!(!tools_section.contains("Claude 会话诊断"));
    assert!(!tools_section.contains("历史会话修复"));
    assert!(!tools_section.contains("list_local_sessions"));
    assert!(!tools_section.contains("delete_local_session"));
    assert!(!tools_section.contains("sync_providers_now"));
    assert!(commands_rs.contains("list_local_sessions"));
    assert!(commands_rs.contains("sync_providers_now"));
}

#[test]
fn session_management_route_contains_history_memory_and_diagnostics() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");

    let session_section = app_tsx
        .split("function SessionManagementScreen")
        .nth(1)
        .and_then(|rest| rest.split("function PluginHubScreen").next())
        .expect("session screen source");

    assert!(session_section.contains("会话管理"));
    assert!(session_section.contains("历史会话修复"));
    assert!(session_section.contains("盘古记忆"));
    assert!(session_section.contains("Codex 会话管理"));
    assert!(session_section.contains("Claude 会话诊断"));
    assert!(session_section.contains("refreshLocalSessions"));
    assert!(session_section.contains("deleteLocalSession"));
    assert!(session_section.contains("repairHistorySessions"));
    assert!(session_section.contains("launchClaudeDesktop"));
    assert!(session_section.contains("installClaudeZhPatch"));
    assert!(!session_section.contains("openClaudeChinese"));
    assert!(styles.contains(".ops-two-column"));
}

#[test]
fn prompt_optimizer_is_integrated_as_tools_card_launcher() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    assert!(!app_tsx.contains("id: \"promptOptimizer\""));
    assert!(!app_tsx.contains("function PromptOptimizerScreen"));
    assert!(app_tsx.contains("function PromptOptimizerCard"));
    assert!(app_tsx.contains("linshenkx/prompt-optimizer"));
    assert!(!app_tsx.contains("http://localhost:8081/mcp"));
    assert!(!app_tsx.contains("isPromptOptimizerStandaloneWindow"));
    assert!(!app_tsx.contains("prompt-optimizer-window-shell"));
    assert!(app_tsx.contains("goPromptOptimizer"));
    assert!(
        !app_tsx.contains("call<PromptOptimizerWindowResult>(\"open_prompt_optimizer_window\")")
    );
    assert!(app_tsx.contains("提示词优化"));
    assert!(app_tsx.contains("PROMPT_OPTIMIZER_URL"));
    assert!(app_tsx.contains("normalizeRoute(window.__CLAUDE_CODEX_PRO_INITIAL_ROUTE)"));
    assert!(app_tsx.contains("routeDocumentTitle"));
    assert!(!app_tsx.contains("return \"提示词优化器\""));
    assert!(styles.contains(".prompt-optimizer-card"));
    assert!(styles.contains(".prompt-optimizer-card-button"));
    assert!(!styles.contains(".prompt-optimizer-hero"));
    assert!(!styles.contains(".prompt-optimizer-window-shell"));
    assert!(!styles.contains(".prompt-usecase-list"));
    assert!(commands_rs.contains("tools_card_external_browser"));
    assert!(!commands_rs.contains("ops_console_initial_route_script"));
    assert!(!commands_rs.contains("prompt_optimizer_window_background_task"));
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
    assert!(app_tsx.contains("id: \"supplier\""));
    assert!(app_tsx.contains("label: \"供应商\""));
    assert!(app_tsx.contains("function SupplierScreen"));
    assert!(app_tsx.contains("switch_relay_profile"));
    assert!(app_tsx.contains("preview_claude_desktop_provider"));
    assert!(app_tsx.contains("apply_claude_desktop_provider"));
    assert!(app_tsx.contains("restore_claude_desktop_provider_official"));
    assert!(app_tsx.contains("if (value === \"relay\") return \"supplier\""));
    assert!(lib_rs.contains("commands::preview_claude_desktop_provider"));
    assert!(lib_rs.contains("commands::apply_claude_desktop_provider"));
    assert!(lib_rs.contains("commands::restore_claude_desktop_provider_official"));
    assert!(!app_tsx.contains("const actions = useMemo("));
    assert!(app_tsx.contains("relay-banner"));
    assert!(app_tsx.contains("relay-banner-open"));
    assert!(app_tsx.contains("route !== \"overview\""));
    assert!(app_tsx.contains("<span>后端链接</span>"));
    assert!(app_tsx.contains("https://api.toporeduce.cn"));
    assert!(app_tsx.contains("打开"));
    assert!(app_tsx.contains("Codex 状态"));
    assert!(app_tsx.contains("Claude 状态"));
    assert!(app_tsx.contains("盘古记忆"));
    assert!(app_tsx.contains("盘古记忆总览"));
    assert!(app_tsx.contains("诊断与修复"));
    assert!(app_tsx.contains("function codexOverviewStatus"));
    assert!(app_tsx.contains("function claudeOverviewStatus"));
    assert!(app_tsx.contains("status-segment-list"));
    assert!(app_tsx.contains("status-segment"));
    assert!(app_tsx.contains("运行中"));
    assert!(app_tsx.contains("未运行"));
    assert!(app_tsx.contains("注入成功"));
    assert!(app_tsx.contains("前端在线"));
    assert!(app_tsx.contains("后端在线"));
    assert!(app_tsx.contains("汉化已注入"));
    assert!(app_tsx.contains("CDP 受阻"));
    assert!(app_tsx.contains("注入异常"));
    assert!(!app_tsx.contains("inject ok"));
    assert!(!app_tsx.contains("FE on"));
    assert!(!app_tsx.contains("BE on"));
    assert!(!app_tsx.contains("Codex 运行"));
    let overview_screen = app_tsx
        .split("function OverviewScreen")
        .nth(1)
        .and_then(|rest| rest.split("function SupplierScreen").next())
        .expect("overview screen source");
    assert!(!overview_screen.contains("Codex 诊断"));
    assert!(!overview_screen.contains("Claude 诊断"));
    assert!(!overview_screen.contains("installKind ?? \"unknown\""));
    assert!(!overview_screen.contains("cdpStatus ?? \"unknown\""));
    let memory_panel = overview_screen
        .split("title=\"盘古记忆总览\"")
        .nth(1)
        .and_then(|rest| rest.split("title=\"诊断与修复\"").next())
        .expect("memory overview panel source");
    assert!(!memory_panel.contains("Claude 一键开发模式"));
    assert!(!overview_screen.contains("插件中心"));
    assert!(!overview_screen.contains("提示词工坊"));
    assert!(!overview_screen.contains("PromptOptimizerCard"));
    let overview_matrix = overview_screen
        .split("<div className=\"ops-matrix\">")
        .nth(1)
        .and_then(|rest| rest.split("</div>").next())
        .expect("overview matrix source");
    assert!(!overview_matrix.contains("actions.installClaudeZhPatch()"));
    assert!(overview_matrix.contains("items={codexStatus.items}"));
    assert!(overview_matrix.contains("items={claudeStatus.items}"));
    assert!(overview_screen.contains("StatusActionTile"));
    assert!(overview_screen.contains("Claude 一键开发模式"));
    assert!(overview_screen.contains("已写入"));
    assert!(overview_screen.contains("actions.configureClaudeDesktopDevMode()"));
    assert!(app_tsx.contains("const [claudeDevModeBusy, setClaudeDevModeBusy] = useState(false);"));
    assert!(app_tsx.contains("setNotice({ title: \"Claude 一键开发模式\", message: \"正在写入 Claude Desktop 开发配置...\", status: \"running\" });"));
    assert!(app_tsx.contains("setNotice({ title: \"Claude 一键开发模式\", message: result.message || result.outcome.message, status: result.status });"));
    assert!(app_tsx.contains("ops-primary-command"));
    assert!(styles.contains(".ops-shell"));
    assert!(styles.contains("grid-template-columns: 92px minmax(0, 1fr)"));
    assert!(styles.contains("height: 100vh;"));
    assert!(styles.contains(".ops-workspace"));
    assert!(styles.contains("min-height: 0;"));
    assert!(styles.contains(".ops-screen"));
    assert!(styles.contains("overflow-y: auto;"));
    assert!(styles.contains("padding-bottom: 32px;"));
    assert!(styles.contains(".ops-commandbar"));
    assert!(styles.contains(".relay-banner"));
    assert!(styles.contains(".relay-banner-open"));
    assert!(styles.contains(".status-tile"));
    assert!(styles.contains(".status-segment-list"));
    assert!(styles.contains(".status-segment.ok"));
    assert!(styles.contains(".status-segment.warn"));
    assert!(styles.contains(".status-segment.muted"));
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
    assert!(tauri_conf.contains("cargo build --manifest-path ../../Cargo.toml -p claude-codex-pro-launcher --bin claude-codex-pro && npm run vite:dev"));
}

#[test]
fn supplier_editor_generates_config_from_editable_supplier_id() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");

    let supplier_config_helper = app_tsx
        .split("function buildSupplierConfigToml")
        .nth(1)
        .and_then(|rest| rest.split("function tomlString").next())
        .expect("supplier config helper source");

    assert!(
        supplier_config_helper
            .contains("const providerId = supplierIdFromName(profile.id || profile.name);")
    );
    assert!(supplier_config_helper.contains("`model_provider = ${tomlString(providerId)}`"));
    assert!(supplier_config_helper.contains("`[model_providers.${providerId}]`"));
    assert!(supplier_config_helper.contains("`name = ${tomlString(providerId)}`"));
    assert!(!supplier_config_helper.contains("model_provider = \"custom\""));
    assert!(!supplier_config_helper.contains("[model_providers.custom]"));
}

#[test]
fn initial_manager_load_is_route_scoped_instead_of_global_prefetch() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let app_tsx = app_tsx.replace("\r\n", "\n");

    assert!(app_tsx.contains("const refreshMemoryAssistStatus = async (silent = false) => {"));
    assert!(app_tsx.contains("options: { trackBusy?: boolean; notify?: boolean } = {}"));
    assert!(app_tsx.contains("const trackBusy = options.trackBusy !== false;"));
    assert!(app_tsx.contains("if (trackBusy) setBusyCount((count) => count + 1);"));
    assert!(app_tsx.contains("if (trackBusy) setBusyCount((count) => Math.max(0, count - 1));"));
    assert!(
        app_tsx.contains("load_overview\"), \"概览\", { trackBusy: !silent, notify: !silent }")
    );
    assert!(app_tsx.contains(
        "load_claude_desktop_status\"), \"Claude Desktop\", { trackBusy: !silent, notify: !silent }"
    ));
    assert!(app_tsx.contains(
        "load_memory_assist_status\"), \"盘古记忆\", { trackBusy: !silent, notify: !silent }"
    ));
    assert!(app_tsx.contains(
        "if (target === \"overview\") {\n      await Promise.all([refreshOverview(true), refreshClaudeLight(true)]);"
    ));
    assert!(app_tsx.contains("afterFirstPaint(() => {\n        void refreshMemoryAssistStatus(true);"));
    assert!(app_tsx.contains("afterFirstPaint(() => {\n        void refreshClaudeZhPatch(true);"));
    assert!(app_tsx.contains("useEffect(() => {\n    void refreshRoute(route);\n  }, [route]);"));
    assert!(!app_tsx.contains(
        "useEffect(() => {\n    void (async () => {\n      await Promise.all([\n        refreshOverview(true),\n        refreshClaude(true),\n        refreshSettings(true),\n        refreshPluginHub(true),"
    ));
}

#[test]
fn codex_restart_passes_detected_app_path_and_uses_non_claude_debug_port() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let commands_rs =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands.rs"))
            .expect("read manager commands.rs");

    assert!(app_tsx.contains("function codexLaunchRequestFromOverview"));
    assert!(app_tsx.contains("overview?.codex_app.path || overview?.latest_launch?.codex_app"));
    assert!(app_tsx.contains("restart_claude_codex_pro\", { request }"));
    assert!(app_tsx.contains("launch_claude_codex_pro\", { request }"));
    assert!(commands_rs.contains("fn normalize_launch_request"));
    assert!(commands_rs.contains("find_running_codex_app_dir()"));
    assert!(commands_rs.contains("current_codex_app_path_for_launch"));
    assert!(commands_rs.contains("fn default_debug_port() -> u16 {\n    9230\n}"));
    assert!(!commands_rs.contains("fn default_debug_port() -> u16 {\n    9229\n}"));
}

#[test]
fn silent_launcher_logs_fatal_startup_errors() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let launcher_main = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .unwrap()
        .join("apps/claude-codex-pro-launcher/src/main.rs");
    let launcher_main = std::fs::read_to_string(&launcher_main).expect("read launcher main.rs");

    assert!(launcher_main.contains("async fn run_launcher()"));
    assert!(launcher_main.contains("\"launcher.fatal\""));
    assert!(launcher_main.contains("\"error\": error.to_string()"));
}

#[test]
fn claude_restart_button_closes_existing_processes_before_launching() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let core_claude = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/claude-codex-pro-core/src/claude_desktop.rs");
    let core_claude = std::fs::read_to_string(&core_claude).expect("read core claude_desktop.rs");

    assert!(app_tsx.contains("\"open_claude_desktop\"), \"启动/重启Claude\""));
    assert!(core_claude.contains("let existing_process_ids = claude_process_ids();"));
    assert!(core_claude.contains("let is_restart = !existing_process_ids.is_empty();"));
    assert!(core_claude.contains("terminate_claude_processes(&existing_process_ids)"));
    assert!(core_claude.contains("wait_for_claude_process_exit("));
    assert!(core_claude.contains("Claude Desktop was closed and restart was requested"));
    assert!(
        core_claude.contains("action: if is_restart { \"restart\" } else { \"open\" }.to_string()")
    );
    assert!(!core_claude.contains("Claude Desktop is already running and was focused."));
}

#[test]
fn supplier_screen_exposes_real_provider_crud_and_switching() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let app_tsx = app_tsx.replace("\r\n", "\n");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");

    let supplier_screen = app_tsx
        .split("function SupplierScreen")
        .nth(1)
        .and_then(|rest| rest.split("function LegacySupplierScreen").next())
        .expect("supplier screen source");

    assert!(supplier_screen.contains("actions.saveSettings(next)"));
    assert!(supplier_screen.contains("actions.switchCodexRelayProfile"));
    assert!(supplier_screen.contains("switchCodexRelayProfile(savedProfile.id, saved.settings)"));
    assert!(supplier_screen.contains("actions.fetchRelayProfileModels"));
    assert!(supplier_screen.contains("const originalId = editingId;"));
    assert!(supplier_screen.contains("profile.id === originalId ? normalized : profile"));
    assert!(supplier_screen.contains("activeRelayId: nextActiveRelayId"));
    assert!(
        supplier_screen
            .contains("actions.showNotice({ title: \"供应商保存\", message: `正在保存供应商")
    );
    assert!(supplier_screen.contains("const savedProfile = saved.relayProfiles.find((profile) => profile.id === normalized.id) ?? normalized;"));
    assert!(supplier_screen.contains("const saveDraft = async (options: { stayInEditor?: boolean } = {}): Promise<SupplierSaveResult | null> => {"));
    assert!(supplier_screen.contains("setDraft(null);"));
    assert!(supplier_screen.contains("saveDraft({ stayInEditor: true })"));
    assert!(supplier_screen.contains("!normalized.name.trim() || (!aggregateDraft && !normalized.baseUrl.trim())"));
    assert!(!supplier_screen.contains(
        "!normalized.name.trim() || !normalized.baseUrl.trim() || !normalized.apiKey.trim()"
    ));
    assert!(supplier_screen.contains("API Key 可以后续补入"));
    assert!(supplier_screen.contains("请先补入 API Key"));
    assert!(app_tsx.contains(
        "const targetProfile = current.relayProfiles.find((profile) => profile.id === profileId);"
    ));
    assert!(app_tsx.contains("该供应商缺少 API Key。记录已可保存，请补入 Key 后再切换写入。"));
    assert!(!supplier_screen.contains("if (!requestedId || requestedId !== normalizedId)"));
    assert!(supplier_screen.contains("const idWasNormalized = requestedId !== normalizedId;"));
    assert!(supplier_screen.contains("actions.showNotice({ title: \"供应商保存\", message: `供应商 ID 已自动整理为「${savedProfile.id}」。`, status: \"ok\" });"));
    assert!(supplier_screen.contains(
        "const updateDraftId = (value: string, options: { normalize?: boolean } = {}) => {"
    ));
    assert!(
        supplier_screen
            .contains("const next = withSupplierGeneratedFiles({ ...current, id: nextId });")
    );
    assert!(supplier_screen.contains(
        "onBlur={(event) => updateDraftId(event.currentTarget.value || draft.name, { normalize: true })}"
    ));
    assert!(supplier_screen.contains(
        "onChange={(event) => updateDraftId(event.currentTarget.value)} value={draft.id}"
    ));
    assert!(!supplier_screen.contains("onChange={(event) => setDraft((current) => current ? { ...current, id: event.currentTarget.value } : current)} value={draft.id}"));
    assert!(!supplier_screen.contains("onChange={(event) => updateDraft({ id: supplierIdFromName(event.currentTarget.value) })} value={generated.id}"));
    assert!(!supplier_screen.contains("input disabled={!isNewDraft} onChange={(event) => updateDraft({ id: supplierIdFromName(event.currentTarget.value) })}"));
    assert!(supplier_screen.contains("createSupplierProfile(appSettings)"));
    assert!(supplier_screen.contains("withSupplierGeneratedFiles"));
    assert!(supplier_screen.contains("SUPPLIER_PRESETS.map"));
    assert!(supplier_screen.contains("添加供应商"));
    assert!(supplier_screen.contains("编辑"));
    assert!(supplier_screen.contains("删除供应商"));
    assert!(app_tsx.contains("function buildSupplierConfigToml"));
    assert!(!supplier_screen.contains("model_provider = \"custom\""));
    assert!(!supplier_screen.contains("[model_providers.custom]"));
    assert!(app_tsx.contains("OPENAI_API_KEY"));
    assert!(app_tsx.contains("fetch_relay_profile_models"));
    assert!(app_tsx.contains("const modelList = profile.modelList ?? \"\";"));
    assert!(app_tsx.contains("const apiKey = profile.apiKey ?? \"\";"));
    assert!(app_tsx.contains("configContents: profile.configContents ?? \"\""));
    assert!(app_tsx.contains("authContents: profile.authContents ?? \"\""));
    assert!(styles.contains(".supplier-card"));
    assert!(styles.contains(".supplier-editor-layout"));
    assert!(styles.contains(".supplier-preset-strip"));
}

#[test]
fn supplier_screen_matches_ccswitch_style_layout_and_drag_sorting() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let app_tsx = app_tsx.replace("\r\n", "\n");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");
    let commands_rs =
        std::fs::read_to_string(manifest_dir.join("src/commands.rs")).expect("read commands.rs");
    let lib_rs = std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs");

    let supplier_screen = app_tsx
        .split("function SupplierScreen")
        .nth(1)
        .and_then(|rest| rest.split("function LegacySupplierScreen").next())
        .expect("supplier screen source");

    assert!(supplier_screen.contains("供应商配置"));
    assert!(supplier_screen.contains("管理 API 供应商、协议、Key 与配置文件"));
    assert!(supplier_screen.contains("供应商列表"));
    assert!(supplier_screen.contains("检测到 OPENAI 环境变量"));
    assert!(supplier_screen.contains("启用供应商配置切换"));
    assert!(supplier_screen.contains("添加供应商"));
    assert!(supplier_screen.contains("添加聚合供应商"));
    assert!(supplier_screen.contains("从第三方导入"));
    assert!(supplier_screen.contains("ccswitch"));
    assert!(supplier_screen.contains("发现"));
    assert!(supplier_screen.contains("刷新列表"));
    assert!(supplier_screen.contains("onDragStart"));
    assert!(supplier_screen.contains("onDragOver"));
    assert!(supplier_screen.contains("onDrop"));
    assert!(supplier_screen.contains("const [supplierOrderIds, setSupplierOrderIds] = useState<string[]>([]);"));
    assert!(supplier_screen.contains("useEffect(() => {\n    setSupplierOrderIds(profiles.map((profile) => profile.id));\n  }, [profileIdsKey]);"));
    assert!(supplier_screen.contains("const supplierOrderFromIds = (ids: string[]) => {"));
    assert!(supplier_screen.contains("const reorderSupplierIds = (sourceId: string, targetId: string, ids = supplierOrderIds) => {"));
    assert!(supplier_screen.contains("const previewSupplierOrder = (sourceId: string, targetId: string) => {"));
    assert!(supplier_screen.contains("setSupplierOrderIds((current) => reorderSupplierIds(sourceId, targetId, current) ?? current);"));
    assert!(supplier_screen.contains("saveSupplierOrder"));
    assert!(supplier_screen.contains("saveSupplierSettings({ ...appSettings, relayProfiles: reordered })"));
    assert!(supplier_screen.contains("setSupplierOrderIds(saved.relayProfiles.map((profile) => profile.id));"));
    assert!(supplier_screen.contains("setSupplierOrderIds(previousIds);"));
    assert!(supplier_screen.contains("供应商顺序保存失败，已恢复原顺序。"));
    assert!(supplier_screen.contains("supplierOrderFromIds(supplierOrderIds).map((profile) => {"));
    assert!(supplier_screen.contains("previewSupplierOrder(draggedId, profile.id);"));

    assert!(supplier_screen.contains("从预设模板创建"));
    assert!(supplier_screen.contains("接入模式"));
    assert!(supplier_screen.contains("Codex 目标"));
    assert!(supplier_screen.contains("混入 API KEY"));
    assert!(supplier_screen.contains("config.toml 预览"));
    assert!(supplier_screen.contains("通用配置文件"));
    assert!(supplier_screen.contains("auth.json"));

    assert!(supplier_screen.contains("聚合策略"));
    assert!(app_tsx.contains("失败切换"));
    assert!(app_tsx.contains("按对话轮转"));
    assert!(app_tsx.contains("按请求轮转"));
    assert!(app_tsx.contains("权重轮转"));
    assert!(supplier_screen.contains("aggregate.strategy / aggregate.members"));
    assert!(supplier_screen.contains("请先添加或选择至少 1 个普通 API 供应商"));

    assert!(app_tsx.contains("importCcswitchCodexProviders"));
    assert!(app_tsx.contains("import_ccswitch_codex_providers"));
    assert!(commands_rs.contains("pub fn import_ccswitch_codex_providers"));
    assert!(lib_rs.contains("commands::import_ccswitch_codex_providers"));
    assert!(styles.contains(".supplier-list-shell"));
    assert!(styles.contains(".supplier-drop-popover"));
    assert!(styles.contains(".supplier-card.dragging"));
    assert!(styles.contains(".supplier-card.drag-over"));
    assert!(styles.contains(".supplier-aggregate-grid"));
}

#[test]
fn claude_dev_mode_button_uses_the_current_provider_draft() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let app_tsx = app_tsx.replace("\r\n", "\n");

    assert!(app_tsx.contains("claudeDesktopProviderDraft.baseUrl.trim()"));
    assert!(app_tsx.contains(
        "call<ClaudeDesktopDevModeConfigureResult>(\"configure_claude_desktop_dev_mode\", request)"
    ));
    assert!(app_tsx.contains("? { request: claudeDesktopProviderDraft }\n      : undefined;"));
    assert!(!app_tsx.contains(
        "claudeDesktopProviderDraft.baseUrl.trim() && claudeDesktopProviderDraft.apiKey.trim()"
    ));
}

#[test]
fn injected_status_bars_are_transparent_single_backend_lamp_and_safe_for_codex_text() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let codex_inject = std::fs::read_to_string(repo_root.join("assets/inject/renderer-inject.js"))
        .expect("read renderer inject");
    let claude_inject =
        std::fs::read_to_string(repo_root.join("assets/inject/claude-chinese-inject.js"))
            .expect("read claude chinese inject");

    assert!(codex_inject.contains(".claude-codex-pro-trigger {"));
    assert!(codex_inject.contains("background: transparent;"));
    assert!(codex_inject.contains("CCP ${claudeCodexProVersion}"));
    assert!(codex_inject.contains("claude-codex-pro-window-status-dot"));
    assert!(!codex_inject.contains("claude-codex-pro-window-status-title\">后端"));
    assert!(!codex_inject.contains("claude-codex-pro-row-title\">后端连接"));
    assert!(!codex_inject.contains("正在检查后端…"));
    assert!(codex_inject.contains("Claude Codex Pro ${claudeCodexProVersion}"));
    assert!(!codex_inject.contains("data-codex-open-manager"));
    assert!(!codex_inject.contains("openManagerFromCodex();"));
    assert!(!codex_inject.contains("function openManagerFromCodex"));

    assert!(claude_inject.contains("#ccp-claude-status-pill"));
    assert!(claude_inject.contains("background: transparent;"));
    assert!(claude_inject.contains("CCP ' + ccpDisplayVersion"));
    assert!(claude_inject.contains("data-ccp-backend-status"));
    assert!(claude_inject.contains("Claude Codex Pro ' + ccpDisplayVersion"));
    assert!(!claude_inject.contains("data-ccp-panel-frontend"));
    assert!(!claude_inject.contains("data-ccp-panel-backend"));
    assert!(!claude_inject.contains("前端注入"));
    assert!(!claude_inject.contains("后端连接"));
    assert!(!claude_inject.contains("后端${state.backend}"));
    assert!(claude_inject.contains("if (/\\bCodex\\b/.test(trimmed)) return value;"));
    assert!(claude_inject.contains("if (/\\bCodex\\b/.test(next)) continue;"));
}

#[test]
fn codex_memory_badge_aligns_with_injection_status_strip() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let codex_inject = std::fs::read_to_string(repo_root.join("assets/inject/renderer-inject.js"))
        .expect("read renderer inject");

    assert!(codex_inject.contains("const left = Math.min(Math.max(8, statusRect.right + 8)"));
    assert!(codex_inject.contains("badge.style.height = `${statusRect.height}px`;"));
    assert!(codex_inject.contains("background: transparent;"));
    assert!(codex_inject.contains("<span>盘古记忆</span>"));
    assert!(codex_inject.contains("display: inline-flex;"));
    assert!(!codex_inject.contains("statusRect.right + 12"));
}

#[test]
fn overview_startup_uses_light_claude_status_and_defers_heavy_checks() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx)
        .expect("read manager App.tsx")
        .replace("\r\n", "\n");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");
    let lib_rs = manifest_dir.join("src/lib.rs");
    let lib_rs = std::fs::read_to_string(&lib_rs).expect("read manager lib.rs");

    let refresh_route = app_tsx
        .split("const refreshRoute = async (target = route) => {")
        .nth(1)
        .and_then(|rest| rest.split("const actions = {").next())
        .expect("refreshRoute body");
    let overview_branch = refresh_route
        .split("if (target === \"overview\") {")
        .nth(1)
        .and_then(|rest| rest.split("} else if").next())
        .expect("overview branch");

    assert!(commands_rs.contains("pub async fn load_claude_desktop_status_light()"));
    assert!(commands_rs.contains("detect_status_light()"));
    assert!(lib_rs.contains("commands::load_claude_desktop_status_light"));
    assert!(app_tsx.contains("\"load_claude_desktop_status_light\""));
    assert!(overview_branch.contains("refreshClaudeLight(true)"));
    assert!(!overview_branch.contains("refreshClaude(true)"));
    assert!(overview_branch.contains("afterFirstPaint"));
    assert!(overview_branch.contains("refreshClaudeZhPatch(true)"));
    assert!(overview_branch.contains("refreshMemoryAssistStatus(true)"));
}

#[test]
fn claude_zh_patch_primary_action_does_not_prompt_for_directory() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    let primary_start = app_tsx
        .find("const installClaudeZhPatch = async () => {")
        .expect("primary zh patch action");
    let manual_start = app_tsx
        .find("const installClaudeZhPatchFromDirectory = async () => {")
        .expect("manual zh patch action");
    let primary_action = &app_tsx[primary_start..manual_start];
    let manual_action = &app_tsx[manual_start
        ..app_tsx[manual_start..]
            .find("const openClaudeChinese")
            .unwrap()
            + manual_start];

    assert!(app_tsx.contains(
        "const writeUiEvent = async (event: string, detail: Record<string, unknown> = {}) => {"
    ));
    assert!(app_tsx.contains("\"write_diagnostic_event\", { event, detail }"));
    assert!(primary_action.contains("void writeUiEvent(\"claude_zh_patch.install.click\")"));
    assert!(manual_action.contains("writeUiEvent(\"claude_zh_patch.manual_install.click\")"));
    assert!(primary_action.contains("\"install_claude_zh_patch\""));
    assert!(primary_action.contains("status: \"running\""));
    assert!(primary_action.contains("waitForPaint()"));
    assert!(primary_action.contains("setNotice({ title: \"Claude 一键汉化\", message: zhPatchNoticeMessage(autoResult), status: autoResult.status })"));
    assert!(
        primary_action.find("setNotice({").unwrap()
            < primary_action
                .find("void writeUiEvent(\"claude_zh_patch.install.click\")")
                .unwrap()
    );
    assert!(
        primary_action
            .find("void writeUiEvent(\"claude_zh_patch.install.click\")")
            .unwrap()
            < primary_action.find("\"install_claude_zh_patch\"").unwrap()
    );
    assert!(!primary_action.contains("open({ directory: true"));
    assert!(!primary_action.contains("install_claude_zh_patch_at_install_root"));
    assert!(manual_action.contains("open({ directory: true"));
    assert!(manual_action.contains("status: \"running\""));
    assert!(manual_action.contains("waitForPaint()"));
    assert!(manual_action.contains("setNotice({ title: \"Claude 手动汉化\", message: zhPatchNoticeMessage(result), status: result.status })"));
    assert!(manual_action.contains("install_claude_zh_patch_at_install_root"));
    assert!(app_tsx.contains("actions.installClaudeZhPatchFromDirectory()"));
    assert!(commands_rs.contains("pub async fn install_claude_zh_patch_at_install_root"));
    assert!(commands_rs.contains("install_root_patch_needs_elevation(&install_root)"));
    assert!(
        commands_rs.contains("install_claude_zh_patch_elevated_at_install_root(&install_root)")
    );
    assert!(commands_rs.contains("status_for_install_root(&install_root)"));
}

#[test]
fn diagnostics_include_running_exe_identity_for_zh_patch_debugging() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = manifest_dir.join("src/lib.rs");
    let lib_rs = std::fs::read_to_string(&lib_rs).expect("read manager lib.rs");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    assert!(commands_rs.contains("pub fn current_exe_path_string() -> String"));
    assert!(commands_rs.contains("pub fn current_exe_last_modified_ms() -> Option<u128>"));
    assert!(commands_rs.contains("exe_path: current_exe_path_string()"));
    assert!(commands_rs.contains("exe_last_modified_ms: current_exe_last_modified_ms()"));
    assert!(commands_rs.contains("\"exePath\": current_exe_path_string()"));
    assert!(commands_rs.contains("\"exeLastModifiedMs\": current_exe_last_modified_ms()"));
    assert!(lib_rs.contains("\"exePath\": commands::current_exe_path_string()"));
    assert!(lib_rs.contains("\"exeLastModifiedMs\": commands::current_exe_last_modified_ms()"));
}

#[test]
fn claude_zh_patch_restore_shows_immediate_running_toast() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");

    let restore_action = app_tsx
        .split("const restoreClaudeZhPatch = async () => {")
        .nth(1)
        .and_then(|rest| rest.split("const launchClaudeDesktop").next())
        .expect("restore zh patch action source");

    assert!(restore_action.contains("void writeUiEvent(\"claude_zh_patch.restore.click\")"));
    assert!(restore_action.contains("setNotice({"));
    assert!(restore_action.contains("title: \"恢复 Claude 官方文件\""));
    assert!(restore_action.contains("status: \"running\""));
    assert!(restore_action.contains("waitForPaint()"));
    assert!(restore_action.contains("setNotice({ title: \"恢复 Claude 官方文件\", message: result.message, status: result.status })"));
    assert!(
        restore_action.find("setNotice({").unwrap()
            < restore_action
                .find("void writeUiEvent(\"claude_zh_patch.restore.click\")")
                .unwrap()
    );
    assert!(restore_action.contains("\"restore_claude_zh_patch\""));
}

#[test]
fn claude_zh_patch_auto_launches_claude_after_successful_install() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");

    assert!(commands_rs.contains("fn complete_claude_zh_patch_install("));
    assert!(commands_rs.contains("claude_codex_pro_core::claude_desktop::open_claude_desktop()"));
    assert!(commands_rs.contains("manager.claude_zh_patch.launch_after_install"));
    assert!(commands_rs.contains("已自动启动/重启 Claude Desktop，请验证界面语言。"));
    assert!(!commands_rs.contains("Restart Claude Desktop to see the result."));

    assert!(app_tsx.contains("已自动启动/重启 Claude Desktop，请验证界面语言。"));
    assert!(!app_tsx.contains("请重启 Claude Desktop 后验证界面语言。"));
}

#[test]
fn claude_zh_patch_msix_path_uses_uac_elevation_instead_of_dead_end() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let main_rs = manifest_dir.join("src/main.rs");
    let main_rs = std::fs::read_to_string(&main_rs).expect("read manager main.rs");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");
    let core_zh_patch = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/claude-codex-pro-core/src/claude_zh_patch.rs");
    let core_zh_patch =
        std::fs::read_to_string(&core_zh_patch).expect("read core claude_zh_patch.rs");

    assert!(main_rs.contains("handle_internal_cli()"));
    assert!(commands_rs.contains("detected_patch_needs_elevation()"));
    assert!(commands_rs.contains("--internal-install-claude-zh-patch"));
    assert!(commands_rs.contains("manager.claude_zh_patch.install.start"));
    assert!(commands_rs.contains("manager.claude_zh_patch.install.close_claude_failed"));
    assert!(commands_rs.contains("manager.claude_zh_patch.install.elevation_required"));
    assert!(commands_rs.contains("manager.claude_zh_patch.install.direct.start"));
    assert!(commands_rs.contains("current_user_sid()"));
    assert!(commands_rs.contains("windows_argument_list(&["));
    assert!(commands_rs.contains("powershell_single_quoted(&argument_list)"));
    assert!(commands_rs.contains("manager.claude_zh_patch.elevated.start"));
    assert!(commands_rs.contains("manager.claude_zh_patch.elevated.exit"));
    assert!(commands_rs.contains("manager.claude_zh_patch.elevated.result"));
    assert!(commands_rs.contains("default_app_state_dir().join(\"tmp\")"));
    assert!(!commands_rs.contains("let result_path = std::env::temp_dir().join"));
    assert!(commands_rs.contains("manager.claude_zh_patch.internal.start"));
    assert!(commands_rs.contains("manager.claude_zh_patch.internal.finish"));
    assert!(commands_rs.contains("manager.claude_zh_patch.internal.result_write_failed"));
    assert!(commands_rs.contains("targetDiagnosticLogPresent"));
    assert!(commands_rs.contains("set_diagnostic_log_path_override"));
    assert!(commands_rs.contains("&diagnostic_log_path"));
    assert!(!commands_rs.contains("Start-Process -FilePath {exe_quoted} -ArgumentList {argument_list_quoted} -Verb RunAs -Wait -PassThru -Environment"));
    assert!(commands_rs.contains("install_claude_zh_patch_internal("));
    assert!(commands_rs.contains("target_appdata.as_deref()"));
    assert!(commands_rs.contains("target_localappdata.as_deref()"));
    assert!(commands_rs.contains("target_install_root.as_deref()"));
    assert!(commands_rs.contains("install_patch_with_remote_resources_elevated_for_user_dirs("));
    assert!(
        commands_rs.contains(
            "install_patch_with_remote_resources_elevated_for_user_dirs_at_install_root("
        )
    );
    assert!(commands_rs.contains("whoami.exe"));
    assert!(commands_rs.contains(".args([\"/user\", \"/fo\", \"csv\", \"/nh\"])"));
    assert!(commands_rs.contains("-Verb RunAs -Wait -PassThru"));
    assert!(core_zh_patch.contains("pub fn detected_patch_needs_elevation()"));
    assert!(core_zh_patch.contains("appx_claude_install_roots()"));
    assert!(core_zh_patch.contains("Get-AppxPackage -Name Claude"));
    assert!(core_zh_patch.contains("Get-AppxPackage -AllUsers -Name Claude"));
    assert!(
        core_zh_patch
            .contains("self.install_kind == \"msix\" && !resource_tree_writable_no_create(self)")
    );
    assert!(core_zh_patch.contains("fn resource_tree_writable_no_create"));
    assert!(core_zh_patch.contains("fn resource_tree_writable_or_create"));
    assert!(core_zh_patch.contains("prepare_elevated_patch_access(paths, target_user_sid)?;"));
    assert!(core_zh_patch.contains("ensure_patch_writable(paths)?;"));
    assert!(core_zh_patch.contains("patch_target_dirs(paths)"));
    assert!(core_zh_patch.contains("probe_writable_dir(&dir)"));
    assert!(core_zh_patch.contains("unique_atomic_temp_path_for(path)"));
    assert!(core_zh_patch.contains("std::fs::write(path, contents).with_context"));
    assert!(core_zh_patch.contains("直接覆盖 Claude 汉化文件失败"));
    assert!(!core_zh_patch.contains("创建 zh-CN.json.tmp 后替换资源文件"));
    assert!(
        core_zh_patch.contains("include_str!(\"../../../assets/claude-zh/frontend-zh-CN.json\")")
    );
    assert!(core_zh_patch.contains("unwrap_or_else(|_| embedded_i18n_resources())"));
    assert!(core_zh_patch.contains("pub fn embedded_i18n_resources() -> RemoteI18nResources"));
    assert!(core_zh_patch.contains("remove_zh_cn_artifacts(paths, &mut changed_files)?;"));
    assert!(core_zh_patch.contains("remove_locale_config(paths, &mut changed_files)?;"));
    assert!(core_zh_patch.contains("scrub_zh_cn_from_chunks(paths, &mut changed_files)?;"));
    assert!(core_zh_patch.contains("access_warnings.extend(grant_current_user_write_access("));
    assert!(!core_zh_patch.contains("grant_current_user_write_access(\n        &paths.app_root"));
    assert!(core_zh_patch.contains("for target in patch_access_targets(paths)"));
    assert!(core_zh_patch.contains(
        "access_warnings.extend(grant_current_user_write_access(&target, target_user_sid)?);"
    ));
    assert!(core_zh_patch.contains("fn patch_target_files(paths: &ClaudeZhPatchPaths)"));
    assert!(core_zh_patch.contains("fn patch_access_targets(paths: &ClaudeZhPatchPaths)"));
    assert!(core_zh_patch.contains("write_patch_file_for_install("));
    assert!(core_zh_patch.contains("retry_write_patch_file_after_elevated_access("));
    assert!(core_zh_patch.contains("管理员授权后写入 Claude 汉化文件仍失败"));
    assert!(core_zh_patch.contains("授权诊断"));
    assert!(core_zh_patch.contains("icacls 当前用户"));
    assert!(core_zh_patch.contains("icacls Users 组"));
    assert!(core_zh_patch.contains("icacls Administrators 组"));
    assert!(core_zh_patch.contains("*S-1-5-32-544:(OI)(CI)F"));
    assert!(core_zh_patch.contains("user_grant.is_err() && users_grant.is_err()"));
    assert!(core_zh_patch.contains("is_real_windows_apps_path(&paths.install_root)"));
    assert!(core_zh_patch.contains("takeown.exe"));
    assert!(core_zh_patch.contains("icacls.exe"));
}

#[test]
fn claude_zh_patch_commands_close_claude_before_writing_resources() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    let install_action = commands_rs
        .split("pub async fn install_claude_zh_patch()")
        .nth(1)
        .and_then(|rest| rest.split("pub fn handle_internal_cli()").next())
        .expect("install command source");
    let manual_action = commands_rs
        .split("pub async fn install_claude_zh_patch_at_install_root")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[tauri::command]\npub fn restore_claude_zh_patch")
                .next()
        })
        .expect("manual command source");
    let restore_action = commands_rs
        .split("pub fn restore_claude_zh_patch()")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[tauri::command]\npub fn new_claude_desktop_chat")
                .next()
        })
        .expect("restore command source");

    assert!(install_action.contains("close_claude_desktop_for_patch()"));
    assert!(install_action.contains("Failed to close Claude Desktop before patch"));
    assert!(manual_action.contains("close_claude_desktop_for_patch()"));
    assert!(manual_action.contains("Failed to close Claude Desktop before manual patch"));
    assert!(restore_action.contains("close_claude_desktop_for_patch()"));
    assert!(restore_action.contains("Failed to close Claude Desktop before restore"));
}

#[test]
fn claude_zh_patch_closes_claude_before_elevation_branch() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    let install_action = commands_rs
        .split("pub async fn install_claude_zh_patch()")
        .nth(1)
        .and_then(|rest| rest.split("pub fn handle_internal_cli()").next())
        .expect("install command source");
    let manual_action = commands_rs
        .split("pub async fn install_claude_zh_patch_at_install_root")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[tauri::command]\npub fn restore_claude_zh_patch")
                .next()
        })
        .expect("manual command source");
    let restore_action = commands_rs
        .split("pub fn restore_claude_zh_patch()")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[tauri::command]\npub fn new_claude_desktop_chat")
                .next()
        })
        .expect("restore command source");

    assert!(
        install_action
            .find("close_claude_desktop_for_patch()")
            .unwrap()
            < install_action
                .find("detected_patch_needs_elevation()")
                .unwrap()
    );
    assert!(
        manual_action
            .find("close_claude_desktop_for_patch()")
            .unwrap()
            < manual_action
                .find("install_root_patch_needs_elevation(&install_root)")
                .unwrap()
    );
    assert!(
        restore_action
            .find("close_claude_desktop_for_patch()")
            .unwrap()
            < restore_action
                .find("detected_patch_needs_elevation()")
                .unwrap()
    );
}

#[test]
fn claude_zh_patch_parent_verifies_final_status_after_elevation() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    let install_action = commands_rs
        .split("pub async fn install_claude_zh_patch()")
        .nth(1)
        .and_then(|rest| rest.split("pub fn handle_internal_cli()").next())
        .expect("install command source");
    let restore_action = commands_rs
        .split("pub fn restore_claude_zh_patch()")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[tauri::command]\npub fn new_claude_desktop_chat")
                .next()
        })
        .expect("restore command source");

    assert!(install_action.contains("if status.status != \"ok\""));
    assert!(install_action.contains("elevated run did not complete"));
    assert!(restore_action.contains("if status.status != \"not_installed\""));
    assert!(restore_action.contains("elevated run left patch residue"));
}

#[test]
fn claude_zh_patch_falls_back_to_uac_when_direct_msix_write_fails() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    let install_action = commands_rs
        .split("pub async fn install_claude_zh_patch()")
        .nth(1)
        .and_then(|rest| rest.split("pub fn handle_internal_cli()").next())
        .expect("install command source");

    assert!(install_action.contains("manager.claude_zh_patch.direct.failed"));
    assert!(install_action.contains("should_retry_claude_zh_patch_with_elevation(&error)"));
    assert!(install_action.contains("install_claude_zh_patch_elevated()"));
    assert!(
        commands_rs.contains(
            "fn should_retry_claude_zh_patch_with_elevation(error: &anyhow::Error) -> bool"
        )
    );
    assert!(commands_rs.contains(
        "should_retry_claude_zh_patch_status_with_elevation(&status.install_kind, error)"
    ));
    assert!(commands_rs.contains("if install_kind != \"msix\""));
}

#[test]
fn claude_zh_patch_manual_install_falls_back_to_uac_when_direct_msix_write_fails() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    let manual_action = commands_rs
        .split("pub async fn install_claude_zh_patch_at_install_root")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[tauri::command]\npub fn restore_claude_zh_patch")
                .next()
        })
        .expect("manual command source");

    assert!(manual_action.contains("manager.claude_zh_patch.manual_direct.failed"));
    assert!(manual_action.contains("manager.claude_zh_patch.manual_install.start"));
    assert!(manual_action.contains("manager.claude_zh_patch.manual_install.close_claude_failed"));
    assert!(manual_action.contains("manager.claude_zh_patch.manual_install.elevation_required"));
    assert!(manual_action.contains("manager.claude_zh_patch.manual_install.direct.start"));
    assert!(manual_action.contains(
        "should_retry_claude_zh_patch_with_elevation_at_install_root(&install_root, &error)"
    ));
    assert!(
        manual_action.contains("install_claude_zh_patch_elevated_at_install_root(&install_root)")
    );
    assert!(
        commands_rs.contains("fn should_retry_claude_zh_patch_with_elevation_at_install_root(")
    );
    assert!(commands_rs.contains("status_for_install_root(install_root)"));
}

#[test]
fn claude_zh_patch_restore_uses_uac_elevation_for_msix_paths() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");
    let core_zh_patch = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/claude-codex-pro-core/src/claude_zh_patch.rs");
    let core_zh_patch =
        std::fs::read_to_string(&core_zh_patch).expect("read core claude_zh_patch.rs");

    assert!(commands_rs.contains("--internal-restore-claude-zh-patch"));
    assert!(commands_rs.contains("manager.claude_zh_patch.restore.start"));
    assert!(commands_rs.contains("manager.claude_zh_patch.restore.close_claude_failed"));
    assert!(commands_rs.contains("manager.claude_zh_patch.restore.elevation_required"));
    assert!(commands_rs.contains("manager.claude_zh_patch.restore.direct.start"));
    assert!(commands_rs.contains("restore_claude_zh_patch_internal("));
    assert!(commands_rs.contains("restore_claude_zh_patch_elevated()"));
    assert!(commands_rs.contains("restore_patch_elevated_for_user_dirs("));
    assert!(core_zh_patch.contains("pub fn restore_patch_elevated_for_user_dirs"));
    assert!(core_zh_patch.contains("restore_patch_at_elevated_for_user(&paths, target_user_sid)"));
    assert!(core_zh_patch.contains("pub fn restore_patch_at_elevated_for_user"));
    assert!(core_zh_patch.contains("prepare_elevated_patch_access(paths, target_user_sid)?;"));
    assert!(core_zh_patch.contains("restore_patch_at(paths)"));
}

#[test]
fn claude_zh_patch_elevated_cli_uses_original_user_data_dirs() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");
    let core_zh_patch = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/claude-codex-pro-core/src/claude_zh_patch.rs");
    let core_zh_patch =
        std::fs::read_to_string(&core_zh_patch).expect("read core claude_zh_patch.rs");

    assert!(commands_rs.contains("current_user_data_dirs()"));
    assert!(commands_rs.contains("windows_argument_list(&["));
    assert!(commands_rs.contains("powershell_single_quoted(&argument_list)"));
    assert!(commands_rs.contains("-ArgumentList {argument_list_quoted}"));
    assert!(commands_rs.contains("target_install_root"));
    let internal_cli = commands_rs
        .split("pub fn handle_internal_cli()")
        .nth(1)
        .and_then(|rest| rest.split("fn install_claude_zh_patch_internal").next())
        .expect("internal cli source");
    assert!(internal_cli.contains("install_claude_zh_patch_internal("));
    assert!(internal_cli.contains("restore_claude_zh_patch_internal("));
    assert!(internal_cli.contains("target_user_sid.as_deref()"));
    assert!(internal_cli.contains("target_appdata.as_deref()"));
    assert!(internal_cli.contains("target_localappdata.as_deref()"));
    assert!(internal_cli.contains("target_install_root.as_deref()"));
    assert!(core_zh_patch.contains("detect_paths_for_user_dirs"));
    assert!(core_zh_patch.contains("with_user_data_dirs"));
    assert!(
        core_zh_patch
            .contains("install_patch_with_remote_resources_elevated_for_user_dirs_at_install_root")
    );
    assert!(core_zh_patch.contains("restore_patch_elevated_for_user_dirs_at_install_root"));
    assert!(core_zh_patch.contains("appdata.join(\"Claude-3p\").join(\"config.json\")"));
    assert!(core_zh_patch.contains("local_appdata.join(BACKUP_DIR_NAME)"));
}

#[test]
fn claude_zh_patch_elevated_process_has_timeout_and_kills_hung_child() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    assert!(commands_rs.contains("CLAUDE_ZH_PATCH_ELEVATED_TIMEOUT"));
    assert!(commands_rs.contains("run_elevated_process_with_timeout"));
    assert!(commands_rs.contains("command.spawn()?"));
    assert!(commands_rs.contains("command.stdout(std::process::Stdio::piped())"));
    assert!(commands_rs.contains("command.stderr(std::process::Stdio::piped())"));
    assert!(commands_rs.contains("child.try_wait()?"));
    assert!(commands_rs.contains("child.wait_with_output()"));
    assert!(commands_rs.contains("stdout={}"));
    assert!(commands_rs.contains("stderr={}"));
    assert!(commands_rs.contains("Elevated child did not write result file"));
    assert!(commands_rs.contains("child.kill()"));
    assert!(commands_rs.contains("Elevated Claude Chinese patch timed out"));
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
    assert!(app_tsx.contains("启动/重启Claude"));
    assert!(app_tsx.contains("Claude 一键汉化"));
    assert!(app_tsx.contains("Claude 一键开发模式"));
    assert!(app_tsx.contains("启动/重启Codex"));
    assert!(app_tsx.contains("load_watcher_state"));
    assert!(app_tsx.contains("install_entrypoints"));
    assert!(app_tsx.contains("uninstall_entrypoints"));
    assert!(app_tsx.contains("install_watcher"));
    assert!(app_tsx.contains("disable_watcher"));

    assert!(app_tsx.contains("function SettingsScreen"));
    assert!(app_tsx.contains("设置文件位置"));
    assert!(app_tsx.contains("Codex 增强矩阵"));
    assert!(app_tsx.contains("Claude 一键汉化"));
    let zh_settings_panel = app_tsx
        .split("<Panel title=\"Claude 一键汉化\"")
        .nth(1)
        .and_then(|rest| rest.split("<Panel title=\"CLI Wrapper\"").next())
        .expect("Claude zh settings panel source");
    assert!(zh_settings_panel.contains("安装类型"));
    assert!(zh_settings_panel.contains("目录可写"));
    assert!(zh_settings_panel.contains("诊断日志"));
    assert!(zh_settings_panel.contains("桌面资源"));
    assert!(zh_settings_panel.contains("前端资源"));
    assert!(zh_settings_panel.contains("Statsig 资源"));
    assert!(!zh_settings_panel.contains("入口 URL"));
    assert!(!zh_settings_panel.contains("wrapped_webview"));
    assert!(app_tsx.contains("CLI Wrapper"));
    assert!(app_tsx.contains("Codex 启动参数"));
    assert!(app_tsx.contains("安全边界"));
    assert!(app_tsx.contains("reset_settings"));
    assert!(app_tsx.contains("reset_image_overlay_settings"));
    assert!(app_tsx.contains("saveSettings"));
    assert!(app_tsx.contains("logsPath: string;"));
    assert!(app_tsx.contains("function zhPatchNoticeMessage"));
    assert!(app_tsx.contains("诊断日志：${logPath}"));
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
    assert!(app_tsx.contains("__CLAUDE_CODEX_PRO_INITIAL_ROUTE"));
    assert!(app_tsx.contains("window.location.search"));
}
