use claude_codex_pro_core::claude_desktop_provider::{
    CLAUDE_DESKTOP_PROVIDER_PROFILE_ID, ClaudeDesktopProviderPaths, ClaudeDesktopProviderRequest,
    apply_claude_desktop_provider_at_paths, apply_claude_desktop_provider_at_paths_with_proxy_port,
    preview_claude_desktop_provider_at_paths,
    preview_claude_desktop_provider_at_paths_with_proxy_port,
    restore_claude_desktop_provider_official_at_paths,
};
use serde_json::{Value, json};

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
    assert!(preview.write_targets.iter().any(|target| {
        target.ends_with("Claude-3p\\configLibrary\\00000000-0000-4000-8000-000000157210.json")
            || target.ends_with("Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json")
    }));
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
        serde_json::from_str(&std::fs::read_to_string(&paths.profile_path).unwrap()).unwrap();
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
    assert_eq!(
        profile["inferenceModels"][2]["name"],
        json!("claude-opus-4-8")
    );
    assert_eq!(
        profile["inferenceModels"][2]["labelOverride"],
        json!("qwen3-coder")
    );
    assert_eq!(profile["inferenceModels"][2]["supports1m"], json!(true));
    assert_eq!(
        profile["inferenceModels"][3]["name"],
        json!("claude-sonnet-4-6")
    );
    assert_eq!(
        profile["inferenceModels"][3]["labelOverride"],
        json!("gpt-5.3")
    );
    assert_eq!(meta["appliedId"], json!(CLAUDE_DESKTOP_PROVIDER_PROFILE_ID));
    assert!(
        meta["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"] == CLAUDE_DESKTOP_PROVIDER_PROFILE_ID
                && entry["name"] == "TopoReduce")
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
fn claude_desktop_provider_apply_uses_actual_proxy_port() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ClaudeDesktopProviderPaths::from_single_root(temp.path());
    let request = ClaudeDesktopProviderRequest {
        name: "TopoReduce".to_string(),
        base_url: "https://api.toporeduce.cn".to_string(),
        api_key: "sk-test-secret".to_string(),
        model_list: String::new(),
    };

    apply_claude_desktop_provider_at_paths_with_proxy_port(&paths, &request, 58432).unwrap();

    let profile: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.profile_path).unwrap()).unwrap();
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

    let error = apply_claude_desktop_provider_at_paths(&paths, &request).unwrap_err();

    let normal: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.normal_config_path).unwrap()).unwrap();
    let threep: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.threep_config_path).unwrap()).unwrap();
    assert!(!error.to_string().trim().is_empty());
    assert_eq!(normal, json!({"deploymentMode":"1p","normal":true}));
    assert_eq!(threep, json!({"deploymentMode":"1p","threep":true}));
    assert!(!paths.profile_path.is_file());
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
    apply_claude_desktop_provider_at_paths(&paths, &request).unwrap();

    let outcome = restore_claude_desktop_provider_official_at_paths(&paths).unwrap();

    let normal: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.normal_config_path).unwrap()).unwrap();
    let threep: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.threep_config_path).unwrap()).unwrap();
    assert_eq!(normal["deploymentMode"], json!("1p"));
    assert_eq!(threep["deploymentMode"], json!("1p"));
    assert!(!paths.profile_path.exists());
    assert!(
        outcome
            .backup_paths
            .iter()
            .all(|path| std::path::Path::new(path).is_file())
    );
}
