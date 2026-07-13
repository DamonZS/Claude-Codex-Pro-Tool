use claude_codex_pro_core::update::{
    Release, UpdateDownloadProgress, download_asset_to, is_newer_version, parse_version_tag,
    release_from_github_payload, release_from_latest_json_payload,
    release_from_latest_redirect_url, safe_asset_name, select_update_asset, validate_update_asset,
};
use serde_json::json;

#[test]
fn parse_version_tag_accepts_prefix_and_suffix() {
    assert_eq!(parse_version_tag("v1.2.3").unwrap(), vec![1, 2, 3]);
    assert_eq!(parse_version_tag("1.2.3").unwrap(), vec![1, 2, 3]);
    assert_eq!(parse_version_tag("v1.2.3-beta.1").unwrap(), vec![1, 2, 3]);
}

#[test]
fn version_comparison_uses_numeric_segments() {
    assert!(is_newer_version("v1.0.10", "1.0.4").unwrap());
    assert!(!is_newer_version("v1.0.4", "1.0.4").unwrap());
    assert!(!is_newer_version("v1.0.3", "1.0.4").unwrap());
}

#[test]
fn v0_auto_release_tags_are_newer_than_legacy_semver_releases() {
    assert!(is_newer_version("V0.01", "1.2.9").unwrap());
    assert!(is_newer_version("V0.02", "V0.01").unwrap());
    assert!(is_newer_version("V0.12", "1.2.9").unwrap());
    assert!(is_newer_version("V1.00", "V0.99").unwrap());
    assert!(!is_newer_version("V0.01", "V0.02").unwrap());
    assert!(!is_newer_version("v1.2.9", "V0.01").unwrap());
    assert!(is_newer_version("V0.39", "dev-0.12.0").unwrap());
}

#[test]
fn github_payload_selects_platform_installer() {
    let release = release_from_github_payload(&json!({
        "tag_name": "v1.0.9",
        "html_url": "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/tag/v1.0.9",
        "body": "fixes",
        "assets": [
            {"name": "source.zip", "browser_download_url": "https://example.test/source.zip"},
            {"name": "claude-codex-pro-manager.exe", "browser_download_url": "https://example.test/manager.exe"},
            {"name": "claude-codex-pro-1.0.9-windows-x64-setup.exe", "browser_download_url": "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/v1.0.9/claude-codex-pro-1.0.9-windows-x64-setup.exe"},
            {"name": "claude-codex-pro-1.0.9-macos-x64.dmg", "browser_download_url": "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/v1.0.9/claude-codex-pro-1.0.9-macos-x64.dmg"},
            {"name": "claude-codex-pro-1.0.9-macos-arm64.dmg", "browser_download_url": "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/v1.0.9/claude-codex-pro-1.0.9-macos-arm64.dmg"}
        ]
    }))
    .unwrap();

    assert_eq!(release.version, "v1.0.9");
    if cfg!(all(windows, target_arch = "x86_64")) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("claude-codex-pro-1.0.9-windows-x64-setup.exe")
        );
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("claude-codex-pro-1.0.9-macos-x64.dmg")
        );
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("claude-codex-pro-1.0.9-macos-arm64.dmg")
        );
    } else {
        assert_eq!(release.asset_name.as_deref(), None);
    }
}

#[test]
fn latest_json_payload_selects_platform_installer_without_github_api_shape() {
    let release = release_from_latest_json_payload(&json!({
        "version": "v1.1.6",
        "url": "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/tag/v1.1.6",
        "body": "静态更新描述",
        "assets": [
            {"name": "source.zip", "url": "https://example.test/source.zip"},
            {"name": "claude-codex-pro-1.1.6-windows-x64-setup.exe", "url": "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/v1.1.6/claude-codex-pro-1.1.6-windows-x64-setup.exe"},
            {"name": "claude-codex-pro-1.1.6-macos-x64.dmg", "url": "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/v1.1.6/claude-codex-pro-1.1.6-macos-x64.dmg"},
            {"name": "claude-codex-pro-1.1.6-macos-arm64.dmg", "url": "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/v1.1.6/claude-codex-pro-1.1.6-macos-arm64.dmg"}
        ]
    }))
    .unwrap();

    assert_eq!(release.version, "v1.1.6");
    assert_eq!(release.body, "静态更新描述");
    if cfg!(all(windows, target_arch = "x86_64")) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("claude-codex-pro-1.1.6-windows-x64-setup.exe")
        );
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("claude-codex-pro-1.1.6-macos-x64.dmg")
        );
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("claude-codex-pro-1.1.6-macos-arm64.dmg")
        );
    } else {
        assert_eq!(release.asset_name.as_deref(), None);
    }
}

#[test]
fn latest_release_redirect_builds_trusted_platform_installer_without_api_payload() {
    let release = release_from_latest_redirect_url(
        "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/tag/V0.39",
    )
    .unwrap();

    assert_eq!(release.version, "V0.39");
    assert_eq!(
        release.url,
        "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/tag/V0.39"
    );
    if cfg!(all(windows, target_arch = "x86_64")) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("claude-codex-pro-0.39-windows-x64-setup.exe")
        );
        assert_eq!(
            release.asset_url.as_deref(),
            Some(
                "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/V0.39/claude-codex-pro-0.39-windows-x64-setup.exe"
            )
        );
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("claude-codex-pro-0.39-macos-x64.dmg")
        );
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("claude-codex-pro-0.39-macos-arm64.dmg")
        );
    } else {
        assert_eq!(release.asset_name, None);
        assert_eq!(release.asset_url, None);
    }
}

#[test]
fn latest_release_redirect_rejects_untrusted_or_malformed_tags() {
    assert!(
        release_from_latest_redirect_url(
            "https://github.com/attacker/Claude-Codex-Pro-Tool/releases/tag/V0.39"
        )
        .is_err()
    );
    assert!(
        release_from_latest_redirect_url(
            "https://example.test/DamonZS/Claude-Codex-Pro-Tool/releases/tag/V0.39"
        )
        .is_err()
    );
    assert!(
        release_from_latest_redirect_url(
            "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/tag/not-a-version"
        )
        .is_err()
    );
}

#[test]
fn asset_selection_prefers_current_platform_artifacts() {
    let assets = vec![
        (
            "claude-codex-pro-plus-9.9.9-windows-x64-setup.exe".to_string(),
            "https://example.test/old-plus-setup.exe".to_string(),
        ),
        (
            "claude-codex-pro-plus-9.9.9-macos-x64.dmg".to_string(),
            "https://example.test/old-plus.dmg".to_string(),
        ),
        (
            "claude-codex-pro.zip".to_string(),
            "https://example.test/source.zip".to_string(),
        ),
        (
            "claude-codex-pro-manager.exe".to_string(),
            "https://example.test/manager.exe".to_string(),
        ),
        (
            "claude-codex-pro-1.0.9-windows-x64-setup.exe".to_string(),
            "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/v1.0.9/claude-codex-pro-1.0.9-windows-x64-setup.exe".to_string(),
        ),
        (
            "claude-codex-pro-1.0.9-macos-x64.dmg".to_string(),
            "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/v1.0.9/claude-codex-pro-1.0.9-macos-x64.dmg".to_string(),
        ),
        (
            "claude-codex-pro-1.0.9-macos-arm64.dmg".to_string(),
            "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/v1.0.9/claude-codex-pro-1.0.9-macos-arm64.dmg".to_string(),
        ),
    ];

    if cfg!(all(windows, target_arch = "x86_64")) {
        let selected = select_update_asset(&assets).unwrap();
        assert_eq!(
            selected.name,
            "claude-codex-pro-1.0.9-windows-x64-setup.exe"
        );
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        let selected = select_update_asset(&assets).unwrap();
        assert_eq!(selected.name, "claude-codex-pro-1.0.9-macos-x64.dmg");
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        let selected = select_update_asset(&assets).unwrap();
        assert_eq!(selected.name, "claude-codex-pro-1.0.9-macos-arm64.dmg");
    } else {
        assert!(select_update_asset(&assets).is_none());
    }
}

#[test]
fn safe_asset_name_rejects_path_traversal() {
    assert_eq!(safe_asset_name("pkg.zip").unwrap(), "pkg.zip");
    assert!(safe_asset_name("../pkg.zip").is_err());
    assert!(safe_asset_name("").is_err());
}

#[test]
fn download_asset_to_writes_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let release = Release {
        version: "v1.0.9".to_string(),
        url: "https://example.test".to_string(),
        body: "fixes".to_string(),
        asset_name: Some("pkg.zip".to_string()),
        asset_url: Some("https://example.test/pkg.zip".to_string()),
    };

    let path = download_asset_to(&release, b"abcdef", dir.path()).unwrap();

    assert_eq!(path, dir.path().join("pkg.zip"));
    assert_eq!(std::fs::read(path).unwrap(), b"abcdef");
}

#[test]
fn download_progress_calculates_bounded_percent_and_camel_case_payload() {
    let progress = UpdateDownloadProgress::new("downloading", 25, Some(100));
    let payload = serde_json::to_value(&progress).unwrap();

    assert_eq!(progress.percent, Some(25));
    assert_eq!(payload["downloadedBytes"], 25);
    assert_eq!(payload["totalBytes"], 100);
    assert_eq!(payload["percent"], 25);
    assert_eq!(
        UpdateDownloadProgress::new("downloading", 150, Some(100)).percent,
        Some(100)
    );
    assert_eq!(
        UpdateDownloadProgress::new("connecting", 0, None).percent,
        None
    );
}

#[test]
fn updater_uses_fast_release_fallback_and_partial_streaming_contract() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should live under crates/claude-codex-pro-core");
    let source =
        std::fs::read_to_string(repo.join("crates/claude-codex-pro-core/src/update.rs")).unwrap();

    assert!(source.contains("DEFAULT_GITHUB_API_URL"));
    assert!(source.contains("DEFAULT_LATEST_RELEASE_URL"));
    assert!(source.contains("fetch_latest_redirect_release"));
    assert!(source.contains("fetch_latest_redirect_release_direct"));
    assert!(source.contains("retry_latest_redirect_release"));
    assert!(source.contains("UPDATE_RELEASE_CACHE_TTL"));
    assert!(source.contains("fetch_update_download_response"));
    assert!(source.contains(".no_proxy()"));
    assert!(source.contains("pub async fn fetch_current_release()"));
    assert!(source.contains("tokio::select!"));
    assert!(source.contains("response.bytes_stream()"));
    assert!(source.contains("format!(\"{safe_name}.part\")"));
    assert!(source.contains("MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH"));
    assert!(!source.contains(".bytes()\n            .await?"));
}

#[test]
fn update_asset_validation_rejects_untrusted_url_components() {
    let Some(name) = current_platform_asset_name() else {
        return;
    };
    let valid =
        format!("https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/V0.42/{name}");

    assert!(validate_update_asset(name, &valid).is_ok());
    assert!(validate_update_asset(name, &valid.replacen("https://", "http://", 1)).is_err());
    assert!(
        validate_update_asset(
            name,
            &valid.replacen("github.com", "github.com.evil.test", 1)
        )
        .is_err()
    );
    assert!(validate_update_asset(name, &valid.replacen("DamonZS", "attacker", 1)).is_err());
    assert!(validate_update_asset(name, &format!("{valid}?download=1")).is_err());
    assert!(
        validate_update_asset(
            name,
            &valid.replace(name, "claude-codex-pro-V0.42-source.zip")
        )
        .is_err()
    );
}

#[test]
fn update_asset_validation_rejects_wrong_platform_installer() {
    let wrong_name = if cfg!(windows) {
        "claude-codex-pro-V0.42-macos-x64.dmg"
    } else {
        "claude-codex-pro-V0.42-windows-x64-setup.exe"
    };
    let url = format!(
        "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download/V0.42/{wrong_name}"
    );

    assert!(validate_update_asset(wrong_name, &url).is_err());
}

fn current_platform_asset_name() -> Option<&'static str> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some("claude-codex-pro-V0.42-windows-x64-setup.exe")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("claude-codex-pro-V0.42-macos-x64.dmg")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("claude-codex-pro-V0.42-macos-arm64.dmg")
    } else {
        None
    }
}

#[test]
fn windows_update_launch_uses_shell_open_path_contract() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should live under crates/claude-codex-pro-core");

    let update_source =
        std::fs::read_to_string(repo.join("crates/claude-codex-pro-core/src/update.rs")).unwrap();
    let lib_source =
        std::fs::read_to_string(repo.join("crates/claude-codex-pro-core/src/lib.rs")).unwrap();
    let windows_source = std::fs::read_to_string(
        repo.join("crates/claude-codex-pro-core/src/windows_integration.rs"),
    )
    .unwrap();

    assert!(update_source.contains("crate::windows_open_path(path)"));
    assert!(!update_source.contains("Command::new(path)"));
    assert!(lib_source.contains("pub fn windows_open_path(path: &std::path::Path)"));
    assert!(lib_source.contains("windows_integration::open_path(path)"));
    assert!(windows_source.contains("pub fn open_path(path: &Path) -> anyhow::Result<()>"));
    assert!(windows_source.contains("ShellExecuteW("));
}
