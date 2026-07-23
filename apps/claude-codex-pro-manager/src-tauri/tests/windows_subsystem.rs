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
                if let Ok(contents) = std::fs::read_to_string(&path).map(normalize_source) {
                    combined.push_str(&contents);
                    combined.push('\n');
                }
            }
        }
    }
    combined
}

/// 读取拆分后某个前端源文件的完整内容（相对 `src/`），用于结构化断言。
fn normalize_source(source: String) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

fn read_source_file(path: &std::path::Path) -> String {
    normalize_source(
        std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("read source file {}: {error}", path.display())),
    )
}

fn read_frontend_file(relative: &str) -> String {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.parent().unwrap().join("src").join(relative);
    read_source_file(&path)
}

fn source_section<'a>(source: &'a str, start_marker: &str, end_marker: &str) -> &'a str {
    let start = source
        .find(start_marker)
        .unwrap_or_else(|| panic!("missing source marker: {start_marker}"));
    let end = source[start..]
        .find(end_marker)
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("missing source marker: {end_marker}"));
    &source[start..end]
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
fn manager_startup_restores_claude_desktop_proxy_helper() {
    let lib = include_str!("../src/lib.rs");
    let commands = include_str!("../src/commands.rs");

    assert!(lib.contains("ensure_claude_desktop_proxy_on_startup"));
    assert!(lib.contains("tauri::async_runtime::spawn"));
    assert!(commands.contains("pub(crate) async fn ensure_claude_desktop_proxy_on_startup"));
    assert!(commands.contains("manager.claude_proxy.startup_ok"));
    assert!(commands.contains("manager.claude_proxy.startup_failed"));
}

#[test]
fn manager_syncs_live_codex_credentials_only_after_successful_provider_writes() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands = read_source_file(&manifest_dir.join("src/commands.rs"));
    let sync_call = "sync_codex_credential_environment_after_apply(&home)";
    assert_eq!(
        commands.matches(sync_call).count(),
        5,
        "only the five production provider-write success paths should synchronize credentials"
    );

    let switch = source_section(
        &commands,
        "fn switch_relay_profile_blocking(",
        "pub async fn preview_claude_desktop_provider(",
    );
    let relay = source_section(
        &commands,
        "fn apply_relay_injection_blocking()",
        "pub async fn apply_pure_api_injection()",
    );
    let pure_api = source_section(
        &commands,
        "fn apply_pure_api_injection_blocking()",
        "pub async fn clear_relay_injection()",
    );

    for (name, section, expected_calls) in [
        ("switch", switch, 1usize),
        ("relay apply", relay, 2usize),
        ("pure API apply", pure_api, 2usize),
    ] {
        assert_eq!(section.matches(sync_call).count(), expected_calls, "{name}");
        for (sync_index, _) in section.match_indices(sync_call) {
            let prefix = &section[..sync_index];
            let success_index = prefix
                .rfind("Ok(result) => {")
                .unwrap_or_else(|| panic!("{name} sync is not inside a successful write branch"));
            let failure_index = prefix.rfind("Err(error) => {").unwrap_or_default();
            assert!(
                success_index > failure_index,
                "{name} sync must not run from a failed write branch"
            );
        }
    }

    let helper = source_section(
        &commands,
        "fn sync_codex_credential_environment_after_apply(",
        "fn log_manager_event(",
    );
    assert!(helper.contains("sync_codex_user_credential_environment_from_home(home)?"));
    let payload_start = helper
        .find("json!({")
        .expect("credential synchronization log payload");
    let payload = &helper[payload_start..];
    for field in ["variableName", "userChanged", "processChanged"] {
        assert!(
            payload.contains(field),
            "missing redacted log field: {field}"
        );
    }
    for forbidden in ["apiKey", "api_key", "token", "secret", "credential"] {
        assert!(
            !payload.contains(forbidden),
            "credential synchronization log payload exposes forbidden field: {forbidden}"
        );
    }
}

#[test]
fn claude_desktop_default_mapping_does_not_reuse_stale_model_list_for_profile() {
    let commands = include_str!("../src/commands.rs");
    let protocol_proxy =
        include_str!("../../../../crates/claude-codex-pro-core/src/protocol_proxy.rs");

    assert!(commands.contains("profile.model_mapping_enabled"));
    assert!(commands.contains("configure_claude_desktop_supplier_with_proxy_port"));
    assert!(protocol_proxy.contains("Some(relay.model_mapping_enabled)"));
    assert!(protocol_proxy.contains("&relay.model_mapping_json"));
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
    let commands_rs = read_source_file(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands.rs"
    )));

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
    let commands_rs = read_source_file(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands.rs"
    )));
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
    let commands_rs = read_source_file(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands.rs"
    )));
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

fn assert_release_workflow_uses_current_hosted_runners(workflow: &str) {
    assert!(workflow.contains("runs-on: windows-latest"));
    assert!(workflow.contains("runner: macos-latest"));
    assert!(workflow.matches("runner: macos-latest").count() >= 2);
    assert!(workflow.contains("uses: actions/checkout@v5"));
    assert!(workflow.contains("uses: actions/setup-node@v5"));
    assert!(workflow.contains("node-version: \"24\""));
    for deprecated in [
        "windows-2025",
        "macos-15-intel",
        "macos-14",
        "macos-26-intel",
        "macos-26",
        "actions/checkout@v4",
        "actions/setup-node@v4",
        "actions/upload-artifact@v4",
        "node-version: \"22\"",
    ] {
        assert!(
            !workflow.contains(deprecated),
            "release workflow should not use deprecated/low-availability runner or action: {deprecated}"
        );
    }
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
        "create_app \"Claude Codex Pro Manager\" \"ClaudeCodexProManager\" \"$BINARY_DIR/claude-codex-pro-manager\" \"com.damonzs.claudecodexpro.manager\" \"false\""
    ));
}

#[test]
fn public_release_packages_do_not_include_user_supplier_or_memory_state() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .unwrap();
    let auto_workflow =
        std::fs::read_to_string(repo_root.join(".github/workflows/auto-release-installers.yml"))
            .expect("read auto release workflow");
    let manual_workflow =
        std::fs::read_to_string(repo_root.join(".github/workflows/release-assets.yml"))
            .expect("read release assets workflow");
    let windows_installer =
        std::fs::read_to_string(repo_root.join("scripts/installer/windows/ClaudeCodexPro.nsi"))
            .expect("read Windows installer");
    let macos_packager =
        std::fs::read_to_string(repo_root.join("scripts/installer/macos/package-dmg.sh"))
            .expect("read macOS packager");

    for release_source in [&auto_workflow, &manual_workflow] {
        assert!(release_source.contains("dist/windows/app/*"));
        assert!(release_source.contains("dist/macos/stage"));
        for forbidden in [
            "settings.json",
            "relayProfiles",
            "memory_assist.sqlite",
            "auth.json",
            "credentials",
            "OPENAI_API_KEY",
            "ANTHROPIC_API_KEY",
            "sk-",
            "%APPDATA%",
            "$HOME/.codex",
            "$HOME/.claude",
            "Library/Application Support",
        ] {
            assert!(
                !release_source.contains(forbidden),
                "release workflow must not package user data marker: {forbidden}"
            );
        }
    }

    assert!(windows_installer.contains(r#"File "${ROOT}\dist\windows\app\claude-codex-pro.exe""#));
    assert!(
        windows_installer
            .contains(r#"File "${ROOT}\dist\windows\app\claude-codex-pro-manager.exe""#)
    );
    assert!(windows_installer.contains("Claude Codex Pro Manager.lnk"));
    assert!(windows_installer.contains("Uninstall Claude Codex Pro.lnk"));
    for forbidden in [
        "settings.json",
        "relayProfiles",
        "memory_assist.sqlite",
        "auth.json",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "sk-",
    ] {
        assert!(!windows_installer.contains(forbidden));
        assert!(!macos_packager.contains(forbidden));
    }
    assert!(macos_packager.contains("create_app \"Claude Codex Pro Manager\""));
    assert!(macos_packager.contains("$BINARY_DIR/claude-codex-pro-manager"));
}

#[test]
fn windows_installer_registers_anonymous_installation_without_blocking_install() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .unwrap();
    let windows_installer =
        read_source_file(&repo_root.join("scripts/installer/windows/ClaudeCodexPro.nsi"));
    let launcher_main =
        read_source_file(&repo_root.join("apps/claude-codex-pro-launcher/src/main.rs"));

    let registration_command = concat!(
        "nsExec::ExecToLog '\"$INSTDIR\\claude-codex-pro.exe\" ",
        "--register-installation --app-version \"${VERSION}\"'"
    );
    assert!(windows_installer.contains(registration_command));
    let registration_position = windows_installer
        .find(registration_command)
        .expect("installation registration command");
    let uninstall_registration_position = windows_installer
        .find("WriteRegStr HKCU \"Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall")
        .expect("uninstall registration");
    let ignored_exit_code_position = windows_installer[registration_position..]
        .find("Pop $0")
        .map(|offset| registration_position + offset)
        .expect("registration exit code is discarded");
    let install_section_end = windows_installer[registration_position..]
        .find("SectionEnd")
        .map(|offset| registration_position + offset)
        .expect("install section end");
    assert!(uninstall_registration_position < registration_position);
    assert!(registration_position < ignored_exit_code_position);
    assert!(ignored_exit_code_position < install_section_end);

    assert!(launcher_main.contains("--register-installation"));
    assert!(launcher_main.contains("--app-version"));
    assert!(launcher_main.contains("install_registration::register_current_installation"));
    let main_body = launcher_main
        .split("async fn main() -> Result<()> {")
        .nth(1)
        .and_then(|rest| rest.split("fn installation_registration_version").next())
        .expect("launcher main body");
    let registration_branch = main_body
        .find("installation_registration_version(&args)")
        .expect("registration argument branch");
    let normal_launcher = main_body
        .find("run_launcher().await")
        .expect("normal launcher call");
    assert!(registration_branch < normal_launcher);
    assert!(!main_body[..registration_branch].contains("acquire_single_instance_guard"));

    for forbidden in [
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "Bearer ",
        "Win32_BaseBoard",
        "SerialNumber",
        "raw_serial",
    ] {
        assert!(!windows_installer.contains(forbidden));
    }
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

    assert_release_workflow_uses_current_hosted_runners(&workflow);
    assert!(workflow.contains("x86_64-apple-darwin"));
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
    let pr_build = std::fs::read_to_string(repo_root.join(".github/workflows/pr-build.yml"))
        .expect("read PR build workflow");
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
    assert!(workflow.contains("git fetch --force --tags origin"));
    assert!(
        workflow
            .contains(r#"gh release list --repo "$REPO" --exclude-drafts --exclude-pre-releases"#)
    );
    assert!(
        workflow.contains(r#"node scripts/release/next-release-tag.js "${published_tags[@]}""#)
    );
    assert!(
        workflow.contains("Deleting orphan release tag $tag before recreating it for this build.")
    );
    assert!(workflow.contains(r#"git push origin ":refs/tags/$tag""#));
    assert!(workflow.contains("name: Prepare auto release tag"));
    assert!(!workflow.contains("Prepare V0.01 release tag"));
    assert!(!workflow.contains("npm run check"));
    assert_eq!(workflow.matches("run: npm run vite:build").count(), 2);
    assert!(!workflow.contains("cargo test --workspace"));
    assert!(workflow.contains("run: cargo build --release"));
    assert!(workflow.contains("cargo build --release --target \"${{ matrix.target }}\""));
    assert!(pr_build.contains("run: npm run check"));
    assert!(pr_build.contains("run: npm run vite:build"));
    assert!(pr_build.contains("cargo test --workspace"));
    assert!(pr_build.contains("run: cargo build --release"));
    assert!(pr_build.contains("cargo build --release --target \"${{ matrix.target }}\""));
    assert!(workflow.contains("Copy-Item target/release/claude-codex-pro.exe"));
    assert!(workflow.contains("Copy-Item target/release/claude-codex-pro-manager.exe"));
    assert_release_workflow_uses_current_hosted_runners(&workflow);
    assert!(workflow.contains("x86_64-apple-darwin"));
    assert!(workflow.contains("aarch64-apple-darwin"));
    assert!(workflow.contains("package-dmg.sh \"$VERSION\" \"${{ matrix.arch }}\""));
    assert!(workflow.contains("gh release upload \"$TAG\" latest.json --clobber"));
    assert!(workflow.contains("gh release edit \"$TAG\" --repo \"$REPO\" --draft=false --latest"));
    assert!(workflow.contains("cleanup-failed-draft:"));
    assert!(workflow.contains("if: ${{ always() && (failure() || cancelled()) }}"));
    assert!(workflow.contains("--json databaseId,isDraft"));
    assert!(workflow.contains("data.isDraft ? \"true\" : \"false\""));
    assert!(workflow.contains("gh api --method DELETE \"repos/$REPO/releases/$release_id\""));
    assert!(workflow.contains("version: tag"));
    assert!(workflow.contains("## 更新内容"));
    assert!(workflow.contains("## 验证"));
    assert!(workflow.contains("## 构建产物说明"));
    assert!(workflow.contains("前端生产资源由 GitHub Actions 构建并嵌入桌面应用"));
    assert!(workflow.contains("Windows 与 macOS release 二进制均在对应平台构建"));
    assert!(!workflow.contains("## Assets 9"));
    assert!(!workflow.contains("Source code (zip)"));
    assert!(!workflow.contains("Source code (tar.gz)"));
    assert!(!workflow.contains("claude-codex-pro-${version}-macos-arm64.dmg"));
    assert!(workflow.contains("Windows x64"));
    assert!(workflow.contains("macOS x64"));
    assert!(workflow.contains("macOS arm64"));
    assert!(workflow.contains("Compress-Archive"));
    assert!(workflow.contains("ditto -c -k --sequesterRsrc"));
    assert!(workflow.contains("uses: actions/upload-artifact@v5"));
    assert!(workflow.contains("name: windows-x64-release-assets"));
    assert!(workflow.contains("uses: actions/download-artifact@v5"));
    assert!(workflow.contains("Upload build assets to release"));
    assert!(workflow.contains(
        "asset_count=\"$(find release-assets -maxdepth 1 -type f | wc -l | tr -d ' ')\""
    ));
    assert!(workflow.contains("Expected 6 build assets before latest.json"));
    assert!(
        workflow.contains("gh release upload \"$TAG\" release-assets/* --clobber --repo \"$REPO\"")
    );
    assert!(workflow.contains("name: macos-${{ matrix.arch }}-release-assets"));

    assert!(release_assets.contains("auto-release-installers-managed"));
    assert!(release_assets.contains("if: ${{ !contains(github.event.release.body"));
    assert!(release_assets.contains("run: npm run vite:build"));
    assert!(release_assets.contains("version: tag"));
    assert!(release_assets.contains("Compress-Archive"));
    assert!(release_assets.contains("dist/windows/*windows-x64.zip"));
    assert!(release_assets.contains("ditto -c -k --sequesterRsrc"));
    assert!(release_assets.contains("dist/macos/*macos-${{ matrix.arch }}.zip"));

    assert!(version_script.contains("RELEASE_TAG_PATTERN = /^[vV](\\d+)\\.(\\d{2})$/"));
    assert!(version_script.contains("assert.equal(nextReleaseTag([]), \"V0.01\")"));
    assert!(version_script.contains("assert.equal(nextReleaseTag([\"V0.12\"]), \"V0.13\")"));
    assert!(version_script.contains("assert.equal(nextReleaseTag([\"V0.99\"]), \"V1.00\")"));

    assert!(version_rs.contains("CLAUDE_CODEX_PRO_RELEASE_VERSION"));
    assert!(version_rs.contains(
        "DEFAULT_RELEASE_VERSION: &str = concat!(\"dev-\", env!(\"CARGO_PKG_VERSION\"))"
    ));
    assert!(!version_rs.contains("DEFAULT_RELEASE_VERSION: &str = \"V0.12\""));
    assert!(!version_rs.contains("None => env!(\"CARGO_PKG_VERSION\")"));
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
    assert!(commands_rs.contains("pub async fn open_claude_chinese_window"));
    assert!(commands_rs.contains("pub async fn open_plugin_hub_window"));
    assert!(commands_rs.contains("tauri::WebviewUrl::External"));
    assert!(commands_rs.contains("https://claude.ai/new"));
    assert!(!commands_rs.contains("tauri::Url::parse(default_url)"));
    assert!(commands_rs.contains("fn claude_chinese_window_shell_url"));
    assert!(commands_rs.contains("data:text/html;charset=utf-8"));
    assert!(commands_rs.contains("Claude 加载中 / 白屏诊断"));
    assert!(commands_rs.contains("在浏览器打开 Claude"));
    assert!(commands_rs.contains("claude-codex-pro://open-external?url="));
    assert!(commands_rs.contains("url.host_str() == Some(\"open-external\")"));
    assert!(
        commands_rs.contains(
            "<iframe id=\"claude-frame\" src=\"{escaped_url}\" title=\"Claude\"></iframe>"
        )
    );
    assert!(commands_rs.contains("pointer-events: none"));
    assert!(commands_rs.contains("fallback.hidden = true"));
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
    let workflow = read_source_file(&repo_root.join(".github/workflows/pr-build.yml"));

    assert!(workflow.contains("run: npm run vite:build"));
    assert!(workflow.contains("run: cargo build --release"));
    assert!(workflow.contains("uses: actions/checkout@v5"));
    assert!(workflow.contains("uses: actions/setup-node@v5"));
    assert!(workflow.contains("uses: actions/upload-artifact@v5"));
    assert!(workflow.contains("node-version: \"24\""));
    assert!(workflow.contains("runner: macos-latest"));
    assert!(workflow.matches("runner: macos-latest").count() >= 2);
    assert!(workflow.contains("dist/macos/stage/Claude Codex Pro Manager.app"));
    assert!(!workflow.contains("Claude Codex Pro 绠＄悊宸ュ叿.app"));
    assert!(!workflow.contains("Claude Codex Pro 管理工具.app"));
    for deprecated in [
        "macos-15-intel",
        "macos-14",
        "actions/checkout@v4",
        "actions/setup-node@v4",
        "actions/upload-artifact@v4",
        "node-version: \"22\"",
    ] {
        assert!(
            !workflow.contains(deprecated),
            "deprecated PR workflow value: {deprecated}"
        );
    }
}

#[test]
fn plugin_hub_is_first_class_ops_console_route() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = read_all_frontend_sources();
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = read_source_file(&styles);
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    assert!(app_tsx.contains("id: \"tools\""));
    assert!(app_tsx.contains("label: \"插件、Skills 与 MCP\""));
    assert!(app_tsx.contains("id: \"sessions\""));
    assert!(app_tsx.contains("label: \"会话与记忆\""));
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

    assert!(tools_section.contains("UnifiedToolInventoryPanel"));
    assert!(tools_section.contains("CodexPluginRepositoryPanel"));
    assert!(tools_section.contains("ClaudePluginRepositoryPanel"));
    assert!(app_tsx.contains("Claude、Codex 工具与插件"));
    assert!(app_tsx.contains("toggleUnifiedToolAsset"));
    assert!(app_tsx.contains("mcpTarget === \"codex\""));
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
    let styles = read_source_file(&styles_path);

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
    assert!(app_tsx.contains("repository.configured ? \"配置已写入\" : \"配置未写入\""));

    assert!(app_tsx.contains("className=\"agent-toggle-group\""));
    assert!(app_tsx.contains("className={`agent-toggle ${app}"));
    assert!(app_tsx.contains("onClick={() => void toggle(asset, app)}"));
    assert!(styles.contains(".agent-toggle-group"));
    assert!(styles.contains(".agent-toggle.claude.enabled"));
    assert!(styles.contains(".agent-toggle.codex.enabled"));
    assert!(styles.contains("grid-template-columns: minmax(0, 1fr) 124px;"));
    assert!(app_tsx.contains("setNotice({ title: \"Codex 插件仓库\", message: result.message || result.repair.message, status: result.status })"));
    assert!(styles.contains("overflow-wrap: anywhere;"));
    assert!(styles.contains("white-space: normal;"));
}

#[test]
fn session_management_route_contains_history_and_codex_claude_session_management() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // Screen 组件已拆分到 src/screens.tsx；结构化切片读该文件。
    let app_tsx = read_frontend_file("screens.tsx");
    let app_shell = read_frontend_file("App.tsx");
    let tauri_bridge = read_frontend_file("tauriBridge.ts");
    let commands_rs = read_source_file(&manifest_dir.join("src/commands.rs"));
    let lib_rs = read_source_file(&manifest_dir.join("src/lib.rs"));
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = read_source_file(&styles);

    let session_section = app_tsx
        .split("function SessionManagementScreen")
        .nth(1)
        .and_then(|rest| rest.split("function PluginHubScreen").next())
        .expect("session screen source");

    assert!(session_section.contains("会话管理"));
    assert!(session_section.contains("历史会话修复"));
    assert!(session_section.contains("Codex 会话管理"));
    assert!(session_section.contains("Claude 会话管理"));
    assert!(session_section.contains("refreshLocalSessions"));
    assert!(session_section.contains("deleteLocalSession"));
    assert!(session_section.contains("refreshClaudeSessions"));
    assert!(session_section.contains("deleteClaudeSession"));
    assert!(session_section.contains("loadClaudeSessionContext"));
    assert!(session_section.contains("groupLocalSessionsByProject(codexSessions)"));
    assert!(session_section.contains("groupClaudeSessionsByProject(claudeSessionsList)"));
    assert!(session_section.contains("renderSessionBrowserPanel"));
    assert!(session_section.contains("session-management-wide-grid"));
    assert!(session_section.contains("session-history-card"));
    assert!(session_section.contains("session-codex-card"));
    assert!(session_section.contains("session-claude-card"));
    assert!(session_section.contains("className=\"codex-session-browser\""));
    assert!(session_section.contains("Codex 本地会话项目列表"));
    assert!(session_section.contains("Claude 本地会话项目列表"));
    assert!(session_section.contains("data: localSessions"));
    assert!(session_section.contains("data: claudeSessions"));
    assert!(!session_section.contains("data: null"));
    assert!(!session_section.contains("Claude 会话扫描尚未接入"));
    assert!(session_section.contains(r#"sourceLabel: "Claude 会话源""#));
    assert!(!session_section.contains(r#"statusLabel: "待接入""#));
    assert!(!session_section.contains(r#"renderSessionBrowserPanel("Claude 会话管理""#));
    assert!(session_section.contains("className=\"codex-session-project-header\""));
    assert!(session_section.contains("className=\"codex-session-main\""));
    assert!(session_section.contains("onClick={() => onOpen?.(session)}"));
    assert!(session_section.contains("formatSessionRelativeTime(session.updatedAtMs)"));
    assert!(
        session_section.contains("onDelete: (session) => void actions.deleteLocalSession(session)")
    );
    assert!(
        session_section
            .contains("onDelete: (session) => void actions.deleteClaudeSession(session)")
    );
    assert!(
        session_section
            .contains("onOpen: (session) => void actions.loadClaudeSessionContext(session)")
    );
    assert!(session_section.contains("claude-session-context-overlay"));
    assert!(session_section.contains("claude-session-context-dialog"));
    assert!(session_section.contains("加载更早内容"));
    assert!(session_section.contains("关闭会话上下文"));
    assert!(app_shell.contains("const [claudeSessions, setClaudeSessions]"));
    assert!(app_shell.contains("const [claudeSessionContext, setClaudeSessionContext]"));
    assert!(app_shell.contains("call<ClaudeSessionsResult>(\"list_claude_sessions\")"));
    assert!(app_shell.contains("call<ClaudeSessionContextPage>(\"load_claude_session_context\""));
    assert!(app_shell.contains("call<DeleteClaudeSessionResult>(\"delete_claude_session\""));
    assert!(commands_rs.contains("pub async fn list_claude_sessions"));
    assert!(commands_rs.contains("pub async fn load_claude_session_context"));
    assert!(commands_rs.contains("pub async fn delete_claude_session"));
    assert!(lib_rs.contains("commands::list_claude_sessions"));
    assert!(lib_rs.contains("commands::load_claude_session_context"));
    assert!(lib_rs.contains("commands::delete_claude_session"));
    assert!(tauri_bridge.contains("command === \"list_claude_sessions\""));
    assert!(tauri_bridge.contains("command === \"load_claude_session_context\""));
    assert!(tauri_bridge.contains("command === \"delete_claude_session\""));
    assert!(session_section.contains("repairHistorySessions"));
    assert!(!session_section.contains("Claude 会话诊断"));
    assert!(!session_section.contains("launchClaudeDesktop"));
    assert!(!session_section.contains("installClaudeZhPatch"));
    assert!(!session_section.contains("openClaudeChinese"));
    assert!(styles.contains(".session-management-wide-grid"));
    assert!(styles.contains("grid-template-columns: minmax(340px, 0.72fr) minmax(560px, 1.28fr);"));
    assert!(styles.contains(".session-codex-card .codex-session-browser"));
    assert!(styles.contains("max-height: 136px;"));
    assert!(styles.contains(".session-claude-card"));
    assert!(styles.contains("grid-column: 1 / -1;"));
    assert!(styles.contains(".codex-session-browser"));
    assert!(!styles.contains("background: #f3eeee;"));
    assert!(styles.contains("rgba(8, 9, 12, 0.72);"));
    assert!(styles.contains(".codex-session-project-header"));
    assert!(styles.contains(".codex-session-main time"));
    assert!(styles.contains(".codex-session-delete"));
    assert!(styles.contains(".claude-session-context-overlay"));
    assert!(styles.contains(".claude-session-context-dialog"));
    assert!(styles.contains(".claude-session-context-message"));
}

#[test]
fn codex_session_management_opens_real_context_viewer() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app = read_frontend_file("App.tsx");
    let screens = read_frontend_file("screens.tsx");
    let commands = read_source_file(&manifest_dir.join("src/commands.rs"));
    let lib = read_source_file(&manifest_dir.join("src/lib.rs"));
    let bridge = read_frontend_file("tauriBridge.ts");

    assert!(commands.contains("pub async fn load_codex_session_context"));
    assert!(commands.contains("Codex 会话数据库不是受信任的已发现路径"));
    assert!(lib.contains("commands::load_codex_session_context"));
    assert!(app.contains("loadEarlierCodexSessionContext"));
    assert!(screens.contains("actions.loadCodexSessionContext(session)"));
    assert!(screens.contains("Codex rollout"));
    assert!(screens.contains("assistant: showingCodexContext ? \"Codex\" : \"Claude\""));
    assert!(bridge.contains("load_codex_session_context"));
}

#[test]
fn manager_startup_commands_run_blocking_work_off_ui_thread() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = read_source_file(&manifest_dir.join("src/commands.rs"));
    let settings_section = commands_rs
        .split("pub async fn load_settings()")
        .nth(1)
        .and_then(|rest| rest.split("#[tauri::command]").next())
        .expect("async load_settings source");
    let claude_window_section = commands_rs
        .split("pub async fn load_claude_chinese_window_status(")
        .nth(1)
        .and_then(|rest| rest.split("#[tauri::command]").next())
        .expect("async Claude window status source");

    assert!(commands_rs.contains("pub async fn load_settings()"));
    assert!(settings_section.contains("spawn_blocking"));
    assert!(settings_section.contains("settings_payload("));
    assert!(commands_rs.contains("pub async fn load_claude_chinese_window_status("));
    assert!(claude_window_section.contains("spawn_blocking"));
    assert!(claude_window_section.contains("detect_status_light"));
}

#[test]
fn prompt_optimizer_feature_is_removed() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = read_all_frontend_sources();
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = read_source_file(&styles);
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");
    let lib_rs = read_source_file(&manifest_dir.join("src/lib.rs"));

    for removed in [
        "promptOptimizer",
        "PromptOptimizerCard",
        "PromptOptimizerScreen",
        "goPromptOptimizer",
        "PROMPT_OPTIMIZER_URL",
        "linshenkx/prompt-optimizer",
        "prompt.always200.com",
    ] {
        assert!(
            !app_tsx.contains(removed),
            "removed frontend feature remains: {removed}"
        );
    }
    for removed in [
        "open_prompt_optimizer_window",
        "PromptOptimizerWindowPayload",
        "prompt_optimizer_window_payload",
        "tools_card_external_browser",
        "prompt.always200.com",
    ] {
        assert!(
            !commands_rs.contains(removed),
            "removed command remains: {removed}"
        );
    }
    assert!(!lib_rs.contains("commands::open_prompt_optimizer_window"));
    assert!(!styles.contains(".prompt-optimizer-"));
    assert!(app_tsx.contains("normalizeRoute(window.__CLAUDE_CODEX_PRO_INITIAL_ROUTE)"));
    assert!(app_tsx.contains("routeDocumentTitle"));
    assert!(app_tsx.contains("__CLAUDE_CODEX_PRO_INITIAL_ROUTE"));
}

#[test]
fn ui_information_architecture_refactor_keeps_frontend_source_contracts() {
    fn normalize_css(value: &str) -> String {
        value.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn css_rule_declarations<'a>(source: &'a str, selector: &str) -> Vec<&'a str> {
        source
            .split('}')
            .filter_map(|block| {
                let (selectors, declarations) = block.rsplit_once('{')?;
                selectors
                    .split(',')
                    .any(|candidate| candidate.lines().last().map(str::trim) == Some(selector))
                    .then_some(declarations)
            })
            .collect()
    }

    fn css_rule_has(source: &str, selector: &str, declaration: &str) -> bool {
        let declaration = normalize_css(declaration);
        css_rule_declarations(source, selector)
            .into_iter()
            .any(|candidate| normalize_css(candidate).contains(&declaration))
    }

    fn css_rule_with_fragments_has(
        source: &str,
        selector_fragments: &[&str],
        declaration: &str,
    ) -> bool {
        let declaration = normalize_css(declaration);
        source.split('}').any(|block| {
            let Some((selectors, declarations)) = block.rsplit_once('{') else {
                return false;
            };
            selector_fragments
                .iter()
                .all(|fragment| selectors.contains(fragment))
                && normalize_css(declarations).contains(&declaration)
        })
    }

    fn css_numeric_property(source: &str, selector: &str, property: &str) -> Option<f32> {
        css_rule_declarations(source, selector)
            .into_iter()
            .flat_map(|declarations| declarations.split(';'))
            .find_map(|declaration| {
                let (name, value) = declaration.split_once(':')?;
                (name.trim() == property)
                    .then(|| value.trim().parse::<f32>().ok())
                    .flatten()
            })
    }

    fn jsx_button_containing<'a>(source: &'a str, marker: &str) -> &'a str {
        let marker_index = source
            .find(marker)
            .unwrap_or_else(|| panic!("missing button marker: {marker}"));
        let start = source[..marker_index]
            .rfind("<button")
            .unwrap_or_else(|| panic!("missing button start for: {marker}"));
        let end = source[marker_index..]
            .find("</button>")
            .map(|offset| marker_index + offset + "</button>".len())
            .unwrap_or_else(|| panic!("missing button end for: {marker}"));
        &source[start..end]
    }

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app = read_frontend_file("App.tsx");
    let app_shell = read_frontend_file("components/AppShell.tsx");
    let clients_screen = read_frontend_file("components/ClientsEnhancementScreen.tsx");
    let main = read_frontend_file("main.tsx");
    let manager_lib = read_source_file(&manifest_dir.join("src/lib.rs"));
    let routes = read_frontend_file("lib/routes.ts");
    let screens = read_frontend_file("screens.tsx");
    let tauri_config = read_source_file(&manifest_dir.join("tauri.conf.json"));
    let types = read_frontend_file("types.ts");
    let workspace = read_frontend_file("workspace.css");

    let primary_routes = source_section(
        &routes,
        "export const routes: RouteItem[] = [",
        "export const compatibilityRoutes",
    );
    assert_eq!(
        primary_routes.matches("id: \"").count(),
        9,
        "the sidebar must expose exactly nine primary destinations"
    );
    for (id, label) in [
        ("overview", "概览"),
        ("supplier", "供应商与路由"),
        ("clients", "客户端与增强"),
        ("sessions", "会话与记忆"),
        ("tools", "插件、Skills 与 MCP"),
        ("maintenance", "维护与诊断"),
        ("settings", "设置"),
    ] {
        assert!(
            primary_routes.contains(&format!("id: \"{id}\", label: \"{label}\"")),
            "missing primary route {id} ({label})"
        );
    }
    assert!(primary_routes.contains("id: \"themes\""));
    assert!(primary_routes.contains("icon: Palette"));
    assert!(primary_routes.contains("id: \"prompts\""));
    assert!(primary_routes.contains("icon: FileText"));
    assert!(!primary_routes.contains("id: \"models\""));
    assert!(!primary_routes.contains("id: \"memory\""));
    assert!(!primary_routes.contains("id: \"about\""));

    let compatibility_routes = source_section(
        &routes,
        "export const compatibilityRoutes: RouteItem[] = [",
        "export const routeCatalog",
    );
    assert_eq!(compatibility_routes.matches("id: \"").count(), 2);
    assert!(compatibility_routes.contains("id: \"memory\", label: \"盘古记忆\""));
    assert!(compatibility_routes.contains("id: \"about\", label: \"关于与更新\""));
    assert!(
        routes.contains(
            "export const routeCatalog: RouteItem[] = [...routes, ...compatibilityRoutes];"
        )
    );
    assert!(routes.contains("return routeCatalog.some((item) => item.id === value);"));
    assert!(routes.contains("if (route === \"memory\") return \"sessions\";"));
    assert!(routes.contains("if (route === \"about\") return \"settings\";"));
    assert!(routes.contains("if (route === \"sessions\" || route === \"memory\")"));
    assert!(routes.contains("{ id: \"sessions\", label: \"会话\" }"));
    assert!(routes.contains("{ id: \"memory\", label: \"盘古记忆\" }"));
    assert!(routes.contains("if (route === \"settings\" || route === \"about\")"));
    assert!(routes.contains("{ id: \"settings\", label: \"偏好设置\" }"));
    assert!(routes.contains("{ id: \"about\", label: \"关于与更新\" }"));
    let document_title = &routes[routes
        .find("export function routeDocumentTitle")
        .expect("document title helper")..];
    assert!(document_title.contains("\"CCP 管理工具\""));
    assert!(document_title.contains("- CCP 管理工具"));
    assert!(!document_title.contains("Claude Codex Pro 管理工具"));
    assert!(routes.contains("normalizeRoute(window.__CLAUDE_CODEX_PRO_INITIAL_ROUTE)"));
    assert!(routes.contains("new URLSearchParams(window.location.search).get(\"view\")"));
    let normalize_route = source_section(
        &routes,
        "export function normalizeRoute",
        "export function routeSubtitle",
    );
    assert!(normalize_route.contains("value === \"models\""));
    assert!(normalize_route.contains("return \"supplier\";"));
    assert!(app.contains("window.addEventListener(\"claude-codex-pro-navigate\", navigate)"));
    assert!(app.contains("const route = normalizeRoute("));
    assert!(app.contains("if (!isRoute(route)) return;"));

    let route_type = source_section(&types, "export type Route =", "export type LegacyRoute");
    for route in [
        "overview",
        "supplier",
        "clients",
        "themes",
        "tools",
        "sessions",
        "memory",
        "maintenance",
        "settings",
        "about",
    ] {
        assert!(
            route_type.contains(&format!("| \"{route}\"")),
            "Route union is missing {route}"
        );
    }
    assert!(
        !route_type.contains("\"models\""),
        "models must not remain a live Route"
    );
    let legacy_route_type = source_section(&types, "export type LegacyRoute", "declare global");
    assert!(legacy_route_type.contains("\"relay\""));
    assert!(legacy_route_type.contains("\"models\""));

    for shell_contract in [
        "<aside className=\"ops-rail\" aria-label=\"一级导航\">",
        "<main className=\"ops-workspace\">",
        "<header className=\"ops-topbar\" data-tauri-drag-region>",
        "<section className=\"ops-screen\">",
        "{routes.map((item) => {",
        "const activePrimaryRoute = primaryRoute(route);",
        "aria-label=\"Agent 范围\"",
        "activeSupplierName",
        "proxyHealth",
    ] {
        assert!(
            app_shell.contains(shell_contract),
            "missing AppShell contract: {shell_contract}"
        );
    }
    assert!(app_shell.contains("const THEME_STORAGE_KEY = \"ccp-manager-theme\";"));
    assert!(app_shell.contains("window.localStorage.getItem(THEME_STORAGE_KEY)"));
    assert!(app_shell.contains("window.localStorage.setItem(THEME_STORAGE_KEY, themePreference)"));
    assert!(app_shell.contains("window.matchMedia?.(\"(prefers-color-scheme: dark)\")"));
    assert!(app_shell.contains("media.addEventListener?.(\"change\", update)"));
    assert!(app_shell.contains("root.dataset.theme = resolvedTheme;"));
    assert!(app_shell.contains("root.classList.toggle(\"dark\", resolvedTheme === \"dark\")"));
    assert!(app_shell.contains("[\"system\", \"跟随系统\", Laptop]"));
    assert!(app_shell.contains("[\"light\", \"浅色\", Sun]"));
    assert!(app_shell.contains("[\"dark\", \"深色\", Moon]"));
    assert!(app_shell.contains("(event.ctrlKey || event.metaKey)"));
    assert!(app_shell.contains("event.key.toLocaleLowerCase() === \"k\""));
    assert!(app_shell.contains("event.key === \"Escape\""));
    assert!(app_shell.contains("event.key === \"ArrowDown\""));
    assert!(app_shell.contains("event.key === \"ArrowUp\""));
    assert!(app_shell.contains("event.key === \"Enter\""));
    assert!(app_shell.contains("return routeCatalog.filter((item) =>"));
    assert!(app_shell.contains("role=\"dialog\""));
    assert!(app_shell.contains("role=\"listbox\""));

    let breadcrumb = source_section(
        &routes,
        "export function routeBreadcrumb",
        "export function routeDomainTabs",
    );
    assert!(breadcrumb.contains("\"CCP\""));
    assert!(!breadcrumb.contains("盘古"));
    let brand_button = jsx_button_containing(&app_shell, "className=\"ops-brand\"");
    assert!(brand_button.contains("<strong>CCP</strong>"));
    assert!(!brand_button.contains("<strong>盘古</strong>"));
    assert!(manager_lib.contains(".title(\"CCP 管理工具\")"));
    assert!(tauri_config.contains("\"title\": \"CCP 管理工具\""));
    let tray_setup = source_section(&manager_lib, "fn setup_tray", "fn show_main_window");
    assert!(tray_setup.contains(".tooltip(\"CCP 管理工具\")"));
    assert!(!tray_setup.contains("Claude Codex Pro 管理工具"));
    let about_screen = &screens[screens
        .find("export const AboutScreen")
        .expect("About screen")..];
    assert!(about_screen.contains("<Panel title=\"关于 CCP\""));
    assert!(about_screen.contains("detail=\"CCP 本地供应商、客户端、会话与维护控制台。\""));
    assert!(!about_screen.contains("<Panel title=\"关于 Claude Codex Pro\""));
    assert!(
        !app_shell.contains("onRefresh"),
        "refresh is a page concern, not a topbar AppShell prop"
    );

    let commandbar = source_section(
        &app_shell,
        "<div className=\"ops-commandbar\">",
        "</header>",
    );
    assert_eq!(
        commandbar.matches("ops-action-command").count(),
        3,
        "the command bar must expose exactly three primary client actions"
    );
    let action_positions = [
        commandbar
            .find("onClick={onRestartCodex}")
            .expect("Codex restart action"),
        commandbar
            .find("onClick={onLaunchClaude}")
            .expect("Claude launch action"),
        commandbar
            .find("onClick={onInstallClaudeZhPatch}")
            .expect("Claude localization action"),
    ];
    assert!(action_positions.windows(2).all(|pair| pair[0] < pair[1]));
    for control in [
        "className=\"ops-command-search\"",
        "className=\"ops-runtime-chip supplier\"",
        "className={`ops-runtime-chip health ${proxyHealth}`}",
    ] {
        assert!(
            commandbar
                .find(control)
                .is_some_and(|position| position < action_positions[0]),
            "topbar context control must remain left of client actions: {control}"
        );
    }
    for (handler, label) in [
        ("onClick={onRestartCodex}", "启动/重启 Codex"),
        ("onClick={onLaunchClaude}", "启动/重启 Claude"),
        ("onClick={onInstallClaudeZhPatch}", "Claude 一键汉化"),
    ] {
        let button = jsx_button_containing(commandbar, handler);
        let icon = button
            .find("aria-hidden=\"true\"")
            .unwrap_or_else(|| panic!("missing action icon: {label}"));
        let text = button
            .find(&format!("<span>{label}</span>"))
            .unwrap_or_else(|| panic!("missing visible action label: {label}"));
        assert!(icon < text, "action icon must precede its label: {label}");
    }

    assert!(app.contains("import { AppShell, type AgentScope, type ProxyHealth }"));
    assert!(app.contains("import { ClientsEnhancementScreen }"));
    assert!(
        !app.contains("route === \"models\""),
        "App must not retain a live models route branch"
    );
    assert!(app.contains("const [agentScope, setAgentScope] = useState<AgentScope>(\"all\")"));
    assert!(app.contains("onAgentScopeChange={setAgentScope}"));
    for action_binding in [
        "onRestartCodex={() => void actions.restartCodex()}",
        "onLaunchClaude={() => void actions.launchClaudeDesktop()}",
        "onInstallClaudeZhPatch={() => void actions.installClaudeZhPatch()}",
    ] {
        assert!(
            app.contains(action_binding),
            "AppShell action no longer delegates to existing behavior: {action_binding}"
        );
    }
    let clients_route = source_section(
        &app,
        "{route === \"clients\" ? (",
        "{route === \"tools\" ? (",
    );
    for prop in [
        "actions={actions}",
        "agentScope={agentScope}",
        "claudeDesktop={claudeDesktop}",
        "claudeDesktopDevMode={claudeDesktopDevMode}",
        "claudeZhPatch={claudeZhPatch}",
        "overview={overview}",
        "settings={settingsDraft ?? settings?.settings ?? null}",
        "watcher={watcher}",
    ] {
        assert!(
            clients_route.contains(prop),
            "clients route is missing real state prop: {prop}"
        );
    }

    for client_contract in [
        "overview?.latest_launch",
        "claudeZhPatch?.status.localeConfigured",
        "claudeDesktopDevMode?.devModeStatus.configured",
        "watcher?.enabled",
        "settings?.relayProfiles.find(",
        "clients.filter((client) => clientMatchesScope(client, agentScope))",
        "agentScope === \"codex\" ? client.id === \"codex\" : client.id !== \"codex\"",
        "<StateCell label=\"已安装\"",
        "<StateCell label=\"已启用\"",
        "<StateCell label=\"当前生效\"",
        "<StateCell label=\"健康状态\"",
        "正在读取本机客户端与增强状态",
        "状态读取失败，页面保留已取得的数据",
        "当前 Agent 范围没有可显示的客户端",
    ] {
        assert!(
            clients_screen.contains(client_contract),
            "missing client/enhancement contract: {client_contract}"
        );
    }
    for action in [
        "actions.restartCodex()",
        "actions.repairFrontendConnection()",
        "actions.launchClaudeDesktop()",
        "actions.installClaudeZhPatch()",
        "actions.configureClaudeDesktopDevMode()",
        "actions.installWatcher()",
        "actions.disableWatcher()",
        "actions.enableWatcher()",
    ] {
        assert!(
            clients_screen.contains(action),
            "client page no longer delegates to existing action: {action}"
        );
    }
    assert!(!clients_screen.contains("invokeCommand"));

    let session_screen = source_section(
        &screens,
        "export const SessionManagementScreen",
        "export const SettingsScreen",
    );
    assert!(
        !session_screen.contains("<Panel title=\"会话管理\""),
        "the sessions page must not repeat a generic introduction panel"
    );
    let session_card_positions = [
        session_screen
            .find("className=\"session-history-card\"")
            .expect("history repair card"),
        session_screen
            .find("className=\"session-codex-card\"")
            .expect("Codex session card"),
        session_screen
            .find("className=\"session-claude-card\"")
            .expect("Claude session card"),
    ];
    assert!(
        session_card_positions
            .windows(2)
            .all(|pair| pair[0] < pair[1]),
        "history repair must precede the paired Codex and Claude session panes"
    );
    for title in [
        "title=\"历史会话修复\"",
        "title: \"Codex 会话管理\"",
        "title: \"Claude 会话管理\"",
    ] {
        assert!(
            session_screen.contains(title),
            "missing session pane: {title}"
        );
    }
    assert!(css_rule_has(
        &workspace,
        ".session-management-wide-grid",
        "grid-template-columns: repeat(2, minmax(0, 1fr));"
    ));
    assert!(css_rule_has(
        &workspace,
        ".session-history-card",
        "grid-column: 1 / -1;"
    ));
    assert!(
        css_rule_declarations(&workspace, ".session-management-wide-grid")
            .into_iter()
            .any(|declarations| normalize_css(declarations).contains("grid-template-columns: 1fr;")),
        "the paired session panes must collapse to one column at narrow widths"
    );

    let tools_screen = source_section(
        &screens,
        "export const ToolsAndPluginsScreen",
        "function UnifiedToolInventoryPanel",
    );
    let repository_grid = source_section(
        tools_screen,
        "<div className=\"repository-status-grid\">",
        "</div>",
    );
    assert!(repository_grid.contains("<CodexPluginRepositoryPanel"));
    assert!(repository_grid.contains("<ClaudePluginRepositoryPanel"));
    assert!(css_rule_has(
        &workspace,
        ".repository-status-grid",
        "grid-template-columns: repeat(2, minmax(0, 1fr));"
    ));
    assert!(
        css_rule_declarations(&workspace, ".repository-status-grid")
            .into_iter()
            .any(|declarations| normalize_css(declarations).contains("grid-template-columns: 1fr;")),
        "repository panes must collapse to one column at narrow widths"
    );
    for (selector, color) in [
        (".unified-tool-copy > strong", "var(--workspace-text)"),
        (
            ".unified-tool-copy > span",
            "var(--workspace-text-secondary)",
        ),
        (
            ".unified-tool-copy > small",
            "var(--workspace-text-secondary)",
        ),
    ] {
        assert!(
            css_rule_has(&workspace, selector, &format!("color: {color};")),
            "tool copy must use a readable workspace token: {selector}"
        );
        assert!(
            css_rule_has(&workspace, selector, "opacity: 1;"),
            "tool copy must not be faded: {selector}"
        );
    }
    assert!(
        css_numeric_property(&workspace, ".ops-shell .agent-toggle", "opacity")
            .is_some_and(|opacity| opacity >= 0.7),
        "Agent controls need a readable base opacity"
    );
    for selector in [
        ".ops-shell .agent-toggle img",
        ".ops-shell .agent-toggle.codex img",
    ] {
        assert!(css_rule_has(&workspace, selector, "filter: none;"));
        assert!(css_rule_has(&workspace, selector, "opacity: 1;"));
    }

    let model_dropdown = source_section(
        &screens,
        "function SupplierModelDropdown",
        "export function SupplierScreen",
    );
    for interaction_contract in [
        "createPortal(",
        "document.body",
        "role=\"listbox\"",
        "role=\"option\"",
        "aria-selected={option === value}",
        "aria-expanded={open}",
        "aria-haspopup=\"listbox\"",
        "onChange(option);",
        "setOpen(false);",
        "event.key === \"Escape\"",
        "requestAnimationFrame(() => triggerRef.current?.focus())",
        "type=\"button\"",
    ] {
        assert!(
            model_dropdown.contains(interaction_contract),
            "model menu is missing an interaction contract: {interaction_contract}"
        );
    }
    let claude_model_mapping = source_section(
        &screens,
        "<div className=\"supplier-model-map-grid header claude\"",
        "<label className=\"ops-form-field\"><span>默认兜底模型</span>",
    );
    let codex_model_mapping = source_section(
        &screens,
        "<div className=\"supplier-codex-catalog-grid header\"",
        "自定义 User-Agent",
    );
    for (name, mapping) in [
        ("Claude", claude_model_mapping),
        ("Codex", codex_model_mapping),
    ] {
        for shared_contract in [
            "className=\"supplier-model-input-dropdown\"",
            "<SupplierModelDropdown",
            "iconOnly",
            "options={supplierModelOptions}",
            "showAvailabilityWarning={false}",
            "triggerLabel=\"选择已获取模型\"",
            "实际请求模型",
        ] {
            assert!(
                mapping.contains(shared_contract),
                "{name} actual-model control is missing: {shared_contract}"
            );
        }
    }
    assert!(
        claude_model_mapping
            .contains("updateSupplierModelMapping(row.role, \"requestModel\", value)")
    );
    assert!(codex_model_mapping.contains("updateSupplierCodexCatalogModel(row.rowId, {"));
    assert!(codex_model_mapping.contains("model: value,"));

    for declaration in [
        "background: var(--workspace-surface);",
        "color: var(--workspace-text);",
    ] {
        assert!(
            css_rule_with_fragments_has(
                &workspace,
                &[
                    ".ops-shell",
                    ".supplier-ccswitch-editor",
                    "input",
                    "select",
                    "textarea",
                ],
                declaration,
            ),
            "supplier editor controls need a high-specificity light-theme override: {declaration}"
        );
    }
    for declaration in [
        "background: hsl(var(--popover));",
        "color: hsl(var(--popover-foreground));",
        "color-scheme: light;",
        "pointer-events: auto;",
    ] {
        assert!(
            css_rule_has(&workspace, ".supplier-model-dropdown-menu", declaration),
            "portal model menu is missing a readable/clickable style: {declaration}"
        );
    }
    assert!(css_rule_with_fragments_has(
        &workspace,
        &[":root.dark", ".supplier-model-dropdown-menu"],
        "color-scheme: dark;"
    ));
    assert!(css_rule_has(
        &workspace,
        ".supplier-model-dropdown-menu button",
        "color: inherit;"
    ));
    assert!(css_rule_has(
        &workspace,
        ".supplier-model-dropdown-menu button:focus-visible",
        "outline: 2px solid hsl(var(--ring));"
    ));

    let legacy_styles_import = main
        .find("import \"./styles.css\";")
        .expect("main.tsx imports the legacy compatibility layer");
    let workspace_import = main
        .find("import \"./workspace.css\";")
        .expect("main.tsx imports the workspace visual layer");
    assert!(
        legacy_styles_import < workspace_import,
        "workspace.css must load after styles.css so its tokens and shell rules win"
    );
    for token in [
        "--workspace-canvas: #f5f6f8;",
        "--workspace-surface: #ffffff;",
        "--workspace-border: #e3e6ea;",
        "--workspace-text: #17191c;",
        "--workspace-text-secondary: #656b73;",
        "--workspace-blue: #0a84ff;",
        "--workspace-success: #10b981;",
        "--workspace-warning: #f59e0b;",
        "--workspace-danger: #ef4444;",
        "--workspace-sidebar-width: 240px;",
        "--workspace-sidebar-collapsed-width: 64px;",
        "--workspace-topbar-height: 54px;",
        "--workspace-control-height: 36px;",
        "--workspace-radius: 8px;",
        "--workspace-control-radius: 6px;",
    ] {
        assert!(
            workspace.contains(token),
            "missing workspace token: {token}"
        );
    }
    for dark_token in [
        ".ops-shell.dark {",
        "--workspace-canvas: #121416;",
        "--workspace-surface: #191c20;",
        "--workspace-surface-raised: #20242a;",
        "--workspace-border: #2c3138;",
        "--workspace-text: #f2f4f7;",
    ] {
        assert!(
            workspace.contains(dark_token),
            "missing dark workspace token: {dark_token}"
        );
    }
    for layout_contract in [
        "grid-template-columns: var(--workspace-sidebar-width) minmax(0, 1fr);",
        "grid-template-columns: var(--workspace-sidebar-collapsed-width) minmax(0, 1fr);",
        "grid-template-rows: var(--workspace-topbar-height) minmax(0, 1fr);",
        "height: var(--workspace-topbar-height);",
        "padding: 20px 24px 32px;",
        "min-width: 960px;",
        "min-height: 640px;",
        "backdrop-filter: blur(28px) saturate(115%);",
        "letter-spacing: 0;",
        "@media (prefers-reduced-motion: reduce)",
    ] {
        assert!(
            workspace.contains(layout_contract),
            "missing workspace layout contract: {layout_contract}"
        );
    }
    assert!(!workspace.contains("linear-gradient("));
    assert!(!workspace.contains("radial-gradient("));
}

#[test]
fn codex_theme_center_route_and_tauri_command_contracts_match() {
    fn assert_json_keys(value: &serde_json::Value, expected: &[&str]) {
        let actual = value
            .as_object()
            .expect("serialized command result must be an object")
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let expected = expected
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected);
    }

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let routes = read_frontend_file("lib/routes.ts");
    let types = read_frontend_file("types.ts");
    let commands = read_source_file(&manifest_dir.join("src/commands.rs"));
    let manager_lib = read_source_file(&manifest_dir.join("src/lib.rs"));

    let primary_routes = source_section(
        &routes,
        "export const routes: RouteItem[] = [",
        "export const compatibilityRoutes",
    );
    assert!(
        primary_routes.contains("id: \"themes\"") && primary_routes.contains("icon: Palette"),
        "themes must be a visible primary navigation destination"
    );
    let route_type = source_section(&types, "export type Route =", "export type LegacyRoute");
    assert!(route_type.contains("| \"themes\""));

    let registered_commands = source_section(
        &manager_lib,
        ".invoke_handler(tauri::generate_handler![",
        ".run(tauri::generate_context!())",
    );
    for command in [
        "list_codex_themes",
        "import_codex_theme",
        "apply_codex_theme",
        "restore_codex_default_theme",
    ] {
        assert_eq!(
            registered_commands
                .matches(&format!("commands::{command}"))
                .count(),
            1,
            "theme command must be registered exactly once: {command}"
        );
    }

    let list_command = source_section(
        &commands,
        "pub fn list_codex_themes()",
        "pub async fn import_codex_theme(",
    );
    assert!(list_command.contains("-> CommandResult<CodexThemeList>"));

    let import_command = source_section(
        &commands,
        "pub async fn import_codex_theme(",
        "pub fn apply_codex_theme(",
    );
    for contract in [
        "source_path: String,",
        "replace_existing: Option<bool>,",
        ") -> CommandResult<CodexThemeSummary>",
    ] {
        assert!(
            import_command.contains(contract),
            "import command contract is missing: {contract}"
        );
    }

    let apply_command = source_section(
        &commands,
        "pub fn apply_codex_theme(",
        "pub fn restore_codex_default_theme()",
    );
    assert!(apply_command.contains("theme_id: String"));
    assert!(apply_command.contains("-> CommandResult<CodexThemeOperationResult>"));

    let restore_command = source_section(
        &commands,
        "pub fn restore_codex_default_theme()",
        "pub async fn preview_plugin_hub_install(",
    );
    assert!(restore_command.contains("-> CommandResult<CodexThemeOperationResult>"));

    let shared_result = source_section(
        &types,
        "export type CommandResult<T>",
        "export type StatusChipTone",
    );
    for contract in ["T & {", "status: Status;", "message: string;"] {
        assert!(shared_result.contains(contract));
    }

    let summary_type = source_section(
        &types,
        "export type CodexThemeSummary = {",
        "export type CodexThemeListResult",
    );
    for contract in [
        "id: string;",
        "name: string;",
        "version: string;",
        "author: string;",
        "description: string;",
        "preview_data_uri: string | null;",
        "builtin: boolean;",
        "current: boolean;",
        "imported_at: number;",
        "updated_at: number;",
        "integrity_sha256: string | null;",
        "previous_version_available: boolean;",
    ] {
        assert!(
            summary_type.contains(contract),
            "CodexThemeSummary is missing: {contract}"
        );
    }

    let list_type = source_section(
        &types,
        "export type CodexThemeListResult",
        "export type CodexThemeImportResult",
    );
    for contract in [
        "CommandResult<{",
        "themes: CodexThemeSummary[];",
        "current_theme_id: string;",
        "generation: number;",
    ] {
        assert!(
            list_type.contains(contract),
            "CodexThemeListResult is missing: {contract}"
        );
    }
    assert!(
        types.contains("export type CodexThemeImportResult = CommandResult<CodexThemeSummary>;")
    );

    let operation_type = source_section(
        &types,
        "export type CodexThemeOperationResult",
        "export type CodexThemeOperationState",
    );
    for contract in [
        "CommandResult<{",
        "theme_id: string;",
        "persisted: boolean;",
        "runtime_applied: boolean;",
        "restart_required: boolean;",
        "rolled_back: boolean;",
        "generation: number;",
    ] {
        assert!(
            operation_type.contains(contract),
            "CodexThemeOperationResult is missing: {contract}"
        );
    }

    let theme = claude_codex_pro_core::codex_theme::CodexThemeSummary {
        id: "fixture-theme".to_string(),
        name: "Fixture Theme".to_string(),
        version: "1.0.0".to_string(),
        author: "Fixture Author".to_string(),
        description: "Fixture description".to_string(),
        preview_data_uri: Some("data:image/png;base64,fixture".to_string()),
        builtin: false,
        current: true,
        imported_at: 1,
        updated_at: 2,
        integrity_sha256: Some("fixture-sha256".to_string()),
        previous_version_available: true,
    };
    let list_json = serde_json::to_value(claude_codex_pro_manager_lib::commands::CommandResult {
        status: "ok".to_string(),
        message: "loaded".to_string(),
        payload: claude_codex_pro_core::codex_theme::CodexThemeList {
            themes: vec![theme.clone()],
            current_theme_id: theme.id.clone(),
            generation: 3,
        },
    })
    .expect("serialize theme list command result");
    assert_json_keys(
        &list_json,
        &[
            "status",
            "message",
            "themes",
            "current_theme_id",
            "generation",
        ],
    );
    assert_json_keys(
        &list_json["themes"][0],
        &[
            "id",
            "name",
            "version",
            "author",
            "description",
            "preview_data_uri",
            "builtin",
            "current",
            "imported_at",
            "updated_at",
            "integrity_sha256",
            "previous_version_available",
        ],
    );

    let import_json = serde_json::to_value(claude_codex_pro_manager_lib::commands::CommandResult {
        status: "ok".to_string(),
        message: "imported".to_string(),
        payload: theme,
    })
    .expect("serialize theme import command result");
    assert_json_keys(
        &import_json,
        &[
            "status",
            "message",
            "id",
            "name",
            "version",
            "author",
            "description",
            "preview_data_uri",
            "builtin",
            "current",
            "imported_at",
            "updated_at",
            "integrity_sha256",
            "previous_version_available",
        ],
    );

    let operation_json =
        serde_json::to_value(claude_codex_pro_manager_lib::commands::CommandResult {
            status: "ok".to_string(),
            message: "applied".to_string(),
            payload: claude_codex_pro_core::codex_theme::CodexThemeOperationResult {
                theme_id: "fixture-theme".to_string(),
                persisted: true,
                runtime_applied: false,
                restart_required: true,
                rolled_back: false,
                generation: 4,
                message: "applied".to_string(),
            },
        })
        .expect("serialize theme operation command result");
    assert_json_keys(
        &operation_json,
        &[
            "status",
            "message",
            "theme_id",
            "persisted",
            "runtime_applied",
            "restart_required",
            "rolled_back",
            "generation",
        ],
    );
}

#[test]
fn codex_theme_center_frontend_ipc_calls_match_tauri_commands() {
    fn ipc_call<'a>(source: &'a str, command: &str) -> &'a str {
        let marker = format!("\"{command}\"");
        for (command_index, _) in source.match_indices(&marker) {
            let prefix_start = source[..command_index]
                .char_indices()
                .rev()
                .nth(199)
                .map(|(index, _)| index)
                .unwrap_or(0);
            let prefix = &source[prefix_start..command_index];
            let Some(call_offset) = prefix.rfind("call") else {
                continue;
            };
            let call_start = prefix_start + call_offset;
            let Some(open_offset) = source[call_start..command_index].find('(') else {
                continue;
            };
            let open_index = call_start + open_offset;
            let mut depth = 0usize;
            let mut quote = None;
            let mut escaped = false;

            for (offset, character) in source[open_index..].char_indices() {
                if let Some(active_quote) = quote {
                    if escaped {
                        escaped = false;
                    } else if character == '\\' {
                        escaped = true;
                    } else if character == active_quote {
                        quote = None;
                    }
                    continue;
                }

                match character {
                    '\'' | '"' | '`' => quote = Some(character),
                    '(' => depth += 1,
                    ')' => {
                        depth = depth
                            .checked_sub(1)
                            .unwrap_or_else(|| panic!("unbalanced IPC call for {command}"));
                        if depth == 0 {
                            let end = open_index + offset + character.len_utf8();
                            return &source[call_start..end];
                        }
                    }
                    _ => {}
                }
            }
        }

        panic!("missing frontend IPC call for {command}");
    }

    let frontend = read_all_frontend_sources();
    let list_call = ipc_call(&frontend, "list_codex_themes");
    assert!(list_call.contains("call<CodexThemeListResult>"));
    assert_eq!(
        list_call
            .split_once("\"list_codex_themes\"")
            .expect("list command literal")
            .1
            .trim(),
        ")",
        "list_codex_themes must not receive frontend arguments"
    );

    let import_call = ipc_call(&frontend, "import_codex_theme");
    assert!(import_call.contains("call<CodexThemeImportResult>"));
    for argument in ["sourcePath", "replaceExisting"] {
        assert!(
            import_call.contains(argument),
            "import_codex_theme is missing camelCase IPC argument: {argument}"
        );
    }
    assert!(!import_call.contains("source_path"));
    assert!(!import_call.contains("replace_existing"));

    let apply_call = ipc_call(&frontend, "apply_codex_theme");
    assert!(apply_call.contains("call<CodexThemeOperationResult>"));
    assert!(apply_call.contains("themeId"));
    assert!(!apply_call.contains("theme_id"));

    let restore_call = ipc_call(&frontend, "restore_codex_default_theme");
    assert!(restore_call.contains("call<CodexThemeOperationResult>"));
    assert_eq!(
        restore_call
            .split_once("\"restore_codex_default_theme\"")
            .expect("restore command literal")
            .1
            .trim(),
        ")",
        "restore_codex_default_theme must not receive frontend arguments"
    );
}

#[test]
fn manager_window_and_ops_console_layout_stay_usable() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // 拆分后：存在性/禁止性断言读前端源码全集（字符串迁到哪个文件都能命中，
    // 且 !contains 覆盖所有前端文件，护栏更强）；结构化切片仍读 App.tsx 单文件。
    let app_tsx = read_all_frontend_sources();
    let screens_file = read_frontend_file("screens.tsx");
    let tauri_bridge = manifest_dir.parent().unwrap().join("src/tauriBridge.ts");
    let tauri_bridge = read_source_file(&tauri_bridge);
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = read_source_file(&styles);
    let lib_rs = read_source_file(&manifest_dir.join("src/lib.rs"));
    let commands_rs = read_source_file(&manifest_dir.join("src/commands.rs"));
    let tauri_conf = read_source_file(&manifest_dir.join("tauri.conf.json"));
    let launcher_main = read_source_file(
        &manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("claude-codex-pro-launcher/src/main.rs"),
    );

    assert!(app_tsx.contains("ops-shell"));
    assert!(app_tsx.contains("ops-rail"));
    assert!(app_tsx.contains("ops-commandbar"));
    assert!(app_tsx.contains("id: \"supplier\""));
    assert!(app_tsx.contains("label: \"供应商与路由\""));
    let routes_file = read_frontend_file("lib/routes.ts");
    let route_source = routes_file
        .split("const routes")
        .nth(1)
        .and_then(|rest| rest.split("function isRoute").next())
        .expect("manager route source");
    assert!(route_source.contains("id: \"maintenance\""));
    assert!(route_source.contains("label: \"维护与诊断\""));
    assert!(route_source.contains("id: \"about\""));
    assert!(route_source.contains("label: \"关于与更新\""));
    assert!(!route_source.contains("label: \"脚本\""));
    assert!(!route_source.contains("label: \"日志\""));
    assert!(app_tsx.contains("function SupplierScreen"));
    assert!(app_tsx.contains("switch_relay_profile"));
    assert!(app_tsx.contains("preview_claude_desktop_provider"));
    assert!(app_tsx.contains("apply_claude_desktop_provider"));
    assert!(app_tsx.contains("restore_claude_desktop_provider_official"));
    assert!(
        app_tsx.contains("if (value === \"relay\" || value === \"models\") return \"supplier\"")
    );
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
    assert!(!overview_screen.contains("拓扑熵减API"));
    assert!(!overview_screen.contains("Codex 诊断"));
    assert!(!overview_screen.contains("Claude 诊断"));
    assert!(!overview_screen.contains("installKind ?? \"unknown\""));
    assert!(!overview_screen.contains("cdpStatus ?? \"unknown\""));
    let announcement_panel = overview_screen
        .split("className=\"overview-announcement-card\"")
        .nth(1)
        .and_then(|rest| rest.split("<div className=\"ops-matrix\">").next())
        .expect("overview announcement card source");
    assert!(announcement_panel.contains("announcement.badge"));
    assert!(announcement_panel.contains("announcement.title"));
    assert!(announcement_panel.contains("announcement.description"));
    assert!(announcement_panel.contains("announcement.buttonLabel"));
    assert!(announcement_panel.contains("actions.openExternalUrl(announcement.url)"));
    assert!(!announcement_panel.contains("拓扑API是CCP官方中转站"));
    assert!(!announcement_panel.contains("https://api.toporeduce.cn"));
    assert!(!announcement_panel.contains("dangerouslySetInnerHTML"));
    assert!(app_tsx.contains("call<AdsResult>(\"load_ads\")"));
    assert!(app_tsx.contains("ads={ads}"));
    let memory_panel = overview_screen
        .split("title=\"盘古记忆总览\"")
        .nth(1)
        .and_then(|rest| rest.split("title=\"诊断与修复\"").next())
        .expect("memory overview panel source");
    assert!(!memory_panel.contains("Claude 一键开发模式"));
    assert!(memory_panel.contains("盘古记忆开关"));
    assert!(memory_panel.contains("运行状态"));
    assert!(memory_panel.contains("Codex 注入"));
    assert!(memory_panel.contains("对话监控"));
    assert!(memory_panel.contains("<MemoryActivityWave active={memoryMonitorActive} />"));
    assert!(!memory_panel.contains("查看/编辑经验教训"));
    assert!(!memory_panel.contains("memory-overview-matrix"));
    assert!(!memory_panel.contains("memory-overview-actions"));
    assert!(!memory_panel.contains("待确认"));
    assert!(!memory_panel.contains("actions.refineLongTermMemory()"));
    assert!(!memory_panel.contains("openMemoryDetails()"));
    assert!(overview_screen.contains("overview-side-stack"));
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
    assert!(styles.contains(".overview-announcement-card"));
    assert!(styles.contains(".overview-announcement-kicker"));
    assert!(styles.contains(".overview-memory-list"));
    assert!(styles.contains(".memory-overview-matrix"));
    assert!(styles.contains("grid-template-columns: repeat(2, minmax(0, 1fr));"));
    assert!(styles.contains(".memory-overview-actions"));
    assert!(styles.contains("grid-template-columns: repeat(3, minmax(0, 1fr));"));
    assert!(styles.contains("max-height: 360px;"));
    assert!(!overview_screen.contains("插件中心"));
    assert!(!overview_screen.contains("提示词工坊"));
    assert!(!overview_screen.contains("PromptOptimizerCard"));
    assert!(overview_screen.contains("刷新概览"));
    assert!(overview_screen.contains("actions.refreshRoute(\"overview\", { notify: true })"));
    assert!(overview_screen.contains("刷新 Claude 第三方配置"));
    assert!(overview_screen.contains("修复前端连接"));
    assert!(overview_screen.contains("修复后端服务"));
    assert!(overview_screen.contains("修复 Claude"));
    assert!(app_tsx.contains("refresh_claude_third_party_config"));
    assert!(app_tsx.contains("repair_frontend_connection"));
    assert!(app_tsx.contains("repair_backend_service"));
    assert!(app_tsx.contains("actions.restoreClaudeZhPatch()"));
    assert!(app_tsx.contains("options: { notify?: boolean } = {}"));
    assert!(app_tsx.contains("const shouldNotify = options.notify === true"));
    assert!(app_tsx.contains("setNotice({ title: refreshTitle, message: `正在刷新${routeLabel(target)}状态...`, status: \"running\" })"));
    assert!(app_tsx.contains("setNotice({ title: refreshTitle, message: `${routeLabel(target)}已刷新。`, status: \"ok\" })"));
    assert!(app_tsx.contains("void refreshRoute(route);"));
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
        .and_then(|rest| rest.split("export function MemoryScreen").next())
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
    assert!(app_tsx.contains("ops-command-search"));
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
    assert!(app_tsx.contains("role=\"dialog\""));
    assert!(app_tsx.contains("aria-modal=\"true\""));
    assert!(!styles.contains("notice-backdrop"));
    assert!(!styles.contains("notice-card"));
    assert!(lib_rs.contains(".inner_size(1180.0, 820.0)"));
    assert!(lib_rs.contains(".min_inner_size(960.0, 640.0)"));
    assert!(tauri_conf.contains("\"width\": 1180"));
    assert!(tauri_conf.contains("\"height\": 820"));
    assert!(tauri_conf.contains("\"minWidth\": 960"));
    assert!(tauri_conf.contains("\"minHeight\": 640"));
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
        "if (target === \"overview\") {\n      await Promise.all([refreshOverview(true), refreshAds(true), refreshClaudeLight(true), refreshClaudeDesktopDevMode(true), refreshSettings(true)]);"
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
    let app_tsx = read_all_frontend_sources();
    let commands_rs = read_source_file(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands.rs"
    )));

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
    assert!(restart_command.contains("find_restartable_launcher_processes()"));
    assert!(restart_command.contains("find_codex_processes()"));
    assert!(restart_command.contains("wait_for_processes_to_exit("));
    assert!(restart_command.contains("旧进程未能及时退出"));
    assert!(restart_command.contains("let restart_started_ms = current_time_ms();"));
    assert!(restart_command.contains("start_restart_injection_monitor("));
    assert!(
        restart_command
            .find("wait_for_processes_to_exit(")
            .expect("bounded process exit wait")
            < restart_command
                .find("spawn_claude_codex_pro_launch(")
                .expect("new launcher spawn")
    );
    // rustfmt may wrap the call across lines, so assert on the call and the
    // forwarded `request` argument separately rather than on a single substring.
    assert!(restart_command.contains("spawn_claude_codex_pro_launch("));
    assert!(restart_command.contains("request,"));
    assert!(commands_rs.contains("fn default_debug_port() -> u16 {\n    9230\n}"));
    assert!(!commands_rs.contains("fn default_debug_port() -> u16 {\n    9229\n}"));
}

#[test]
fn codex_restart_monitors_current_theme_and_new_renderer_injection_in_background() {
    let commands_rs = read_source_file(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands.rs"
    )));
    let monitor = commands_rs
        .split("fn start_restart_injection_monitor")
        .nth(1)
        .and_then(|rest| rest.split("fn spawn_silent_launcher").next())
        .expect("restart injection monitor source");

    assert!(monitor.contains("codex_frontend_injection_enabled(&settings)"));
    assert!(monitor.contains("CodexThemeStore::open_default()"));
    assert!(monitor.contains("theme.current_theme_id != \"default\""));
    assert!(monitor.contains("settings_injection_enabled || theme_injection_enabled"));
    assert!(monitor.contains("tauri::async_runtime::spawn(async move"));
    assert!(monitor.contains("wait_for_codex_launch_ports("));
    assert!(monitor.contains("wait_for_renderer_frontend_after("));
    assert!(monitor.contains("manager.restart_injection_monitor_started"));
    assert!(monitor.contains("manager.restart_injection_confirmed"));
    assert!(monitor.contains("manager.restart_injection_timeout"));
    assert!(monitor.contains("\"stage\": \"launch_ports\""));
    assert!(monitor.contains("\"stage\": \"renderer_heartbeat\""));
    assert!(monitor.contains("\"theme_id\": theme_id"));
    assert!(monitor.contains("\"theme_generation\": theme_generation"));
    assert!(!monitor.contains("app_path"));
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
    assert!(
        core_claude
            .contains(".filter(|path| path.is_file() && !is_windowsapps_executable_path(path))")
    );
    assert!(core_claude.contains("fn is_windowsapps_executable_path(path: &Path) -> bool"));
    assert!(core_claude.contains("normalized.contains(\"\\\\windowsapps\\\\\")"));
    assert!(core_claude.contains(".or_else(|| {"));
    assert!(core_claude.contains("claude_desktop_executable_path()"));
    assert!(
        core_claude.contains(".filter(|path| !is_windowsapps_executable_path(path.as_path()))")
    );
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
    let styles = read_source_file(&styles);

    let supplier_screen = screens_file
        .split("function SupplierScreen")
        .nth(1)
        .and_then(|rest| rest.split("function LegacySupplierScreen").next())
        .expect("supplier screen source");

    assert!(supplier_screen.contains("actions.saveSettings(next)"));
    assert!(supplier_screen.contains("actions.switchSupplierProfile"));
    assert!(
        supplier_screen
            .contains("actions.switchSupplierProfile(targetApp, normalized.id, nextSettings)")
    );
    assert!(supplier_screen.contains("applySupplier?: boolean"));
    assert!(supplier_screen.contains("saveDraft({ stayInEditor: true, applySupplier: true })"));
    assert!(supplier_screen.contains(
        "targetApp === \"claude-desktop\" && !!originalId && currentActiveId === originalId"
    ));
    assert!(!supplier_screen.contains(
        "switchSupplierProfile(savedProfile.targetApp || \"codex\", savedProfile.id, saved.settings)"
    ));
    assert!(
        !supplier_screen
            .contains("onClick={() => void actions.switchCodexRelayProfile(profile.id)}")
    );
    assert!(supplier_screen.contains("actions.fetchRelayProfileModels"));
    assert!(supplier_screen.contains("const originalId = editingId;"));
    assert!(supplier_screen.contains("profile.id === originalId ? normalized : profile"));
    assert!(supplier_screen.contains("withActiveSupplierId({"));
    assert!(supplier_screen.contains("activeClaudeRelayId"));
    assert!(supplier_screen.contains("activeClaudeDesktopRelayId"));
    assert!(supplier_screen.contains(
        "const nextActiveRelayId = !aggregateDraft && originalId && currentActiveId === originalId"
    ));
    assert!(supplier_screen.contains(
        "const nextForTarget = (_targetApp: SupplierTargetApp, currentId: string) => currentId === profile.id ? \"\" : currentId;"
    ));
    assert!(
        supplier_screen
            .contains("title: shouldApplySupplier ? \"供应商保存并应用\" : \"供应商保存\"")
    );
    assert!(supplier_screen.contains("const savedProfile = saved.relayProfiles.find((profile) => profile.id === normalized.id) ?? normalized;"));
    assert!(supplier_screen.contains("const saveDraft = async (options: { stayInEditor?: boolean; applySupplier?: boolean } = {}): Promise<SupplierSaveResult | null> => {"));
    assert!(supplier_screen.contains("setDraft(null);"));
    assert!(supplier_screen.contains("saveDraft({ stayInEditor: true, applySupplier: true })"));
    assert!(
        supplier_screen
            .contains("!normalized.name.trim() || (!aggregateDraft && !normalized.baseUrl.trim())")
    );
    assert!(!supplier_screen.contains(
        "!normalized.name.trim() || !normalized.baseUrl.trim() || !normalized.apiKey.trim()"
    ));
    assert!(supplier_screen.contains("API Key 可以后续补入"));
    assert!(supplier_screen.contains("该供应商缺少 API Key，未修改当前生效配置。"));
    assert!(app_tsx.contains(
        "const targetProfile = current.relayProfiles.find((profile) => profile.id === profileId);"
    ));
    assert!(app_tsx.contains("const previousActiveProfile = current.relayProfiles.find((profile) => profile.id === current.activeRelayId);"));
    assert!(app_tsx.contains("previousActiveProfile.targetApp === \"codex\""));
    assert!(app_tsx.contains("该供应商缺少 API Key。记录已可保存，请补入 Key 后再切换写入。"));
    assert!(!supplier_screen.contains("if (!requestedId || requestedId !== normalizedId)"));
    assert!(supplier_screen.contains("const idWasNormalized = requestedId !== normalizedId;"));
    assert!(supplier_screen.contains("actions.showNotice({ title: \"供应商保存\", message: `供应商 ID 已自动整理为「${savedProfile.id}」。`, status: \"ok\" });"));
    assert!(supplier_screen.contains("const updateNewDraftIdFromName = (value: string) => {"));
    assert!(supplier_screen.contains("if (!isNewDraft) return;"));
    assert!(
        supplier_screen.contains("const next = normalizeDraftProfile({ ...current, id: nextId });")
    );
    assert!(
        supplier_screen
            .contains("const nextId = uniqueSupplierProfileId(profiles, value || current.name);")
    );
    assert!(supplier_screen.contains(
        "SUPPLIER_PRESETS.filter((preset) => preset.id === \"openai\" || preset.id === \"anthropic\").map"
    ));
    assert!(
        supplier_screen
            .contains("onBlur={(event) => updateNewDraftIdFromName(event.currentTarget.value)}")
    );
    assert!(!supplier_screen.contains("<span>供应商 ID</span>"));
    assert!(!supplier_screen.contains("value={generated.id}"));
    assert!(!supplier_screen.contains("onChange={(event) => setDraft((current) => current ? { ...current, id: event.currentTarget.value } : current)} value={draft.id}"));
    assert!(!supplier_screen.contains("onChange={(event) => updateDraft({ id: supplierIdFromName(event.currentTarget.value) })} value={generated.id}"));
    assert!(!supplier_screen.contains("input disabled={!isNewDraft} onChange={(event) => updateDraft({ id: supplierIdFromName(event.currentTarget.value) })}"));
    assert!(supplier_screen.contains("createSupplierProfile(appSettings)"));
    assert!(supplier_screen.contains("withSupplierGeneratedFiles"));
    assert!(!supplier_screen.contains("SUPPLIER_PRESETS.map"));
    assert!(supplier_screen.contains("添加供应商"));
    assert!(supplier_screen.contains("编辑"));
    assert!(supplier_screen.contains("删除供应商"));
    assert!(app_tsx.contains("function buildSupplierConfigToml"));
    assert!(!supplier_screen.contains("model_provider = \"custom\""));
    assert!(!supplier_screen.contains("[model_providers.custom]"));
    assert!(app_tsx.contains("OPENAI_API_KEY"));
    assert!(app_tsx.contains("fetch_relay_profile_models"));
    assert!(
        app_tsx.contains("const hasExplicitModelList = typeof profile.modelList === \"string\";")
    );
    assert!(app_tsx.contains("modelList: hasExplicitModelList ? modelList : model,"));
    assert!(app_tsx.contains("const apiKey = supplierProfileResolvedApiKey(profile);"));
    assert!(app_tsx.contains("function supplierProfileResolvedApiKey(profile: RelayProfile)"));
    assert!(app_tsx.contains("supplierProfilePrefersConfigApiKey(profile)"));
    assert!(app_tsx.contains("configKey || authKey"));
    assert!(app_tsx.contains("authKey || configKey"));
    assert!(app_tsx.contains("function supplierApiKeyFromAuthContents(contents: string)"));
    assert!(app_tsx.contains("function supplierApiKeyFromConfigContents(contents: string)"));
    assert!(app_tsx.contains("ANTHROPIC_API_KEY"));
    assert!(app_tsx.contains("parsed.env"));
    assert!(app_tsx.contains("supplierProfileHasApiKey(targetProfile)"));
    assert!(
        supplier_screen.contains("shouldApplySupplier && !supplierProfileHasApiKey(normalized)")
    );
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
    let app_tsx = read_all_frontend_sources();
    let screens_file = read_frontend_file("screens.tsx").replace("\r\n", "\n");
    let supplier_lib = read_frontend_file("lib/supplier.ts").replace("\r\n", "\n");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = read_source_file(&styles);
    let commands_rs =
        std::fs::read_to_string(manifest_dir.join("src/commands.rs")).expect("read commands.rs");
    let lib_rs = std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs");

    let supplier_screen = screens_file
        .split("function SupplierScreen")
        .nth(1)
        .and_then(|rest| rest.split("function LegacySupplierScreen").next())
        .expect("supplier screen source");

    assert!(supplier_screen.contains("supplier-target-filter"));
    assert!(supplier_screen.contains("supplier-control-row"));
    assert!(supplier_screen.contains("supplier-route-master-toggle"));
    assert!(supplier_screen.contains("routableSupplierProfiles"));
    assert!(!supplier_screen.contains("visibleRoutableSupplierProfiles"));
    assert!(supplier_screen.contains("supplierRouteSwitchEnabled"));
    assert!(
        supplier_screen
            .contains("routableSupplierProfiles.some((profile) => !!profile.routeEnabled)")
    );
    assert!(
        !supplier_screen
            .contains("routableSupplierProfiles.some((profile) => supplierRouteEnabled(profile))")
    );
    assert!(supplier_screen.contains("toggleVisibleSupplierRouting"));
    assert!(supplier_screen.contains("const supplierRouteGroup = supplierTargetFilter;"));
    assert!(supplier_screen.contains("supplierRouteGroup === \"claude-desktop\""));
    assert!(supplier_screen.contains(
        "profiles.filter((profile) => supplierTargetForProfile(profile) === supplierRouteGroup)"
    ));
    assert!(!supplier_screen.contains("target === \"claude\" || target === \"claude-desktop\""));
    assert!(supplier_screen.contains("const withSupplierRoutingState = (profile: RelayProfile"));
    assert!(supplier_screen.contains("routeMode: targetApp === \"codex\""));
    assert!(
        supplier_screen
            .contains("return withSupplierRoutingState(profile, supplierRouteGroup, enabled);")
    );
    assert!(
        supplier_screen
            .contains("withSupplierRoutingState(profile, targetApp, !!profile.routeEnabled)")
    );
    assert!(supplier_screen.contains("supplierRouteGroupLabel"));
    assert!(!supplier_screen.contains("?????"));
    assert!(supplier_screen.contains("routeEnabled: enabled"));
    assert!(supplier_screen.contains(
        "const claudeDesktopMode = targetApp === \"codex\" ? \"\" : enabled ? \"proxy\" : \"direct\""
    ));
    assert!(supplier_screen.contains("claudeDesktopMode,"));
    assert!(supplier_screen.contains("supplierTargetFilter"));
    assert!(supplier_screen.contains("Codex"));
    assert!(supplier_screen.contains("Claude"));
    assert!(supplier_screen.contains("Claude Desktop"));
    assert!(supplier_screen.contains("ccswitch"));
    assert!(supplier_screen.contains(r#"<Edit className="h-4 w-4" />"#));
    assert!(supplier_screen.contains(r#"<Activity className="h-4 w-4" />"#));
    assert!(supplier_screen.contains(r#"<BarChart3 className="h-4 w-4" />"#));
    assert!(
        supplier_screen
            .contains("const supplierModelOptions = Array.from(new Set((modelFetch !== null")
    );
    assert!(supplier_screen.contains("? modelFetch.models"));
    assert!(
        supplier_screen
            .contains(": supplierDirectModelRows(generated.modelList).map((row) => row.model)")
    );
    assert!(!supplier_screen.contains("...(modelFetch?.models ?? [])"));
    assert!(
        !supplier_screen
            .contains("...modelRowsForDraft.flatMap((row) => [row.requestModel, row.displayName])")
    );
    assert!(supplier_screen.contains("const applyOneClickModelMapping = () => {"));
    assert!(
        supplier_screen
            .contains("lowerOptions.find(({ lower }) => lower.includes(row.role))?.option")
    );
    assert!(supplier_screen.contains("请先获取模型"));
    assert!(!supplier_screen.contains("<strong>是否开启路由</strong>"));
    assert!(!supplier_screen.contains("<span>路由</span><input"));
    assert!(!supplier_screen.contains("routeEnabled: nextRequiresRoute ? true"));
    assert!(supplier_screen.contains("请返回供应商列表开启"));
    assert!(screens_file.contains("function SupplierModelDropdown"));
    assert!(screens_file.contains("window.innerHeight"));
    assert!(screens_file.contains("role=\"listbox\""));
    assert!(supplier_screen.contains("<SupplierModelDropdown"));
    assert!(!supplier_screen.contains("<select className=\"supplier-model-map-select\""));
    assert!(supplier_screen.contains("row.displayName"));
    assert!(supplier_screen.contains("row.requestModel"));
    assert!(supplier_screen.contains("row.supports1m"));
    assert!(!supplier_screen.contains("?? API ??"));
    assert!(!supplier_screen.contains("???? API key"));
    assert!(supplier_screen.contains("supplierDisplayUrl"));
    assert!(!supplier_screen.contains(r#"<PencilRuler className="h-4 w-4" />"#));
    assert!(supplier_screen.contains("supplier-drop-popover"));
    assert!(supplier_screen.contains("const refreshSupplierList = async () => {"));
    assert!(supplier_screen.contains(r#"actions.refreshRoute("supplier", { notify: true })"#));
    assert!(supplier_screen.contains("supplierRefreshBusy"));
    assert!(supplier_screen.contains("刷新供应商列表"));

    assert!(!supplier_screen.contains(" onDragStart"));
    assert!(!supplier_screen.contains(" onDragEnter"));
    assert!(!supplier_screen.contains(" onDragOver"));
    assert!(!supplier_screen.contains(" onDrop"));
    assert!(!supplier_screen.contains(" draggable key={profile.id}"));
    assert!(!supplier_screen.contains("SUPPLIER_DRAG_MIME_TYPE"));
    assert!(!supplier_screen.contains("event.dataTransfer"));
    assert!(supplier_screen.contains("const beginSupplierPointerDrag = (event: ReactPointerEvent<HTMLElement>, profileId: string) => {"));
    assert!(
        supplier_screen
            .contains(r#"window.addEventListener("pointermove", handlePointerMove, true);"#)
    );
    assert!(
        supplier_screen
            .contains(r#"window.removeEventListener("pointermove", handlePointerMove, true);"#)
    );
    assert!(supplier_screen.contains("moveEvent.preventDefault();"));
    assert!(supplier_screen.contains("dragHandle.releasePointerCapture(event.pointerId);"));
    assert!(
        supplier_screen
            .contains("const [supplierOrderIds, setSupplierOrderIds] = useState<string[]>([]);")
    );
    assert!(supplier_screen.contains("const supplierOrderFromIds = (ids: string[]) => {"));
    assert!(supplier_screen.contains("const reorderSupplierIds = (sourceId: string, targetId: string, ids = supplierOrderIds) => {"));
    assert!(
        !supplier_screen
            .contains("const previewSupplierOrder = (sourceId: string, targetId: string) => {")
    );
    assert!(!supplier_screen.contains("dragStartOrderIds"));
    assert!(supplier_screen.contains("const baselineIds = supplierOrderFromIds(supplierOrderIds.length ? supplierOrderIds : profiles.map((profile) => profile.id))"));
    assert!(
        supplier_screen
            .contains("reorderSupplierIds(current.sourceId, targetId, current.latestIds)")
    );
    assert!(!supplier_screen.contains("current.baselineIds"));
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
    assert!(
        supplier_screen
            .contains("filteredOrderedProfiles.map((profile) => renderSupplierCard(profile))")
    );
    assert!(supplier_screen.contains("beginSupplierPointerDrag(event, profile.id)"));
    assert!(supplier_screen.contains("supplierTargetIdFromPointer"));
    assert!(supplier_screen.contains("const [supplierDragOverlay, setSupplierDragOverlay]"));
    assert!(supplier_screen.contains("setSupplierDragOverlay({"));
    assert!(supplier_screen.contains("top: moveEvent.clientY - overlay.offsetY"));
    assert!(supplier_screen.contains("setSupplierDragOverlay(null);"));
    assert!(supplier_screen.contains("drag-overlay-card"));
    assert!(supplier_screen.contains("drag-source"));
    assert!(supplier_screen.contains("supplier-card-action-button"));
    assert!(supplier_screen.contains("supplier-card-use-button"));
    assert!(!supplier_screen.contains("supplier-badge current"));
    assert!(!supplier_screen.contains("\\u4f7f\\u7528\\u4e2d</span>"));
    assert!(supplier_screen.contains("supplier-url-link"));

    assert!(supplier_screen.contains("supplier-ccswitch-editor"));
    assert!(supplier_screen.contains("source-parity"));
    assert!(supplier_screen.contains("supplier-ccswitch-form-card"));
    assert!(supplier_screen.contains("supplier-form-avatar"));
    assert!(supplier_screen.contains("supplier-mapping-card"));
    assert!(supplier_screen.contains("关闭时按原始模型 ID 直传"));
    assert!(supplier_screen.contains(
        "checked={!!generated.modelMappingEnabled} onChange={(value) => updateDraft({ modelMappingEnabled: value })}"
    ));
    assert!(
        !supplier_screen.contains("<ToggleSwitch checked disabled onChange={() => undefined} />")
    );
    assert!(supplier_lib.contains(
        "const modelMappingEnabled = typeof profile.modelMappingEnabled === \"boolean\""
    ));
    assert!(supplier_lib.contains("modelMappingEnabled,"));
    assert!(supplier_lib.contains("const routeRows = generated.modelMappingEnabled"));
    assert!(supplier_lib.contains("? supplierModelMappingRows(generated).filter"));
    assert!(
        !supplier_screen
            .contains("modelMappingEnabled: enabled ? true : profile.modelMappingEnabled")
    );
    assert!(supplier_screen.contains("supplier-advanced-card"));
    assert!(supplier_screen.contains("supplier-codex-catalog-grid"));
    assert!(supplier_lib.contains("export type SupplierCodexCatalogRow"));
    assert!(supplier_lib.contains("export function supplierCodexCatalogRows"));
    assert!(supplier_lib.contains("export function supplierCodexCatalogJson"));
    assert!(supplier_lib.contains("codexCatalogJson: profile.codexCatalogJson ?? \"\""));
    assert!(app_tsx.contains("codexCatalogJson?: string;"));
    assert!(supplier_screen.contains("const addSupplierCodexCatalogModel = () =>"));
    assert!(supplier_screen.contains("const updateSupplierCodexCatalogModel = ("));
    assert!(supplier_screen.contains("const removeSupplierCodexCatalogModel = ("));
    assert!(supplier_screen.contains("supplierCodexCatalogJson(rows)"));
    assert!(supplier_screen.contains("if (draft.targetApp === \"codex\") return;"));
    assert!(supplier_screen.contains("例如: DeepSeek V4 Flash"));
    assert!(supplier_screen.contains("例如: deepseek-v4-flash"));
    assert!(supplier_screen.contains("例如: 128000"));
    assert!(supplier_screen.contains("SUPPLIER_USER_AGENT_PRESETS"));
    assert!(screens_file.contains("claude-cli/2.1.161 (external, cli)"));
    assert!(screens_file.contains("claude-cli/2.1.161"));
    assert!(screens_file.contains("claude-code/1.0.0"));
    assert!(screens_file.contains("claude-code/0.1.0"));
    assert!(screens_file.contains("Kilo-Code/1.0"));
    assert!(supplier_screen.contains("API Key"));
    assert!(supplier_screen.contains("SUPPLIER_API_FORMAT_OPTIONS"));
    assert!(supplier_screen.contains("supplierModelMappingJson(rows)"));
    assert!(supplier_screen.contains("supplierModelMappingText(rows)"));
    assert!(
        supplier_lib
            .contains("{ role: \"subagent\", label: \"Subagent\", routeId: \"claude-subagent\"")
    );
    assert!(
        supplier_screen
            .contains("modelRowsForDraft.find((row) => row.role === \"subagent\")?.requestModel")
    );
    assert!(supplier_lib.contains("export type SupplierDirectModelRow"));
    assert!(supplier_lib.contains("export function supplierDirectModelRows"));
    assert!(supplier_lib.contains("export function supplierDirectModelList"));
    assert!(supplier_lib.contains("export function supplierDirectModelIsClaudeDesktopSafe"));
    assert!(
        supplier_lib
            .contains("const hasExplicitModelList = typeof profile.modelList === \"string\";")
    );
    assert!(supplier_lib.contains("modelList: hasExplicitModelList ? modelList : model,"));
    assert!(supplier_screen.contains("generated.modelMappingEnabled ? ("));
    assert!(supplier_screen.contains("手动指定 Claude Desktop 模型列表（高级，可选）"));
    assert!(supplier_screen.contains("supplier-direct-model-list"));
    assert!(supplier_screen.contains("const addSupplierDirectModel = () =>"));
    assert!(supplier_screen.contains("const updateSupplierDirectModel = ("));
    assert!(supplier_screen.contains("const removeSupplierDirectModel = ("));
    assert!(supplier_screen.contains("supplierDirectModelList(rows)"));
    assert!(supplier_screen.contains("if (isClaudeTarget && draft.modelMappingEnabled) return;"));
    assert!(supplier_screen.contains("supplierModelFetchRequestRef"));
    assert!(
        supplier_screen.contains("if (requestId !== supplierModelFetchRequestRef.current) return;")
    );
    assert!(supplier_screen.contains("row.rowId"));
    assert!(supplier_screen.contains("supplierDirectModelIsClaudeDesktopSafe"));
    assert!(supplier_screen.contains("Claude Desktop 直连模型 ID 无效"));
    assert!(supplier_screen.contains("redactSupplierConfig"));
    assert!(supplier_screen.contains("readOnly={!showSupplierApiKey}"));
    assert!(supplier_lib.contains("\"clientsecret\""));
    assert!(supplier_lib.contains("\"password\""));
    assert!(supplier_lib.contains("\"privatekey\""));
    assert!(supplier_lib.contains("normalized.endsWith(\"secret\")"));
    assert!(supplier_screen.contains("获取模型列表"));
    assert!(supplier_screen.contains("添加模型"));
    assert!(supplier_screen.contains("aria-label=\"删除模型\""));
    assert!(!supplier_screen.contains("modelMappingEnabled: true, modelList:"));
    assert!(app_tsx.contains("model: row.requestModel.trim()"));
    assert!(app_tsx.contains("return [key, row.requestModel.trim()];"));
    assert!(
        !app_tsx.contains(r#"model: `${row.requestModel.trim()}${row.supports1m ? " [1M]" : ""}`"#)
    );
    assert!(!app_tsx.contains(
        r#"return [key, `${row.requestModel.trim()}${row.supports1m ? " [1M]" : ""}`];"#
    ));
    assert!(supplier_screen.contains("auth.json"));
    assert!(supplier_screen.contains("config.toml"));
    assert!(supplier_screen.contains("Chat Completions"));
    assert!(supplier_screen.contains("Responses"));
    assert!(supplier_screen.contains("supplierTestConfigOpen"));
    assert!(supplier_screen.contains("supplierPricingConfigOpen"));
    assert!(supplier_screen.contains("setSupplierTestConfigOpen"));
    assert!(supplier_screen.contains("setSupplierPricingConfigOpen"));
    assert!(!supplier_screen.contains("supplier-ccswitch-collapse-card expanded"));
    assert!(supplier_screen.contains("supplier-ccswitch-savebar"));

    assert!(app_tsx.contains("Anthropic / Claude"));
    assert!(app_tsx.contains("Anthropic Messages"));
    assert!(app_tsx.contains("OpenAI Chat Completions"));
    assert!(app_tsx.contains("OpenAI Responses API"));
    assert!(app_tsx.contains("Gemini Native generateContent"));
    assert!(supplier_screen.contains("routeEnabled"));
    assert!(supplier_screen.contains("const routeEnabled = !!generated.routeEnabled;"));
    assert!(supplier_lib.contains("export function supplierRouteEnabled(profile: RelayProfile)"));
    assert!(supplier_lib.contains("return !!profile.routeEnabled;"));
    assert!(
        supplier_lib.contains(
            "claudeDesktopMode: supplierRouteEnabled(generated) ? \"proxy\" : \"direct\""
        )
    );

    assert!(supplier_screen.contains("聚合策略"));
    assert!(app_tsx.contains("失败切换"));
    assert!(app_tsx.contains("按对话轮转"));
    assert!(app_tsx.contains("按请求轮转"));
    assert!(app_tsx.contains("权重轮转"));
    assert!(supplier_screen.contains("aggregate.strategy / aggregate.members"));

    assert!(app_tsx.contains("importCcswitchCodexProviders"));
    assert!(app_tsx.contains("import_ccswitch_codex_providers"));
    assert!(commands_rs.contains("pub fn import_ccswitch_codex_providers"));
    assert!(lib_rs.contains("commands::import_ccswitch_codex_providers"));
    assert!(styles.contains(".supplier-list-shell"));
    assert!(styles.contains(".supplier-drop-popover"));
    assert!(styles.contains(".supplier-card.dragging"));
    assert!(styles.contains(".supplier-card.drag-over"));
    assert!(styles.contains(".supplier-aggregate-grid"));
    assert!(styles.contains(".supplier-card.drag-overlay-card"));
    assert!(styles.contains("position: fixed"));
    assert!(styles.contains(".supplier-card.drag-source"));
    assert!(styles.contains(".supplier-card-action-button"));
    assert!(styles.contains(".supplier-card-use-button"));
    assert!(styles.contains(".supplier-url-link"));
    assert!(styles.contains(".supplier-ccswitch-editor"));
    assert!(styles.contains("grid-template-rows: auto minmax(0, 1fr) auto;"));
    assert!(styles.contains("overflow-y: auto;"));
    assert!(styles.contains(".supplier-control-row"));
    assert!(styles.contains("justify-content: space-between;"));
    assert!(styles.contains("width: 100%;"));
    assert!(styles.contains("max-width: none;"));
    assert!(styles.contains("padding: 32px max(16px, calc((100% - 880px) / 2));"));
    assert!(styles.contains("textarea.supplier-config-json"));
    assert!(styles.contains(".supplier-route-master-toggle"));
    assert!(!styles.contains(".supplier-model-map-select"));
    assert!(styles.contains(".supplier-model-dropdown-trigger"));
    assert!(styles.contains(".supplier-model-dropdown-menu"));
    assert!(styles.contains("position: fixed;"));
    assert!(styles.contains("overflow-y: auto;"));
    assert!(styles.contains(".supplier-ccswitch-form-grid.two"));
    assert!(styles.contains("align-items: start;"));
    assert!(styles.contains("align-content: start;"));
    assert!(styles.contains("top: 50%;"));
    assert!(styles.contains("transform: translate(0, -50%);"));
    assert!(styles.contains("transform: translate(22px, -50%);"));
    assert!(styles.contains(
        ".supplier-ccswitch-collapse-card:not(.expanded) .supplier-ccswitch-collapse-head"
    ));
    assert!(styles.contains(".supplier-ccswitch-collapse-body"));
}

#[test]
fn claude_dev_mode_button_preserves_the_active_supplier_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_tsx = manifest_dir.parent().unwrap().join("src/App.tsx");
    let app_tsx = std::fs::read_to_string(&app_tsx).expect("read manager App.tsx");
    let app_tsx = app_tsx.replace("\r\n", "\n");
    let core_plugin_hub =
        manifest_dir.join("../../../crates/claude-codex-pro-core/src/plugin_hub.rs");
    let core_plugin_hub =
        std::fs::read_to_string(core_plugin_hub).expect("read core plugin_hub.rs");

    assert!(app_tsx.contains("const providerRequest = claudeDesktopProviderDraft.apiKey.trim()"));
    assert!(app_tsx.contains("? claudeDesktopProviderDraft\n      : null;"));
    assert!(app_tsx.contains(
        "const request = providerRequest?.baseUrl.trim() ? { request: providerRequest } : undefined;"
    ));
    assert!(app_tsx.contains(
        "call<ClaudeDesktopDevModeConfigureResult>(\"configure_claude_desktop_dev_mode\", request)"
    ));
    assert!(!app_tsx.contains("activeDesktopProfile"));
    assert!(!app_tsx.contains("supplierProfileResolvedApiKey"));
    assert!(core_plugin_hub.contains("active_relay_profile_for_target(\"claude-desktop\")"));
    assert!(
        core_plugin_hub
            .contains("has_saved_desktop_supplier.then_some(relay.model_mapping_enabled)")
    );
    assert!(core_plugin_hub.contains("relay.model_mapping_json.clone()"));
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
    assert!(!codex_inject.contains("chineseOverlayEnabled: \"claudeAppChineseOverlayEnabled\""));
    assert!(codex_inject.contains("settings.chineseOverlayEnabled = false;"));
    assert!(!codex_inject.contains("ensureClaudeChineseOverlayObserver();"));
    assert!(!codex_inject.contains("runScanStep(refreshClaudeChineseOverlay);"));

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
    assert!(claude_inject.contains("existingRuntime?.ready"));
    assert!(claude_inject.contains("const runtime = { ready: false, refresh: null };"));
    assert!(claude_inject.contains("runtime.ready = true;"));
    assert!(claude_inject.contains("right.length - left.length"));
}

#[test]
fn audit_remediation_frontend_contracts_are_locked_down() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let app = read_frontend_file("App.tsx");
    let screens = read_frontend_file("screens.tsx");
    let update = read_frontend_file("lib/update.ts");
    let button = read_frontend_file("components/ui/button.tsx");
    let styles = read_frontend_file("styles.css");
    let commands = read_source_file(&manifest_dir.join("src/commands.rs"));

    assert!(update.contains("expectedVersion: updateInfo.latestVersion"));
    assert!(app.contains("{ expectedVersion: release.expectedVersion }"));
    assert!(!app.contains("call<UpdateResult>(\"perform_update\", release ? { release }"));
    assert!(commands.contains("pub async fn perform_update(\n    app: tauri::AppHandle,"));
    assert!(commands.contains("claude_codex_pro_core::update::fetch_current_release().await"));
    assert!(commands.contains("app.emit(\"update-download-progress\", progress)"));
    assert!(app.contains("listen<UpdateDownloadProgress>(\"update-download-progress\""));
    assert!(screens.contains("className=\"update-download-progress-track\""));
    assert!(screens.contains("disabled={updateRunning || !release || !updateInfo?.assetUrl}"));
    assert!(screens.contains("已获取最新 Release 版本与安装资源。"));
    assert!(button.contains("active:scale-[0.98]"));
    assert!(styles.contains("@keyframes update-download-indeterminate"));

    assert!(app.contains("const settingsDraftRevisionRef = useRef(0);"));
    assert!(app.contains("settingsDraftRevisionRef.current += 1;"));
    assert!(app.contains("const draftRevision = beginSettingsDraftRequest();"));
    assert!(app.contains("commitSettingsDraftRequest(draftRevision, result.settings);"));
    assert!(app.contains("const requestId = ++memorySearchRequestRef.current;"));
    assert!(app.contains("requestId === memorySearchRequestRef.current"));

    assert!(screens.contains("readOnly value={visibleCodexAuthJson}"));
    assert!(screens.contains("readOnly value={visibleCodexConfigToml}"));
    assert!(screens.contains("value={visibleHeaderOverride}"));
    assert!(screens.contains("value={visibleBodyOverride}"));
    assert!(screens.contains("readOnly={!showSupplierApiKey}"));
    assert!(screens.contains("const loadFailed = Boolean(data && statusFailed(data.status));"));
    assert!(screens.contains("role=\"alert\""));
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
        .split(
            "const refreshRoute = async (target = route, options: { notify?: boolean } = {}) => {",
        )
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
    assert!(core_zh_patch.contains("icacls Administrators 组"));
    assert!(core_zh_patch.contains("*S-1-5-32-544:(OI)(CI)F"));
    assert!(!core_zh_patch.contains("*S-1-5-32-545:(OI)(CI)M"));
    assert!(core_zh_patch.contains("user_grant.is_err() && admins_grant.is_err()"));
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
    assert!(validation.contains("command.creation_flags(crate::windows_create_no_window())"));
    assert!(validation.contains("let output = command.output()"));
    assert!(validation.contains("String::from_utf8_lossy(&output.stderr)"));
    assert!(validation.contains(".take(8)"));
    assert!(validation.contains("node --check failed for"));
    assert!(!validation.contains("command.stderr(std::process::Stdio::null())"));
}

#[test]
fn plugin_hub_marketplace_and_worktree_git_spawns_suppress_console_window() {
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

    // Codex marketplace 自动修复会在管理器启动时执行多个 Git 子命令；该异步
    // helper 同样必须隐藏窗口，否则每个 clone / sparse-checkout 都会弹出终端。
    let codex_marketplace = std::fs::read_to_string(core_dir.join("codex_plugin_marketplace.rs"))
        .expect("read core codex_plugin_marketplace.rs");
    let safe_git_command = codex_marketplace
        .split("fn safe_git_command(hooks: &Path) -> tokio::process::Command")
        .nth(1)
        .and_then(|rest| rest.split("\nfn ").next())
        .expect("safe_git_command source");
    assert!(safe_git_command.contains("tokio::process::Command::new(\"git\")"));
    assert!(safe_git_command.contains("creation_flags(crate::windows_create_no_window())"));

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
    assert!(repair.contains("ensure_detached_helper(helper_port)"));
    assert!(repair.contains("wait_helper_backend_online(helper_port).await"));
    assert!(repair.contains("正在自动启动本地 helper 后端"));
    assert!(!repair.contains("请先点击“修复后端服务”"));
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
    assert!(restart.contains("let old_launcher_pids ="));
    assert!(restart.contains("let old_codex_pids ="));
    assert!(restart.contains("stop_codex_processes()"));
    assert!(restart.contains("wait_for_processes_to_exit"));
    assert!(restart.contains("force_kill_process_tree_for_frontend_repair(&old_process_pids)"));
    assert!(restart.contains("旧 launcher/Codex 进程仍未退出"));
    assert!(restart.contains("select_repair_debug_port(default_debug_port()).await"));
    assert!(restart.contains("debug_port: selected_debug_port"));
    assert!(restart.contains("let launch_started_at_ms = current_time_ms();"));
    assert!(restart.contains("launch_started_at_ms,"));
    assert!(restart.contains("正在等待 Codex 自启完成"));
    assert!(commands_rs.contains("taskkill.exe"));
    assert!(commands_rs.contains(".args([\"/PID\", &pid.to_string(), \"/F\", \"/T\"])"));

    let wait_ports = commands_rs
        .split("async fn wait_for_codex_launch_ports")
        .nth(1)
        .and_then(|rest| {
            rest.split("async fn wait_for_renderer_frontend_after")
                .next()
        })
        .expect("wait_for_codex_launch_ports source");
    assert!(wait_ports.contains("repair_launch_status("));
    assert!(wait_ports.contains("helper 后端仍需恢复"));
    assert!(wait_ports.contains("launch_started_at_ms"));
    assert!(wait_ports.contains("codex_debug_port_online(request.debug_port)"));
    assert!(wait_ports.contains("helper_backend_online(request.helper_port)"));
    assert!(wait_ports.contains("status.started_at_ms >= launch_started_at_ms"));
    assert!(wait_ports.contains("status.debug_port.is_some()"));
    assert!(wait_ports.contains("status.debug_port_online"));
    assert!(wait_ports.contains("if !requested_debug_port_online"));

    let wait = commands_rs
        .split("async fn wait_for_renderer_frontend_after")
        .nth(1)
        .expect("wait_for_renderer_frontend_after source");
    assert!(wait.contains("heartbeat.timestamp_ms >= min_timestamp_ms"));
    assert!(wait.contains("renderer_frontend_heartbeat_confirms_injection(&heartbeat)"));
}

#[test]
fn manager_status_rejects_renderer_heartbeat_from_previous_codex_launch() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands_rs = manifest_dir.join("src/commands.rs");
    let commands_rs = std::fs::read_to_string(&commands_rs).expect("read manager commands.rs");

    assert!(commands_rs.contains("fn renderer_heartbeat_is_current("));
    assert!(commands_rs.contains("timestamp_ms >= launch_started_at_ms"));

    let memory_status = commands_rs
        .split("fn enrich_memory_status")
        .nth(1)
        .and_then(|rest| rest.split("fn normalize_memory_runtime_status").next())
        .expect("enrich_memory_status source");
    assert!(memory_status.contains("renderer_heartbeat_is_current("));

    let launch_status = commands_rs
        .split("fn refresh_launch_port_status")
        .nth(1)
        .and_then(|rest| rest.split("fn codex_debug_port_online").next())
        .expect("refresh_launch_port_status source");
    assert!(launch_status.contains("renderer_heartbeat_is_current("));
}

#[test]
fn settings_and_tools_route_keep_full_ops_controls() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // 存在性断言读前端源码全集；结构化切片读 screens.tsx（SettingsScreen 已拆到 screens.tsx）。
    let app_tsx = read_all_frontend_sources();
    let screens_file = read_frontend_file("screens.tsx");
    let styles = manifest_dir.parent().unwrap().join("src/styles.css");
    let styles = read_source_file(&styles);

    assert!(app_tsx.contains("function ToolsAndPluginsScreen"));
    assert!(app_tsx.contains("function MaintenanceToolsPanel"));
    assert!(app_tsx.contains("label: \"插件、Skills 与 MCP\""));
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

#[test]
fn about_screen_exposes_contact_entrypoints() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manager_root = manifest_dir.parent().unwrap();
    let screens = read_frontend_file("screens.tsx");
    let styles = read_frontend_file("styles.css");
    let qr_asset = manager_root.join("src/assets/contact-wechat-qr.jpg");

    assert!(qr_asset.exists());
    assert!(screens.contains("contactWechatQr"));
    assert!(screens.contains("CONTACT_QQ_GROUP_PRIMARY_URL"));
    assert!(screens.contains("CONTACT_QQ_GROUP_SECONDARY_URL"));
    assert!(screens.contains("合作请联系微信"));
    assert!(screens.contains("官方QQ群："));
    assert!(screens.contains("10061615"));
    assert!(screens.contains("1076215359"));
    assert!(screens.contains("一键添加"));
    assert!(!screens.contains("合作代理请联系微信"));
    assert!(screens.contains("扫码添加微信，备注合作代理。"));
    assert!(screens.contains("actions.openExternalUrl(CONTACT_QQ_GROUP_PRIMARY_URL)"));
    assert!(screens.contains("actions.openExternalUrl(CONTACT_QQ_GROUP_SECONDARY_URL)"));
    assert!(screens.contains("type=\"button\""));
    assert!(screens.contains("https://qm.qq.com/cgi-bin/qm/qr?k=uwNon9opx0Arfovyo5qJQQ2jUvlxSpmf&jump_from=webapi&authKey=El8Xwz9ZqefrpE4BhW9xWQsEAUFvptw74MBsRKRJTw5x5QiEPiG0fmdVIf9VuMWg"));
    assert!(screens.contains("https://qm.qq.com/cgi-bin/qm/qr?k=cIeUYUFyy0ypTWMqo8CfgRwq8jU_OrXy&jump_from=webapi&authKey=njT7ceHMggvpptkiy9xD6FbBubVGCDof0cnX0adhLgUvi9kKZP4OY51M1xWZBy68"));
    assert!(styles.contains(".contact-card"));
    assert!(styles.contains(".contact-line"));
    assert!(styles.contains(".contact-link"));
    assert!(styles.contains(".contact-wechat"));
    assert!(styles.contains(".contact-qr"));
}

#[test]
fn pangu_memory_new_project_guide_keeps_lazy_loading_and_complete_contract() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manager_root = manifest_dir.parent().expect("manager root");
    let commands = read_source_file(&manifest_dir.join("src/commands.rs"));
    let command_registry = read_source_file(&manifest_dir.join("src/lib.rs"));
    let app = read_frontend_file("App.tsx");
    let screens = read_frontend_file("screens.tsx");
    let types = read_frontend_file("types.ts");

    assert!(commands.contains("pub async fn load_memory_new_project_guide()"));
    assert!(commands.contains("MemoryAssistStore::default().new_project_guide()"));
    assert!(commands.contains("guide: MemoryNewProjectGuide::default()"));
    assert!(command_registry.contains("commands::load_memory_new_project_guide"));
    assert!(commands.contains("fn restrict_manager_memory_workspace(workspace: &str) -> String"));
    assert!(
        commands
            .contains("request.workspace = restrict_manager_memory_workspace(&request.workspace);")
    );
    assert!(
        commands.contains("let workspace = restrict_manager_memory_workspace(&request.workspace);")
    );
    assert!(commands.contains("let range_days = if request.range_days <= 7 { 7 } else { 30 };"));
    assert!(commands.contains(
        "MemoryAssistStore::default().outcome_dashboard(&requested_workspace, range_days)"
    ));
    assert!(commands.contains("MemoryAssistStore::default().list_candidates(&workspace, true)"));

    for field in [
        "generatedAt: number",
        "sourceItemCount: number",
        "sourceWorkspaceCount: number",
        "pitfalls: MemoryNewProjectExperience[]",
        "bestPractices: MemoryNewProjectExperience[]",
        "prompt: string",
    ] {
        assert!(
            types.contains(field),
            "missing frontend guide field: {field}"
        );
    }
    assert!(types.contains("workspaceBreakdown: Array<{ key: string; count: number }>"));
    assert!(types.contains("categoryBreakdown: Array<{ key: string; count: number }>"));
    assert!(screens.contains("workspace-${item.key}"));
    assert!(screens.contains("category-${item.key}"));

    let initial_memory_refresh = app
        .split("const refreshMemoryAssist = async")
        .nth(1)
        .and_then(|rest| rest.split("const refreshMemoryOutcomeDashboard").next())
        .expect("initial memory refresh source");
    assert!(!initial_memory_refresh.contains("load_memory_new_project_guide"));
    assert!(!initial_memory_refresh.contains("workspace: MEMORY_ALL_WORKSPACES"));
    assert!(app.contains("call<MemoryNewProjectGuideResult>(\"load_memory_new_project_guide\")"));
    assert!(app.contains("guide && statusOk(guide.status)"));

    for copy in [
        "继续当前项目",
        "开启新项目",
        "最近更新：",
        "来源范围",
        "源记忆",
        "精选经验",
        "完整提示词",
        "真实命中",
        "高级诊断",
    ] {
        assert!(screens.contains(copy), "missing memory UI copy: {copy}");
    }
    assert!(screens.contains("<details className=\"memory-diagnostics\">"));

    let styles = read_source_file(&manager_root.join("src/styles.css"));
    assert!(styles.contains(".memory-start-grid"));
    assert!(styles.contains(".memory-new-project-preview"));
    assert!(styles.contains("@media (max-width:"));
}

#[test]
fn session_context_messages_do_not_shrink_and_hide_their_bodies() {
    let styles = read_frontend_file("styles.css");
    let workspace = read_frontend_file("workspace.css");
    let message_rule = source_section(&workspace, ".claude-session-context-message {\n", "\n}");

    assert!(
        styles.contains(".claude-session-context-body {") && styles.contains("overflow-y: auto;"),
        "the shared Codex and Claude message list must remain vertically scrollable"
    );
    assert!(
        message_rule.contains("flex: 0 0 auto;"),
        "session message cards must not shrink and clip their body text"
    );
    assert!(
        message_rule.contains("min-width: 0;"),
        "session message cards must still fit the context viewer width"
    );
}
#[test]
fn windows_private_file_acl_command_stays_hidden() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let settings_path = manifest_dir.join("../../../crates/claude-codex-pro-core/src/settings.rs");
    let settings = read_source_file(&settings_path);
    let secure_private_path = source_section(
        &settings,
        "#[cfg(windows)]\nfn secure_private_path",
        "#[cfg(not(any(unix, windows)))]",
    );

    assert!(secure_private_path.contains("CommandExt"));
    assert!(secure_private_path.contains("creation_flags(windows_create_no_window())"));
}

#[test]
fn provider_sync_skipped_status_is_not_reported_as_success() {
    let commands = include_str!("../src/commands.rs");
    let section = source_section(
        commands,
        "pub async fn sync_providers_now",
        "fn is_success_sync_status",
    );

    assert!(section.contains("failed(&message, payload)"));
    assert!(section.contains("供应商同步未执行：{}"));
}

#[test]
fn history_session_repair_toasts_progress_and_restarts_codex_only_after_success() {
    let app_tsx = read_frontend_file("App.tsx");
    let repair_action = app_tsx
        .split("const repairHistorySessions = async () => {")
        .nth(1)
        .and_then(|rest| rest.split("const deleteLocalSession").next())
        .expect("history session repair action source");

    assert!(repair_action.contains("title: \"历史会话修复\""));
    assert!(repair_action.contains("message: \"正在修复历史会话，请稍候。\""));
    assert!(repair_action.contains("status: \"running\""));
    assert!(repair_action.contains("await waitForPaint()"));
    assert!(repair_action.contains("statusOk(result.status)"));
    assert!(repair_action.contains("即将重启 Codex"));
    assert!(repair_action.contains("await restartCodex(true)"));
    assert!(
        repair_action.find("statusOk(result.status)").unwrap()
            < repair_action.find("await restartCodex(true)").unwrap()
    );
}

#[test]
fn repair_restart_skips_only_the_duplicate_provider_sync() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands = read_source_file(&manifest_dir.join("src/commands.rs"));
    let app = read_frontend_file("App.tsx");
    let launcher = read_source_file(
        &manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("claude-codex-pro-launcher/src/main.rs"),
    );

    assert!(app.contains("const restartCodex = async (skipProviderSync = false)"));
    assert!(app.contains("skipProviderSync"));
    assert!(commands.contains("pub skip_provider_sync: bool"));
    assert!(commands.contains("command.arg(\"--skip-provider-sync\")"));
    assert!(launcher.contains("skip_provider_sync_requested"));
    assert!(launcher.contains("if self.skip_provider_sync"));
}

#[test]
fn provider_sync_keeps_rollout_contents_on_disk_instead_of_memory() {
    let source = include_str!("../../../../crates/claude-codex-pro-data/src/provider_sync.rs");
    let change = source_section(source, "struct SessionChange", "struct RolloutRewrite");

    assert!(!change.contains("original_text"));
    assert!(!change.contains("next_text"));
    assert!(source.contains("backup_dir.join(\"session-files\")"));
    assert!(source.contains("fs::read_to_string(&change.path)"));
}
