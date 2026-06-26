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
