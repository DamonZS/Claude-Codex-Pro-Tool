use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use claude_codex_pro_core::claude_desktop_provider::{
    ClaudeDesktopProviderOutcome, ClaudeDesktopProviderPreview, ClaudeDesktopProviderRequest,
};
use claude_codex_pro_core::install::SILENT_BINARY;
use claude_codex_pro_core::memory_assist::{
    MemoryAssistStatus, MemoryAssistStore, MemoryCandidate, MemoryCandidateRequest, MemoryExport,
    MemoryImportRequest, MemoryItem, MemoryItemRequest, MemoryQueryRequest, MemoryQueryResult,
    MemorySelfCheckRequest, MemorySelfCheckResult, MemorySessionRequest, MemorySessionSummary,
};
use claude_codex_pro_core::models::{DeleteResult, SessionRef};
use claude_codex_pro_core::plugin_hub::{
    self, ClaudeDesktopDevModeOutcome, ClaudeDesktopDevModeStatus, ClaudeDesktopMarketplaceOutcome,
    ClaudeDesktopMarketplaceStatus, ClaudeDesktopOrgPluginOutcome, ClaudeDesktopOrgPluginStatus,
    CodexHookTrustPreview, McpbPackageOutcome, PluginHubCatalog, PluginInstallOutcome,
    PluginInstallPreview,
};
use claude_codex_pro_core::script_market::{self, MarketScript, ScriptMarketManifest};
use claude_codex_pro_core::settings::{BackendSettings, RelayProfile, SettingsStore};
use claude_codex_pro_core::status::{LaunchStatus, StatusStore};
use claude_codex_pro_core::user_scripts::UserScriptManager;
use claude_codex_pro_core::zed_remote::{ZedOpenStrategy, ZedRemoteProject};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::Manager;
use tokio::io::{AsyncReadExt as TokioAsyncReadExt, AsyncWriteExt as TokioAsyncWriteExt};
use toml_edit::DocumentMut;

use crate::install::{self, InstallActionResult, InstallOptions};

static CLAUDE_DESKTOP_PROXY_PORT: OnceLock<Mutex<Option<u16>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
pub struct CommandResult<T>
where
    T: Serialize,
{
    pub status: String,
    pub message: String,
    #[serde(flatten)]
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeZhPatchCliResult {
    status: String,
    message: String,
}

const CLAUDE_ZH_PATCH_ELEVATED_TIMEOUT: Duration = Duration::from_secs(300);
const REPAIR_CODEX_FRONTEND_TIMEOUT: Duration = Duration::from_secs(15);
const REPAIR_CODEX_RESTART_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionPayload {
    pub version: String,
    pub exe_path: String,
    pub exe_last_modified_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathState {
    pub status: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OverviewPayload {
    pub codex_app: PathState,
    pub codex_version: Option<String>,
    pub silent_shortcut: PathState,
    pub management_shortcut: PathState,
    pub latest_launch: Option<LaunchStatus>,
    pub current_version: String,
    pub update_status: String,
    pub settings_path: String,
    pub logs_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopPayload {
    pub process_count: usize,
    pub executable_paths: Vec<String>,
    pub install_kind: String,
    pub cdp_status: String,
    pub cdp_blocker: String,
    pub debug_flags_present: bool,
    pub debug_ports: Vec<u16>,
    pub inspector_ports: Vec<u16>,
    pub listening_ports: Vec<u16>,
    pub debug_evidence: Vec<String>,
    pub supported_integration: String,
    pub integrity_status: String,
    pub integrity_message: String,
    pub executable_audits: Vec<claude_codex_pro_core::claude_desktop::ClaudeDesktopExecutableAudit>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopActionPayload {
    pub process_id: Option<u32>,
    pub action: String,
    pub foreground_verified: bool,
    pub foreground_process_id: Option<u32>,
    pub foreground_title: Option<String>,
    pub observed_window_titles: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopDraftRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopDraftPayload {
    pub process_id: Option<u32>,
    pub action: String,
    pub input_chars: usize,
    pub auto_submitted: bool,
    pub foreground_verified: bool,
    pub foreground_process_id: Option<u32>,
    pub foreground_title: Option<String>,
    pub observed_window_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopIntegrityPayload {
    pub executable_audits: Vec<claude_codex_pro_core::claude_desktop::ClaudeDesktopExecutableAudit>,
    pub policy: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeChineseWindowPayload {
    pub open: bool,
    pub label: String,
    pub default_url: String,
    pub injection_mode: String,
    pub cdp_status: String,
    pub cdp_blocker: String,
    pub official_install_kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeZhPatchPayload {
    pub status: claude_codex_pro_core::claude_zh_patch::ClaudeZhPatchStatus,
    pub changed_files: Vec<String>,
    pub backup_dir: String,
    pub logs_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHubWindowPayload {
    pub open: bool,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptOptimizerWindowPayload {
    pub open: bool,
    pub label: String,
    pub default_url: String,
    pub integration_mode: String,
    pub license: String,
}

fn claude_zh_patch_payload(
    status: claude_codex_pro_core::claude_zh_patch::ClaudeZhPatchStatus,
    changed_files: Vec<String>,
) -> ClaudeZhPatchPayload {
    ClaudeZhPatchPayload {
        backup_dir: status.backup_dir.clone(),
        status,
        changed_files,
        logs_path: claude_codex_pro_core::paths::default_diagnostic_log_path()
            .to_string_lossy()
            .to_string(),
    }
}

fn complete_claude_zh_patch_install(
    message: String,
    status: claude_codex_pro_core::claude_zh_patch::ClaudeZhPatchStatus,
    changed_files: Vec<String>,
) -> CommandResult<ClaudeZhPatchPayload> {
    let launch = claude_codex_pro_core::claude_desktop::open_claude_desktop();
    log_manager_event(
        "manager.claude_zh_patch.launch_after_install",
        json!({
            "status": &launch.status,
            "message": &launch.message,
            "action": &launch.action,
            "processId": launch.process_id,
            "foregroundVerified": launch.foreground_verified,
            "foregroundProcessId": launch.foreground_process_id,
            "foregroundTitle": &launch.foreground_title,
            "observedWindowTitles": &launch.observed_window_titles,
        }),
    );

    let payload = claude_zh_patch_payload(status, changed_files);
    if matches!(launch.status.as_str(), "ok" | "accepted") {
        ok(
            &format!("{message} 已自动启动/重启 Claude Desktop，请验证界面语言。"),
            payload,
        )
    } else {
        ok(
            &format!(
                "{message} 汉化已写入，但自动启动 Claude Desktop 失败：{}",
                launch.message
            ),
            payload,
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsPayload {
    pub settings: BackendSettings,
    pub settings_path: String,
    pub user_scripts: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSessionsPayload {
    pub db_path: String,
    pub db_paths: Vec<String>,
    pub sessions: Vec<claude_codex_pro_data::LocalSession>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZedRemoteProjectsPayload {
    pub projects: Vec<ZedRemoteProject>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZedRemoteOpenPayload {
    pub url: String,
    pub strategy: ZedOpenStrategy,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteLocalSessionRequest {
    pub session_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub db_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayPayload {
    pub authenticated: bool,
    pub auth_source: String,
    pub account_label: Option<String>,
    pub config_path: String,
    pub configured: bool,
    pub requires_openai_auth: bool,
    pub has_bearer_token: bool,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayFilesPayload {
    pub config_path: String,
    pub auth_path: String,
    pub config_contents: String,
    pub auth_contents: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelaySwitchPayload {
    pub settings: BackendSettings,
    pub relay: RelayPayload,
    pub settings_path: String,
    pub user_scripts: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopProviderPreviewPayload {
    pub preview: ClaudeDesktopProviderPreview,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopProviderApplyPayload {
    pub outcome: ClaudeDesktopProviderOutcome,
    #[serde(rename = "devModeStatus")]
    pub dev_mode_status: ClaudeDesktopDevModeStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsBackfillPayload {
    pub settings: BackendSettings,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEntriesPayload {
    pub settings: BackendSettings,
    pub entries: claude_codex_pro_core::relay_config::CodexContextEntries,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveContextEntriesPayload {
    pub entries: claude_codex_pro_core::relay_config::CodexContextEntries,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeContextEntriesPayload {
    pub config_path: String,
    pub entries: claude_codex_pro_core::relay_config::CodexContextEntries,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractRelayCommonConfigPayload {
    pub common_config_contents: String,
    pub profile_config_contents: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayProfileTestPayload {
    pub http_status: u16,
    pub endpoint: String,
    pub response_preview: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayProfileModelsPayload {
    pub models: Vec<String>,
    pub endpoint: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRelayFileRequest {
    pub kind: String,
    pub contents: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackfillRelayProfileRequest {
    pub settings: BackendSettings,
    pub profile_id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSettingsRequest {
    pub settings: BackendSettings,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEntryRequest {
    pub settings: BackendSettings,
    pub kind: String,
    pub id: String,
    pub toml_body: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDeleteRequest {
    pub settings: BackendSettings,
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeContextEntryRequest {
    pub kind: String,
    pub id: String,
    pub body: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeContextDeleteRequest {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractRelayCommonConfigRequest {
    pub config_contents: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    #[serde(default)]
    pub app_path: String,
    #[serde(default = "default_debug_port")]
    pub debug_port: u16,
    #[serde(default = "default_helper_port")]
    pub helper_port: u16,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRequest {
    #[serde(default = "default_log_lines")]
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogsPayload {
    pub path: String,
    pub text: String,
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticsPayload {
    pub report: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WatcherPayload {
    pub enabled: bool,
    pub disabled_flag: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdsPayload {
    pub version: u64,
    pub ads: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScriptMarketPayload {
    pub market: Value,
    pub user_scripts: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexPluginMarketplacePayload {
    pub marketplace: claude_codex_pro_core::codex_plugin_marketplace::CodexPluginMarketplaceStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexPluginMarketplaceRepairPayload {
    pub repair: claude_codex_pro_core::codex_plugin_marketplace::CodexPluginMarketplaceRepair,
    pub marketplace: claude_codex_pro_core::codex_plugin_marketplace::CodexPluginMarketplaceStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHubPayload {
    pub catalog: PluginHubCatalog,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHubItemRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHookTrustPayload {
    pub preview: CodexHookTrustPreview,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpbPackagePayload {
    pub package: McpbPackageOutcome,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistStatusPayload {
    pub memory: MemoryAssistStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryAssistRuntimeSnapshot {
    enabled: bool,
    injected: bool,
    status: String,
    active: bool,
    workspace: String,
    total_items: i64,
    pending_candidates: i64,
    summary: String,
    source: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DiagnosticLogRecord {
    timestamp_ms: u64,
    event: String,
    detail: Value,
}

#[derive(Debug, Clone, Default)]
struct RendererRuntimeHeartbeat {
    timestamp_ms: u64,
    runtime: Option<MemoryAssistRuntimeSnapshot>,
    runtime_reported: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistQueryPayload {
    pub memory: MemoryQueryResult,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistItemsPayload {
    pub items: Vec<MemoryItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistItemPayload {
    pub item: MemoryItem,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistCandidatesPayload {
    pub candidates: Vec<MemoryCandidate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistCandidatePayload {
    pub candidate: MemoryCandidate,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistSelfCheckPayload {
    pub report: MemorySelfCheckResult,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistExportPayload {
    pub data: MemoryExport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssistSessionPayload {
    pub summary: MemorySessionSummary,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCandidateListRequest {
    #[serde(default)]
    pub workspace: String,
    #[serde(default = "default_true")]
    pub include_global: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryIdRequest {
    pub id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryIdAndItemRequest {
    pub id: String,
    pub item: MemoryItemRequest,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupPayload {
    pub show_update: bool,
}

#[tauri::command]
pub fn backend_version() -> CommandResult<VersionPayload> {
    ok(
        "Backend version loaded.",
        VersionPayload {
            version: claude_codex_pro_core::version::VERSION.to_string(),
            exe_path: current_exe_path_string(),
            exe_last_modified_ms: current_exe_last_modified_ms(),
        },
    )
}

#[tauri::command]
pub fn startup_options() -> CommandResult<StartupPayload> {
    ok(
        "Startup options loaded.",
        StartupPayload {
            show_update: startup_should_show_update(),
        },
    )
}

pub fn startup_should_show_update() -> bool {
    should_show_update(
        std::env::args(),
        std::env::var("CLAUDE_CODEX_PRO_SHOW_UPDATE")
            .ok()
            .as_deref(),
    )
}

fn default_true() -> bool {
    true
}

pub fn current_exe_path_string() -> String {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub fn current_exe_last_modified_ms() -> Option<u128> {
    std::env::current_exe()
        .ok()
        .and_then(|path| fs::metadata(path).ok())
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
}

fn should_show_update<I, S>(args: I, env_value: Option<&str>) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter().any(|arg| arg.as_ref() == "--show-update") || env_value == Some("1")
}

#[tauri::command]
pub async fn load_overview() -> CommandResult<OverviewPayload> {
    let payload = tauri::async_runtime::spawn_blocking(load_overview_payload).await;
    let Ok((codex_app_path, entrypoints, latest_launch)) = payload else {
        return failed(
            "Overview background task failed.",
            OverviewPayload {
                codex_app: path_state(None),
                codex_version: None,
                silent_shortcut: path_state(None),
                management_shortcut: path_state(None),
                latest_launch: None,
                current_version: claude_codex_pro_core::version::VERSION.to_string(),
                update_status: "not_checked".to_string(),
                settings_path: claude_codex_pro_core::paths::default_settings_path()
                    .to_string_lossy()
                    .to_string(),
                logs_path: claude_codex_pro_core::paths::default_diagnostic_log_path()
                    .to_string_lossy()
                    .to_string(),
            },
        );
    };
    ok(
        "Overview loaded.",
        OverviewPayload {
            codex_version: codex_app_path
                .as_deref()
                .and_then(claude_codex_pro_core::app_paths::codex_app_version),
            codex_app: path_state(codex_app_path),
            silent_shortcut: shortcut_state(entrypoints.silent_shortcut),
            management_shortcut: shortcut_state(entrypoints.management_shortcut),
            latest_launch,
            current_version: claude_codex_pro_core::version::VERSION.to_string(),
            update_status: "not_checked".to_string(),
            settings_path: claude_codex_pro_core::paths::default_settings_path()
                .to_string_lossy()
                .to_string(),
            logs_path: claude_codex_pro_core::paths::default_diagnostic_log_path()
                .to_string_lossy()
                .to_string(),
        },
    )
}

#[tauri::command]
pub async fn load_claude_desktop_status() -> CommandResult<ClaudeDesktopPayload> {
    let status =
        tauri::async_runtime::spawn_blocking(claude_codex_pro_core::claude_desktop::detect_status)
            .await
            .unwrap_or_else(|_| claude_codex_pro_core::claude_desktop::detect_status_light());
    claude_desktop_status_result(status, "status")
}

#[tauri::command]
pub async fn load_claude_desktop_status_light() -> CommandResult<ClaudeDesktopPayload> {
    let status = tauri::async_runtime::spawn_blocking(
        claude_codex_pro_core::claude_desktop::detect_status_light,
    )
    .await
    .unwrap_or_else(|_| claude_codex_pro_core::claude_desktop::detect_status_light());
    claude_desktop_status_result(status, "status_light")
}

fn claude_desktop_status_result(
    status: claude_codex_pro_core::claude_desktop::ClaudeDesktopStatus,
    log_action: &str,
) -> CommandResult<ClaudeDesktopPayload> {
    let message = status.message.clone();
    let result = CommandResult {
        status: status.status.clone(),
        message,
        payload: ClaudeDesktopPayload {
            process_count: status.process_count,
            executable_paths: status.executable_paths,
            install_kind: status.install_kind,
            cdp_status: status.cdp_status,
            cdp_blocker: status.cdp_blocker,
            debug_flags_present: status.debug_flags_present,
            debug_ports: status.debug_ports,
            inspector_ports: status.inspector_ports,
            listening_ports: status.listening_ports,
            debug_evidence: status.debug_evidence,
            supported_integration: status.supported_integration,
            integrity_status: status.integrity_status,
            integrity_message: status.integrity_message,
            executable_audits: status.executable_audits,
        },
    };
    log_claude_desktop_command(log_action, &result);
    result
}

#[tauri::command]
pub async fn load_claude_desktop_integrity() -> CommandResult<ClaudeDesktopIntegrityPayload> {
    let result = tauri::async_runtime::spawn_blocking(
        claude_codex_pro_core::claude_desktop::detect_integrity_status,
    )
    .await
    .unwrap_or_else(
        |_| claude_codex_pro_core::claude_desktop::ClaudeDesktopIntegrityStatus {
            status: "failed".to_string(),
            message: "Claude Desktop integrity audit background task failed.".to_string(),
            executable_audits: Vec::new(),
            policy: "read_only_audit_no_executable_or_asar_patch".to_string(),
        },
    );
    let command_result = CommandResult {
        status: result.status,
        message: result.message,
        payload: ClaudeDesktopIntegrityPayload {
            executable_audits: result.executable_audits,
            policy: result.policy,
        },
    };
    log_claude_desktop_command("integrity", &command_result);
    command_result
}

#[tauri::command]
pub fn focus_claude_desktop() -> CommandResult<ClaudeDesktopActionPayload> {
    let result = claude_codex_pro_core::claude_desktop::focus_claude_window();
    let command_result = CommandResult {
        status: result.status,
        message: result.message,
        payload: ClaudeDesktopActionPayload {
            process_id: result.process_id,
            action: result.action,
            foreground_verified: result.foreground_verified,
            foreground_process_id: result.foreground_process_id,
            foreground_title: result.foreground_title,
            observed_window_titles: result.observed_window_titles,
        },
    };
    log_claude_desktop_command("focus", &command_result);
    command_result
}

#[tauri::command]
pub fn verify_claude_desktop() -> CommandResult<ClaudeDesktopActionPayload> {
    let result = claude_codex_pro_core::claude_desktop::verify_claude_target();
    let command_result = CommandResult {
        status: result.status,
        message: result.message,
        payload: ClaudeDesktopActionPayload {
            process_id: result.process_id,
            action: result.action,
            foreground_verified: result.foreground_verified,
            foreground_process_id: result.foreground_process_id,
            foreground_title: result.foreground_title,
            observed_window_titles: result.observed_window_titles,
        },
    };
    log_claude_desktop_command("verify", &command_result);
    command_result
}

#[tauri::command]
pub fn open_claude_desktop_devtools() -> CommandResult<ClaudeDesktopActionPayload> {
    let result = claude_codex_pro_core::claude_desktop::open_claude_devtools();
    let command_result = CommandResult {
        status: result.status,
        message: result.message,
        payload: ClaudeDesktopActionPayload {
            process_id: result.process_id,
            action: result.action,
            foreground_verified: result.foreground_verified,
            foreground_process_id: result.foreground_process_id,
            foreground_title: result.foreground_title,
            observed_window_titles: result.observed_window_titles,
        },
    };
    log_claude_desktop_command("open_devtools", &command_result);
    command_result
}

#[tauri::command]
pub async fn open_claude_desktop() -> CommandResult<ClaudeDesktopActionPayload> {
    let helper_status = ensure_claude_desktop_proxy_helper().await;
    let proxy_port = helper_status
        .as_ref()
        .copied()
        .unwrap_or_else(|_| current_claude_desktop_proxy_port_hint());
    let helper_online = helper_status.is_ok() && wait_helper_backend_online(proxy_port).await;
    let result = claude_codex_pro_core::claude_desktop::open_claude_desktop();
    let result = if let Err(error) = helper_status {
        claude_codex_pro_core::claude_desktop::ClaudeDesktopActionResult {
            status: if result.status == "ok" {
                "warning".to_string()
            } else {
                result.status.clone()
            },
            message: format!("{} 本地模型代理启动失败：{error}", result.message),
            ..result
        }
    } else if !helper_online {
        claude_codex_pro_core::claude_desktop::ClaudeDesktopActionResult {
            status: if result.status == "ok" {
                "warning".to_string()
            } else {
                result.status.clone()
            },
            message: format!(
                "{} 本地模型代理已请求启动，但 127.0.0.1:{proxy_port}/backend/status 尚未响应。",
                result.message
            ),
            ..result
        }
    } else {
        result
    };
    let command_result = CommandResult {
        status: result.status,
        message: result.message,
        payload: ClaudeDesktopActionPayload {
            process_id: result.process_id,
            action: result.action,
            foreground_verified: result.foreground_verified,
            foreground_process_id: result.foreground_process_id,
            foreground_title: result.foreground_title,
            observed_window_titles: result.observed_window_titles,
        },
    };
    log_claude_desktop_command("open", &command_result);
    command_result
}

async fn ensure_claude_desktop_proxy_helper() -> anyhow::Result<u16> {
    if let Some(port) = cached_claude_desktop_proxy_port() {
        if claude_codex_pro_core::launcher::ensure_detached_helper(port)
            .await
            .is_ok()
        {
            return Ok(port);
        }
    }
    let preferred = claude_codex_pro_core::protocol_proxy::DEFAULT_CLAUDE_DESKTOP_PROXY_PORT;
    match claude_codex_pro_core::launcher::ensure_detached_helper(preferred).await {
        Ok(()) => {
            set_cached_claude_desktop_proxy_port(preferred);
            Ok(preferred)
        }
        Err(first_error) => {
            let fallback = claude_codex_pro_core::ports::find_available_loopback_port();
            if fallback == 0 || fallback == preferred {
                return Err(first_error);
            }
            claude_codex_pro_core::launcher::ensure_detached_helper(fallback)
                .await
                .with_context(|| {
                    format!(
                        "Claude Desktop proxy fallback port {fallback} failed after preferred port {preferred} failed: {first_error}"
                    )
                })?;
            set_cached_claude_desktop_proxy_port(fallback);
            log_manager_event(
                "manager.claude_proxy.fallback_port",
                json!({
                    "preferredPort": preferred,
                    "fallbackPort": fallback,
                    "reason": first_error.to_string()
                }),
            );
            Ok(fallback)
        }
    }
}

fn current_claude_desktop_proxy_port_hint() -> u16 {
    cached_claude_desktop_proxy_port().unwrap_or_else(|| {
        let preferred = claude_codex_pro_core::protocol_proxy::DEFAULT_CLAUDE_DESKTOP_PROXY_PORT;
        if claude_codex_pro_core::ports::can_bind_loopback_port(preferred)
            || helper_backend_online(preferred)
        {
            preferred
        } else {
            claude_codex_pro_core::ports::find_available_loopback_port()
        }
    })
}

fn cached_claude_desktop_proxy_port() -> Option<u16> {
    CLAUDE_DESKTOP_PROXY_PORT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|guard| *guard)
}

fn set_cached_claude_desktop_proxy_port(port: u16) {
    if let Ok(mut guard) = CLAUDE_DESKTOP_PROXY_PORT
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *guard = Some(port);
    }
}

#[tauri::command]
pub async fn open_claude_chinese_window(
    app: tauri::AppHandle,
) -> CommandResult<ClaudeChineseWindowPayload> {
    let status = claude_codex_pro_core::claude_desktop::detect_status_light();
    let label = "claude-chinese";
    let default_url = "https://claude.ai/new";
    let script = claude_codex_pro_core::assets::claude_chinese_injection_script();
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.eval(script);
        return ok(
            "Claude localization window focused.",
            claude_chinese_window_payload(&app, &status),
        );
    }

    let url = match tauri::Url::parse(default_url) {
        Ok(url) => url,
        Err(error) => {
            return failed(
                &format!("Claude localization URL is invalid: {error}"),
                claude_chinese_window_payload(&app, &status),
            );
        }
    };
    let handle = app.clone();
    let nav_handle = app.clone();
    let build_result = tauri::async_runtime::spawn_blocking(move || {
        tauri::WebviewWindowBuilder::new(&handle, label, tauri::WebviewUrl::External(url))
            .title("Claude localization")
            .inner_size(1220.0, 860.0)
            .min_inner_size(980.0, 720.0)
            .initialization_script(script)
            .on_navigation(move |url| {
                if url.scheme() == "claude-codex-pro" && url.host_str() == Some("plugin-hub") {
                    let app = nav_handle.clone();
                    let route_app = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        let _ = route_main_window_to_plugin_hub(&route_app);
                    });
                    return false;
                }
                true
            })
            .build()
    })
    .await;
    match build_result {
        Ok(Ok(window)) => {
            let _ = window.set_focus();
            let _ = window.eval(script);
            ok(
                "Claude localization window opened.",
                claude_chinese_window_payload(&app, &status),
            )
        }
        Ok(Err(error)) => failed(
            &format!("Claude localization window failed to open: {error}"),
            claude_chinese_window_payload(&app, &status),
        ),
        Err(error) => failed(
            &format!("Claude localization background task failed: {error}"),
            claude_chinese_window_payload(&app, &status),
        ),
    }
}

#[tauri::command]
pub async fn open_plugin_hub_window(
    app: tauri::AppHandle,
) -> CommandResult<PluginHubWindowPayload> {
    match route_main_window_to_plugin_hub(&app) {
        Ok(()) => ok(
            "Plugin hub opened in manager.",
            PluginHubWindowPayload {
                open: true,
                label: "main".to_string(),
            },
        ),
        Err(error) => failed(
            &format!("Plugin hub failed to open in manager: {error}"),
            PluginHubWindowPayload {
                open: false,
                label: "main".to_string(),
            },
        ),
    }
}

#[tauri::command]
pub async fn open_prompt_optimizer_window(
    app: tauri::AppHandle,
) -> CommandResult<PromptOptimizerWindowPayload> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.eval(main_window_route_script("tools"));
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    let payload = prompt_optimizer_window_payload(false);
    match open_url(&payload.default_url) {
        Ok(()) => ok("Prompt optimizer opened in the system browser.", payload),
        Err(error) => failed(
            &format!("Prompt optimizer failed to open: {error}"),
            payload,
        ),
    }
}

#[tauri::command]
pub fn load_claude_chinese_window_status(
    app: tauri::AppHandle,
) -> CommandResult<ClaudeChineseWindowPayload> {
    let status = claude_codex_pro_core::claude_desktop::detect_status_light();
    ok(
        "Claude localization status loaded.",
        claude_chinese_window_payload(&app, &status),
    )
}

#[tauri::command]
pub fn load_claude_zh_patch_status() -> CommandResult<ClaudeZhPatchPayload> {
    let status = claude_codex_pro_core::claude_zh_patch::detect_status();
    ok(
        &status.message.clone(),
        claude_zh_patch_payload(status, Vec::new()),
    )
}

#[tauri::command]
pub async fn install_claude_zh_patch() -> CommandResult<ClaudeZhPatchPayload> {
    log_manager_event("manager.claude_zh_patch.install.start", json!({}));
    if !claude_codex_pro_core::claude_desktop::close_claude_desktop_for_patch() {
        log_manager_event(
            "manager.claude_zh_patch.install.close_claude_failed",
            json!({}),
        );
        let status = claude_codex_pro_core::claude_zh_patch::detect_status();
        return failed(
            "Failed to close Claude Desktop before patch. Please exit Claude and retry.",
            claude_zh_patch_payload(status, Vec::new()),
        );
    }
    if claude_codex_pro_core::claude_zh_patch::detected_patch_needs_elevation() {
        log_manager_event(
            "manager.claude_zh_patch.install.elevation_required",
            json!({}),
        );
        match install_claude_zh_patch_elevated() {
            Ok(result) if result.status == "ok" => {
                let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                if status.status != "ok" {
                    return failed(
                        &format!(
                            "Claude Chinese patch elevated run did not complete: {}",
                            status.message
                        ),
                        claude_zh_patch_payload(status, Vec::new()),
                    );
                }
                return complete_claude_zh_patch_install(result.message, status, Vec::new());
            }
            Ok(result) => {
                let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                return failed(
                    &format!(
                        "Claude Chinese patch elevated run failed: {}",
                        result.message
                    ),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
            Err(error) => {
                let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                return failed(
                    &format!(
                        "Claude Chinese patch requires administrator approval, but elevation failed: {error}"
                    ),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
        }
    }
    log_manager_event("manager.claude_zh_patch.install.direct.start", json!({}));
    match claude_codex_pro_core::claude_zh_patch::install_patch_with_remote_resources().await {
        Ok(outcome) => complete_claude_zh_patch_install(
            outcome.status.message.clone(),
            outcome.status,
            outcome.changed_files,
        ),
        Err(error) => {
            log_manager_event(
                "manager.claude_zh_patch.direct.failed",
                json!({
                    "error": error.to_string(),
                }),
            );
            if should_retry_claude_zh_patch_with_elevation(&error) {
                match install_claude_zh_patch_elevated() {
                    Ok(result) if result.status == "ok" => {
                        let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                        if status.status != "ok" {
                            return failed(
                                &format!(
                                    "Claude Chinese patch elevated fallback did not complete: {}",
                                    status.message
                                ),
                                claude_zh_patch_payload(status, Vec::new()),
                            );
                        }
                        return complete_claude_zh_patch_install(
                            result.message,
                            status,
                            Vec::new(),
                        );
                    }
                    Ok(result) => {
                        let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                        return failed(
                            &format!(
                                "Claude Chinese patch elevated fallback failed: {}",
                                result.message
                            ),
                            claude_zh_patch_payload(status, Vec::new()),
                        );
                    }
                    Err(elevation_error) => {
                        let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                        return failed(
                            &format!(
                                "Claude Chinese patch requires administrator approval, but fallback elevation failed: {elevation_error}; direct error: {error}"
                            ),
                            claude_zh_patch_payload(status, Vec::new()),
                        );
                    }
                }
            }
            let status = claude_codex_pro_core::claude_zh_patch::detect_status();
            failed(
                &format!("Claude manual Chinese patch failed: {error}"),
                claude_zh_patch_payload(status, Vec::new()),
            )
        }
    }
}

fn should_retry_claude_zh_patch_with_elevation(error: &anyhow::Error) -> bool {
    let status = claude_codex_pro_core::claude_zh_patch::detect_status();
    should_retry_claude_zh_patch_status_with_elevation(&status.install_kind, error)
}

fn should_retry_claude_zh_patch_with_elevation_at_install_root(
    install_root: &Path,
    error: &anyhow::Error,
) -> bool {
    let status = claude_codex_pro_core::claude_zh_patch::status_for_install_root(install_root);
    should_retry_claude_zh_patch_status_with_elevation(&status.install_kind, error)
}

fn should_retry_claude_zh_patch_status_with_elevation(
    install_kind: &str,
    error: &anyhow::Error,
) -> bool {
    if install_kind != "msix" {
        return false;
    }
    let error = error.to_string().to_ascii_lowercase();
    error.contains("access is denied")
        || error.contains("permission denied")
        || error.contains("windowsapps")
        || error.contains("zh-cn.json")
        || error.contains(".tmp")
}

pub fn handle_internal_cli() -> bool {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        return false;
    };
    if command != "--internal-install-claude-zh-patch"
        && command != "--internal-restore-claude-zh-patch"
    {
        return false;
    }
    let result_path = args.next().map(PathBuf::from);
    let target_user_sid = args.next().filter(|value| !value.trim().is_empty());
    let target_appdata = args.next().filter(|value| !value.trim().is_empty());
    let target_localappdata = args.next().filter(|value| !value.trim().is_empty());
    let target_install_root = args.next().filter(|value| !value.trim().is_empty());
    let target_diagnostic_log = args.next().filter(|value| !value.trim().is_empty());
    if let Some(path) = target_diagnostic_log.as_deref() {
        claude_codex_pro_core::diagnostic_log::set_diagnostic_log_path_override(Some(
            PathBuf::from(path),
        ));
    }
    log_manager_event(
        "manager.claude_zh_patch.internal.start",
        json!({
            "command": command,
            "targetUserSidPresent": target_user_sid.is_some(),
            "targetAppDataPresent": target_appdata.is_some(),
            "targetLocalAppDataPresent": target_localappdata.is_some(),
            "targetInstallRootPresent": target_install_root.is_some(),
            "targetDiagnosticLogPresent": target_diagnostic_log.is_some(),
        }),
    );
    let result = match command.as_str() {
        "--internal-install-claude-zh-patch" => install_claude_zh_patch_internal(
            target_user_sid.as_deref(),
            target_appdata.as_deref(),
            target_localappdata.as_deref(),
            target_install_root.as_deref(),
        ),
        "--internal-restore-claude-zh-patch" => restore_claude_zh_patch_internal(
            target_user_sid.as_deref(),
            target_appdata.as_deref(),
            target_localappdata.as_deref(),
            target_install_root.as_deref(),
        ),
        _ => unreachable!(),
    };
    let cli_result = match result {
        Ok(message) => ClaudeZhPatchCliResult {
            status: "ok".to_string(),
            message,
        },
        Err(error) => ClaudeZhPatchCliResult {
            status: "failed".to_string(),
            message: error.to_string(),
        },
    };
    log_manager_event(
        "manager.claude_zh_patch.internal.finish",
        json!({
            "command": command,
            "status": cli_result.status,
            "message": cli_result.message,
        }),
    );
    if let Some(path) = result_path {
        if let Ok(text) = serde_json::to_string(&cli_result) {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(error) = fs::write(&path, text) {
                log_manager_event(
                    "manager.claude_zh_patch.internal.result_write_failed",
                    json!({
                        "path": path,
                        "error": error.to_string(),
                    }),
                );
            }
        }
    }
    cli_result.status == "ok"
}

fn install_claude_zh_patch_internal(
    target_user_sid: Option<&str>,
    target_appdata: Option<&str>,
    target_localappdata: Option<&str>,
    target_install_root: Option<&str>,
) -> anyhow::Result<String> {
    if !claude_codex_pro_core::claude_desktop::close_claude_desktop_for_patch() {
        anyhow::bail!("Failed to close Claude Desktop before patch");
    }
    let runtime = tokio::runtime::Runtime::new()?;
    let appdata = target_appdata.map(Path::new);
    let localappdata = target_localappdata.map(Path::new);
    if let Some(root) = target_install_root.map(Path::new) {
        runtime.block_on(claude_codex_pro_core::claude_zh_patch::install_patch_with_remote_resources_elevated_for_user_dirs_at_install_root(
            root,
            target_user_sid,
            appdata,
            localappdata,
        ))?;
    } else {
        runtime.block_on(claude_codex_pro_core::claude_zh_patch::install_patch_with_remote_resources_elevated_for_user_dirs(
            target_user_sid,
            appdata,
            localappdata,
        ))?;
    }
    Ok("Claude Chinese patch installed.".to_string())
}

fn restore_claude_zh_patch_internal(
    target_user_sid: Option<&str>,
    target_appdata: Option<&str>,
    target_localappdata: Option<&str>,
    target_install_root: Option<&str>,
) -> anyhow::Result<String> {
    if !claude_codex_pro_core::claude_desktop::close_claude_desktop_for_patch() {
        anyhow::bail!("Failed to close Claude Desktop before restore");
    }
    let appdata = target_appdata.map(Path::new);
    let localappdata = target_localappdata.map(Path::new);
    if let Some(root) = target_install_root.map(Path::new) {
        claude_codex_pro_core::claude_zh_patch::restore_patch_elevated_for_user_dirs_at_install_root(
            root,
            target_user_sid,
            appdata,
            localappdata,
        )?;
    } else {
        claude_codex_pro_core::claude_zh_patch::restore_patch_elevated_for_user_dirs(
            target_user_sid,
            appdata,
            localappdata,
        )?;
    }
    Ok("Claude official files restored.".to_string())
}

fn install_claude_zh_patch_elevated() -> anyhow::Result<ClaudeZhPatchCliResult> {
    run_claude_zh_patch_elevated("--internal-install-claude-zh-patch", None)
}

fn restore_claude_zh_patch_elevated() -> anyhow::Result<ClaudeZhPatchCliResult> {
    run_claude_zh_patch_elevated("--internal-restore-claude-zh-patch", None)
}

fn install_claude_zh_patch_elevated_at_install_root(
    install_root: &Path,
) -> anyhow::Result<ClaudeZhPatchCliResult> {
    run_claude_zh_patch_elevated("--internal-install-claude-zh-patch", Some(install_root))
}

fn run_claude_zh_patch_elevated(
    internal_command: &str,
    install_root: Option<&Path>,
) -> anyhow::Result<ClaudeZhPatchCliResult> {
    let exe = std::env::current_exe()?;
    let result_dir = claude_codex_pro_core::paths::default_app_state_dir().join("tmp");
    fs::create_dir_all(&result_dir).with_context(|| {
        format!(
            "Failed to create Claude Chinese patch result directory: {}",
            result_dir.display()
        )
    })?;
    let result_path = result_dir.join(format!(
        "claude-codex-pro-zh-patch-{}-{}.json",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    ));
    if result_path.exists() {
        let _ = fs::remove_file(&result_path);
    }
    let exe_quoted = powershell_single_quoted(&exe.to_string_lossy());
    let target_user_sid = current_user_sid().unwrap_or_default();
    let (target_appdata, target_localappdata) = current_user_data_dirs();
    let target_install_root = install_root
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            claude_codex_pro_core::claude_zh_patch::detect_status()
                .install_root
                .unwrap_or_default()
        });
    let diagnostic_log_path = claude_codex_pro_core::paths::default_diagnostic_log_path()
        .to_string_lossy()
        .to_string();
    let argument_list = windows_argument_list(&[
        internal_command,
        &result_path.to_string_lossy(),
        &target_user_sid,
        &target_appdata,
        &target_localappdata,
        &target_install_root,
        &diagnostic_log_path,
    ]);
    let argument_list_quoted = powershell_single_quoted(&argument_list);
    log_manager_event(
        "manager.claude_zh_patch.elevated.start",
        json!({
            "command": internal_command,
            "exe": exe,
            "resultPath": result_path,
            "diagnosticLogPath": diagnostic_log_path,
            "targetUserSidPresent": !target_user_sid.trim().is_empty(),
            "targetAppDataPresent": !target_appdata.trim().is_empty(),
            "targetLocalAppDataPresent": !target_localappdata.trim().is_empty(),
            "targetInstallRoot": target_install_root,
        }),
    );
    let script = format!(
        "$ErrorActionPreference='Stop'; try {{ $p = Start-Process -FilePath {exe_quoted} -ArgumentList {argument_list_quoted} -Verb RunAs -Wait -PassThru; if ($null -eq $p) {{ exit 1 }}; exit $p.ExitCode }} catch {{ Write-Error $_; exit 1 }}"
    );
    let mut command = std::process::Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-WindowStyle",
        "Hidden",
        "-Command",
        &script,
    ]);
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(claude_codex_pro_core::windows_create_no_window());
    }
    let output = run_elevated_process_with_timeout(&mut command)?;
    log_manager_event(
        "manager.claude_zh_patch.elevated.exit",
        json!({
            "command": internal_command,
            "resultPath": result_path,
            "success": output.status.success(),
            "exitCode": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout).trim(),
            "stderr": String::from_utf8_lossy(&output.stderr).trim(),
        }),
    );
    if !output.status.success() {
        anyhow::bail!(
            "User cancelled elevation or elevated child failed: {:?}; stdout={}; stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let contents = fs::read_to_string(&result_path).with_context(|| {
        format!(
            "Elevated child did not write result file: {}; stdout={}; stderr={}",
            result_path.display(),
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        )
    })?;
    let _ = fs::remove_file(&result_path);
    let result = serde_json::from_str::<ClaudeZhPatchCliResult>(&contents)?;
    log_manager_event(
        "manager.claude_zh_patch.elevated.result",
        json!({
            "command": internal_command,
            "status": result.status,
            "message": result.message,
        }),
    );
    Ok(result)
}

fn powershell_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn windows_argument_list(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| windows_quote_arg(value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn windows_quote_arg(value: &str) -> String {
    let mut quoted = String::from("\"");
    let mut backslashes = 0;
    for ch in value.chars() {
        if ch == '\\' {
            backslashes += 1;
            continue;
        }
        if ch == '"' {
            quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
            quoted.push('"');
        } else {
            quoted.push_str(&"\\".repeat(backslashes));
            quoted.push(ch);
        }
        backslashes = 0;
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

fn run_elevated_process_with_timeout(
    command: &mut std::process::Command,
) -> anyhow::Result<std::process::Output> {
    let mut child = command.spawn()?;
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return child
                .wait_with_output()
                .context("Failed to collect elevated Claude Chinese patch output");
        }
        if started.elapsed() >= CLAUDE_ZH_PATCH_ELEVATED_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait_with_output();
            anyhow::bail!(
                "Elevated Claude Chinese patch timed out. Confirm the UAC prompt was handled and retry."
            );
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn current_user_sid() -> Option<String> {
    let output = std::process::Command::new("whoami.exe")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.split(',')
        .map(|part| part.trim().trim_matches('"'))
        .find(|part| {
            part.starts_with("S-")
                && part
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || ch == '-' || ch == 'S')
        })
        .map(str::to_string)
}

fn current_user_data_dirs() -> (String, String) {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
    (appdata, localappdata)
}

#[tauri::command]
pub async fn install_claude_zh_patch_at_install_root(
    install_root: String,
) -> CommandResult<ClaudeZhPatchPayload> {
    let install_root = PathBuf::from(install_root);
    log_manager_event(
        "manager.claude_zh_patch.manual_install.start",
        json!({
            "installRoot": install_root,
        }),
    );
    if !claude_codex_pro_core::claude_desktop::close_claude_desktop_for_patch() {
        log_manager_event(
            "manager.claude_zh_patch.manual_install.close_claude_failed",
            json!({}),
        );
        let status = claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
        return failed(
            "Failed to close Claude Desktop before manual patch. Please exit Claude and retry.",
            claude_zh_patch_payload(status, Vec::new()),
        );
    }
    if claude_codex_pro_core::claude_zh_patch::install_root_patch_needs_elevation(&install_root) {
        log_manager_event(
            "manager.claude_zh_patch.manual_install.elevation_required",
            json!({
                "installRoot": install_root,
            }),
        );
        match install_claude_zh_patch_elevated_at_install_root(&install_root) {
            Ok(result) if result.status == "ok" => {
                let status =
                    claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
                if status.status != "ok" {
                    return failed(
                        &format!(
                            "Claude manual Chinese patch elevated run did not complete: {}",
                            status.message
                        ),
                        claude_zh_patch_payload(status, Vec::new()),
                    );
                }
                return complete_claude_zh_patch_install(result.message, status, Vec::new());
            }
            Ok(result) => {
                let status =
                    claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
                return failed(
                    &format!(
                        "Claude manual Chinese patch elevated run failed: {}",
                        result.message
                    ),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
            Err(error) => {
                let status =
                    claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
                return failed(
                    &format!(
                        "Claude manual Chinese patch requires administrator approval, but elevation failed: {error}"
                    ),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
        }
    }
    log_manager_event(
        "manager.claude_zh_patch.manual_install.direct.start",
        json!({
            "installRoot": install_root,
        }),
    );
    match claude_codex_pro_core::claude_zh_patch::install_patch_at_install_root_with_remote_resources(&install_root).await {
        Ok(outcome) => complete_claude_zh_patch_install(
            outcome.status.message.clone(),
            outcome.status,
            outcome.changed_files,
        ),
        Err(error) => {
            log_manager_event(
                "manager.claude_zh_patch.manual_direct.failed",
                json!({
                    "installRoot": install_root,
                    "error": error.to_string(),
                }),
            );
            if should_retry_claude_zh_patch_with_elevation_at_install_root(&install_root, &error) {
                match install_claude_zh_patch_elevated_at_install_root(&install_root) {
                    Ok(result) if result.status == "ok" => {
                        let status = claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
                        if status.status != "ok" {
                            return failed(
                                &format!("Claude manual Chinese patch elevated fallback did not complete: {}", status.message),
                                claude_zh_patch_payload(status, Vec::new()),
                            );
                        }
                        return complete_claude_zh_patch_install(result.message, status, Vec::new());
                    }
                    Ok(result) => {
                        let status = claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
                        return failed(
                            &format!("Claude manual Chinese patch elevated fallback failed: {}", result.message),
                            claude_zh_patch_payload(status, Vec::new()),
                        );
                    }
                    Err(elevation_error) => {
                        let status = claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
                        return failed(
                            &format!("Claude manual Chinese patch requires administrator approval, but fallback elevation failed: {elevation_error}; direct error: {error}"),
                            claude_zh_patch_payload(status, Vec::new()),
                        );
                    }
                }
            }
            let status = claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
            failed(
                &format!("Claude manual Chinese patch failed: {error}"),
                claude_zh_patch_payload(status, Vec::new()),
            )
        }
    }
}

#[tauri::command]
pub fn restore_claude_zh_patch() -> CommandResult<ClaudeZhPatchPayload> {
    log_manager_event("manager.claude_zh_patch.restore.start", json!({}));
    if !claude_codex_pro_core::claude_desktop::close_claude_desktop_for_patch() {
        log_manager_event(
            "manager.claude_zh_patch.restore.close_claude_failed",
            json!({}),
        );
        let status = claude_codex_pro_core::claude_zh_patch::detect_status();
        return failed(
            "Failed to close Claude Desktop before restore. Please exit Claude and retry.",
            claude_zh_patch_payload(status, Vec::new()),
        );
    }
    if claude_codex_pro_core::claude_zh_patch::detected_patch_needs_elevation() {
        log_manager_event(
            "manager.claude_zh_patch.restore.elevation_required",
            json!({}),
        );
        match restore_claude_zh_patch_elevated() {
            Ok(result) if result.status == "ok" => {
                let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                if status.status != "not_installed" {
                    return failed(
                        &format!(
                            "Claude official restore elevated run left patch residue: {}",
                            status.message
                        ),
                        claude_zh_patch_payload(status, Vec::new()),
                    );
                }
                return ok(
                    &format!("{} Restart Claude Desktop.", result.message),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
            Ok(result) => {
                let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                return failed(
                    &format!(
                        "Claude official restore elevated run failed: {}",
                        result.message
                    ),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
            Err(error) => {
                let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                return failed(
                    &format!(
                        "Claude official restore requires administrator approval, but elevation failed: {error}"
                    ),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
        }
    }
    log_manager_event("manager.claude_zh_patch.restore.direct.start", json!({}));
    match claude_codex_pro_core::claude_zh_patch::restore_patch() {
        Ok(outcome) => ok(
            "Claude official files restored from backup.",
            claude_zh_patch_payload(outcome.status, outcome.changed_files),
        ),
        Err(error) => {
            let status = claude_codex_pro_core::claude_zh_patch::detect_status();
            failed(
                &format!("Claude localization restore failed: {error}"),
                claude_zh_patch_payload(status, Vec::new()),
            )
        }
    }
}

#[tauri::command]
pub fn new_claude_desktop_chat() -> CommandResult<ClaudeDesktopActionPayload> {
    let result = claude_codex_pro_core::claude_desktop::new_claude_chat();
    let command_result = CommandResult {
        status: result.status,
        message: result.message,
        payload: ClaudeDesktopActionPayload {
            process_id: result.process_id,
            action: result.action,
            foreground_verified: result.foreground_verified,
            foreground_process_id: result.foreground_process_id,
            foreground_title: result.foreground_title,
            observed_window_titles: result.observed_window_titles,
        },
    };
    log_claude_desktop_command("new_chat", &command_result);
    command_result
}

#[tauri::command]
pub fn paste_claude_desktop_draft(
    request: ClaudeDesktopDraftRequest,
) -> CommandResult<ClaudeDesktopDraftPayload> {
    let result = claude_codex_pro_core::claude_desktop::paste_draft_to_claude(&request.text);
    let command_result = CommandResult {
        status: result.status,
        message: result.message,
        payload: ClaudeDesktopDraftPayload {
            process_id: result.process_id,
            action: result.action,
            input_chars: result.input_chars,
            auto_submitted: result.auto_submitted,
            foreground_verified: result.foreground_verified,
            foreground_process_id: result.foreground_process_id,
            foreground_title: result.foreground_title,
            observed_window_titles: result.observed_window_titles,
        },
    };
    log_claude_desktop_command("paste_draft", &command_result);
    command_result
}

#[tauri::command]
pub fn submit_claude_desktop_text(
    request: ClaudeDesktopDraftRequest,
) -> CommandResult<ClaudeDesktopDraftPayload> {
    let result = claude_codex_pro_core::claude_desktop::submit_text_to_claude(&request.text);
    let command_result = CommandResult {
        status: result.status,
        message: result.message,
        payload: ClaudeDesktopDraftPayload {
            process_id: result.process_id,
            action: result.action,
            input_chars: result.input_chars,
            auto_submitted: result.auto_submitted,
            foreground_verified: result.foreground_verified,
            foreground_process_id: result.foreground_process_id,
            foreground_title: result.foreground_title,
            observed_window_titles: result.observed_window_titles,
        },
    };
    log_claude_desktop_command("submit", &command_result);
    command_result
}

fn log_claude_desktop_command<T>(operation: &str, result: &CommandResult<T>)
where
    T: Serialize,
{
    let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
        &format!("manager.claude_desktop.{operation}"),
        result,
    );
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairConnectionPayload {
    pub target: String,
    pub frontend_injected: bool,
    pub backend_online: bool,
    pub codex_frontend_injected: bool,
    pub codex_backend_online: bool,
    pub claude_backend_online: bool,
    pub debug_port: Option<u16>,
    pub helper_port: Option<u16>,
    pub claude_proxy_port: Option<u16>,
    pub details: Vec<String>,
}

#[tauri::command]
pub async fn repair_frontend_connection() -> CommandResult<RepairConnectionPayload> {
    let mut details = Vec::new();
    let mut latest = StatusStore::default()
        .load_latest()
        .ok()
        .flatten()
        .map(refresh_launch_port_status);

    let initial_runtime_online = latest_renderer_runtime_heartbeat()
        .as_ref()
        .is_some_and(renderer_runtime_heartbeat_is_ready);

    if latest.as_ref().is_some_and(|status| {
        status.debug_port.is_some() && !status.debug_port_online && !initial_runtime_online
    }) {
        latest = restart_codex_for_frontend_repair(&mut details).await;
    }

    let codex_backend_online = latest
        .as_ref()
        .is_some_and(|status| status.helper_port_online);
    let runtime_heartbeat = latest_renderer_runtime_heartbeat();
    let runtime_online = runtime_heartbeat
        .as_ref()
        .is_some_and(renderer_runtime_heartbeat_is_ready);
    let codex_frontend_ok = if runtime_online {
        if let Some(heartbeat) = runtime_heartbeat.as_ref() {
            details.push(format!(
                "Codex 前端运行时心跳在线，最近上报于 {}。",
                heartbeat.timestamp_ms
            ));
        }
        true
    } else if let Some(status) = latest.as_ref() {
        match (status.debug_port, status.helper_port) {
            (Some(debug_port), Some(helper_port)) => {
                details.push(format!(
                    "Codex CDP 端口：{debug_port}，后端端口：{helper_port}"
                ));
                if !status.debug_port_online {
                    details.push(format!(
                        "Codex CDP 端口 127.0.0.1:{debug_port} 仍离线或 /json 不可用；已尝试自动重启，请确认 Codex 安装路径可用。"
                    ));
                    false
                } else if !status.helper_port_online {
                    details.push(format!(
                        "Codex 后端 127.0.0.1:{helper_port}/backend/status 未在线；请先点击“修复后端服务”。"
                    ));
                    false
                } else {
                    let reinjected = match tokio::time::timeout(
                        REPAIR_CODEX_FRONTEND_TIMEOUT,
                        claude_codex_pro_core::launcher::force_reinject_bridge(
                            debug_port,
                            helper_port,
                        ),
                    )
                    .await
                    {
                        Ok(value) => value,
                        Err(_) => {
                            details.push("Codex 前端桥接强制刷新超时。".to_string());
                            false
                        }
                    };
                    if reinjected {
                        details.push("Codex 前端桥接已刷新并注入最新脚本。".to_string());
                    } else {
                        details.push("Codex 前端桥接刷新未确认。".to_string());
                    }
                    reinjected
                }
            }
            _ => {
                details.push("最近一次 Codex 启动记录缺少 CDP 或后端端口。".to_string());
                false
            }
        }
    } else {
        details.push("未找到最近一次 Codex 启动记录。".to_string());
        false
    };

    let claude_proxy_port = cached_claude_desktop_proxy_port()
        .or_else(|| Some(current_claude_desktop_proxy_port_hint()));
    let claude_backend_online = claude_proxy_port.is_some_and(helper_backend_online);

    let frontend_injected = codex_frontend_ok;
    let status = if frontend_injected { "ok" } else { "failed" };
    CommandResult {
        status: status.to_string(),
        message: match status {
            "ok" => "Codex 前端连接已修复并确认注入。".to_string(),
            _ => "Codex 前端连接修复未确认可注入，请查看详情。".to_string(),
        },
        payload: RepairConnectionPayload {
            target: "codex".to_string(),
            frontend_injected,
            backend_online: codex_backend_online && claude_backend_online,
            codex_frontend_injected: codex_frontend_ok,
            codex_backend_online,
            claude_backend_online,
            debug_port: latest.as_ref().and_then(|status| status.debug_port),
            helper_port: latest.as_ref().and_then(|status| status.helper_port),
            claude_proxy_port,
            details,
        },
    }
}

async fn restart_codex_for_frontend_repair(details: &mut Vec<String>) -> Option<LaunchStatus> {
    details.push("检测到旧 Codex CDP 端口离线，正在自动重启 Codex 注入入口。".to_string());
    let Some(app_path) = current_codex_app_path_for_launch() else {
        details.push("未找到 Codex 应用路径，无法自动重启 Codex。".to_string());
        return StatusStore::default()
            .load_latest()
            .ok()
            .flatten()
            .map(refresh_launch_port_status);
    };

    claude_codex_pro_core::watcher::stop_launcher_processes_for_codex_restart();
    claude_codex_pro_core::watcher::stop_codex_processes();

    let request = LaunchRequest {
        app_path: app_path.to_string_lossy().to_string(),
        debug_port: default_debug_port(),
        helper_port: default_helper_port(),
    };
    if let Err(error) = spawn_silent_launcher(&request) {
        details.push(format!("自动重启 Codex 失败：{error}"));
        return StatusStore::default()
            .load_latest()
            .ok()
            .flatten()
            .map(refresh_launch_port_status);
    }

    let started = Instant::now();
    while started.elapsed() < REPAIR_CODEX_RESTART_TIMEOUT {
        if let Some(status) = StatusStore::default()
            .load_latest()
            .ok()
            .flatten()
            .map(refresh_launch_port_status)
        {
            if status.debug_port_online && status.helper_port_online {
                details.push(format!(
                    "已自动重启 Codex，CDP 端口 {debug_port} 与后端端口 {helper_port} 已上线。",
                    debug_port = status.debug_port.unwrap_or(request.debug_port),
                    helper_port = status.helper_port.unwrap_or(request.helper_port)
                ));
                return Some(status);
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    details.push("已发起 Codex 自动重启，但等待新 CDP / 后端端口上线超时。".to_string());
    StatusStore::default()
        .load_latest()
        .ok()
        .flatten()
        .map(refresh_launch_port_status)
}

#[tauri::command]
pub async fn repair_backend_service() -> CommandResult<RepairConnectionPayload> {
    let mut details = Vec::new();
    let helper_port = StatusStore::default()
        .load_latest()
        .ok()
        .flatten()
        .and_then(|status| status.helper_port)
        .unwrap_or_else(default_helper_port);
    let codex_helper = match claude_codex_pro_core::launcher::ensure_detached_helper(helper_port)
        .await
    {
        Ok(()) => {
            let online = wait_helper_backend_online(helper_port).await;
            details.push(if online {
                format!("Codex backend verified at 127.0.0.1:{helper_port}/backend/status.")
            } else {
                format!(
                    "Codex backend was requested, but 127.0.0.1:{helper_port}/backend/status did not respond yet."
                )
            });
            online
        }
        Err(error) => {
            details.push(format!("Codex backend failed to start: {error}"));
            false
        }
    };
    let mut claude_proxy_port = current_claude_desktop_proxy_port_hint();
    let claude_helper = match ensure_claude_desktop_proxy_helper().await {
        Ok(port) => {
            claude_proxy_port = port;
            let online = wait_helper_backend_online(port).await;
            details.push(if online {
                format!("Claude local model proxy verified at 127.0.0.1:{port}/backend/status.")
            } else {
                format!("Claude local model proxy was requested, but 127.0.0.1:{port}/backend/status did not respond yet.")
            });
            online
        }
        Err(error) => {
            details.push(format!("Claude local model proxy failed to start: {error}"));
            false
        }
    };
    let backend_online = codex_helper && claude_helper;
    let any_backend_online = codex_helper || claude_helper;
    let status = if backend_online {
        "ok"
    } else if any_backend_online {
        "degraded"
    } else {
        "failed"
    };
    CommandResult {
        status: status.to_string(),
        message: match status {
            "ok" => "Backend services are repaired; Codex and Claude frontends can reconnect."
                .to_string(),
            "degraded" => {
                "Backend services are partially repaired; check details for the offline side."
                    .to_string()
            }
            _ => "Backend service repair failed; check diagnostic logs.".to_string(),
        },
        payload: RepairConnectionPayload {
            target: "local_backends".to_string(),
            frontend_injected: false,
            backend_online,
            codex_frontend_injected: false,
            codex_backend_online: codex_helper,
            claude_backend_online: claude_helper,
            debug_port: None,
            helper_port: Some(helper_port),
            claude_proxy_port: Some(claude_proxy_port),
            details,
        },
    }
}

#[tauri::command]
pub async fn refresh_claude_third_party_config()
-> CommandResult<ClaudeDesktopDevModeConfigurePayload> {
    let proxy_port = match ensure_claude_desktop_proxy_helper().await {
        Ok(port) => port,
        Err(error) => {
            return failed(
                &format!("刷新 Claude 第三方配置失败：本地模型代理启动失败：{error}"),
                ClaudeDesktopDevModeConfigurePayload {
                    outcome: ClaudeDesktopDevModeOutcome {
                        configured: false,
                        normal_config_path: String::new(),
                        threep_config_path: String::new(),
                        profile_path: String::new(),
                        profile_meta_path: String::new(),
                        backup_paths: Vec::new(),
                        message: error.to_string(),
                    },
                    dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
                },
            );
        }
    };
    match plugin_hub::configure_claude_desktop_dev_mode_with_proxy_port(None, proxy_port) {
        Ok(outcome) => {
            let helper_message = if wait_helper_backend_online(proxy_port).await {
                format!("本地模型代理 127.0.0.1:{proxy_port} 已验证在线。")
            } else {
                format!(
                    "本地模型代理已请求使用 127.0.0.1:{proxy_port}，但 /backend/status 暂未响应。"
                )
            };
            let status = plugin_hub::load_claude_desktop_dev_mode_status();
            ok(
                &format!("Claude 第三方配置已刷新；{helper_message}"),
                ClaudeDesktopDevModeConfigurePayload {
                    outcome,
                    dev_mode_status: status,
                },
            )
        }
        Err(error) => failed(
            &format!("刷新 Claude 第三方配置失败：{error}"),
            ClaudeDesktopDevModeConfigurePayload {
                outcome: ClaudeDesktopDevModeOutcome {
                    configured: false,
                    normal_config_path: String::new(),
                    threep_config_path: String::new(),
                    profile_path: String::new(),
                    profile_meta_path: String::new(),
                    backup_paths: Vec::new(),
                    message: error.to_string(),
                },
                dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
            },
        ),
    }
}

#[tauri::command]
pub fn launch_claude_codex_pro(request: LaunchRequest) -> CommandResult<Value> {
    let request = normalize_launch_request(request);
    spawn_claude_codex_pro_launch(request, "Launch task started in the background.")
}

#[tauri::command]
pub fn restart_claude_codex_pro(request: LaunchRequest) -> CommandResult<Value> {
    let request = normalize_launch_request(request);
    claude_codex_pro_core::watcher::stop_launcher_processes();
    claude_codex_pro_core::watcher::stop_codex_processes();
    spawn_claude_codex_pro_launch(request, "Codex restart task is running in the background.")
}

fn normalize_launch_request(mut request: LaunchRequest) -> LaunchRequest {
    if request.app_path.trim().is_empty() {
        if let Some(path) = current_codex_app_path_for_launch() {
            request.app_path = path.to_string_lossy().to_string();
        }
    }
    request
}

fn current_codex_app_path_for_launch() -> Option<PathBuf> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    StatusStore::default()
        .load_latest()
        .ok()
        .flatten()
        .and_then(|status| status.codex_app)
        .and_then(|path| {
            claude_codex_pro_core::app_paths::normalize_codex_app_path(Path::new(&path))
        })
        .or_else(|| claude_codex_pro_core::app_paths::find_running_codex_app_dir())
        .or_else(|| {
            claude_codex_pro_core::app_paths::resolve_codex_app_dir_with_saved(
                None,
                Some(settings.codex_app_path.as_str()),
            )
        })
}

fn spawn_claude_codex_pro_launch(
    request: LaunchRequest,
    accepted_message: &str,
) -> CommandResult<Value> {
    let debug_port = request.debug_port;
    let helper_port = request.helper_port;
    let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
        "manager.launch_requested",
        json!({
            "debug_port": debug_port,
            "helper_port": helper_port,
            "app_path": request.app_path.trim()
        }),
    );
    match spawn_silent_launcher(&request) {
        Ok(()) => CommandResult {
            status: "accepted".to_string(),
            message: accepted_message.to_string(),
            payload: json!({
                "debugPort": debug_port,
                "helperPort": helper_port
            }),
        },
        Err(error) => failed(
            &format!("找不到静默启动器或启动失败：{error}"),
            json!({
                "debugPort": debug_port,
                "helperPort": helper_port
            }),
        ),
    }
}

fn spawn_silent_launcher(request: &LaunchRequest) -> anyhow::Result<()> {
    let launcher = resolve_silent_launcher_path()?;
    let mut command = std::process::Command::new(&launcher);
    if !request.app_path.trim().is_empty() {
        command.arg("--app-path").arg(request.app_path.trim());
    }
    command
        .arg("--debug-port")
        .arg(request.debug_port.to_string())
        .arg("--helper-port")
        .arg(request.helper_port.to_string());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| anyhow::anyhow!("Failed to start {}: {error}", launcher.to_string_lossy()))
}

pub fn resolve_silent_launcher_path() -> anyhow::Result<PathBuf> {
    let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let companion =
        claude_codex_pro_core::install::companion_binary_path_from_exe(&current_exe, SILENT_BINARY);
    if companion.is_file() {
        return Ok(companion);
    }

    let exe_dir = current_exe.parent().unwrap_or_else(|| Path::new("."));
    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let launcher_name = format!("{SILENT_BINARY}{exe_suffix}");
    let mut candidates = vec![
        exe_dir.join(&launcher_name),
        PathBuf::from("target").join("debug").join(&launcher_name),
        PathBuf::from("target").join("release").join(&launcher_name),
    ];
    if let Some(profile_dir) = exe_dir.parent() {
        candidates.push(profile_dir.join("debug").join(&launcher_name));
        candidates.push(profile_dir.join("release").join(&launcher_name));
        if let Some(target_dir) = profile_dir.parent() {
            candidates.push(target_dir.join("debug").join(&launcher_name));
            candidates.push(target_dir.join("release").join(&launcher_name));
        }
    }

    candidates.sort();
    candidates.dedup();

    if let Some(path) = candidates.iter().find(|path| path.is_file()).cloned() {
        return Ok(path);
    }

    let searched = candidates
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("; ");
    bail!("Silent launcher {launcher_name} was not found; searched: {searched}")
}

#[tauri::command]
pub fn load_settings() -> CommandResult<SettingsPayload> {
    settings_payload("Settings loaded.", "Failed to load settings.")
}

#[tauri::command]
pub fn save_settings(settings: BackendSettings) -> CommandResult<SettingsPayload> {
    let settings = normalize_settings_before_save(settings);
    match SettingsStore::default().save(&settings) {
        Ok(()) => {
            let wrapper_message = refresh_cli_wrapper_after_settings_save(&settings);
            settings_payload(
                &format!("Settings saved.{wrapper_message}"),
                "Failed to reload settings after save.",
            )
        }
        Err(error) => failed(
            &format!("Failed to save settings: {error}"),
            SettingsPayload {
                settings,
                settings_path: claude_codex_pro_core::paths::default_settings_path()
                    .to_string_lossy()
                    .to_string(),
                user_scripts: user_script_inventory(),
            },
        ),
    }
}

#[tauri::command]
pub fn list_local_sessions() -> CommandResult<LocalSessionsPayload> {
    let home = claude_codex_pro_core::codex_sqlite::default_codex_home_dir();
    let db_paths = claude_codex_pro_core::codex_sqlite::codex_session_db_paths_from_home(&home);
    let mut sessions = Vec::new();
    let mut errors = Vec::new();
    for db_path in &db_paths {
        let adapter = local_session_adapter(db_path);
        match adapter.list_local_sessions() {
            Ok(mut items) => sessions.append(&mut items),
            Err(error) if db_path.exists() => {
                errors.push(format!("{}: {error}", db_path.to_string_lossy()));
            }
            Err(_) => {}
        }
    }
    sessions.sort_by(|left, right| {
        right
            .updated_at_ms
            .cmp(&left.updated_at_ms)
            .then_with(|| right.id.cmp(&left.id))
    });
    let mut seen_session_ids = std::collections::HashSet::new();
    sessions.retain(|session| seen_session_ids.insert(session.id.clone()));
    let payload = LocalSessionsPayload {
        db_path: db_paths
            .first()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        db_paths: db_paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
        sessions,
    };
    if errors.is_empty() {
        ok(
            &format!("Loaded {} local sessions.", payload.sessions.len()),
            payload,
        )
    } else {
        failed(
            &format!("Failed to read some local sessions: {}", errors.join("; ")),
            payload,
        )
    }
}

#[tauri::command]
pub fn load_memory_assist_status() -> CommandResult<MemoryAssistStatusPayload> {
    match MemoryAssistStore::default().status() {
        Ok(memory) => ok(
            "Pangu memory status loaded.",
            MemoryAssistStatusPayload {
                memory: enrich_memory_status(memory),
            },
        ),
        Err(error) => failed(
            &format!("Failed to load Pangu memory status: {error}"),
            MemoryAssistStatusPayload {
                memory: empty_memory_status(),
            },
        ),
    }
}

#[tauri::command]
pub fn query_memory_assist(request: MemoryQueryRequest) -> CommandResult<MemoryAssistQueryPayload> {
    match MemoryAssistStore::default().query(request.clone()) {
        Ok(memory) => ok(
            "Memory query completed.",
            MemoryAssistQueryPayload { memory },
        ),
        Err(error) => failed(
            &format!("Memory query failed: {error}"),
            MemoryAssistQueryPayload {
                memory: MemoryQueryResult {
                    query: request.query,
                    workspace: request.workspace,
                    results: Vec::new(),
                },
            },
        ),
    }
}

#[tauri::command]
pub fn list_memory_assist_items(
    request: MemoryQueryRequest,
) -> CommandResult<MemoryAssistItemsPayload> {
    match MemoryAssistStore::default().list_items(request) {
        Ok(items) => ok(
            &format!("Loaded {} memory items.", items.len()),
            MemoryAssistItemsPayload { items },
        ),
        Err(error) => failed(
            &format!("Failed to load memory list: {error}"),
            MemoryAssistItemsPayload { items: Vec::new() },
        ),
    }
}

#[tauri::command]
pub fn learn_memory_assist_item(
    request: MemoryItemRequest,
) -> CommandResult<MemoryAssistItemPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "Pangu memory is disabled.",
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        );
    }
    match MemoryAssistStore::default().learn_item(request) {
        Ok(item) => ok("Memory saved.", MemoryAssistItemPayload { item }),
        Err(error) => failed(
            &format!("Failed to save memory: {error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
    }
}

#[tauri::command]
pub fn update_memory_assist_item(
    request: MemoryIdAndItemRequest,
) -> CommandResult<MemoryAssistItemPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "Pangu memory is disabled.",
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        );
    }
    match MemoryAssistStore::default().update_item(&request.id, request.item) {
        Ok(item) => ok("Memory updated.", MemoryAssistItemPayload { item }),
        Err(error) => failed(
            &format!("Failed to update memory: {error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
    }
}

#[tauri::command]
pub fn delete_memory_assist_item(
    request: MemoryIdRequest,
) -> CommandResult<MemoryAssistItemPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "Pangu memory is disabled.",
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        );
    }
    match MemoryAssistStore::default().delete_item(&request.id) {
        Ok(item) => ok("Memory deleted.", MemoryAssistItemPayload { item }),
        Err(error) => failed(
            &format!("Failed to delete memory: {error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
    }
}

#[tauri::command]
pub fn create_memory_assist_candidate(
    request: MemoryCandidateRequest,
) -> CommandResult<MemoryAssistCandidatePayload> {
    if !memory_assist_candidate_enabled() {
        return failed(
            "Pangu memory auto learning is disabled.",
            MemoryAssistCandidatePayload {
                candidate: empty_memory_candidate(),
            },
        );
    }
    match MemoryAssistStore::default().create_candidate(request) {
        Ok(candidate) => ok(
            "Pending memory created.",
            MemoryAssistCandidatePayload { candidate },
        ),
        Err(error) => failed(
            &format!("Failed to create pending memory: {error}"),
            MemoryAssistCandidatePayload {
                candidate: empty_memory_candidate(),
            },
        ),
    }
}

#[tauri::command]
pub fn list_memory_assist_candidates(
    request: MemoryCandidateListRequest,
) -> CommandResult<MemoryAssistCandidatesPayload> {
    match MemoryAssistStore::default().list_candidates(&request.workspace, request.include_global) {
        Ok(candidates) => ok(
            &format!("Loaded {} pending memories.", candidates.len()),
            MemoryAssistCandidatesPayload { candidates },
        ),
        Err(error) => failed(
            &format!("Failed to load pending memories: {error}"),
            MemoryAssistCandidatesPayload {
                candidates: Vec::new(),
            },
        ),
    }
}

#[tauri::command]
pub fn approve_memory_assist_candidate(
    request: MemoryIdRequest,
) -> CommandResult<MemoryAssistItemPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "Pangu memory is disabled.",
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        );
    }
    match MemoryAssistStore::default().approve_candidate(&request.id) {
        Ok(item) => ok("Pending memory approved.", MemoryAssistItemPayload { item }),
        Err(error) => failed(
            &format!("Failed to approve pending memory: {error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
    }
}

#[tauri::command]
pub fn reject_memory_assist_candidate(
    request: MemoryIdRequest,
) -> CommandResult<MemoryAssistCandidatePayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "Pangu memory is disabled.",
            MemoryAssistCandidatePayload {
                candidate: empty_memory_candidate(),
            },
        );
    }
    match MemoryAssistStore::default().reject_candidate(&request.id) {
        Ok(candidate) => ok(
            "Pending memory rejected.",
            MemoryAssistCandidatePayload { candidate },
        ),
        Err(error) => failed(
            &format!("Failed to reject pending memory: {error}"),
            MemoryAssistCandidatePayload {
                candidate: empty_memory_candidate(),
            },
        ),
    }
}

#[tauri::command]
pub fn load_memory_assist_session(
    request: MemorySessionRequest,
) -> CommandResult<MemoryAssistSessionPayload> {
    match MemoryAssistStore::default().session_summary(request) {
        Ok(summary) => ok(
            "Memory session summary loaded.",
            MemoryAssistSessionPayload { summary },
        ),
        Err(error) => failed(
            &format!("Failed to load memory session summary: {error}"),
            MemoryAssistSessionPayload {
                summary: MemorySessionSummary {
                    workspace: String::new(),
                    total_items: 0,
                    pending_candidates: 0,
                    injected_items: Vec::new(),
                    summary: String::new(),
                },
            },
        ),
    }
}

#[tauri::command]
pub fn run_memory_assist_selfcheck(
    request: MemorySelfCheckRequest,
) -> CommandResult<MemoryAssistSelfCheckPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "Pangu memory is disabled.",
            MemoryAssistSelfCheckPayload {
                report: MemorySelfCheckResult {
                    status: "failed".to_string(),
                    repaired: false,
                    backup_path: None,
                    checks: Vec::new(),
                },
            },
        );
    }
    match MemoryAssistStore::default().run_selfcheck(request) {
        Ok(report) => ok(
            "Pangu memory self-check completed.",
            MemoryAssistSelfCheckPayload { report },
        ),
        Err(error) => failed(
            &format!("Pangu memory self-check failed: {error}"),
            MemoryAssistSelfCheckPayload {
                report: MemorySelfCheckResult {
                    status: "failed".to_string(),
                    repaired: false,
                    backup_path: None,
                    checks: Vec::new(),
                },
            },
        ),
    }
}

#[tauri::command]
pub fn export_memory_assist() -> CommandResult<MemoryAssistExportPayload> {
    match MemoryAssistStore::default().export_json() {
        Ok(data) => ok(
            "Pangu memory data exported.",
            MemoryAssistExportPayload { data },
        ),
        Err(error) => failed(
            &format!("Pangu memory export failed: {error}"),
            MemoryAssistExportPayload {
                data: MemoryExport {
                    schema_version: "memory-assist/v1".to_string(),
                    exported_at: 0,
                    items: Vec::new(),
                    candidates: Vec::new(),
                },
            },
        ),
    }
}

#[tauri::command]
pub fn import_memory_assist(
    request: MemoryImportRequest,
) -> CommandResult<MemoryAssistStatusPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "Pangu memory is disabled.",
            MemoryAssistStatusPayload {
                memory: empty_memory_status(),
            },
        );
    }
    match MemoryAssistStore::default().import_json(request) {
        Ok(memory) => ok(
            "Pangu memory data imported.",
            MemoryAssistStatusPayload { memory },
        ),
        Err(error) => failed(
            &format!("Pangu memory import failed: {error}"),
            MemoryAssistStatusPayload {
                memory: empty_memory_status(),
            },
        ),
    }
}

fn empty_memory_status() -> MemoryAssistStatus {
    MemoryAssistStatus {
        status: "failed".to_string(),
        db_path: claude_codex_pro_core::memory_assist::default_memory_assist_db_path()
            .to_string_lossy()
            .to_string(),
        total_items: 0,
        pending_candidates: 0,
        workspaces: Vec::new(),
        latest_backup_path: None,
        enabled: false,
        inject_enabled: false,
        auto_suggest_enabled: false,
        runtime_status: "failed".to_string(),
        runtime_message: "Pangu memory is unavailable.".to_string(),
        codex_injected: false,
        claude_injected: false,
        codex_workspace: String::new(),
        active: false,
        active_source: "idle".to_string(),
    }
}

fn empty_memory_item() -> MemoryItem {
    MemoryItem {
        id: String::new(),
        text: String::new(),
        workspace: String::new(),
        category: String::new(),
        tags: Vec::new(),
        source: String::new(),
        source_session_id: String::new(),
        created_at: 0,
        updated_at: 0,
        last_accessed_at: 0,
        access_count: 0,
    }
}

fn empty_memory_candidate() -> MemoryCandidate {
    MemoryCandidate {
        id: String::new(),
        text: String::new(),
        workspace: String::new(),
        category: String::new(),
        tags: Vec::new(),
        source: String::new(),
        reason: String::new(),
        source_session_id: String::new(),
        status: "failed".to_string(),
        created_at: 0,
        updated_at: 0,
    }
}

fn memory_assist_write_enabled() -> bool {
    let settings = SettingsStore::default().load().unwrap_or_default();
    settings.memory_assist_enabled
}

fn memory_assist_candidate_enabled() -> bool {
    let settings = SettingsStore::default().load().unwrap_or_default();
    settings.memory_assist_enabled && settings.memory_assist_auto_suggest_enabled
}

fn enrich_memory_status(mut memory: MemoryAssistStatus) -> MemoryAssistStatus {
    let settings = SettingsStore::default().load().unwrap_or_default();
    memory.enabled = settings.memory_assist_enabled;
    memory.inject_enabled = settings.memory_assist_inject_enabled;
    memory.auto_suggest_enabled = settings.memory_assist_auto_suggest_enabled;

    let heartbeat = latest_renderer_runtime_heartbeat();
    let heartbeat_is_fresh = heartbeat
        .as_ref()
        .is_some_and(|item| renderer_heartbeat_is_fresh(item.timestamp_ms));
    let runtime_snapshot = read_codex_memory_runtime_snapshot().or_else(|| {
        heartbeat
            .filter(|_| heartbeat_is_fresh)
            .and_then(|item| item.runtime)
    });

    if let Some(runtime) = runtime_snapshot {
        memory.runtime_status = if runtime.injected {
            runtime.status.clone()
        } else if memory.enabled && memory.inject_enabled {
            "waiting".to_string()
        } else {
            "disabled".to_string()
        };
        memory.runtime_message = if runtime.summary.trim().is_empty() {
            "Pangu memory runtime synchronized.".to_string()
        } else {
            runtime.summary.clone()
        };
        memory.codex_injected = runtime.injected;
        memory.codex_workspace = runtime.workspace.clone();
        memory.active = runtime.active || heartbeat_is_fresh;
        memory.active_source = if runtime.source.trim().is_empty() {
            "codex".to_string()
        } else {
            runtime.source.clone()
        };
        if runtime.total_items > 0 {
            memory.total_items = runtime.total_items;
        }
        if runtime.pending_candidates > 0 {
            memory.pending_candidates = runtime.pending_candidates;
        }
    } else {
        memory.runtime_status = if memory.enabled && memory.inject_enabled {
            "not_checked".to_string()
        } else {
            "disabled".to_string()
        };
        memory.runtime_message = if memory.enabled && memory.inject_enabled {
            "Waiting for Codex memory runtime injection.".to_string()
        } else {
            "Pangu memory is currently disabled.".to_string()
        };
    }

    memory.claude_injected = false;
    memory
}

fn latest_renderer_runtime_heartbeat() -> Option<RendererRuntimeHeartbeat> {
    let path = claude_codex_pro_core::diagnostic_log::diagnostic_log_path();
    let text = fs::read_to_string(path).ok()?;
    let mut fallback: Option<RendererRuntimeHeartbeat> = None;
    for record in text
        .lines()
        .rev()
        .take(240)
        .filter_map(|line| serde_json::from_str::<DiagnosticLogRecord>(line).ok())
        .filter(|record| {
            record.event == "renderer.memory_runtime" || record.event == "renderer.script_loaded"
        })
    {
        if record.event == "renderer.memory_runtime" {
            let runtime = record
                .detail
                .get("detail")
                .and_then(|detail| detail.get("runtime"))
                .cloned()
                .and_then(|value| {
                    serde_json::from_value::<MemoryAssistRuntimeSnapshot>(value).ok()
                });
            return Some(RendererRuntimeHeartbeat {
                timestamp_ms: record.timestamp_ms,
                runtime,
                runtime_reported: true,
            });
        }
        if fallback.is_none() {
            fallback = Some(RendererRuntimeHeartbeat {
                timestamp_ms: record.timestamp_ms,
                runtime: None,
                runtime_reported: false,
            });
        }
    }
    fallback
}

fn renderer_heartbeat_is_fresh(timestamp_ms: u64) -> bool {
    current_time_ms().saturating_sub(timestamp_ms) <= 45_000
}

fn renderer_runtime_heartbeat_is_ready(heartbeat: &RendererRuntimeHeartbeat) -> bool {
    heartbeat.runtime_reported
        && renderer_heartbeat_is_fresh(heartbeat.timestamp_ms)
        && heartbeat
            .runtime
            .as_ref()
            .map(|runtime| runtime.status != "failed")
            .unwrap_or(true)
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn read_codex_memory_runtime_snapshot() -> Option<MemoryAssistRuntimeSnapshot> {
    let latest = StatusStore::default().load_latest().ok().flatten()?;
    let debug_port = latest.debug_port?;
    let targets =
        tauri::async_runtime::block_on(claude_codex_pro_core::cdp::list_targets(debug_port))
            .ok()?;
    let target = claude_codex_pro_core::cdp::pick_injectable_codex_page_target(&targets).ok()?;
    let websocket_url = target.web_socket_debugger_url.as_deref()?;
    let result = tauri::async_runtime::block_on(claude_codex_pro_core::bridge::evaluate_script(
        websocket_url,
        r#"(() => window.__claudeCodexProMemoryAssistRuntime || null)()"#,
    ))
    .ok()?;
    let value = result
        .get("result")
        .and_then(|result| result.get("result"))
        .and_then(|result| result.get("value"))?
        .clone();
    serde_json::from_value::<MemoryAssistRuntimeSnapshot>(value).ok()
}

#[tauri::command]
pub fn list_zed_remote_projects() -> CommandResult<ZedRemoteProjectsPayload> {
    let result = claude_codex_pro_core::zed_remote::list_zed_remote_projects_response(&json!({}));
    if result.get("status").and_then(Value::as_str) == Some("ok") {
        let projects = serde_json::from_value::<Vec<ZedRemoteProject>>(
            result
                .get("projects")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
        )
        .unwrap_or_default();
        return ok(
            &format!("Loaded {} Zed remote projects.", projects.len()),
            ZedRemoteProjectsPayload { projects },
        );
    }
    failed(
        result
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Failed to load Zed remote projects."),
        ZedRemoteProjectsPayload {
            projects: Vec::new(),
        },
    )
}

#[tauri::command]
pub fn open_zed_remote(payload: Value) -> CommandResult<ZedRemoteOpenPayload> {
    let result = claude_codex_pro_core::zed_remote::open_zed_remote(&payload);
    let strategy = result
        .get("strategy")
        .cloned()
        .and_then(|value| serde_json::from_value::<ZedOpenStrategy>(value).ok())
        .unwrap_or_default();
    let url = result
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if result.get("status").and_then(Value::as_str) == Some("ok") {
        return ok(
            "Zed Remote link opened.",
            ZedRemoteOpenPayload { url, strategy },
        );
    }
    failed(
        result
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Failed to open Zed Remote link."),
        ZedRemoteOpenPayload { url, strategy },
    )
}

#[tauri::command]
pub fn forget_zed_remote_project(id: String) -> CommandResult<ZedRemoteProjectsPayload> {
    let result =
        claude_codex_pro_core::zed_remote::forget_zed_remote_project_response(&json!({ "id": id }));
    if result.get("status").and_then(Value::as_str) != Some("ok") {
        return failed(
            result
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Failed to forget Zed remote project."),
            ZedRemoteProjectsPayload {
                projects: Vec::new(),
            },
        );
    }
    list_zed_remote_projects()
}

#[tauri::command]
pub fn delete_local_session(request: DeleteLocalSessionRequest) -> CommandResult<DeleteResult> {
    let session_id = request.session_id.trim();
    if session_id.is_empty() {
        return failed(
            "Session ID cannot be empty.",
            DeleteResult {
                status: claude_codex_pro_core::models::DeleteStatus::Failed,
                session_id: String::new(),
                message: "Session ID cannot be empty.".to_string(),
                undo_token: None,
                backup_path: None,
            },
        );
    }
    let session = SessionRef {
        session_id: session_id.to_string(),
        title: request.title,
    };
    let mut candidate_paths = Vec::new();
    if let Some(path) = request.db_path.as_deref() {
        let path = PathBuf::from(path);
        if !candidate_paths.iter().any(|candidate| candidate == &path) {
            candidate_paths.push(path);
        }
    }
    for path in claude_codex_pro_core::codex_sqlite::codex_session_db_paths_from_home(
        &claude_codex_pro_core::codex_sqlite::default_codex_home_dir(),
    ) {
        if !candidate_paths.iter().any(|candidate| candidate == &path) {
            candidate_paths.push(path);
        }
    }
    log_manager_event(
        "manager.delete_local_session.start",
        json!({
            "session_id": session_id,
            "title": session.title,
            "requested_db_path": request.db_path,
            "candidate_paths": candidate_paths
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
        }),
    );
    let result = claude_codex_pro_data::delete_local_from_paths(
        candidate_paths.clone(),
        claude_codex_pro_data::BackupStore::new(
            claude_codex_pro_core::paths::default_app_state_dir().join("backups"),
        ),
        &session,
    );
    log_manager_event(
        "manager.delete_local_session.finish",
        json!({
            "session_id": session_id,
            "final_status": format!("{:?}", result.status),
            "final_message": result.message,
            "candidate_paths": candidate_paths
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
        }),
    );
    let status = if matches!(
        result.status,
        claude_codex_pro_core::models::DeleteStatus::LocalDeleted
    ) {
        "ok"
    } else {
        "failed"
    };
    CommandResult {
        status: status.to_string(),
        message: result.message.clone(),
        payload: result,
    }
}

fn local_session_adapter(db_path: &Path) -> claude_codex_pro_data::SQLiteStorageAdapter {
    claude_codex_pro_data::SQLiteStorageAdapter::new(
        db_path,
        claude_codex_pro_data::BackupStore::new(
            claude_codex_pro_core::paths::default_app_state_dir().join("backups"),
        ),
    )
}

fn normalize_settings_before_save(mut settings: BackendSettings) -> BackendSettings {
    if let Some(path) = claude_codex_pro_core::app_paths::normalize_codex_app_path(Path::new(
        &settings.codex_app_path,
    )) {
        settings.codex_app_path = path.to_string_lossy().to_string();
    }
    settings.relay_common_config_contents =
        claude_codex_pro_core::relay_config::sanitize_common_config_contents(
            &settings.relay_common_config_contents,
        );
    let (common_without_context, extracted_context) =
        split_relay_context_config_sections(&settings.relay_common_config_contents);
    settings.relay_common_config_contents = common_without_context;
    settings.relay_context_config_contents =
        relay_join_config_sections(&[&settings.relay_context_config_contents, &extracted_context]);
    settings.relay_context_config_contents =
        claude_codex_pro_core::relay_config::sanitize_common_config_contents(
            &settings.relay_context_config_contents,
        );
    for profile in &mut settings.relay_profiles {
        if let Err(error) =
            claude_codex_pro_core::relay_config::normalize_relay_profile_for_storage(profile)
        {
            log_manager_event(
                "manager.normalize_relay_profile_for_storage.failed",
                json!({
                    "profileId": profile.id,
                    "profileName": profile.name,
                    "error": error.to_string()
                }),
            );
        }
    }
    let common_config = relay_combined_common_config(&settings);
    if !common_config.trim().is_empty() {
        for profile in &mut settings.relay_profiles {
            if !profile.use_common_config || profile.config_contents.trim().is_empty() {
                continue;
            }
            match claude_codex_pro_core::relay_config::strip_common_config_from_config(
                &profile.config_contents,
                &common_config,
            ) {
                Ok(stripped) => {
                    profile.config_contents =
                        strip_common_config_text_fallback(&stripped, &common_config);
                }
                Err(_) => {
                    profile.config_contents =
                        strip_common_config_text_fallback(&profile.config_contents, &common_config);
                }
            }
        }
    }
    settings.provider_sync_saved_providers =
        normalize_provider_sync_provider_list(settings.provider_sync_saved_providers);
    settings.provider_sync_manual_providers =
        normalize_provider_sync_provider_list(settings.provider_sync_manual_providers);
    settings.provider_sync_last_selected_provider = settings
        .provider_sync_last_selected_provider
        .trim()
        .to_string();
    settings
}

fn normalize_provider_sync_provider_list(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            result.push(trimmed.to_string());
        }
    }
    result.sort();
    result
}

fn relay_combined_common_config(settings: &BackendSettings) -> String {
    relay_join_config_sections(&[
        &settings.relay_common_config_contents,
        &settings.relay_context_config_contents,
    ])
}

fn relay_join_config_sections(sections: &[&str]) -> String {
    let sections = sections
        .iter()
        .map(|section| section.trim())
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>();
    if sections.is_empty() {
        String::new()
    } else {
        claude_codex_pro_core::relay_config::normalize_config_text(&format!(
            "{}\n",
            sections.join("\n\n")
        ))
    }
}

fn split_relay_context_config_sections(config: &str) -> (String, String) {
    let mut common = Vec::new();
    let mut context = Vec::new();
    let mut in_context_table = false;

    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_context_table = trimmed.starts_with("[mcp_servers.")
                || trimmed.starts_with("[skills.")
                || trimmed.starts_with("[plugins.");
        }
        if in_context_table {
            context.push(line);
        } else {
            common.push(line);
        }
    }

    (
        relay_join_config_sections(&[&common.join("\n")]),
        relay_join_config_sections(&[&context.join("\n")]),
    )
}

fn strip_common_config_text_fallback(config_contents: &str, common_config: &str) -> String {
    let common = common_config_anchors(common_config);
    if common.root_keys.is_empty() && common.table_headers.is_empty() {
        return ensure_text_newline(config_contents.trim_end());
    }

    let mut kept = Vec::new();
    let mut skipping_table = false;

    for line in config_contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let header = trimmed.to_string();
            skipping_table = common.table_headers.contains(&header);
            if skipping_table {
                continue;
            }
        }

        if skipping_table {
            continue;
        }

        if let Some(key) = toml_key_from_line(trimmed) {
            if common.root_keys.contains(key) {
                continue;
            }
        }

        kept.push(line);
    }

    ensure_text_newline(kept.join("\n").trim_end())
}

struct CommonConfigAnchors {
    root_keys: std::collections::HashSet<String>,
    table_headers: std::collections::HashSet<String>,
}

fn common_config_anchors(common_config: &str) -> CommonConfigAnchors {
    let mut root_keys = std::collections::HashSet::new();
    let mut table_headers = std::collections::HashSet::new();
    let mut in_table = false;

    for line in common_config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_table = true;
            table_headers.insert(trimmed.to_string());
            continue;
        }
        if !in_table {
            if let Some(key) = toml_key_from_line(trimmed) {
                root_keys.insert(key.to_string());
            }
        }
    }

    CommonConfigAnchors {
        root_keys,
        table_headers,
    }
}

fn toml_key_from_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (key, _) = trimmed.split_once('=')?;
    let key = key.trim();
    if key.is_empty() { None } else { Some(key) }
}

fn ensure_text_newline(value: &str) -> String {
    if value.trim().is_empty() {
        String::new()
    } else {
        format!("{}\n", value.trim_end())
    }
}

#[tauri::command]
pub async fn load_provider_sync_targets() -> CommandResult<Value> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    let result = tauri::async_runtime::spawn_blocking(|| {
        claude_codex_pro_data::load_provider_sync_targets(None)
    })
    .await
    .map_err(|error| anyhow::anyhow!("provider target discovery task failed: {error}"));
    match result {
        Ok(mut targets) => {
            let manual = settings
                .provider_sync_manual_providers
                .iter()
                .chain(settings.provider_sync_saved_providers.iter())
                .filter_map(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect::<Vec<_>>();
            merge_manual_provider_sync_targets(&mut targets, &manual, &settings);
            ok(
                "Provider sync targets loaded.",
                serde_json::to_value(targets).unwrap_or_else(|_| json!({})),
            )
        }
        Err(error) => failed(
            &format!("Provider sync targets failed to load: {error}"),
            json!({}),
        ),
    }
}

fn merge_manual_provider_sync_targets(
    targets: &mut claude_codex_pro_data::ProviderSyncTargetList,
    manual: &[String],
    settings: &BackendSettings,
) {
    for id in manual {
        if let Some(existing) = targets.targets.iter_mut().find(|target| target.id == *id) {
            if !existing
                .sources
                .contains(&claude_codex_pro_data::ProviderSyncTargetSource::Manual)
            {
                existing
                    .sources
                    .push(claude_codex_pro_data::ProviderSyncTargetSource::Manual);
                existing.sources.sort();
            }
            existing.is_manual = settings.provider_sync_manual_providers.contains(id);
            existing.is_saved = settings.provider_sync_saved_providers.contains(id);
        } else {
            targets
                .targets
                .push(claude_codex_pro_data::ProviderSyncTargetOption {
                    id: id.clone(),
                    sources: vec![claude_codex_pro_data::ProviderSyncTargetSource::Manual],
                    is_current_provider: *id == targets.current_provider,
                    is_manual: settings.provider_sync_manual_providers.contains(id),
                    is_saved: settings.provider_sync_saved_providers.contains(id),
                });
        }
    }
    targets.targets.sort_by(|left, right| {
        right
            .is_current_provider
            .cmp(&left.is_current_provider)
            .then_with(|| left.id.cmp(&right.id))
    });
}

#[tauri::command]
pub async fn sync_providers_now(target_provider: Option<String>) -> CommandResult<Value> {
    let target_provider = target_provider
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let target_for_settings = target_provider.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        claude_codex_pro_data::run_provider_sync_with_target(None, target_provider.as_deref())
    })
    .await
    .map_err(|error| anyhow::anyhow!("provider sync task failed: {error}"));
    match result {
        Ok(sync) => {
            if is_success_sync_status(&sync.status) {
                persist_provider_sync_selection(
                    target_for_settings
                        .as_deref()
                        .unwrap_or(&sync.target_provider),
                );
            }
            ok(
                &format!(
                    "Provider synced once: {} session files, {} sqlite rows, {} locked files skipped.",
                    sync.changed_session_files,
                    sync.sqlite_rows_updated,
                    sync.skipped_locked_rollout_files.len()
                ),
                json!({
                    "syncStatus": sync.status,
                    "targetProvider": sync.target_provider,
                    "changedSessionFiles": sync.changed_session_files,
                    "skippedLockedRolloutFiles": sync.skipped_locked_rollout_files,
                    "sqliteRowsUpdated": sync.sqlite_rows_updated,
                    "sqliteProviderRowsUpdated": sync.sqlite_provider_rows_updated,
                    "sqliteUserEventRowsUpdated": sync.sqlite_user_event_rows_updated,
                    "sqliteCwdRowsUpdated": sync.sqlite_cwd_rows_updated,
                    "updatedWorkspaceRoots": sync.updated_workspace_roots,
                    "encryptedContentWarning": sync.encrypted_content_warning,
                    "backupDir": sync.backup_dir,
                    "syncMessage": sync.message,
                }),
            )
        }
        Err(error) => failed(&format!("Provider sync failed: {error}"), json!({})),
    }
}

fn is_success_sync_status(status: &claude_codex_pro_data::ProviderSyncStatus) -> bool {
    matches!(status, claude_codex_pro_data::ProviderSyncStatus::Synced)
}

fn persist_provider_sync_selection(provider: &str) {
    let trimmed = provider.trim();
    if trimmed.is_empty() {
        return;
    }
    let store = SettingsStore::default();
    let mut settings = store.load().unwrap_or_default();
    settings.provider_sync_last_selected_provider = trimmed.to_string();
    if !settings
        .provider_sync_saved_providers
        .iter()
        .any(|item| item == trimmed)
    {
        settings
            .provider_sync_saved_providers
            .push(trimmed.to_string());
    }
    settings.provider_sync_saved_providers =
        normalize_provider_sync_provider_list(settings.provider_sync_saved_providers);
    let _ = store.save(&settings);
}

#[tauri::command]
pub async fn load_ads() -> CommandResult<AdsPayload> {
    match claude_codex_pro_core::ads::fetch_ad_list().await {
        Ok(payload) => ok("Recommendations loaded.", ads_payload(payload)),
        Err(error) => failed(
            &format!("Recommendations failed to load: {error}"),
            AdsPayload {
                version: 1,
                ads: Vec::new(),
            },
        ),
    }
}

#[tauri::command]
pub async fn refresh_script_market() -> CommandResult<ScriptMarketPayload> {
    match script_market::fetch_market_manifest(script_market::DEFAULT_MARKET_INDEX_URL).await {
        Ok(manifest) => ok(
            "Script market refreshed.",
            script_market_payload_from_manifest(&manifest, "ok", "Script market refreshed."),
        ),
        Err(error) => failed(
            &format!("Script market failed to load: {error}"),
            failed_script_market_payload(&format!("Script market failed to load: {error}")),
        ),
    }
}

#[tauri::command]
pub async fn install_market_script(id: String) -> CommandResult<ScriptMarketPayload> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return failed(
            "Script id cannot be empty.",
            failed_script_market_payload("Script id cannot be empty."),
        );
    }
    let manifest =
        match script_market::fetch_market_manifest(script_market::DEFAULT_MARKET_INDEX_URL).await {
            Ok(manifest) => manifest,
            Err(error) => {
                return failed(
                    &format!("Script market failed to load: {error}"),
                    failed_script_market_payload(&format!("Script market failed to load: {error}")),
                );
            }
        };
    let Some(script) = manifest.scripts.iter().find(|script| script.id == trimmed) else {
        return failed(
            "Script was not found in the market manifest.",
            script_market_payload_from_manifest(
                &manifest,
                "failed",
                "Script was not found in the market manifest.",
            ),
        );
    };
    let manager = default_user_script_manager();
    match script_market::install_market_script(&manager, script).await {
        Ok(()) => ok(
            "Script installed.",
            script_market_payload_from_manifest(&manifest, "ok", "Script installed."),
        ),
        Err(error) => failed(
            &format!("Script installation failed: {error}"),
            script_market_payload_from_manifest(
                &manifest,
                "failed",
                &format!("Script installation failed: {error}"),
            ),
        ),
    }
}

#[tauri::command]
pub fn load_codex_plugin_marketplace_status() -> CommandResult<CodexPluginMarketplacePayload> {
    let status = claude_codex_pro_core::codex_plugin_marketplace::status();
    ok(
        &status.message.clone(),
        CodexPluginMarketplacePayload {
            marketplace: status,
        },
    )
}

#[tauri::command]
pub async fn repair_codex_plugin_marketplace() -> CommandResult<CodexPluginMarketplaceRepairPayload>
{
    match claude_codex_pro_core::codex_plugin_marketplace::repair().await {
        Ok(repair) => {
            let marketplace = claude_codex_pro_core::codex_plugin_marketplace::status();
            ok(
                &repair.message.clone(),
                CodexPluginMarketplaceRepairPayload {
                    repair,
                    marketplace,
                },
            )
        }
        Err(error) => {
            let marketplace = claude_codex_pro_core::codex_plugin_marketplace::status();
            failed(
                &format!("Codex OpenAI plugin marketplace repair failed: {error}"),
                CodexPluginMarketplaceRepairPayload {
                    repair: claude_codex_pro_core::codex_plugin_marketplace::CodexPluginMarketplaceRepair {
                        codex_home: marketplace.codex_home.clone(),
                        marketplace_root: marketplace.marketplace_root.clone(),
                        initialized: false,
                        configured: false,
                        config_registered: marketplace.config_registered,
                        needs_repair: marketplace.needs_repair,
                        message: error.to_string(),
                    },
                    marketplace,
                },
            )
        }
    }
}

#[tauri::command]
pub async fn refresh_plugin_hub_catalog() -> CommandResult<PluginHubPayload> {
    let catalog = plugin_hub::fetch_catalog().await;
    ok(
        "Plugin hub catalog refreshed.",
        PluginHubPayload { catalog },
    )
}

#[tauri::command]
pub async fn get_plugin_hub_catalog() -> CommandResult<PluginHubPayload> {
    let catalog = plugin_hub::fetch_catalog().await;
    ok("Plugin hub catalog loaded.", PluginHubPayload { catalog })
}

#[tauri::command]
pub async fn preview_plugin_hub_install(
    request: PluginHubItemRequest,
) -> CommandResult<PluginInstallPreview> {
    match plugin_hub::preview_install(request.id.trim()).await {
        Ok(preview) => ok("Plugin install preview loaded.", preview),
        Err(error) => failed(
            &format!("Plugin install preview failed: {error}"),
            empty_plugin_install_preview(request.id),
        ),
    }
}

#[tauri::command]
pub async fn install_plugin_hub_item(
    request: PluginHubItemRequest,
) -> CommandResult<PluginInstallOutcome> {
    match plugin_hub::install_item(request.id.trim()).await {
        Ok(outcome) => {
            let status = if outcome.installed { "ok" } else { "failed" };
            CommandResult {
                status: status.to_string(),
                message: outcome.message.clone(),
                payload: outcome,
            }
        }
        Err(error) => {
            let message = format!("Plugin install failed: {error}");
            CommandResult {
                status: "failed".to_string(),
                message: message.clone(),
                payload: empty_plugin_install_outcome_with_message(request.id, message),
            }
        }
    }
}

#[tauri::command]
pub async fn uninstall_plugin_hub_item(
    request: PluginHubItemRequest,
) -> CommandResult<PluginHubPayload> {
    match plugin_hub::uninstall_item(request.id.trim()) {
        Ok(_) => {
            let catalog = plugin_hub::fetch_catalog().await;
            ok(
                "Plugin removed. Restart Codex or Claude if needed.",
                PluginHubPayload { catalog },
            )
        }
        Err(error) => {
            let catalog = plugin_hub::fetch_catalog().await;
            failed(
                &format!("Plugin removal failed: {error}"),
                PluginHubPayload { catalog },
            )
        }
    }
}

#[tauri::command]
pub fn preview_ponytail_codex_hooks() -> CommandResult<PluginHookTrustPayload> {
    match plugin_hub::preview_ponytail_codex_hooks() {
        Ok(preview) => ok(&preview.message.clone(), PluginHookTrustPayload { preview }),
        Err(error) => failed(
            &format!("Ponytail Codex hooks preview failed: {error}"),
            PluginHookTrustPayload {
                preview: CodexHookTrustPreview {
                    config_path: String::new(),
                    hooks: Vec::new(),
                    message: error.to_string(),
                },
            },
        ),
    }
}

#[tauri::command]
pub fn trust_ponytail_codex_hooks() -> CommandResult<PluginHookTrustPayload> {
    match plugin_hub::trust_ponytail_codex_hooks() {
        Ok(preview) => ok(&preview.message.clone(), PluginHookTrustPayload { preview }),
        Err(error) => failed(
            &format!("Ponytail Codex hooks trust failed: {error}"),
            PluginHookTrustPayload {
                preview: CodexHookTrustPreview {
                    config_path: String::new(),
                    hooks: Vec::new(),
                    message: error.to_string(),
                },
            },
        ),
    }
}

#[tauri::command]
pub fn generate_ponytail_mcpb_installer() -> CommandResult<McpbPackagePayload> {
    match plugin_hub::generate_and_open_ponytail_mcpb() {
        Ok(package) => ok(&package.message.clone(), McpbPackagePayload { package }),
        Err(error) => failed(
            &format!("Generate Ponytail MCPB failed: {error}"),
            McpbPackagePayload {
                package: McpbPackageOutcome {
                    mcpb_path: String::new(),
                    manifest_path: String::new(),
                    opened: false,
                    message: error.to_string(),
                },
            },
        ),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeDesktopOrgPluginPayload {
    #[serde(rename = "orgPluginStatus")]
    pub org_plugin_status: ClaudeDesktopOrgPluginStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeDesktopOrgPluginInstallPayload {
    pub outcome: ClaudeDesktopOrgPluginOutcome,
    #[serde(rename = "orgPluginStatus")]
    pub org_plugin_status: ClaudeDesktopOrgPluginStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeDesktopLocalBundlePayload {
    pub outcome: plugin_hub::ClaudeDesktopLocalBundleOutcome,
    #[serde(rename = "devModeStatus")]
    pub dev_mode_status: ClaudeDesktopDevModeStatus,
    #[serde(rename = "orgPluginStatus")]
    pub org_plugin_status: ClaudeDesktopOrgPluginStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeDesktopMarketplacePayload {
    #[serde(rename = "marketplaceStatus")]
    pub marketplace_status: ClaudeDesktopMarketplaceStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeDesktopMarketplaceOpenPayload {
    pub outcome: ClaudeDesktopMarketplaceOutcome,
    #[serde(rename = "marketplaceStatus")]
    pub marketplace_status: ClaudeDesktopMarketplaceStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeDesktopMarketplaceRepairPayload {
    pub outcome: ClaudeDesktopMarketplaceOutcome,
    #[serde(rename = "marketplaceStatus")]
    pub marketplace_status: ClaudeDesktopMarketplaceStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeDesktopDevModePayload {
    #[serde(rename = "devModeStatus")]
    pub dev_mode_status: ClaudeDesktopDevModeStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeDesktopDevModeConfigurePayload {
    pub outcome: ClaudeDesktopDevModeOutcome,
    #[serde(rename = "devModeStatus")]
    pub dev_mode_status: ClaudeDesktopDevModeStatus,
}

#[tauri::command]
pub fn load_claude_desktop_org_plugin_status() -> CommandResult<ClaudeDesktopOrgPluginPayload> {
    let status = plugin_hub::load_claude_desktop_org_plugin_status();
    ok(
        &status.message.clone(),
        ClaudeDesktopOrgPluginPayload {
            org_plugin_status: status,
        },
    )
}

#[tauri::command]
pub fn load_claude_desktop_marketplace_status() -> CommandResult<ClaudeDesktopMarketplacePayload> {
    let status = plugin_hub::load_claude_desktop_marketplace_status();
    ok(
        &status.message.clone(),
        ClaudeDesktopMarketplacePayload {
            marketplace_status: status,
        },
    )
}

#[tauri::command]
pub fn load_claude_desktop_dev_mode_status() -> CommandResult<ClaudeDesktopDevModePayload> {
    let status = plugin_hub::load_claude_desktop_dev_mode_status();
    ok(
        &status.message.clone(),
        ClaudeDesktopDevModePayload {
            dev_mode_status: status,
        },
    )
}

#[tauri::command]
pub async fn configure_claude_desktop_dev_mode(
    request: Option<ClaudeDesktopProviderRequest>,
) -> CommandResult<ClaudeDesktopDevModeConfigurePayload> {
    let proxy_port = match ensure_claude_desktop_proxy_helper().await {
        Ok(port) => port,
        Err(error) => {
            return failed(
                &format!("Claude local proxy failed to start before writing dev mode: {error}"),
                ClaudeDesktopDevModeConfigurePayload {
                    outcome: ClaudeDesktopDevModeOutcome {
                        configured: false,
                        normal_config_path: String::new(),
                        threep_config_path: String::new(),
                        profile_path: String::new(),
                        profile_meta_path: String::new(),
                        backup_paths: Vec::new(),
                        message: error.to_string(),
                    },
                    dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
                },
            );
        }
    };
    match plugin_hub::configure_claude_desktop_dev_mode_with_proxy_port(
        request.as_ref(),
        proxy_port,
    ) {
        Ok(outcome) => {
            let helper_message = if wait_helper_backend_online(proxy_port).await {
                format!(" Local model proxy verified on 127.0.0.1:{proxy_port}.")
            } else {
                format!(
                    " Local model proxy requested on 127.0.0.1:{proxy_port}, but /backend/status did not respond yet."
                )
            };
            let status = plugin_hub::load_claude_desktop_dev_mode_status();
            ok(
                &format!("{}{}", outcome.message, helper_message),
                ClaudeDesktopDevModeConfigurePayload {
                    outcome,
                    dev_mode_status: status,
                },
            )
        }
        Err(error) => failed(
            &format!("Configure Claude Desktop development mode failed: {error}"),
            ClaudeDesktopDevModeConfigurePayload {
                outcome: ClaudeDesktopDevModeOutcome {
                    configured: false,
                    normal_config_path: String::new(),
                    threep_config_path: String::new(),
                    profile_path: String::new(),
                    profile_meta_path: String::new(),
                    backup_paths: Vec::new(),
                    message: error.to_string(),
                },
                dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
            },
        ),
    }
}

#[tauri::command]
pub fn open_ponytail_claude_desktop_marketplace_setup()
-> CommandResult<ClaudeDesktopMarketplaceOpenPayload> {
    match plugin_hub::open_ponytail_claude_desktop_marketplace_setup() {
        Ok(outcome) => {
            let status = plugin_hub::load_claude_desktop_marketplace_status();
            ok(
                &outcome.message.clone(),
                ClaudeDesktopMarketplaceOpenPayload {
                    outcome,
                    marketplace_status: status,
                },
            )
        }
        Err(error) => failed(
            &format!("Open Claude Desktop plugin marketplace setup failed: {error}"),
            ClaudeDesktopMarketplaceOpenPayload {
                outcome: ClaudeDesktopMarketplaceOutcome {
                    repaired: false,
                    config_path: String::new(),
                    repositories: Vec::new(),
                    message: error.to_string(),
                },
                marketplace_status: plugin_hub::load_claude_desktop_marketplace_status(),
            },
        ),
    }
}

#[tauri::command]
pub fn repair_claude_desktop_marketplaces() -> CommandResult<ClaudeDesktopMarketplaceRepairPayload>
{
    match plugin_hub::repair_claude_desktop_marketplaces() {
        Ok(outcome) => {
            let status = plugin_hub::load_claude_desktop_marketplace_status();
            ok(
                &outcome.message.clone(),
                ClaudeDesktopMarketplaceRepairPayload {
                    outcome,
                    marketplace_status: status,
                },
            )
        }
        Err(error) => failed(
            &format!("Repair Claude Desktop plugin marketplaces failed: {error}"),
            ClaudeDesktopMarketplaceRepairPayload {
                outcome: ClaudeDesktopMarketplaceOutcome {
                    repaired: false,
                    config_path: String::new(),
                    repositories: Vec::new(),
                    message: error.to_string(),
                },
                marketplace_status: plugin_hub::load_claude_desktop_marketplace_status(),
            },
        ),
    }
}

#[tauri::command]
pub fn open_claude_desktop_org_plugins_dir() -> CommandResult<ClaudeDesktopOrgPluginPayload> {
    match plugin_hub::open_claude_desktop_org_plugins_dir() {
        Ok(status) => ok(
            &status.message.clone(),
            ClaudeDesktopOrgPluginPayload {
                org_plugin_status: status,
            },
        ),
        Err(error) => failed(
            &format!("Open Claude Desktop organization plugin directory failed: {error}"),
            ClaudeDesktopOrgPluginPayload {
                org_plugin_status: plugin_hub::load_claude_desktop_org_plugin_status(),
            },
        ),
    }
}

#[tauri::command]
pub fn install_ponytail_claude_desktop_org_plugin()
-> CommandResult<ClaudeDesktopOrgPluginInstallPayload> {
    match plugin_hub::install_ponytail_claude_desktop_org_plugin() {
        Ok(outcome) => {
            let status = plugin_hub::load_claude_desktop_org_plugin_status();
            ok(
                &outcome.message.clone(),
                ClaudeDesktopOrgPluginInstallPayload {
                    outcome,
                    org_plugin_status: status,
                },
            )
        }
        Err(error) => failed(
            &format!("Install Ponytail Claude Desktop organization plugin failed: {error}"),
            ClaudeDesktopOrgPluginInstallPayload {
                outcome: ClaudeDesktopOrgPluginOutcome {
                    installed: false,
                    org_plugins_dir: String::new(),
                    plugin_dir: String::new(),
                    manifest_path: String::new(),
                    plugin_json_path: String::new(),
                    copied_skills: Vec::new(),
                    backup_path: None,
                    message: error.to_string(),
                },
                org_plugin_status: plugin_hub::load_claude_desktop_org_plugin_status(),
            },
        ),
    }
}

#[tauri::command]
pub async fn install_ponytail_claude_desktop_local_bundle()
-> CommandResult<ClaudeDesktopLocalBundlePayload> {
    match plugin_hub::install_ponytail_claude_desktop_local_bundle().await {
        Ok(outcome) => ok(
            &outcome.message.clone(),
            ClaudeDesktopLocalBundlePayload {
                outcome,
                dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
                org_plugin_status: plugin_hub::load_claude_desktop_org_plugin_status(),
            },
        ),
        Err(error) => failed(
            &format!("Install Claude Desktop local plugin bundle failed: {error}"),
            ClaudeDesktopLocalBundlePayload {
                outcome: plugin_hub::ClaudeDesktopLocalBundleOutcome {
                    dev_mode: ClaudeDesktopDevModeOutcome {
                        configured: false,
                        normal_config_path: String::new(),
                        threep_config_path: String::new(),
                        profile_path: String::new(),
                        profile_meta_path: String::new(),
                        backup_paths: Vec::new(),
                        message: error.to_string(),
                    },
                    codex_mcp: empty_plugin_install_outcome_with_message(
                        "desktop:claude-codex-pro-codex".to_string(),
                        error.to_string(),
                    ),
                    ponytail_mcp: empty_plugin_install_outcome_with_message(
                        "ponytail:claude-desktop-mcp".to_string(),
                        error.to_string(),
                    ),
                    organization_plugin: ClaudeDesktopOrgPluginOutcome {
                        installed: false,
                        org_plugins_dir: String::new(),
                        plugin_dir: String::new(),
                        manifest_path: String::new(),
                        plugin_json_path: String::new(),
                        copied_skills: Vec::new(),
                        backup_path: None,
                        message: error.to_string(),
                    },
                    message: error.to_string(),
                },
                dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
                org_plugin_status: plugin_hub::load_claude_desktop_org_plugin_status(),
            },
        ),
    }
}

#[tauri::command]
pub fn set_user_script_enabled(key: String, enabled: bool) -> CommandResult<SettingsPayload> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return failed("Script key cannot be empty.", fallback_settings_payload());
    }
    let manager = default_user_script_manager();
    match manager.set_script_enabled(trimmed, enabled) {
        Ok(_) => settings_payload(
            if enabled {
                "Script enabled."
            } else {
                "Script disabled."
            },
            "Script setting update failed",
        ),
        Err(error) => failed(
            &format!("Script setting update failed: {error}"),
            fallback_settings_payload(),
        ),
    }
}

#[tauri::command]
pub fn delete_user_script(key: String) -> CommandResult<SettingsPayload> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return failed("Script key cannot be empty.", fallback_settings_payload());
    }
    let manager = default_user_script_manager();
    match manager.delete_user_script(trimmed) {
        Ok(_) => settings_payload("Script deleted.", "Script deletion failed"),
        Err(error) => failed(
            &format!("Script deletion failed: {error}"),
            fallback_settings_payload(),
        ),
    }
}

#[tauri::command]
pub fn open_external_url(url: String) -> CommandResult<Value> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return failed("Only http or https links can be opened.", json!({}));
    }
    match open_url(trimmed) {
        Ok(()) => ok(
            "Link opened in the system browser.",
            json!({ "url": trimmed }),
        ),
        Err(error) => failed(
            &format!("Failed to open link: {error}"),
            json!({ "url": trimmed }),
        ),
    }
}

#[tauri::command]
pub async fn install_entrypoints() -> InstallActionResult {
    tauri::async_runtime::spawn_blocking(install::install_entrypoints)
        .await
        .unwrap_or_else(|error| install_background_failure("Install entrypoints", error))
}

#[tauri::command]
pub async fn uninstall_entrypoints(options: InstallOptions) -> InstallActionResult {
    tauri::async_runtime::spawn_blocking(move || install::uninstall_entrypoints(options))
        .await
        .unwrap_or_else(|error| install_background_failure("Uninstall entrypoints", error))
}

#[tauri::command]
pub async fn repair_shortcuts() -> InstallActionResult {
    tauri::async_runtime::spawn_blocking(install::repair_shortcuts)
        .await
        .unwrap_or_else(|error| install_background_failure("Repair shortcuts", error))
}

#[tauri::command]
pub fn repair_backend() -> CommandResult<SettingsPayload> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    let message = match claude_codex_pro_core::cli_wrapper::ensure_cli_wrapper(&settings) {
        Ok(Some(install)) => format!(
            "Command wrapper updated: {}.",
            install.real_codex.to_string_lossy()
        ),
        Ok(None) => "Command wrapper is already up to date.".to_string(),
        Err(error) => format!("Command wrapper update failed: {error}"),
    };
    settings_payload(&message, "Backend repair failed")
}

#[tauri::command]
pub async fn check_update() -> CommandResult<Value> {
    match claude_codex_pro_core::update::check_for_update(claude_codex_pro_core::version::VERSION)
        .await
    {
        Ok(update) => {
            let status = if update.update_available {
                "ok"
            } else {
                "not_checked"
            };
            CommandResult {
                status: status.to_string(),
                message: if update.update_available {
                    "Update is available.".to_string()
                } else {
                    "You are already on the latest version.".to_string()
                },
                payload: json!({
                    "currentVersion": update.current_version,
                    "latestVersion": update.latest_version,
                    "releaseSummary": update.release_summary,
                    "assetName": update.asset_name,
                    "assetUrl": update.asset_url,
                    "updateAvailable": update.update_available,
                    "progress": 0
                }),
            }
        }
        Err(error) => failed(
            &format!("Update check failed: {error}"),
            json!({
                "currentVersion": claude_codex_pro_core::version::VERSION,
                "latestVersion": Value::Null,
                "releaseSummary": "",
                "assetName": Value::Null,
                "assetUrl": Value::Null,
                "updateAvailable": false,
                "progress": 0
            }),
        ),
    }
}

#[tauri::command]
pub async fn perform_update(
    release: Option<claude_codex_pro_core::update::Release>,
) -> CommandResult<Value> {
    let Some(release) = release else {
        return failed(
            "Please check for updates before installing; no release asset is selected.",
            json!({
                "currentVersion": claude_codex_pro_core::version::VERSION,
                "progress": 0
            }),
        );
    };
    let download_dir = claude_codex_pro_core::paths::default_app_state_dir().join("updates");
    match claude_codex_pro_core::update::perform_update(&release, &download_dir).await {
        Ok(result) => ok(
            "Installer downloaded and started. Follow the installer prompts to finish updating.",
            json!({
                "currentVersion": claude_codex_pro_core::version::VERSION,
                "latestVersion": result.release.version,
                "releaseSummary": result.release.body,
                "installedPath": result.installer_path.to_string_lossy(),
                "launched": result.launched,
                "progress": 100
            }),
        ),
        Err(error) => failed(
            &format!("Update installation failed: {error}"),
            json!({
                "currentVersion": claude_codex_pro_core::version::VERSION,
                "latestVersion": release.version,
                "releaseSummary": release.body,
                "progress": 0
            }),
        ),
    }
}

#[tauri::command]
pub fn load_watcher_state() -> CommandResult<WatcherPayload> {
    ok("Watcher state loaded.", watcher_payload())
}

#[tauri::command]
pub fn install_watcher() -> CommandResult<WatcherPayload> {
    let launcher_path = match resolve_silent_launcher_path() {
        Ok(path) => path,
        Err(error) => {
            return failed(
                &format!("Install watcher failed: {error}"),
                watcher_payload(),
            );
        }
    };
    match claude_codex_pro_core::watcher::install_watcher(&launcher_path, default_debug_port()) {
        Ok(()) => ok("Watcher installed.", watcher_payload()),
        Err(error) => failed(
            &format!("Install watcher failed: {error}"),
            watcher_payload(),
        ),
    }
}

#[tauri::command]
pub fn uninstall_watcher() -> CommandResult<WatcherPayload> {
    match claude_codex_pro_core::watcher::uninstall_watcher() {
        Ok(()) => ok("Watcher uninstalled.", watcher_payload()),
        Err(error) => failed(
            &format!("Uninstall watcher failed: {error}"),
            watcher_payload(),
        ),
    }
}

#[tauri::command]
pub fn enable_watcher() -> CommandResult<WatcherPayload> {
    match claude_codex_pro_core::watcher::enable_watcher() {
        Ok(()) => ok("Watcher enabled.", watcher_payload()),
        Err(error) => failed(
            &format!("Enable watcher failed: {error}"),
            watcher_payload(),
        ),
    }
}

#[tauri::command]
pub fn disable_watcher() -> CommandResult<WatcherPayload> {
    match claude_codex_pro_core::watcher::disable_watcher() {
        Ok(()) => ok("Watcher disabled.", watcher_payload()),
        Err(error) => failed(
            &format!("Disable watcher failed: {error}"),
            watcher_payload(),
        ),
    }
}

#[tauri::command]
pub fn read_latest_logs(request: LogRequest) -> CommandResult<LogsPayload> {
    let path = claude_codex_pro_core::paths::default_diagnostic_log_path();
    match read_tail(&path, request.lines) {
        Ok(text) => ok(
            "Logs loaded.",
            LogsPayload {
                path: path.to_string_lossy().to_string(),
                text,
                lines: request.lines,
            },
        ),
        Err(error) => failed(
            &format!("Failed to read logs: {error}"),
            LogsPayload {
                path: path.to_string_lossy().to_string(),
                text: String::new(),
                lines: request.lines,
            },
        ),
    }
}

#[tauri::command]
pub fn copy_diagnostics() -> CommandResult<DiagnosticsPayload> {
    ok(
        "Diagnostics report generated.",
        DiagnosticsPayload {
            report: diagnostics_report(),
        },
    )
}

#[tauri::command]
pub fn reset_settings() -> CommandResult<SettingsPayload> {
    let settings = BackendSettings::default();
    match SettingsStore::default().save(&settings) {
        Ok(()) => settings_payload("Settings reset to defaults.", "Settings reset failed"),
        Err(error) => failed(
            &format!("Settings reset failed: {error}"),
            SettingsPayload {
                settings,
                settings_path: claude_codex_pro_core::paths::default_settings_path()
                    .to_string_lossy()
                    .to_string(),
                user_scripts: user_script_inventory(),
            },
        ),
    }
}

#[tauri::command]
pub fn reset_image_overlay_settings() -> CommandResult<SettingsPayload> {
    let store = SettingsStore::default();
    let mut settings = store.load().unwrap_or_default();
    let defaults = BackendSettings::default();
    settings.codex_app_image_overlay_enabled = defaults.codex_app_image_overlay_enabled;
    settings.codex_app_image_overlay_path = defaults.codex_app_image_overlay_path;
    settings.codex_app_image_overlay_opacity = defaults.codex_app_image_overlay_opacity;
    let settings = normalize_settings_before_save(settings);
    match store.save(&settings) {
        Ok(()) => settings_payload(
            "Image overlay settings reset.",
            "Image overlay reset failed",
        ),
        Err(error) => failed(
            &format!("Image overlay reset failed: {error}"),
            SettingsPayload {
                settings,
                settings_path: claude_codex_pro_core::paths::default_settings_path()
                    .to_string_lossy()
                    .to_string(),
                user_scripts: user_script_inventory(),
            },
        ),
    }
}

#[tauri::command]
pub fn relay_status() -> CommandResult<RelayPayload> {
    let status = claude_codex_pro_core::relay_config::default_relay_status();
    let message = if status.authenticated {
        "ChatGPT login status detected."
    } else {
        "ChatGPT login status was not detected. You can still configure Codex API mode."
    };
    ok(message, relay_payload(status, None))
}

#[tauri::command]
pub fn read_relay_files() -> CommandResult<RelayFilesPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    match relay_files_payload_from_home(&home) {
        Ok(payload) => ok("Relay files loaded.", payload),
        Err(error) => failed(
            &format!("Failed to read relay files: {error}"),
            RelayFilesPayload {
                config_path: home.join("config.toml").to_string_lossy().to_string(),
                auth_path: home.join("auth.json").to_string_lossy().to_string(),
                config_contents: String::new(),
                auth_contents: String::new(),
            },
        ),
    }
}

#[tauri::command]
pub fn save_relay_file(request: SaveRelayFileRequest) -> CommandResult<RelayFilesPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    match save_relay_file_in_home(&home, &request.kind, &request.contents)
        .and_then(|_| relay_files_payload_from_home(&home))
    {
        Ok(payload) => ok("Relay file saved.", payload),
        Err(error) => failed(
            &format!("Failed to save relay file: {error}"),
            relay_files_payload_from_home(&home).unwrap_or_else(|_| RelayFilesPayload {
                config_path: home.join("config.toml").to_string_lossy().to_string(),
                auth_path: home.join("auth.json").to_string_lossy().to_string(),
                config_contents: String::new(),
                auth_contents: String::new(),
            }),
        ),
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayProfileSwitchRequest {
    pub settings: BackendSettings,
    #[serde(default)]
    pub previous_active_relay_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcswitchImportPayload {
    pub db_path: String,
    pub profiles: Vec<RelayProfile>,
    pub scanned: usize,
}

#[tauri::command]
pub fn import_ccswitch_codex_providers() -> CommandResult<CcswitchImportPayload> {
    let Some(db_path) = default_ccswitch_db_path() else {
        return failed(
            "cc-switch database was not found at ~/.cc-switch/cc-switch.db.",
            CcswitchImportPayload {
                db_path: String::new(),
                profiles: Vec::new(),
                scanned: 0,
            },
        );
    };
    match read_ccswitch_codex_profiles(&db_path) {
        Ok((profiles, scanned)) => {
            let count = profiles.len();
            ok(
                &format!("Imported {count} Codex provider profiles from cc-switch."),
                CcswitchImportPayload {
                    db_path: db_path.to_string_lossy().to_string(),
                    profiles,
                    scanned,
                },
            )
        }
        Err(error) => failed(
            &format!("Failed to read cc-switch providers: {error}"),
            CcswitchImportPayload {
                db_path: db_path.to_string_lossy().to_string(),
                profiles: Vec::new(),
                scanned: 0,
            },
        ),
    }
}

#[tauri::command]
pub fn switch_relay_profile(
    request: RelayProfileSwitchRequest,
) -> CommandResult<RelaySwitchPayload> {
    let Ok(_guard) = relay_switch_mutex().lock() else {
        let status = claude_codex_pro_core::relay_config::default_relay_status();
        return failed(
            "Provider switching is already running. Please try again later.",
            relay_switch_payload(
                SettingsStore::default().load().unwrap_or_default(),
                status,
                None,
            ),
        );
    };
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let store = SettingsStore::default();
    let previous_active_relay_id = request.previous_active_relay_id;
    let settings = normalize_settings_before_save(request.settings);
    log_manager_event(
        "manager.switch_relay_profile.start",
        json!({
            "previousActiveRelayId": previous_active_relay_id,
            "targetRelayId": settings.active_relay_id
        }),
    );
    match claude_codex_pro_core::relay_switch::switch_relay_profile_in_home(
        &store,
        &home,
        settings,
        &previous_active_relay_id,
    ) {
        Ok(result) => {
            let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
            log_manager_event(
                "manager.switch_relay_profile.ok",
                json!({
                    "targetRelayId": result.settings.active_relay_id,
                    "configured": status.configured,
                    "backupPath": result.backup_path.as_ref()
                }),
            );
            ok(
                "Provider profile switched.",
                relay_switch_payload(result.settings, status, result.backup_path),
            )
        }
        Err(error) => {
            let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
            let settings = store.load().unwrap_or_default();
            log_manager_event(
                "manager.switch_relay_profile.failed",
                json!({
                    "previousActiveRelayId": previous_active_relay_id,
                    "activeRelayId": settings.active_relay_id,
                    "error": error.to_string()
                }),
            );
            failed(
                &format!("Provider profile switch failed: {error}"),
                relay_switch_payload(settings, status, None),
            )
        }
    }
}

#[tauri::command]
pub fn preview_claude_desktop_provider(
    request: ClaudeDesktopProviderRequest,
) -> CommandResult<ClaudeDesktopProviderPreviewPayload> {
    log_manager_event(
        "manager.claude_desktop_provider.preview",
        json!({
            "baseUrl": request.base_url.trim(),
            "modelLines": request.model_list.lines().filter(|line| !line.trim().is_empty()).count()
        }),
    );
    let proxy_port = current_claude_desktop_proxy_port_hint();
    match claude_codex_pro_core::claude_desktop_provider::preview_claude_desktop_provider_with_proxy_port(
        &request,
        proxy_port,
    ) {
        Ok(preview) => ok(
            &format!("Claude Desktop provider preview generated for local proxy port {proxy_port}."),
            ClaudeDesktopProviderPreviewPayload { preview },
        ),
        Err(error) => failed(
            &format!("Claude Desktop provider preview failed: {error}"),
            ClaudeDesktopProviderPreviewPayload {
                preview: empty_claude_desktop_provider_preview(),
            },
        ),
    }
}

#[tauri::command]
pub async fn apply_claude_desktop_provider(
    request: ClaudeDesktopProviderRequest,
) -> CommandResult<ClaudeDesktopProviderApplyPayload> {
    log_manager_event(
        "manager.claude_desktop_provider.apply.start",
        json!({
            "baseUrl": request.base_url.trim(),
            "modelLines": request.model_list.lines().filter(|line| !line.trim().is_empty()).count()
        }),
    );
    let _ = plugin_hub::persist_claude_desktop_provider_request_to_settings(&request);
    let proxy_port = match ensure_claude_desktop_proxy_helper().await {
        Ok(port) => port,
        Err(error) => {
            log_manager_event(
                "manager.claude_desktop_provider.apply.proxy_failed",
                json!({ "error": error.to_string() }),
            );
            return failed(
                &format!("Claude local proxy failed to start before provider write: {error}"),
                ClaudeDesktopProviderApplyPayload {
                    outcome: empty_claude_desktop_provider_outcome(error.to_string()),
                    dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
                },
            );
        }
    };
    match claude_codex_pro_core::claude_desktop_provider::apply_claude_desktop_provider_with_proxy_port(
        &request,
        proxy_port,
    ) {
        Ok(outcome) => {
            log_manager_event(
                "manager.claude_desktop_provider.apply.ok",
                json!({
                    "normalConfigPath": outcome.normal_config_path,
                    "threepConfigPath": outcome.threep_config_path,
                    "proxyPort": proxy_port,
                    "backupCount": outcome.backup_paths.len()
                }),
            );
            ok(
                &outcome.message.clone(),
                ClaudeDesktopProviderApplyPayload {
                    outcome,
                    dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
                },
            )
        }
        Err(error) => {
            log_manager_event(
                "manager.claude_desktop_provider.apply.failed",
                json!({ "error": error.to_string() }),
            );
            failed(
                &format!("Claude Desktop provider apply failed: {error}"),
                ClaudeDesktopProviderApplyPayload {
                    outcome: empty_claude_desktop_provider_outcome(error.to_string()),
                    dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
                },
            )
        }
    }
}

#[tauri::command]
pub fn restore_claude_desktop_provider_official() -> CommandResult<ClaudeDesktopProviderApplyPayload>
{
    log_manager_event("manager.claude_desktop_provider.restore.start", json!({}));
    match claude_codex_pro_core::claude_desktop_provider::restore_claude_desktop_provider_official()
    {
        Ok(outcome) => {
            log_manager_event(
                "manager.claude_desktop_provider.restore.ok",
                json!({ "backupCount": outcome.backup_paths.len() }),
            );
            ok(
                &outcome.message.clone(),
                ClaudeDesktopProviderApplyPayload {
                    outcome,
                    dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
                },
            )
        }
        Err(error) => {
            log_manager_event(
                "manager.claude_desktop_provider.restore.failed",
                json!({ "error": error.to_string() }),
            );
            failed(
                &format!("Claude Desktop provider restore failed: {error}"),
                ClaudeDesktopProviderApplyPayload {
                    outcome: empty_claude_desktop_provider_outcome(error.to_string()),
                    dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
                },
            )
        }
    }
}

#[tauri::command]
pub fn write_diagnostic_event(event: String, detail: Value) -> CommandResult<Value> {
    let event = sanitize_manager_event(&event);
    match claude_codex_pro_core::diagnostic_log::append_diagnostic_log(&event, detail) {
        Ok(()) => ok("Diagnostic event written.", json!({})),
        Err(error) => failed(
            &format!("Failed to write diagnostic event: {error}"),
            json!({}),
        ),
    }
}

#[tauri::command]
pub fn backfill_relay_profile_from_live(
    request: BackfillRelayProfileRequest,
) -> CommandResult<SettingsBackfillPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let mut settings = request.settings;
    let requested_profile_id = request.profile_id.clone();
    log_manager_event(
        "manager.backfill_relay_profile_from_live.start",
        json!({
            "profileId": requested_profile_id,
            "activeRelayId": settings.active_relay_id
        }),
    );
    let Some(profile) = settings
        .relay_profiles
        .iter_mut()
        .find(|profile| profile.id == request.profile_id)
    else {
        log_manager_event(
            "manager.backfill_relay_profile_from_live.missing_profile",
            json!({
                "profileId": requested_profile_id
            }),
        );
        return failed(
            "The selected provider profile was not found. Save it first and try again.",
            SettingsBackfillPayload { settings },
        );
    };

    match claude_codex_pro_core::relay_config::backfill_relay_profile_from_home_with_common(
        &home,
        profile,
        &mut settings.relay_context_config_contents,
    ) {
        Ok(()) => {
            log_manager_event(
                "manager.backfill_relay_profile_from_live.ok",
                json!({
                    "profileId": requested_profile_id
                }),
            );
            ok(
                "Provider profile backfilled from live relay files.",
                SettingsBackfillPayload { settings },
            )
        }
        Err(error) => {
            log_manager_event(
                "manager.backfill_relay_profile_from_live.failed",
                json!({
                    "profileId": requested_profile_id,
                    "error": error.to_string()
                }),
            );
            failed(
                &format!("Failed to backfill provider profile from live relay files: {error}"),
                SettingsBackfillPayload { settings },
            )
        }
    }
}

#[tauri::command]
pub fn list_context_entries(
    request: ContextSettingsRequest,
) -> CommandResult<ContextEntriesPayload> {
    match claude_codex_pro_core::relay_config::list_context_entries_from_common_config(
        &request.settings.relay_context_config_contents,
    ) {
        Ok(entries) => ok(
            "Context entries loaded.",
            ContextEntriesPayload {
                settings: request.settings,
                entries,
            },
        ),
        Err(error) => failed(
            &format!("Failed to load context entries: {error}"),
            ContextEntriesPayload {
                settings: request.settings,
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub fn read_live_context_entries() -> CommandResult<LiveContextEntriesPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let config_path = home.join("config.toml");
    let config = read_optional_text_file(&config_path).unwrap_or_default();
    match claude_codex_pro_core::relay_config::list_context_entries_from_common_config(&config) {
        Ok(entries) => ok(
            "Live context entries loaded.",
            LiveContextEntriesPayload { entries },
        ),
        Err(error) => failed(
            &format!("Failed to load live context entries: {error}"),
            LiveContextEntriesPayload {
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub fn upsert_context_entry(request: ContextEntryRequest) -> CommandResult<ContextEntriesPayload> {
    let mut settings = request.settings;
    match claude_codex_pro_core::relay_config::upsert_context_entry_in_common_config(
        &settings.relay_context_config_contents,
        &request.kind,
        &request.id,
        &request.toml_body,
    ) {
        Ok(common) => {
            settings.relay_context_config_contents = common;
            list_context_entries(ContextSettingsRequest { settings })
        }
        Err(error) => failed(
            &format!("Failed to save context entry: {error}"),
            ContextEntriesPayload {
                settings,
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub fn sync_live_context_entries(
    request: ContextSettingsRequest,
) -> CommandResult<LiveContextEntriesPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let config_path = home.join("config.toml");
    let current_config = match read_optional_text_file(&config_path) {
        Ok(config) => config,
        Err(error) => {
            return failed(
                &format!("Failed to read live config.toml: {error}"),
                LiveContextEntriesPayload {
                    entries: empty_context_entries(),
                },
            );
        }
    };
    let updated_config = match claude_codex_pro_core::relay_config::sync_live_config_context_entries(
        &current_config,
        &request.settings.relay_context_config_contents,
    ) {
        Ok(config) => config,
        Err(error) => {
            return failed(
                &format!("Failed to sync live context entries: {error}"),
                LiveContextEntriesPayload {
                    entries: empty_context_entries(),
                },
            );
        }
    };
    if let Some(parent) = config_path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            return failed(
                &format!("Failed to create Codex config directory: {error}"),
                LiveContextEntriesPayload {
                    entries: empty_context_entries(),
                },
            );
        }
    }
    if let Err(error) = std::fs::write(&config_path, &updated_config) {
        return failed(
            &format!("Failed to write live config.toml: {error}"),
            LiveContextEntriesPayload {
                entries: empty_context_entries(),
            },
        );
    }
    match claude_codex_pro_core::relay_config::list_context_entries_from_common_config(
        &updated_config,
    ) {
        Ok(entries) => ok(
            "Live context entries synchronized.",
            LiveContextEntriesPayload { entries },
        ),
        Err(error) => failed(
            &format!("Failed to read synchronized live context entries: {error}"),
            LiveContextEntriesPayload {
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub fn delete_context_entry(request: ContextDeleteRequest) -> CommandResult<ContextEntriesPayload> {
    let mut settings = request.settings;
    match claude_codex_pro_core::relay_config::delete_context_entry_from_common_config(
        &settings.relay_context_config_contents,
        &request.kind,
        &request.id,
    ) {
        Ok(common) => {
            settings.relay_context_config_contents = common;
            list_context_entries(ContextSettingsRequest { settings })
        }
        Err(error) => failed(
            &format!("Failed to delete context entry: {error}"),
            ContextEntriesPayload {
                settings,
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub fn list_claude_context_entries() -> CommandResult<ClaudeContextEntriesPayload> {
    match plugin_hub::list_claude_desktop_mcp_entries() {
        Ok(mcp) => {
            let org = plugin_hub::load_claude_desktop_org_plugin_status();
            let market = plugin_hub::load_claude_desktop_marketplace_status();
            ok(
                "Claude context entries loaded.",
                ClaudeContextEntriesPayload {
                    config_path: mcp.config_path,
                    entries: claude_entries_from_status(mcp.entries, org, market),
                },
            )
        }
        Err(error) => failed(
            &format!("Failed to load Claude context entries: {error}"),
            ClaudeContextEntriesPayload {
                config_path: String::new(),
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub fn upsert_claude_context_entry(
    request: ClaudeContextEntryRequest,
) -> CommandResult<ClaudeContextEntriesPayload> {
    if request.kind != "mcp" {
        return failed(
            "Claude currently supports only MCP context entries; skills and plugins are managed by Claude plugin flows.",
            ClaudeContextEntriesPayload {
                config_path: String::new(),
                entries: empty_context_entries(),
            },
        );
    }
    match plugin_hub::upsert_claude_desktop_mcp_entry(&request.id, &request.body) {
        Ok(mcp) => {
            let org = plugin_hub::load_claude_desktop_org_plugin_status();
            let market = plugin_hub::load_claude_desktop_marketplace_status();
            ok(
                "Claude MCP entry saved.",
                ClaudeContextEntriesPayload {
                    config_path: mcp.config_path,
                    entries: claude_entries_from_status(mcp.entries, org, market),
                },
            )
        }
        Err(error) => failed(
            &format!("Failed to save Claude MCP entry: {error}"),
            ClaudeContextEntriesPayload {
                config_path: String::new(),
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub fn delete_claude_context_entry(
    request: ClaudeContextDeleteRequest,
) -> CommandResult<ClaudeContextEntriesPayload> {
    if request.kind != "mcp" {
        return failed(
            "Claude currently supports deleting only MCP context entries.",
            ClaudeContextEntriesPayload {
                config_path: String::new(),
                entries: empty_context_entries(),
            },
        );
    }
    match plugin_hub::delete_claude_desktop_mcp_entry(&request.id) {
        Ok(mcp) => {
            let org = plugin_hub::load_claude_desktop_org_plugin_status();
            let market = plugin_hub::load_claude_desktop_marketplace_status();
            ok(
                "Claude MCP entry deleted.",
                ClaudeContextEntriesPayload {
                    config_path: mcp.config_path,
                    entries: claude_entries_from_status(mcp.entries, org, market),
                },
            )
        }
        Err(error) => failed(
            &format!("Failed to delete Claude MCP entry: {error}"),
            ClaudeContextEntriesPayload {
                config_path: String::new(),
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub fn extract_relay_common_config(
    request: ExtractRelayCommonConfigRequest,
) -> CommandResult<ExtractRelayCommonConfigPayload> {
    match claude_codex_pro_core::relay_config::extract_common_config_from_config(
        &request.config_contents,
    )
    .and_then(|common_config_contents| {
        let profile_config_contents =
            claude_codex_pro_core::relay_config::strip_common_config_from_config(
                &request.config_contents,
                &common_config_contents,
            )?;
        Ok(ExtractRelayCommonConfigPayload {
            common_config_contents,
            profile_config_contents,
        })
    }) {
        Ok(payload) => ok("Common relay config extracted.", payload),
        Err(error) => failed(
            &format!("Failed to extract common relay config: {error}"),
            ExtractRelayCommonConfigPayload {
                common_config_contents: String::new(),
                profile_config_contents: request.config_contents,
            },
        ),
    }
}

#[tauri::command]
pub async fn test_relay_profile(profile: RelayProfile) -> CommandResult<RelayProfileTestPayload> {
    let profile_name = if profile.name.trim().is_empty() {
        "Unnamed provider"
    } else {
        profile.name.trim()
    };
    let settings = SettingsStore::default().load().unwrap_or_default();
    let test_model: String = if !profile.test_model.trim().is_empty() {
        profile.test_model.trim().to_string()
    } else {
        let from_profile = claude_codex_pro_core::relay_config::relay_profile_model(&profile);
        if from_profile.trim().is_empty() {
            settings.relay_test_model.trim().to_string()
        } else {
            from_profile
        }
    };
    match claude_codex_pro_core::relay_config::test_relay_profile(&profile, &test_model).await {
        Ok(result) => {
            let status = if result.http_status < 400 {
                "ok"
            } else {
                "failed"
            };
            let preview = result.response_preview.trim();
            let detail = if preview.is_empty() {
                "No response preview.".to_string()
            } else {
                format!("Preview: {preview}")
            };
            CommandResult {
                status: status.to_string(),
                message: format!(
                    "Provider {profile_name} tested with model {test_model}; HTTP {}; {detail}",
                    result.http_status
                ),
                payload: RelayProfileTestPayload {
                    http_status: result.http_status,
                    endpoint: result.endpoint,
                    response_preview: result.response_preview,
                },
            }
        }
        Err(error) => failed(
            &format!("Provider {profile_name} test failed: {error}"),
            RelayProfileTestPayload {
                http_status: 0,
                endpoint: String::new(),
                response_preview: String::new(),
            },
        ),
    }
}

#[tauri::command]
pub async fn fetch_relay_profile_models(
    profile: RelayProfile,
) -> CommandResult<RelayProfileModelsPayload> {
    let profile_name = if profile.name.trim().is_empty() {
        "Unnamed provider"
    } else {
        profile.name.trim()
    };
    match claude_codex_pro_core::model_catalog::fetch_relay_profile_model_ids(&profile).await {
        Ok((models, endpoint)) => ok(
            &format!(
                "Loaded {} models for provider {profile_name}.",
                models.len()
            ),
            RelayProfileModelsPayload { models, endpoint },
        ),
        Err(error) => failed(
            &format!("Failed to load models for provider {profile_name}: {error}"),
            RelayProfileModelsPayload {
                models: Vec::new(),
                endpoint: String::new(),
            },
        ),
    }
}

#[tauri::command]
pub fn apply_relay_injection() -> CommandResult<RelayPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let settings = SettingsStore::default().load().unwrap_or_default();
    if !settings.relay_profiles_enabled {
        let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
        return failed(
            "Provider profiles are disabled; config.toml and auth.json were not written.",
            relay_payload(status, None),
        );
    }
    let relay = settings.active_relay_profile();
    log_relay_apply_request("manager.apply_relay_injection", &settings, &relay);
    if relay_has_complete_files(&relay) {
        return match claude_codex_pro_core::relay_config::apply_relay_profile_to_home_with_switch_rules_and_computer_use_guard(
            &home,
            &relay,
            &relay_combined_common_config(&settings),
            settings.computer_use_guard_enabled,
        ) {
            Ok(result) => {
                let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
                log_relay_apply_result(
                    "manager.apply_relay_injection.ok",
                    &relay,
                    &status,
                    result.backup_path.as_ref(),
                    None,
                );
                ok(
                    "Provider switched using compatibility rules.",
                    relay_payload(status, result.backup_path),
                )
            }
            Err(error) => {
                let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
                log_relay_apply_result(
                    "manager.apply_relay_injection.failed",
                    &relay,
                    &status,
                    None,
                    Some(error.to_string()),
                );
                failed(
                    &format!("Full relay profile switch failed: {error}"),
                    relay_payload(status, None),
                )
            }
        };
    }

    let auth = claude_codex_pro_core::relay_config::chatgpt_auth_status_from_home(&home);
    if !auth.authenticated {
        let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
        log_relay_apply_result(
            "manager.apply_relay_injection.failed",
            &relay,
            &status,
            None,
            Some("ChatGPT login status was not detected".to_string()),
        );
        return failed(
            "ChatGPT login status was not detected, so relay configuration was not written.",
            relay_payload(status, None),
        );
    }

    match claude_codex_pro_core::relay_config::apply_relay_config_to_home_with_protocol(
        &home,
        &relay.base_url,
        &relay.api_key,
        relay.protocol,
        claude_codex_pro_core::protocol_proxy::DEFAULT_PROTOCOL_PROXY_PORT,
    ) {
        Ok(result) => {
            let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
            log_relay_apply_result(
                "manager.apply_relay_injection.ok",
                &relay,
                &status,
                result.backup_path.as_ref(),
                None,
            );
            ok(
                "Relay configuration written. The API key is not shown in the UI.",
                relay_payload(status, result.backup_path),
            )
        }
        Err(error) => {
            let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
            log_relay_apply_result(
                "manager.apply_relay_injection.failed",
                &relay,
                &status,
                None,
                Some(error.to_string()),
            );
            failed(
                &format!("Failed to write relay configuration: {error}"),
                relay_payload(status, None),
            )
        }
    }
}

#[tauri::command]
pub fn apply_pure_api_injection() -> CommandResult<RelayPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let settings = SettingsStore::default().load().unwrap_or_default();
    if !settings.relay_profiles_enabled {
        let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
        return failed(
            "Provider profiles are disabled; config.toml and auth.json were not written.",
            relay_payload(status, None),
        );
    }
    let relay = settings.active_relay_profile();
    log_relay_apply_request("manager.apply_pure_api_injection", &settings, &relay);
    if relay_has_complete_files(&relay) {
        return match claude_codex_pro_core::relay_config::apply_relay_profile_to_home_with_switch_rules_and_computer_use_guard(
            &home,
            &relay,
            &relay_combined_common_config(&settings),
            settings.computer_use_guard_enabled,
        ) {
            Ok(result) => {
                let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
                log_relay_apply_result(
                    "manager.apply_pure_api_injection.ok",
                    &relay,
                    &status,
                    result.backup_path.as_ref(),
                    None,
                );
                if !status.configured {
                    return failed(
                        "Pure API config was written, but no complete custom provider was detected. Check config.toml and the provider API key.",
                        relay_payload(status, result.backup_path),
                    );
                }
                ok(
                    "Provider switched using compatibility rules.",
                    relay_payload(status, result.backup_path),
                )
            }
            Err(error) => {
                let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
                log_relay_apply_result(
                    "manager.apply_pure_api_injection.failed",
                    &relay,
                    &status,
                    None,
                    Some(error.to_string()),
                );
                failed(
                    &format!("Pure API profile switch failed: {error}"),
                    relay_payload(status, None),
                )
            }
        };
    }

    match claude_codex_pro_core::relay_config::apply_pure_api_config_to_home_with_protocol(
        &home,
        &relay.base_url,
        &relay.api_key,
        relay.protocol,
        claude_codex_pro_core::protocol_proxy::DEFAULT_PROTOCOL_PROXY_PORT,
    ) {
        Ok(result) => {
            let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
            log_relay_apply_result(
                "manager.apply_pure_api_injection.ok",
                &relay,
                &status,
                result.backup_path.as_ref(),
                None,
            );
            if !status.configured {
                return failed(
                    "Pure API config was written, but no complete custom provider was detected. Check config.toml and the provider API key.",
                    relay_payload(status, result.backup_path),
                );
            }
            ok(
                "Pure API mode written: config.toml uses the custom provider and auth.json uses the selected provider.",
                relay_payload(status, result.backup_path),
            )
        }
        Err(error) => {
            let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
            log_relay_apply_result(
                "manager.apply_pure_api_injection.failed",
                &relay,
                &status,
                None,
                Some(error.to_string()),
            );
            failed(
                &format!("Failed to write pure API mode: {error}"),
                relay_payload(status, None),
            )
        }
    }
}

#[tauri::command]
pub fn clear_relay_injection() -> CommandResult<RelayPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let settings = SettingsStore::default().load().unwrap_or_default();
    let relay = settings.active_relay_profile();
    log_manager_event("manager.clear_relay_injection.start", json!({}));
    let auth_contents = (relay.relay_mode == claude_codex_pro_core::settings::RelayMode::Official
        && !relay.official_mix_api_key
        && !relay.auth_contents.trim().is_empty())
    .then_some(relay.auth_contents.as_str());
    match claude_codex_pro_core::relay_config::clear_relay_config_to_home_with_auth(
        &home,
        auth_contents,
    ) {
        Ok(result) => {
            let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
            log_manager_event(
                "manager.clear_relay_injection.ok",
                json!({
                    "configured": status.configured,
                    "backupPath": result.backup_path.as_ref()
                }),
            );
            ok(
                "Custom relay API mode cleared and switched back to official ChatGPT login mode.",
                relay_payload(status, result.backup_path),
            )
        }
        Err(error) => {
            let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
            log_manager_event(
                "manager.clear_relay_injection.failed",
                json!({
                    "configured": status.configured,
                    "error": error.to_string()
                }),
            );
            failed(
                &format!("Failed to clear relay configuration: {error}"),
                relay_payload(status, None),
            )
        }
    }
}

fn relay_has_complete_files(relay: &claude_codex_pro_core::settings::RelayProfile) -> bool {
    if relay.relay_mode == claude_codex_pro_core::settings::RelayMode::Official
        && relay.official_mix_api_key
    {
        return !relay.config_contents.trim().is_empty();
    }
    !relay.config_contents.trim().is_empty() && !relay.auth_contents.trim().is_empty()
}

fn log_relay_apply_request(
    event: &str,
    settings: &BackendSettings,
    relay: &claude_codex_pro_core::settings::RelayProfile,
) {
    let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
        event,
        json!({
            "activeRelayId": settings.active_relay_id,
            "relayId": relay.id,
            "relayName": relay.name,
            "relayMode": relay.relay_mode,
            "protocol": relay.protocol,
            "baseUrl": relay.base_url,
            "hasConfigContents": !relay.config_contents.trim().is_empty(),
            "hasAuthContents": !relay.auth_contents.trim().is_empty(),
            "configContainsProxy": relay.config_contents.contains("127.0.0.1:57321")
        }),
    );
}

fn log_relay_apply_result(
    event: &str,
    relay: &claude_codex_pro_core::settings::RelayProfile,
    status: &claude_codex_pro_core::relay_config::RelayStatus,
    backup_path: Option<&String>,
    error: Option<String>,
) {
    log_manager_event(
        event,
        json!({
            "relayId": relay.id,
            "relayName": relay.name,
            "relayMode": relay.relay_mode,
            "protocol": relay.protocol,
            "configured": status.configured,
            "requiresOpenaiAuth": status.requires_openai_auth,
            "hasBearerToken": status.has_bearer_token,
            "backupPath": backup_path,
            "error": error
        }),
    );
}

fn log_manager_event(event: &str, detail: Value) {
    let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(event, detail);
}

fn sanitize_manager_event(event: &str) -> String {
    let suffix = event
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let suffix = suffix.trim_matches(['.', '_', '-']).trim();
    if suffix.is_empty() {
        "manager.ui.event".to_string()
    } else if suffix.starts_with("manager.") {
        suffix.to_string()
    } else {
        format!("manager.ui.{suffix}")
    }
}

fn refresh_cli_wrapper_after_settings_save(settings: &BackendSettings) -> String {
    match claude_codex_pro_core::cli_wrapper::ensure_cli_wrapper(settings) {
        Ok(Some(install)) => format!(
            " Command wrapper updated: {}.",
            install.real_codex.to_string_lossy()
        ),
        Ok(None) => String::new(),
        Err(error) => format!(" Command wrapper update failed: {error}."),
    }
}

fn relay_payload(
    status: claude_codex_pro_core::relay_config::RelayStatus,
    backup_path: Option<String>,
) -> RelayPayload {
    RelayPayload {
        authenticated: status.authenticated,
        auth_source: status.auth_source,
        account_label: status.account_label,
        config_path: status.config_path,
        configured: status.configured,
        requires_openai_auth: status.requires_openai_auth,
        has_bearer_token: status.has_bearer_token,
        backup_path,
    }
}

fn relay_switch_payload(
    settings: BackendSettings,
    status: claude_codex_pro_core::relay_config::RelayStatus,
    backup_path: Option<String>,
) -> RelaySwitchPayload {
    RelaySwitchPayload {
        settings,
        relay: relay_payload(status, backup_path),
        settings_path: claude_codex_pro_core::paths::default_settings_path()
            .to_string_lossy()
            .to_string(),
        user_scripts: user_script_inventory(),
    }
}

fn relay_switch_mutex() -> &'static Mutex<()> {
    static RELAY_SWITCH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    RELAY_SWITCH_LOCK.get_or_init(|| Mutex::new(()))
}

fn empty_claude_desktop_provider_preview() -> ClaudeDesktopProviderPreview {
    ClaudeDesktopProviderPreview {
        profile_id: String::new(),
        profile_name: String::new(),
        normal_config_path: String::new(),
        threep_config_path: String::new(),
        profile_path: String::new(),
        meta_path: String::new(),
        write_targets: Vec::new(),
        config_diff: String::new(),
        redacted_profile: String::new(),
    }
}

fn empty_claude_desktop_provider_outcome(message: String) -> ClaudeDesktopProviderOutcome {
    ClaudeDesktopProviderOutcome {
        configured: false,
        normal_config_path: String::new(),
        threep_config_path: String::new(),
        profile_path: String::new(),
        meta_path: String::new(),
        backup_paths: Vec::new(),
        message,
    }
}

fn empty_context_entries() -> claude_codex_pro_core::relay_config::CodexContextEntries {
    claude_codex_pro_core::relay_config::CodexContextEntries {
        mcp_servers: Vec::new(),
        skills: Vec::new(),
        plugins: Vec::new(),
    }
}

fn claude_entries_from_status(
    mcp_entries: Vec<plugin_hub::ClaudeDesktopMcpEntry>,
    org: ClaudeDesktopOrgPluginStatus,
    market: ClaudeDesktopMarketplaceStatus,
) -> claude_codex_pro_core::relay_config::CodexContextEntries {
    let mut entries = empty_context_entries();
    entries.mcp_servers = mcp_entries
        .into_iter()
        .map(
            |entry| claude_codex_pro_core::relay_config::CodexContextEntry {
                id: entry.id,
                kind: "mcp".to_string(),
                title: entry.title,
                summary: entry.summary,
                toml_body: entry.json_body,
                enabled: entry.enabled,
            },
        )
        .collect();
    entries
        .skills
        .push(claude_codex_pro_core::relay_config::CodexContextEntry {
            id: "ponytail".to_string(),
            kind: "skill".to_string(),
            title: "Ponytail Skills".to_string(),
            summary: if org.ponytail_installed {
                format!("Installed: {}", org.ponytail_plugin_dir)
            } else {
                format!("Not installed: {}", org.org_plugins_dir)
            },
            toml_body: serde_json::to_string_pretty(&json!({
                "pluginDir": org.ponytail_plugin_dir,
                "orgPluginsDir": org.org_plugins_dir,
                "writable": org.writable
            }))
            .unwrap_or_else(|_| "{}".to_string()),
            enabled: org.ponytail_installed,
        });
    entries
        .plugins
        .push(claude_codex_pro_core::relay_config::CodexContextEntry {
            id: market.plugin,
            kind: "plugin".to_string(),
            title: market.marketplace,
            summary: market.message,
            toml_body: serde_json::to_string_pretty(&json!({
                "deepLink": market.deep_link,
                "canAutoWrite": market.can_auto_write,
                "configPath": market.config_path,
                "repositories": market.repositories
            }))
            .unwrap_or_else(|_| "{}".to_string()),
            enabled: market.supported,
        });
    entries
}

fn relay_files_payload_from_home(home: &std::path::Path) -> anyhow::Result<RelayFilesPayload> {
    let config_path = home.join("config.toml");
    let auth_path = home.join("auth.json");
    Ok(RelayFilesPayload {
        config_path: config_path.to_string_lossy().to_string(),
        auth_path: auth_path.to_string_lossy().to_string(),
        config_contents: read_optional_text_file(&config_path)?,
        auth_contents: read_optional_text_file(&auth_path)?,
    })
}

fn save_relay_file_in_home(
    home: &std::path::Path,
    kind: &str,
    contents: &str,
) -> anyhow::Result<()> {
    let path = match kind {
        "config" => home.join("config.toml"),
        "auth" => home.join("auth.json"),
        other => anyhow::bail!("Unknown relay file type: {other}"),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(())
}

fn read_optional_text_file(path: &std::path::Path) -> anyhow::Result<String> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

fn ads_payload(payload: Value) -> AdsPayload {
    AdsPayload {
        version: payload.get("version").and_then(Value::as_u64).unwrap_or(1),
        ads: payload
            .get("ads")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    }
}

fn open_url(url: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        claude_codex_pro_core::windows_open_url(url)
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| anyhow::anyhow!("Failed to launch system browser: {error}"))
    }
}

fn settings_payload(message: &str, failure_context: &str) -> CommandResult<SettingsPayload> {
    match settings_payload_value() {
        Ok(payload) => ok(message, payload),
        Err((error, payload)) => failed(&format!("{failure_context}: {error}"), payload),
    }
}

fn settings_payload_value() -> Result<SettingsPayload, (anyhow::Error, SettingsPayload)> {
    let store = SettingsStore::default();
    let settings_path = claude_codex_pro_core::paths::default_settings_path()
        .to_string_lossy()
        .to_string();
    match store.load() {
        Ok(settings) => Ok(SettingsPayload {
            settings,
            settings_path,
            user_scripts: user_script_inventory(),
        }),
        Err(error) => Err((
            error,
            SettingsPayload {
                settings: BackendSettings::default(),
                settings_path,
                user_scripts: user_script_inventory(),
            },
        )),
    }
}

fn fallback_settings_payload() -> SettingsPayload {
    SettingsPayload {
        settings: SettingsStore::default().load().unwrap_or_default(),
        settings_path: claude_codex_pro_core::paths::default_settings_path()
            .to_string_lossy()
            .to_string(),
        user_scripts: user_script_inventory(),
    }
}

fn user_script_inventory() -> Value {
    default_user_script_manager()
        .inventory()
        .unwrap_or_else(|error| {
            json!({
                "enabled": true,
                "scripts": [],
                "error": error.to_string()
            })
        })
}

fn failed_script_market_payload(message: &str) -> ScriptMarketPayload {
    ScriptMarketPayload {
        market: json!({
            "status": "failed",
            "message": message,
            "indexUrl": script_market::DEFAULT_MARKET_INDEX_URL,
            "updatedAt": "",
            "scripts": []
        }),
        user_scripts: user_script_inventory(),
    }
}

fn script_market_payload_from_manifest(
    manifest: &ScriptMarketManifest,
    status: &str,
    message: &str,
) -> ScriptMarketPayload {
    let user_scripts = user_script_inventory();
    let installed = installed_market_versions(&user_scripts);
    let scripts = manifest
        .scripts
        .iter()
        .map(|script| market_script_payload(script, &installed))
        .collect::<Vec<_>>();
    ScriptMarketPayload {
        market: json!({
            "status": status,
            "message": message,
            "indexUrl": script_market::DEFAULT_MARKET_INDEX_URL,
            "updatedAt": manifest.updated_at.clone().unwrap_or_default(),
            "scripts": scripts
        }),
        user_scripts,
    }
}

fn installed_market_versions(user_scripts: &Value) -> BTreeMap<String, String> {
    user_scripts
        .get("scripts")
        .and_then(Value::as_array)
        .map(|scripts| {
            scripts
                .iter()
                .filter_map(|script| {
                    let id = script.get("market_id").and_then(Value::as_str)?;
                    if id.is_empty() {
                        return None;
                    }
                    let version = script
                        .get("version")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    Some((id.to_string(), version))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn market_script_payload(script: &MarketScript, installed: &BTreeMap<String, String>) -> Value {
    let installed_version = installed.get(&script.id).cloned().unwrap_or_default();
    let is_installed = !installed_version.is_empty();
    json!({
        "id": script.id,
        "name": script.name,
        "description": script.description,
        "version": script.version,
        "author": script.author,
        "tags": script.tags,
        "homepage": script.homepage,
        "script_url": script.script_url,
        "sha256": script.sha256,
        "installed": is_installed,
        "installedVersion": installed_version,
        "updateAvailable": is_installed && installed.get(&script.id).map(|version| version != &script.version).unwrap_or(false)
    })
}

fn claude_chinese_window_payload(
    app: &tauri::AppHandle,
    status: &claude_codex_pro_core::claude_desktop::ClaudeDesktopStatus,
) -> ClaudeChineseWindowPayload {
    ClaudeChineseWindowPayload {
        open: app.get_webview_window("claude-chinese").is_some(),
        label: "claude-chinese".to_string(),
        default_url: "https://claude.ai/new".to_string(),
        injection_mode: "wrapped_webview".to_string(),
        cdp_status: status.cdp_status.clone(),
        cdp_blocker: status.cdp_blocker.clone(),
        official_install_kind: status.install_kind.clone(),
    }
}

fn prompt_optimizer_window_payload(open: bool) -> PromptOptimizerWindowPayload {
    PromptOptimizerWindowPayload {
        open,
        label: "main".to_string(),
        default_url: "https://prompt.always200.com".to_string(),
        integration_mode: "tools_card_external_browser".to_string(),
        license: "AGPL-3.0-only".to_string(),
    }
}

fn route_main_window_to_plugin_hub(app: &tauri::AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Err(tauri::Error::WindowNotFound);
    };
    window.show()?;
    let _ = window.unminimize();
    let _ = window.set_focus();
    window.eval(main_window_route_script("tools"))?;
    Ok(())
}

fn main_window_route_script(route: &str) -> String {
    let route = serde_json::to_string(route).unwrap_or_else(|_| "\"overview\"".to_string());
    format!(
        "window.dispatchEvent(new CustomEvent('claude-codex-pro-navigate', {{ detail: {{ route: {route} }} }}));"
    )
}

fn empty_plugin_item(id: String) -> claude_codex_pro_core::plugin_hub::PluginCatalogItem {
    claude_codex_pro_core::plugin_hub::PluginCatalogItem {
        id,
        name: String::new(),
        description: String::new(),
        source_id: String::new(),
        source_label: String::new(),
        source_url: String::new(),
        category: String::new(),
        author: String::new(),
        homepage: String::new(),
        license: String::new(),
        tags: Vec::new(),
        install_kind: claude_codex_pro_core::plugin_hub::InstallKind::ResourceLink,
        install_status: claude_codex_pro_core::plugin_hub::InstallStatus::Unsupported,
        install_command: Vec::new(),
        config_preview: String::new(),
        risk: String::new(),
        requirements: Vec::new(),
    }
}

fn empty_plugin_install_preview(id: String) -> PluginInstallPreview {
    PluginInstallPreview {
        item: empty_plugin_item(id),
        can_install: false,
        action: "failed".to_string(),
        command: Vec::new(),
        config_diff: String::new(),
        message: String::new(),
    }
}

fn empty_plugin_install_outcome_with_message(id: String, message: String) -> PluginInstallOutcome {
    let preview = empty_plugin_install_preview(id);
    PluginInstallOutcome {
        item: preview.item.clone(),
        preview,
        installed: false,
        message,
        stdout: String::new(),
        stderr: String::new(),
        backup_path: None,
    }
}

fn default_user_script_manager() -> UserScriptManager {
    let config_dir = user_scripts_config_dir();
    UserScriptManager::new(
        builtin_user_scripts_dir(),
        config_dir.join("user_scripts"),
        config_dir.join("user_scripts.json"),
    )
}

fn user_scripts_config_dir() -> PathBuf {
    if cfg!(windows) {
        if let Some(roaming) = std::env::var_os("APPDATA") {
            return PathBuf::from(roaming).join("Claude Codex Pro");
        }
        if let Some(home) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) {
            return home
                .join("AppData")
                .join("Roaming")
                .join("Claude Codex Pro");
        }
    }
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|dirs| dirs.home_dir().join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    config_root.join("claude-codex-pro")
}

fn builtin_user_scripts_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .map(|path| path.join("user_scripts"))
        .unwrap_or_else(|| PathBuf::from("user_scripts"))
}

fn diagnostics_report() -> String {
    let (codex_app_path, entrypoints, latest_launch) = load_overview_payload();
    let overview = ok(
        "Overview loaded.",
        OverviewPayload {
            codex_version: codex_app_path
                .as_deref()
                .and_then(claude_codex_pro_core::app_paths::codex_app_version),
            codex_app: path_state(codex_app_path),
            silent_shortcut: shortcut_state(entrypoints.silent_shortcut),
            management_shortcut: shortcut_state(entrypoints.management_shortcut),
            latest_launch,
            current_version: claude_codex_pro_core::version::VERSION.to_string(),
            update_status: "not_checked".to_string(),
            settings_path: claude_codex_pro_core::paths::default_settings_path()
                .to_string_lossy()
                .to_string(),
            logs_path: claude_codex_pro_core::paths::default_diagnostic_log_path()
                .to_string_lossy()
                .to_string(),
        },
    );
    let settings = SettingsStore::default().load().unwrap_or_default();
    let generated_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    serde_json::to_string_pretty(&json!({
        "generatedAtMs": generated_at_ms,
        "version": claude_codex_pro_core::version::VERSION,
        "exePath": current_exe_path_string(),
        "exeLastModifiedMs": current_exe_last_modified_ms(),
        "overview": overview.payload,
        "settings": settings,
        "logs": {
            "diagnosticLogPath": claude_codex_pro_core::paths::default_diagnostic_log_path(),
            "latestStatusPath": claude_codex_pro_core::paths::default_latest_status_path()
        },
        "platform": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH
        }
    }))
    .unwrap_or_else(|error| format!("Diagnostics report serialization failed: {error}"))
}

fn load_overview_payload() -> (
    Option<PathBuf>,
    install::EntryPointState,
    Option<LaunchStatus>,
) {
    let settings = SettingsStore::default().load().unwrap_or_default();
    let latest_launch = StatusStore::default()
        .load_latest()
        .unwrap_or(None)
        .map(refresh_launch_port_status);
    (
        claude_codex_pro_core::app_paths::resolve_codex_app_dir_with_saved(
            None,
            Some(settings.codex_app_path.as_str()),
        )
        .or_else(claude_codex_pro_core::app_paths::find_running_codex_app_dir),
        install::inspect_entrypoints(),
        latest_launch,
    )
}

fn refresh_launch_port_status(mut status: LaunchStatus) -> LaunchStatus {
    status.debug_port_online = status
        .debug_port
        .is_some_and(|port| codex_debug_port_online(port));
    status.helper_port_online = status
        .helper_port
        .is_some_and(|port| helper_backend_online(port));
    if let Some(heartbeat) = latest_renderer_runtime_heartbeat() {
        status.frontend_runtime_online = renderer_heartbeat_is_fresh(heartbeat.timestamp_ms);
        status.frontend_runtime_seen_at_ms = Some(heartbeat.timestamp_ms);
    } else {
        status.frontend_runtime_online = false;
        status.frontend_runtime_seen_at_ms = None;
    }
    status
}

fn codex_debug_port_online(port: u16) -> bool {
    tcp_port_open(port) && codex_debug_json_ready(port)
}

fn helper_backend_online(port: u16) -> bool {
    if !tcp_port_open(port) {
        return false;
    }
    let Ok(mut stream) = connect_loopback(port) else {
        return false;
    };
    let request = b"POST /backend/status HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}";
    if stream.write_all(request).is_err() {
        return false;
    }
    let mut response = String::new();
    stream.read_to_string(&mut response).is_ok()
        && response.starts_with("HTTP/1.1 200")
        && response.contains("\"status\":\"ok\"")
        && response.contains("\"transport\":\"http-helper\"")
        && response.contains("\"version\":")
}

async fn wait_helper_backend_online(port: u16) -> bool {
    for _ in 0..12 {
        if async_helper_backend_online(port).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    false
}

async fn async_helper_backend_online(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(Ok(mut stream)) = tokio::time::timeout(
        Duration::from_millis(500),
        tokio::net::TcpStream::connect(address),
    )
    .await
    else {
        return false;
    };
    let request = b"POST /backend/status HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}";
    if tokio::time::timeout(Duration::from_millis(500), stream.write_all(request))
        .await
        .map_or(true, |result| result.is_err())
    {
        return false;
    }
    let mut response = String::new();
    tokio::time::timeout(
        Duration::from_millis(800),
        stream.read_to_string(&mut response),
    )
    .await
    .is_ok_and(|result| result.is_ok())
        && response.starts_with("HTTP/1.1 200")
        && response.contains("\"status\":\"ok\"")
        && response.contains("\"transport\":\"http-helper\"")
        && response.contains("\"version\":")
}

fn codex_debug_json_ready(port: u16) -> bool {
    let Ok(mut stream) = connect_loopback(port) else {
        return false;
    };
    let request = b"GET /json HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if stream.write_all(request).is_err() {
        return false;
    }
    let mut response = String::new();
    stream.read_to_string(&mut response).is_ok()
        && response.starts_with("HTTP/1.1 200")
        && response.contains("webSocketDebuggerUrl")
}

fn tcp_port_open(port: u16) -> bool {
    connect_loopback(port).is_ok()
}

fn connect_loopback(port: u16) -> std::io::Result<TcpStream> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let stream = TcpStream::connect_timeout(&address, Duration::from_millis(250))?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
    Ok(stream)
}

fn install_background_failure(action: &str, error: impl std::fmt::Display) -> InstallActionResult {
    let state = install::inspect_entrypoints();
    InstallActionResult {
        status: "failed".to_string(),
        message: format!("{action} background task failed: {error}"),
        silent_shortcut: state.silent_shortcut,
        management_shortcut: state.management_shortcut,
    }
}

fn watcher_payload() -> WatcherPayload {
    let flag = claude_codex_pro_core::watcher::default_watcher_disabled_flag();
    WatcherPayload {
        enabled: !flag.exists(),
        disabled_flag: flag.to_string_lossy().to_string(),
    }
}

fn read_tail(path: &Path, max_lines: usize) -> std::io::Result<String> {
    let contents = fs::read_to_string(path)?;
    let mut lines = contents.lines().rev().take(max_lines).collect::<Vec<_>>();
    lines.reverse();
    Ok(lines.join("\n"))
}

fn path_state(path: Option<PathBuf>) -> PathState {
    match path {
        Some(path) => PathState {
            status: "found".to_string(),
            path: Some(path.to_string_lossy().to_string()),
        },
        None => PathState {
            status: "missing".to_string(),
            path: None,
        },
    }
}

fn shortcut_state(shortcut: install::ShortcutState) -> PathState {
    PathState {
        status: if shortcut.installed {
            "installed".to_string()
        } else {
            "missing".to_string()
        },
        path: shortcut.path,
    }
}

fn default_ccswitch_db_path() -> Option<PathBuf> {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().join(".cc-switch").join("cc-switch.db"))
        .filter(|path| path.is_file())
}

fn read_ccswitch_codex_profiles(db_path: &Path) -> anyhow::Result<(Vec<RelayProfile>, usize)> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("闁瑰灚鎸哥槐?{}", db_path.display()))?;
    let mut statement = conn.prepare(
        "SELECT id, name, settings_config, COALESCE(sort_index, 0)
         FROM providers
         WHERE app_type = 'codex'
         ORDER BY COALESCE(sort_index, 0), name COLLATE NOCASE, id COLLATE NOCASE",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut profiles = Vec::new();
    let mut scanned = 0usize;
    for row in rows {
        scanned += 1;
        let (id, name, settings_config) = row?;
        if let Some(profile) = ccswitch_codex_profile_from_settings(&id, &name, &settings_config) {
            profiles.push(profile);
        }
    }
    Ok((profiles, scanned))
}

fn ccswitch_codex_profile_from_settings(
    id: &str,
    name: &str,
    settings: &str,
) -> Option<RelayProfile> {
    let parsed = serde_json::from_str::<Value>(settings).ok()?;
    let config = parsed
        .get("config")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let auth = parsed.get("auth").unwrap_or(&Value::Null);
    let base_url = codex_config_base_url(config)
        .or_else(|| string_at(&parsed, &["base_url", "baseUrl", "apiBaseUrl"]))
        .unwrap_or_default();
    if base_url.trim().is_empty() {
        return None;
    }
    let api_key = codex_auth_api_key(auth)
        .or_else(|| string_at(&parsed, &["apiKey", "api_key", "OPENAI_API_KEY"]))
        .unwrap_or_default();
    let model = codex_config_model(config)
        .or_else(|| string_at(&parsed, &["model", "testModel"]))
        .unwrap_or_else(|| "gpt-5.5".to_string());
    let mut profile = RelayProfile {
        id: format!("{}-ccswitch", supplier_id_from_import(id)),
        name: format!("{name} (ccswitch)"),
        model,
        base_url: base_url.trim().to_string(),
        upstream_base_url: base_url.trim().to_string(),
        api_key: api_key.trim().to_string(),
        relay_mode: claude_codex_pro_core::settings::RelayMode::PureApi,
        user_agent: "ccswitch".to_string(),
        ..RelayProfile::default()
    };
    profile.test_model = profile.model.clone();
    profile.config_contents = imported_supplier_config_toml(&profile);
    profile.auth_contents = format!(
        "{}\n",
        serde_json::to_string_pretty(&json!({ "OPENAI_API_KEY": profile.api_key }))
            .unwrap_or_else(|_| "{}".to_string())
    );
    Some(profile)
}

fn codex_auth_api_key(auth: &Value) -> Option<String> {
    auth.get("OPENAI_API_KEY")
        .or_else(|| auth.get("api_key"))
        .or_else(|| auth.get("apiKey"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn codex_config_model(config: &str) -> Option<String> {
    let doc = config.parse::<DocumentMut>().ok()?;
    doc.get("model")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn codex_config_base_url(config: &str) -> Option<String> {
    let doc = config.parse::<DocumentMut>().ok()?;
    if let Some(provider_id) = doc.get("model_provider").and_then(|item| item.as_str()) {
        if let Some(value) = doc
            .get("model_providers")
            .and_then(|item| item.as_table())
            .and_then(|providers| providers.get(provider_id))
            .and_then(|item| item.as_table())
            .and_then(|provider| provider.get("base_url"))
            .and_then(|item| item.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        {
            return Some(value);
        }
    }
    doc.get("base_url")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn string_at(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn supplier_id_from_import(value: &str) -> String {
    let mut output = String::new();
    let mut previous_dash = false;
    for ch in value.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            previous_dash = false;
        } else if !previous_dash && !output.is_empty() {
            output.push('-');
            previous_dash = true;
        }
    }
    while output.ends_with('-') {
        output.pop();
    }
    if output.is_empty() {
        "provider".to_string()
    } else {
        output
    }
}

fn imported_supplier_config_toml(profile: &RelayProfile) -> String {
    let provider_id = supplier_id_from_import(&profile.id);
    let mut lines = Vec::new();
    if !profile.model.trim().is_empty() {
        lines.push(format!(
            "model = {}",
            serde_json::to_string(profile.model.trim()).unwrap()
        ));
    }
    lines.push(format!(
        "model_provider = {}",
        serde_json::to_string(&provider_id).unwrap()
    ));
    lines.push("model_reasoning_effort = \"high\"".to_string());
    lines.push("disable_response_storage = true".to_string());
    lines.push(String::new());
    lines.push(format!("[model_providers.{provider_id}]"));
    lines.push(format!(
        "name = {}",
        serde_json::to_string(&provider_id).unwrap()
    ));
    lines.push("wire_api = \"responses\"".to_string());
    lines.push("requires_openai_auth = true".to_string());
    lines.push("env_key = \"OPENAI_API_KEY\"".to_string());
    lines.push(format!(
        "base_url = {}",
        serde_json::to_string(profile.base_url.trim()).unwrap()
    ));
    lines.push(String::new());
    lines.join("\n")
}

fn ok<T: Serialize>(message: &str, payload: T) -> CommandResult<T> {
    CommandResult {
        status: "ok".to_string(),
        message: message.to_string(),
        payload,
    }
}

fn failed<T: Serialize>(message: &str, payload: T) -> CommandResult<T> {
    CommandResult {
        status: "failed".to_string(),
        message: message.to_string(),
        payload,
    }
}

fn default_debug_port() -> u16 {
    9230
}

fn default_helper_port() -> u16 {
    57321
}

fn default_log_lines() -> usize {
    200
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn test_path_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn backend_version_returns_structured_payload() {
        let result = backend_version();

        assert_eq!(result.status, "ok");
        assert!(!result.payload.version.is_empty());
        assert!(!result.payload.exe_path.is_empty());
        assert!(result.payload.exe_last_modified_ms.is_some());
    }

    #[test]
    fn startup_options_returns_structured_payload() {
        let result = startup_options();

        assert_eq!(result.status, "ok");
    }

    #[test]
    fn startup_options_honors_show_update_environment() {
        unsafe {
            std::env::set_var("CLAUDE_CODEX_PRO_SHOW_UPDATE", "1");
        }

        let result = startup_options();

        unsafe {
            std::env::remove_var("CLAUDE_CODEX_PRO_SHOW_UPDATE");
        }

        assert_eq!(result.status, "ok");
        assert!(result.payload.show_update);
    }

    #[test]
    fn startup_options_honors_show_update_argument() {
        assert!(should_show_update(
            ["claude-codex-pro-manager.exe", "--show-update"],
            None
        ));
    }

    #[test]
    #[test]
    fn plugin_install_failure_serializes_non_empty_top_level_message() {
        let result = failed(
            "Plugin install failed: please sign in to Claude Code CLI.",
            empty_plugin_install_outcome_with_message(
                "official:test".to_string(),
                "Please sign in to Claude Code CLI.".to_string(),
            ),
        );
        let serialized = serde_json::to_value(&result).unwrap();

        assert_eq!(
            serialized["message"],
            serde_json::json!("Plugin install failed: please sign in to Claude Code CLI.")
        );
        assert_eq!(
            serialized["installMessage"],
            serde_json::json!("Please sign in to Claude Code CLI.")
        );
    }

    #[test]
    fn overview_contains_expected_operational_fields() {
        let result = tauri::async_runtime::block_on(load_overview());

        assert_eq!(result.status, "ok");
        assert!(!result.payload.current_version.is_empty());
        assert!(
            result.payload.codex_version.is_none()
                || result
                    .payload
                    .codex_version
                    .as_deref()
                    .is_some_and(|version| !version.is_empty())
        );
        assert!(matches!(
            result.payload.codex_app.status.as_str(),
            "found" | "missing"
        ));
        assert!(matches!(
            result.payload.silent_shortcut.status.as_str(),
            "installed" | "missing"
        ));
    }

    #[test]
    fn update_install_requires_release_payload() {
        let result = tauri::async_runtime::block_on(perform_update(None));

        assert!(result.message.contains("Please check for updates"));
    }

    #[test]
    fn watcher_state_returns_disabled_flag_path() {
        let result = load_watcher_state();

        assert_eq!(result.status, "ok");
        assert!(result.payload.disabled_flag.contains("watcher.disabled"));
    }

    #[test]
    fn claude_desktop_status_preserves_read_only_audit_contract() {
        let result = tauri::async_runtime::block_on(load_claude_desktop_status());

        assert!(matches!(result.status.as_str(), "ok" | "not_running"));
        assert_eq!(result.payload.supported_integration, "external_automation");
        assert!(matches!(
            result.payload.cdp_status.as_str(),
            "blocked" | "observed_but_unverified" | "node_inspector_ready"
        ));

        for audit in &result.payload.executable_audits {
            assert!(!audit.patch_eligible);
            assert_eq!(
                audit.mutation_policy,
                "blocked_no_executable_asar_signature_or_integrity_metadata_changes"
            );
            assert!(
                audit
                    .notes
                    .iter()
                    .any(|note| note.contains("Read-only audit"))
            );
        }
    }

    #[test]
    fn claude_desktop_integrity_returns_policy_and_audit_shape() {
        let result = tauri::async_runtime::block_on(load_claude_desktop_integrity());

        assert!(matches!(
            result.status.as_str(),
            "ok" | "warning" | "not_checked"
        ));
        assert_eq!(
            result.payload.policy,
            "read_only_audit_no_executable_or_asar_patch"
        );
        for audit in &result.payload.executable_audits {
            assert!(!audit.patch_eligible);
            assert!(!audit.integrity_level.is_empty());
            assert!(!audit.verification_scope.is_empty());
        }
    }

    #[test]
    fn claude_desktop_provider_preview_command_redacts_api_key() {
        let result = preview_claude_desktop_provider(ClaudeDesktopProviderRequest {
            name: "TopoReduce".to_string(),
            base_url: "https://api.toporeduce.cn".to_string(),
            api_key: "sk-manager-secret".to_string(),
            model_list: claude_codex_pro_core::protocol_proxy::claude_desktop_default_model_list(),
        });

        assert_eq!(result.status, "ok");
        assert!(
            result
                .payload
                .preview
                .config_diff
                .contains("***redacted***")
        );
        assert!(
            !result
                .payload
                .preview
                .config_diff
                .contains("sk-manager-secret")
        );
        assert!(
            !result
                .payload
                .preview
                .redacted_profile
                .contains("sk-manager-secret")
        );
    }

    #[test]
    fn paste_claude_desktop_draft_rejects_empty_text_without_submit() {
        let result = paste_claude_desktop_draft(ClaudeDesktopDraftRequest {
            text: "   ".to_string(),
        });

        assert_eq!(result.status, "failed");
        assert_eq!(result.payload.action, "paste_draft");
        assert_eq!(result.payload.input_chars, 0);
        assert!(!result.payload.auto_submitted);
    }

    #[test]
    fn submit_claude_desktop_text_rejects_empty_text_without_submit() {
        let result = submit_claude_desktop_text(ClaudeDesktopDraftRequest {
            text: "   ".to_string(),
        });

        assert_eq!(result.status, "failed");
        assert_eq!(result.payload.action, "paste_and_submit");
        assert_eq!(result.payload.input_chars, 0);
        assert!(!result.payload.auto_submitted);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires a running local Claude Desktop process"]
    fn verify_claude_desktop_command_verifies_foreground_without_input() {
        let result = verify_claude_desktop();

        if result.status == "failed" && result.message.contains("not running") {
            eprintln!("Claude Desktop is not running; live verify skipped.");
            return;
        }

        eprintln!(
            "verify result: status={}, action={}, foreground_verified={}, pid={:?}, title={:?}",
            result.status,
            result.payload.action,
            result.payload.foreground_verified,
            result.payload.foreground_process_id,
            result.payload.foreground_title
        );
        assert_eq!(result.payload.action, "verify_target");
        assert_eq!(result.status, "ok");
        assert!(result.payload.foreground_verified);
        assert!(result.payload.foreground_process_id.is_some());
        assert!(result.payload.foreground_title.is_some());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires a running local Claude Desktop process"]
    fn open_claude_desktop_devtools_command_sends_shortcut() {
        let result = open_claude_desktop_devtools();

        if result.status == "failed" && result.message.contains("not running") {
            eprintln!("Claude Desktop is not running; live devtools command skipped.");
            return;
        }

        eprintln!(
            "devtools command: status={}, action={}, foreground_verified={}, pid={:?}, title={:?}",
            result.status,
            result.payload.action,
            result.payload.foreground_verified,
            result.payload.foreground_process_id,
            result.payload.foreground_title
        );
        assert_eq!(result.payload.action, "open_devtools");
        assert_eq!(result.status, "ok");
        assert!(result.payload.foreground_verified);
        assert!(result.payload.foreground_process_id.is_some());
        assert!(result.payload.foreground_title.is_some());
    }

    #[test]
    fn claude_desktop_verify_command_writes_diagnostic_log_record() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("claude-codex-pro.log");
        claude_codex_pro_core::diagnostic_log::set_diagnostic_log_path_for_tests(Some(
            log_path.clone(),
        ));

        let result = verify_claude_desktop();
        let contents = std::fs::read_to_string(&log_path).unwrap();
        claude_codex_pro_core::diagnostic_log::set_diagnostic_log_path_for_tests(None);

        assert!(matches!(result.status.as_str(), "ok" | "failed"));
        assert!(contents.contains("\"event\":\"manager.claude_desktop.verify\""));
        assert!(contents.contains("\"status\":"));
        assert!(contents.contains("\"message\":"));
    }

    #[test]
    fn missing_logs_return_failed_status() {
        let result = read_latest_logs(LogRequest { lines: 25 });

        if result.payload.text.is_empty() {
            assert_eq!(result.status, "failed");
        }
    }

    #[test]
    fn relay_payload_does_not_expose_token_text() {
        let payload = relay_payload(
            claude_codex_pro_core::relay_config::RelayStatus {
                authenticated: true,
                auth_source: "registry.json".to_string(),
                account_label: Some("user@example.test".to_string()),
                config_path: "config.toml".to_string(),
                configured: true,
                requires_openai_auth: true,
                has_bearer_token: true,
            },
            None,
        );
        let text = serde_json::to_string(&payload).unwrap();

        assert!(!text.contains("sk-"));
        assert!(text.contains("hasBearerToken"));
    }

    #[test]
    fn relay_files_payload_reads_config_and_auth_contents() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("config.toml"),
            "model_provider = \"custom\"\n",
        )
        .unwrap();
        std::fs::write(
            temp.path().join("auth.json"),
            "{\"OPENAI_API_KEY\":\"sk-test\"}\n",
        )
        .unwrap();

        let payload = relay_files_payload_from_home(temp.path()).unwrap();

        assert!(payload.config_path.ends_with("config.toml"));
        assert!(payload.auth_path.ends_with("auth.json"));
        assert_eq!(payload.config_contents, "model_provider = \"custom\"\n");
        assert_eq!(payload.auth_contents, "{\"OPENAI_API_KEY\":\"sk-test\"}\n");
    }

    #[test]
    fn delete_local_session_falls_back_when_requested_db_no_longer_contains_thread() {
        let temp = tempfile::tempdir().unwrap();
        let previous_codex_home = std::env::var_os("CODEX_HOME");
        let codex_home = temp.path().join("codex-home");
        let sqlite_dir = codex_home.join("sqlite");
        std::fs::create_dir_all(&sqlite_dir).unwrap();
        let stale_db = sqlite_dir.join("codex-dev.db");
        let active_db = sqlite_dir.join("state_5.sqlite");
        let rollout_path = temp.path().join("rollout.jsonl");
        std::fs::write(&rollout_path, "{\"type\":\"message\"}\n").unwrap();
        let stale = rusqlite::Connection::open(&stale_db).unwrap();
        stale
            .execute(
                "CREATE TABLE threads (id TEXT PRIMARY KEY, rollout_path TEXT, title TEXT)",
                [],
            )
            .unwrap();
        drop(stale);
        let active = rusqlite::Connection::open(&active_db).unwrap();
        active
            .execute(
                "CREATE TABLE threads (id TEXT PRIMARY KEY, rollout_path TEXT, title TEXT)",
                [],
            )
            .unwrap();
        active
            .execute(
                "INSERT INTO threads VALUES ('t1', ?1, 'Active Thread')",
                [rollout_path.to_string_lossy().to_string()],
            )
            .unwrap();
        drop(active);

        unsafe {
            std::env::set_var("CODEX_HOME", &codex_home);
        }
        let result = delete_local_session(DeleteLocalSessionRequest {
            session_id: "t1".to_string(),
            title: "Active Thread".to_string(),
            db_path: Some(stale_db.to_string_lossy().to_string()),
        });
        unsafe {
            if let Some(value) = previous_codex_home {
                std::env::set_var("CODEX_HOME", value);
            } else {
                std::env::remove_var("CODEX_HOME");
            }
        }

        assert_eq!(result.status, "ok");
        assert_eq!(
            result.payload.status,
            claude_codex_pro_core::models::DeleteStatus::LocalDeleted
        );
        let active = rusqlite::Connection::open(&active_db).unwrap();
        assert_eq!(
            active
                .query_row("SELECT COUNT(*) FROM threads WHERE id = 't1'", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            0
        );
    }

    #[test]
    fn list_local_sessions_deduplicates_threads_across_current_and_legacy_dbs() {
        let temp = tempfile::tempdir().unwrap();
        let previous_codex_home = std::env::var_os("CODEX_HOME");
        let codex_home = temp.path().join("codex-home");
        let sqlite_dir = codex_home.join("sqlite");
        std::fs::create_dir_all(&sqlite_dir).unwrap();
        let current_db = sqlite_dir.join("state_5.sqlite");
        let legacy_db = codex_home.join("state_5.sqlite");
        create_minimal_thread_db(&current_db, "t1", "Current Copy", 100);
        create_minimal_thread_db(&legacy_db, "t1", "Legacy Copy", 200);

        unsafe {
            std::env::set_var("CODEX_HOME", &codex_home);
        }
        let result = list_local_sessions();
        restore_codex_home(previous_codex_home);

        assert_eq!(result.status, "ok");
        assert_eq!(result.payload.sessions.len(), 1);
        assert_eq!(result.payload.sessions[0].id, "t1");
        assert_eq!(result.payload.sessions[0].title, "Legacy Copy");
        assert_eq!(
            result.payload.sessions[0].db_path,
            legacy_db.to_string_lossy()
        );
    }

    #[test]
    fn delete_local_session_removes_duplicate_threads_from_all_candidate_dbs() {
        let temp = tempfile::tempdir().unwrap();
        let previous_codex_home = std::env::var_os("CODEX_HOME");
        let codex_home = temp.path().join("codex-home");
        let sqlite_dir = codex_home.join("sqlite");
        std::fs::create_dir_all(&sqlite_dir).unwrap();
        let current_db = sqlite_dir.join("state_5.sqlite");
        let legacy_db = codex_home.join("state_5.sqlite");
        create_minimal_thread_db(&current_db, "t1", "Current Copy", 100);
        create_minimal_thread_db(&legacy_db, "t1", "Legacy Copy", 200);

        unsafe {
            std::env::set_var("CODEX_HOME", &codex_home);
        }
        let result = delete_local_session(DeleteLocalSessionRequest {
            session_id: "t1".to_string(),
            title: "Legacy Copy".to_string(),
            db_path: Some(legacy_db.to_string_lossy().to_string()),
        });
        restore_codex_home(previous_codex_home);

        assert_eq!(result.status, "ok");
        assert_eq!(thread_count(&current_db, "t1"), 0);
        assert_eq!(thread_count(&legacy_db, "t1"), 0);
    }

    fn create_minimal_thread_db(path: &Path, id: &str, title: &str, updated_at_ms: i64) {
        let db = rusqlite::Connection::open(path).unwrap();
        db.execute(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, rollout_path TEXT, title TEXT, updated_at_ms INTEGER)",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO threads VALUES (?1, '', ?2, ?3)",
            (id, title, updated_at_ms),
        )
        .unwrap();
    }

    fn thread_count(path: &Path, id: &str) -> i64 {
        let db = rusqlite::Connection::open(path).unwrap();
        db.query_row("SELECT COUNT(*) FROM threads WHERE id = ?1", [id], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap()
    }

    fn restore_codex_home(previous: Option<std::ffi::OsString>) {
        unsafe {
            if let Some(value) = previous {
                std::env::set_var("CODEX_HOME", value);
            } else {
                std::env::remove_var("CODEX_HOME");
            }
        }
    }

    #[test]
    fn apply_relay_profile_to_home_with_switch_rules_preserves_custom_provider_id() {
        let temp = tempfile::tempdir().unwrap();
        let profile = RelayProfile {
            relay_mode: claude_codex_pro_core::settings::RelayMode::PureApi,
            protocol: claude_codex_pro_core::settings::RelayProtocol::Responses,
            config_contents: "model_provider = \"ai\"\nmodel = \"gpt-image-2\"\n\n[model_providers.ai]\nname = \"ai\"\nwire_api = \"responses\"\nrequires_openai_auth = true\nbase_url = \"https://ahg.codes\"\n"
                .to_string(),
            auth_contents: "{}\n".to_string(),
            ..RelayProfile::default()
        };

        claude_codex_pro_core::relay_config::apply_relay_profile_to_home_with_switch_rules(
            temp.path(),
            &profile,
            "",
        )
        .unwrap();

        let applied = std::fs::read_to_string(temp.path().join("config.toml")).unwrap();
        assert!(applied.contains("model_provider = \"ai\""));
        assert!(applied.contains("[model_providers.ai]"));
        assert!(!applied.contains("[model_providers.custom]"));
    }

    #[test]
    fn save_relay_file_in_home_only_allows_known_files() {
        let temp = tempfile::tempdir().unwrap();

        save_relay_file_in_home(temp.path(), "config", "model = \"gpt-5\"\n").unwrap();
        save_relay_file_in_home(temp.path(), "auth", "{}\n").unwrap();

        assert_eq!(
            std::fs::read_to_string(temp.path().join("config.toml")).unwrap(),
            "model = \"gpt-5\"\n"
        );
        assert_eq!(
            std::fs::read_to_string(temp.path().join("auth.json")).unwrap(),
            "{}\n"
        );
        assert!(save_relay_file_in_home(temp.path(), "../bad", "").is_err());
    }

    #[test]
    fn normalize_settings_before_save_preserves_profile_context_until_manual_extract() {
        let settings = BackendSettings {
            relay_common_config_contents: "[mcp_servers.context7]\ncommand = \"npx\"\n".to_string(),
            relay_profiles: vec![RelayProfile {
                use_common_config: false,
                relay_mode: claude_codex_pro_core::settings::RelayMode::PureApi,
                config_contents: "model = \"gpt-5\"\n\n[mcp_servers.context7]\ncommand = \"npx\"\n"
                    .to_string(),
                ..RelayProfile::default()
            }],
            ..BackendSettings::default()
        };

        let normalized = normalize_settings_before_save(settings);

        assert!(
            normalized.relay_profiles[0]
                .config_contents
                .contains("model = \"gpt-5\"")
        );
        assert!(
            normalized.relay_profiles[0]
                .config_contents
                .contains("[mcp_servers.context7]")
        );
        assert!(
            normalized
                .relay_context_config_contents
                .contains("[mcp_servers.context7]")
        );
        assert!(
            !normalized
                .relay_common_config_contents
                .contains("[mcp_servers")
        );
    }

    #[test]
    fn reset_image_overlay_settings_preserves_supplier_settings() {
        let _guard = test_path_lock();
        let temp = tempfile::tempdir().unwrap();
        let settings_path = temp.path().join("settings.json");
        let previous =
            claude_codex_pro_core::paths::set_settings_path_for_tests(Some(settings_path));

        let settings = BackendSettings {
            codex_app_image_overlay_enabled: true,
            codex_app_image_overlay_path: "C:\\Users\\me\\Pictures\\overlay.png".to_string(),
            codex_app_image_overlay_opacity: 42,
            active_relay_id: "supplier-a".to_string(),
            relay_profiles: vec![RelayProfile {
                id: "supplier-a".to_string(),
                name: "濞撴碍绋戠花鏌ュ疮?A".to_string(),
                relay_mode: claude_codex_pro_core::settings::RelayMode::PureApi,
                api_key: "sk-test".to_string(),
                ..RelayProfile::default()
            }],
            ..BackendSettings::default()
        };
        SettingsStore::default().save(&settings).unwrap();

        let result = reset_image_overlay_settings();
        claude_codex_pro_core::paths::set_settings_path_for_tests(previous);

        assert_eq!(result.status, "ok");
        assert!(!result.payload.settings.codex_app_image_overlay_enabled);
        assert_eq!(result.payload.settings.codex_app_image_overlay_path, "");
        assert_eq!(result.payload.settings.codex_app_image_overlay_opacity, 35);
        assert_eq!(result.payload.settings.active_relay_id, "supplier-a");
        assert_eq!(result.payload.settings.relay_profiles.len(), 1);
        assert_eq!(result.payload.settings.relay_profiles[0].id, "supplier-a");
        assert_eq!(result.payload.settings.relay_profiles[0].api_key, "sk-test");
    }

    #[test]
    fn memory_assist_commands_respect_disabled_settings_before_writing() {
        let _guard = test_path_lock();
        let temp = tempfile::tempdir().unwrap();
        let settings_path = temp.path().join("settings.json");
        let memory_path = temp.path().join("memory.sqlite");
        let previous_settings =
            claude_codex_pro_core::paths::set_settings_path_for_tests(Some(settings_path));
        let previous_memory =
            claude_codex_pro_core::memory_assist::set_memory_assist_db_path_for_tests(Some(
                memory_path,
            ));

        let settings = BackendSettings {
            memory_assist_enabled: false,
            memory_assist_auto_suggest_enabled: false,
            ..BackendSettings::default()
        };
        SettingsStore::default().save(&settings).unwrap();
        let loaded = SettingsStore::default().load().unwrap();
        assert!(!loaded.memory_assist_enabled);
        assert!(!loaded.memory_assist_auto_suggest_enabled);

        let learned = learn_memory_assist_item(MemoryItemRequest {
            text: "should not persist".to_string(),
            workspace: "repo-a".to_string(),
            category: "manual".to_string(),
            tags: Vec::new(),
            source: "manager".to_string(),
            source_session_id: String::new(),
        });
        let candidate = create_memory_assist_candidate(MemoryCandidateRequest {
            text: "should not become candidate".to_string(),
            workspace: "repo-a".to_string(),
            category: "preference".to_string(),
            tags: Vec::new(),
            source: "manager".to_string(),
            reason: "test".to_string(),
            source_session_id: String::new(),
        });
        let status = load_memory_assist_status();

        claude_codex_pro_core::memory_assist::set_memory_assist_db_path_for_tests(previous_memory);
        claude_codex_pro_core::paths::set_settings_path_for_tests(previous_settings);

        assert_eq!(learned.status, "failed");
        assert!(!learned.message.is_empty());
        assert_eq!(candidate.status, "failed");
        assert!(!candidate.message.is_empty());
        assert_eq!(status.payload.memory.total_items, 0);
        assert_eq!(status.payload.memory.pending_candidates, 0);
    }

    #[test]
    fn memory_assist_candidate_command_respects_auto_suggest_disabled() {
        let _guard = test_path_lock();
        let temp = tempfile::tempdir().unwrap();
        let settings_path = temp.path().join("settings.json");
        let memory_path = temp.path().join("memory.sqlite");
        let previous_settings =
            claude_codex_pro_core::paths::set_settings_path_for_tests(Some(settings_path));
        let previous_memory =
            claude_codex_pro_core::memory_assist::set_memory_assist_db_path_for_tests(Some(
                memory_path,
            ));

        let settings = BackendSettings {
            memory_assist_enabled: true,
            memory_assist_auto_suggest_enabled: false,
            ..BackendSettings::default()
        };
        SettingsStore::default().save(&settings).unwrap();
        let loaded = SettingsStore::default().load().unwrap();
        assert!(loaded.memory_assist_enabled);
        assert!(!loaded.memory_assist_auto_suggest_enabled);

        let learned = learn_memory_assist_item(MemoryItemRequest {
            text: "manual memory still works".to_string(),
            workspace: "repo-a".to_string(),
            category: "manual".to_string(),
            tags: Vec::new(),
            source: "manager".to_string(),
            source_session_id: String::new(),
        });
        let candidate = create_memory_assist_candidate(MemoryCandidateRequest {
            text: "auto suggestion should not persist".to_string(),
            workspace: "repo-a".to_string(),
            category: "preference".to_string(),
            tags: Vec::new(),
            source: "manager".to_string(),
            reason: "test".to_string(),
            source_session_id: String::new(),
        });
        let status = load_memory_assist_status();

        claude_codex_pro_core::memory_assist::set_memory_assist_db_path_for_tests(previous_memory);
        claude_codex_pro_core::paths::set_settings_path_for_tests(previous_settings);

        assert_eq!(learned.status, "ok");
        assert_eq!(candidate.status, "failed");
        assert!(!candidate.message.is_empty());
        assert_eq!(status.payload.memory.total_items, 1);
        assert_eq!(status.payload.memory.pending_candidates, 0);
    }

    #[test]
    fn normalize_settings_before_save_preserves_official_profile_auth() {
        let settings = BackendSettings {
            relay_profiles: vec![RelayProfile {
                relay_mode: claude_codex_pro_core::settings::RelayMode::Official,
                official_mix_api_key: false,
                auth_contents: r#"{"auth_mode":"chatgpt","tokens":{"access_token":"edited"}}"#
                    .to_string(),
                config_contents: "model_provider = \"custom\"\n".to_string(),
                ..RelayProfile::default()
            }],
            ..BackendSettings::default()
        };

        let normalized = normalize_settings_before_save(settings);

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&normalized.relay_profiles[0].auth_contents)
                .unwrap(),
            serde_json::json!({"auth_mode":"chatgpt","tokens":{"access_token":"edited"}})
        );
        assert!(normalized.relay_profiles[0].config_contents.is_empty());
    }

    #[test]
    fn normalize_settings_before_save_strips_common_from_enabled_profile() {
        let settings = BackendSettings {
            relay_common_config_contents: r#"model_reasoning_effort = "high"

[features]
goals = true

[plugins."superpowers@openai-curated"]
enabled = true
"#
            .to_string(),
            relay_profiles: vec![RelayProfile {
                use_common_config: true,
                relay_mode: claude_codex_pro_core::settings::RelayMode::PureApi,
                config_contents: r#"model = "gpt-5"
model_reasoning_effort = "high"

[features]
goals = true
model_reasoning_effort = "high"

[plugins."superpowers@openai-curated"]
enabled = true
"#
                .to_string(),
                ..RelayProfile::default()
            }],
            ..BackendSettings::default()
        };

        let normalized = normalize_settings_before_save(settings);
        let config = &normalized.relay_profiles[0].config_contents;

        assert!(config.contains("model = \"gpt-5\""));
        assert!(!config.contains("model_reasoning_effort"));
        assert!(!config.contains("[features]"));
        assert!(!config.contains("[plugins.\"superpowers@openai-curated\"]"));
    }

    #[test]
    fn normalize_settings_before_save_repairs_invalid_profile_common_duplication() {
        let settings = BackendSettings {
            relay_common_config_contents: r#"model_reasoning_effort = "high"

[marketplaces.openai-bundled]
last_updated = "2026-05-25T11:52:46Z"
"#
            .to_string(),
            relay_profiles: vec![RelayProfile {
                use_common_config: true,
                relay_mode: claude_codex_pro_core::settings::RelayMode::PureApi,
                config_contents: r#"model = "gpt-5"
model_reasoning_effort = "high"

[marketplaces.openai-bundled]
last_updated = "2026-05-25T11:52:46Z"

[marketplaces.openai-bundled]
last_updated = "2026-05-25T11:52:46Z"
"#
                .to_string(),
                ..RelayProfile::default()
            }],
            ..BackendSettings::default()
        };

        let normalized = normalize_settings_before_save(settings);
        let config = &normalized.relay_profiles[0].config_contents;

        assert!(config.contains("model = \"gpt-5\""));
        assert!(!config.contains("model_reasoning_effort"));
        assert!(!config.contains("[marketplaces.openai-bundled]"));
    }

    #[test]
    fn normalize_settings_before_save_removes_model_catalog_from_common_config() {
        let settings = BackendSettings {
            relay_common_config_contents: r#"model_catalog_json = "C:\\Users\\Administrator\\.codex\\model-catalogs\\relay-a.json"
model_catalog_json = 'C:\Users\Administrator\.codex\model-catalogs\relay-b.json'
model_reasoning_effort = "high"
"#
            .to_string(),
            ..BackendSettings::default()
        };

        let normalized = normalize_settings_before_save(settings);

        assert!(
            !normalized
                .relay_common_config_contents
                .contains("model_catalog_json")
        );
        assert!(
            normalized
                .relay_common_config_contents
                .contains("model_reasoning_effort = \"high\"")
        );
    }

    #[test]
    fn context_entry_commands_update_settings_payload() {
        let settings = BackendSettings::default();
        let upsert = upsert_context_entry(ContextEntryRequest {
            settings: settings.clone(),
            kind: "mcp".to_string(),
            id: "context7".to_string(),
            toml_body: "command = \"npx\"\n".to_string(),
        });

        assert_eq!(upsert.status, "ok");
        assert!(
            upsert
                .payload
                .settings
                .relay_context_config_contents
                .contains("[mcp_servers.context7]")
        );

        let listed = list_context_entries(ContextSettingsRequest {
            settings: upsert.payload.settings.clone(),
        });
        assert_eq!(listed.payload.entries.mcp_servers[0].id, "context7");

        let deleted = delete_context_entry(ContextDeleteRequest {
            settings: upsert.payload.settings,
            kind: "mcp".to_string(),
            id: "context7".to_string(),
        });
        assert_eq!(deleted.status, "ok");
        assert!(
            !deleted
                .payload
                .settings
                .relay_context_config_contents
                .contains("[mcp_servers.context7]")
        );
    }

    #[test]
    fn ads_payload_keeps_version_and_ad_items() {
        let payload = ads_payload(json!({
            "version": 1,
            "ads": [{"id": "ad-1", "type": "normal", "title": "Ad"}]
        }));

        assert_eq!(payload.version, 1);
        assert_eq!(payload.ads.len(), 1);
        assert_eq!(payload.ads[0]["id"], json!("ad-1"));
    }

    #[test]
    fn open_external_url_rejects_non_http_urls() {
        let result = open_external_url("file:///C:/Windows/win.ini".to_string());

        assert_eq!(result.status, "failed");
        assert!(result.message.contains("Only http or https"));
    }
}
