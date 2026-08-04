use claude_codex_pro_core::credential_environment::current_user_credential_environment_value_result;
#[cfg(windows)]
use claude_codex_pro_core::credential_environment::with_temporary_codex_user_credential_environment_from_home;
use claude_codex_pro_core::credential_environment::{
    analyze_credential_environment, clear_codex_user_credential_environment,
    diagnose_codex_credential_environment, valid_environment_variable_name,
};
use claude_codex_pro_core::settings::{BackendSettings, RelayMode, RelayProfile};

fn settings_for_environment(name: &str) -> BackendSettings {
    let profile = RelayProfile {
        id: "test".to_string(),
        api_key: "current".to_string(),
        relay_mode: RelayMode::PureApi,
        config_contents: format!(
            "model_provider = \"test\"\n[model_providers.test]\nenv_key = \"{name}\"\n"
        ),
        ..RelayProfile::default()
    };
    BackendSettings {
        active_relay_id: "test".to_string(),
        relay_profiles: vec![profile],
        ..BackendSettings::default()
    }
}

#[cfg(windows)]
fn current_user_environment_value(name: &str) -> Option<String> {
    current_user_environment_entry(name).map(|(_, value)| value)
}

#[cfg(windows)]
fn current_user_environment_entry(name: &str) -> Option<(String, String)> {
    let output = std::process::Command::new("reg.exe")
        .args(["query", r"HKCU\Environment", "/v", name])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().find_map(|line| {
        let line = line.trim();
        let mut fields = line.split_whitespace();
        (fields.next()? == name).then(|| {
            let value_type = fields.next().unwrap_or_default();
            let value = line
                .find(value_type)
                .map(|index| line[index + value_type.len()..].trim().to_string())
                .unwrap_or_default();
            (value_type.to_string(), value)
        })
    })
}

#[cfg(windows)]
fn set_current_user_environment_value(name: &str, value: &str) {
    set_current_user_environment_value_with_type(name, "REG_SZ", value);
}

#[cfg(windows)]
fn set_current_user_environment_value_with_type(name: &str, value_type: &str, value: &str) {
    let output = std::process::Command::new("reg.exe")
        .args([
            "add",
            r"HKCU\Environment",
            "/v",
            name,
            "/t",
            value_type,
            "/d",
            value,
            "/f",
        ])
        .output()
        .expect("write temporary user environment value");
    assert!(output.status.success());
}

struct ScopedTestEnvironment {
    name: String,
    process_value: Option<std::ffi::OsString>,
    #[cfg(windows)]
    user_value: Option<String>,
}

impl ScopedTestEnvironment {
    fn cleared(name: &str) -> Self {
        let process_value = std::env::var_os(name);
        #[cfg(windows)]
        let user_value = current_user_environment_value(name);

        unsafe {
            std::env::remove_var(name);
        }
        #[cfg(windows)]
        clear_codex_user_credential_environment(&settings_for_environment(name), name).unwrap();

        Self {
            name: name.to_string(),
            process_value,
            #[cfg(windows)]
            user_value,
        }
    }
}

impl Drop for ScopedTestEnvironment {
    fn drop(&mut self) {
        #[cfg(windows)]
        match self.user_value.as_deref() {
            Some(value) => set_current_user_environment_value(&self.name, value),
            None => {
                let _ = clear_codex_user_credential_environment(
                    &settings_for_environment(&self.name),
                    &self.name,
                );
            }
        }

        match self.process_value.as_ref() {
            Some(value) => unsafe { std::env::set_var(&self.name, value) },
            None => unsafe { std::env::remove_var(&self.name) },
        }
    }
}

#[test]
fn matching_environment_value_is_not_a_conflict() {
    let result = analyze_credential_environment(
        "OPENAI_API_KEY",
        "sk-current",
        Some("sk-current"),
        Some("sk-current"),
        None,
    );

    assert!(result.present);
    assert!(!result.conflict);
    assert!(result.user_present);
    assert!(result.process_present);
}

#[test]
fn mismatched_environment_value_is_a_conflict_without_exposing_secrets() {
    let result = analyze_credential_environment(
        "OPENAI_API_KEY",
        "sk-current",
        Some("bad"),
        Some("different"),
        None,
    );

    assert!(result.conflict);
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(!serialized.contains("sk-current"));
    assert!(!serialized.contains("different"));
    assert!(!serialized.contains("bad"));
}

#[test]
fn environment_without_profile_key_is_reported_but_not_called_a_conflict() {
    let result =
        analyze_credential_environment("OPENAI_API_KEY", "", Some("inherited"), None, None);

    assert!(result.present);
    assert!(!result.conflict);
}

#[test]
fn a_detected_user_session_environment_is_clearable_on_every_platform() {
    let result = analyze_credential_environment(
        "OPENAI_API_KEY",
        "current",
        Some("current"),
        Some("current"),
        None,
    );

    assert!(result.can_clear_user);
    assert!(!result.external_source_likely);
}

#[test]
fn a_process_only_environment_is_reported_as_externally_managed() {
    let result =
        analyze_credential_environment("OPENAI_API_KEY", "current", Some("current"), None, None);

    assert!(result.external_source_likely);
    assert!(result.can_clear_user);
    assert!(!result.user_scope.is_empty());
}

#[test]
fn unix_user_session_cleanup_uses_platform_stores_without_editing_shell_profiles() {
    let source = include_str!("../src/credential_environment.rs").replace("\r\n", "\n");
    for required in [
        "#[cfg(target_os = \"macos\")]",
        "Command::new(\"launchctl\")",
        "\"getenv\"",
        "\"unsetenv\"",
        "#[cfg(target_os = \"linux\")]",
        "Command::new(\"systemctl\")",
        "\"show-environment\"",
        "\"unset-environment\"",
    ] {
        assert!(
            source.contains(required),
            "missing Unix session environment contract: {required}"
        );
    }
    for forbidden in [
        ".zshrc",
        ".bashrc",
        ".bash_profile",
        "/etc/environment",
        "environment.d",
    ] {
        assert!(
            !source.contains(forbidden),
            "credential cleanup must not edit shell or system profile source: {forbidden}"
        );
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn set_native_user_session_environment(name: &str, value: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("launchctl");
    #[cfg(target_os = "macos")]
    command.args(["setenv", name, value]);

    #[cfg(target_os = "linux")]
    let mut command = std::process::Command::new("systemctl");
    #[cfg(target_os = "linux")]
    command.args(["--user", "set-environment", &format!("{name}={value}")]);

    let output = command.output().map_err(|error| {
        anyhow::anyhow!("native user session environment setup failed: {error}")
    })?;
    if !output.status.success() {
        anyhow::bail!(
            "native user session environment setup exited with status {:?}",
            output.status.code()
        );
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn restore_native_user_session_environment(name: &str, value: Option<&str>) -> anyhow::Result<()> {
    match value {
        Some(value) => set_native_user_session_environment(name, value),
        None => {
            #[cfg(target_os = "macos")]
            let mut command = std::process::Command::new("launchctl");
            #[cfg(target_os = "macos")]
            command.args(["unsetenv", name]);

            #[cfg(target_os = "linux")]
            let mut command = std::process::Command::new("systemctl");
            #[cfg(target_os = "linux")]
            command.args(["--user", "unset-environment", name]);

            let output = command.output().map_err(|error| {
                anyhow::anyhow!("native user session environment restore failed: {error}")
            })?;
            if !output.status.success() {
                anyhow::bail!(
                    "native user session environment restore exited with status {:?}",
                    output.status.code()
                );
            }
            Ok(())
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct NativeUserSessionEnvironmentGuard {
    name: String,
    process_value: Option<std::ffi::OsString>,
    user_value: Option<String>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl NativeUserSessionEnvironmentGuard {
    fn install(name: &str, value: &str) -> Option<Self> {
        let user_value = match current_user_credential_environment_value_result(name) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("skipping native user-session test: {error}");
                return None;
            }
        };
        let process_value = std::env::var_os(name);
        set_native_user_session_environment(name, value)
            .expect("install native user session test environment");
        unsafe {
            std::env::set_var(name, value);
        }
        Some(Self {
            name: name.to_string(),
            process_value,
            user_value,
        })
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Drop for NativeUserSessionEnvironmentGuard {
    fn drop(&mut self) {
        let _ = restore_native_user_session_environment(&self.name, self.user_value.as_deref());
        match self.process_value.as_ref() {
            Some(value) => unsafe { std::env::set_var(&self.name, value) },
            None => unsafe { std::env::remove_var(&self.name) },
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn native_user_session_cleanup_removes_session_and_manager_copies() {
    const NAME: &str = "CCP_TEST_NATIVE_CREDENTIAL_ENV_CLEANUP";
    let Some(_environment) = NativeUserSessionEnvironmentGuard::install(NAME, "session-value")
    else {
        return;
    };

    let settings = settings_for_environment(NAME);
    let before = diagnose_codex_credential_environment(&settings);
    assert!(before.present);
    assert!(before.user_present);
    assert!(before.process_present);
    assert!(before.user_scope_available);

    let cleared = clear_codex_user_credential_environment(&settings, NAME).unwrap();
    assert!(!cleared.user_present);
    assert!(!cleared.process_present);
    assert!(cleared.restart_required);
    assert_eq!(
        current_user_credential_environment_value_result(NAME).unwrap(),
        None
    );
    assert!(std::env::var_os(NAME).is_none());
}

#[cfg(target_os = "linux")]
#[test]
fn linux_unavailable_user_manager_is_reported_without_claiming_cleanup() {
    let settings = settings_for_environment("CCP_TEST_LINUX_USER_MANAGER");
    let diagnostic = diagnose_codex_credential_environment(&settings);
    if diagnostic.user_scope_available {
        return;
    }
    assert!(diagnostic.user_scope_error.is_some());
    assert!(!diagnostic.can_clear_user);
}

#[test]
fn cleanup_variable_name_validation_is_strict() {
    assert!(valid_environment_variable_name("OPENAI_API_KEY"));
    assert!(valid_environment_variable_name("CCP_TEST_123"));
    assert!(!valid_environment_variable_name(""));
    assert!(!valid_environment_variable_name("OPENAI-API-KEY"));
    assert!(!valid_environment_variable_name("OPENAI_API_KEY=bad"));
    assert!(!valid_environment_variable_name("CODEX_HOME\\test"));
}

#[test]
fn cleanup_does_not_mutate_the_manager_copy_before_user_scope_succeeds() {
    let source = include_str!("../src/credential_environment.rs").replace("\r\n", "\n");
    let cleanup = source
        .split("pub fn clear_codex_user_credential_environment(")
        .nth(1)
        .and_then(|rest| rest.split("pub fn valid_environment_variable_name").next())
        .expect("credential environment cleanup source");
    let user_scope_cleanup = cleanup
        .find("clear_current_user_credential_environment_value(requested_name)?")
        .expect("user-scope cleanup");
    let manager_cleanup = cleanup
        .find("std::env::remove_var(requested_name)")
        .expect("Manager process cleanup");

    assert!(user_scope_cleanup < manager_cleanup);
}

#[test]
fn temporary_launch_environment_serializes_registry_and_process_mutation() {
    let source = include_str!("../src/credential_environment.rs").replace("\r\n", "\n");
    assert!(source.contains("TemporaryCodexCredentialEnvironmentLock::acquire()"));

    let helper_start = source
        .find("pub fn with_temporary_codex_user_credential_environment_from_home<T>(")
        .expect("temporary credential environment helper");
    let helper = &source[helper_start..];
    let lock = helper
        .find("TemporaryCodexCredentialEnvironmentLock::acquire()")
        .expect("cross-process temporary environment lock");
    let apply = helper
        .find("TemporaryUserCredentialEnvironment::apply")
        .expect("temporary environment mutation");
    assert!(
        lock < apply,
        "the lock must cover apply, activation, and restore"
    );
}

#[cfg(windows)]
fn write_temporary_provider_home(home: &std::path::Path, name: &str, credential: &str) {
    std::fs::create_dir_all(home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!("model_provider = \"custom\"\n[model_providers.custom]\nenv_key = \"{name}\"\n"),
    )
    .unwrap();
    std::fs::write(
        home.join("auth.json"),
        format!(r#"{{"OPENAI_API_KEY":"{credential}"}}"#),
    )
    .unwrap();
}

#[cfg(windows)]
#[test]
fn windows_scoped_environment_child_process() {
    let Some(home) = std::env::var_os("CCP_TEST_SCOPED_ENV_CHILD_HOME") else {
        return;
    };
    let ready_path = std::env::var_os("CCP_TEST_SCOPED_ENV_CHILD_READY");
    let hold_millis = std::env::var("CCP_TEST_SCOPED_ENV_CHILD_HOLD_MS")
        .unwrap()
        .parse::<u64>()
        .unwrap();

    with_temporary_codex_user_credential_environment_from_home(std::path::Path::new(&home), || {
        if let Some(ready_path) = ready_path.as_ref() {
            std::fs::write(ready_path, b"ready")?;
        }
        std::thread::sleep(std::time::Duration::from_millis(hold_millis));
        Ok(())
    })
    .unwrap();
}

#[cfg(windows)]
#[test]
fn windows_temporary_launch_environment_serializes_across_launcher_processes() {
    const NAME: &str = "CCP_TEST_CREDENTIAL_ENV_CROSS_PROCESS";
    let _environment = ScopedTestEnvironment::cleared(NAME);
    let temp = tempfile::tempdir().unwrap();
    let first_home = temp.path().join("first");
    let second_home = temp.path().join("second");
    let first_ready = temp.path().join("first-ready");
    write_temporary_provider_home(&first_home, NAME, "first-temporary-credential");
    write_temporary_provider_home(&second_home, NAME, "second-temporary-credential");

    let test_binary = std::env::current_exe().unwrap();
    let mut first = std::process::Command::new(&test_binary)
        .args([
            "--exact",
            "windows_scoped_environment_child_process",
            "--nocapture",
        ])
        .env("CCP_TEST_SCOPED_ENV_CHILD_HOME", &first_home)
        .env("CCP_TEST_SCOPED_ENV_CHILD_READY", &first_ready)
        .env("CCP_TEST_SCOPED_ENV_CHILD_HOLD_MS", "500")
        .env_remove(NAME)
        .spawn()
        .unwrap();

    let started = std::time::Instant::now();
    while !first_ready.exists() && started.elapsed() < std::time::Duration::from_secs(60) {
        if let Some(status) = first.try_wait().unwrap() {
            panic!("first launcher exited before activation scope: {status}");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        first_ready.exists(),
        "first launcher did not enter activation scope"
    );

    let mut second = std::process::Command::new(&test_binary)
        .args([
            "--exact",
            "windows_scoped_environment_child_process",
            "--nocapture",
        ])
        .env("CCP_TEST_SCOPED_ENV_CHILD_HOME", &second_home)
        .env("CCP_TEST_SCOPED_ENV_CHILD_HOLD_MS", "1000")
        .env_remove(NAME)
        .spawn()
        .unwrap();

    assert!(first.wait().unwrap().success());
    assert!(second.wait().unwrap().success());
    assert!(
        current_user_environment_value(NAME).is_none(),
        "overlapping launcher processes must not restore another launcher's temporary credential"
    );
}

#[cfg(windows)]
#[test]
fn windows_cleanup_waits_for_temporary_launch_before_deleting_the_previous_value() {
    const NAME: &str = "CCP_TEST_CREDENTIAL_ENV_DELETE_DURING_LAUNCH";
    let _environment = ScopedTestEnvironment::cleared(NAME);
    set_current_user_environment_value(NAME, "stale-user-value");

    let temp = tempfile::tempdir().unwrap();
    let provider_home = temp.path().join("provider");
    let ready_path = temp.path().join("launch-ready");
    write_temporary_provider_home(&provider_home, NAME, "temporary-launch-credential");

    let test_binary = std::env::current_exe().unwrap();
    let mut launcher = std::process::Command::new(&test_binary)
        .args([
            "--exact",
            "windows_scoped_environment_child_process",
            "--nocapture",
        ])
        .env("CCP_TEST_SCOPED_ENV_CHILD_HOME", &provider_home)
        .env("CCP_TEST_SCOPED_ENV_CHILD_READY", &ready_path)
        .env("CCP_TEST_SCOPED_ENV_CHILD_HOLD_MS", "800")
        .env_remove(NAME)
        .spawn()
        .unwrap();

    let started = std::time::Instant::now();
    while !ready_path.exists() && started.elapsed() < std::time::Duration::from_secs(10) {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        ready_path.exists(),
        "launcher did not enter activation scope"
    );

    let cleared = clear_codex_user_credential_environment(&settings_for_environment(NAME), NAME)
        .expect("cleanup must serialize with temporary launch injection");
    assert!(launcher.wait().unwrap().success());
    assert!(!cleared.user_present);
    assert!(
        current_user_environment_value(NAME).is_none(),
        "cleanup must win after the temporary launcher restores its previous value"
    );
}

#[cfg(windows)]
#[test]
fn windows_missing_user_environment_value_is_distinct_from_a_registry_read_error() {
    let unique_name = format!(
        "CCP_TEST_MISSING_VALUE_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    assert_eq!(
        current_user_credential_environment_value_result(&unique_name).unwrap(),
        None
    );
}

#[cfg(windows)]
#[test]
fn windows_cleanup_removes_only_the_named_user_environment_value() {
    use std::process::Command;

    const NAME: &str = "CCP_TEST_CREDENTIAL_ENV_CLEANUP";
    let _environment = ScopedTestEnvironment::cleared(NAME);
    let registry_path = r"HKCU\Environment";
    let add = Command::new("reg.exe")
        .args([
            "add",
            registry_path,
            "/v",
            NAME,
            "/t",
            "REG_SZ",
            "/d",
            "stale",
            "/f",
        ])
        .output()
        .unwrap();
    assert!(add.status.success());

    let profile = RelayProfile {
        id: "test".to_string(),
        api_key: "current".to_string(),
        relay_mode: RelayMode::PureApi,
        config_contents: format!(
            "model_provider = \"test\"\n[model_providers.test]\nenv_key = \"{NAME}\"\n"
        ),
        ..RelayProfile::default()
    };
    let settings = BackendSettings {
        active_relay_id: "test".to_string(),
        relay_profiles: vec![profile],
        ..BackendSettings::default()
    };

    let before = diagnose_codex_credential_environment(&settings);
    assert!(before.user_present);
    assert!(before.conflict);

    let cleared = clear_codex_user_credential_environment(&settings, NAME).unwrap();
    assert!(!cleared.user_present);
    assert!(cleared.restart_required);

    let query = Command::new("reg.exe")
        .args(["query", registry_path, "/v", NAME])
        .output()
        .unwrap();
    assert!(!query.status.success());
}

#[cfg(windows)]
#[test]
fn windows_process_only_environment_can_be_cleared_and_keeps_external_source_warning() {
    const NAME: &str = "CCP_TEST_PROCESS_ONLY_CREDENTIAL_ENV_CLEANUP";
    let _environment = ScopedTestEnvironment::cleared(NAME);
    unsafe {
        std::env::set_var(NAME, "inherited-test-value");
    }
    let settings = settings_for_environment(NAME);

    let before = diagnose_codex_credential_environment(&settings);
    assert!(before.process_present);
    assert!(!before.user_present);
    assert!(before.can_clear_user);
    assert!(before.external_source_likely);

    let cleared = clear_codex_user_credential_environment(&settings, NAME).unwrap();

    assert!(!cleared.present);
    assert!(!cleared.process_present);
    assert!(cleared.restart_required);
    assert!(cleared.external_source_likely);
    assert!(std::env::var_os(NAME).is_none());
}

#[cfg(windows)]
#[test]
fn windows_deleted_user_environment_is_not_recreated_for_the_next_codex_launch() {
    const NAME: &str = "CCP_TEST_CREDENTIAL_ENV_DELETE_PERSISTENCE";
    const CREDENTIAL: &str = "test-launch-credential";
    let _environment = ScopedTestEnvironment::cleared(NAME);
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("config.toml"),
        format!("model_provider = \"custom\"\n[model_providers.custom]\nenv_key = \"{NAME}\"\n"),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("auth.json"),
        format!(r#"{{"OPENAI_API_KEY":"{CREDENTIAL}"}}"#),
    )
    .unwrap();

    clear_codex_user_credential_environment(&settings_for_environment(NAME), NAME).unwrap();
    assert!(current_user_environment_value(NAME).is_none());

    with_temporary_codex_user_credential_environment_from_home(temp.path(), || {
        assert_eq!(std::env::var(NAME).ok().as_deref(), Some(CREDENTIAL));
        assert_eq!(
            current_user_environment_value(NAME).as_deref(),
            Some(CREDENTIAL)
        );
        Ok(())
    })
    .unwrap();

    assert!(
        current_user_environment_value(NAME).is_none(),
        "launch credential injection must not recreate a deleted user environment variable"
    );
    assert!(std::env::var_os(NAME).is_none());
}

#[cfg(windows)]
#[test]
fn windows_temporary_launch_environment_restores_existing_user_and_process_values() {
    const NAME: &str = "CCP_TEST_CREDENTIAL_ENV_RESTORE_EXISTING";
    const USER_VALUE: &str = "existing-user-value";
    const PROCESS_VALUE: &str = "existing-process-value";
    const CREDENTIAL: &str = "test-launch-credential";
    let _environment = ScopedTestEnvironment::cleared(NAME);
    set_current_user_environment_value(NAME, USER_VALUE);
    unsafe {
        std::env::set_var(NAME, PROCESS_VALUE);
    }
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("config.toml"),
        format!("model_provider = \"custom\"\n[model_providers.custom]\nenv_key = \"{NAME}\"\n"),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("auth.json"),
        format!(r#"{{"OPENAI_API_KEY":"{CREDENTIAL}"}}"#),
    )
    .unwrap();

    with_temporary_codex_user_credential_environment_from_home(temp.path(), || {
        assert_eq!(std::env::var(NAME).ok().as_deref(), Some(CREDENTIAL));
        assert_eq!(
            current_user_environment_value(NAME).as_deref(),
            Some(CREDENTIAL)
        );
        Ok(())
    })
    .unwrap();

    assert_eq!(std::env::var(NAME).ok().as_deref(), Some(PROCESS_VALUE));
    assert_eq!(
        current_user_environment_value(NAME).as_deref(),
        Some(USER_VALUE)
    );
}

#[cfg(windows)]
#[test]
fn windows_temporary_launch_environment_hides_inactive_provider_keys_and_restores_them() {
    const ACTIVE_NAME: &str = "CCP_TEST_CREDENTIAL_ENV_ACTIVE";
    const INACTIVE_NAME: &str = "CCP_TEST_CREDENTIAL_ENV_INACTIVE";
    const CREDENTIAL: &str = "test-launch-credential";
    const INACTIVE_USER_VALUE: &str = "inactive-user-value";
    const INACTIVE_PROCESS_VALUE: &str = "inactive-process-value";
    let _active_environment = ScopedTestEnvironment::cleared(ACTIVE_NAME);
    let _inactive_environment = ScopedTestEnvironment::cleared(INACTIVE_NAME);
    set_current_user_environment_value(INACTIVE_NAME, INACTIVE_USER_VALUE);
    unsafe {
        std::env::set_var(INACTIVE_NAME, INACTIVE_PROCESS_VALUE);
    }
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("config.toml"),
        format!(
            r#"model_provider = "active"

[model_providers.active]
env_key = "{ACTIVE_NAME}"

[model_providers.inactive]
env_key = "{INACTIVE_NAME}"
"#
        ),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("auth.json"),
        format!(r#"{{"OPENAI_API_KEY":"{CREDENTIAL}"}}"#),
    )
    .unwrap();

    with_temporary_codex_user_credential_environment_from_home(temp.path(), || {
        assert_eq!(std::env::var(ACTIVE_NAME).ok().as_deref(), Some(CREDENTIAL));
        assert_eq!(
            current_user_environment_value(ACTIVE_NAME).as_deref(),
            Some(CREDENTIAL)
        );
        assert!(std::env::var_os(INACTIVE_NAME).is_none());
        assert!(current_user_environment_value(INACTIVE_NAME).is_none());
        Ok(())
    })
    .unwrap();

    assert!(std::env::var_os(ACTIVE_NAME).is_none());
    assert!(current_user_environment_value(ACTIVE_NAME).is_none());
    assert_eq!(
        std::env::var(INACTIVE_NAME).ok().as_deref(),
        Some(INACTIVE_PROCESS_VALUE)
    );
    assert_eq!(
        current_user_environment_value(INACTIVE_NAME).as_deref(),
        Some(INACTIVE_USER_VALUE)
    );
}

#[cfg(windows)]
#[test]
fn windows_temporary_launch_environment_restores_expandable_value_and_type() {
    const NAME: &str = "CCP_TEST_CREDENTIAL_ENV_RESTORE_EXPANDABLE";
    const RAW_USER_VALUE: &str = r"%USERPROFILE%\ccp-test-provider-key";
    const CREDENTIAL: &str = "test-launch-credential";
    let _environment = ScopedTestEnvironment::cleared(NAME);
    set_current_user_environment_value_with_type(NAME, "REG_EXPAND_SZ", RAW_USER_VALUE);
    let temp = tempfile::tempdir().unwrap();
    write_temporary_provider_home(temp.path(), NAME, CREDENTIAL);

    with_temporary_codex_user_credential_environment_from_home(temp.path(), || Ok(())).unwrap();

    assert_eq!(
        current_user_environment_entry(NAME),
        Some(("REG_EXPAND_SZ".to_string(), RAW_USER_VALUE.to_string()))
    );
}

#[cfg(windows)]
#[test]
fn windows_temporary_launch_environment_restores_after_activation_error() {
    const NAME: &str = "CCP_TEST_CREDENTIAL_ENV_RESTORE_ERROR";
    const CREDENTIAL: &str = "test-launch-credential";
    let _environment = ScopedTestEnvironment::cleared(NAME);
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("config.toml"),
        format!("model_provider = \"custom\"\n[model_providers.custom]\nenv_key = \"{NAME}\"\n"),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("auth.json"),
        format!(r#"{{"OPENAI_API_KEY":"{CREDENTIAL}"}}"#),
    )
    .unwrap();

    let error = with_temporary_codex_user_credential_environment_from_home(
        temp.path(),
        || -> anyhow::Result<()> { anyhow::bail!("simulated activation failure") },
    )
    .unwrap_err();

    assert!(error.to_string().contains("simulated activation failure"));
    assert!(current_user_environment_value(NAME).is_none());
    assert!(std::env::var_os(NAME).is_none());
}
