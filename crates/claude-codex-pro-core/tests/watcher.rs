use claude_codex_pro_core::watcher::{
    build_spawn_launcher_command, build_watcher_install_plan, cdp_listening, codex_process_ids,
    disable_watcher_at, enable_watcher_at, filter_killable_launcher_processes,
    filter_restartable_launcher_processes, should_recover_stale_launcher, watcher_disabled_flag,
};

#[test]
fn cdp_listening_returns_true_for_bound_loopback_port() {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();

    assert!(cdp_listening(port));
}

#[test]
fn cdp_listening_returns_true_for_bound_ipv6_loopback_port() {
    let listener = std::net::TcpListener::bind("[::1]:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    assert!(cdp_listening(port));
}

#[test]
fn cdp_listening_returns_false_for_closed_port() {
    let port = {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.local_addr().unwrap().port()
    };

    assert!(!cdp_listening(port));
}

#[test]
fn watcher_enable_and_disable_toggle_flag() {
    let dir = tempfile::tempdir().unwrap();
    let flag = watcher_disabled_flag(dir.path());

    disable_watcher_at(dir.path()).unwrap();
    assert!(flag.exists());

    enable_watcher_at(dir.path()).unwrap();
    assert!(!flag.exists());
}

#[test]
fn watcher_install_plan_registers_rust_launcher_at_logon() {
    let plan = build_watcher_install_plan("C:/Tools/claude-codex-pro.exe".into(), 9333);

    assert_eq!(plan.run_value_name, "ClaudeCodexProWatcher");
    assert_eq!(
        plan.run_value,
        "\"C:/Tools/claude-codex-pro.exe\" --debug-port 9333"
    );
    assert_eq!(plan.shortcut_name, "ClaudeCodexProWatcher.lnk");
    assert_eq!(plan.shortcut_target, "C:/Tools/claude-codex-pro.exe");
    assert_eq!(plan.shortcut_arguments, "--debug-port 9333");
}

#[test]
fn spawn_launcher_command_points_to_silent_binary_only() {
    let command = build_spawn_launcher_command("C:/Tools/claude-codex-pro.exe", 9444);

    assert_eq!(command[0], "C:/Tools/claude-codex-pro.exe");
    assert!(command.contains(&"--debug-port".to_string()));
    assert!(command.contains(&"9444".to_string()));
    assert!(!command.iter().any(|part| part.contains("manager")));
}

#[test]
fn codex_process_filter_keeps_windowsapps_and_normal_codex_processes() {
    let processes = [
        (
            11,
            r"C:\Program Files\WindowsApps\OpenAI.Codex_1.0.0.0_x64__abc\app\Codex.exe",
        ),
        (12, r"C:\Tools\Codex.exe"),
        (
            13,
            r"C:\Program Files\WindowsApps\Other.App_1.0.0.0_x64__abc\app\Codex.exe",
        ),
        (
            15,
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.707.3748.0_x64__abc\app\ChatGPT.exe",
        ),
        (14, r"C:\Tools\not-codex.exe"),
    ];

    assert_eq!(codex_process_ids(processes), vec![11, 12, 13, 15]);
}

#[test]
fn launcher_process_filter_protects_current_process_ancestry() {
    let processes = [
        (10, 0, "claude-codex-pro.exe"),
        (20, 10, "claude-codex-pro.exe"),
        (30, 20, "claude-codex-pro.exe"),
        (40, 10, "claude-codex-pro.exe"),
        (50, 10, "claude-codex-pro-manager.exe"),
    ];

    assert_eq!(filter_killable_launcher_processes(processes, 30), vec![40]);
}

#[test]
fn repair_restart_launcher_filter_only_protects_current_process() {
    let processes = [
        (10, "claude-codex-pro.exe"),
        (20, "claude-codex-pro-manager.exe"),
        (30, "claude-codex-pro.exe"),
        (40, "codex.exe"),
    ];

    assert_eq!(
        filter_restartable_launcher_processes(processes, 20),
        vec![10, 30]
    );
    assert_eq!(
        filter_restartable_launcher_processes(processes, 30),
        vec![10]
    );
}

#[test]
fn stale_launcher_recovery_only_runs_when_codex_and_cdp_are_absent() {
    assert!(should_recover_stale_launcher(false, false));
    assert!(!should_recover_stale_launcher(true, false));
    assert!(!should_recover_stale_launcher(false, true));
    assert!(!should_recover_stale_launcher(true, true));
}
