use claude_codex_pro_core::credential_environment::{
    analyze_credential_environment, clear_codex_user_credential_environment,
    credential_environment_requires_update, diagnose_codex_credential_environment,
    sync_codex_user_credential_environment, sync_codex_user_credential_environment_from_home,
    valid_environment_variable_name,
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
            line.find(value_type)
                .map(|index| line[index + value_type.len()..].trim().to_string())
                .unwrap_or_default()
        })
    })
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
            Some(value) => {
                let _ = sync_codex_user_credential_environment(&self.name, value);
            }
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
fn cleanup_variable_name_validation_is_strict() {
    assert!(valid_environment_variable_name("OPENAI_API_KEY"));
    assert!(valid_environment_variable_name("CCP_TEST_123"));
    assert!(!valid_environment_variable_name(""));
    assert!(!valid_environment_variable_name("OPENAI-API-KEY"));
    assert!(!valid_environment_variable_name("OPENAI_API_KEY=bad"));
    assert!(!valid_environment_variable_name("CODEX_HOME\\test"));
}

#[test]
fn credential_environment_sync_decision_only_updates_missing_or_different_values() {
    assert!(credential_environment_requires_update(None, "current"));
    assert!(!credential_environment_requires_update(
        Some("current"),
        "current"
    ));
    assert!(credential_environment_requires_update(
        Some("stale"),
        "current"
    ));
    assert!(!credential_environment_requires_update(None, "  "));
}

#[test]
fn sync_from_home_uses_custom_env_key_updates_once_and_redacts_result() {
    const NAME: &str = "CCP_TEST_CREDENTIAL_ENV_SYNC";
    const FIRST: &str = "credential-first-value";
    const SECOND: &str = "credential-second-value";
    let _environment = ScopedTestEnvironment::cleared(NAME);
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("config.toml"),
        format!("model_provider = \"custom\"\n[model_providers.custom]\nenv_key = \"{NAME}\"\n"),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("auth.json"),
        format!(r#"{{"OPENAI_API_KEY":"{FIRST}"}}"#),
    )
    .unwrap();

    let first = sync_codex_user_credential_environment_from_home(temp.path())
        .unwrap()
        .unwrap();
    assert_eq!(first.variable_name, NAME);
    assert!(first.process_changed);
    #[cfg(windows)]
    assert!(first.user_changed);
    #[cfg(not(windows))]
    assert!(!first.user_changed);
    assert_eq!(std::env::var(NAME).unwrap(), FIRST);

    let unchanged = sync_codex_user_credential_environment_from_home(temp.path())
        .unwrap()
        .unwrap();
    assert!(!unchanged.process_changed);
    assert!(!unchanged.user_changed);

    std::fs::write(
        temp.path().join("auth.json"),
        format!(r#"{{"OPENAI_API_KEY":"{SECOND}"}}"#),
    )
    .unwrap();
    let updated = sync_codex_user_credential_environment_from_home(temp.path())
        .unwrap()
        .unwrap();
    assert!(updated.process_changed);
    #[cfg(windows)]
    assert!(updated.user_changed);
    #[cfg(not(windows))]
    assert!(!updated.user_changed);
    assert_eq!(std::env::var(NAME).unwrap(), SECOND);

    let serialized = serde_json::to_string(&updated).unwrap();
    assert!(!serialized.contains(FIRST));
    assert!(!serialized.contains(SECOND));
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
