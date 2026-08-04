use claude_codex_pro_core::protocol_proxy::{
    DEFAULT_PROTOCOL_PROXY_PORT, local_responses_proxy_base_url,
};
use claude_codex_pro_core::relay_config::codex_provider_auth_environment_from_home;
use claude_codex_pro_core::relay_switch::switch_relay_profile_in_home;
use claude_codex_pro_core::settings::{
    BackendSettings, LaunchMode, RelayMode, RelayProfile, SettingsStore,
};

#[test]
fn switch_rolls_back_active_settings_when_live_write_fails() {
    let temp = tempfile::tempdir().unwrap();
    let store = SettingsStore::new(temp.path().join("settings.json"));
    let original = BackendSettings {
        active_relay_id: "a".to_string(),
        relay_profiles: vec![pure_profile("a", "https://a.example/v1", "sk-a")],
        ..BackendSettings::default()
    };
    store.save(&original).unwrap();
    std::fs::create_dir(temp.path().join("codex")).unwrap();
    std::fs::write(
        temp.path().join("codex").join("auth.json"),
        r#"{"OPENAI_API_KEY":"sk-a"}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("codex").join("config.toml"),
        r#"model_provider = "custom"

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://a.example/v1"
"#,
    )
    .unwrap();
    let next = BackendSettings {
        active_relay_id: "b".to_string(),
        relay_profiles: vec![
            pure_profile("a", "https://a.example/v1", "sk-a"),
            RelayProfile {
                id: "b".to_string(),
                name: "B".to_string(),
                relay_mode: RelayMode::PureApi,
                config_contents: "model_provider = \"custom\"\n".to_string(),
                auth_contents: "{bad json".to_string(),
                ..RelayProfile::default()
            },
        ],
        ..BackendSettings::default()
    };

    let error = switch_relay_profile_in_home(&store, &temp.path().join("codex"), next, "a")
        .expect_err("invalid auth should fail switch");

    assert!(error.to_string().contains("auth.json"));
    assert_eq!(store.load().unwrap().active_relay_id, "a");
}

#[test]
fn switch_backfills_previous_profile_from_live_before_selecting_target() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("codex");
    std::fs::create_dir(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        r#"model = "edited-live-model"
model_provider = "manual_a"

[model_providers.manual_a]
name = "manual_a"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://edited-a.example/v1"
"#,
    )
    .unwrap();
    std::fs::write(
        home.join("auth.json"),
        r#"{"OPENAI_API_KEY":"sk-edited-a"}"#,
    )
    .unwrap();
    let store = SettingsStore::new(temp.path().join("settings.json"));
    let original = BackendSettings {
        active_relay_id: "a".to_string(),
        relay_profiles: vec![
            pure_profile("a", "https://a.example/v1", "sk-a"),
            pure_profile("b", "https://b.example/v1", "sk-b"),
        ],
        ..BackendSettings::default()
    };
    store.save(&original).unwrap();
    let next = BackendSettings {
        active_relay_id: "b".to_string(),
        relay_profiles: original.relay_profiles.clone(),
        ..BackendSettings::default()
    };

    switch_relay_profile_in_home(&store, &home, next, "a").unwrap();

    let stored = store.load().unwrap();
    let previous = stored
        .relay_profiles
        .iter()
        .find(|profile| profile.id == "a")
        .unwrap();
    assert!(previous.config_contents.contains("edited-live-model"));
    assert!(previous.config_contents.contains("manual_a"));
    assert_eq!(stored.active_relay_id, "b");
    assert_eq!(stored.launch_mode, LaunchMode::Patch);
}

#[test]
fn route_disabled_ccp_supplier_stays_active_across_restart_reads() {
    const PROVIDER_ID: &str = "ccp-direct-provider";
    const BASE_URL: &str = "https://provider.example.test/v1";
    const API_KEY: &str = "sk-test-ccp-direct-provider";
    const ENV_KEY: &str = "CCP_DIRECT_PROVIDER_API_KEY";

    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("codex");
    std::fs::create_dir(&home).unwrap();
    let store = SettingsStore::new(temp.path().join("settings.json"));
    let profile = RelayProfile {
        id: PROVIDER_ID.to_string(),
        name: "CCP Direct Provider".to_string(),
        model: "gpt-test".to_string(),
        base_url: BASE_URL.to_string(),
        upstream_base_url: BASE_URL.to_string(),
        api_key: API_KEY.to_string(),
        api_key_explicit: true,
        relay_mode: RelayMode::PureApi,
        route_enabled: true,
        config_contents: format!(
            r#"model = "gpt-test"
model_provider = "{PROVIDER_ID}"

[model_providers.{PROVIDER_ID}]
name = "CCP Direct Provider"
base_url = "{BASE_URL}"
wire_api = "responses"
requires_openai_auth = true
env_key = "{ENV_KEY}"
"#
        ),
        auth_contents: format!(r#"{{"OPENAI_API_KEY":"{API_KEY}"}}"#),
        target_app: "codex".to_string(),
        ..RelayProfile::default()
    };
    let routed_settings = BackendSettings {
        active_relay_id: PROVIDER_ID.to_string(),
        relay_profiles: vec![profile],
        ..BackendSettings::default()
    };

    switch_relay_profile_in_home(&store, &home, routed_settings, "").unwrap();

    let routed_config = std::fs::read_to_string(home.join("config.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let routed_base_url = routed_config
        .get("model_providers")
        .and_then(toml_edit::Item::as_table)
        .and_then(|providers| providers.get(PROVIDER_ID))
        .and_then(toml_edit::Item::as_table)
        .and_then(|provider| provider.get("base_url"))
        .and_then(toml_edit::Item::as_str);
    let local_proxy_base_url = local_responses_proxy_base_url(DEFAULT_PROTOCOL_PROXY_PORT);
    assert_eq!(routed_base_url, Some(local_proxy_base_url.as_str()));

    let mut direct_settings = store.load().unwrap();
    let direct_profile = direct_settings
        .relay_profiles
        .iter_mut()
        .find(|profile| profile.id == PROVIDER_ID)
        .unwrap();
    assert_eq!(direct_profile.upstream_base_url, BASE_URL);
    assert!(
        direct_profile.api_key == API_KEY,
        "settings reload must retain the selected supplier credential"
    );
    direct_profile.route_enabled = false;

    switch_relay_profile_in_home(&store, &home, direct_settings, PROVIDER_ID).unwrap();

    let stored = store.load().unwrap();
    assert_eq!(stored.active_relay_id, PROVIDER_ID);
    let stored_profile = stored
        .relay_profiles
        .iter()
        .find(|profile| profile.id == PROVIDER_ID)
        .unwrap();
    assert!(!stored_profile.route_enabled);

    let config_path = home.join("config.toml");
    let auth_path = home.join("auth.json");
    let config_before = std::fs::read(&config_path).unwrap();
    let auth_before = std::fs::read(&auth_path).unwrap();
    let live_config = std::str::from_utf8(&config_before)
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let live_provider = live_config
        .get("model_providers")
        .and_then(toml_edit::Item::as_table)
        .and_then(|providers| providers.get(PROVIDER_ID))
        .and_then(toml_edit::Item::as_table)
        .unwrap();
    assert_eq!(
        live_config
            .get("model_provider")
            .and_then(toml_edit::Item::as_str),
        Some(PROVIDER_ID)
    );
    assert_eq!(
        live_provider
            .get("base_url")
            .and_then(toml_edit::Item::as_str),
        Some(BASE_URL)
    );
    let live_auth: serde_json::Value = serde_json::from_slice(&auth_before).unwrap();
    assert!(
        live_auth
            .get("OPENAI_API_KEY")
            .and_then(serde_json::Value::as_str)
            == Some(API_KEY),
        "live auth must retain the selected CCP supplier credential"
    );

    let (env_key, api_key) = codex_provider_auth_environment_from_home(&home).unwrap();

    assert_eq!(env_key, ENV_KEY);
    assert!(
        api_key == API_KEY,
        "restart-facing provider lookup must return the selected credential"
    );
    assert_eq!(std::fs::read(&config_path).unwrap(), config_before);
    assert_eq!(std::fs::read(&auth_path).unwrap(), auth_before);
}

fn pure_profile(id: &str, base_url: &str, key: &str) -> RelayProfile {
    RelayProfile {
        id: id.to_string(),
        name: id.to_uppercase(),
        relay_mode: RelayMode::PureApi,
        config_contents: format!(
            r#"model_provider = "custom"

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true
base_url = "{base_url}"
"#
        ),
        auth_contents: format!(r#"{{"OPENAI_API_KEY":"{key}"}}"#),
        ..RelayProfile::default()
    }
}
