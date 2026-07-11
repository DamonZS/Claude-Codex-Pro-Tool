use claude_codex_pro_core::claude_desktop_provider::{
    ClaudeDesktopProviderPaths, ClaudeDesktopProviderRequest,
    apply_claude_desktop_provider_at_paths, apply_claude_desktop_provider_at_paths_with_proxy_port,
    preview_claude_desktop_provider_at_paths,
    preview_claude_desktop_provider_at_paths_with_proxy_port,
    restore_claude_desktop_provider_official_at_paths,
};
use serde_json::{Value, json};

const CC_SWITCH_PROFILE_ID: &str = "00000000-0000-4000-8000-000000157210";

#[test]
fn claude_desktop_provider_preview_redacts_secret_and_shows_real_targets() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: "claude-sonnet-4-6\nclaude-opus-4-8 [1m]".to_string(),
    };

    let preview = preview_claude_desktop_provider_at_paths(&paths, &request).unwrap();

    assert!(preview.config_diff.contains("deploymentMode = 3p"));
    assert!(
        preview
            .config_diff
            .contains("http://127.0.0.1:57331/claude-desktop")
    );
    assert!(preview.config_diff.contains("***redacted***"));
    assert!(!preview.config_diff.contains("sk-test-secret"));
    assert!(preview.redacted_profile.contains("***redacted***"));
    assert!(!preview.redacted_profile.contains("sk-test-secret"));
    assert!(preview.write_targets.contains(&preview.profile_path));
}

#[test]
fn claude_desktop_provider_preview_uses_ccp_additive_profile_id() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };

    let preview = preview_claude_desktop_provider_at_paths(&paths, &request).unwrap();

    assert_ne!(preview.profile_id, CC_SWITCH_PROFILE_ID);
    assert!(preview.profile_id.starts_with("cc120012-"));
    assert!(
        preview
            .profile_path
            .ends_with(&format!("{}.json", preview.profile_id))
    );
    assert!(!preview.profile_path.contains("sk-test-secret"));
}

#[test]
fn claude_desktop_provider_preview_uses_actual_proxy_port() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };

    let preview =
        preview_claude_desktop_provider_at_paths_with_proxy_port(&paths, &request, 58431).unwrap();

    assert!(
        preview
            .config_diff
            .contains("http://127.0.0.1:58431/claude-desktop")
    );
}

#[test]
fn claude_desktop_provider_rejects_fake_localhost_urls() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "http://localhost.evil.com/v1".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };

    let error = preview_claude_desktop_provider_at_paths(&paths, &request).unwrap_err();

    assert!(error.to_string().contains("Base URL"));
}

#[test]
fn claude_desktop_provider_apply_writes_gateway_profile_meta_and_backups() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    std::fs::create_dir_all(paths.normal_config_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(paths.threep_config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &paths.normal_config_path,
        r#"{"deploymentMode":"1p","windowBounds":{"width":1200}}"#,
    )
    .unwrap();
    std::fs::write(&paths.threep_config_path, r#"{"deploymentMode":"1p"}"#).unwrap();
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: "gpt-5.3\ngpt-5.5 [1M]\nqwen3-coder".to_string(),
    };

    let outcome = apply_claude_desktop_provider_at_paths(&paths, &request).unwrap();

    let normal: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.normal_config_path).unwrap()).unwrap();
    let threep: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.threep_config_path).unwrap()).unwrap();
    let profile: Value =
        serde_json::from_str(&std::fs::read_to_string(&outcome.profile_path).unwrap()).unwrap();
    let meta: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.meta_path).unwrap()).unwrap();

    assert_eq!(normal["deploymentMode"], json!("3p"));
    assert_eq!(normal["windowBounds"]["width"], json!(1200));
    assert_eq!(threep["deploymentMode"], json!("3p"));
    assert_eq!(profile["inferenceProvider"], json!("gateway"));
    assert_eq!(
        profile["inferenceGatewayBaseUrl"],
        json!("http://127.0.0.1:57331/claude-desktop")
    );
    assert_eq!(profile["inferenceGatewayApiKey"], json!("sk-test-secret"));
    assert_eq!(profile["inferenceGatewayAuthScheme"], json!("bearer"));
    assert_eq!(profile["disableDeploymentModeChooser"], json!(true));
    assert_eq!(profile["coworkEgressAllowedHosts"], json!(["*"]));
    assert_eq!(
        profile["inferenceModels"][0]["name"],
        json!("claude-fable-5")
    );
    assert_eq!(
        profile["inferenceModels"][0]["labelOverride"],
        json!("gpt-5.3")
    );
    assert_eq!(
        profile["inferenceModels"][1]["name"],
        json!("claude-haiku-4-5")
    );
    assert_eq!(
        profile["inferenceModels"][1]["labelOverride"],
        json!("gpt-5.5")
    );
    assert_eq!(profile["inferenceModels"][1]["supports1m"], json!(true));
    assert_eq!(
        profile["inferenceModels"][2]["name"],
        json!("claude-opus-4-8")
    );
    assert_eq!(
        profile["inferenceModels"][2]["labelOverride"],
        json!("qwen3-coder")
    );
    assert!(profile["inferenceModels"][2].get("supports1m").is_none());
    assert_eq!(
        profile["inferenceModels"][3]["name"],
        json!("claude-sonnet-4-6")
    );
    assert_eq!(
        profile["inferenceModels"][3]["labelOverride"],
        json!("gpt-5.3")
    );
    let applied_id = std::path::Path::new(&outcome.profile_path)
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert_eq!(meta["appliedId"], json!(applied_id));
    assert!(
        meta["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"] == meta["appliedId"] && entry["name"] == "TopoReduce")
    );
    assert!(
        outcome
            .backup_paths
            .iter()
            .all(|path| std::path::Path::new(path).is_file())
    );
    assert!(outcome.backup_paths.len() >= 2);
}

#[test]
fn claude_desktop_provider_apply_preserves_ccswitch_profile_and_appends_ccp_profile() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    std::fs::create_dir_all(&paths.config_library_dir).unwrap();
    let ccswitch_profile_path = paths
        .config_library_dir
        .join(format!("{CC_SWITCH_PROFILE_ID}.json"));
    let ccswitch_profile = r#"{"owner":"cc-switch","customSetting":"keep"}"#;
    std::fs::write(&ccswitch_profile_path, ccswitch_profile).unwrap();
    std::fs::write(
        &paths.meta_path,
        serde_json::to_string_pretty(&json!({
            "appliedId": CC_SWITCH_PROFILE_ID,
            "customMeta": "keep",
            "entries": [{
                "id": CC_SWITCH_PROFILE_ID,
                "name": "CC Switch"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };

    let outcome = apply_claude_desktop_provider_at_paths(&paths, &request).unwrap();
    let meta: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.meta_path).unwrap()).unwrap();

    assert_eq!(
        std::fs::read_to_string(ccswitch_profile_path).unwrap(),
        ccswitch_profile
    );
    assert_ne!(meta["appliedId"], json!(CC_SWITCH_PROFILE_ID));
    assert_eq!(meta["customMeta"], json!("keep"));
    assert!(
        meta["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["id"] == CC_SWITCH_PROFILE_ID && entry["name"] == "CC Switch" })
    );
    assert!(
        meta["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["id"] == meta["appliedId"] && entry["name"] == "TopoReduce" })
    );
    assert!(std::path::Path::new(&outcome.profile_path).is_file());
}

#[test]
fn claude_desktop_provider_profile_is_idempotent_per_provider_and_additive_across_providers() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    let first = ClaudeDesktopProviderRequest {
        name: "Provider A".to_string(),
        base_url: "https://a.example.com/v1".to_string(),
        api_key: "key-a-1".to_string(),
        model_list: String::new(),
    };
    let mut first_with_new_key = first.clone();
    first_with_new_key.name = " provider a ".to_string();
    first_with_new_key.base_url = "https://a.example.com/v1/".to_string();
    first_with_new_key.api_key = "key-a-2".to_string();
    let second = ClaudeDesktopProviderRequest {
        name: "Provider B".to_string(),
        base_url: "https://b.example.com/v1".to_string(),
        api_key: "key-b".to_string(),
        model_list: String::new(),
    };

    let first_outcome = apply_claude_desktop_provider_at_paths(&paths, &first).unwrap();
    let repeated_outcome =
        apply_claude_desktop_provider_at_paths(&paths, &first_with_new_key).unwrap();
    let second_outcome = apply_claude_desktop_provider_at_paths(&paths, &second).unwrap();
    let meta: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.meta_path).unwrap()).unwrap();

    assert_eq!(first_outcome.profile_path, repeated_outcome.profile_path);
    assert_ne!(first_outcome.profile_path, second_outcome.profile_path);
    assert!(!first_outcome.profile_path.contains("key-a-1"));
    assert!(!repeated_outcome.profile_path.contains("key-a-2"));
    assert_eq!(meta["entries"].as_array().unwrap().len(), 2);
    assert!(std::path::Path::new(&first_outcome.profile_path).is_file());
    assert!(std::path::Path::new(&second_outcome.profile_path).is_file());
}

#[test]
fn claude_desktop_provider_updates_managed_fields_without_dropping_profile_extensions() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };
    let preview = preview_claude_desktop_provider_at_paths(&paths, &request).unwrap();
    let profile_path = std::path::PathBuf::from(&preview.profile_path);
    std::fs::create_dir_all(profile_path.parent().unwrap()).unwrap();
    std::fs::write(
        &profile_path,
        r#"{"customExtension":{"enabled":true},"inferenceGatewayApiKey":"old"}"#,
    )
    .unwrap();
    std::fs::write(
        &paths.meta_path,
        serde_json::to_string_pretty(&json!({
            "appliedId": preview.profile_id,
            "customMeta": "keep",
            "entries": [{
                "id": preview.profile_id,
                "name": "Old Name",
                "customEntry": "keep"
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    apply_claude_desktop_provider_at_paths(&paths, &request).unwrap();
    let profile: Value =
        serde_json::from_str(&std::fs::read_to_string(profile_path).unwrap()).unwrap();
    let meta: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.meta_path).unwrap()).unwrap();

    assert_eq!(profile["customExtension"]["enabled"], json!(true));
    assert_eq!(profile["inferenceGatewayApiKey"], json!("sk-test-secret"));
    assert_eq!(meta["customMeta"], json!("keep"));
    assert!(meta["entries"].as_array().unwrap().iter().any(|entry| {
        entry["id"] == meta["appliedId"]
            && entry["name"] == "TopoReduce"
            && entry["customEntry"] == "keep"
    }));
}

#[test]
fn claude_desktop_provider_apply_uses_actual_proxy_port() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };

    let outcome =
        apply_claude_desktop_provider_at_paths_with_proxy_port(&paths, &request, 58432).unwrap();

    let profile: Value =
        serde_json::from_str(&std::fs::read_to_string(&outcome.profile_path).unwrap()).unwrap();
    assert_eq!(
        profile["inferenceGatewayBaseUrl"],
        json!("http://127.0.0.1:58432/claude-desktop")
    );
    assert_eq!(
        profile["inferenceModels"][0]["name"],
        json!("claude-fable-5")
    );
    assert_eq!(
        profile["inferenceModels"][1]["name"],
        json!("claude-haiku-4-5")
    );
    assert_eq!(
        profile["inferenceModels"][2]["name"],
        json!("claude-opus-4-8")
    );
    assert_eq!(
        profile["inferenceModels"][3]["name"],
        json!("claude-sonnet-4-6")
    );
}

#[test]
fn claude_desktop_provider_apply_recovers_invalid_json_configs() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    std::fs::create_dir_all(paths.normal_config_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(paths.threep_config_path.parent().unwrap()).unwrap();
    std::fs::write(&paths.normal_config_path, r#"{"deploymentMode":"#).unwrap();
    std::fs::write(&paths.threep_config_path, r#"{"deploymentMode":"#).unwrap();
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };

    apply_claude_desktop_provider_at_paths(&paths, &request).unwrap();

    let normal: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.normal_config_path).unwrap()).unwrap();
    let threep: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.threep_config_path).unwrap()).unwrap();
    assert_eq!(normal["deploymentMode"], json!("3p"));
    assert_eq!(threep["deploymentMode"], json!("3p"));
    assert!(
        std::fs::read_dir(paths.normal_config_path.parent().unwrap())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".invalid."))
    );
}

#[test]
fn claude_desktop_provider_apply_accepts_bom_prefixed_json_configs() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    std::fs::create_dir_all(paths.normal_config_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(paths.threep_config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &paths.normal_config_path,
        "\u{feff}{\"deploymentMode\":\"1p\",\"windowBounds\":{\"width\":1200}}",
    )
    .unwrap();
    std::fs::write(
        &paths.threep_config_path,
        "\u{feff}{\"deploymentMode\":\"1p\"}",
    )
    .unwrap();
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };

    apply_claude_desktop_provider_at_paths(&paths, &request).unwrap();

    let normal_raw = std::fs::read_to_string(&paths.normal_config_path).unwrap();
    let normal: Value = serde_json::from_str(&normal_raw).unwrap();
    assert!(!normal_raw.starts_with('\u{feff}'));
    assert_eq!(normal["deploymentMode"], json!("3p"));
    assert_eq!(normal["windowBounds"]["width"], json!(1200));
}

#[test]
fn claude_desktop_provider_apply_rolls_back_when_late_write_fails() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    std::fs::create_dir_all(paths.normal_config_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(paths.threep_config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &paths.normal_config_path,
        r#"{"deploymentMode":"1p","normal":true}"#,
    )
    .unwrap();
    std::fs::write(
        &paths.threep_config_path,
        r#"{"deploymentMode":"1p","threep":true}"#,
    )
    .unwrap();
    std::fs::write(&paths.config_library_dir, "not a directory").unwrap();
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };
    let expected_profile_path = std::path::PathBuf::from(
        preview_claude_desktop_provider_at_paths(&paths, &request)
            .unwrap()
            .profile_path,
    );

    let error = apply_claude_desktop_provider_at_paths(&paths, &request).unwrap_err();

    let normal: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.normal_config_path).unwrap()).unwrap();
    let threep: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.threep_config_path).unwrap()).unwrap();
    assert!(!error.to_string().trim().is_empty());
    assert_eq!(normal, json!({"deploymentMode":"1p","normal":true}));
    assert_eq!(threep, json!({"deploymentMode":"1p","threep":true}));
    assert!(!expected_profile_path.is_file());
}

#[test]
fn claude_desktop_provider_restore_switches_back_to_official_mode() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };
    let applied = apply_claude_desktop_provider_at_paths(&paths, &request).unwrap();
    let applied_profile_path = std::path::PathBuf::from(&applied.profile_path);

    let outcome = restore_claude_desktop_provider_official_at_paths(&paths).unwrap();

    let normal: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.normal_config_path).unwrap()).unwrap();
    let threep: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.threep_config_path).unwrap()).unwrap();
    assert_eq!(normal["deploymentMode"], json!("1p"));
    assert_eq!(threep["deploymentMode"], json!("1p"));
    assert!(applied_profile_path.exists());
    let meta: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.meta_path).unwrap()).unwrap();
    assert!(
        meta["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["id"] == meta["appliedId"] && entry["name"] == "TopoReduce" })
    );
    assert!(
        outcome
            .backup_paths
            .iter()
            .all(|path| std::path::Path::new(path).is_file())
    );
}
