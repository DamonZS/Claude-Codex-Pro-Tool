use base64::Engine;
use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;

use crate::settings::BackendSettings;

const RENDERER_SCRIPT: &str = include_str!("../../../assets/inject/renderer-inject.js");
const CLAUDE_CHINESE_INJECT_SCRIPT: &str =
    include_str!("../../../assets/inject/claude-chinese-inject.js");
const CODEX_THEME_LOADER_SCRIPT: &str =
    include_str!("../../../assets/inject/codex-theme-loader.js");
const SUPPORT_PAYMENT_QR: &[u8] = include_bytes!("../../../assets/images/support-payment-qr.png");
const CONTACT_WECHAT_QR: &[u8] = include_bytes!("../../../assets/images/contact-wechat-qr.jpg");
pub const DIAGNOSTIC_BUILD_ID: &str = "diag-20260518-1";
pub const CODEX_THEME_PAYLOAD_GLOBAL: &str = "__CLAUDE_CODEX_PRO_CODEX_THEME_PAYLOAD__";
pub const CODEX_THEME_LOADER_GLOBAL: &str = "__CLAUDE_CODEX_PRO_CODEX_THEME_LOADER__";
pub const CODEX_THEME_RESULT_GLOBAL: &str = "__CLAUDE_CODEX_PRO_CODEX_THEME_RESULT__";
pub const CODEX_THEME_STYLE_ID: &str = "claude-codex-pro-codex-theme";

pub fn renderer_script() -> &'static str {
    RENDERER_SCRIPT
}

pub fn claude_chinese_injection_script() -> &'static str {
    CLAUDE_CHINESE_INJECT_SCRIPT
}

pub fn codex_theme_loader_script() -> &'static str {
    CODEX_THEME_LOADER_SCRIPT
}

pub fn codex_theme_injection_script<T: Serialize>(payload: &T) -> anyhow::Result<String> {
    let payload = serde_json::to_string(payload)?;
    Ok(format!(
        "window.{CODEX_THEME_PAYLOAD_GLOBAL} = {payload};\n{}",
        codex_theme_loader_script()
    ))
}

pub fn injection_script(helper_port: u16) -> String {
    injection_script_with_settings(helper_port, &BackendSettings::default())
}

pub fn injection_script_with_settings(helper_port: u16, settings: &BackendSettings) -> String {
    let helper_url = format!("http://127.0.0.1:{helper_port}");
    let image_overlay = image_overlay_config(helper_port, settings);
    let support_payment_qr = image_data_uri("image/png", SUPPORT_PAYMENT_QR);
    let contact_wechat_qr = image_data_uri("image/jpeg", CONTACT_WECHAT_QR);
    let announcement = crate::ads::bundled_ad_config();
    let plugin_marketplaces = crate::codex_plugin_marketplace::local_plugin_marketplaces();
    // The helper token is embedded here so the injected renderer (and only it)
    // can authenticate to the local helper. It sits in the bootstrap prologue,
    // which runs in a closure scope, so a random web page that never received
    // this script cannot read the token off `window`.
    let helper_token = crate::helper_auth::helper_token();
    format!(
        "window.__CODEX_SESSION_DELETE_HELPER__ = {};\nwindow.{} = {};\nwindow.__CLAUDE_CODEX_PRO_VERSION__ = {};\nwindow.__CLAUDE_CODEX_PRO_BUILD__ = {};\nwindow.__CLAUDE_CODEX_PRO_IMAGE_OVERLAY__ = {};\nwindow.__CLAUDE_CODEX_PRO_SUPPORT_PAYMENT_QR__ = {};\nwindow.__CLAUDE_CODEX_PRO_CONTACT_WECHAT_QR__ = {};\nwindow.__CLAUDE_CODEX_PRO_ANNOUNCEMENT__ = {};\nwindow.__CLAUDE_CODEX_PRO_PLUGIN_MARKETPLACES__ = {};\n{}",
        serde_json::to_string(&helper_url).expect("helper URL should serialize"),
        crate::helper_auth::HELPER_TOKEN_GLOBAL,
        serde_json::to_string(helper_token).expect("helper token should serialize"),
        serde_json::to_string(crate::version::VERSION).expect("version should serialize"),
        serde_json::to_string(DIAGNOSTIC_BUILD_ID).expect("build id should serialize"),
        serde_json::to_string(&image_overlay).expect("image overlay config should serialize"),
        serde_json::to_string(&support_payment_qr).expect("support payment QR should serialize"),
        serde_json::to_string(&contact_wechat_qr).expect("contact WeChat QR should serialize"),
        serde_json::to_string(&announcement).expect("announcement config should serialize"),
        serde_json::to_string(&plugin_marketplaces)
            .expect("plugin marketplace config should serialize"),
        renderer_script(),
    )
}

pub fn image_overlay_config(helper_port: u16, settings: &BackendSettings) -> Value {
    let has_path = !settings.codex_app_image_overlay_path.trim().is_empty();
    let enabled = settings.codex_app_image_overlay_enabled && has_path;
    let data_url = if enabled {
        image_file_data_uri(Path::new(settings.codex_app_image_overlay_path.trim()))
            .unwrap_or_default()
    } else {
        String::new()
    };
    json!({
        "enabled": enabled && !data_url.is_empty(),
        "opacity": f64::from(settings.codex_app_image_overlay_opacity.clamp(1, 100)) / 100.0,
        "dataUrl": data_url,
        "imageUrl": if enabled {
            format!("http://127.0.0.1:{helper_port}/overlay/image")
        } else {
            String::new()
        },
    })
}

fn image_data_uri(mime_type: &str, bytes: &[u8]) -> String {
    format!(
        "data:{mime_type};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

fn image_file_data_uri(path: &Path) -> Option<String> {
    let mime_type = image_content_type(path)?;
    let bytes = std::fs::read(path).ok()?;
    Some(image_data_uri(mime_type, &bytes))
}

fn image_content_type(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => Some("image/png"),
        Some("jpg") | Some("jpeg") => Some("image/jpeg"),
        Some("webp") => Some("image/webp"),
        Some("gif") => Some("image/gif"),
        Some("bmp") => Some("image/bmp"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_theme_loader_v4_materializes_and_releases_blob_assets() {
        let script = codex_theme_loader_script();

        for contract in [
            "const LOADER_VERSION = 4",
            "css_variables",
            "root_attributes",
            "asset_data_uris",
            "assetCssVariables",
            "new Blob([bytes]",
            "URL.createObjectURL",
            "URL.revokeObjectURL",
            "dataUriToBlobAsset",
            "stageAssets",
            "migrateV3Ownership",
            "payload.rootClasses",
            "payload.rootAttributes",
        ] {
            assert!(
                script.contains(contract),
                "Codex theme loader must consume explicit v4 payload state: {contract}"
            );
        }
        assert!(
            !script.contains("return `url(\"${dataUri}\")`"),
            "large Data URIs must not be assigned directly to CSS variables"
        );
        assert!(
            !script.contains("extractRootThemeClasses"),
            "root classes must come from root_attributes, not CSS text scanning"
        );
    }

    #[test]
    fn codex_theme_loader_rejects_external_css_resources_and_keeps_local_assets() {
        let script = codex_theme_loader_script();

        assert!(script.contains("theme CSS must not contain @import"));
        assert!(script.contains("theme CSS contains an unsafe resource URL"));
        assert!(script.contains("CSS_IMPORT_PATTERN"));
        assert!(script.contains("CSS_URL_PATTERN"));
        assert!(script.contains("SAFE_CSS_URL_PATTERN"));
        assert!(script.contains("data:image\\/(?:png|jpeg|webp)"));
        assert!(script.contains("blob:"));
        assert!(script.contains("(?:\\.\\.?\\/|\\/(?!\\/))"));
        assert!(!script.contains("https?:"));
        assert!(!script.contains("file:"));
    }

    #[test]
    fn codex_theme_loader_records_and_conditionally_restores_ownership() {
        let script = codex_theme_loader_script();

        for contract in [
            "hadOriginal",
            "originalValue",
            "writtenValue",
            "originallyPresent",
            "originalPriority",
            "writtenPriority",
            "variableMatchesWrittenValue",
            "attributeMatchesWrittenValue",
            "restoreOwnedRootState",
            "ownership_conflict",
        ] {
            assert!(
                script.contains(contract),
                "Codex theme loader must preserve per-item ownership: {contract}"
            );
        }
    }

    #[test]
    fn codex_theme_loader_repairs_same_generation_and_converges_style() {
        let script = codex_theme_loader_script();

        for contract in [
            "payloadSignature === payload.signature",
            "repairActivePayload",
            "convergeOwnedStyle",
            "document.querySelectorAll(STYLE_SELECTOR)",
            "styleNodes.slice(1)",
            "repair_verification_failed",
            "\"repaired\"",
            "\"healthy\"",
        ] {
            assert!(
                script.contains(contract),
                "Codex theme loader must repair and verify owned state: {contract}"
            );
        }
    }

    #[test]
    fn codex_theme_injection_keeps_a_dedicated_mount_point() {
        let script = codex_theme_injection_script(&serde_json::json!({
            "theme_id": "contract-theme",
            "generation": 7,
            "css": ":root { color: var(--ccp-theme-accent); }",
            "is_default": false,
            "css_variables": {
                "--ccp-theme-accent": "#ff3344"
            },
            "root_attributes": {
                "classes": ["ccp-theme-contract"],
                "attributes": {
                    "data-ccp-theme-tone": "dark"
                }
            },
            "asset_data_uris": {
                "--ccp-theme-art": "data:image/png;base64,iVBORw0KGgo="
            }
        }))
        .expect("theme payload should serialize");

        assert!(script.contains(CODEX_THEME_PAYLOAD_GLOBAL));
        assert!(script.contains(CODEX_THEME_LOADER_GLOBAL));
        assert!(script.contains(CODEX_THEME_RESULT_GLOBAL));
        assert!(script.contains(CODEX_THEME_STYLE_ID));
        assert!(!script.contains("CLAUDE_CODEX_PRO_TRANSLATE"));
        assert!(!script.contains("MutationObserver"));
        assert!(!script.contains("addEventListener"));
    }
}
