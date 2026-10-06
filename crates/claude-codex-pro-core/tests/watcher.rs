use claude_codex_pro_core::watcher::{
    build_spawn_launcher_command, build_watcher_install_plan, cdp_listening, codex_process_ids,
    disable_watcher_at, enable_watcher_at, filter_killable_launcher_processes,
    filter_restartable_launcher_processes, filter_same_executable_processes,
    parse_macos_running_process_inventory, should_recover_stale_launcher,
    wait_for_process_ids_to_exit_with, watcher_disabled_flag,
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
        "\"C:/Tools/claude-codex-pro.exe\" --launcher --debug-port 9333"
    );
    assert_eq!(plan.shortcut_name, "ClaudeCodexProWatcher.lnk");
    assert_eq!(plan.shortcut_target, "C:/Tools/claude-codex-pro.exe");
    assert_eq!(plan.shortcut_arguments, "--launcher --debug-port 9333");
}

#[test]
fn spawn_launcher_command_points_to_silent_binary_only() {
    let command = build_spawn_launcher_command("C:/Tools/claude-codex-pro.exe", 9444);

    assert_eq!(command[0], "C:/Tools/claude-codex-pro.exe");
    assert!(command.contains(&"--launcher".to_string()));
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
fn codex_process_filter_recognizes_macos_app_main_process_only() {
    let processes = [
        (11, "/Applications/Codex.app/Contents/MacOS/ChatGPT"),
        (
            12,
            "/Applications/Codex.app/Contents/Frameworks/Codex Framework.framework/Versions/Current/Helpers/Codex (Renderer).app/Contents/MacOS/Codex (Renderer)",
        ),
        (13, "/Applications/Codex.app/Contents/Resources/codex"),
        (14, "/Users/test/.local/bin/codex"),
    ];

    assert_eq!(codex_process_ids(processes), vec![11]);
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
fn repair_restart_launcher_filter_recognizes_macos_silent_launcher_only() {
    let processes = [
        (
            10,
            "/Applications/Claude Codex Pro.app/Contents/MacOS/claude-codex-pro",
        ),
        (
            20,
            "/Applications/Claude Codex Pro Manager.app/Contents/MacOS/claude-codex-pro-manager",
        ),
        (30, "/tmp/claude-codex-pro"),
    ];

    assert_eq!(
        filter_restartable_launcher_processes(processes, 30),
        vec![10]
    );
}

#[test]
fn manager_exit_filter_selects_only_other_instances_of_the_full_executable_path() {
    let root = std::env::temp_dir();
    let executable = root.join("ccp-exit-install/claude-codex-pro.exe");
    let other_install = root.join("ccp-other-install/claude-codex-pro.exe");
    let codex = root.join("Codex/Codex.exe");
    let claude = root.join("Claude/Claude.exe");
    let relative = std::path::Path::new("claude-codex-pro.exe");
    let processes = [
        (10, Some(executable.as_path())),
        (20, Some(executable.as_path())),
        (30, Some(other_install.as_path())),
        (40, None),
        (50, Some(relative)),
        (60, Some(codex.as_path())),
        (70, Some(claude.as_path())),
    ];

    assert_eq!(
        filter_same_executable_processes(processes, 10, &executable),
        vec![20]
    );
    assert!(filter_same_executable_processes(processes, 10, relative).is_empty());
}

#[test]
#[cfg(windows)]
fn manager_exit_filter_normalizes_windows_case_and_separators() {
    assert_eq!(
        filter_same_executable_processes(
            [(
                20,
                Some(std::path::Path::new("c:/tools/CCP/claude-codex-pro.EXE"))
            )],
            10,
            std::path::Path::new(r"C:\Tools\ccp\claude-codex-pro.exe"),
        ),
        vec![20]
    );
}

#[test]
#[cfg(not(windows))]
fn manager_exit_filter_preserves_case_on_other_platforms() {
    assert!(
        filter_same_executable_processes(
            [(
                20,
                Some(std::path::Path::new("/Applications/CCP/claude-codex-pro"))
            )],
            10,
            std::path::Path::new("/Applications/ccp/claude-codex-pro"),
        )
        .is_empty()
    );
}

#[test]
fn stale_launcher_recovery_only_runs_when_codex_and_cdp_are_absent() {
    assert!(should_recover_stale_launcher(false, false));
    assert!(!should_recover_stale_launcher(true, false));
    assert!(!should_recover_stale_launcher(false, true));
    assert!(!should_recover_stale_launcher(true, true));
}

#[test]
fn process_exit_wait_rechecks_until_every_requested_pid_is_gone() {
    let mut snapshots = [vec![101, 202], vec![202], vec![]].into_iter();
    let mut checks = 0;
    let mut sleeps = 0;

    let exited = wait_for_process_ids_to_exit_with(
        &[101, 202],
        3,
        || {
            checks += 1;
            snapshots.next().unwrap_or_default()
        },
        || sleeps += 1,
    );

    assert!(exited);
    assert_eq!(checks, 3);
    assert_eq!(sleeps, 2);
}

#[test]
fn process_exit_wait_stops_after_the_bounded_number_of_checks() {
    let mut checks = 0;
    let mut sleeps = 0;

    let exited = wait_for_process_ids_to_exit_with(
        &[101],
        3,
        || {
            checks += 1;
            vec![101]
        },
        || sleeps += 1,
    );

    assert!(!exited);
    assert_eq!(checks, 3);
    assert_eq!(sleeps, 2);
}

#[test]
fn macos_process_inventory_excludes_zombies_from_running_processes() {
    let output = "  101 Ss   /Applications/Codex.app/Contents/MacOS/ChatGPT\n\
                  202 Z    /Applications/Claude Codex Pro.app/Contents/MacOS/claude-codex-pro\n\
                  303 S+   /tmp/claude-codex-pro\n";

    assert_eq!(
        parse_macos_running_process_inventory(output),
        vec![
            (
                101,
                "/Applications/Codex.app/Contents/MacOS/ChatGPT".to_string()
            ),
            (303, "/tmp/claude-codex-pro".to_string()),
        ]
    );
}
