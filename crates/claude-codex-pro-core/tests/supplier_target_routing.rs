use claude_codex_pro_core::claude_desktop_provider::ClaudeDesktopProviderRequest;
use claude_codex_pro_core::plugin_hub::persist_claude_desktop_provider_request_to_settings;
use claude_codex_pro_core::protocol_proxy::claude_desktop_models_response_for_settings;
use claude_codex_pro_core::settings::{
    BackendSettings, RelayProfile, relay_profile_resolved_api_key,
};

fn profile(id: &str, target_app: &str) -> RelayProfile {
    RelayProfile {
        id: id.to_string(),
        name: id.to_string(),
        target_app: target_app.to_string(),
        ..RelayProfile::default()
    }
}

#[test]
fn target_specific_active_profiles_do_not_fall_back_to_codex_selection() {
    let settings = BackendSettings {
        relay_profiles: vec![
            profile("codex-a", "codex"),
            profile("claude-a", "claude"),
            profile("desktop-a", "claude-desktop"),
        ],
        active_relay_id: "codex-a".to_string(),
        active_claude_relay_id: "claude-a".to_string(),
        active_claude_desktop_relay_id: "desktop-a".to_string(),
        ..BackendSettings::default()
    };

    assert_eq!(
        settings.active_relay_profile_for_target("codex").id,
        "codex-a"
    );
    assert_eq!(
        settings.active_relay_profile_for_target("claude").id,
        "claude-a"
    );
    assert_eq!(
        settings
            .active_relay_profile_for_target("claude-desktop")
            .id,
        "desktop-a"
    );
}

#[test]
fn legacy_settings_without_target_ids_load_and_use_target_local_fallbacks() {
    let settings: BackendSettings = serde_json::from_value(serde_json::json!({
        "activeRelayId": "codex-a",
        "relayProfiles": [
            { "id": "codex-a", "name": "Codex", "targetApp": "codex" },
            { "id": "claude-a", "name": "Claude", "targetApp": "claude" },
            { "id": "desktop-a", "name": "Desktop", "targetApp": "claude-desktop" }
        ]
    }))
    .unwrap();

    assert!(settings.active_claude_relay_id.is_empty());
    assert!(settings.active_claude_desktop_relay_id.is_empty());
    assert_eq!(
        settings.active_relay_profile_for_target("claude").id,
        "claude-a"
    );
    assert_eq!(
        settings
            .active_relay_profile_for_target("claude-desktop")
            .id,
        "desktop-a"
    );
    assert_eq!(settings.active_relay_id, "codex-a");
}

#[test]
fn resolved_api_key_reads_anthropic_keys_from_auth_and_nested_config_env() {
    let auth_profile = RelayProfile {
        auth_contents: r#"{"ANTHROPIC_API_KEY":"test-anthropic-key"}"#.to_string(),
        ..RelayProfile::default()
    };
    let config_profile = RelayProfile {
        config_contents: r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"test-nested-token"}}"#.to_string(),
        ..RelayProfile::default()
    };

    assert_eq!(
        relay_profile_resolved_api_key(&auth_profile),
        "test-anthropic-key"
    );
    assert_eq!(
        relay_profile_resolved_api_key(&config_profile),
        "test-nested-token"
    );
}

#[test]
fn claude_desktop_model_catalog_uses_desktop_target_instead_of_codex_target() {
    let mut codex = profile("codex-a", "codex");
    codex.model_list = "codex-only-model".to_string();
    let mut desktop = profile("desktop-a", "claude-desktop");
    desktop.model_list = "desktop-upstream-model".to_string();
    let settings = BackendSettings {
        relay_profiles: vec![codex, desktop],
        active_relay_id: "codex-a".to_string(),
        active_claude_desktop_relay_id: "desktop-a".to_string(),
        ..BackendSettings::default()
    };

    let response = claude_desktop_models_response_for_settings(&settings);
    let serialized = serde_json::to_string(&response).unwrap();

    assert!(serialized.contains("desktop-upstream-model"));
    assert!(!serialized.contains("codex-only-model"));
}

#[test]
fn persisting_desktop_provider_does_not_replace_codex_active_provider() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let previous =
        claude_codex_pro_core::paths::set_settings_path_for_tests(Some(settings_path.clone()));
    let settings = BackendSettings {
        relay_profiles: vec![
            profile("codex-a", "codex"),
            profile("desktop-a", "claude-desktop"),
        ],
        active_relay_id: "codex-a".to_string(),
        ..BackendSettings::default()
    };
    claude_codex_pro_core::settings::SettingsStore::default()
        .save(&settings)
        .unwrap();

    persist_claude_desktop_provider_request_to_settings(&ClaudeDesktopProviderRequest {
        name: "Desktop A".to_string(),
        base_url: "https://desktop.example/v1".to_string(),
        api_key: "test-desktop-key".to_string(),
        model_list: "claude-sonnet-test".to_string(),
    })
    .unwrap();
    let loaded = claude_codex_pro_core::settings::SettingsStore::default()
        .load()
        .unwrap();
    claude_codex_pro_core::paths::set_settings_path_for_tests(previous);

    assert_eq!(loaded.active_relay_id, "codex-a");
    assert!(!loaded.active_claude_desktop_relay_id.is_empty());
    let desktop = loaded.active_relay_profile_for_target("claude-desktop");
    assert_eq!(
        claude_codex_pro_core::protocol_proxy::claude_desktop_resolved_upstream_base_url(
            &desktop, &loaded,
        ),
        "https://desktop.example/v1"
    );
}
