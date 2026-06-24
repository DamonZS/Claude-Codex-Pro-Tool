use claude_codex_pro_core::claude_desktop_provider::{
    apply_claude_desktop_provider_at_paths, preview_claude_desktop_provider_at_paths,
    restore_claude_desktop_provider_official_at_paths, ClaudeDesktopProviderPaths,
    ClaudeDesktopProviderRequest, CLAUDE_DESKTOP_PROVIDER_PROFILE_ID,
};
use serde_json::{json, Value};

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
    assert!(preview.config_diff.contains("https://api.toporeduce.cn"));
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
        model_list: String::new(),
    };

    let outcome = apply_claude_desktop_provider_at_paths(&paths, &request).unwrap();

    let normal: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.normal_config_path).unwrap())
            .unwrap();
    let threep: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.threep_config_path).unwrap())
            .unwrap();
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
        json!("https://api.toporeduce.cn")
    );
    assert_eq!(profile["inferenceGatewayApiKey"], json!("sk-test-secret"));
    assert_eq!(profile["inferenceGatewayAuthScheme"], json!("bearer"));
    assert_eq!(profile["disableDeploymentModeChooser"], json!(true));
    assert_eq!(profile["coworkEgressAllowedHosts"], json!(["*"]));
    assert_eq!(meta["appliedId"], json!(CLAUDE_DESKTOP_PROVIDER_PROFILE_ID));
    assert!(outcome.backup_paths.iter().all(|path| std::path::Path::new(path).is_file()));
    assert!(outcome.backup_paths.len() >= 2);
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
    std::fs::write(&paths.threep_config_path, r#"{"deploymentMode":"1p","threep":true}"#)
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
        serde_json::from_str(&std::fs::read_to_string(&paths.normal_config_path).unwrap())
            .unwrap();
    let threep: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.threep_config_path).unwrap())
            .unwrap();
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
        serde_json::from_str(&std::fs::read_to_string(&paths.normal_config_path).unwrap())
            .unwrap();
    let threep: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.threep_config_path).unwrap())
            .unwrap();
    assert_eq!(normal["deploymentMode"], json!("1p"));
    assert_eq!(threep["deploymentMode"], json!("1p"));
    assert!(!paths.profile_path.exists());
    assert!(outcome.backup_paths.iter().all(|path| std::path::Path::new(path).is_file()));
}
