use claude_codex_pro_core::claude_provider::apply_claude_provider_at_path;
use claude_codex_pro_core::settings::{RelayProfile, RelayProtocol};
use serde_json::{Value, json};

#[test]
fn claude_provider_write_merges_env_without_overwriting_existing_settings() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join(".claude").join("settings.json");
    std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&json!({
            "permissions": { "allow": ["Read"] },
            "hooks": { "PreToolUse": [{ "matcher": "Bash" }] },
            "customField": { "keep": true },
            "env": { "EXISTING_VALUE": "keep" }
        }))
        .unwrap(),
    )
    .unwrap();
    let profile = RelayProfile {
        id: "claude-a".to_string(),
        name: "Claude A".to_string(),
        target_app: "claude".to_string(),
        protocol: RelayProtocol::Responses,
        config_contents: serde_json::to_string_pretty(&json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://claude.example/v1",
                "ANTHROPIC_AUTH_TOKEN": "test-claude-token",
                "ANTHROPIC_MODEL": "claude-sonnet-test",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-test"
            }
        }))
        .unwrap(),
        auth_contents: r#"{"ANTHROPIC_AUTH_TOKEN":"test-claude-token"}"#.to_string(),
        ..RelayProfile::default()
    };

    let outcome = apply_claude_provider_at_path(&settings_path, &profile).unwrap();
    let written: Value = serde_json::from_slice(&std::fs::read(&settings_path).unwrap()).unwrap();

    assert_eq!(written["permissions"]["allow"][0], json!("Read"));
    assert_eq!(written["hooks"]["PreToolUse"][0]["matcher"], json!("Bash"));
    assert_eq!(written["customField"]["keep"], json!(true));
    assert_eq!(written["env"]["EXISTING_VALUE"], json!("keep"));
    assert_eq!(
        written["env"]["ANTHROPIC_BASE_URL"],
        json!("https://claude.example/v1")
    );
    assert_eq!(
        written["env"]["ANTHROPIC_AUTH_TOKEN"],
        json!("test-claude-token")
    );
    assert_eq!(
        written["env"]["ANTHROPIC_MODEL"],
        json!("claude-sonnet-test")
    );
    assert!(outcome.backup_path.is_some());
    assert!(std::path::Path::new(outcome.backup_path.as_ref().unwrap()).is_file());
}

#[test]
fn claude_provider_rejects_missing_key_without_changing_existing_file() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let original = br#"{"customField":"keep"}"#;
    std::fs::write(&settings_path, original).unwrap();
    let profile = RelayProfile {
        id: "claude-empty".to_string(),
        name: "Claude Empty".to_string(),
        target_app: "claude".to_string(),
        upstream_base_url: "https://claude.example/v1".to_string(),
        ..RelayProfile::default()
    };

    let error = apply_claude_provider_at_path(&settings_path, &profile).unwrap_err();

    assert!(error.to_string().contains("API Key"));
    assert_eq!(std::fs::read(&settings_path).unwrap(), original);
}
