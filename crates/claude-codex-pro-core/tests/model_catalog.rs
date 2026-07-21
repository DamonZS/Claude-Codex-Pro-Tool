use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Mutex;
use std::thread;

use claude_codex_pro_core::model_catalog::{
    build_models_url_candidates, discover_relay_profile_model_ids, fetch_relay_profile_model_ids,
    read_codex_model_catalog, read_codex_model_catalog_from_home,
};
use claude_codex_pro_core::settings::{
    BackendSettings, RelayMode, RelayProfile, RelayProtocol, SettingsStore,
};
use serde_json::json;

static MODEL_CATALOG_ENV_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn model_catalog_fetches_models_from_codex_config_provider() {
    let temp = tempfile::tempdir().unwrap();
    let server = spawn_models_server(json!({
        "data": [
            {"id": "qwen3-coder"},
            {"id": "deepseek-coder"}
        ]
    }));
    write_config(
        temp.path(),
        &format!(
            r#"
model = "qwen3-coder"
model_provider = "relay"

[model_providers.relay]
name = "Relay"
base_url = "{}"
experimental_bearer_token = "relay-key"
"#,
            server.base_url
        ),
    );

    let result = read_codex_model_catalog_from_home(
        temp.path(),
        &HashMap::new(),
        reqwest::Client::builder().no_proxy().build().unwrap(),
    )
    .await;

    assert_eq!(result["status"], "ok");
    assert_eq!(result["model_provider"], "relay");
    assert_eq!(result["provider_name"], "Relay");
    assert_eq!(result["default_model"], "qwen3-coder");
    assert_eq!(result["models"], json!(["qwen3-coder", "deepseek-coder"]));
    assert_eq!(
        result["sources"][0]["endpoint"],
        format!("{}/v1/models", server.base_url)
    );
    assert_eq!(
        result["responses_api"],
        json!({
            "status": "unknown",
            "endpoint": "",
            "message": ""
        })
    );
    assert_eq!(result["sources"][0]["responses_api"]["status"], "unknown");
    assert_eq!(result["sources"][0]["attempts"][0]["action"], "accepted");
    let requests = server.finish();
    assert_eq!(requests[0].path, "/v1/models");
    assert_eq!(requests[0].authorization, "Bearer relay-key");
    assert!(requests[0].x_api_key.is_empty());
    assert!(requests[0].anthropic_version.is_empty());
}

#[tokio::test]
async fn model_catalog_marks_empty_success_payload_without_accepting_it() {
    let temp = tempfile::tempdir().unwrap();
    let server = spawn_models_server(json!({ "data": [] }));
    write_config(
        temp.path(),
        &format!(
            r#"
model_provider = "relay"

[model_providers.relay]
name = "Relay"
base_url = "{}"
experimental_bearer_token = "relay-key"
"#,
            server.base_url
        ),
    );

    let result = read_codex_model_catalog_from_home(
        temp.path(),
        &HashMap::new(),
        reqwest::Client::builder().no_proxy().build().unwrap(),
    )
    .await;

    assert_eq!(result["models"], json!([]));
    assert_eq!(
        result["sources"][0]["attempts"][0]["action"],
        "empty_payload"
    );
    let requests = server.finish();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/v1/models");
}

#[tokio::test]
async fn relay_profile_model_discovery_resolves_serialized_api_keys() {
    for key_name in ["ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY"] {
        for key_in_auth_contents in [true, false] {
            let server = spawn_models_server(json!({
                "data": [{"id": "claude-sonnet-4-6"}]
            }));
            let mut profile = RelayProfile {
                id: "claude-provider".to_string(),
                name: "Claude Provider".to_string(),
                upstream_base_url: server.base_url.clone(),
                target_app: "claude".to_string(),
                api_format: "Anthropic Messages".to_string(),
                model_list: "must-not-be-discovered".to_string(),
                model_mapping: "haiku=must-not-be-discovered".to_string(),
                ..RelayProfile::default()
            };
            let serialized_key =
                serde_json::to_string(&HashMap::from([(key_name, "resolved-profile-key")]))
                    .unwrap();
            if key_in_auth_contents {
                profile.auth_contents = serialized_key;
            } else {
                profile.config_contents = serialized_key;
            }

            let (models, endpoint) = fetch_relay_profile_model_ids(&profile).await.unwrap();

            assert_eq!(models, vec!["claude-sonnet-4-6"]);
            assert_eq!(endpoint, format!("{}/v1/models", server.base_url));
            let requests = server.finish();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].path, "/v1/models");
            assert_eq!(requests[0].authorization, "Bearer resolved-profile-key");
            assert!(requests[0].x_api_key.is_empty());
            assert!(requests[0].anthropic_version.is_empty());
        }
    }
}

#[tokio::test]
async fn claude_model_discovery_uses_current_config_key_when_auth_key_is_stale() {
    let server = spawn_models_server(json!({
        "data": [{"id": "claude-opus-4-8"}]
    }));
    let profile = RelayProfile {
        id: "claude-desktop-provider".to_string(),
        upstream_base_url: server.base_url.clone(),
        target_app: "claude-desktop".to_string(),
        api_format: "Anthropic Messages".to_string(),
        auth_contents: json!({
            "ANTHROPIC_AUTH_TOKEN": "test-stale-auth-key"
        })
        .to_string(),
        config_contents: json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "test-current-config-key"
            }
        })
        .to_string(),
        ..RelayProfile::default()
    };

    let (models, endpoint) = fetch_relay_profile_model_ids(&profile).await.unwrap();

    assert_eq!(models, vec!["claude-opus-4-8"]);
    assert_eq!(endpoint, format!("{}/v1/models", server.base_url));
    let requests = server.finish();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/v1/models");
    assert_eq!(requests[0].authorization, "Bearer test-current-config-key");
    assert_ne!(requests[0].authorization, "Bearer test-stale-auth-key");
    assert!(requests[0].anthropic_version.is_empty());
}

#[tokio::test]
async fn openai_relay_profile_model_discovery_keeps_bearer_only_auth() {
    let server = spawn_models_server(json!({
        "data": [{"id": "gpt-5.6-sol"}]
    }));
    let profile = RelayProfile {
        id: "codex-provider".to_string(),
        upstream_base_url: server.base_url.clone(),
        target_app: "codex".to_string(),
        api_format: "openai_responses".to_string(),
        auth_contents: json!({"OPENAI_API_KEY": "openai-profile-key"}).to_string(),
        ..RelayProfile::default()
    };

    let (models, _) = fetch_relay_profile_model_ids(&profile).await.unwrap();

    assert_eq!(models, vec!["gpt-5.6-sol"]);
    let requests = server.finish();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].authorization, "Bearer openai-profile-key");
    assert!(requests[0].x_api_key.is_empty());
    assert!(requests[0].anthropic_version.is_empty());
}

#[tokio::test]
async fn relay_profile_model_discovery_rejects_empty_standard_catalog_without_fallback() {
    let server = spawn_route_server(vec![("/v1/models", 200, json!({"data": []}))], 1);
    let profile = RelayProfile {
        id: "claude-provider".to_string(),
        upstream_base_url: server.base_url.clone(),
        api_key: "current-profile-key".to_string(),
        target_app: "claude-desktop".to_string(),
        api_format: "Anthropic Messages".to_string(),
        auth_contents: json!({"ANTHROPIC_AUTH_TOKEN": "stale-profile-key"}).to_string(),
        model_list: "mapped-list-model".to_string(),
        model_mapping: "haiku=mapped-request-model".to_string(),
        model_mapping_json: json!({
            "haiku": {"requestModel": "mapped-json-model"}
        })
        .to_string(),
        ..RelayProfile::default()
    };

    let error = fetch_relay_profile_model_ids(&profile)
        .await
        .expect_err("an empty current-key catalog must remain an explicit failure")
        .to_string();

    assert!(error.contains("当前 Key"));
    assert!(error.contains("没有返回可用模型"));
    assert!(!error.contains("current-profile-key"));
    assert!(!error.contains("stale-profile-key"));
    let requests = server.finish();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/v1/models");
    assert_eq!(requests[0].authorization, "Bearer current-profile-key");
}

#[tokio::test]
async fn standard_model_discovery_preserves_successful_empty_catalog() {
    let server = spawn_route_server(vec![("/v1/models", 200, json!({"data": []}))], 1);
    let profile = RelayProfile {
        upstream_base_url: server.base_url.clone(),
        target_app: "claude-desktop".to_string(),
        api_format: "Anthropic Messages".to_string(),
        auth_contents: json!({"ANTHROPIC_AUTH_TOKEN": "probe-key"}).to_string(),
        ..RelayProfile::default()
    };

    let (models, endpoint) = discover_relay_profile_model_ids(&profile).await.unwrap();

    assert!(models.is_empty());
    assert_eq!(endpoint, format!("{}/v1/models", server.base_url));
    let requests = server.finish();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/v1/models");
    assert_eq!(requests[0].authorization, "Bearer probe-key");
}

#[tokio::test]
async fn relay_profile_model_discovery_does_not_hide_models_endpoint_auth_failure() {
    let server = spawn_route_server(
        vec![("/v1/models", 401, json!({"error": "invalid token"}))],
        1,
    );
    let profile = RelayProfile {
        upstream_base_url: server.base_url.clone(),
        target_app: "claude".to_string(),
        auth_contents: json!({"ANTHROPIC_API_KEY": "secret-auth-failure-key"}).to_string(),
        ..RelayProfile::default()
    };

    let error = fetch_relay_profile_model_ids(&profile)
        .await
        .expect_err("401 must remain a model discovery failure")
        .to_string();

    assert!(error.contains("HTTP 401"));
    assert!(!error.contains("secret-auth-failure-key"));
    let requests = server.finish();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/v1/models");
    assert_eq!(requests[0].authorization, "Bearer secret-auth-failure-key");
    assert!(requests[0].x_api_key.is_empty());
    assert!(requests[0].anthropic_version.is_empty());
}

#[test]
fn model_url_candidates_match_cc_switch_compatibility_order() {
    assert_eq!(
        build_models_url_candidates("https://api.example.test").unwrap(),
        vec!["https://api.example.test/v1/models"]
    );
    assert_eq!(
        build_models_url_candidates("https://api.example.test/v1").unwrap(),
        vec!["https://api.example.test/v1/models"]
    );
    assert_eq!(
        build_models_url_candidates("https://api.example.test/api/coding/paas/v4").unwrap(),
        vec![
            "https://api.example.test/api/coding/paas/v4/models",
            "https://api.example.test/api/coding/paas/v4/v1/models",
        ]
    );
    assert_eq!(
        build_models_url_candidates("https://api.example.test/api/anthropic").unwrap(),
        vec![
            "https://api.example.test/api/anthropic/v1/models",
            "https://api.example.test/v1/models",
            "https://api.example.test/models",
        ]
    );
    assert_eq!(
        build_models_url_candidates("https://api.example.test/v1/messages").unwrap(),
        vec!["https://api.example.test/v1/models"]
    );

    let candidates = build_models_url_candidates(
        "https://user:secret@example.test/api/anthropic?route=blue#fragment",
    )
    .unwrap();
    assert_eq!(
        candidates,
        vec![
            "https://example.test/api/anthropic/v1/models?route=blue",
            "https://example.test/v1/models?route=blue",
            "https://example.test/models?route=blue",
        ]
    );
    assert!(!candidates.join("\n").contains("user:secret"));
}

#[tokio::test]
async fn relay_profile_model_discovery_preserves_query_but_redacts_reported_endpoint() {
    let server = spawn_route_server(
        vec![(
            "/v1/models?route=blue",
            200,
            json!({"data": [{"id": "claude-fable-4-1"}]}),
        )],
        1,
    );
    let profile = RelayProfile {
        upstream_base_url: format!("{}?route=blue", server.base_url),
        target_app: "claude-desktop".to_string(),
        api_format: "Anthropic Messages".to_string(),
        api_key: "current-profile-key".to_string(),
        api_key_explicit: true,
        ..RelayProfile::default()
    };

    let (models, endpoint) = fetch_relay_profile_model_ids(&profile).await.unwrap();

    assert_eq!(models, vec!["claude-fable-4-1"]);
    assert_eq!(endpoint, format!("{}/v1/models", server.base_url));
    let requests = server.finish();
    assert_eq!(requests[0].path, "/v1/models?route=blue");
    assert_eq!(requests[0].authorization, "Bearer current-profile-key");
}

#[tokio::test]
async fn relay_profile_model_discovery_parses_wrapped_string_and_map_catalogs() {
    for (payload, expected) in [
        (
            json!({"response": {"payload": {"models": [{"modelId": "claude-opus-4-8"}, {"id": "claude-fable-4-1"}]}}}),
            vec!["claude-opus-4-8", "claude-fable-4-1"],
        ),
        (
            json!({"result": "{\"models\":[{\"id\":\"claude-sonnet-4-5\"}]}"}),
            vec!["claude-sonnet-4-5"],
        ),
        (
            json!({"data": {"claude-haiku-4-5": {}, "claude-fable-4-1": {"enabled": true}}}),
            vec!["claude-haiku-4-5", "claude-fable-4-1"],
        ),
    ] {
        let server = spawn_models_server(payload);
        let profile = RelayProfile {
            upstream_base_url: server.base_url.clone(),
            api_key: "current-profile-key".to_string(),
            api_key_explicit: true,
            ..RelayProfile::default()
        };

        let (mut models, _) = fetch_relay_profile_model_ids(&profile).await.unwrap();
        let mut expected = expected;
        models.sort_unstable();
        expected.sort_unstable();

        assert_eq!(models, expected);
        server.finish();
    }
}

#[tokio::test]
async fn relay_profile_model_discovery_rejects_ambiguous_empty_and_business_error_payloads() {
    for (payload, expected_message) in [
        (json!({}), "JSON"),
        (json!(""), "JSON"),
        (
            json!({"status": "failed", "message": "catalog denied"}),
            "业务错误",
        ),
    ] {
        let server = spawn_models_server(payload);
        let profile = RelayProfile {
            upstream_base_url: server.base_url.clone(),
            api_key: "current-profile-key".to_string(),
            api_key_explicit: true,
            ..RelayProfile::default()
        };

        let error = fetch_relay_profile_model_ids(&profile)
            .await
            .expect_err("ambiguous or failed payload must not be accepted")
            .to_string();

        assert!(
            error.contains(expected_message),
            "unexpected error: {error}"
        );
        assert!(!error.contains("current-profile-key"));
        server.finish();
    }
}

#[tokio::test]
async fn relay_profile_model_discovery_retries_compatible_catalog_after_404_or_405() {
    for retry_status in [404, 405] {
        let server = spawn_route_server(
            vec![
                (
                    "/api/anthropic/v1/models",
                    retry_status,
                    json!({"error": "not found"}),
                ),
                (
                    "/v1/models",
                    200,
                    json!({"data": [{"id": "claude-opus-4-7"}]}),
                ),
            ],
            2,
        );
        let profile = RelayProfile {
            id: format!("compat-{retry_status}"),
            upstream_base_url: format!("{}/api/anthropic", server.base_url),
            target_app: "claude-desktop".to_string(),
            api_format: "Anthropic Messages".to_string(),
            auth_contents: json!({
                "ANTHROPIC_AUTH_TOKEN": "current-profile-key"
            })
            .to_string(),
            ..RelayProfile::default()
        };

        let (models, endpoint) = fetch_relay_profile_model_ids(&profile).await.unwrap();

        assert_eq!(models, vec!["claude-opus-4-7"]);
        assert_eq!(endpoint, format!("{}/v1/models", server.base_url));
        let requests = server.finish();
        assert_eq!(
            requests
                .iter()
                .map(|request| request.path.as_str())
                .collect::<Vec<_>>(),
            vec!["/api/anthropic/v1/models", "/v1/models"]
        );
        for request in requests {
            assert_eq!(request.authorization, "Bearer current-profile-key");
            assert!(request.x_api_key.is_empty());
            assert!(request.anthropic_version.is_empty());
        }
    }
}

#[tokio::test]
async fn relay_profile_model_discovery_stops_compatible_fallback_after_401() {
    let server = spawn_route_server(
        vec![
            (
                "/api/anthropic/v1/models",
                401,
                json!({"error": "invalid token"}),
            ),
            (
                "/v1/models",
                200,
                json!({"data": [{"id": "must-not-be-used"}]}),
            ),
        ],
        1,
    );
    let profile = RelayProfile {
        upstream_base_url: format!("{}/api/anthropic", server.base_url),
        target_app: "claude-desktop".to_string(),
        api_format: "Anthropic Messages".to_string(),
        auth_contents: json!({
            "ANTHROPIC_API_KEY": "current-profile-api-key"
        })
        .to_string(),
        ..RelayProfile::default()
    };

    let error = fetch_relay_profile_model_ids(&profile)
        .await
        .expect_err("401 must stop compatible endpoint fallback")
        .to_string();

    assert!(error.contains("HTTP 401"));
    assert!(!error.contains("current-profile-api-key"));
    let requests = server.finish();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/api/anthropic/v1/models");
    assert_eq!(requests[0].authorization, "Bearer current-profile-api-key");
    assert!(requests[0].x_api_key.is_empty());
    assert!(requests[0].anthropic_version.is_empty());
}

#[tokio::test]
async fn model_catalog_uses_active_relay_profile_model_list_for_display() {
    let _guard = MODEL_CATALOG_ENV_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let codex_home = temp.path().join("codex-home");
    std::fs::create_dir_all(&codex_home).unwrap();
    let settings_path = temp.path().join("settings.json");
    let previous_codex_home = std::env::var_os("CODEX_HOME");
    let previous_settings_path =
        claude_codex_pro_core::paths::set_settings_path_for_tests(Some(settings_path.clone()));
    unsafe {
        std::env::set_var("CODEX_HOME", &codex_home);
    }

    let result = async {
        SettingsStore::new(settings_path)
            .save(&BackendSettings {
                active_relay_id: "relay-a".to_string(),
                relay_profiles: vec![RelayProfile {
                    id: "relay-a".to_string(),
                    name: "Relay A".to_string(),
                    model: "qwen3-coder".to_string(),
                    base_url: "https://example.test/v1".to_string(),
                    protocol: RelayProtocol::Responses,
                    relay_mode: RelayMode::PureApi,
                    model_list: "deepseek-coder\nqwen3-coder\nclaude-compatible".to_string(),
                    codex_catalog_json: r#"[
                        {"displayName":"DeepSeek V4 Flash","model":"deepseek-coder","contextWindow":128000},
                        {"displayName":"Qwen 3 Coder","model":"qwen3-coder","contextWindow":"200000"}
                    ]"#
                    .to_string(),
                    ..RelayProfile::default()
                }],
                ..BackendSettings::default()
            })
            .unwrap();

        read_codex_model_catalog().await
    }
    .await;

    match previous_codex_home {
        Some(value) => unsafe {
            std::env::set_var("CODEX_HOME", value);
        },
        None => unsafe {
            std::env::remove_var("CODEX_HOME");
        },
    }
    claude_codex_pro_core::paths::set_settings_path_for_tests(previous_settings_path);

    assert_eq!(result["status"], "ok");
    assert_eq!(result["model_provider"], "relay-a");
    assert_eq!(result["provider_name"], "Relay A");
    assert_eq!(result["default_model"], "qwen3-coder");
    assert_eq!(
        result["models"],
        json!(["qwen3-coder", "deepseek-coder", "claude-compatible"])
    );
    assert_eq!(result["sources"][0]["type"], "relay_profile_model_list");
    assert_eq!(
        result["model_descriptors"],
        json!([
            {
                "model": "deepseek-coder",
                "display_name": "DeepSeek V4 Flash",
                "context_window": 128000
            },
            {
                "model": "qwen3-coder",
                "display_name": "Qwen 3 Coder",
                "context_window": 200000
            }
        ])
    );
}

#[tokio::test]
async fn model_catalog_merges_local_catalog_file_for_active_relay_profile() {
    let _guard = MODEL_CATALOG_ENV_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let codex_home = temp.path().join("codex-home");
    std::fs::create_dir_all(&codex_home).unwrap();
    std::fs::write(
        codex_home.join("model-catalog.gpt-5.6.json"),
        serde_json::to_vec(&json!({
            "models": [
                {"slug": "gpt-5.6-sol", "display_name": "GPT-5.6 Sol", "visibility": "list", "supported_in_api": true},
                {"slug": "gpt-5.6-terra", "display_name": "GPT-5.6 Terra", "visibility": "list", "supported_in_api": true},
                {"slug": "gpt-5.6-hidden", "display_name": "Hidden", "visibility": "hide", "supported_in_api": true},
                {"slug": "gpt-5.6-disabled", "display_name": "Disabled", "visibility": "list", "supported_in_api": false}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let settings_path = temp.path().join("settings.json");
    let previous_codex_home = std::env::var_os("CODEX_HOME");
    let previous_settings_path =
        claude_codex_pro_core::paths::set_settings_path_for_tests(Some(settings_path.clone()));
    unsafe {
        std::env::set_var("CODEX_HOME", &codex_home);
    }

    let result = async {
        SettingsStore::new(settings_path)
            .save(&BackendSettings {
                active_relay_id: "relay-a".to_string(),
                relay_profiles: vec![RelayProfile {
                    id: "relay-a".to_string(),
                    name: "Relay A".to_string(),
                    model: "gpt-5.5".to_string(),
                    base_url: "https://example.test/v1".to_string(),
                    protocol: RelayProtocol::Responses,
                    relay_mode: RelayMode::PureApi,
                    model_list: "gpt-5.5".to_string(),
                    ..RelayProfile::default()
                }],
                ..BackendSettings::default()
            })
            .unwrap();

        read_codex_model_catalog().await
    }
    .await;

    match previous_codex_home {
        Some(value) => unsafe {
            std::env::set_var("CODEX_HOME", value);
        },
        None => unsafe {
            std::env::remove_var("CODEX_HOME");
        },
    }
    claude_codex_pro_core::paths::set_settings_path_for_tests(previous_settings_path);

    assert_eq!(result["status"], "ok");
    assert_eq!(result["default_model"], "gpt-5.5");
    assert_eq!(
        result["models"],
        json!(["gpt-5.5", "gpt-5.6-sol", "gpt-5.6-terra"])
    );
    assert_eq!(result["sources"][1]["type"], "model_catalog_json");
    assert_eq!(result["sources"][1]["models"], 2);
}

#[tokio::test]
async fn model_catalog_uses_single_provider_when_root_model_provider_is_absent() {
    let temp = tempfile::tempdir().unwrap();
    let server = spawn_models_server(json!({
        "models": ["moonshot-v1", "mimo-v2.5-pro"]
    }));
    write_config(
        temp.path(),
        &format!(
            r#"
[model_providers.only]
name = "Only Provider"
base_url = "{}/v1"
"#,
            server.base_url
        ),
    );

    let result = read_codex_model_catalog_from_home(
        temp.path(),
        &HashMap::new(),
        reqwest::Client::builder().no_proxy().build().unwrap(),
    )
    .await;

    assert_eq!(result["status"], "ok");
    assert_eq!(result["model_provider"], "only");
    assert_eq!(result["models"], json!(["moonshot-v1", "mimo-v2.5-pro"]));
    let requests = server.finish();
    assert_eq!(requests[0].path, "/v1/models");
    assert_eq!(result["responses_api"]["status"], "unknown");
}

#[tokio::test]
async fn model_catalog_merges_models_from_config_model_catalog_json() {
    let temp = tempfile::tempdir().unwrap();
    let server = spawn_models_server(json!({
        "data": [
            {"id": "qwen3-coder"}
        ]
    }));
    let catalog_path = temp.path().join("custom-models.json");
    std::fs::write(
        &catalog_path,
        json!({
            "models": [
                {
                    "slug": "gpt-5.6",
                    "display_name": "GPT-5.6",
                    "visibility": "list",
                    "supported_in_api": true
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    write_config(
        temp.path(),
        &format!(
            r#"
model = "gpt-5.6"
model_provider = "relay"
model_catalog_json = "{}"

[model_providers.relay]
name = "Relay"
base_url = "{}"
experimental_bearer_token = "relay-key"
"#,
            catalog_path.display().to_string().replace('\\', "\\\\"),
            server.base_url
        ),
    );

    let result = read_codex_model_catalog_from_home(
        temp.path(),
        &HashMap::new(),
        reqwest::Client::builder().no_proxy().build().unwrap(),
    )
    .await;

    assert_eq!(result["status"], "ok");
    assert_eq!(result["default_model"], "gpt-5.6");
    assert_eq!(result["models"], json!(["qwen3-coder", "gpt-5.6"]));
    server.finish();
}

#[tokio::test]
async fn model_catalog_reads_single_quoted_config_model_catalog_json_path() {
    let temp = tempfile::tempdir().unwrap();
    let catalog_path = temp.path().join("literal-path-models.json");
    std::fs::write(
        &catalog_path,
        json!({
            "models": [
                {
                    "slug": "gpt-5.6",
                    "visibility": "list",
                    "supported_in_api": true
                },
                {
                    "slug": "hidden-test-model",
                    "visibility": "hidden",
                    "supported_in_api": true
                },
                {
                    "slug": "chatgpt-only-test-model",
                    "visibility": "list",
                    "supported_in_api": false
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    write_config(
        temp.path(),
        &format!(
            r#"
model = "gpt-5.6"
model_catalog_json = '{}'
"#,
            catalog_path.display()
        ),
    );

    let result = read_codex_model_catalog_from_home(
        temp.path(),
        &HashMap::new(),
        reqwest::Client::builder().no_proxy().build().unwrap(),
    )
    .await;

    assert_eq!(result["status"], "ok");
    assert_eq!(result["default_model"], "gpt-5.6");
    assert_eq!(result["models"], json!(["gpt-5.6"]));
    assert_eq!(result["sources"][0]["status"], "ok");
    assert_eq!(result["sources"][0]["models"], 1);
}

#[tokio::test]
async fn model_catalog_leaves_responses_api_unknown_without_probe() {
    let temp = tempfile::tempdir().unwrap();
    let server = spawn_models_server(json!({
        "data": [
            {"id": "legacy-model"}
        ]
    }));
    write_config(
        temp.path(),
        &format!(
            r#"
model = "legacy-model"

[model_providers.legacy]
name = "Legacy"
base_url = "{}"
"#,
            server.base_url
        ),
    );

    let result = read_codex_model_catalog_from_home(
        temp.path(),
        &HashMap::new(),
        reqwest::Client::builder().no_proxy().build().unwrap(),
    )
    .await;

    assert_eq!(result["status"], "ok");
    assert_eq!(result["responses_api"]["status"], "unknown");
    assert_eq!(result["responses_api"]["endpoint"], "");
    assert_eq!(result["sources"][0]["responses_api"]["status"], "unknown");
    let requests = server.finish();
    assert_eq!(requests[0].path, "/v1/models");
}

fn write_config(home: &Path, contents: &str) {
    std::fs::write(home.join("config.toml"), contents.trim_start()).unwrap();
}

struct ModelsServer {
    base_url: String,
    handle: thread::JoinHandle<Vec<ModelsRequest>>,
}

impl ModelsServer {
    fn finish(self) -> Vec<ModelsRequest> {
        self.handle.join().unwrap()
    }
}

struct ModelsRequest {
    path: String,
    authorization: String,
    x_api_key: String,
    anthropic_version: String,
}

fn spawn_models_server(payload: serde_json::Value) -> ModelsServer {
    spawn_route_server(vec![("/v1/models", 200, payload)], 1)
}

fn spawn_route_server(
    routes: Vec<(&str, u16, serde_json::Value)>,
    expected_requests: usize,
) -> ModelsServer {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let base_url = format!("http://{address}");
    listener
        .set_nonblocking(true)
        .expect("listener should switch to nonblocking mode");
    let routes = routes
        .into_iter()
        .map(|(path, status, payload)| (path.to_string(), status, payload.to_string()))
        .collect::<Vec<_>>();
    let handle = thread::spawn(move || {
        let started = std::time::Instant::now();
        let mut requests = Vec::new();
        let mut last_request_at = None;
        while started.elapsed() < std::time::Duration::from_secs(2) {
            if requests.len() >= expected_requests
                && last_request_at.is_some_and(|last: std::time::Instant| {
                    last.elapsed() >= std::time::Duration::from_millis(100)
                })
            {
                break;
            }
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            };
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("test request read timeout should be configured");
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 4096];
            let read_started = std::time::Instant::now();
            while !buffer.windows(4).any(|window| window == b"\r\n\r\n")
                && read_started.elapsed() < std::time::Duration::from_secs(2)
            {
                match stream.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(bytes) => buffer.extend_from_slice(&chunk[..bytes]),
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(error) => panic!("failed to read test request: {error}"),
                }
            }
            if buffer.is_empty() {
                continue;
            }
            let request = String::from_utf8_lossy(&buffer).to_string();
            let request_path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or_default()
                .to_string();
            let header_value = |wanted: &str| {
                request
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case(wanted)
                            .then(|| value.trim().to_string())
                    })
                    .unwrap_or_default()
            };
            let authorization = header_value("authorization");
            let x_api_key = header_value("x-api-key");
            let anthropic_version = header_value("anthropic-version");
            let (status, body) = routes
                .iter()
                .find(|(path, _, _)| path == &request_path)
                .map(|(_, status, body)| (*status, body.as_str()))
                .unwrap_or((404, r#"{"error":"not found"}"#));
            let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            requests.push(ModelsRequest {
                path: request_path,
                authorization,
                x_api_key,
                anthropic_version,
            });
            last_request_at = Some(std::time::Instant::now());
        }
        requests
    });
    ModelsServer { base_url, handle }
}
