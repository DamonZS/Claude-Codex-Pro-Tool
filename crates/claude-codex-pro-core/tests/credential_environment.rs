use claude_codex_pro_core::credential_environment::{
    analyze_credential_environment, clear_codex_user_credential_environment,
    diagnose_codex_credential_environment, valid_environment_variable_name,
};
use claude_codex_pro_core::settings::{BackendSettings, RelayMode, RelayProfile};

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

#[cfg(windows)]
#[test]
fn windows_cleanup_removes_only_the_named_user_environment_value() {
    use std::process::Command;

    const NAME: &str = "CCP_TEST_CREDENTIAL_ENV_CLEANUP";
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
