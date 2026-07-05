/// 递归拼接 manager 前端 `src/` 下所有 `.ts` / `.tsx` 源码，作为断言目标。
///
/// App.tsx 已被拆分成 types.ts / constants.ts / lib/* / actions.ts /
/// components/* / screens/* 等多个文件（任务#3）。原先按 `src/App.tsx` 单文件
/// 做的 `contains(...)` 回归护栏断言，凡是"某字符串必须/禁止出现在前端源码里"
/// 的语义，改用本函数读取全部前端源码拼接后的字符串——字符串迁到哪个文件都能命中，
/// 且 `!contains` 覆盖面更广（禁止它出现在任何前端文件里），不会削弱护栏。
///
/// 注意：仅对"字符串在前端源码全集里唯一/不会假阳性"的断言使用本函数。对那些
/// 字符串合法存在于多个组件、需要限定在特定组件文件内判断的结构化断言，仍读取
/// 对应组件文件的完整内容（见各测试内注释）。
fn read_all_frontend_sources() -> String {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.parent().unwrap().join("src");
    let mut combined = String::new();
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let is_ts = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "ts" || ext == "tsx")
                .unwrap_or(false);
            if is_ts {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    combined.push_str(&contents);
                    combined.push('\n');
                }
            }
        }
    }
    combined
}

/// 读取拆分后某个前端源文件的完整内容（相对 `src/`），用于结构化断言。
fn read_frontend_file(relative: &str) -> String {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.parent().unwrap().join("src").join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read frontend file {relative}"))
}

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
    let capability = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/capabilities/default.json"
    ))
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
    assert!(workflow.contains("run: npm run vite:build"));
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
    assert!(release_assets.contains("run: npm run vite:build"));
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
    let app_tsx = read_all_frontend_sources();
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
fn pr_build_workflow_refreshes_manager_frontend_before_packaging() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .unwrap();
    let workflow = std::fs::read_to_string(repo_root.join(".github/workflows/pr-build.yml"))
        .expect("read pr build workflow");

    assert!(workflow.contains("run: npm run vite:build"));
    assert!(workflow.contains("run: cargo build --release"));
}

#[test]
fn plugin_hub_is_first_class_ops_console_route() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = read_all_frontend_sources();
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
    let app_tsx = read_all_frontend_sources();
    let screens_file = read_frontend_file("screens.tsx");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    assert!(!app_tsx.contains("id: \"context\""));
    assert!(!app_tsx.contains("id: \"pluginHub\""));
    assert!(app_tsx.contains("id: \"maintenance\""));
    assert!(app_tsx.contains("id: \"about\""));
    assert!(!app_tsx.contains("id: \"scripts\""));
    assert!(!app_tsx.contains("id: \"logs\""));
    let tools_section = screens_file
        .split("function ToolsAndPluginsScreen")
        .nth(1)
        .and_then(|rest| rest.split("function SessionManagementScreen").next())
        .expect("tools screen source");

    assert!(tools_section.contains("ContextManagerPanel"));
    assert!(tools_section.contains("scope=\"codex\""));
    assert!(tools_section.contains("scope=\"claude\""));
    assert!(tools_section.contains("claudeContextEntries"));
    assert!(app_tsx.contains("Codex 工具与插件"));
    assert!(app_tsx.contains("Claude 工具与插件"));
    assert!(app_tsx.contains("list_context_entries"));
    assert!(app_tsx.contains("upsert_context_entry"));
    assert!(app_tsx.contains("delete_context_entry"));
    assert!(app_tsx.contains("list_claude_context_entries"));
    assert!(app_tsx.contains("upsert_claude_context_entry"));
    assert!(app_tsx.contains("delete_claude_context_entry"));
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
fn tools_route_auto_detects_and_repairs_plugin_repositories_with_visible_feedback() {
    let app_tsx = read_all_frontend_sources();
    let app_tsx_file = read_frontend_file("App.tsx");

    assert!(app_tsx.contains("PLUGIN_REPOSITORY_REPAIR_PROMPT_KEY_PREFIX"));
    assert!(app_tsx.contains("function codexPluginMarketplaceNeedsRepair"));
    assert!(app_tsx.contains("function claudeDesktopMarketplaceNeedsRepair"));
    assert!(app_tsx.contains("const promptAndRepairPluginRepositories = async"));
    assert!(app_tsx.contains("window.confirm(pluginRepositoryRepairPromptMessage(codex, claude))"));
    assert!(app_tsx.contains(
        "await promptAndRepairPluginRepositories(codexMarketplaceStatus, claudeMarketplaceStatus)"
    ));

    let tools_route = app_tsx_file
        .split("} else if (target === \"tools\")")
        .nth(1)
        .and_then(|rest| rest.split("} else if (target === \"sessions\")").next())
        .expect("tools route refresh source");
    assert!(tools_route.contains("refreshCodexPluginMarketplace(true)"));
    assert!(tools_route.contains("refreshClaudeDesktopMarketplace(true)"));
    assert!(tools_route.contains("codexMarketplaceStatus"));
    assert!(tools_route.contains("claudeMarketplaceStatus"));
    assert!(tools_route.contains("promptAndRepairPluginRepositories"));

    let codex_refresh = app_tsx_file
        .split("const refreshCodexPluginMarketplace = async")
        .nth(1)
        .and_then(|rest| rest.split("const refreshLocalSessions").next())
        .expect("codex marketplace refresh source");
    assert!(codex_refresh.contains("status: \"running\""));
    assert!(codex_refresh.contains("load_codex_plugin_marketplace_status"));
    assert!(codex_refresh.contains("setNotice({ title:"));

    let claude_refresh = app_tsx_file
        .split("const refreshClaudeDesktopMarketplace = async")
        .nth(1)
        .and_then(|rest| rest.split("const refreshClaudeDesktopDevMode").next())
        .expect("claude marketplace refresh source");
    assert!(claude_refresh.contains("status: \"running\""));
    assert!(claude_refresh.contains("load_claude_desktop_marketplace_status"));
    assert!(claude_refresh.contains("setNotice({ title:"));

    let codex_repair = app_tsx_file
        .split("const repairCodexPluginMarketplace = async")
        .nth(1)
        .and_then(|rest| rest.split("const promptAndRepairPluginRepositories").next())
        .expect("codex marketplace repair source");
    assert!(codex_repair.contains("status: \"running\""));
    assert!(codex_repair.contains("repair_codex_plugin_marketplace"));

    let claude_repair = app_tsx_file
        .split("const repairClaudeDesktopMarketplaces = async")
        .nth(1)
        .and_then(|rest| rest.split("const configureClaudeDesktopDevMode").next())
        .expect("claude marketplace repair source");
    assert!(claude_repair.contains("status: \"running\""));
    assert!(claude_repair.contains("repair_claude_desktop_marketplaces"));

    assert!(
        !app_tsx
            .contains("onClick={() => void actions.openPonytailClaudeDesktopMarketplaceSetup()}")
    );
    assert!(!app_tsx.contains("可选官方仓库"));
}

#[test]
fn plugin_memory_tools_ui_regression_is_locked_down() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // 拆分后：字符串存在性护栏改读前端源码全集（迁到哪个文件都命中；!contains 覆盖更广）。
    let app_tsx = read_all_frontend_sources();
    let styles_path = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles_path).expect("read manager styles.css");

    assert!(!app_tsx.contains("<span>后端链接</span>"));
    assert!(!app_tsx.contains("className=\"ops-topbar-pill\""));
    assert!(styles.contains("grid-template-columns: minmax(220px, 1fr) auto;"));

    assert!(!app_tsx.contains("label=\"Codex 官方仓库\""));
    assert!(app_tsx.contains("repositories.map((repository) => ("));
    assert!(app_tsx.contains("awesome-codex-plugins"));
    assert!(app_tsx.contains("https://github.com/hashgraph-online/awesome-codex-plugins.git"));
    assert!(app_tsx.contains("Product Design Skill 仓库"));
    assert!(app_tsx.contains("codex-skills-alternative"));
    assert!(app_tsx.contains("codex-skills-alternative-marketplace"));
    assert!(app_tsx.contains("codexMarketplaceAutoRegisterRef"));
    assert!(app_tsx.contains("repairCodexPluginMarketplace(true)"));
    assert!(app_tsx.contains("repository.configured ? \"已写入\" : \"未写入\""));

    assert!(app_tsx.contains("className=\"context-entry-actions\""));
    assert!(styles.contains(".context-entry-actions"));
    assert!(styles.contains("grid-template-columns: 48px 32px 32px;"));
    assert!(styles.contains(".context-entry-actions .toggle-switch"));
    assert!(styles.contains("grid-column: 1;"));
    assert!(styles.contains("width: 48px;"));
    assert!(styles.contains("height: 26px;"));
    assert!(styles.contains("align-items: center;"));
    assert!(styles.contains("flex: 0 0 20px;"));
    // 拨钮垂直居中改用 top/bottom:0 + margin-block:auto（不再靠 translate 的 -50% 兜底，
    // 那种写法会因 box-shadow 显得偏下）；选中态只做水平位移 translate(22px, 0)。
    assert!(styles.contains("transform: translate(22px, 0);"));
    assert!(styles.contains("margin-block: auto;"));
    assert!(!styles.contains(".context-entry-actions .toggle-switch.checked .toggle-switch-thumb"));
    assert!(!styles.contains(".context-entry-actions .toggle-switch-thumb"));
    assert!(styles.contains("grid-template-columns: minmax(0, 1fr) 124px;"));
    assert!(app_tsx.contains("setNotice({ title: \"Codex 插件仓库\", message: result.message || result.repair.message, status: result.status })"));
    assert!(styles.contains("overflow-wrap: anywhere;"));
    assert!(styles.contains("white-space: normal;"));
}

#[test]
fn session_management_route_contains_history_memory_and_diagnostics() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // Screen 组件已拆分到 src/screens.tsx；结构化切片读该文件。
    let app_tsx = read_frontend_file("screens.tsx");
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
    assert!(session_section.contains("groupLocalSessionsByProject(sessions)"));
    assert!(session_section.contains("className=\"codex-session-browser\""));
    assert!(session_section.contains("Codex 本地会话项目列表"));
    assert!(session_section.contains("className=\"codex-session-project-header\""));
    assert!(session_section.contains("className=\"codex-session-main\""));
    assert!(session_section.contains("formatSessionRelativeTime(session.updatedAtMs)"));
    assert!(session_section.contains("actions.deleteLocalSession(session)"));
    assert!(session_section.contains("repairHistorySessions"));
    assert!(session_section.contains("launchClaudeDesktop"));
    assert!(session_section.contains("installClaudeZhPatch"));
    assert!(!session_section.contains("openClaudeChinese"));
    assert!(styles.contains(".ops-two-column"));
    assert!(styles.contains(".codex-session-browser"));
    assert!(!styles.contains("background: #f3eeee;"));
    assert!(styles.contains("rgba(8, 9, 12, 0.72);"));
    assert!(styles.contains(".codex-session-project-header"));
    assert!(styles.contains(".codex-session-main time"));
    assert!(styles.contains(".codex-session-delete"));
}

#[test]
fn prompt_optimizer_is_integrated_as_tools_card_launcher() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = read_all_frontend_sources();
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
    // 拆分后：存在性/禁止性断言读前端源码全集（字符串迁到哪个文件都能命中，
    // 且 !contains 覆盖所有前端文件，护栏更强）；结构化切片仍读 App.tsx 单文件。
    let app_tsx = read_all_frontend_sources();
    let screens_file = read_frontend_file("screens.tsx");
    let tauri_bridge = manifest_dir.parent().unwrap().join("src/tauriBridge.ts");
    let tauri_bridge = std::fs::read_to_string(&tauri_bridge).expect("read manager tauriBridge.ts");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");
    let lib_rs =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read manager lib.rs");
    let commands_rs = std::fs::read_to_string(manifest_dir.join("src/commands.rs"))
        .expect("read manager commands.rs");
    let tauri_conf =
        std::fs::read_to_string(manifest_dir.join("tauri.conf.json")).expect("read tauri config");
    let launcher_main = std::fs::read_to_string(
        manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("claude-codex-pro-launcher/src/main.rs"),
    )
    .expect("read launcher main.rs");

    assert!(app_tsx.contains("ops-shell"));
    assert!(app_tsx.contains("ops-rail"));
    assert!(app_tsx.contains("ops-commandbar"));
    assert!(app_tsx.contains("id: \"supplier\""));
    assert!(app_tsx.contains("label: \"供应商\""));
    let routes_file = read_frontend_file("lib/routes.ts");
    let route_source = routes_file
        .split("const routes")
        .nth(1)
        .and_then(|rest| rest.split("function isRoute").next())
        .expect("manager route source");
    assert!(route_source.contains("id: \"maintenance\""));
    assert!(route_source.contains("label: \"维护\""));
    assert!(route_source.contains("id: \"about\""));
    assert!(route_source.contains("label: \"关于\""));
    assert!(!route_source.contains("label: \"脚本\""));
    assert!(!route_source.contains("label: \"日志\""));
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
    assert!(!app_tsx.contains("route !== \"overview\""));
    assert!(!app_tsx.contains("<span>后端链接</span>"));
    assert!(!app_tsx.contains("className=\"ops-topbar-pill\""));
    assert!(app_tsx.contains("Codex 状态"));
    assert!(app_tsx.contains("Claude 状态"));
    assert!(app_tsx.contains("盘古记忆"));
    assert!(app_tsx.contains("盘古记忆总览"));
    assert!(app_tsx.contains("诊断与修复"));
    assert!(app_tsx.contains("function codexOverviewStatus"));
    assert!(app_tsx.contains("function claudeOverviewStatus"));
    assert!(app_tsx.contains("function memoryOverviewStatus"));
    assert!(app_tsx.contains(
        "const result = await run(() => call<SettingsResult>(\"load_settings\"), \"设置\""
    ));
    assert!(app_tsx.contains("if (result) {\n      setSettings(result);"));
    assert!(app_tsx.contains(
        "const saved = await actions.saveSettings({ ...settings, memoryAssistEnabled: enabled });"
    ));
    assert!(app_tsx.contains("if (saved) await actions.refreshMemoryAssist();"));
    assert!(!app_tsx.contains("设置尚未加载，无法切换盘古记忆。"));
    assert!(app_tsx.contains("status-segment-list"));
    assert!(app_tsx.contains("status-segment"));
    assert!(app_tsx.contains("status-action-tile"));
    assert!(app_tsx.contains("运行中"));
    assert!(app_tsx.contains("未运行"));
    assert!(app_tsx.contains("注入成功"));
    assert!(app_tsx.contains("前端在线"));
    assert!(app_tsx.contains("launch?.frontend_runtime_online || launch?.debug_port_online"));
    assert!(app_tsx.contains("CDP 离线"));
    assert!(app_tsx.contains("后端在线"));
    assert!(app_tsx.contains("汉化已注入"));
    assert!(app_tsx.contains("claudeOverviewStatus(claudeDesktop, claudeZhPatch)"));
    assert!(!app_tsx.contains("前端已注入"));
    assert!(!app_tsx.contains("包装窗口已注入"));
    assert!(!app_tsx.contains("claudeOverviewStatus(claudeDesktop, claudeZhPatch, claudeChinese)"));
    assert!(!app_tsx.contains("前端未注入"));
    assert!(app_tsx.contains("Inspector 在线"));
    assert!(!app_tsx.contains("CDP 未检测"));
    assert!(!app_tsx.contains("CDP 受阻"));
    assert!(!app_tsx.contains("调试受限"));
    assert!(!app_tsx.contains("cdpStatus === \"blocked\" ? \"CDP 受阻\""));
    assert!(app_tsx.contains("const cdpWarn = !inspectorReady && cdpStatus === \"failed\""));
    assert!(app_tsx.contains("注入异常"));
    assert!(!app_tsx.contains("inject ok"));
    assert!(!app_tsx.contains("FE on"));
    assert!(!app_tsx.contains("BE on"));
    assert!(!app_tsx.contains("Codex 运行"));
    let overview_screen = screens_file
        .split("function OverviewScreen")
        .nth(1)
        .and_then(|rest| rest.split("function SupplierScreen").next())
        .expect("overview screen source");
    assert!(!overview_screen.contains("relay-banner"));
    assert!(!overview_screen.contains("官方中转站"));
    assert!(!overview_screen.contains("拓扑熵减API"));
    assert!(!overview_screen.contains("https://api.toporeduce.cn"));
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
    assert!(memory_panel.contains("查看/编辑经验教训"));
    assert!(memory_panel.contains("提炼经验教训"));
    assert!(memory_panel.contains("memory-overview-matrix"));
    assert!(memory_panel.contains("memory-overview-actions"));
    assert!(!memory_panel.contains("待确认"));
    assert!(memory_panel.contains("actions.refineLongTermMemory()"));
    assert!(memory_panel.contains("openMemoryDetails()"));
    assert!(overview_screen.contains("overview-side-stack"));
    assert!(overview_screen.contains("<OverviewMemoryDetails"));
    assert!(app_tsx.contains("function OverviewMemoryDetails"));
    assert!(app_tsx.contains("经验教训手册详情"));
    assert!(app_tsx.contains("提炼结果会合成为一条精简手册，可在这里直接查看和编辑。"));
    assert!(app_tsx.contains("const refineLongTermMemory = async () => {"));
    assert!(app_tsx.contains("\"提炼经验教训\""));
    assert!(app_tsx.contains(
        "正在使用 Codex 本地 SQLite、rollout 会话文件和 memory_assist.sqlite 遍历工作区与会话"
    ));
    assert!(app_tsx.contains("writeUiEvent(\"memory.refine_long_term.click\""));
    assert!(
        app_tsx.contains("function memoryRefineSummary(result: MemorySelfCheckResult): string")
    );
    assert!(app_tsx.contains("check.name === \"history\""));
    assert!(app_tsx.contains("结果：${historyMessage}"));
    assert!(app_tsx.contains("call<MemorySelfCheckResult>(\"run_memory_assist_selfcheck\", { request: { repair: true } })"));
    assert!(app_tsx.contains("writeUiEvent(\"manager.ui.action.start\""));
    assert!(app_tsx.contains("writeUiEvent(\"manager.ui.action.result\""));
    assert!(app_tsx.contains("writeUiEvent(\"manager.ui.action.failed\""));
    assert!(app_tsx.contains("writeUiEvent(\"manager.ui.button.click\""));
    assert!(app_tsx.contains("function buttonLogLabel(button: HTMLButtonElement): string"));
    assert!(app_tsx.contains("document.addEventListener(\"click\", handleButtonClick, true)"));
    assert!(commands_rs.contains("manager.memory.selfcheck.start"));
    assert!(commands_rs.contains("manager.memory.selfcheck.result"));
    assert!(commands_rs.contains("manager.memory.selfcheck.failed"));
    assert!(commands_rs.contains("\"historyScan\": \"all_visible_workspaces_and_sessions\""));
    assert!(styles.contains(".overview-side-stack"));
    assert!(styles.contains(".overview-memory-list"));
    assert!(styles.contains(".memory-overview-matrix"));
    assert!(styles.contains("grid-template-columns: repeat(2, minmax(0, 1fr));"));
    assert!(styles.contains(".memory-overview-actions"));
    assert!(styles.contains("grid-template-columns: repeat(3, minmax(0, 1fr));"));
    assert!(styles.contains("max-height: 360px;"));
    assert!(!overview_screen.contains("插件中心"));
    assert!(!overview_screen.contains("提示词工坊"));
    assert!(!overview_screen.contains("PromptOptimizerCard"));
    assert!(overview_screen.contains("刷新 Claude 第三方配置"));
    assert!(overview_screen.contains("修复前端连接"));
    assert!(overview_screen.contains("修复后端服务"));
    assert!(app_tsx.contains("refresh_claude_third_party_config"));
    assert!(app_tsx.contains("repair_frontend_connection"));
    assert!(app_tsx.contains("repair_backend_service"));
    assert!(app_tsx.contains("codexFrontendInjected"));
    assert!(!app_tsx.contains("claudeFrontendInjected"));
    assert!(app_tsx.contains("codexBackendOnline"));
    assert!(app_tsx.contains("claudeBackendOnline"));
    assert!(app_tsx.contains("launchStatus === \"degraded\""));
    assert!(lib_rs.contains("commands::refresh_claude_third_party_config"));
    assert!(lib_rs.contains("commands::repair_frontend_connection"));
    assert!(lib_rs.contains("commands::repair_backend_service"));
    assert!(lib_rs.contains("commands::update_memory_assist_item"));
    assert!(app_tsx.contains("\"update_memory_assist_item\""));
    assert!(tauri_bridge.contains("command === \"update_memory_assist_item\""));
    let memory_assist_panel = screens_file
        .split("function MemoryAssistPanel")
        .nth(1)
        .and_then(|rest| rest.split("function SessionManagementScreen").next())
        .expect("memory assist panel source");
    assert!(memory_assist_panel.contains("<strong>经验教训手册</strong>"));
    assert!(memory_assist_panel.contains("memory-lesson-card"));
    assert!(!memory_assist_panel.contains("<strong>待确认</strong>"));
    assert!(!memory_assist_panel.contains("approveMemoryAssistCandidate(candidate.id)"));
    assert!(!memory_assist_panel.contains("rejectMemoryAssistCandidate(candidate.id)"));
    assert!(memory_assist_panel.contains("const allItems = items?.items ?? [];"));
    assert!(!memory_assist_panel.contains("items?.items.slice(0, 5)"));
    assert!(memory_assist_panel.contains("beginEditMemory"));
    assert!(memory_assist_panel.contains("saveEditedMemory"));
    assert!(memory_assist_panel.contains("actions.updateMemoryAssistItem"));
    assert!(memory_assist_panel.contains("actions.refineLongTermMemory()"));
    assert!(memory_assist_panel.contains("workspace: item.workspace"));
    assert!(memory_assist_panel.contains("tags: item.tags"));
    assert!(memory_assist_panel.contains("sourceSessionId: item.sourceSessionId"));
    assert!(commands_rs.contains("codex_frontend_injected"));
    assert!(!commands_rs.contains("claude_frontend_injected"));
    assert!(commands_rs.contains("codex_backend_online"));
    assert!(commands_rs.contains("claude_backend_online"));
    assert!(commands_rs.contains("frontend_runtime_online"));
    assert!(commands_rs.contains("latest_renderer_runtime_heartbeat"));
    assert!(commands_rs.contains("renderer_heartbeat_is_fresh"));
    assert!(commands_rs.contains("renderer_frontend_heartbeat_confirms_injection"));
    assert!(commands_rs.contains("heartbeat.runtime_reported"));
    assert!(commands_rs.contains("renderer_heartbeat_is_fresh(heartbeat.timestamp_ms)"));
    assert!(commands_rs.contains(".map(|runtime| runtime.status != \"failed\")"));
    assert!(commands_rs.contains("renderer.memory_runtime"));
    assert!(commands_rs.contains("renderer.script_loaded"));
    assert!(commands_rs.contains("#[serde(default)]"));
    assert!(commands_rs.contains("fn normalize_memory_runtime_status"));
    assert!(commands_rs.contains("\"idle\" => \"ok\".to_string()"));
    assert!(commands_rs.contains(".take(2_000)"));
    assert!(commands_rs.contains("Codex 前端脚本已注入，正在等待盘古记忆运行时同步。"));
    assert!(commands_rs.contains("等待真实对话消息后写入盘古记忆。"));
    assert!(app_tsx.contains("status === \"idle\""));
    assert!(commands_rs.contains("force_reinject_bridge"));
    assert!(commands_rs.contains("stop_launcher_processes_for_codex_restart"));
    assert!(launcher_main.contains("MemoryAssistStore"));
    assert!(launcher_main.contains("MemoryCaptureRequest"));
    assert!(launcher_main.contains("async fn memory_session(&self, payload: Value)"));
    assert!(launcher_main.contains("self.memory_store.session_summary(request)"));
    assert!(launcher_main.contains("async fn memory_capture(&self, payload: Value)"));
    assert!(launcher_main.contains("self.memory_store.record_capture(request)"));
    assert!(launcher_main.contains("async fn memory_resolve_workspace(&self, payload: Value)"));
    assert!(launcher_main.contains("resolve_codex_memory_workspace_response"));
    assert!(launcher_main.contains("async fn memory_status(&self)"));
    assert!(!commands_rs.contains("repair_claude_frontend_via_node_inspector"));
    assert!(!commands_rs.contains("repair_claude_frontend_via_wrapped_window"));
    assert!(!commands_rs.contains("open_claude_chinese_window(app.clone()).await"));
    assert!(commands_rs.contains("Claude 汉化窗口已打开。"));
    assert!(commands_rs.contains("pub async fn repair_frontend_connection("));
    assert!(commands_rs.contains("pub async fn repair_frontend_connection()"));
    assert!(!commands_rs.contains("BrowserWindow.getAllWindows"));
    assert!(commands_rs.contains("inspector_ports: status.inspector_ports"));
    assert!(commands_rs.contains("本地模型代理启动失败"));
    assert!(commands_rs.contains("if !status.debug_port_online"));
    assert!(commands_rs.contains("Codex CDP 端口 127.0.0.1:{debug_port}"));
    assert!(commands_rs.contains("强制刷新超时"));
    assert!(!commands_rs.contains("codex_frontend_ok && claude_probe.injected"));
    assert!(commands_rs.contains("codex_helper && claude_helper"));
    assert!(commands_rs.contains("\"degraded\""));
    let overview_matrix = overview_screen
        .split("<div className=\"ops-matrix\">")
        .nth(1)
        .and_then(|rest| rest.split("</div>").next())
        .expect("overview matrix source");
    assert!(!overview_matrix.contains("actions.installClaudeZhPatch()"));
    assert!(overview_matrix.contains("items={codexStatus.items}"));
    assert!(!overview_matrix.contains("Codex 版本"));
    assert!(overview_matrix.contains("items={claudeStatus.items}"));
    assert!(overview_matrix.contains("items={memoryStatus.items}"));
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
    assert!(styles.contains(".status-tile"));
    assert!(styles.contains(".status-segment-list"));
    assert!(styles.contains(".status-segment.ok"));
    assert!(styles.contains(".status-segment.warn"));
    assert!(styles.contains(".status-segment.muted"));
    assert!(styles.contains(".memory-overview-header"));
    assert!(styles.contains(".memory-activity-wave"));
    assert!(styles.contains(".memory-activity-wave[data-active=\"true\"]"));
    assert!(styles.contains(".toast-wrap"));
    assert!(styles.contains(".status-action-tile .status-segment-list"));
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
fn claude_launch_waits_for_real_debug_readiness() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let core_claude = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/claude-codex-pro-core/src/claude_desktop.rs");
    let core_claude = std::fs::read_to_string(&core_claude).expect("read core claude desktop");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    let open_section = core_claude
        .split("pub fn open_claude_desktop() -> ClaudeDesktopActionResult")
        .nth(1)
        .and_then(|rest| rest.split("pub fn enter_claude_desktop_devtools").next())
        .expect("open_claude_desktop source");

    assert!(open_section.contains("wait_for_claude_launch_readiness"));
    assert!(open_section.contains("\"warning\""));
    assert!(open_section.contains("Claude Desktop 已启动"));
    assert!(core_claude.contains("struct LaunchReadiness"));
    assert!(core_claude.contains("fn launch_readiness_from_status"));
    // The Node inspector debug port (`--inspect=127.0.0.1:9229`) was removed from
    // the launch command: it exposed an unauthenticated inspector any local
    // process could attach to and run code inside Claude's main process, and
    // nothing here ever connected to it. Readiness now keys off a running
    // process, not a debug channel. (The `extract_node_inspector_port` parser and
    // its test data still mention the flag on purpose — that only reads a port out
    // of an observed command line for diagnostics; it never passes the flag.)
    let launch_section = core_claude
        .split("fn launch_claude_desktop_app")
        .nth(1)
        .and_then(|rest| rest.split("fn claude_desktop_executable_path").next())
        .expect("launch_claude_desktop_app source");
    // The flag must not be PASSED as a launch argument. An explanatory comment in
    // the function may still name it, so we assert on the argument-passing form
    // (`args([... --inspect ...])`) rather than any mention of the string.
    assert!(!launch_section.contains("args([\"--inspect"));
    assert!(!launch_section.contains("--inspect=127.0.0.1:9229\"]"));
    assert!(core_claude.contains("fn launch_readiness_warns_when_no_process_is_running"));
    assert!(
        core_claude
            .contains("fn launch_readiness_is_ready_when_process_is_running_without_debug_port")
    );
    assert!(commands_rs.contains("\"warning\".to_string()"));
    assert!(commands_rs.contains("本地模型代理已请求启动"));
}

#[test]
fn supplier_editor_generates_config_from_editable_supplier_id() {
    // buildSupplierConfigToml / tomlString 已抽到 lib/supplier.ts，切片读该文件。
    let app_tsx = read_frontend_file("lib/supplier.ts");

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
    let app_tsx = read_all_frontend_sources().replace("\r\n", "\n");

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
        "if (target === \"overview\") {\n      await Promise.all([refreshOverview(true), refreshClaudeLight(true), refreshClaudeDesktopDevMode(true), refreshSettings(true)]);"
    ));
    assert!(app_tsx.contains("const devModeValue = claudeDevModeBusy ? \"写入中...\" : devModeConfigured ? \"已写入\" : \"写入开发配置\";"));
    assert!(
        app_tsx.contains(
            "afterFirstPaintIfFresh(() => {\n        void refreshMemoryAssistStatus(true);"
        )
    );
    assert!(
        app_tsx
            .contains("afterFirstPaintIfFresh(() => {\n        void refreshClaudeZhPatch(true);")
    );
    assert!(app_tsx.contains("useEffect(() => {\n    void refreshRoute(route);\n  }, [route]);"));
    assert!(!app_tsx.contains(
        "useEffect(() => {\n    void (async () => {\n      await Promise.all([\n        refreshOverview(true),\n        refreshClaude(true),\n        refreshSettings(true),\n        refreshPluginHub(true),"
    ));
}

#[test]
fn codex_restart_passes_detected_app_path_and_uses_non_claude_debug_port() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = read_all_frontend_sources();
    let commands_rs =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands.rs"))
            .expect("read manager commands.rs");

    assert!(app_tsx.contains("function codexLaunchRequestFromOverview"));
    assert!(app_tsx.contains("overview?.codex_app.path || overview?.latest_launch?.codex_app"));
    assert!(app_tsx.contains("restart_claude_codex_pro\", { request }"));
    assert!(app_tsx.contains("launch_claude_codex_pro\", { request }"));
    assert!(commands_rs.contains("fn normalize_launch_request"));
    assert!(commands_rs.contains("fn codex_launch_app_path_from_candidate"));
    assert!(commands_rs.contains("codex_launch_app_path_from_candidate(Path::new(&requested))"));
    assert!(commands_rs.contains("\"manager.launch_path_stale\""));
    assert!(commands_rs.contains("build_codex_executable(&normalized)"));
    assert!(commands_rs.contains("find_running_codex_app_dir()"));
    assert!(commands_rs.contains("current_codex_app_path_for_launch"));
    let launch_path_helper = commands_rs
        .split("fn current_codex_app_path_for_launch")
        .nth(1)
        .and_then(|rest| rest.split("fn spawn_claude_codex_pro_launch").next())
        .expect("current codex app path helper source");
    assert!(
        launch_path_helper
            .find("find_running_codex_app_dir()")
            .expect("running codex path lookup")
            < launch_path_helper
                .find("StatusStore::default()")
                .expect("latest status path lookup")
    );
    let restart_command = commands_rs
        .split("pub async fn restart_claude_codex_pro")
        .nth(1)
        .and_then(|rest| rest.split("fn normalize_launch_request").next())
        .expect("restart_claude_codex_pro source");
    assert!(restart_command.contains("stop_launcher_processes_for_codex_restart()"));
    assert!(!restart_command.contains("stop_launcher_processes();"));
    assert!(restart_command.contains("stop_codex_processes();"));
    // rustfmt may wrap the call across lines, so assert on the call and the
    // forwarded `request` argument separately rather than on a single substring.
    assert!(restart_command.contains("spawn_claude_codex_pro_launch("));
    assert!(restart_command.contains("request,"));
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
    assert!(core_claude.contains(".filter(|path| path.is_file())"));
    assert!(core_claude.contains(".or_else(claude_desktop_executable_path)"));
    assert!(core_claude.contains("let mut seen = std::collections::HashSet::new();"));
    assert!(core_claude.contains("paths.sort_by(|left, right| right.cmp(left));"));
    assert!(core_claude.contains("Claude Desktop 已关闭并重新启动。"));
    assert!(
        core_claude.contains("action: if is_restart { \"restart\" } else { \"open\" }.to_string()")
    );
    assert!(!core_claude.contains("Claude Desktop is already running and was focused."));
}

#[test]
fn supplier_screen_exposes_real_provider_crud_and_switching() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // 存在性断言读前端源码全集；结构化切片读 screens.tsx（SupplierScreen 已拆分到 screens.tsx）。
    let app_tsx = read_all_frontend_sources().replace("\r\n", "\n");
    let screens_file = read_frontend_file("screens.tsx").replace("\r\n", "\n");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");

    let supplier_screen = screens_file
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
    assert!(
        supplier_screen
            .contains("!normalized.name.trim() || (!aggregateDraft && !normalized.baseUrl.trim())")
    );
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
    assert!(app_tsx.contains("const apiKey = supplierProfileResolvedApiKey(profile);"));
    assert!(app_tsx.contains("function supplierProfileResolvedApiKey(profile: RelayProfile)"));
    assert!(app_tsx.contains("function supplierApiKeyFromAuthContents(contents: string)"));
    assert!(app_tsx.contains("function supplierApiKeyFromConfigContents(contents: string)"));
    assert!(app_tsx.contains("supplierProfileHasApiKey(targetProfile)"));
    assert!(app_tsx.contains("supplierProfileHasApiKey(savedProfile)"));
    assert!(app_tsx.contains("function supplierProfileIsCcswitch(profile: RelayProfile)"));
    assert!(app_tsx.contains(
        "const importedById = new Map(imported.map((profile) => [profile.id, profile]));"
    ));
    assert!(app_tsx.contains("if (importedProfile && supplierProfileIsCcswitch(profile))"));
    assert!(app_tsx.contains("importedById.delete(profile.id);"));
    assert!(app_tsx.contains("id: uniqueSupplierProfileId(nextProfiles, profile.id)"));
    assert!(
        app_tsx
            .contains("已从 cc-switch 更新 ${updatedCount} 个、新增 ${addedCount} 个供应商配置。")
    );
    assert!(!app_tsx.contains("const nextImported = imported.map((profile) => {"));
    assert!(!app_tsx.contains("const normalized = normalizeSupplierProfile({\n    ...profile,\n    configContents: \"\",\n    authContents: \"\",\n  });"));
    assert!(!app_tsx.contains("targetProfile && !targetProfile.apiKey.trim()"));
    assert!(!app_tsx.contains("if (!savedProfile.apiKey.trim())"));
    assert!(app_tsx.contains("configContents: profile.configContents ?? \"\""));
    assert!(app_tsx.contains("authContents: profile.authContents ?? \"\""));
    assert!(styles.contains(".supplier-card"));
    assert!(styles.contains(".supplier-editor-layout"));
    assert!(styles.contains(".supplier-preset-strip"));
}

#[test]
fn supplier_screen_matches_ccswitch_style_layout_and_drag_sorting() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // 存在性断言读前端源码全集（聚合策略标签等已迁到 constants.ts 仍能命中）；
    // 结构化切片读 App.tsx 单文件（SupplierScreen 仍在 App.tsx 内）。
    let app_tsx = read_all_frontend_sources();
    // SupplierScreen 已随 Screen 组件抽到 screens.tsx，结构化切片读该文件。
    let screens_file = read_frontend_file("screens.tsx").replace("\r\n", "\n");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = std::fs::read_to_string(&styles).expect("read manager styles.css");
    let commands_rs =
        std::fs::read_to_string(manifest_dir.join("src/commands.rs")).expect("read commands.rs");
    let lib_rs = std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs");

    let supplier_screen = screens_file
        .split("function SupplierScreen")
        .nth(1)
        .and_then(|rest| rest.split("function LegacySupplierScreen").next())
        .expect("supplier screen source");

    assert!(supplier_screen.contains("供应商配置"));
    assert!(!supplier_screen.contains("管理 API 供应商、协议、Key 与配置文件"));
    assert!(!supplier_screen.contains("供应商列表"));
    assert!(!supplier_screen.contains("Claude Desktop 3P 状态"));
    assert!(!supplier_screen.contains("当前配置摘要"));
    assert!(supplier_screen.contains("检测到 OPENAI 环境变量"));
    assert!(supplier_screen.contains("启用供应商配置切换"));
    assert!(supplier_screen.contains("添加供应商"));
    assert!(supplier_screen.contains("添加聚合供应商"));
    assert!(supplier_screen.contains("从第三方导入"));
    assert!(supplier_screen.contains("ccswitch"));
    assert!(supplier_screen.contains("<Pencil className=\"h-4 w-4 tilted-pen-icon\" />"));
    assert!(!supplier_screen.contains("<PencilRuler className=\"h-4 w-4\" />"));
    assert!(supplier_screen.contains("发现"));
    assert!(supplier_screen.contains("刷新列表"));
    assert!(supplier_screen.contains("onDragStart"));
    assert!(supplier_screen.contains("onDragEnter"));
    assert!(supplier_screen.contains("onDragOver"));
    assert!(supplier_screen.contains("onDrop"));
    assert!(supplier_screen.contains("SUPPLIER_DRAG_MIME_TYPE"));
    assert!(
        supplier_screen.contains("event.dataTransfer.setData(SUPPLIER_DRAG_MIME_TYPE, profileId);")
    );
    assert!(supplier_screen.contains("event.dataTransfer.setData(\"text/plain\", profileId);"));
    assert!(supplier_screen.contains("event.dataTransfer.dropEffect = \"move\";"));
    assert!(supplier_screen.contains("event.dataTransfer.getData(SUPPLIER_DRAG_MIME_TYPE)"));
    assert!(supplier_screen.contains(
        "const beginSupplierDrag = (event: DragEvent<HTMLElement>, profileId: string) => {"
    ));
    assert!(
        supplier_screen.contains("const supplierDragSourceId = (event: DragEvent<HTMLElement>) =>")
    );
    assert!(
        supplier_screen
            .contains("const [supplierOrderIds, setSupplierOrderIds] = useState<string[]>([]);")
    );
    assert!(supplier_screen.contains("useEffect(() => {\n    setSupplierOrderIds(profiles.map((profile) => profile.id));\n  }, [profileIdsKey]);"));
    assert!(supplier_screen.contains("const supplierOrderFromIds = (ids: string[]) => {"));
    assert!(supplier_screen.contains("const reorderSupplierIds = (sourceId: string, targetId: string, ids = supplierOrderIds) => {"));
    assert!(
        supplier_screen
            .contains("const previewSupplierOrder = (sourceId: string, targetId: string) => {")
    );
    assert!(supplier_screen.contains("setSupplierOrderIds((current) => reorderSupplierIds(sourceId, targetId, current) ?? current);"));
    assert!(supplier_screen.contains("saveSupplierOrder"));
    assert!(
        supplier_screen
            .contains("saveSupplierSettings({ ...appSettings, relayProfiles: reordered })")
    );
    assert!(
        supplier_screen
            .contains("setSupplierOrderIds(saved.relayProfiles.map((profile) => profile.id));")
    );
    assert!(supplier_screen.contains("setSupplierOrderIds(previousIds);"));
    assert!(supplier_screen.contains("供应商顺序保存失败，已恢复原顺序。"));
    assert!(
        supplier_screen.contains(
            "const orderedProfiles = useMemo(() => supplierOrderFromIds(supplierOrderIds)"
        )
    );
    assert!(supplier_screen.contains("orderedProfiles.map((profile) => {"));
    assert!(
        supplier_screen.contains("onDragStart={(event) => beginSupplierDrag(event, profile.id)}")
    );
    assert!(
        supplier_screen.contains("onDragOver={(event) => previewSupplierDrag(event, profile.id)}")
    );
    assert!(supplier_screen.contains("title=\"拖拽排序\""));

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
    assert!(codex_inject.contains("window.__claudeCodexProMemoryAssistRuntime"));
    assert!(codex_inject.contains("function codexMemoryExposeRuntime"));
    assert!(codex_inject.contains("function codexMemoryPulseActivity"));
    assert!(codex_inject.contains("sendClaudeCodexProDiagnostic(\"memory_runtime\""));
    assert!(codex_inject.contains("__claudeCodexProMemoryHeartbeatTimer"));
    assert!(codex_inject.contains("window.setInterval(() =>"));
    assert!(codex_inject.contains("activeUntil"));
    assert!(codex_inject.contains("data-active"));
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
    // 存在性断言读前端源码全集（JSX 调用随 Screen 迁到 screens.tsx）；
    // primary/manual 动作切片锚定 App() 内部动作定义，仍在 App.tsx。
    let app_tsx = read_all_frontend_sources();
    let app_tsx_file = read_frontend_file("App.tsx");
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    let primary_start = app_tsx_file
        .find("const installClaudeZhPatch = async () => {")
        .expect("primary zh patch action");
    let manual_start = app_tsx_file
        .find("const installClaudeZhPatchFromDirectory = async () => {")
        .expect("manual zh patch action");
    let primary_action = &app_tsx_file[primary_start..manual_start];
    let manual_action = &app_tsx_file[manual_start
        ..app_tsx_file[manual_start..]
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
    let app_tsx = read_all_frontend_sources();

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
    assert!(core_zh_patch.contains("i18n_resources_for_install().await"));
    assert!(core_zh_patch.contains("_ => embedded_i18n_resources()"));
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
fn claude_zh_patch_javascript_validation_runs_node_without_console_window() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
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
    let validation = core_zh_patch
        .split("fn validate_patched_javascript_chunk")
        .nth(1)
        .and_then(|rest| rest.split("fn write_patch_file").next())
        .expect("javascript validation source");

    assert!(validation.contains("Command::new(\"node\")"));
    assert!(validation.contains("command.stdin(std::process::Stdio::null())"));
    assert!(validation.contains("command.stdout(std::process::Stdio::null())"));
    assert!(validation.contains("command.stderr(std::process::Stdio::null())"));
    assert!(validation.contains("command.creation_flags(crate::windows_create_no_window())"));
}

#[test]
fn plugin_hub_and_worktree_git_spawns_suppress_console_window() {
    // 回归护栏：Windows 下所有会 spawn 外部程序（git clone/pull、npm install、
    // 官方插件安装、上游 worktree 的 git 调用）的代码路径都必须带 CREATE_NO_WINDOW，
    // 否则每次调用都会闪出黑色终端窗并抢焦点，进而打断供应商列表的拖拽排序。
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let core_dir = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/claude-codex-pro-core/src");

    let plugin_hub =
        std::fs::read_to_string(core_dir.join("plugin_hub.rs")).expect("read core plugin_hub.rs");
    // run_command 是 git clone/pull 与 npm install 的公共出口。
    let run_command = plugin_hub
        .split("fn run_command(command: &[String])")
        .nth(1)
        .and_then(|rest| rest.split("\nfn ").next())
        .expect("run_command source");
    assert!(run_command.contains("command.iter().skip(1)"));
    assert!(run_command.contains("child.creation_flags(crate::windows_create_no_window())"));

    // 官方 Claude 插件安装单独走一段 spawn，也必须隐藏窗口。
    let official_install = plugin_hub
        .split("fn install_official_claude_plugin(")
        .nth(1)
        .and_then(|rest| rest.split("\nfn ").next())
        .expect("install_official_claude_plugin source");
    assert!(official_install.contains("child.creation_flags(crate::windows_create_no_window())"));

    // 上游 worktree 的 git 调用统一走 git_command()，且该 helper 带隐藏窗口标志。
    let worktree_git = std::fs::read_to_string(core_dir.join("upstream_worktree/git.rs"))
        .expect("read upstream_worktree/git.rs");
    let git_command = worktree_git
        .split("fn git_command() -> Command")
        .nth(1)
        .and_then(|rest| rest.split("\npub fn ").next())
        .expect("git_command source");
    assert!(git_command.contains("Command::new(\"git\")"));
    assert!(git_command.contains("command.creation_flags(crate::windows_create_no_window())"));
    // 不再有裸的 Command::new("git")（除 helper 自身那一处）。
    assert_eq!(worktree_git.matches("Command::new(\"git\")").count(), 1);
    assert_eq!(worktree_git.matches("git_command()").count(), 4);
}

#[test]
fn claude_zh_patch_remote_i18n_fetch_has_short_embedded_fallback() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
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

    assert!(core_zh_patch.contains("const REMOTE_I18N_FETCH_TIMEOUT"));
    assert!(core_zh_patch.contains("Duration::from_secs(2)"));
    assert!(core_zh_patch.contains("async fn i18n_resources_for_install()"));
    assert!(core_zh_patch.contains("tokio::time::timeout"));
    assert!(core_zh_patch.contains("_ => embedded_i18n_resources()"));
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
            rest.split("#[tauri::command]\npub async fn restore_claude_zh_patch")
                .next()
        })
        .expect("manual command source");
    let restore_action = commands_rs
        .split("fn restore_claude_zh_patch_blocking()")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[tauri::command]\npub fn new_claude_desktop_chat")
                .next()
        })
        .expect("restore command source");

    assert!(install_action.contains("close_claude_desktop_for_patch()"));
    assert!(install_action.contains("打补丁前关闭 Claude Desktop 失败"));
    assert!(manual_action.contains("close_claude_desktop_for_patch()"));
    assert!(manual_action.contains("手动打补丁前关闭 Claude Desktop 失败"));
    assert!(restore_action.contains("close_claude_desktop_for_patch()"));
    assert!(restore_action.contains("还原前关闭 Claude Desktop 失败"));
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
            rest.split("#[tauri::command]\npub async fn restore_claude_zh_patch")
                .next()
        })
        .expect("manual command source");
    let restore_action = commands_rs
        .split("fn restore_claude_zh_patch_blocking()")
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
        .split("fn restore_claude_zh_patch_blocking()")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[tauri::command]\npub fn new_claude_desktop_chat")
                .next()
        })
        .expect("restore command source");

    assert!(install_action.contains("if status.status != \"ok\""));
    assert!(install_action.contains("提权运行未完成"));
    assert!(restore_action.contains("if status.status != \"not_installed\""));
    assert!(restore_action.contains("仍残留汉化文件"));
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
            rest.split("#[tauri::command]\npub async fn restore_claude_zh_patch")
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
    // The elevated child's stdout/stderr must be drained on dedicated threads
    // (read_to_end) rather than read only after exit: a child that writes past
    // the ~64 KiB pipe buffer would otherwise deadlock until the timeout. Assert
    // the deadlock-safe draining instead of the old post-exit wait_with_output().
    assert!(commands_rs.contains("pipe.read_to_end(&mut buf)"));
    assert!(commands_rs.contains("std::thread::spawn(move ||"));
    assert!(commands_rs.contains("stdout={}"));
    assert!(commands_rs.contains("stderr={}"));
    assert!(commands_rs.contains("提权子进程未写入结果文件"));
    assert!(commands_rs.contains("child.kill()"));
    assert!(commands_rs.contains("Claude 汉化补丁提权执行超时"));
}

#[test]
fn frontend_connection_repair_forces_codex_restart_and_requires_new_heartbeat() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    let repair = commands_rs
        .split("pub async fn repair_frontend_connection()")
        .nth(1)
        .and_then(|rest| {
            rest.split("async fn restart_codex_for_frontend_repair")
                .next()
        })
        .expect("repair_frontend_connection source");
    assert!(repair.contains("let repair_started_ms = current_time_ms();"));
    assert!(repair.contains("restart_codex_for_frontend_repair(&mut details).await"));
    assert!(!repair.contains("let initial_runtime_online"));
    assert!(repair.contains("wait_for_renderer_frontend_after("));
    assert!(repair.contains("前端脚本已在本次修复后加载"));
    assert!(repair.contains("旧注入状态不会被判定为成功"));
    assert!(
        commands_rs
            .contains("const REPAIR_CODEX_RESTART_TIMEOUT: Duration = Duration::from_secs(90);")
    );
    assert!(
        commands_rs
            .contains("const REPAIR_CODEX_FRONTEND_TIMEOUT: Duration = Duration::from_secs(45);")
    );

    let restart = commands_rs
        .split("async fn restart_codex_for_frontend_repair")
        .nth(1)
        .and_then(|rest| {
            rest.split("async fn wait_for_renderer_runtime_after")
                .next()
        })
        .expect("restart_codex_for_frontend_repair source");
    assert!(restart.contains("stop_launcher_processes_for_codex_restart()"));
    assert!(restart.contains("stop_codex_processes()"));
    assert!(restart.contains("tokio::time::sleep(Duration::from_millis(800)).await"));
    assert!(
        restart
            .contains("wait_for_codex_launch_ports(&request, REPAIR_CODEX_RESTART_TIMEOUT).await")
    );
    assert!(restart.contains("正在等待 Codex 自启完成"));

    let wait_ports = commands_rs
        .split("async fn wait_for_codex_launch_ports")
        .nth(1)
        .and_then(|rest| {
            rest.split("async fn wait_for_renderer_frontend_after")
                .next()
        })
        .expect("wait_for_codex_launch_ports source");
    assert!(wait_ports.contains("status.debug_port_online && status.helper_port_online"));
    assert!(wait_ports.contains("codex_debug_port_online(request.debug_port)"));
    assert!(wait_ports.contains("helper_backend_online(request.helper_port)"));

    let wait = commands_rs
        .split("async fn wait_for_renderer_frontend_after")
        .nth(1)
        .expect("wait_for_renderer_frontend_after source");
    assert!(wait.contains("heartbeat.timestamp_ms >= min_timestamp_ms"));
    assert!(wait.contains("renderer_frontend_heartbeat_confirms_injection(&heartbeat)"));
}

#[test]
fn settings_and_tools_route_keep_full_ops_controls() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // 存在性断言读前端源码全集；结构化切片读 screens.tsx（SettingsScreen 已拆到 screens.tsx）。
    let app_tsx = read_all_frontend_sources();
    let screens_file = read_frontend_file("screens.tsx");
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
    let settings_screen = screens_file
        .split("function SettingsScreen")
        .nth(1)
        .and_then(|rest| rest.split("function AboutScreen").next())
        .expect("settings screen source");
    let zh_settings_panel = screens_file
        .split("<Panel title=\"Claude 一键汉化\"")
        .nth(1)
        .and_then(|rest| rest.split("<Panel title=\"CLI 命令包装器\"").next())
        .expect("Claude zh settings panel source");
    assert!(zh_settings_panel.contains("安装类型"));
    assert!(zh_settings_panel.contains("目录可写"));
    assert!(zh_settings_panel.contains("诊断日志"));
    assert!(zh_settings_panel.contains("桌面资源"));
    assert!(zh_settings_panel.contains("前端资源"));
    assert!(zh_settings_panel.contains("Statsig 资源"));
    assert!(!zh_settings_panel.contains("入口 URL"));
    assert!(!zh_settings_panel.contains("wrapped_webview"));
    assert!(app_tsx.contains("CLI 命令包装器"));
    assert!(settings_screen.contains("<LogsScreen actions={actions} logs={logs} />"));
    assert!(!settings_screen.contains("<Panel title=\"Codex 启动参数\""));
    assert!(!settings_screen.contains("<Panel title=\"图片覆盖\""));
    assert!(!settings_screen.contains("<Panel title=\"盘古记忆\""));
    assert!(!settings_screen.contains("<Panel title=\"安全边界\""));
    assert!(!settings_screen.contains("保存启动参数"));
    assert!(!settings_screen.contains("保存图片覆盖"));
    assert!(!settings_screen.contains("保存盘古记忆设置"));
    assert!(!settings_screen.contains("重置图片覆盖"));
    assert!(!settings_screen.contains("重置设置"));
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
    let app_tsx = read_all_frontend_sources();

    assert!(vite_config.contains("base: \"./\""));
    assert!(app_tsx.contains("__CLAUDE_CODEX_PRO_INITIAL_ROUTE"));
    assert!(app_tsx.contains("window.location.search"));
}
