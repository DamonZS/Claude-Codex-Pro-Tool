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
