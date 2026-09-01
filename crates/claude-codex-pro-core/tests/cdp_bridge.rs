use base64::Engine;
use claude_codex_pro_core::assets;
use claude_codex_pro_core::bridge::{self, BRIDGE_BINDING_NAME};
use claude_codex_pro_core::cdp::{
    CdpTarget, is_codex_page_target, list_targets, pick_injectable_codex_page_target,
    pick_page_target,
};

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::future::Future;
use std::io::Write;
use std::net::SocketAddr;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{Notify, oneshot};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

fn target(id: &str, kind: &str, title: &str, url: &str, websocket_url: Option<&str>) -> CdpTarget {
    CdpTarget {
        id: id.to_string(),
        target_type: kind.to_string(),
        title: title.to_string(),
        url: url.to_string(),
        web_socket_debugger_url: websocket_url.map(str::to_string),
    }
}

fn source_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let (_, rest) = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing source section start: {start}"));
    rest.split_once(end)
        .map(|(section, _)| section)
        .unwrap_or_else(|| panic!("missing source section end: {end}"))
}

#[test]
fn bridge_script_defines_expected_globals_and_binding() {
    let script = bridge::build_bridge_script(BRIDGE_BINDING_NAME);

    assert!(script.contains("window.__codexSessionDeleteBridge"));
    assert!(script.contains("window.__codexSessionDeleteResolve"));
    assert!(script.contains("window.__codexSessionDeleteReject"));
    assert!(script.contains("codexSessionDeleteV2"));
    assert!(script.contains("window.__codexSessionDeleteCallbacks instanceof Map"));
    assert!(script.contains("Number.isSafeInteger(window.__codexSessionDeleteSeq)"));
    assert!(!script.contains("window.__codexSessionDeleteCallbacks = new Map();"));
    assert!(!script.contains("window.__codexSessionDeleteSeq = 0;"));
}

#[test]
fn injection_script_prefixes_helper_url() {
    let script = assets::injection_script(57321);

    assert!(script.contains("window.__CODEX_SESSION_DELETE_HELPER__"));
    assert!(script.contains("http://127.0.0.1:57321"));
    assert!(script.contains("window.__CLAUDE_CODEX_PRO_VERSION__"));
    assert!(script.contains(claude_codex_pro_core::version::VERSION));
    assert!(script.contains("https://discord.gg/Q9cbMaWsb"));
    assert!(script.contains("data-claude-codex-pro-discord"));
}

#[test]
fn injection_script_does_not_scan_codex_model_menus() {
    let script = assets::injection_script(57321);

    for forbidden in [
        "function isClaudeCodexProDialogNode(node)",
        "codexModelMenuCandidates",
        "codexModelMenuItemLooksLikeModel",
        "cleanupCodexInjectedModelGroups",
        "installCodexModelDropdownObserver",
        "data-claude-codex-pro-model-group",
        "CCP 模型增强",
    ] {
        assert!(
            !script.contains(forbidden),
            "obsolete model-menu marker: {forbidden}"
        );
    }
}

#[test]
fn injection_script_never_translates_codex_page_content() {
    let script = assets::injection_script(57321);

    assert!(!script.contains("chineseOverlayEnabled: \"claudeAppChineseOverlayEnabled\""));
    assert!(script.contains("settings.chineseOverlayEnabled = false;"));
    let translator = script
        .split("function translateClaudeChineseText(value) {")
        .nth(1)
        .and_then(|rest| {
            rest.split("function protectClaudeChineseOverlayBrands")
                .next()
        })
        .expect("Codex translation guard function");
    assert!(translator.contains("return String(value || \"\");"));
    assert!(!translator.contains("replaceAll"));
    assert!(!translator.contains("new Map"));
    assert!(!script.contains("ensureClaudeChineseOverlayObserver();"));
    assert!(!script.contains("runScanStep(refreshClaudeChineseOverlay);"));
}

#[test]
fn injection_script_exposes_image_overlay_config() {
    let temp = tempfile::tempdir().unwrap();
    let image_path = temp.path().join("overlay.png");
    std::fs::write(
        &image_path,
        base64::engine::general_purpose::STANDARD
            .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=")
            .unwrap(),
    )
    .unwrap();
    let settings = claude_codex_pro_core::settings::BackendSettings {
        codex_app_image_overlay_enabled: true,
        codex_app_image_overlay_path: image_path.to_string_lossy().to_string(),
        codex_app_image_overlay_opacity: 42,
        ..Default::default()
    };
    let script = assets::injection_script_with_settings(57321, &settings);

    assert!(script.contains("window.__CLAUDE_CODEX_PRO_IMAGE_OVERLAY__"));
    assert!(script.contains("window.__CLAUDE_CODEX_PRO_SUPPORT_PAYMENT_QR__"));
    assert!(script.contains("\"enabled\":true"));
    assert!(script.contains("\"opacity\":0.42"));
    assert!(script.contains("\"dataUrl\":\"data:image/png;base64,"));
    assert!(script.contains("http://127.0.0.1:57321/overlay/image"));
}

#[test]
fn injection_script_installs_image_overlay_from_data_uri() {
    let script = assets::injection_script(57321);

    assert!(script.contains("const source = config.dataUrl || \"\""));
    assert!(script.contains("image.src = source"));
    assert!(script.contains("image_overlay_installed"));
}

#[test]
fn injection_script_marks_diagnostic_build_and_reports_script_loaded() {
    let script = assets::injection_script(57321);

    assert!(script.contains("window.__CLAUDE_CODEX_PRO_BUILD__"));
    assert!(script.contains(claude_codex_pro_core::assets::DIAGNOSTIC_BUILD_ID));
    assert!(script.contains("script_loaded"));
    assert!(script.contains("data-claude-codex-pro-build"));
}

#[test]
fn injection_script_anchors_status_entry_to_right_titlebar_controls() {
    let script = assets::injection_script(57321);

    assert!(script.contains("const claudeCodexProMenuVersion = \"12\""));
    assert!(script.contains("menu.dataset.claudeCodexProMenuVersion = claudeCodexProMenuVersion;"));
    assert!(script.contains("findCodexStatusRightAnchor"));
    assert!(script.contains("codexTitlebarControlLabel"));
    assert!(script.contains("function codexWindowControlsOverlayAnchor(headerRect)"));
    assert!(script.contains("navigator.windowControlsOverlay"));
    assert!(script.contains("getTitlebarAreaRect"));
    assert!(script.contains("overlayRect.right < window.innerWidth - 1"));
    assert!(script.contains("left: overlayRect.right"));
    assert!(script.contains("const overlayAnchor = codexWindowControlsOverlayAnchor(headerRect);"));
    assert!(script.contains("if (overlayAnchor) return overlayAnchor;"));
    assert!(script.contains("const minimizeKeywords = [\"minimize\", \"最小化\"]"));
    assert!(script.contains("document.querySelectorAll?.('[aria-label], [title], [data-testid]')"));
    assert!(script.contains("rect.top > headerRect.bottom"));
    assert!(script.contains("rect.left >= headerRect.left + headerRect.width * 0.5"));
    assert!(script.contains("anchorRect.left - menuWidth - 8"));
    assert!(script.contains("headerRect.right - menuWidth - 16"));
    assert!(script.contains("--claude-codex-pro-menu-left"));
    assert!(script.contains("CCP ${claudeCodexProVersion}"));
    assert!(script.contains(
        "setCssPropIfChanged(menu, \"--claude-codex-pro-menu-left\", `${fallbackLeft}px`)"
    ));
    assert!(script.contains("statusRect.left - badgeWidth - 8"));
    assert!(script.contains("windowControlsOverlay.addEventListener(\"geometrychange\""));
    assert!(script.contains("windowControlsOverlay.removeEventListener(\"geometrychange\""));
    assert!(!script.contains("data-codex-frontend-indicator=\"true\""));
    assert!(script.contains("data-codex-backend-indicator=\"true\""));
    assert!(script.contains(".claude-codex-pro-window-status-dot[data-status=\"checking\"]"));
    assert!(script.contains("background: transparent"));
    assert!(script.contains("color: inherit"));
    assert!(script.contains("color: #a9a4a9"));
    assert!(!script.contains("--claude-codex-pro-window-text-color"));
    assert!(!script.contains("function setWindowTextColorFromAnchor"));
    assert!(script.contains(".codex-memory-count { color: inherit; font-weight: 700; }"));
    assert!(script.contains("border: 1px solid #dce3ed"));
    assert!(script.contains("border-radius: 8px"));
    assert!(script.contains("background: #ffffff"));
    assert!(script.contains("color: #172033"));
    assert!(script.contains("color: #64748b"));
    assert!(script.contains("background: #0f766e"));
    assert!(!script.contains("html:not(.light):not([data-theme=\"light\"]) #${claudeCodexProMenuId}.${claudeCodexProMenuFloatingClass}"));
    assert!(script.contains("updateCodexMemoryBadgePosition"));
    assert!(script.contains("openClaudeCodexProModal()"));
    assert!(script.contains("hasRenderableStatusLabel"));
    assert!(script.contains("trigger.querySelector(\"[data-codex-backend-indicator]\")"));
    assert!(script.contains(".claude-codex-pro-window-status-title"));
    assert!(script.contains("trigger.dataset.claudeCodexProTriggerLabel = \"ccp-status-v2\""));
}

#[test]
fn injection_script_uses_compact_pangu_control_deck_layout() {
    let script = assets::injection_script(57321);

    assert!(script.contains("width: min(780px, calc(100vw - 72px))"));
    assert!(script.contains("height: min(540px, calc(100vh - 64px))"));
    assert!(script.contains("grid-template-columns: 148px minmax(0, 1fr)"));
    assert!(script.contains("min-height: 60px"));
    assert!(script.contains("min-height: 58px"));
    assert!(script.contains("overflow-y: auto"));
    assert!(script.contains("@media (max-width: 900px)"));
    assert!(script.contains("@media (max-width: 720px)"));
    assert!(script.contains("height: min(540px, calc(100vh - 32px))"));
}

#[test]
fn injection_script_memory_badge_hides_workspace_label() {
    let script = assets::injection_script(57321);

    assert!(script.contains("codexMemoryState.pendingCandidates ? `<span>待确认 ${codexMemoryState.pendingCandidates}</span>` : \"\""));
    assert!(!script.contains("codexMemoryState.pendingCandidates ? `待确认 ${codexMemoryState.pendingCandidates}` : codexMemoryState.workspace"));
}

#[test]
fn injection_script_modal_hides_user_scripts_management() {
    let script = assets::injection_script(57321);

    assert!(script.contains("data-claude-codex-pro-tab=\"home\""));
    assert!(script.contains("data-claude-codex-pro-tab=\"recommendations\""));
    assert!(script.contains("data-claude-codex-pro-tab=\"support\""));
    assert!(!script.contains("data-claude-codex-pro-tab=\"userScripts\""));
    assert!(!script.contains("data-claude-codex-pro-panel=\"userScripts\""));
    assert!(!script.contains("data-codex-user-scripts-enabled"));
    assert!(!script.contains("data-codex-user-scripts-reload"));
    assert!(!script.contains("data-codex-user-script-list"));
    assert!(!script.contains("data-codex-user-script-key"));
    assert!(!script.contains("if (tab === \"userScripts\") loadUserScripts();"));
    assert!(!script.contains("loadUserScripts();"));
    assert!(!script.contains("\"/user-scripts/list\""));
}

#[test]
fn injection_script_exposes_contact_tab_with_qq_groups_and_wechat_qr() {
    let script = assets::injection_script(57321);

    assert!(script.contains("window.__CLAUDE_CODEX_PRO_CONTACT_WECHAT_QR__"));
    assert!(script.contains("data-claude-codex-pro-tab=\"contact\""));
    assert!(script.contains("data-claude-codex-pro-panel=\"contact\""));
    assert!(script.contains("data-claude-codex-pro-tab=\"home\""));
    assert!(script.contains("data-claude-codex-pro-tab=\"recommendations\""));
    assert!(script.contains("data-claude-codex-pro-tab=\"support\""));
    assert!(script.contains("合作请联系微信"));
    assert!(script.contains("官方QQ群："));
    assert!(script.contains("10061615"));
    assert!(script.contains("1076215359"));
    assert!(script.contains("一键添加"));
    assert!(script.contains("claude-codex-pro-contact-group-number"));
    assert!(script.contains(
        ".claude-codex-pro-control-deck .claude-codex-pro-contact-group-number { color: #e8fff8;"
    ));
    assert!(script.contains("data:image/jpeg;base64,"));
    assert!(script.contains("https://qm.qq.com/cgi-bin/qm/qr?k=uwNon9opx0Arfovyo5qJQQ2jUvlxSpmf&jump_from=webapi&authKey=El8Xwz9ZqefrpE4BhW9xWQsEAUFvptw74MBsRKRJTw5x5QiEPiG0fmdVIf9VuMWg"));
    assert!(script.contains("https://qm.qq.com/cgi-bin/qm/qr?k=cIeUYUFyy0ypTWMqo8CfgRwq8jU_OrXy&jump_from=webapi&authKey=njT7ceHMggvpptkiy9xD6FbBubVGCDof0cnX0adhLgUvi9kKZP4OY51M1xWZBy68"));
    assert!(script.contains("target=\"_blank\" rel=\"noreferrer\""));
    assert!(script.contains("data-claude-codex-pro-setting"));
    assert!(script.contains("data-codex-backend-status"));
    assert!(script.contains("data-codex-backend-repair"));
}

#[test]
fn injection_script_uses_pangu_control_deck_theme() {
    let script = assets::injection_script(57321);

    assert!(script.contains("claude-codex-pro-control-deck"));
    assert!(script.contains("PANGU LOCAL CONTROL DECK"));
    assert!(script.contains("盘古本地控制舱"));
    assert!(script.contains("模型桥接"));
    assert!(script.contains("盘古记忆"));
    assert!(script.contains("模型与插件通道"));
    assert!(script.contains("会话与工作流"));
    assert!(script.contains("本地运维与诊断"));
    assert!(script.contains("z-index: 2147483647"));
    assert!(script.contains("new KeyboardEvent(\"keydown\", { key: \"Escape\""));
    assert!(script.contains("document.activeElement.blur()"));
    assert!(script.contains("@media (max-width: 720px)"));
    assert!(script.contains("@media (max-width: 900px)"));
    assert!(script.contains("@media (prefers-reduced-motion: reduce)"));
    assert!(script.contains(":focus-visible"));
    assert!(script.contains("backdrop-filter: blur(46px) saturate(1.72) contrast(1.08)"));
    assert!(script.contains("grid-template-columns: repeat(2, minmax(0, 1fr))"));
    assert!(script.contains("--ccp-deck-sheen"));
    assert!(!script.contains("rgba(67, 214, 181, .025) 1px"));
    assert!(!script.contains("claude-codex-pro-comic-shell"));
    assert!(!script.contains("POWER PANEL"));

    assert!(script.contains("data-claude-codex-pro-dialog=\"true\""));
    assert!(script.contains("data-claude-codex-pro-tab=\"home\""));
    assert!(script.contains("data-claude-codex-pro-tab=\"recommendations\""));
    assert!(script.contains("data-claude-codex-pro-tab=\"support\""));
    assert!(script.contains("data-claude-codex-pro-setting"));
    assert!(script.contains("data-codex-backend-status"));
    assert!(script.contains("data-codex-backend-repair"));
    assert!(script.contains("data-codex-service-tier-standard"));
    assert!(script.contains("data-claude-codex-pro-discord"));
    assert!(script.contains("data-claude-codex-pro-issue"));
}

#[test]
fn injection_script_modal_close_does_not_toggle_settings() {
    let script = assets::injection_script(57321);

    assert!(script.contains("function claudeCodexProConfiguredSettings()"));
    assert!(script.contains("const settings = claudeCodexProConfiguredSettings();"));
    assert!(script.contains("const configuredSettings = claudeCodexProConfiguredSettings();"));
    assert!(script.contains("button.dataset.enabled = String(!!configuredSettings[key]);"));
    assert!(
        script.contains("setClaudeCodexProSetting(key, !claudeCodexProConfiguredSettings()[key]);")
    );

    let close_handler = script
        .split("closeButton?.addEventListener(\"click\", (event) => {")
        .nth(1)
        .and_then(|tail| tail.split("}, true);").next())
        .expect("close button handler should be present");
    assert!(close_handler.contains("event.preventDefault();"));
    assert!(close_handler.contains("event.stopPropagation();"));
    assert!(close_handler.contains("overlay.remove();"));

    let overlay_close_branch = script
        .split(
            "if (event.target === overlay || target?.closest(\".claude-codex-pro-modal-close\")) {",
        )
        .nth(1)
        .and_then(|tail| tail.split("const tabButton = target?.closest").next())
        .expect("overlay close branch should be before modal action handling");
    assert!(overlay_close_branch.contains("overlay.remove();"));
    assert!(overlay_close_branch.contains("return;"));
    assert!(!overlay_close_branch.contains("setClaudeCodexProSetting("));
    assert!(!overlay_close_branch.contains("setBackendSetting("));
}

#[test]
fn claude_chinese_script_exposes_left_anchored_status_panel() {
    let script = assets::claude_chinese_injection_script();

    assert!(script.contains("ccp-claude-status-pill"));
    assert!(script.contains("ccp-claude-status-panel"));
    assert!(script.contains("findLeftAnchor"));
    assert!(script.contains("findWindowLeftAnchor"));
    assert!(script.contains("document.querySelector(\"aside\")"));
    assert!(script.contains("CCP ' + ccpDisplayVersion"));
    assert!(script.contains("pill.style.left = \"44px\""));
    assert!(!script.contains("data-ccp-frontend-status"));
    assert!(script.contains("data-ccp-backend-status"));
    assert!(script.contains("isEditableUi"));
    assert!(script.contains("[contenteditable]"));
    assert!(script.contains("[role=\"textbox\"]"));
    assert!(script.contains("tauriInvoke(\"backend_version\", {})"));
    assert!(script.contains("scheduleBackendHeartbeat"));
    assert!(script.contains("openStatusPanel"));
    assert!(script.contains("openPluginHub"));
    assert!(script.contains("data-ccp-toggle-chinese"));
    assert!(!script.contains("translateX(-50%)"));
}

#[test]
fn injection_script_fetches_ads_without_bridge() {
    let script = assets::injection_script(57321);

    assert!(script.contains("directFetchClaudeCodexProAds"));
    assert!(script.contains("cacheBustClaudeCodexProAdUrl"));
    assert!(script.contains("Date.now()"));
    assert!(script.contains("window.__CLAUDE_CODEX_PRO_ANNOUNCEMENT__"));
    assert!(script.contains("DamonZS/Claude-Codex-Pro-Tool/main/assets/config/announcement.json"));
    assert!(script.contains("DamonZS/Claude-Codex-Pro-Tool@main/assets/config/announcement.json"));
    assert!(script.contains("ad.buttonLabel"));
    assert!(!script.contains("DamonZS/Claude-Codex-Pro-Tool-Ad-List"));
    assert!(!script.contains("拓扑熵减API｜ClaudeCodexPro官方中转站"));
    assert!(
        !script.contains(
            "claudeCodexProAds = normalizeClaudeCodexProAds(await postJson(\"/ads\", {}));"
        )
    );
}

#[test]
fn injection_script_times_out_backend_bridge_calls_and_falls_back_to_helper() {
    let script = assets::injection_script(57321);

    assert!(script.contains("bridgeWithBackendTimeout"));
    assert!(script.contains("const bridgeStatus = bridgeWithBackendTimeout(path, payload);"));
    assert!(script.contains("const helperStatus = fetchBackendStatusFromHelper(path, payload);"));
    assert!(script.contains("bridgeStatus.then((result) => ({ source: \"bridge\", result }))"));
    assert!(script.contains("helperStatus.then((result) => ({ source: \"helper\", result }))"));
    assert!(script.contains(
        "const second = first.source === \"bridge\" ? await helperStatus : await bridgeStatus;"
    ));
    assert!(script.contains("backend_bridge_timeout"));
    assert!(script.contains("/backend/repair"));
    assert!(script.contains("backend_status_bridge_failed_http_fallback_ok"));
    assert!(script.contains("backend_status_bridge_and_http_failed"));
}

#[test]
fn injection_script_replaces_stale_backend_heartbeat_and_debounces_failures() {
    let script = assets::injection_script(57321);

    assert!(script.contains("clearInterval(window.__claudeCodexProBackendHeartbeat);"));
    assert!(script.contains("window.__claudeCodexProBackendHeartbeatGeneration"));
    assert!(script.contains("claudeCodexProBackendHeartbeatGeneration !== window.__claudeCodexProBackendHeartbeatGeneration"));
    assert!(script.contains("claudeCodexProBackendFailureThreshold = 3"));
    assert!(script.contains("claudeCodexProBackendConsecutiveFailures"));
    assert!(script.contains("document.visibilityState !== \"visible\""));
    assert!(script.contains("window.__claudeCodexProBackendVisibilityHandler"));
}

#[test]
fn injection_script_explains_plugin_patch_is_unneeded_in_relay_mode() {
    let script = assets::injection_script(57321);

    assert!(script.contains("兼容增强模式下无需开启"));
}

#[test]
fn injection_script_menu_exposes_three_independent_plugin_switches() {
    let script = assets::injection_script(57321);

    assert!(script.contains("插件市场解锁"));
    assert!(script.contains("data-claude-codex-pro-setting=\"pluginMarketplaceUnlock\""));
    assert!(script.contains("强制解锁入口"));
    assert!(script.contains("data-claude-codex-pro-setting=\"pluginEntryUnlock\""));
    assert!(script.contains("特殊插件强制安装"));
    assert!(script.contains("data-claude-codex-pro-setting=\"forcePluginInstall\""));
    assert!(script.contains("恢复 1.1.9 的入口解锁方式"));
}

#[test]
fn injection_script_skips_plugin_patch_work_in_relay_mode() {
    let script = assets::injection_script(57321);

    assert!(script.contains("function pluginPatchDisabledInRelayMode()"));
    assert!(script.contains("!claudeCodexProBackendSettingsLoaded"));
    assert!(script.contains("if (pluginPatchDisabledInRelayMode()) return"));
    assert!(script.contains("clearPluginPatchArtifacts()"));
}

#[test]
fn injection_script_defines_version_gated_plugin_unlock_strategy() {
    let script = assets::injection_script(57321);

    assert!(script.contains("codexPluginLegacyEntryUnlockBeforeVersion = \"26.601.2237\""));
    assert!(script.contains("function parseCodexVersionParts(version)"));
    assert!(script.contains("function compareCodexVersions(left, right)"));
    assert!(script.contains("function codexPluginUnlockStrategy()"));
    assert!(script.contains("const comparison = compareCodexVersions(version, codexPluginLegacyEntryUnlockBeforeVersion)"));
    assert!(script.contains("return comparison < 0 ? \"legacy\" : \"modern\""));
}

#[test]
fn injection_script_gates_legacy_and_modern_plugin_unlock_by_codex_version() {
    let script = assets::injection_script(57321);

    assert!(script.contains("const pluginUnlockStrategy = codexPluginUnlockStrategy()"));
    assert!(script.contains("if ((pluginUnlockStrategy === \"legacy\" || pluginUnlockStrategy === \"unknown\") && settings.pluginEntryUnlock)"));
    assert!(script.contains("if ((pluginUnlockStrategy === \"modern\" || pluginUnlockStrategy === \"unknown\") && settings.pluginMarketplaceUnlock)"));
    assert!(script.contains("plugin_unlock_strategy_selected"));
    assert!(script.contains("window.__codexPluginUnlockStrategyLogged"));
}

#[test]
fn injection_script_restores_legacy_plugin_sidebar_entry_unlock() {
    let script = assets::injection_script(57321);

    assert!(script.contains("pluginEntryUnlock: true"));
    assert!(script.contains("pluginEntryUnlock: \"codexAppPluginEntryUnlock\""));
    assert!(script.contains("function reactFiberFrom(element)"));
    assert!(script.contains("function authContextValueFrom(element)"));
    assert!(script.contains("function spoofChatGPTAuthMethod(element)"));
    assert!(script.contains("auth.setAuthMethod(\"chatgpt\")"));
    assert!(script.contains("function pluginEntryButton()"));
    assert!(script.contains("function enablePluginEntry()"));
    assert!(script.contains("if (!claudeCodexProSettings().pluginEntryUnlock) return"));
    assert!(script.contains("pluginButton.addEventListener(\"click\", () => {"));
    assert!(script.contains("spoofChatGPTAuthMethod(pluginButton);"));
    assert!(script.contains("插件 - 已解锁"));
    assert!(script.contains("Plugins - Unlocked"));
}

#[test]
fn injection_script_keeps_plugin_marketplace_unlock_separate_from_entry_unlock() {
    let script = assets::injection_script(57321);

    assert!(script.contains("pluginMarketplaceUnlock: true"));
    assert!(script.contains("pluginMarketplaceUnlock: \"codexAppPluginMarketplaceUnlock\""));
    assert!(script.contains("if (!claudeCodexProSettings().pluginMarketplaceUnlock) return"));
    assert!(script.contains("installPluginBuildFlavorFilterPatch"));
    assert!(script.contains("installPluginMarketplaceRequestPatch"));
}

#[test]
fn codex_multica_uses_current_page_host_with_modern_app_initial_fallback() {
    let script = assets::injection_script(57321);
    let host = source_between(
        &script,
        "const codexPageHostAllowedMethods",
        "async function codexSettingStorageModule",
    );

    for method in [
        "skills/list",
        "thread/start",
        "thread/read",
        "thread/fork",
        "turn/start",
        "turn/interrupt",
    ] {
        assert!(host.contains(&format!("\"{method}\"")));
    }
    // Prefer the legacy signal/store export when the current Codex build still
    // exposes it, then recover the page-owned client from the modern app root.
    assert!(host.contains("loadCodexAppModule(\"app-server-manager-signals-\")"));
    assert!(host.contains("loadCodexAppModule(\"app-initial-\")"));
    assert!(host.contains("function codexPageHostReactRootFiber()"));
    assert!(host.contains("__reactContainer$"));
    assert!(host.contains("value?.current || value?._internalRoot?.current"));
    assert!(host.contains("window.__codexRoot?._internalRoot?.current"));
    assert!(host.contains("state?.appScope"));
    assert!(host.contains("function codexPageHostAppScopeValid(appScope)"));
    assert!(host.contains("typeof appScope.get === \"function\""));
    assert!(host.contains("appScope.queryClient"));
    assert!(host.contains("return hasScopeGetter || hasScopeNode || hasQueryClient;"));
    assert!(host.contains("module?.FRt"));
    assert!(host.contains("function codexPageHostIdFromActiveThread()"));
    assert!(host.contains("data-app-action-sidebar-thread-host-id"));
    assert!(host.contains("module.FRt(appScope, hostId)"));
    assert!(host.contains("client.sendRequest(\"skills/list\", {})"));
    assert!(host.contains("skills.error"));
    assert!(host.contains("capabilities: []"));
    assert!(host.contains("pageHostProbe: { skillsList: true, nativeTaskHost: false }"));
    assert!(host.contains("codexPageHostInitializeResponse = selected.initializeResponse"));
    assert!(host.contains("normalizedMethod === \"initialize\" && selected.initializeResponse"));
    assert!(host.contains("client.sendRequest(normalizedMethod, params)"));
    assert!(host.contains("window.__claudeCodexProCodexPageHostRequest"));
    assert!(host.contains("return await client.sendRequest(normalizedMethod, params);"));
    assert!(!host.contains("multicaWorkspaceLoadCurrentRoute(true, 2000"));
    let cleanup = source_between(
        host,
        "function cleanupCodexPageHostRequest()",
        "// Called only through CCP's CDP bridge",
    );
    assert!(cleanup.contains("codexPageHostClient = null"));
    assert!(cleanup.contains("codexPageHostClientPromise = null"));
    assert!(cleanup.contains("codexPageHostInitializeResponse = null"));
    for forbidden in [
        "register_runtime",
        "register_managed_codex_runtime",
        "codex.exe app-server",
        "child_process",
        "spawn(",
    ] {
        assert!(
            !host.contains(forbidden),
            "page host adapter must not create or register a runtime: {forbidden}"
        );
    }
}

#[test]
fn codex_multica_workspace_anchors_after_plugin_before_projects() {
    let script = assets::injection_script(57321);

    assert!(script.contains("nav[role=\"navigation\"] button.sidebar-item"));
    assert!(script.contains("aside.app-shell-left-panel button.sidebar-item"));
    assert!(script.contains("button.h-token-nav-row.w-full"));
    assert!(script.contains(
        "pluginAnchorButton: 'nav[role=\"navigation\"] button, [role=\"navigation\"] button, aside.app-shell-left-panel button, aside button'"
    ));
    assert!(
        script
            .contains("pluginAnchorRegion: 'nav[role=\"navigation\"], [role=\"navigation\"], aside.app-shell-left-panel, aside'")
    );
    assert!(script.contains("M8.25031 1.46094"));
    assert!(script.contains("M7.94562 14.0277"));
    assert!(script.contains("/^(插件|Plugins)"));
    assert!(
        script
            .contains("pluginButton.parentElement.insertBefore(entry, pluginButton.nextSibling);")
    );
    assert!(script.contains("entry.previousElementSibling !== pluginButton"));
    assert!(script.contains("function multicaPluginAnchorMutationNode(node)"));
    assert!(script.contains("data-ccp-multica-nav=\"true\""));
    assert!(script.contains("entry.setAttribute(\"aria-label\", \"我的任务\")"));
    assert!(script.contains("entry.title = \"我的任务\""));
    assert!(script.contains("label.textContent = \"我的任务\""));
    assert!(script.contains("label.dataset.ccpMulticaNavLabel = \"true\""));
    assert!(script.contains("entry.querySelector?.('[data-ccp-multica-nav-label=\"true\"]')"));
}

#[test]
fn codex_multica_workspace_is_shadow_dom_singleton_without_product_shell() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );

    assert!(script.contains("window.__claudeCodexProMulticaWorkspaceCleanup?.();"));
    assert!(
        script
            .contains("window.__claudeCodexProMulticaWorkspaceCleanup = cleanupMulticaWorkspace;")
    );
    assert!(script.contains("host.attachShadow({ mode: \"open\" })"));
    assert!(script.contains("host.id = \"ccp-multica-workspace-root\""));
    assert!(script.contains("entry.dataset.ccpMulticaNav = \"true\""));
    assert!(script.contains("multicaWorkspaceState.entry?.remove?.();"));
    assert!(script.contains("multicaWorkspaceState.host?.remove?.();"));
    assert!(script.contains("multicaWorkspaceRestoreMain();"));
    assert!(script.contains("#ccp-multica-workspace-root"));
    assert!(script.contains("[data-ccp-multica-nav=\"true\"]"));
    assert!(workspace.contains("multicaWorkspaceState.root = { shell, content }"));
    for obsolete_shell in [
        "ccp-multica-header",
        "ccp-multica-status",
        "ccp-multica-close",
        "ccp-multica-nav-list",
        "Local Multica Workspace",
        "multicaWorkspaceRenderTabs",
        "root?.workspaceName",
    ] {
        assert!(
            !workspace.contains(obsolete_shell),
            "obsolete Multica product shell marker: {obsolete_shell}"
        );
    }
    assert!(!workspace.contains(", \"Multica 工作区\")"));
    assert!(!workspace.contains("aria-label\", \"关闭工作区\""));
}

#[test]
fn codex_multica_workspace_hide_preserves_background_work_until_full_cleanup() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let hide = source_between(
        workspace,
        "function multicaWorkspaceHide()",
        "async function multicaWorkspaceBackgroundSync()",
    );
    let background = source_between(
        workspace,
        "function multicaWorkspaceStartBackgroundSync()",
        "function multicaWorkspaceScheduleAnchorRetry()",
    );
    let ensure = source_between(
        workspace,
        "function ensureMulticaWorkspaceRuntime()",
        "function cleanupMulticaWorkspace()",
    );
    let cleanup = source_between(
        workspace,
        "function cleanupMulticaWorkspace()",
        "window.__claudeCodexProMulticaWorkspaceCleanup = cleanupMulticaWorkspace;",
    );

    assert!(script.contains("multicaWorkspaceEnabled: true"));
    assert!(script.contains("multicaWorkspaceEnabled: \"multicaWorkspaceEnabled\""));
    assert!(workspace.contains("function multicaWorkspaceFeatureEnabled()"));
    assert!(workspace.contains("return true;"));
    assert!(ensure.contains("if (!multicaWorkspaceFeatureEnabled())"));
    assert!(
        ensure
            .contains("window.__claudeCodexProMulticaWorkspaceCleanup = cleanupMulticaWorkspace;")
    );
    assert!(ensure.contains("cleanupMulticaWorkspace();"));
    assert!(hide.contains("multicaWorkspaceState.opened = false"));
    assert!(hide.contains("multicaWorkspaceRestoreMain()"));
    assert!(hide.contains("multicaWorkspaceState.host.style.display = \"none\""));
    assert!(hide.contains("multicaWorkspaceState.entry.setAttribute(\"aria-current\", \"false\")"));
    for forbidden in [
        "multicaWorkspaceCancelQuery",
        "multicaWorkspaceCancelBootstrap",
        "activeRequests.forEach",
        "activeRequests.clear",
        "querySeq += 1",
        "bootstrapSeq += 1",
        "executionSeq += 1",
        "loading.clear",
        "backgroundTimer",
        "entry?.remove",
        "host?.remove",
    ] {
        assert!(
            !hide.contains(forbidden),
            "hiding the board must preserve background work: {forbidden}"
        );
    }
    assert!(cleanup.contains("clearTimeout(multicaWorkspaceState.anchorTimer)"));
    assert!(cleanup.contains("multicaWorkspaceCancelQuery();"));
    assert!(cleanup.contains("multicaWorkspaceCancelBootstrap();"));
    assert!(cleanup.contains("multicaWorkspaceState.activeRequests.forEach"));
    assert!(cleanup.contains("multicaWorkspaceState.activeRequests.clear()"));
    assert!(cleanup.contains("multicaWorkspaceStopBackgroundSync();"));
    assert!(background.contains("multicaWorkspaceState.backgroundTimer ="));
    assert!(background.contains("clearTimeout") || background.contains("clearInterval"));
    assert!(cleanup.contains("multicaWorkspaceState.entry?.remove?.();"));
    assert!(cleanup.contains("multicaWorkspaceState.host?.remove?.();"));
    assert!(workspace.contains("multicaWorkspaceBackgroundIntervalMs"));
    assert!(workspace.contains("multicaWorkspaceState.backgroundTimer ="));

    for forbidden in [
        "/multica/runtime/",
        "/multica/managed/",
        "/supplier/",
        "/suppliers/",
        "/route/",
        "/proxy/",
        "helperBase",
        "start_managed_runtime",
        "register_runtime",
    ] {
        assert!(
            !workspace.contains(forbidden),
            "workspace feature flag crossed runtime or supplier/proxy boundary: {forbidden}"
        );
    }
}

#[test]
fn codex_multica_workspace_settings_persist_and_apply_the_ui_only_toggle() {
    let script = assets::injection_script(57321);
    let workspace = script
        .split("// The workspace is deliberately kept in this injection file")
        .nth(1)
        .and_then(|rest| rest.split("function labelUnlockedPluginEntry").next())
        .expect("Multica workspace segment");
    let settings = workspace
        .split("function multicaWorkspaceRenderSettings(content)")
        .nth(1)
        .and_then(|rest| {
            rest.split("function multicaWorkspaceRenderContent()")
                .next()
        })
        .expect("Multica workspace settings segment");

    assert!(settings.contains("启用本地 Multica 工作区"));
    assert!(settings.contains("setBackendSetting(\"multicaWorkspaceEnabled\", nextValue)"));
    assert!(script.contains("settings?.[key] !== value"));
    assert!(settings.contains("ensureMulticaWorkspaceRuntime();"));
    assert!(settings.contains("cleanupMulticaWorkspace();"));
    assert!(workspace.contains("if (module.key === \"settings\")"));
    assert!(workspace.contains("multicaWorkspaceRenderSettings(content);"));

    for forbidden in [
        "/supplier/",
        "/suppliers/",
        "/route/",
        "/proxy/",
        "/multica/runtime/",
        "/multica/managed/",
        "register_managed_codex_runtime",
        "register_runtime",
        "codex.exe app-server",
    ] {
        assert!(
            !settings.contains(forbidden),
            "workspace settings crossed the UI-only boundary: {forbidden}"
        );
    }
}

#[test]
fn codex_multica_workspace_renders_my_issues_as_direct_seven_column_board() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );

    let mut previous = 0;
    for route in [
        "my-issues",
        "issues",
        "projects",
        "autopilots",
        "agents",
        "squads",
        "usage",
        "runtimes",
        "skills",
        "settings",
    ] {
        let needle = format!("key: \"{route}\"");
        let position = workspace
            .find(&needle)
            .unwrap_or_else(|| panic!("missing {route}"));
        assert!(position >= previous, "module order changed at {route}");
        previous = position;
    }
    previous = 0;
    for status in [
        "backlog",
        "todo",
        "in_progress",
        "in_review",
        "done",
        "blocked",
        "cancelled",
    ] {
        let needle = format!("key: \"{status}\"");
        let position = workspace
            .find(&needle)
            .unwrap_or_else(|| panic!("missing board status {status}"));
        assert!(
            position >= previous,
            "board column order changed at {status}"
        );
        previous = position;
    }
    let render = source_between(
        workspace,
        "function multicaWorkspaceRenderContent()",
        "async function multicaWorkspaceLoadBootstrap",
    );
    assert!(render.contains("content.dataset.route = module.key"));
    assert!(render.contains("if (module.key === \"my-issues\")"));
    let board_call = render
        .find("multicaWorkspaceRenderIssueBoard(content, module)")
        .expect("my-issues direct board render");
    let generic_header = render
        .find("const header = multicaWorkspaceEl")
        .expect("generic module header");
    assert!(board_call < generic_header);
    assert!(render[board_call..generic_header].contains("return;"));
    assert!(render.contains("multicaWorkspaceAppendModuleMenu(header)"));
    let board = source_between(
        workspace,
        "function multicaWorkspaceRenderIssueBoard(content, module)",
        "function multicaWorkspaceAppendSkillItem",
    );
    assert!(board.contains("multicaWorkspaceAppendModuleMenu(heading)"));
    assert!(board.contains("multicaWorkspaceRenderNativeInventory(page)"));
    assert!(board.contains("multicaWorkspaceBoardColumns.forEach"));
    assert!(board.contains("lane.dataset.multicaBoardStatus = column.key"));
    let dependency_loader = source_between(
        workspace,
        "async function multicaWorkspaceLoadAgentFilterDependencies",
        "function multicaWorkspaceRefreshBoardSource",
    );
    assert!(dependency_loader.contains("for (const dependency of ["));
    assert!(dependency_loader.contains("{ key: \"agents\", label: \"智能体\" }"));
    assert!(dependency_loader.contains("{ key: \"squads\", label: \"小队\" }"));
    assert!(dependency_loader.contains("{ key: \"issues\", label: \"任务\" }"));
    assert!(
        dependency_loader.contains(
            "await multicaWorkspaceQuery(moduleForMulticaWorkspace(dependency.key), force)"
        )
    );
    assert!(!dependency_loader.contains("Promise.all"));
    let inventory = source_between(
        workspace,
        "function multicaWorkspaceNativeSessionRows()",
        "function multicaWorkspaceRenderIssueBoard",
    );
    assert!(inventory.contains("data-app-action-sidebar-thread-id"));
    assert!(inventory.contains(".slice(0, 100)"));
    assert!(inventory.contains("row.click()"));
    assert!(inventory.contains("暂无已绑定的本地智能体"));
    assert!(inventory.contains("已绑定原生执行"));
    let refresh = source_between(
        workspace,
        "function multicaWorkspaceRefreshBoardSource",
        "function multicaWorkspaceAppendBoardCard",
    );
    assert!(refresh.contains("multicaWorkspaceState.issueFilter === \"agents\""));
    assert!(board.contains("正在读取智能体和小队"));
    assert!(board.contains("读取智能体和小队失败"));
    assert!(workspace.contains("grid-template-columns: repeat(7, 280px)"));
    assert!(workspace.contains("overflow-x: auto"));
    assert!(
        workspace
            .contains("multicaWorkspaceRequest(\"/multica/workspace/bootstrap\", {}, timeoutMs)")
    );
    assert!(workspace.contains("multicaWorkspaceRequest(\"/multica/workspace/query\""));
    assert!(workspace.contains("postJson(\"/multica/skills/review\""));
    assert!(workspace.contains("postJson(\"/multica/skills/bindings\", {}"));
    assert!(workspace.contains("postJson(\"/multica/skills/bind\", payload"));
    assert!(workspace.contains("postJson(\"/multica/skills/unbind\", payload"));
    assert!(workspace.contains("保存绑定"));
    assert!(workspace.contains("解绑"));
    assert!(workspace.contains("审查并信任"));
    assert!(workspace.contains("撤销信任"));
    assert!(workspace.contains("assignee_type"));
    assert!(workspace.contains("parent_issue_id"));
    assert!(workspace.contains("lead_type"));
    assert!(workspace.contains("execution_mode"));
    assert!(workspace.contains("concurrency_limit"));
    assert!(workspace.contains("permission_mode"));
    assert!(workspace.contains("invocation_targets"));
    assert!(workspace.contains("max_concurrent_tasks"));
    assert!(workspace.contains("thinking_level"));
    assert!(workspace.contains("service_tier"));
    assert!(workspace.contains("流程元数据（JSON）"));
    assert!(workspace.contains("自定义属性（JSON）"));
    assert!(workspace.contains("labels"));
    assert!(workspace.contains("reactions"));
    assert!(workspace.contains("last_activity_at"));
    assert!(workspace.contains("必须是有效 JSON"));
    assert!(workspace.contains("multicaWorkspaceNormalizeEditableEntity"));
    assert!(workspace.contains("postJson(\"/manager/open\", {})"));
    assert!(workspace.contains("textContent"));
    assert!(!workspace.contains("innerHTML"));
    assert!(!workspace.contains("createElement(\"iframe\")"));
    assert!(!workspace.contains("helperBase"));
    assert!(!workspace.contains("fetch("));
    assert!(!workspace.contains("localStorage"));
    assert!(!workspace.contains("sessionStorage"));
    assert!(!workspace.contains("history."));
    assert!(!workspace.contains("location."));
}

#[test]
fn codex_multica_workspace_keeps_native_surface_until_board_is_ready() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let open = source_between(
        workspace,
        "async function multicaWorkspaceOpen()",
        "function multicaWorkspaceHide()",
    );
    let fail_open = source_between(
        workspace,
        "function multicaWorkspaceFailOpen(message = \"\")",
        "function multicaWorkspaceFeatureEnabled()",
    );
    let board = source_between(
        workspace,
        "function multicaWorkspaceRenderIssueBoard(content, module)",
        "function multicaWorkspaceAppendSkillItem",
    );

    let preflight = open
        .find("const ready = await multicaWorkspaceLoadCurrentRoute(true, 15000, openSequence);")
        .expect("open preflight must await bootstrap and my-issues");
    let takeover = open
        .find("multicaWorkspaceState.opened = true;")
        .expect("board takeover assignment");
    assert!(
        preflight < takeover,
        "native main must stay visible during preflight"
    );
    assert!(open.contains("!multicaWorkspaceState.opening"));
    assert!(open.contains("本地任务暂不可用，请点击重试"));

    let opened_branch = fail_open
        .split("if (multicaWorkspaceState.opened) {")
        .nth(1)
        .and_then(|rest| rest.split("return;").next())
        .expect("opened fail-open branch");
    assert!(opened_branch.contains("multicaWorkspaceState.opening = false"));
    assert!(!opened_branch.contains("multicaWorkspaceHide()"));

    assert!(board.contains("notice.dataset.state = \"warning\""));
    assert!(board.contains("当前 Codex 执行能力不可用，本地任务仍可查看和编辑"));
    assert!(board.contains("assignedFilterEmpty"));
    assert!(board.contains("当前没有分配给本地用户的任务"));
    assert!(board.contains("查看全部任务"));
    assert!(board.contains("multicaWorkspaceState.issueFilter = \"all\""));
}

#[test]
fn codex_multica_unavailable_entry_is_visible_and_retryable() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let entry = source_between(
        workspace,
        "function multicaWorkspaceEnsureEntryAvailabilityBadge(entry)",
        "function multicaWorkspaceSetStatus",
    );
    let availability = source_between(
        workspace,
        "function multicaWorkspaceSetEntryAvailability(message = \"\")",
        "function multicaWorkspaceBridgeUnavailable",
    );

    assert!(entry.contains("data-ccp-multica-nav-availability=\"true\""));
    assert!(entry.contains("multicaWorkspaceEnsureEntryAvailabilityBadge(entry);"));
    assert!(availability.contains("badge.textContent = \"未连接\""));
    assert!(availability.contains("badge.style.display = \"inline-flex\""));
    assert!(availability.contains("点击重试"));
    assert!(
        availability.contains("entry.setAttribute(\"aria-label\", \"我的任务，未连接，点击重试\")")
    );
    assert!(availability.contains("badge.style.display = \"none\""));
    assert!(entry.contains("multicaWorkspaceOpen();"));
}

#[test]
fn codex_multica_inventory_only_skills_use_camel_case_runtime_dto_fields() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let skill_gate = source_between(
        workspace,
        "function multicaWorkspaceSkillsInventoryReadOnly()",
        "function multicaWorkspaceSkillExecutionSupported",
    );

    assert!(
        skill_gate
            .contains("runtime?.skillsInventorySupported ?? runtime?.skills_inventory_supported")
    );
    assert!(skill_gate.contains("runtime?.skillsSupported ?? runtime?.skills_supported"));
    assert!(skill_gate.contains("inventorySupported === true && executionSupported !== true"));
}

#[test]
fn codex_multica_native_agent_inventory_requires_codex_thread_binding() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let inventory = source_between(
        workspace,
        "function multicaWorkspaceRenderNativeInventory(parent)",
        "function multicaWorkspaceRenderIssueBoard",
    );

    assert!(inventory.contains("codexThreadId"));
    assert!(inventory.contains("codex_thread_id"));
    assert!(inventory.contains("const boundAgents = agents.filter"));
    assert!(inventory.contains("boundAgents.length === 0"));
    assert!(!inventory.contains("未绑定"));
}

#[test]
fn codex_multica_autopilot_toggle_uses_upstream_status_field() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let item = source_between(
        workspace,
        "function multicaWorkspaceAppendEntityItem(parent, item, module)",
        "function multicaWorkspaceIssueStatus",
    );

    assert!(item.contains("resource === \"autopilots\""));
    assert!(item.contains("{ status: paused ? \"active\" : \"paused\" }"));
    assert!(item.contains("status === \"archived\""));
    assert!(!item.contains("resource === \"agents\" || resource === \"autopilots\""));
}

#[test]
fn codex_multica_autopilot_manual_trigger_records_unsupported_host_run() {
    let item = assets::injection_script(57321);
    assert!(item.contains("multicaWorkspaceTriggerAutopilot"));
    assert!(item.contains("reason_code: \"codex_host_execution_unavailable\""));
    assert!(item.contains("status: \"unsupported\""));
    assert!(item.contains("立即触发"));
    assert!(item.contains("multicaWorkspaceCreateAutopilotTrigger"));
    assert!(item.contains("multicaWorkspaceDeleteAutopilotTrigger"));
    assert!(item.contains("multicaWorkspaceUpdateAutopilotTrigger"));
    assert!(item.contains("编辑触发器"));
    assert!(item.contains("multicaWorkspaceToggleAutopilotCollaborator"));
    assert!(item.contains("管理协作者"));
    assert!(item.contains("运行历史"));
}

#[test]
fn codex_multica_comments_expose_resolve_and_unresolve_actions() {
    let script = assets::injection_script(57321);
    assert!(script.contains("标记已解决"));
    assert!(script.contains("取消解决"));
    assert!(script.contains("resolved_at: null"));
    assert!(script.contains("resolved_by_type: \"member\""));
}

#[test]
fn codex_multica_issue_cards_expose_subscription_toggle() {
    let script = assets::injection_script(57321);
    assert!(script.contains("multicaWorkspaceToggleIssueSubscription"));
    assert!(script.contains("multicaWorkspaceUnsubscribeIssueSubtree"));
    assert!(script.contains("取消树订阅"));
    assert!(script.contains("取消订阅"));
    assert!(script.contains("订阅"));
}

#[test]
fn codex_multica_issue_and_comment_cards_expose_reaction_toggle() {
    let script = assets::injection_script(57321);
    assert!(script.contains("multicaWorkspaceToggleReaction"));
    assert!(script.contains("multicaWorkspaceToggleLabel"));
    assert!(script.contains("label_ids"));
    assert!(script.contains("targetType === \"issue\" ? \"issue_id\" : \"comment_id\""));
    assert!(script.contains("actor_type: \"member\""));
}

#[test]
fn codex_multica_agent_assignment_dispatches_native_subagent() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let action = source_between(
        workspace,
        "async function multicaWorkspaceRunExecutionAction",
        "function multicaWorkspaceAppendExecutionAttempts",
    );
    assert!(action.contains("assigneeKind === \"agent\""));
    assert!(action.contains(
        "multicaWorkspaceObjectValue(issue, \"assignee_type\", \"assigneeKind\", \"assignee_kind\")"
    ));
    assert!(action.contains("multicaWorkspaceObjectValue(issue, \"assignee_id\", \"assigneeId\")"));
    assert!(action.contains("payload.executionKind = \"subagent\""));
    assert!(action.contains("payload.agentId = assigneeId"));
    assert!(action.contains("payload.parentThreadId = parentThreadId"));
}

#[test]
fn codex_multica_native_navigation_hides_without_intercepting_codex_clicks() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let handler = source_between(
        workspace,
        "multicaWorkspaceState.navHandler = (event) => {",
        "document.addEventListener(\"click\", multicaWorkspaceState.navHandler, true)",
    );

    for selector in [
        "[data-app-action-sidebar-thread-id]",
        "[data-app-action-sidebar-project-row]",
        "nav a[href]",
        "aside a[href]",
        "[role=\"link\"]",
        "[role=\"button\"]",
    ] {
        assert!(
            workspace.contains(selector),
            "missing native navigation selector: {selector}"
        );
    }
    assert!(handler.contains("multicaWorkspaceHide()"));
    assert!(!handler.contains("preventDefault"));
    assert!(!handler.contains("stopPropagation"));
    assert!(!handler.contains("cleanupMulticaWorkspace"));
}

#[test]
fn codex_multica_open_execution_activates_real_thread_row_before_hiding() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let activation = source_between(
        workspace,
        "function multicaWorkspaceThreadIdMatches(value, threadId)",
        "async function multicaWorkspaceRunExecutionAction",
    );
    let action = source_between(
        workspace,
        "async function multicaWorkspaceRunExecutionAction",
        "function multicaWorkspaceAppendExecutionAttempts",
    );

    assert!(activation.contains("candidate.endsWith(`:${expected}`)"));
    assert!(activation.contains("[data-app-action-sidebar-thread-id]"));
    assert!(activation.contains("clickTarget.click?.()"));
    assert!(
        activation
            .contains("[data-app-action-sidebar-thread-active=\"true\"], [aria-current=\"page\"]")
    );
    assert!(activation.contains("multicaWorkspaceNativeThreadIsActive(current) &&"));
    assert!(activation.contains("multicaWorkspaceThreadIdMatches(activeThreadId, threadId)"));
    assert_eq!(activation.matches("multicaWorkspaceHide()").count(), 1);
    assert!(activation.contains("throw new Error(\"Codex 未激活目标对话\")"));
    for field in [
        "result.handle",
        "threadId",
        "thread_id",
        "codexThreadId",
        "codex_thread_id",
    ] {
        assert!(action.contains(field), "open result must extract {field}");
    }
    assert!(action.contains("await multicaWorkspaceActivateNativeThread(threadId)"));
    assert!(
        action.contains("multicaWorkspaceState.executionNotice = { state: \"error\", message }")
    );
    assert!(action.contains("if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent()"));
    assert!(!action.contains("multicaWorkspaceHide()"));
}

#[test]
fn injection_script_gates_memory_auto_suggest_by_dom_injection_setting() {
    let script = assets::injection_script(57321);

    assert!(script.contains("function codexMemoryMaybeSuggestCandidate"));
    assert!(
        script.contains(
            "if (!settings.memoryAssistEnabled || !settings.memoryAssistInjectEnabled || !settings.memoryAssistAutoSuggestEnabled) return"
        ),
        "auto-suggest must stop when DOM memory injection is disabled"
    );
}

#[test]
fn injection_script_deduplicates_memory_capture_requests() {
    let script = assets::injection_script(57321);
    let start = script
        .find("const codexMemoryCaptureTtlMs")
        .expect("memory capture deduplication state exists");
    let end = script[start..]
        .find("function codexMemorySetMessage")
        .map(|offset| start + offset)
        .expect("memory capture deduplication precedes memory UI helpers");
    let capture_section = &script[start..end];

    assert!(capture_section.contains("const codexMemoryCaptureTtlMs = 30 * 60 * 1000;"));
    assert!(capture_section.contains("const codexMemoryCaptureMaxEntries = 128;"));
    assert!(
        capture_section
            .contains("while (codexMemoryCaptureRecent.size > codexMemoryCaptureMaxEntries)")
    );
    assert!(capture_section.contains("workspace: payload.workspace"));
    assert!(capture_section.contains("text: payload.text"));
    assert!(capture_section.contains("candidateTriggered: payload.candidateTriggered"));
    assert!(capture_section.contains("candidateReason: payload.candidateReason"));
    assert!(capture_section.contains("skipReason: payload.skipReason"));
    assert!(
        capture_section.contains("const inFlight = codexMemoryCaptureInFlight.get(fingerprint);")
    );
    assert!(capture_section.contains("if (inFlight) return inFlight;"));
    assert!(capture_section.contains("codexMemoryCaptureInFlight.set(fingerprint, request);"));
    assert!(capture_section.contains("codexMemoryRememberCapture(fingerprint, result);"));
    assert!(capture_section.contains("codexMemoryCaptureRecent.delete(fingerprint);"));
    assert!(capture_section.contains("codexMemoryCaptureInFlight.delete(fingerprint);"));
}

#[test]
fn injection_script_global_enhancement_toggle_does_not_hide_enabled_child_features() {
    let script = assets::injection_script(57321);

    assert!(script.contains("function hasAnyCodexFrontendEnhancementEnabled(settings)"));
    assert!(script.contains("\"multicaWorkspaceEnabled\","));
    assert!(script.contains(
        "claudeCodexProBackendSettings.enhancementsEnabled === false && !hasAnyCodexFrontendEnhancementEnabled(settings)"
    ));
}

#[test]
fn injection_script_refreshes_memory_after_backend_settings_load() {
    let script = assets::injection_script(57321);

    assert!(script.contains("codexMemoryUpdateBadge();"));
    assert!(script.contains("void codexMemoryLoadSession(true);"));
    assert!(script.contains("void codexMemoryMaybeSuggestCandidate();"));
}

#[test]
fn injection_script_replaces_stale_memory_heartbeat_on_reinject() {
    let script = assets::injection_script(57321);

    assert!(script.contains("clearInterval(window.__claudeCodexProMemoryHeartbeatTimer);"));
    assert!(script.contains("window.__claudeCodexProMemoryHeartbeatTimer = window.setInterval"));
}

#[test]
fn injection_script_memory_auto_suggest_recognizes_project_requirements() {
    let script = assets::injection_script(57321);

    assert!(script.contains("function codexMemoryLooksLearnableText"));
    assert!(script.contains("project requirement phrase"));
    assert!(script.contains("ui workflow requirement"));
    assert!(script.contains("workflow requirement"));
    assert!(script.contains("function codexMemoryLooksLikeChatter"));
    assert!(script.contains("function codexMemoryLooksLikeTitleOnly"));
    assert!(script.contains("codexMemoryLooksLikeChatter(text)"));
    assert!(script.contains("codexMemoryLooksLikeTitleOnly(text)"));
    assert!(script.contains("async function codexMemoryRecordCapture"));
    assert!(script.contains("postJson(\"/memory/capture\""));
    assert!(script.contains("skipReason: \"not_learnable\""));
    assert!(script.contains("skipReason: \"duplicate_recent_memory\""));
    assert!(script.contains("skipReason: \"learn_failed\""));
    assert!(script.contains("postJson(\"/memory/learn\""));
    assert!(script.contains("memory_auto_learned"));
    assert!(script.contains("database_failed"));
    assert!(script.contains("await codexMemoryLoadSession(true);"));
    assert!(
        !script.contains("Math.max(1, Number(codexMemoryState.pendingCandidates || 0) + 1)"),
        "candidate count must be synchronized from backend state instead of optimistic +1"
    );
}

#[test]
fn injection_script_auto_suggest_only_reads_explicit_user_turns() {
    let script = assets::injection_script(57321);
    let start = script
        .find("function codexMemoryLatestUserText")
        .expect("memory latest-user function exists");
    let end = script[start..]
        .find("function codexMemorySuggestionFromText")
        .map(|offset| start + offset)
        .expect("memory suggestion function follows latest-user function");
    let function_body = &script[start..end];

    assert!(function_body.contains("codexMemoryConversationMessages(\"user\")"));
    assert!(script.contains("[data-message-author-role=\"user\"]"));
    assert!(script.contains("function codexMemoryUserMessageCandidates"));
    assert!(script.contains("nodeOrAncestorLooksLikeCodexUserBubble(child)"));
    assert!(script.contains(".group.flex.w-full.flex-col.items-end.justify-end.gap-1"));
    assert!(
        !function_body.contains("[data-testid=\"conversation-turn\"]"),
        "auto-suggest fallback must not read generic conversation turns"
    );
    assert!(
        !function_body.contains("main [class*=\"user\"]"),
        "auto-suggest fallback must not infer user role from class substring"
    );
}

#[test]
fn injection_script_memory_session_uses_conversation_root_not_sidebar_titles() {
    let script = assets::injection_script(57321);

    assert!(script.contains("function codexMemoryConversationRoot"));
    assert!(script.contains("function codexMemoryConversationMessages"));
    assert!(script.contains("codexMemoryNodeIsInsideConversation"));
    assert!(script.contains("[data-app-action-sidebar-thread-id]"));
    assert!(script.contains("[role=\"navigation\"]"));
    assert!(!script.contains("等待真实对话消息后写入盘古记忆"));
    assert!(!script.contains("document.querySelectorAll('[data-message-author-role=\"user\"], [data-testid=\"conversation-turn\"], main .prose')"));
    let workspace_start = script
        .find("function codexMemoryWorkspace()")
        .expect("memory workspace function exists");
    let workspace_end = script[workspace_start..]
        .find("function codexMemoryWorkspaceIsPathFallback")
        .map(|offset| workspace_start + offset)
        .expect("memory workspace fallback helper follows workspace function");
    let workspace_body = &script[workspace_start..workspace_end];
    assert!(workspace_body.contains("codex:thread:"));
    assert!(workspace_body.contains("codex:path:"));
    assert!(workspace_body.contains("rememberCodexMemoryProjectContext(project)"));
    assert!(workspace_body.contains("readCodexMemoryProjectContext()"));
    assert!(script.contains("/memory/resolve-workspace"));
    assert!(
        !workspace_body.contains("document.title"),
        "memory workspace must not use conversation titles such as codex:你好"
    );
    assert!(
        !workspace_body.contains("[data-thread-title]"),
        "memory workspace must not use sidebar/thread title text"
    );
}

#[test]
fn injection_script_unlocks_nested_disabled_plugin_install_buttons() {
    let script = assets::injection_script(57321);

    assert!(script.contains("button[aria-disabled=\"true\"]"));
    assert!(script.contains("[role=\"button\"][data-disabled]"));
    assert!(script.contains("installButtonUnlockNodes"));
    assert!(script.contains("patchReactDisabledProps"));
    assert!(script.contains("props[\"data-disabled\"] = undefined"));
    assert!(script.contains("button.querySelectorAll?.(\"button, [role='button'], [disabled], [aria-disabled], [data-disabled]"));
    assert!(script.contains("button.dataset.codexForceInstallUnlocked"));
}

#[test]
fn injection_script_keeps_bundled_marketplace_name_for_default_filter() {
    let script = assets::injection_script(57321);

    assert!(script.contains("codexPluginMarketplaceUnlockVersion = \"13\""));
    assert!(script.contains("if (name === \"openai-bundled\") return \"\""));
    assert!(
        !script.contains(
            "if (name === \"openai-bundled\") return \"claude-codex-pro-openai-bundled\""
        )
    );
    assert!(script.contains("if (name === \"openai-bundled\" || name === \"claude-codex-pro-openai-bundled\") return \"OpenAI插件1(Claude Codex Pro)\""));
}

#[test]
fn injection_script_does_not_bypass_plugin_marketplace_search_filters() {
    let script = assets::injection_script(57321);

    assert!(script.contains("codexPluginMarketplaceUnlockVersion = \"13\""));
    assert!(script.contains("isCodexPluginBuildFlavorFilter"));
    assert!(script.contains("source.includes(\"!u(e.marketplaceName)||e.marketplaceName===r\")"));
    assert!(script.contains("source.includes(\"!ne(e.marketplaceName)||e.marketplaceName===n\")"));
    assert!(script.contains("source.includes(\"!t.includes(e.name)\")"));
    assert!(!script.contains("if (!source.includes(\"marketplaceName\")) return false"));
    assert!(!script.contains("if (!source.includes(\"name\")) return false"));
}

#[test]
fn injection_script_expands_api_key_plugin_marketplace_requests() {
    let script = assets::injection_script(57321);

    assert!(script.contains("codexPluginMarketplaceUnlockVersion = \"13\""));
    assert!(script.contains("installPluginMarketplaceRequestPatch"));
    assert!(script.contains("installPluginBuildFlavorFilterPatch"));
    assert!(script.contains("Array.prototype.filter"));
    assert!(script.contains("codexPluginBuildFlavorFilterPatch"));
    assert!(script.contains("isCodexPluginBuildFlavorFilter"));
    assert!(script.contains(
        "codexPluginOfficialMarketplaceName(plugin?.marketplaceName) && !callback(plugin)"
    ));
    assert!(script.contains("isCodexPluginMarketplaceHiddenFilter"));
    assert!(script.contains(
        "codexPluginOfficialMarketplaceName(marketplace?.name) && !callback(marketplace)"
    ));
    assert!(script.contains("plugin_marketplace_hidden_filter_bypassed"));
    assert!(script.contains("method === \"list-plugins\""));
    assert!(script.contains("method === \"vscode://codex/list-plugins\""));
    assert!(script.contains("method === \"plugin/list\""));
    assert!(script.contains("delete next.marketplaceKinds"));
    assert!(script.contains("patchPluginMarketplaceResult"));
    assert!(script.contains("expandVisibleOfficialMarketplacePlugins"));
    assert!(script.contains("pluginMarketplaceMatchesQuery"));
    assert!(script.contains("result.plugins.push(plugin)"));
    assert!(script.contains("pluginMarketplaceAliasForName"));
    assert!(script.contains("marketplace.name = alias"));
    assert!(script.contains("restorePluginMarketplaceName"));
    assert!(script.contains(
        "next.remoteMarketplaceName = restorePluginMarketplaceName(next.remoteMarketplaceName)"
    ));
    assert!(script.contains("if (name === \"openai-bundled\") return \"\""));
    assert!(
        script.contains(
            "if (name === \"openai-curated\") return \"claude-codex-pro-openai-curated\""
        )
    );
    assert!(script.contains(
        "if (name === \"openai-api-curated\") return \"claude-codex-pro-openai-api-curated\""
    ));
    assert!(script.contains("restored === \"openai-api-curated\""));
    assert!(script.contains("pluginName + \"@\" + restorePluginMarketplaceName"));
    assert!(script.contains("const tokens = normalizedQuery.split(/\\s+/).filter(Boolean);"));
    assert!(script.contains("plugin?.interface?.displayName"));
    assert!(script.contains("plugin?.interface?.longDescription"));
    assert!(script.contains("...(Array.isArray(plugin?.keywords) ? plugin.keywords : [])"));
    assert!(script.contains("return tokens.every((token) => haystack.includes(token));"));
    assert!(script.contains(
        "if (name === \"openai-primary-runtime\") return \"claude-codex-pro-openai-primary-runtime\""
    ));
    assert!(script.contains("OpenAI插件1(Claude Codex Pro)"));
    assert!(script.contains("OpenAI插件2(Claude Codex Pro)"));
    assert!(script.contains("OpenAI插件3(Claude Codex Pro)"));
    assert!(script.contains("method === \"install-plugin\""));
    assert!(script.contains("method === \"vscode://codex/plugin/install\""));
    assert!(script.contains("method === \"plugin/install\""));
    assert!(script.contains("plugin_marketplace_response_expanded"));
    assert!(script.contains("plugin_build_flavor_filter_bypassed"));
    assert!(script.contains("plugin_install_request_debug"));
    assert!(script.contains("plugin_install_request_failed"));
    assert!(script.contains("patchPluginMarketplaceRequestMessage"));
    assert!(script.contains("patchPluginMarketplaceResponseData"));
    assert!(script.contains("looksLikePluginMarketplaceResult"));
    assert!(script.contains("installPluginMarketplaceBridgePatch"));
    assert!(script.contains("installPluginMarketplaceWindowEventPatchOnly"));
    assert!(script.contains("bridge.sendMessageFromView = function claudeCodexProPluginMarketplacePatchedSendMessageFromView"));
    assert!(script.contains("window.__codexPluginMarketplaceOriginalDispatchEvent"));
    assert!(script.contains(
        "event?.type === \"codex-message-from-view\" && detail?.type === \"mcp-request\""
    ));
    assert!(script.contains("data?.type === \"fetch-response\""));
    assert!(script.contains("clearPluginMarketplaceQueryCache"));
    assert!(!script.contains("marketplace.path ="));
    assert!(!script.contains("codexPluginMarketplacePathAliasForName"));
    assert!(!script.contains("spoofAnyCodexAuthContext"));
}

#[test]
fn injection_script_merges_real_local_marketplace_snapshots_into_plugin_results() {
    let script = assets::injection_script(57321);

    assert!(script.contains("window.__CLAUDE_CODEX_PRO_PLUGIN_MARKETPLACES__"));
    assert!(script.contains("function localPluginMarketplaces()"));
    assert!(script.contains("function mergeLocalPluginMarketplaces(result)"));
    assert!(script.contains(
        "const locals = localPluginMarketplaces().map(prepareLocalPluginMarketplace).filter(Boolean)"
    ));
    assert!(script.contains("result.marketplaces.push(local)"));
    assert!(script.contains("patchedCount += mergeLocalPluginMarketplaces(result)"));
    assert!(script.contains("plugin_marketplace_local_merged"));
}

#[test]
fn injection_script_deletes_marketplace_kinds_to_request_default_catalog() {
    let script = assets::injection_script(57321);

    assert!(script.contains("delete next.marketplaceKinds"));
    assert!(script.contains("plugin_marketplace_request_expanded"));
    assert!(!script.contains("codexPluginAllowedMarketplaceKinds"));
    assert!(!script.contains("codexPluginExpandedMarketplaceKinds"));
    assert!(!script.contains("next.marketplaceKinds = Array.from(new Set"));
}

#[test]
fn injection_script_logs_marketplace_grouping_diagnostics() {
    let script = assets::injection_script(57321);

    assert!(script.contains("plugin_marketplace_response_debug"));
    assert!(script.contains("marketplaces: result.marketplaces.map"));
    assert!(script.contains("pluginMarketplaceCounts"));
    assert!(script.contains("remoteMarketplaceName"));
}

#[test]
fn injection_script_keeps_force_install_unlock_visual_state_sticky() {
    let script = assets::injection_script(57321);

    assert!(script.contains("codex-force-install-unlocked"));
    assert!(script.contains("codexForcePluginInstallRefreshIntervalMs"));
    assert!(script.contains("refreshForcePluginInstallUnlockLoop"));
    assert!(script.contains("setInterval(() => {"));
}

#[test]
fn injection_script_loads_backend_settings_before_initial_scan() {
    let script = assets::injection_script(57321);
    let startup_call = script
        .rfind("void loadBackendSettingsForStartup();")
        .expect("script should load backend settings on startup");
    let footer = &script[startup_call..];
    let initial_scan = footer
        .find("scan();")
        .expect("script should perform an initial scan");
    let footer_marker = footer
        .find("window.__codexProjectMoveApplyProjection")
        .expect("script should continue bootstrapping after the initial scan");

    assert!(initial_scan < footer_marker);
    assert!(script.contains("if (attempt < 60)"));
}

#[test]
fn injection_script_exposes_conversation_view_width_control() {
    let script = assets::injection_script(57321);

    assert!(script.contains("conversationView: false"));
    assert!(script.contains("conversationView"));
    assert!(script.contains("conversationViewMaxWidth"));
    assert!(script.contains("对话居中宽度"));
    assert!(script.contains("data-claude-codex-pro-conversation-view-width"));
    assert!(script.contains("conversationViewWidth()"));
    assert!(script.contains("normalizeConversationViewWidth"));
}

#[test]
fn injection_script_keeps_session_action_buttons_in_pr_style() {
    let script = assets::injection_script(57321);

    assert!(script.contains("actionButtonClass = \"codex-session-action-button\""));
    assert!(script.contains("background: transparent;"));
    assert!(script.contains("background: #363839;"));
    assert!(script.contains("cursor: default;"));
}

#[test]
fn injection_script_moves_export_and_project_move_into_more_menu() {
    let script = assets::injection_script(57321).replace("\r\n", "\n");

    assert!(script.contains("moreButtonClass = \"codex-session-more-button\""));
    assert!(script.contains("moreMenuClass = \"codex-session-more-menu\""));
    assert!(script.contains("configureActionButton(moreButton, \"更多操作\", \"…\")"));
    assert!(script.contains("createSessionMoreMenuItem(\"导出\""));
    assert!(script.contains("createSessionMoreMenuItem(\"移动\""));
    assert!(script.contains("group.appendChild(moreButton)"));
    assert!(script.contains("installMoreButtonEvents(row, moreButton, openMoreMenu)"));
    assert!(script.contains("installSessionMoreMenuAutoClose(row, moreMenu)"));
    assert!(script.contains("updateSessionMoreMenuDirection(moreButton, moreMenu)"));
    assert!(script.contains("positionSessionMoreMenu(moreButton, moreMenu)"));
    assert!(script.contains("document.body.appendChild(moreMenu)"));
    assert!(script.contains("position: fixed;"));
    assert!(script.contains("codex-session-more-menu-open-up"));
    assert!(script.contains("transform: translateY(calc(-100% - 34px));"));
    assert!(script.contains("positionSessionMoreMenu(moreButton, moreMenu);"));
    assert!(script.contains("row.classList.toggle(\"codex-session-more-open\""));
    assert!(script.contains(".${actionGroupClass} {"));
    assert!(script.contains("position: absolute;"));
    assert!(script.contains("pointer-events: none;"));
    assert!(script.contains("[data-codex-delete-row=\"true\"]:hover .${actionGroupClass} {\n        opacity: 1;\n        pointer-events: auto;\n      }"));
    assert!(script.contains("[data-codex-delete-row=\"true\"].codex-session-more-open .${actionGroupClass} {\n        opacity: 1;\n        pointer-events: auto;\n        z-index: 2147483201;"));
    assert!(!script.contains("installActionButtonEvents(row, moreButton, openMoreMenu)"));
    assert!(!script.contains("group.appendChild(exportButton)"));
    assert!(!script.contains("group.appendChild(moveButton)"));
}

#[test]
fn injection_script_does_not_add_delete_controls_on_archived_page() {
    let script = assets::injection_script(57321);

    assert!(script.contains("attachArchivedPageDeleteButton"));
    assert!(script.contains("data-codex-archive-row-action"));
    assert!(script.contains("dataset.codexArchiveRowAction = \"export\""));
    assert!(!script.contains("dataset.codexArchiveRowAction = \"delete\""));
    assert!(!script.contains("installArchivedDeleteAllButton"));
    assert!(!script.contains("删除全部归档"));
}

#[test]
fn injection_script_does_not_modify_codex_model_selection() {
    let script = assets::injection_script(57321);

    // The catalog endpoint remains a read-only source for service-tier state.
    assert!(script.contains("/codex-model-catalog"));
    assert!(script.contains("loadCodexModelCatalog"));

    for forbidden in [
        // JSON/Statsig/React/App Server model-list mutation paths.
        "patchModelNameArray",
        "patchModelArray",
        "patchModelContainer",
        "patchModelJsonResponse",
        "installModelJsonResponsePatch",
        "patchStatsigModelDynamicConfig",
        "patchStatsigModelWhitelist",
        "patchObjectGraphForModels",
        "patchReactModelState",
        "patchAppServerModelMessages",
        "patchMcpModelResponseData",
        "patchAppServerModelResult",
        "patchAppServerModelRequestClient",
        "installAppServerModelRequestPatch",
        "patchCodexModelWhitelist",
        // DOM menu scanning, injected groups and selection persistence.
        "codexModelMenuSurfaceSelector",
        "codexModelMenuCandidates",
        "codexModelMenuHasModel",
        "codexModelDropdownPatch",
        "data-claude-codex-pro-model-group",
        "data-claude-codex-pro-injected-model",
        "CCP 模型增强",
        "claudeCodexPro.injectedCodexModelSelection",
        "readCodexInjectedModelSelection",
        "writeCodexInjectedModelSelection",
        "applyCodexModelSelectionRequestOverride",
        "model_request_override_applied",
        "model_dropdown_dom_patched",
        "modelWhitelistUnlock",
        "list-models-for-host",
        "model/list",
        "Response.prototype.json",
    ] {
        assert!(
            !script.contains(forbidden),
            "Codex model-selection injection marker must be absent: {forbidden}"
        );
    }
}

#[test]
fn injection_script_exposes_fast_service_tier_control() {
    let script = assets::injection_script(57321);

    assert!(script.contains("default-service-tier"));
    assert!(script.contains("setting-storage-"));
    assert!(script.contains("codexAppAssetUrl"));
    assert!(script.contains("codexThreadServiceTierOverrides"));
    assert!(script.contains("setCodexThreadServiceTierMode"));
    assert!(script.contains("codexServiceTierRequestOverride"));
    assert!(script.contains("codexServiceTierSupportedFastModels"));
    assert!(script.contains("\"gpt-5.4\""));
    assert!(script.contains("\"gpt-5.5\""));
    assert!(script.contains("codexServiceTierFastSupportedForModel"));
    assert!(script.contains("codexServiceTierModelForRequest"));
    assert!(script.contains("codexServiceTierMaybeLoadModelCatalog"));
    assert!(script.contains("fastBlocked"));
    assert!(script.contains("data-tier=\"unsupported\""));
    assert!(script.contains("nextParams.service_tier = override.serviceTier"));
    assert!(script.contains("serviceTierControls: false"));
    assert!(script.contains("data-claude-codex-pro-setting=\"serviceTierControls\""));
    assert!(script.contains("data-codex-service-tier-controls"));
    assert!(script.contains("removeCodexServiceTierBadges"));
    assert!(script.contains("installCodexServiceTierDispatcherPatch"));
    assert!(script.contains("服务模式"));
    assert!(script.contains("data-codex-service-tier-status"));
    assert!(script.contains("data-codex-service-tier-inherit"));
    assert!(script.contains("data-codex-service-tier-standard"));
    assert!(script.contains("data-codex-service-tier-fast"));
    assert!(script.contains("data-codex-service-tier-custom"));
    assert!(script.contains("data-codex-service-tier-thread-inherit"));
    assert!(script.contains("data-codex-service-tier-thread-standard"));
    assert!(script.contains("data-codex-service-tier-thread-fast"));
    assert!(script.contains("global-standard"));
    assert!(script.contains("global-fast"));
    assert!(script.contains("defaultMode"));
    assert!(script.contains("codexServiceTierEffectiveThreadMode"));
    assert!(script.contains("codexServiceTierDefaultModeForControlMode"));
    assert!(script.contains("normalizeCodexServiceTierControlMode(state.mode) !== \"custom\""));
    assert!(script.contains("state.draft = null"));
    assert!(script.contains("后端未连接，无法切换服务模式"));
    assert!(script.contains("未连接"));
    assert!(script.contains("thread/start"));
    assert!(script.contains("thread/resume"));
    assert!(script.contains("turn/start"));
    assert!(script.contains("send-cli-request-for-host"));
    assert!(script.contains("start-conversation"));
    assert!(script.contains("applyCodexRequestOverrides(\"thread/start\", message)"));
    assert!(
        script
            .contains("return applyCodexServiceTierRequestOverride(method, params, threadIdHint);")
    );
    assert!(script.contains("codex-service-tier-badge"));
    assert!(script.contains("installCodexServiceTierBadge"));
    assert!(script.contains("toggleCodexServiceTierFromBadge"));
    assert!(script.contains("wireCodexServiceTierBadge"));
    assert!(script.contains("codexServiceTierBadgePlacement"));
    assert!(script.contains("codexServiceTierBadgeFooterGroup"));
    assert!(script.contains("codexServiceTierFindComposerEl"));
    assert!(script.contains("codexServiceTierVisibleComposerFooters"));
    assert!(script.contains("codexServiceTierBestComposerFooter"));
    assert!(script.contains("codexServiceTierComposerCandidates"));
    assert!(script.contains("codexServiceTierComposerScore"));
    assert!(script.contains("data-codex-service-tier-badge"));
    assert!(script.contains("codexServiceTierBadgeWired"));
    assert!(script.contains("setAttribute(\"role\", \"button\")"));
    assert!(script.contains("setAttribute(\"tabindex\", \"0\")"));
    assert!(script.contains("继承 config.toml"));
    assert!(script.contains("service_tier=\\\"priority\\\""));
    assert!(script.contains("Fast 仅支持"));
    assert!(script.contains("当前 thread"));
    assert!(script.contains("standard"));
    assert!(script.contains("fast"));
}

#[test]
fn injection_script_applies_fast_service_tier_contract() {
    let cases = run_service_tier_contract_harness();

    assert_eq!(cases["supportedFast"]["serviceTier"], "priority");
    assert_eq!(cases["supportedFast"]["service_tier"], "priority");

    assert_eq!(
        cases["unsupportedModel"]["serviceTier"],
        serde_json::Value::Null
    );
    assert_eq!(
        cases["unsupportedModel"]["service_tier"],
        serde_json::Value::Null
    );

    assert_eq!(cases["turnWithoutModel"]["serviceTier"], "priority");
    assert_eq!(cases["turnWithoutModelDiagnosticModel"], "gpt-5.4");

    assert_eq!(
        cases["customInheritUnsupported"]["serviceTier"],
        serde_json::Value::Null
    );
    assert_eq!(
        cases["customInheritUnsupported"]["service_tier"],
        serde_json::Value::Null
    );

    assert_eq!(cases["startConversation"]["serviceTier"], "priority");
}

fn run_service_tier_contract_harness() -> serde_json::Value {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let script_path = temp.path().join("renderer-inject.js");
    let harness_path = temp.path().join("service-tier-harness.cjs");
    std::fs::write(&script_path, assets::injection_script(57321))
        .expect("injection script should be written");
    let mut harness = std::fs::File::create(&harness_path).expect("harness should be created");
    write!(
        harness,
        r#"
const scriptPath = {script_path};
const store = new Map();
store.set("claudeCodexProSettings", JSON.stringify({{ serviceTierControls: true }}));
function node() {{
  return {{
    appendChild() {{}},
    prepend() {{}},
    remove() {{}},
    setAttribute() {{}},
    removeAttribute() {{}},
    addEventListener() {{}},
    querySelector() {{ return null; }},
    querySelectorAll() {{ return []; }},
    closest() {{ return null; }},
    classList: {{ add() {{}}, remove() {{}}, toggle() {{}}, contains() {{ return false; }} }},
    dataset: {{}},
    style: {{}},
    children: [],
    isConnected: true,
    textContent: "",
    innerHTML: "",
  }};
}}
globalThis.window = globalThis;
window.__CLAUDE_CODEX_PRO_TEST_SERVICE_TIER__ = true;
globalThis.document = {{
  scripts: [],
  documentElement: node(),
  body: node(),
  createElement: () => node(),
  querySelector: () => null,
  querySelectorAll: () => [],
  addEventListener() {{}},
  removeEventListener() {{}},
}};
globalThis.localStorage = {{
  getItem: (key) => store.has(key) ? store.get(key) : null,
  setItem: (key, value) => store.set(key, String(value)),
  removeItem: (key) => store.delete(key),
}};
globalThis.location = {{ href: "https://codex.test/thread/thread-12345678", pathname: "/thread/thread-12345678", search: "", hash: "" }};
window.location = globalThis.location;
globalThis.navigator = {{ userAgent: "node-test" }};
globalThis.performance = {{ getEntriesByType: () => [] }};
require(scriptPath);
const api = window.__claudeCodexProServiceTierTest;
api.setServiceTierState({{ serviceTier: "priority", fastTierValue: "priority" }});
api.setModelCatalog({{ status: "ok", model: "gpt-5.4", default_model: "gpt-5.4", models: ["gpt-5.4", "gpt-5.5"] }});

api.setThreadState({{ mode: "global-fast", defaultMode: "fast", entries: {{}} }});
const supportedFast = api.applyServiceTierOverride("turn/start", {{
  threadId: "thread-12345678",
  model: "gpt-5.4",
  service_tier: null,
}}, "conv-should-not-be-model");

const unsupportedModel = api.applyServiceTierOverride("turn/start", {{
  threadId: "thread-12345678",
  model: "gpt-4.1",
  service_tier: "priority",
}}, "conv-should-not-be-model");

const turnWithoutModel = api.applyServiceTierOverride("turn/start", {{
  threadId: "thread-12345678",
  service_tier: null,
}}, "conversation-should-not-be-model");
const turnWithoutModelDiagnosticModel = api.diagnostics().at(-1)?.detail?.model;

api.setModelCatalog({{ status: "ok", model: "gpt-4.1", default_model: "gpt-4.1", models: ["gpt-4.1"] }});
api.setThreadState({{ mode: "custom", defaultMode: "inherit", entries: {{}}, draft: {{ mode: "inherit", at: Date.now() }} }});
api.setServiceTierState({{ serviceTier: "priority" }});
const customInheritUnsupported = api.applyServiceTierOverride("turn/start", {{
  threadId: "thread-12345678",
  service_tier: "priority",
}}, "");

api.setModelCatalog({{ status: "ok", model: "gpt-5.5", default_model: "gpt-5.5", models: ["gpt-5.5"] }});
api.setThreadState({{ mode: "global-fast", defaultMode: "fast", entries: {{}} }});
const startConversation = api.requestOverride({{
  type: "start-conversation",
  threadId: "thread-12345678",
  model: "gpt-5.5",
}});

process.stdout.write(JSON.stringify({{
  supportedFast,
  unsupportedModel,
  turnWithoutModel,
  turnWithoutModelDiagnosticModel,
  customInheritUnsupported,
  startConversation,
}}));
"#,
        script_path = serde_json::to_string(&script_path.to_string_lossy().to_string())
            .expect("script path should serialize")
    )
    .expect("harness should be written");
    drop(harness);

    let output = Command::new("node")
        .arg(&harness_path)
        .output()
        .expect("node should run service-tier harness");
    assert!(
        output.status.success(),
        "node harness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("harness stdout should be JSON")
}

#[test]
fn injection_script_restores_thread_scroll_positions() {
    let script = assets::injection_script(57321);

    assert!(script.contains("threadScrollRestore"));
    assert!(script.contains("codexThreadScroll"));
    assert!(script.contains("installThreadScrollRouteHooks"));
    assert!(script.contains("scheduleThreadScrollSync"));
}

#[test]
fn injection_script_installs_upstream_branch_dropdown_adapter() {
    let script = assets::injection_script(57321);

    assert!(script.contains("installUpstreamBranchDropdownAdapter"));
    assert!(script.contains("installUpstreamPendingWorktreeDispatcherPatch"));
    assert!(script.contains("data-codex-upstream-branch-option"));
    assert!(script.contains("codexUpstreamBranchSelection"));
    assert!(script.contains("/upstream-worktree/defaults"));
    assert!(script.contains("/upstream-worktree/prepare"));
    assert!(script.contains("injectUpstreamBranchOptions"));
    assert!(script.contains("Upstream"));
    assert!(script.contains("data-base-branch"));
    assert!(script.contains("data-project-id"));
    assert!(script.contains("MutationObserver"));
    assert!(script.contains("upstreamWorktreePayloadFromSelection"));
    assert!(script.contains("readUpstreamBranchSelection"));
    assert!(script.contains("writeUpstreamBranchSelection(null)"));
    assert!(script.contains("currentProjectRepoPathFromSelectedProjectButton"));
    assert!(script.contains("currentProjectRepoPathFromStartButton"));
    assert!(script.contains("Start new chat in"));
    assert!(script.contains("codexUpstreamProjectContext"));
    assert!(script.contains("rememberStartNewChatProjectContext"));
    assert!(script.contains("currentProjectContextForBranchMenu"));
    assert!(script.contains("remoteProjectContextFromGlobalState"));
    assert!(script.contains("upstreamBranchDefaultsInflight = new Map()"));
    assert!(script.contains("upstreamRemoteBranchDefaultsCacheTtlMs"));
    assert!(script.contains("upstreamBranchDefaultsInflight.delete(cacheKey)"));
    assert!(script.contains("projectId:"));
    assert!(script.contains("data-codex-upstream-branch-selection-label"));
    assert!(script.contains("syncUpstreamBranchTriggerLabel"));
    assert!(script.contains("syncUpstreamBranchMenuSelection"));
    assert!(script.contains("applyUpstreamPendingWorktreeOverride"));
    assert!(script.contains("pending-worktree-create"));
    assert!(script.contains("qualifiedSourceRef"));
    assert!(script.contains("refs/remotes/${remote}/${baseBranch}"));
    assert!(script.contains("startingState: { ...request.startingState, branchName: sourceRef }"));
    assert!(script.contains("data-codex-upstream-branch-check"));
    assert!(script.contains("data-codex-upstream-branch-icon"));
    assert!(script.contains("branchIconSvg"));
    assert!(script.contains("checkmarkSvg"));
    assert!(script.contains("aria-checked"));
    assert!(script.contains("check.removeAttribute(\"hidden\")"));
    assert!(script.contains("check.setAttribute(\"hidden\", \"\")"));
    assert!(script.contains("handleNativeBranchSelection"));
    assert!(script.contains("clearUpstreamBranchTriggerLabel"));
    assert!(!script.contains(r#"text.includes("/")"#));
    assert!(script.contains("newWorktreeModeActive"));
    assert!(script.contains("effectiveElementRect"));
    assert!(script.contains("removeUpstreamBranchOptions"));
    assert!(script.contains("cleanupInvalidUpstreamBranchOptions"));
    assert!(script.contains("branchMenuInNewWorktreeMode"));
    assert!(script.contains("branchMenuTriggerIsBranchControl"));
    assert!(script.contains("actual-upstream-refs-v16"));
    assert!(script.contains("create and checkout new branch"));
    assert!(script.contains("if (/^start in"));
    assert!(script.contains("if (!branchMenuInNewWorktreeMode(trigger))"));
}

#[test]
fn injection_script_prevents_switching_to_branches_used_by_other_worktrees() {
    let script = assets::injection_script(57321);

    assert!(script.contains("data-codex-branch-worktree-path"));
    assert!(script.contains("annotateBranchMenuWorktreeUsage"));
    assert!(script.contains("branchWorktreePathFromMenuItem"));
    assert!(script.contains("该分支已在另一个 worktree 使用"));
    assert!(script.contains("event.stopImmediatePropagation?.()"));
}

#[test]
fn injection_script_rebuilds_upstream_options_for_each_project_branch_menu() {
    let script = assets::injection_script(57321);

    assert!(script.contains("currentProjectRepoPathForBranchMenu"));
    assert!(script.contains("repoPathFromProjectLabel"));
    assert!(script.contains("projectContextFromProjectLabel"));
    assert!(script.contains("upstreamBranchOptionsMatchRefs"));
    assert!(script.contains("upstreamBranchDefaultsCache = new Map()"));
    assert!(script.contains("actual-upstream-refs-v16"));
}

#[test]
fn manager_ui_exposes_pure_api_relay_mode_button() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should live under crates/claude-codex-pro-core");
    let source =
        std::fs::read_to_string(repo.join("apps/claude-codex-pro-manager/src/App.tsx")).unwrap();
    let commands =
        std::fs::read_to_string(repo.join("apps/claude-codex-pro-manager/src-tauri/src/lib.rs"))
            .unwrap();

    assert!(source.contains("官方混入 API Key"));
    assert!(source.contains("纯 API"));
    assert!(source.contains("apply_pure_api_injection"));
    assert!(commands.contains("commands::apply_pure_api_injection"));
}

#[test]
fn manager_disabling_active_codex_routing_reapplies_supplier_without_clearing_api_mode() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should live under crates/claude-codex-pro-core");
    let source =
        std::fs::read_to_string(repo.join("apps/claude-codex-pro-manager/src/screens.tsx"))
            .unwrap();
    let handler_start = source
        .find("const toggleVisibleSupplierRouting = async")
        .expect("supplier routing handler should exist");
    let handler_end = source[handler_start..]
        .find("const supplierOrderFromIds")
        .expect("supplier routing handler should have a stable boundary");
    let handler = &source[handler_start..handler_start + handler_end];
    let codex_branch_start = handler
        .find("if (isDisablingActiveRoute && supplierRouteGroup === \"codex\")")
        .expect("active Codex route-disable branch should exist");
    let codex_branch_end = handler[codex_branch_start..]
        .find("if (isDisablingActiveRoute && supplierRouteGroup === \"claude-desktop\")")
        .expect("Codex and Claude Desktop branches should remain separate");
    let codex_branch = &handler[codex_branch_start..codex_branch_start + codex_branch_end];
    let direct_switch = "actions.switchSupplierProfile(\"codex\", activeProfileId, nextSettings)";
    let switch_index = codex_branch
        .find(direct_switch)
        .expect("the Codex branch should reapply the active supplier");
    let failure_guard_index = codex_branch
        .find("if (!switched || !statusOk(switched.status)) return")
        .expect("failed supplier reapplication should stop the route toggle");
    let success_notice_index = codex_branch
        .find("actions.showNotice")
        .expect("successful route disable should report completion");

    assert!(
        handler.contains("const nextSettings = { ...appSettings, relayProfiles: nextProfiles }"),
        "the direct switch must receive profiles with routing disabled",
    );
    assert!(
        codex_branch.contains(direct_switch),
        "disabling the active Codex route must reapply the same supplier in direct mode",
    );
    assert!(
        !codex_branch.contains("actions.clearRelayMode"),
        "the route toggle must not invoke the explicit official-mode cleanup action",
    );
    assert!(
        !codex_branch.contains("saveSupplierSettings"),
        "a failed supplier reapplication must not be followed by a settings-only save",
    );
    assert!(
        switch_index < failure_guard_index && failure_guard_index < success_notice_index,
        "the failed-switch guard must run before the success notice",
    );
    assert!(
        source.contains(
            "const [supplierRouteToggleBusy, setSupplierRouteToggleBusy] = useState(false)"
        ),
        "the route toggle should expose an in-flight disabled state",
    );
    assert!(
        source.contains("const supplierRouteToggleInFlightRef = useRef(false)"),
        "the route handler needs a synchronous duplicate-call guard",
    );
    assert!(
        handler.contains("supplierRouteToggleInFlightRef.current")
            && handler.contains("setSupplierRouteToggleBusy(true)")
            && handler.contains("setSupplierRouteToggleBusy(false)"),
        "the route handler must acquire and release its in-flight guard",
    );
    assert!(
        source.contains("const supplierRouteSwitchDisabled = supplierRouteToggleBusy ||"),
        "the visible route toggle must be disabled while a route change is running",
    );
}

#[test]
fn cdp_target_deserializes_websocket_field() {
    let target: CdpTarget = serde_json::from_value(json!({
        "id": "page-1",
        "type": "page",
        "title": "Codex",
        "url": "https://codex.test",
        "webSocketDebuggerUrl": "ws://debug",
    }))
    .expect("target should deserialize");

    assert_eq!(target.target_type, "page");
    assert_eq!(
        target.web_socket_debugger_url.as_deref(),
        Some("ws://debug")
    );
}

#[test]
fn runtime_evaluate_params_sets_expected_flags() {
    let params = bridge::runtime_evaluate_params("1 + 1");

    assert_eq!(params["expression"], "1 + 1");
    assert_eq!(params["awaitPromise"], false);
    assert_eq!(params["returnByValue"], true);
    assert_eq!(params["allowUnsafeEvalBlockedByCSP"], true);
}

#[test]
fn runtime_evaluate_params_can_await_promise_for_bridge_health_checks() {
    let params = bridge::runtime_evaluate_params_with_await_promise("Promise.resolve(true)", true);

    assert_eq!(params["expression"], "Promise.resolve(true)");
    assert_eq!(params["awaitPromise"], true);
    assert_eq!(params["returnByValue"], true);
    assert_eq!(params["allowUnsafeEvalBlockedByCSP"], true);
}

#[test]
fn bridge_health_check_script_uses_real_backend_round_trip() {
    let script = bridge::bridge_health_check_script();

    assert!(script.contains("__codexSessionDeleteBridge"));
    assert!(script.contains("__CLAUDE_CODEX_PRO_MODAL_THEME__"));
    assert!(script.contains("pangu-control-deck"));
    assert!(script.contains("/backend/status"));
    assert!(script.contains("Promise.race"));
    assert!(script.contains("setTimeout"));
}

#[test]
fn bridge_result_expressions_json_escape_inputs() {
    let resolve = bridge::resolve_bridge_expression("request\"1", &json!({"status": "ok"}))
        .expect("resolve expression should build");
    let reject = bridge::reject_bridge_expression("request\"1", "bad \"value\"")
        .expect("reject expression should build");

    assert_eq!(
        resolve,
        r#"window.__codexSessionDeleteResolve("request\"1", {"status":"ok"})"#
    );
    assert_eq!(
        reject,
        r#"window.__codexSessionDeleteReject("request\"1", "bad \"value\"")"#
    );
}

#[test]
fn pick_page_target_prefers_codex_title_or_url() {
    let targets = vec![
        target(
            "first",
            "page",
            "Other",
            "https://example.test",
            Some("ws://first"),
        ),
        target(
            "second",
            "page",
            "Codex",
            "https://example.test",
            Some("ws://second"),
        ),
        target(
            "third",
            "page",
            "Other",
            "https://codex.test",
            Some("ws://third"),
        ),
    ];

    let picked = pick_page_target(&targets).expect("target should be selected");

    assert_eq!(picked.id, "second");
}

#[test]
fn pick_page_target_leniently_falls_back_to_first_injectable_page() {
    let targets = vec![
        target(
            "browser",
            "browser",
            "Codex",
            "https://codex.test",
            Some("ws://browser"),
        ),
        target(
            "first",
            "page",
            "Other",
            "https://example.test",
            Some("ws://first"),
        ),
        target(
            "second",
            "page",
            "Other 2",
            "https://example.test/2",
            Some("ws://second"),
        ),
    ];

    let picked = pick_page_target(&targets).expect("target should be selected");

    assert_eq!(picked.id, "first");
}

#[test]
fn pick_page_target_rejects_non_pages_and_pages_without_websocket() {
    let targets = vec![
        target(
            "browser",
            "browser",
            "Codex",
            "https://codex.test",
            Some("ws://browser"),
        ),
        target("page-no-ws", "page", "Codex", "https://codex.test", None),
    ];

    let error = pick_page_target(&targets).expect_err("no injectable page should be selected");

    assert!(
        error
            .to_string()
            .contains("No injectable page target found")
    );
}

#[test]
fn pick_injectable_codex_page_target_rejects_non_codex_pages() {
    let targets = vec![
        target(
            "browser",
            "browser",
            "Codex",
            "https://codex.test",
            Some("ws://browser"),
        ),
        target(
            "other-page",
            "page",
            "Other App",
            "https://example.test",
            Some("ws://other"),
        ),
    ];

    let error = pick_injectable_codex_page_target(&targets)
        .expect_err("non-Codex page must not be selected for injection");

    assert!(
        error
            .to_string()
            .contains("No injectable Codex page target found")
    );
}

#[test]
fn pick_injectable_codex_page_target_requires_websocket() {
    let targets = vec![target("codex", "page", "Codex", "https://codex.test", None)];

    let error = pick_injectable_codex_page_target(&targets)
        .expect_err("Codex page without websocket must not be selected for injection");

    assert!(
        error
            .to_string()
            .contains("No injectable Codex page target found")
    );
}

#[test]
fn packaged_codex_shell_is_recognized_when_title_is_chatgpt() {
    let main = target(
        "main",
        "page",
        "ChatGPT",
        "app://-/index.html",
        Some("ws://main"),
    );
    let avatar = target(
        "avatar",
        "page",
        "ChatGPT",
        "app://-/index.html?initialRoute=%2Favatar-overlay",
        Some("ws://avatar"),
    );

    assert!(is_codex_page_target(&main));
    assert!(!is_codex_page_target(&avatar));
    assert_eq!(
        pick_injectable_codex_page_target(&[avatar, main])
            .unwrap()
            .id,
        "main"
    );
}

#[tokio::test]
async fn list_targets_can_query_ipv6_loopback_cdp_endpoint() {
    let listener = TcpListener::bind("[::1]:0")
        .await
        .expect("IPv6 loopback listener should bind");
    let port = listener.local_addr().unwrap().port();
    let body = serde_json::to_vec(&json!([
        {
            "id": "page-1",
            "type": "page",
            "title": "Codex",
            "url": "app://-/index.html",
            "webSocketDebuggerUrl": format!("ws://[::1]:{port}/devtools/page/page-1"),
        }
    ]))
    .unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("request should arrive");
        let mut request = [0_u8; 1024];
        let _ = stream.readable().await;
        let _ = stream.try_read(&mut request);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .try_write(response.as_bytes())
            .expect("response headers should write");
        stream.try_write(&body).expect("response body should write");
    });

    let targets = list_targets(port)
        .await
        .expect("CDP target query should fall back to IPv6 loopback");

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].id, "page-1");
    server.await.expect("server task should complete");
}

#[tokio::test]
async fn install_bridge_routes_binding_while_waiting_for_command_response() {
    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("claude-codex-pro.log");
    claude_codex_pro_core::diagnostic_log::set_diagnostic_log_path_for_tests(Some(
        log_path.clone(),
    ));
    let (url, request_rx) = spawn_cdp_server(|mut socket| async move {
        for expected_id in 1..=4 {
            let command = recv_json(&mut socket).await;
            assert_eq!(command["id"], expected_id);
            send_json(&mut socket, json!({ "id": expected_id, "result": {} })).await;
        }

        let evaluate = recv_json(&mut socket).await;
        assert_eq!(evaluate["id"], 5);
        assert_eq!(evaluate["method"], "Runtime.evaluate");
        send_json(
            &mut socket,
            json!({
                "method": "Runtime.bindingCalled",
                "params": {
                    "payload": serde_json::to_string(&json!({
                        "id": "request-1",
                        "path": "delete",
                        "payload": { "target": "session" },
                    })).unwrap(),
                },
            }),
        )
        .await;
        send_json(&mut socket, json!({ "id": 5, "result": {} })).await;

        let response = recv_json(&mut socket).await;
        assert_eq!(response["method"], "Runtime.evaluate");
        assert!(
            response["params"]["expression"]
                .as_str()
                .expect("expression should be string")
                .contains("__codexSessionDeleteResolve")
        );
        send_json(&mut socket, json!({ "id": response["id"], "result": {} })).await;
        close_socket(&mut socket).await;
    })
    .await;

    let handled = Arc::new(AtomicBool::new(false));
    let handler = {
        let handled = Arc::clone(&handled);
        Arc::new(move |path: String, payload: serde_json::Value| {
            let handled = Arc::clone(&handled);
            Box::pin(async move {
                assert_eq!(path, "delete");
                assert_eq!(payload["target"], "session");
                handled.store(true, Ordering::SeqCst);
                Ok(json!({ "status": "ok" }))
            })
                as Pin<Box<dyn Future<Output = anyhow::Result<serde_json::Value>> + Send>>
        })
    };

    tokio::time::timeout(
        Duration::from_secs(2),
        bridge::install_bridge(&url, BRIDGE_BINDING_NAME, handler, &[]),
    )
    .await
    .expect("bridge should not hang while processing interleaved binding call")
    .expect("bridge should keep processing interleaved binding call");
    request_rx
        .await
        .expect("server task should finish without panicking");
    assert!(handled.load(Ordering::SeqCst));
    let contents = std::fs::read_to_string(&log_path).unwrap();
    assert!(contents.contains("bridge.resolve_start"));
    assert!(contents.contains("bridge.resolve_ok"));
    claude_codex_pro_core::diagnostic_log::set_diagnostic_log_path_for_tests(None);
}

#[tokio::test]
async fn install_bridge_immediately_evaluates_new_document_scripts() {
    let (url, request_rx) = spawn_cdp_server(|mut socket| async move {
        for expected_id in 1..=5 {
            let command = recv_json(&mut socket).await;
            assert_eq!(command["id"], expected_id);
            send_json(&mut socket, json!({ "id": expected_id, "result": {} })).await;
        }

        let add_main = recv_json(&mut socket).await;
        assert_eq!(add_main["method"], "Page.addScriptToEvaluateOnNewDocument");
        assert_eq!(add_main["params"]["source"], "window.mainInjected = true;");
        send_json(&mut socket, json!({ "id": add_main["id"], "result": {} })).await;

        let eval_main = recv_json(&mut socket).await;
        assert_eq!(eval_main["method"], "Runtime.evaluate");
        assert_eq!(
            eval_main["params"]["expression"],
            "window.mainInjected = true;"
        );
        send_json(&mut socket, json!({ "id": eval_main["id"], "result": {} })).await;

        let add_user = recv_json(&mut socket).await;
        assert_eq!(add_user["method"], "Page.addScriptToEvaluateOnNewDocument");
        assert_eq!(add_user["params"]["source"], "window.userInjected = true;");
        send_json(&mut socket, json!({ "id": add_user["id"], "result": {} })).await;

        let eval_user = recv_json(&mut socket).await;
        assert_eq!(eval_user["method"], "Runtime.evaluate");
        assert_eq!(
            eval_user["params"]["expression"],
            "window.userInjected = true;"
        );
        send_json(&mut socket, json!({ "id": eval_user["id"], "result": {} })).await;

        close_socket(&mut socket).await;
    })
    .await;

    tokio::time::timeout(
        Duration::from_secs(2),
        bridge::install_bridge(
            &url,
            BRIDGE_BINDING_NAME,
            noop_handler(),
            &[
                "window.mainInjected = true;".to_string(),
                "window.userInjected = true;".to_string(),
            ],
        ),
    )
    .await
    .expect("bridge should not hang while evaluating new document scripts")
    .expect("bridge should evaluate new document scripts immediately");
    request_rx
        .await
        .expect("server task should finish without panicking");
}

#[tokio::test]
async fn install_bridge_returns_after_installing_and_keeps_message_pump_alive() {
    let (url, request_rx) = spawn_cdp_server(|mut socket| async move {
        for expected_id in 1..=5 {
            let command = recv_json(&mut socket).await;
            assert_eq!(command["id"], expected_id);
            send_json(&mut socket, json!({ "id": expected_id, "result": {} })).await;
        }

        let add_script = recv_json(&mut socket).await;
        assert_eq!(
            add_script["method"],
            "Page.addScriptToEvaluateOnNewDocument"
        );
        send_json(&mut socket, json!({ "id": add_script["id"], "result": {} })).await;

        let eval_script = recv_json(&mut socket).await;
        assert_eq!(eval_script["method"], "Runtime.evaluate");
        send_json(
            &mut socket,
            json!({ "id": eval_script["id"], "result": {} }),
        )
        .await;

        send_json(
            &mut socket,
            json!({
                "method": "Runtime.bindingCalled",
                "params": {
                    "payload": serde_json::to_string(&json!({
                        "id": "after-return",
                        "path": "status",
                        "payload": {},
                    })).unwrap(),
                },
            }),
        )
        .await;

        let resolve = recv_json(&mut socket).await;
        assert!(
            resolve["params"]["expression"]
                .as_str()
                .expect("expression should be string")
                .contains("after-return")
        );
        send_json(&mut socket, json!({ "id": resolve["id"], "result": {} })).await;
        close_socket(&mut socket).await;
    })
    .await;

    let handled = Arc::new(AtomicBool::new(false));
    let handler = {
        let handled = Arc::clone(&handled);
        Arc::new(move |_path: String, _payload: serde_json::Value| {
            let handled = Arc::clone(&handled);
            Box::pin(async move {
                handled.store(true, Ordering::SeqCst);
                Ok(json!({ "status": "ok" }))
            })
                as Pin<Box<dyn Future<Output = anyhow::Result<serde_json::Value>> + Send>>
        })
    };

    tokio::time::timeout(
        Duration::from_secs(2),
        bridge::install_bridge(
            &url,
            BRIDGE_BINDING_NAME,
            handler,
            &["window.ready = true;".to_string()],
        ),
    )
    .await
    .expect("bridge install should return after setup")
    .expect("bridge install should succeed");

    request_rx
        .await
        .expect("server task should finish without panicking");
    assert!(handled.load(Ordering::SeqCst));
}

#[tokio::test]
async fn install_bridge_command_error_mentions_method_and_id() {
    let (url, request_rx) = spawn_cdp_server(|mut socket| async move {
        let command = recv_json(&mut socket).await;
        assert_eq!(command["method"], "Runtime.enable");
        send_json(
            &mut socket,
            json!({
                "id": command["id"],
                "error": { "code": -32000, "message": "Runtime disabled" },
            }),
        )
        .await;
        close_socket(&mut socket).await;
    })
    .await;

    let handler = noop_handler();
    let error = tokio::time::timeout(
        Duration::from_secs(2),
        bridge::install_bridge(&url, BRIDGE_BINDING_NAME, handler, &[]),
    )
    .await
    .expect("bridge should not hang on CDP error response")
    .expect_err("CDP error response should fail install");
    let message = error.to_string();

    request_rx
        .await
        .expect("server task should finish without panicking");
    assert!(message.contains("Runtime.enable"), "{message}");
    assert!(message.contains("id 1"), "{message}");
    assert!(message.contains("Runtime disabled"), "{message}");
}

#[tokio::test]
async fn install_bridge_rejects_bad_payload_with_id_and_continues_after_unparseable_payload() {
    let (url, request_rx) = spawn_cdp_server(|mut socket| async move {
        for expected_id in 1..=5 {
            let command = recv_json(&mut socket).await;
            assert_eq!(command["id"], expected_id);
            send_json(&mut socket, json!({ "id": expected_id, "result": {} })).await;
        }

        send_json(
            &mut socket,
            json!({
                "method": "Runtime.bindingCalled",
                "params": { "payload": "{\"id\":\"bad-1\",\"payload\":{}" },
            }),
        )
        .await;
        send_json(
            &mut socket,
            json!({
                "method": "Runtime.bindingCalled",
                "params": { "payload": "not json" },
            }),
        )
        .await;
        send_json(
            &mut socket,
            json!({
                "method": "Runtime.bindingCalled",
                "params": {
                    "payload": serde_json::to_string(&json!({
                        "id": "ok-1",
                        "path": "delete",
                        "payload": {},
                    })).unwrap(),
                },
            }),
        )
        .await;

        let reject = recv_json(&mut socket).await;
        assert!(
            reject["params"]["expression"]
                .as_str()
                .expect("expression should be string")
                .contains("__codexSessionDeleteReject")
        );
        assert!(
            reject["params"]["expression"]
                .as_str()
                .expect("expression should be string")
                .contains("bad-1")
        );
        send_json(&mut socket, json!({ "id": reject["id"], "result": {} })).await;

        let resolve = recv_json(&mut socket).await;
        assert!(
            resolve["params"]["expression"]
                .as_str()
                .expect("expression should be string")
                .contains("__codexSessionDeleteResolve")
        );
        assert!(
            resolve["params"]["expression"]
                .as_str()
                .expect("expression should be string")
                .contains("ok-1")
        );
        send_json(&mut socket, json!({ "id": resolve["id"], "result": {} })).await;
        close_socket(&mut socket).await;
    })
    .await;

    tokio::time::timeout(
        Duration::from_secs(2),
        bridge::install_bridge(&url, BRIDGE_BINDING_NAME, noop_handler(), &[]),
    )
    .await
    .expect("bridge should not hang after bad payload")
    .expect("bad payloads should not terminate the bridge loop");
    request_rx
        .await
        .expect("server task should finish without panicking");
}

#[tokio::test]
async fn install_bridge_queues_consecutive_bindings_without_recursive_dispatch() {
    let (url, request_rx) = spawn_cdp_server(|mut socket| async move {
        for expected_id in 1..=5 {
            let command = recv_json(&mut socket).await;
            assert_eq!(command["id"], expected_id);
            send_json(&mut socket, json!({ "id": expected_id, "result": {} })).await;
        }

        for request_id in ["first", "second", "third"] {
            send_json(
                &mut socket,
                json!({
                    "method": "Runtime.bindingCalled",
                    "params": {
                        "payload": serde_json::to_string(&json!({
                            "id": request_id,
                            "path": "delete",
                            "payload": { "request": request_id },
                        })).unwrap(),
                    },
                }),
            )
            .await;
        }

        let first = recv_json(&mut socket).await;
        assert_eq!(first["method"], "Runtime.evaluate");
        assert_expression_contains_request(&first, "first");
        let second = recv_json(&mut socket).await;
        assert_eq!(second["method"], "Runtime.evaluate");
        assert_expression_contains_request(&second, "second");
        assert_ne!(second["id"], first["id"]);

        let third = recv_json(&mut socket).await;
        assert_eq!(third["method"], "Runtime.evaluate");
        assert_expression_contains_request(&third, "third");
        assert_ne!(third["id"], first["id"]);
        assert_ne!(third["id"], second["id"]);

        close_socket(&mut socket).await;
    })
    .await;

    let handler = Arc::new(|_path: String, payload: serde_json::Value| {
        Box::pin(async move { Ok(json!({ "status": "ok", "request": payload["request"] })) })
            as Pin<Box<dyn Future<Output = anyhow::Result<serde_json::Value>> + Send>>
    });

    tokio::time::timeout(
        Duration::from_secs(2),
        bridge::install_bridge(&url, BRIDGE_BINDING_NAME, handler, &[]),
    )
    .await
    .expect("bridge should not hang while draining queued binding calls")
    .expect("bridge should process queued binding calls");
    request_rx
        .await
        .expect("server task should finish without panicking");
}

#[tokio::test]
async fn install_bridge_does_not_wait_for_resolve_runtime_evaluate_ack() {
    let (url, request_rx) = spawn_cdp_server(|mut socket| async move {
        for expected_id in 1..=5 {
            let command = recv_json(&mut socket).await;
            assert_eq!(command["id"], expected_id);
            send_json(&mut socket, json!({ "id": expected_id, "result": {} })).await;
        }

        send_json(
            &mut socket,
            json!({
                "method": "Runtime.bindingCalled",
                "params": {
                    "payload": serde_json::to_string(&json!({
                        "id": "first",
                        "path": "/backend/status",
                        "payload": {},
                    })).unwrap(),
                },
            }),
        )
        .await;
        let first_resolve = recv_json(&mut socket).await;
        assert_eq!(first_resolve["method"], "Runtime.evaluate");
        assert_expression_contains_request(&first_resolve, "first");

        send_json(
            &mut socket,
            json!({
                "method": "Runtime.bindingCalled",
                "params": {
                    "payload": serde_json::to_string(&json!({
                        "id": "second",
                        "path": "/backend/status",
                        "payload": {},
                    })).unwrap(),
                },
            }),
        )
        .await;
        let second_resolve =
            tokio::time::timeout(Duration::from_millis(500), recv_json(&mut socket))
                .await
                .expect(
                    "second resolve should be sent without waiting for first Runtime.evaluate ack",
                );
        assert_eq!(second_resolve["method"], "Runtime.evaluate");
        assert_expression_contains_request(&second_resolve, "second");
        close_socket(&mut socket).await;
    })
    .await;

    let handler = Arc::new(|_path: String, _payload: serde_json::Value| {
        Box::pin(async { Ok(json!({ "status": "ok" })) })
            as Pin<Box<dyn Future<Output = anyhow::Result<serde_json::Value>> + Send>>
    });

    tokio::time::timeout(
        Duration::from_secs(2),
        bridge::install_bridge(&url, BRIDGE_BINDING_NAME, handler, &[]),
    )
    .await
    .expect("bridge install should not wait for resolve ack")
    .expect("bridge install should survive missing resolve ack");
    request_rx
        .await
        .expect("server task should finish without panicking");
}

#[tokio::test]
async fn install_bridge_does_not_queue_backend_status_behind_slow_route() {
    let release_slow = Arc::new(Notify::new());
    let server_release = Arc::clone(&release_slow);
    let (url, request_rx) = spawn_cdp_server(move |mut socket| async move {
        for expected_id in 1..=5 {
            let command = recv_json(&mut socket).await;
            assert_eq!(command["id"], expected_id);
            send_json(&mut socket, json!({ "id": expected_id, "result": {} })).await;
        }

        for (request_id, path) in [
            ("slow-request", "/slow"),
            ("status-request", "/backend/status"),
        ] {
            send_json(
                &mut socket,
                json!({
                    "method": "Runtime.bindingCalled",
                    "params": {
                        "payload": serde_json::to_string(&json!({
                            "id": request_id,
                            "path": path,
                            "payload": {},
                        })).unwrap(),
                    },
                }),
            )
            .await;
        }

        let status_resolve =
            tokio::time::timeout(Duration::from_millis(500), recv_json(&mut socket))
                .await
                .expect("backend status must resolve while the earlier route is still blocked");
        assert_expression_contains_request(&status_resolve, "status-request");

        server_release.notify_one();
        let slow_resolve = recv_json(&mut socket).await;
        assert_expression_contains_request(&slow_resolve, "slow-request");
        close_socket(&mut socket).await;
    })
    .await;

    let handler_release = Arc::clone(&release_slow);
    let handler = Arc::new(move |path: String, _payload: serde_json::Value| {
        let release_slow = Arc::clone(&handler_release);
        Box::pin(async move {
            if path == "/slow" {
                release_slow.notified().await;
            }
            Ok(json!({ "status": "ok", "path": path }))
        }) as Pin<Box<dyn Future<Output = anyhow::Result<serde_json::Value>> + Send>>
    });

    tokio::time::timeout(
        Duration::from_secs(2),
        bridge::install_bridge(&url, BRIDGE_BINDING_NAME, handler, &[]),
    )
    .await
    .expect("bridge install should return after setup")
    .expect("bridge install should succeed");
    request_rx
        .await
        .expect("server task should finish without panicking");
}

type TestSocket = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

async fn spawn_cdp_server<F, Fut>(handler: F) -> (String, oneshot::Receiver<()>)
where
    F: FnOnce(TestSocket) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test listener should bind");
    let address = listener.local_addr().expect("listener should have address");
    let (done_tx, done_rx) = oneshot::channel();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("client should connect");
        let socket = accept_async(stream)
            .await
            .expect("websocket should upgrade");
        handler(socket).await;
        let _ = done_tx.send(());
    });

    (websocket_url(address), done_rx)
}

fn websocket_url(address: SocketAddr) -> String {
    format!("ws://{address}")
}

async fn recv_json(socket: &mut TestSocket) -> serde_json::Value {
    let message = socket
        .next()
        .await
        .expect("client should send message")
        .expect("message should be readable");
    let Message::Text(text) = message else {
        panic!("expected text websocket message");
    };
    serde_json::from_str(&text).expect("message should be JSON")
}

async fn send_json(socket: &mut TestSocket, value: serde_json::Value) {
    socket
        .send(Message::Text(value.to_string().into()))
        .await
        .expect("message should send");
}

fn assert_expression_contains_request(command: &serde_json::Value, request_id: &str) {
    let expression = command["params"]["expression"]
        .as_str()
        .expect("expression should be string");
    assert!(
        expression.contains("__codexSessionDeleteResolve"),
        "{expression}"
    );
    assert!(expression.contains(request_id), "{expression}");
}

async fn close_socket(socket: &mut TestSocket) {
    socket.close(None).await.expect("websocket should close");
    let _ = tokio::time::timeout(Duration::from_millis(200), socket.next()).await;
}

fn noop_handler() -> bridge::BridgeHandler {
    Arc::new(|_, _| {
        Box::pin(async { Ok(json!({ "status": "ok" })) })
            as Pin<Box<dyn Future<Output = anyhow::Result<serde_json::Value>> + Send>>
    })
}
