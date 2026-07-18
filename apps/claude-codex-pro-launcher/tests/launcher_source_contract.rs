#[test]
fn launcher_does_not_open_manager_for_update_prompts() {
    let source = include_str!("../src/main.rs");

    assert!(!source.contains("notify_manager_when_update_available"));
    assert!(!source.contains("open_manager_with_update_prompt"));
    assert!(!source.contains("--show-update"));
    assert!(!source.contains("check_for_update"));
}

#[test]
fn launcher_runtime_uses_default_launch_debug_port() {
    let source = include_str!("../src/main.rs");

    assert!(source.contains("LaunchOptions::default().debug_port"));
    assert!(!source.contains("LauncherRuntimeService::new(\r\n                9229"));
    assert!(!source.contains("LauncherRuntimeService::new(\n                9229"));
}

#[test]
fn launcher_watchdog_reinjects_with_full_context_and_updates_status() {
    let source = include_str!("../src/main.rs");
    let watchdog = source
        .split("async fn start_bridge_watchdog")
        .nth(1)
        .and_then(|rest| {
            rest.split("async fn start_computer_use_guard_watchdog")
                .next()
        })
        .expect("LauncherHooks start_bridge_watchdog implementation");

    assert!(watchdog.contains("bridge_health_ok(debug_port)"));
    assert!(watchdog.contains("BridgeContext::core_with_data_and_app_dir"));
    assert!(watchdog.contains("inject_with_context("));
    assert!(watchdog.contains("status.status = \"running\".to_string()"));
    assert!(watchdog.contains("status_store.save_latest(&status)"));
}

#[test]
fn launcher_registers_theme_as_an_isolated_new_document_script() {
    let source = include_str!("../src/main.rs");
    let injection = source
        .split("async fn try_inject_with_context")
        .nth(1)
        .and_then(|rest| rest.split("fn default_codex_db_path").next())
        .expect("try_inject_with_context implementation");

    let target_selection = injection
        .find("pick_injectable_codex_page_target")
        .expect("Codex target selection");
    let theme_loading = injection
        .find("codex_theme_new_document_script()")
        .expect("theme new-document script loading");
    assert!(target_selection < theme_loading);
    assert!(injection.contains("let mut new_document_scripts = vec![script];"));
    assert!(injection.contains("new_document_scripts.push(theme_script);"));
    assert!(injection.contains("new_document_scripts.push(user_bundle);"));
    assert!(injection.contains("&new_document_scripts"));
}

#[test]
fn launcher_theme_loading_failure_is_non_blocking_and_redacted() {
    let source = include_str!("../src/main.rs");
    let helpers = source
        .split("fn log_codex_theme_injection_skipped")
        .nth(1)
        .and_then(|rest| rest.split("async fn inject_with_context").next())
        .expect("theme injection helper implementations");

    assert!(helpers.contains("CodexThemeStore::open_default()"));
    assert!(helpers.contains("store.active_theme_payload()"));
    assert!(helpers.contains("assets::codex_theme_injection_script(&payload)"));
    assert!(helpers.contains("launcher.codex_theme_injection_skipped"));
    assert!(helpers.contains("\"runtime_applied\": false"));
    assert!(helpers.contains("return None;"));
    assert!(!helpers.contains("error.to_string()"));
    assert!(!helpers.contains("\"error\""));
}

#[test]
fn active_non_default_theme_enables_injection_for_new_and_existing_codex_instances() {
    let source = include_str!("../src/main.rs");

    assert!(source.contains("fn active_codex_theme_requires_injection() -> bool"));
    assert!(source.contains("let enabled = !payload.is_default;"));
    assert!(source.contains("fn codex_theme_injection_enabled(&self) -> bool"));
    assert!(source.contains("active_codex_theme_requires_injection()"));
    assert!(source.contains("settings_injection_enabled || theme_injection_enabled"));
    assert!(source.contains("launcher.codex_theme_injection_requirement"));
    assert!(source.contains("launcher.codex_theme_script_ready"));
}
