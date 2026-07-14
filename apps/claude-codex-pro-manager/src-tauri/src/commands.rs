use std::collections::{BTreeMap, BTreeSet};
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
use claude_codex_pro_core::credential_environment::CredentialEnvironmentDiagnostic;
use claude_codex_pro_core::install::{MCP_BINARY, SILENT_BINARY};
use claude_codex_pro_core::memory_assist::{
    MemoryAssistMigrationRequest, MemoryAssistMigrationResult, MemoryAssistStatus,
    MemoryAssistStore, MemoryCandidate, MemoryCandidateRequest, MemoryCaptureProgressStatus,
    MemoryExport, MemoryImportRequest, MemoryItem, MemoryItemRequest, MemoryNewProjectGuide,
    MemoryOutcomeDashboard, MemoryQueryRequest, MemoryQueryResult, MemorySelfCheckRequest,
    MemorySelfCheckResult, MemorySessionRequest, MemorySessionSummary,
    migrate_memory_assist_data_dir as migrate_memory_assist_data_dir_core,
};
use claude_codex_pro_core::models::{DeleteResult, SessionRef};
use claude_codex_pro_core::plugin_hub::{
    self, ClaudeDesktopDevModeOutcome, ClaudeDesktopDevModeStatus, ClaudeDesktopMarketplaceOutcome,
    ClaudeDesktopMarketplaceStatus, ClaudeDesktopOrgPluginOutcome, ClaudeDesktopOrgPluginStatus,
    CodexHookTrustPreview, McpbPackageOutcome, PluginHubCatalog, PluginInstallOutcome,
    PluginInstallPreview,
};
use claude_codex_pro_core::script_market::{self, MarketScript, ScriptMarketManifest};
use claude_codex_pro_core::settings::{
    BackendSettings, RelayProfile, SettingsStore, relay_profile_resolved_api_key,
};
use claude_codex_pro_core::status::{LaunchStatus, StatusStore};
use claude_codex_pro_core::user_scripts::UserScriptManager;
use claude_codex_pro_core::zed_remote::{ZedOpenStrategy, ZedRemoteProject};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{Emitter, Manager};
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
const REPAIR_CODEX_FRONTEND_TIMEOUT: Duration = Duration::from_secs(45);
const REPAIR_CODEX_RESTART_TIMEOUT: Duration = Duration::from_secs(90);
const REPAIR_CODEX_PORT_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);

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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteClaudeSessionRequest {
    pub session_id: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadClaudeSessionContextRequest {
    pub session_id: String,
    pub source_path: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCodexSessionContextRequest {
    pub session_id: String,
    pub db_path: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteClaudeSessionPayload {
    pub session_id: String,
    pub backup_path: Option<String>,
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
pub struct UnifiedToolInventoryPayload {
    pub inventory: claude_codex_pro_core::unified_tool_inventory::UnifiedToolInventory,
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
pub struct UnifiedToolToggleRequest {
    pub id: String,
    pub kind: String,
    pub app: String,
    pub enabled: bool,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexCustomMarketplaceRequest {
    pub marketplace: claude_codex_pro_core::settings::CodexCustomMarketplace,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexCustomMarketplaceRemoveRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexCustomMarketplacesPayload {
    pub custom_marketplaces: Vec<claude_codex_pro_core::settings::CodexCustomMarketplace>,
    pub marketplace: claude_codex_pro_core::codex_plugin_marketplace::CodexPluginMarketplaceStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionExportRequest {
    pub session_id: String,
    #[serde(default)]
    pub db_path: Option<String>,
    pub format: claude_codex_pro_data::SessionExportFormat,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionExportPayload {
    pub export: Option<claude_codex_pro_data::SessionExport>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMigrationRequest {
    pub session_id: String,
    #[serde(default)]
    pub db_path: Option<String>,
    #[serde(default)]
    pub target_cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMigrationPayload {
    pub migration: Option<claude_codex_pro_data::ClaudeCodeMigration>,
    pub claude_code_available: bool,
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
#[serde(default)]
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
pub struct MemoryOutcomeDashboardPayload {
    pub dashboard: MemoryOutcomeDashboard,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryNewProjectGuidePayload {
    pub guide: MemoryNewProjectGuide,
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
pub struct MemoryOutcomeDashboardRequest {
    #[serde(default)]
    pub workspace: String,
    #[serde(default = "default_memory_outcome_range_days")]
    pub range_days: usize,
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
        "后端版本已加载。",
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
        "启动选项已加载。",
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

fn default_memory_outcome_range_days() -> usize {
    30
}

fn restrict_manager_memory_workspace(workspace: &str) -> String {
    let workspace = workspace.trim();
    if workspace.is_empty() || workspace == "__all__" {
        "global".to_string()
    } else {
        workspace.to_string()
    }
}

fn restrict_manager_memory_query(request: &mut MemoryQueryRequest) {
    request.workspace = restrict_manager_memory_workspace(&request.workspace);
    request.include_global = true;
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
            "加载概览的后台任务失败。",
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
        "概览已加载。",
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
            message: "Claude Desktop 完整性审计后台任务失败。".to_string(),
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
                        "Claude Desktop 代理回退端口 {fallback} 启动失败（首选端口 {preferred} 也已失败）：{first_error}"
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

pub(crate) async fn ensure_claude_desktop_proxy_on_startup() {
    match ensure_claude_desktop_proxy_helper().await {
        Ok(port) => log_manager_event(
            "manager.claude_proxy.startup_ok",
            json!({
                "port": port,
                "address": format!("http://127.0.0.1:{port}/claude-desktop")
            }),
        ),
        Err(error) => log_manager_event(
            "manager.claude_proxy.startup_failed",
            json!({
                "error": error.to_string()
            }),
        ),
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
            "Claude 汉化窗口已聚焦。",
            claude_chinese_window_payload(&app, &status),
        );
    }

    let url = match claude_chinese_window_shell_url(default_url) {
        Ok(url) => url,
        Err(error) => {
            return failed(
                &format!("Claude 汉化窗口 URL 无效：{error}"),
                claude_chinese_window_payload(&app, &status),
            );
        }
    };
    let handle = app.clone();
    let nav_handle = app.clone();
    let build_result = tauri::async_runtime::spawn_blocking(move || {
        tauri::WebviewWindowBuilder::new(&handle, label, tauri::WebviewUrl::External(url))
            .title("Claude 汉化")
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
                if url.scheme() == "claude-codex-pro" && url.host_str() == Some("open-external") {
                    let target = url
                        .query_pairs()
                        .find_map(|(key, value)| (key == "url").then(|| value.into_owned()));
                    if let Some(target) = target
                        && (target.starts_with("https://") || target.starts_with("http://"))
                    {
                        let _ = open_url(&target);
                    }
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
                "Claude 汉化窗口已打开。",
                claude_chinese_window_payload(&app, &status),
            )
        }
        Ok(Err(error)) => failed(
            &format!("Claude 汉化窗口打开失败：{error}"),
            claude_chinese_window_payload(&app, &status),
        ),
        Err(error) => failed(
            &format!("Claude 汉化后台任务失败：{error}"),
            claude_chinese_window_payload(&app, &status),
        ),
    }
}

fn claude_chinese_window_shell_url(default_url: &str) -> anyhow::Result<tauri::Url> {
    let html = claude_chinese_window_shell_html(default_url);
    Ok(tauri::Url::parse(&format!(
        "data:text/html;charset=utf-8,{}",
        percent_encode_data_url(&html)
    ))?)
}

fn claude_chinese_window_shell_html(default_url: &str) -> String {
    let escaped_url = html_escape(default_url);
    let encoded_url = percent_encode_query_component(default_url);
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'self' data: https://claude.ai https://*.claude.ai; frame-src https://claude.ai https://*.claude.ai; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src data: https:;">
  <title>Claude 加载诊断</title>
  <style>
    html, body {{ margin: 0; width: 100%; height: 100%; background: #090d18; color: #e5edff; font-family: Inter, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
    .shell {{ display: grid; grid-template-rows: auto minmax(0, 1fr); width: 100%; height: 100%; }}
    .banner {{ display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 14px 18px; border-bottom: 1px solid rgba(148, 163, 184, .24); background: linear-gradient(135deg, rgba(79, 70, 229, .28), rgba(20, 184, 166, .14)); box-shadow: 0 18px 50px rgba(0,0,0,.28); }}
    .title {{ font-size: 15px; font-weight: 850; }}
    .hint {{ margin-top: 4px; color: #b9c6dd; font-size: 12px; line-height: 1.5; }}
    .actions {{ display: flex; flex-wrap: wrap; gap: 8px; }}
    .button {{ display: inline-flex; align-items: center; justify-content: center; min-height: 34px; padding: 0 12px; border: 1px solid rgba(45, 212, 191, .56); border-radius: 999px; background: rgba(45, 212, 191, .14); color: #b5fff3; font-size: 12px; font-weight: 800; text-decoration: none; }}
    .frame-wrap {{ position: relative; min-height: 0; }}
    iframe {{ width: 100%; height: 100%; border: 0; background: #ffffff; }}
    .fallback {{ position: absolute; right: 18px; bottom: 18px; max-width: 360px; padding: 14px 16px; border: 1px solid rgba(248, 181, 75, .38); border-radius: 18px; background: rgba(15, 23, 42, .88); box-shadow: 0 22px 70px rgba(0,0,0,.35); color: #f8fafc; pointer-events: none; }}
    .fallback strong {{ display: block; margin-bottom: 6px; font-size: 13px; }}
    .fallback p {{ margin: 0; color: #cbd5e1; font-size: 12px; line-height: 1.55; }}
  </style>
</head>
<body>
  <main class="shell">
    <section class="banner">
      <div>
        <div class="title">Claude 加载中 / 白屏诊断</div>
        <div class="hint">如果下方区域保持空白，通常是 Claude 官方页面在 WebView 中被登录态、网络、CSP 或兼容性限制阻塞。请使用右侧按钮在系统浏览器打开。</div>
      </div>
      <div class="actions">
        <a class="button" href="claude-codex-pro://open-external?url={encoded_url}">在浏览器打开 Claude</a>
        <a class="button" href="{escaped_url}" target="_blank" rel="noreferrer">普通链接打开</a>
      </div>
    </section>
    <section class="frame-wrap">
      <iframe id="claude-frame" src="{escaped_url}" title="Claude"></iframe>
      <aside class="fallback" id="claude-frame-fallback">
        <strong>看见白屏时不要等待</strong>
        <p>这是本地诊断兜底层，说明管理工具没有卡死。请点击“在浏览器打开 Claude”，或回到管理工具使用“启动/重启Claude”。</p>
      </aside>
    </section>
  </main>
  <script>
    document.getElementById("claude-frame")?.addEventListener("load", () => {{
      const fallback = document.getElementById("claude-frame-fallback");
      if (fallback) fallback.hidden = true;
    }}, {{ once: true }});
  </script>
</body>
</html>"#
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn percent_encode_query_component(value: &str) -> String {
    percent_encode_bytes(value.as_bytes())
}

fn percent_encode_data_url(value: &str) -> String {
    percent_encode_bytes(value.as_bytes())
}

fn percent_encode_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len());
    for byte in bytes {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[tauri::command]
pub async fn open_plugin_hub_window(
    app: tauri::AppHandle,
) -> CommandResult<PluginHubWindowPayload> {
    match route_main_window_to_plugin_hub(&app) {
        Ok(()) => ok(
            "插件中心已在管理器中打开。",
            PluginHubWindowPayload {
                open: true,
                label: "main".to_string(),
            },
        ),
        Err(error) => failed(
            &format!("插件中心在管理器中打开失败：{error}"),
            PluginHubWindowPayload {
                open: false,
                label: "main".to_string(),
            },
        ),
    }
}

#[tauri::command]
pub async fn load_claude_chinese_window_status(
    app: tauri::AppHandle,
) -> CommandResult<ClaudeChineseWindowPayload> {
    let status = tauri::async_runtime::spawn_blocking(
        claude_codex_pro_core::claude_desktop::detect_status_light,
    )
    .await
    .unwrap_or_else(|_| claude_codex_pro_core::claude_desktop::detect_status_light());
    ok(
        "Claude 汉化状态已加载。",
        claude_chinese_window_payload(&app, &status),
    )
}

#[tauri::command]
pub async fn load_claude_zh_patch_status() -> CommandResult<ClaudeZhPatchPayload> {
    // detect_status() 会遍历读取并扫描 Claude Desktop 安装目录下大量 JS chunk，
    // 单次可达 20~30 秒。若作为同步命令直接在主线程执行，会阻塞窗口消息泵导致
    // 整个管理器窗口 "未响应"。改为 spawn_blocking 放到阻塞线程池，与
    // load_claude_desktop_status 保持同一套路，窗口在检测期间保持可交互。
    let status =
        tauri::async_runtime::spawn_blocking(claude_codex_pro_core::claude_zh_patch::detect_status)
            .await
            .unwrap_or_else(|_| claude_codex_pro_core::claude_zh_patch::detect_status());
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
            "打补丁前关闭 Claude Desktop 失败。请退出 Claude 后重试。",
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
                        &format!("Claude 汉化提权运行未完成：{}", status.message),
                        claude_zh_patch_payload(status, Vec::new()),
                    );
                }
                return complete_claude_zh_patch_install(result.message, status, Vec::new());
            }
            Ok(result) => {
                let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                return failed(
                    &format!("Claude 汉化提权运行失败：{}", result.message),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
            Err(error) => {
                let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                return failed(
                    &format!("Claude 汉化需要管理员授权，但提权失败：{error}"),
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
                                &format!("Claude 汉化提权回退运行未完成：{}", status.message),
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
                            &format!("Claude 汉化提权回退运行失败：{}", result.message),
                            claude_zh_patch_payload(status, Vec::new()),
                        );
                    }
                    Err(elevation_error) => {
                        let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                        return failed(
                            &format!(
                                "Claude 汉化需要管理员授权，但回退提权失败：{elevation_error}；直接执行错误：{error}"
                            ),
                            claude_zh_patch_payload(status, Vec::new()),
                        );
                    }
                }
            }
            let status = claude_codex_pro_core::claude_zh_patch::detect_status();
            failed(
                &format!("Claude 手动汉化失败：{error}"),
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
    let mut exit_code = if cli_result.status == "ok" { 0 } else { 1 };
    if let Some(path) = result_path {
        if let Ok(text) = serde_json::to_string(&cli_result) {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(error) = fs::write(&path, text) {
                exit_code = 1;
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
    std::process::exit(exit_code);
}

fn install_claude_zh_patch_internal(
    target_user_sid: Option<&str>,
    target_appdata: Option<&str>,
    target_localappdata: Option<&str>,
    target_install_root: Option<&str>,
) -> anyhow::Result<String> {
    if !claude_codex_pro_core::claude_desktop::close_claude_desktop_for_patch() {
        anyhow::bail!("汉化前关闭 Claude Desktop 失败");
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
    Ok("Claude 汉化补丁已安装。".to_string())
}

fn restore_claude_zh_patch_internal(
    target_user_sid: Option<&str>,
    target_appdata: Option<&str>,
    target_localappdata: Option<&str>,
    target_install_root: Option<&str>,
) -> anyhow::Result<String> {
    if !claude_codex_pro_core::claude_desktop::close_claude_desktop_for_patch() {
        anyhow::bail!("还原前关闭 Claude Desktop 失败");
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
    Ok("Claude 官方文件已还原。".to_string())
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
    fs::create_dir_all(&result_dir)
        .with_context(|| format!("创建 Claude 汉化结果目录失败：{}", result_dir.display()))?;
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
            "用户取消提权或提权子进程失败：{:?}；stdout={}；stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let contents = fs::read_to_string(&result_path).with_context(|| {
        format!(
            "提权子进程未写入结果文件：{}；stdout={}；stderr={}",
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
    use std::io::Read;

    let mut child = command.spawn()?;
    // Drain stdout/stderr on dedicated threads. The old code only polled
    // try_wait() and read the pipes after exit: if the elevated child wrote more
    // than the pipe buffer (~64 KiB) it would block on the write while we waited
    // for it to exit — a deadlock that only broke at the 5-minute timeout.
    let stdout_reader = child.stdout.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        })
    });
    let stderr_reader = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        })
    });

    let collect = |reader: Option<std::thread::JoinHandle<Vec<u8>>>| -> Vec<u8> {
        reader
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default()
    };

    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(std::process::Output {
                status,
                stdout: collect(stdout_reader),
                stderr: collect(stderr_reader),
            });
        }
        if started.elapsed() >= CLAUDE_ZH_PATCH_ELEVATED_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("Claude 汉化补丁提权执行超时。请确认已处理 UAC 提示后重试。");
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
            "手动打补丁前关闭 Claude Desktop 失败。请退出 Claude 后重试。",
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
                        &format!("Claude 手动汉化提权运行未完成：{}", status.message),
                        claude_zh_patch_payload(status, Vec::new()),
                    );
                }
                return complete_claude_zh_patch_install(result.message, status, Vec::new());
            }
            Ok(result) => {
                let status =
                    claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
                return failed(
                    &format!("Claude 手动汉化提权运行失败：{}", result.message),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
            Err(error) => {
                let status =
                    claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
                return failed(
                    &format!("Claude 手动汉化需要管理员授权，但提权失败：{error}"),
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
                                &format!("Claude 手动汉化提权回退运行未完成：{}", status.message),
                                claude_zh_patch_payload(status, Vec::new()),
                            );
                        }
                        return complete_claude_zh_patch_install(result.message, status, Vec::new());
                    }
                    Ok(result) => {
                        let status = claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
                        return failed(
                            &format!("Claude 手动汉化提权回退运行失败：{}", result.message),
                            claude_zh_patch_payload(status, Vec::new()),
                        );
                    }
                    Err(elevation_error) => {
                        let status = claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
                        return failed(
                            &format!("Claude 手动汉化需要管理员授权，但回退提权失败：{elevation_error}；直接执行错误：{error}"),
                            claude_zh_patch_payload(status, Vec::new()),
                        );
                    }
                }
            }
            let status = claude_codex_pro_core::claude_zh_patch::status_for_install_root(&install_root);
            failed(
                &format!("Claude 手动汉化失败：{error}"),
                claude_zh_patch_payload(status, Vec::new()),
            )
        }
    }
}

#[tauri::command]
pub async fn restore_claude_zh_patch() -> CommandResult<ClaudeZhPatchPayload> {
    // Restore closes Claude Desktop (kill + wait), scans files for status, and may
    // poll an elevated PowerShell process for up to ~5 minutes. On the UI thread
    // that froze the whole WebView for the duration; run it on the blocking pool.
    tauri::async_runtime::spawn_blocking(restore_claude_zh_patch_blocking)
        .await
        .unwrap_or_else(|join_error| {
            let status = claude_codex_pro_core::claude_zh_patch::detect_status();
            failed(
                &format!("Claude 汉化还原任务失败：{join_error}"),
                claude_zh_patch_payload(status, Vec::new()),
            )
        })
}

fn restore_claude_zh_patch_blocking() -> CommandResult<ClaudeZhPatchPayload> {
    log_manager_event("manager.claude_zh_patch.restore.start", json!({}));
    if !claude_codex_pro_core::claude_desktop::close_claude_desktop_for_patch() {
        log_manager_event(
            "manager.claude_zh_patch.restore.close_claude_failed",
            json!({}),
        );
        let status = claude_codex_pro_core::claude_zh_patch::detect_status();
        return failed(
            "还原前关闭 Claude Desktop 失败。请退出 Claude 后重试。",
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
                            "Claude 官方文件还原提权运行后仍残留汉化文件：{}",
                            status.message
                        ),
                        claude_zh_patch_payload(status, Vec::new()),
                    );
                }
                return ok(
                    &format!("{} 请重启 Claude Desktop。", result.message),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
            Ok(result) => {
                let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                return failed(
                    &format!("Claude 官方文件还原提权运行失败：{}", result.message),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
            Err(error) => {
                let status = claude_codex_pro_core::claude_zh_patch::detect_status();
                return failed(
                    &format!("Claude 官方文件还原需要管理员授权，但提权失败：{error}"),
                    claude_zh_patch_payload(status, Vec::new()),
                );
            }
        }
    }
    log_manager_event("manager.claude_zh_patch.restore.direct.start", json!({}));
    match claude_codex_pro_core::claude_zh_patch::restore_patch() {
        Ok(outcome) => ok(
            "Claude 官方文件已从备份还原。",
            claude_zh_patch_payload(outcome.status, outcome.changed_files),
        ),
        Err(error) => {
            let status = claude_codex_pro_core::claude_zh_patch::detect_status();
            failed(
                &format!("Claude 汉化还原失败：{error}"),
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
    let repair_started_ms = current_time_ms();
    details.push("已请求重启 Codex 注入入口，旧前端心跳不会作为本次修复成功依据。".to_string());
    let latest = restart_codex_for_frontend_repair(&mut details).await;

    let mut codex_backend_online = latest
        .as_ref()
        .is_some_and(|status| status.helper_port_online);
    let codex_frontend_ok = if let Some(status) = latest.as_ref() {
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
                } else {
                    if !status.helper_port_online {
                        details.push(format!(
                            "Codex 后端 127.0.0.1:{helper_port}/backend/status 未在线，正在自动启动本地 helper 后端。"
                        ));
                        match claude_codex_pro_core::launcher::ensure_detached_helper(helper_port)
                            .await
                        {
                            Ok(()) => {
                                codex_backend_online =
                                    wait_helper_backend_online(helper_port).await;
                                if codex_backend_online {
                                    details.push(format!(
                                        "Codex 后端已在 127.0.0.1:{helper_port}/backend/status 验证在线。"
                                    ));
                                } else {
                                    details.push(format!(
                                        "已请求启动 Codex 后端，但 127.0.0.1:{helper_port}/backend/status 尚未响应。"
                                    ));
                                }
                            }
                            Err(error) => {
                                codex_backend_online = false;
                                details.push(format!("自动启动 Codex 后端失败：{error}"));
                            }
                        }
                    }
                    if !codex_backend_online {
                        return CommandResult {
                            status: "failed".to_string(),
                            message: "Codex 前端连接修复未确认可注入，请查看详情。".to_string(),
                            payload: RepairConnectionPayload {
                                target: "codex".to_string(),
                                frontend_injected: false,
                                backend_online: false,
                                codex_frontend_injected: false,
                                codex_backend_online,
                                claude_backend_online: false,
                                debug_port: Some(debug_port),
                                helper_port: Some(helper_port),
                                claude_proxy_port: None,
                                details,
                            },
                        };
                    }
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
                    if !reinjected {
                        false
                    } else if let Some(heartbeat) = wait_for_renderer_frontend_after(
                        repair_started_ms,
                        REPAIR_CODEX_FRONTEND_TIMEOUT,
                    )
                    .await
                    {
                        if heartbeat.runtime_reported {
                            details.push(format!(
                                "Codex 前端运行时已在本次修复后重新上报，时间戳 {}。",
                                heartbeat.timestamp_ms
                            ));
                        } else {
                            details.push(format!(
                                "Codex 前端脚本已在本次修复后加载，时间戳 {}；盘古记忆运行时将在页面同步后继续上报。",
                                heartbeat.timestamp_ms
                            ));
                        }
                        true
                    } else {
                        details.push("未等到本次修复后的 Codex 前端脚本或运行时新心跳；旧注入状态不会被判定为成功。".to_string());
                        false
                    }
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
    details.push("正在关闭旧 Codex 与 claude-codex-pro.exe 启动器进程。".to_string());
    let Some(app_path) = current_codex_app_path_for_launch() else {
        details.push("未找到 Codex 应用路径，无法自动重启 Codex。".to_string());
        return StatusStore::default()
            .load_latest()
            .ok()
            .flatten()
            .map(refresh_launch_port_status);
    };

    let old_launcher_pids = claude_codex_pro_core::watcher::find_restartable_launcher_processes();
    let old_codex_pids = claude_codex_pro_core::watcher::find_codex_processes();
    let mut old_process_pids = old_launcher_pids;
    old_process_pids.extend(old_codex_pids);
    old_process_pids.sort_unstable();
    old_process_pids.dedup();
    let stopped_launchers =
        claude_codex_pro_core::watcher::stop_launcher_processes_for_codex_restart();
    let stopped_codex = claude_codex_pro_core::watcher::stop_codex_processes();
    details.push(format!(
        "已请求结束 {stopped_codex} 个 Codex 进程、{stopped_launchers} 个启动器进程。"
    ));
    if !old_process_pids.is_empty() {
        if !wait_for_processes_to_exit_async(&old_process_pids, Duration::from_secs(3)).await {
            details.push(format!(
                "旧 launcher/Codex 进程未按预期退出，正在强制结束 PID：{}。",
                old_process_pids
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            let killed = force_kill_process_tree_for_frontend_repair(&old_process_pids);
            details.push(format!(
                "已发起 taskkill 兜底结束 {killed} 个旧 launcher/Codex 进程。"
            ));
            if !wait_for_processes_to_exit_async(&old_process_pids, Duration::from_secs(8)).await {
                details.push(
                    "旧 launcher/Codex 进程仍未退出，本次不会继续启动新 Codex，避免复用旧注入状态。".to_string(),
                );
                return StatusStore::default()
                    .load_latest()
                    .ok()
                    .flatten()
                    .map(refresh_launch_port_status);
            }
        }
        details.push("旧 launcher/Codex 进程已确认退出，开始启动新的 Codex。".to_string());
    } else {
        tokio::time::sleep(Duration::from_millis(800)).await;
    }

    let selected_debug_port = select_repair_debug_port(default_debug_port()).await;
    if selected_debug_port != default_debug_port() {
        details.push(format!(
            "首选 CDP 端口 {} 尚未释放，本次修复改用可用端口 {selected_debug_port}。",
            default_debug_port()
        ));
    }

    let request = LaunchRequest {
        app_path: app_path.to_string_lossy().to_string(),
        debug_port: selected_debug_port,
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

    details.push("已启动 Codex，正在等待 Codex 自启完成、CDP 与后端端口上线。".to_string());
    if let Some(status) = wait_for_codex_launch_ports(&request, REPAIR_CODEX_RESTART_TIMEOUT).await
    {
        if status.helper_port_online {
            details.push(format!(
                "Codex 自启完成，CDP 端口 {debug_port} 与后端端口 {helper_port} 已上线。",
                debug_port = status.debug_port.unwrap_or(request.debug_port),
                helper_port = status.helper_port.unwrap_or(request.helper_port)
            ));
        } else {
            details.push(format!(
                "Codex 自启完成，CDP 端口 {debug_port} 已上线；后端端口 {helper_port} 将继续自动修复。",
                debug_port = status.debug_port.unwrap_or(request.debug_port),
                helper_port = status.helper_port.unwrap_or(request.helper_port)
            ));
        }
        return Some(status);
    }

    details.push("已发起 Codex 自动重启，但等待自启完成、CDP / 后端端口上线超时。".to_string());
    StatusStore::default()
        .load_latest()
        .ok()
        .flatten()
        .map(refresh_launch_port_status)
}

async fn wait_for_processes_to_exit_async(pids: &[u32], timeout: Duration) -> bool {
    let pids = pids.to_vec();
    tauri::async_runtime::spawn_blocking(move || {
        claude_codex_pro_core::watcher::wait_for_processes_to_exit(&pids, timeout)
    })
    .await
    .unwrap_or(false)
}

async fn select_repair_debug_port(requested: u16) -> u16 {
    let started = Instant::now();
    while started.elapsed() < REPAIR_CODEX_PORT_RELEASE_TIMEOUT {
        if claude_codex_pro_core::ports::can_bind_loopback_port(requested) {
            return requested;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    select_repair_debug_port_with(
        requested,
        claude_codex_pro_core::ports::can_bind_loopback_port,
        claude_codex_pro_core::ports::find_available_loopback_port,
    )
}

fn select_repair_debug_port_with(
    requested: u16,
    can_bind: impl Fn(u16) -> bool,
    find_available: impl Fn() -> u16,
) -> u16 {
    if can_bind(requested) {
        return requested;
    }
    match find_available() {
        0 => requested,
        available => available,
    }
}

#[cfg(windows)]
fn force_kill_process_tree_for_frontend_repair(pids: &[u32]) -> usize {
    let mut killed = 0;
    for pid in pids {
        let status = std::process::Command::new("taskkill.exe")
            .args(["/PID", &pid.to_string(), "/F", "/T"])
            .status();
        if status.as_ref().is_ok_and(|status| status.success()) {
            killed += 1;
        }
    }
    killed
}

#[cfg(not(windows))]
fn force_kill_process_tree_for_frontend_repair(_pids: &[u32]) -> usize {
    0
}

async fn wait_for_codex_launch_ports(
    request: &LaunchRequest,
    timeout: Duration,
) -> Option<LaunchStatus> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let latest = StatusStore::default()
            .load_latest()
            .ok()
            .flatten()
            .map(refresh_launch_port_status);
        if let Some(status) = repair_launch_status(
            request,
            latest,
            codex_debug_port_online(request.debug_port),
            helper_backend_online(request.helper_port),
            current_time_ms(),
        ) {
            return Some(status);
        }
        tokio::time::sleep(Duration::from_millis(750)).await;
    }
    None
}

fn repair_launch_status(
    request: &LaunchRequest,
    latest: Option<LaunchStatus>,
    requested_debug_port_online: bool,
    helper_port_online: bool,
    detected_at_ms: u64,
) -> Option<LaunchStatus> {
    if let Some(status) = latest
        .filter(|status| status.debug_port == Some(request.debug_port) && status.debug_port_online)
    {
        return Some(status);
    }
    if !requested_debug_port_online {
        return None;
    }
    Some(LaunchStatus {
        status: if helper_port_online {
            "ok".to_string()
        } else {
            "running_degraded".to_string()
        },
        message: if helper_port_online {
            "前端修复期间检测到 Codex 启动端口。".to_string()
        } else {
            "前端修复期间检测到 Codex CDP 已上线，helper 后端仍需恢复。".to_string()
        },
        started_at_ms: detected_at_ms,
        codex_app: Some(request.app_path.clone()),
        debug_port: Some(request.debug_port),
        helper_port: Some(request.helper_port),
        debug_port_online: true,
        helper_port_online,
        frontend_runtime_online: false,
        frontend_runtime_seen_at_ms: None,
    })
}

async fn wait_for_renderer_frontend_after(
    min_timestamp_ms: u64,
    timeout: Duration,
) -> Option<RendererRuntimeHeartbeat> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if let Some(heartbeat) = latest_renderer_runtime_heartbeat() {
            if heartbeat.timestamp_ms >= min_timestamp_ms
                && renderer_frontend_heartbeat_confirms_injection(&heartbeat)
            {
                return Some(heartbeat);
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    None
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
    let codex_helper =
        match claude_codex_pro_core::launcher::ensure_detached_helper(helper_port).await {
            Ok(()) => {
                let online = wait_helper_backend_online(helper_port).await;
                details.push(if online {
                format!("Codex 后端已在 127.0.0.1:{helper_port}/backend/status 验证在线。")
            } else {
                format!(
                    "已请求启动 Codex 后端，但 127.0.0.1:{helper_port}/backend/status 尚未响应。"
                )
            });
                online
            }
            Err(error) => {
                details.push(format!("Codex 后端启动失败：{error}"));
                false
            }
        };
    let mut claude_proxy_port = current_claude_desktop_proxy_port_hint();
    let claude_helper = match ensure_claude_desktop_proxy_helper().await {
        Ok(port) => {
            claude_proxy_port = port;
            let online = wait_helper_backend_online(port).await;
            details.push(if online {
                format!("Claude 本地模型代理已在 127.0.0.1:{port}/backend/status 验证在线。")
            } else {
                format!(
                    "已请求启动 Claude 本地模型代理，但 127.0.0.1:{port}/backend/status 尚未响应。"
                )
            });
            online
        }
        Err(error) => {
            details.push(format!("Claude 本地模型代理启动失败：{error}"));
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
            "ok" => "后端服务已修复；Codex 与 Claude 前端可以重新连接。".to_string(),
            "degraded" => "后端服务已部分修复；请查看详情了解仍离线的一侧。".to_string(),
            _ => "后端服务修复失败；请查看诊断日志。".to_string(),
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
pub async fn launch_claude_codex_pro(request: LaunchRequest) -> CommandResult<Value> {
    match tauri::async_runtime::spawn_blocking(move || normalize_launch_request(request)).await {
        Ok(request) => spawn_claude_codex_pro_launch(request, "启动任务已在后台运行。"),
        Err(error) => failed(&format!("启动 Codex 任务失败：{error}"), json!({})),
    }
}

#[tauri::command]
pub async fn restart_claude_codex_pro(request: LaunchRequest) -> CommandResult<Value> {
    // Both normalize_launch_request (app-path probing) and the two stop_* calls
    // enumerate/kill processes — on Windows those go through taskkill/WMI and can
    // take seconds. Running them on the UI thread froze the WebView during a
    // restart, so move the whole teardown onto the blocking pool.
    let prepared =
        tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<LaunchRequest> {
            let request = normalize_launch_request(request);
            let mut old_process_pids =
                claude_codex_pro_core::watcher::find_restartable_launcher_processes();
            old_process_pids.extend(claude_codex_pro_core::watcher::find_codex_processes());
            old_process_pids.sort_unstable();
            old_process_pids.dedup();
            claude_codex_pro_core::watcher::stop_launcher_processes_for_codex_restart();
            claude_codex_pro_core::watcher::stop_codex_processes();
            if !claude_codex_pro_core::watcher::wait_for_processes_to_exit(
                &old_process_pids,
                Duration::from_secs(8),
            ) {
                anyhow::bail!("旧进程未能及时退出，本次不会启动新的 Codex")
            }
            Ok(request)
        })
        .await;
    match prepared {
        Ok(Ok(request)) => spawn_claude_codex_pro_launch(request, "重启 Codex 任务已在后台运行。"),
        Ok(Err(error)) => failed(&format!("重启 Codex 任务失败：{error}"), json!({})),
        Err(error) => failed(&format!("重启 Codex 任务失败：{error}"), json!({})),
    }
}

fn normalize_launch_request(mut request: LaunchRequest) -> LaunchRequest {
    let requested = request.app_path.trim().to_string();
    if !requested.is_empty() {
        if let Some(path) = codex_launch_app_path_from_candidate(Path::new(&requested)) {
            request.app_path = path.to_string_lossy().to_string();
            return request;
        }
        let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
            "manager.launch_path_stale",
            json!({ "app_path": requested }),
        );
    }
    if let Some(path) = current_codex_app_path_for_launch() {
        request.app_path = path.to_string_lossy().to_string();
    }
    request
}

fn current_codex_app_path_for_launch() -> Option<PathBuf> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    claude_codex_pro_core::app_paths::find_running_codex_app_dir()
        .and_then(|path| codex_launch_app_path_from_candidate(&path))
        .or_else(|| {
            StatusStore::default()
                .load_latest()
                .ok()
                .flatten()
                .and_then(|status| status.codex_app)
                .and_then(|path| codex_launch_app_path_from_candidate(Path::new(&path)))
        })
        .or_else(|| {
            let saved = settings.codex_app_path.trim();
            (!saved.is_empty())
                .then(|| codex_launch_app_path_from_candidate(Path::new(saved)))
                .flatten()
        })
        .or_else(|| {
            claude_codex_pro_core::app_paths::resolve_codex_app_dir(None)
                .and_then(|path| codex_launch_app_path_from_candidate(&path))
        })
}

fn codex_launch_app_path_from_candidate(path: &Path) -> Option<PathBuf> {
    let normalized = claude_codex_pro_core::app_paths::normalize_codex_app_path(path)?;
    let executable = claude_codex_pro_core::app_paths::build_codex_executable(&normalized);
    executable.exists().then_some(normalized)
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
        .map_err(|error| anyhow::anyhow!("启动 {} 失败：{error}", launcher.to_string_lossy()))
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
    // 已安装场景下 NSIS 会把 claude-codex-pro.exe 与管理器放在同一 $INSTDIR；
    // dev 场景下 beforeDevCommand 会先 `cargo build -p claude-codex-pro-launcher`
    // 把它产到 target/debug。两种入口都缺失时，多半是直接跑了 manager.exe 却
    // 没先编译 launcher。给出可执行的恢复指引，而不是只抛一串搜索路径。
    bail!(
        "未找到静默启动器 {launcher_name}。开发环境请先运行 \
         `cargo build -p claude-codex-pro-launcher --bin claude-codex-pro`（或直接 `npm run dev`），\
         已安装环境请重新运行安装包修复。已搜索路径：{searched}"
    )
}

#[tauri::command]
pub async fn load_settings() -> CommandResult<SettingsPayload> {
    tauri::async_runtime::spawn_blocking(|| settings_payload("设置已加载。", "加载设置失败。"))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("加载设置任务失败：{join_error}"),
                fallback_settings_payload(),
            )
        })
}

#[tauri::command]
pub async fn save_settings(settings: BackendSettings) -> CommandResult<SettingsPayload> {
    // Saving normalizes the settings, writes settings.json, and (via
    // refresh_cli_wrapper_after_settings_save) may write CLI wrapper files — all
    // synchronous disk IO that must stay off the UI thread.
    tauri::async_runtime::spawn_blocking(move || save_settings_blocking(settings))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("保存设置失败：{join_error}"),
                SettingsPayload {
                    settings: BackendSettings::default(),
                    settings_path: claude_codex_pro_core::paths::default_settings_path()
                        .to_string_lossy()
                        .to_string(),
                    user_scripts: user_script_inventory(),
                },
            )
        })
}

fn save_settings_blocking(settings: BackendSettings) -> CommandResult<SettingsPayload> {
    let settings = normalize_settings_before_save(settings);
    match SettingsStore::default().save(&settings) {
        Ok(()) => {
            let wrapper_message = refresh_cli_wrapper_after_settings_save(&settings);
            settings_payload(
                &format!("设置已保存。{wrapper_message}"),
                "保存后重新加载设置失败。",
            )
        }
        Err(error) => failed(
            &format!("保存设置失败：{error}"),
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
pub async fn list_local_sessions() -> CommandResult<LocalSessionsPayload> {
    // Enumerates and reads every Codex session DB (SQLite) plus rollout paths.
    // On the UI thread this froze the session list on machines with large
    // histories; move the whole scan onto the blocking pool.
    tauri::async_runtime::spawn_blocking(list_local_sessions_blocking)
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("读取本地会话失败：{join_error}"),
                LocalSessionsPayload {
                    db_path: String::new(),
                    db_paths: Vec::new(),
                    sessions: Vec::new(),
                },
            )
        })
}

#[tauri::command]
pub async fn load_codex_session_context(
    request: LoadCodexSessionContextRequest,
) -> CommandResult<claude_codex_pro_data::CodexSessionContextPage> {
    let fallback = empty_codex_session_context(&request);
    tauri::async_runtime::spawn_blocking(move || load_codex_session_context_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("读取 Codex 会话上下文任务失败：{join_error}"),
                fallback,
            )
        })
}

fn load_codex_session_context_blocking(
    request: LoadCodexSessionContextRequest,
) -> CommandResult<claude_codex_pro_data::CodexSessionContextPage> {
    let session_id = request.session_id.trim();
    if session_id.is_empty() {
        return failed(
            "Codex 会话 ID 不能为空。",
            empty_codex_session_context(&request),
        );
    }
    let candidates = session_candidate_db_paths(None);
    let db_path = match request
        .db_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(requested) => {
            let requested = PathBuf::from(requested);
            if !candidates.iter().any(|candidate| candidate == &requested) {
                return failed(
                    "Codex 会话数据库不是受信任的已发现路径。",
                    empty_codex_session_context(&request),
                );
            }
            requested
        }
        None => candidates
            .into_iter()
            .find(|candidate| candidate.exists())
            .unwrap_or_default(),
    };
    match claude_codex_pro_data::load_codex_session_context(
        &db_path,
        session_id,
        request.offset,
        request.limit,
    ) {
        Ok(Some(page)) => ok(
            &format!("已加载 {} 条 Codex 会话消息。", page.messages.len()),
            page,
        ),
        Ok(None) => failed(
            "未找到对应的 Codex 会话或 rollout 文件。",
            empty_codex_session_context(&request),
        ),
        Err(error) => failed(
            &format!("读取 Codex 会话上下文失败：{error}"),
            empty_codex_session_context(&request),
        ),
    }
}

fn empty_codex_session_context(
    request: &LoadCodexSessionContextRequest,
) -> claude_codex_pro_data::CodexSessionContextPage {
    claude_codex_pro_data::CodexSessionContextPage {
        session_id: request.session_id.clone(),
        title: String::new(),
        cwd: String::new(),
        db_path: request.db_path.clone().unwrap_or_default(),
        rollout_path: String::new(),
        total_messages: 0,
        offset: request.offset.unwrap_or_default(),
        messages: Vec::new(),
        has_more_before: false,
    }
}

#[tauri::command]
pub async fn list_claude_sessions()
-> CommandResult<claude_codex_pro_core::claude_sessions::ClaudeSessionsInventory> {
    log_manager_event("manager.list_claude_sessions.start", json!({}));
    tauri::async_runtime::spawn_blocking(list_claude_sessions_blocking)
        .await
        .unwrap_or_else(|join_error| {
            let message = format!("读取 Claude 会话任务失败：{join_error}");
            log_manager_event(
                "manager.list_claude_sessions.finish",
                json!({ "status": "failed", "message": message }),
            );
            failed(&message, empty_claude_sessions_inventory())
        })
}

fn list_claude_sessions_blocking()
-> CommandResult<claude_codex_pro_core::claude_sessions::ClaudeSessionsInventory> {
    match claude_codex_pro_core::claude_sessions::list_claude_sessions() {
        Ok(inventory) => {
            let warning_count = inventory.warnings.len();
            let message = if warning_count == 0 {
                format!(
                    "已从 {} 个来源加载 {} 条 Claude 会话。",
                    inventory.source_paths.len(),
                    inventory.sessions.len()
                )
            } else {
                format!(
                    "已从 {} 个来源加载 {} 条 Claude 会话，另有 {} 个来源需要检查。",
                    inventory.source_paths.len(),
                    inventory.sessions.len(),
                    warning_count
                )
            };
            log_manager_event(
                "manager.list_claude_sessions.finish",
                json!({
                    "status": if warning_count == 0 { "ok" } else { "needs_review" },
                    "source_count": inventory.source_paths.len(),
                    "session_count": inventory.sessions.len(),
                    "warning_count": warning_count,
                }),
            );
            CommandResult {
                status: if warning_count == 0 {
                    "ok".to_string()
                } else {
                    "needs_review".to_string()
                },
                message,
                payload: inventory,
            }
        }
        Err(error) => {
            let message = format!("读取 Claude 会话失败：{error}");
            log_manager_event(
                "manager.list_claude_sessions.finish",
                json!({ "status": "failed", "message": message }),
            );
            failed(&message, empty_claude_sessions_inventory())
        }
    }
}

fn empty_claude_sessions_inventory()
-> claude_codex_pro_core::claude_sessions::ClaudeSessionsInventory {
    claude_codex_pro_core::claude_sessions::ClaudeSessionsInventory {
        source_root: String::new(),
        source_paths: Vec::new(),
        sessions: Vec::new(),
        warnings: Vec::new(),
    }
}

#[tauri::command]
pub async fn load_claude_session_context(
    request: LoadClaudeSessionContextRequest,
) -> CommandResult<claude_codex_pro_core::claude_sessions::ClaudeSessionContextPage> {
    let fallback = empty_claude_session_context(&request);
    tauri::async_runtime::spawn_blocking(move || load_claude_session_context_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            let message = format!("读取 Claude 会话上下文任务失败：{join_error}");
            failed(&message, fallback)
        })
}

fn load_claude_session_context_blocking(
    request: LoadClaudeSessionContextRequest,
) -> CommandResult<claude_codex_pro_core::claude_sessions::ClaudeSessionContextPage> {
    let session_id = request.session_id.trim();
    let source_path = request.source_path.trim();
    if session_id.is_empty() || source_path.is_empty() {
        return failed(
            "Claude 会话 ID 和来源路径不能为空。",
            empty_claude_session_context(&request),
        );
    }
    log_manager_event(
        "manager.load_claude_session_context.start",
        json!({
            "session_id": session_id,
            "offset": request.offset,
            "limit": request.limit,
        }),
    );
    match claude_codex_pro_core::claude_sessions::load_claude_session_context(
        session_id,
        Path::new(source_path),
        request.offset,
        request.limit,
    ) {
        Ok(page) => {
            log_manager_event(
                "manager.load_claude_session_context.finish",
                json!({
                    "session_id": session_id,
                    "status": "ok",
                    "offset": page.offset,
                    "message_count": page.messages.len(),
                    "total_messages": page.total_messages,
                }),
            );
            ok(
                &format!(
                    "已加载 Claude 会话上下文：本页 {} 条，共 {} 条。",
                    page.messages.len(),
                    page.total_messages
                ),
                page,
            )
        }
        Err(error) => {
            let message = format!("读取 Claude 会话上下文失败：{error}");
            log_manager_event(
                "manager.load_claude_session_context.finish",
                json!({
                    "session_id": session_id,
                    "status": "failed",
                    "offset": request.offset,
                    "limit": request.limit,
                    "message": message,
                }),
            );
            failed(&message, empty_claude_session_context(&request))
        }
    }
}

fn empty_claude_session_context(
    request: &LoadClaudeSessionContextRequest,
) -> claude_codex_pro_core::claude_sessions::ClaudeSessionContextPage {
    claude_codex_pro_core::claude_sessions::ClaudeSessionContextPage {
        session_id: request.session_id.clone(),
        title: String::new(),
        cwd: String::new(),
        source_path: request.source_path.clone(),
        source_kind: String::new(),
        total_messages: 0,
        offset: request.offset.unwrap_or(0),
        messages: Vec::new(),
        has_more_before: false,
    }
}

fn list_local_sessions_blocking() -> CommandResult<LocalSessionsPayload> {
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
            &format!("已加载 {} 条本地会话。", payload.sessions.len()),
            payload,
        )
    } else {
        failed(
            &format!("部分本地会话读取失败：{}", errors.join("; ")),
            payload,
        )
    }
}

#[tauri::command]
pub async fn load_memory_assist_status() -> CommandResult<MemoryAssistStatusPayload> {
    // This runs SQLite queries and, via enrich_memory_status, two blocking
    // block_on CDP round-trips. On the UI thread those freeze the whole WebView
    // whenever the status panel polls (and worse the larger the log grows), so
    // move the whole thing onto the blocking pool.
    let computed = tauri::async_runtime::spawn_blocking(|| {
        MemoryAssistStore::default()
            .status()
            .map(enrich_memory_status)
    })
    .await;
    match computed {
        Ok(Ok(memory)) => ok("盘古记忆状态已加载。", MemoryAssistStatusPayload { memory }),
        Ok(Err(error)) => failed(
            &format!("加载盘古记忆状态失败：{error}"),
            MemoryAssistStatusPayload {
                memory: empty_memory_status(),
            },
        ),
        Err(error) => failed(
            &format!("加载盘古记忆状态失败：{error}"),
            MemoryAssistStatusPayload {
                memory: empty_memory_status(),
            },
        ),
    }
}

#[tauri::command]
pub async fn migrate_memory_assist_data_dir(
    request: MemoryAssistMigrationRequest,
) -> Result<MemoryAssistMigrationResult, String> {
    match tauri::async_runtime::spawn_blocking(move || migrate_memory_assist_data_dir_core(request))
        .await
    {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(error)) => Err(error.to_string()),
        Err(error) => Err(format!("迁移盘古记忆数据失败：{error}")),
    }
}

#[tauri::command]
pub async fn query_memory_assist(
    mut request: MemoryQueryRequest,
) -> CommandResult<MemoryAssistQueryPayload> {
    // SQLite query with keyword scoring/ranking; keep it off the UI thread.
    restrict_manager_memory_query(&mut request);
    let query = request.query.clone();
    let workspace = request.workspace.clone();
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().query_with_activity(request, "manager", "search", None)
    })
    .await;
    match computed {
        Ok(Ok(memory)) => ok("记忆查询已完成。", MemoryAssistQueryPayload { memory }),
        Ok(Err(error)) => failed(
            &format!("记忆查询失败：{error}"),
            MemoryAssistQueryPayload {
                memory: MemoryQueryResult {
                    query,
                    workspace,
                    results: Vec::new(),
                },
            },
        ),
        Err(error) => failed(
            &format!("记忆查询任务失败：{error}"),
            MemoryAssistQueryPayload {
                memory: MemoryQueryResult {
                    query,
                    workspace,
                    results: Vec::new(),
                },
            },
        ),
    }
}

#[tauri::command]
pub async fn load_memory_outcome_dashboard(
    request: MemoryOutcomeDashboardRequest,
) -> CommandResult<MemoryOutcomeDashboardPayload> {
    let workspace = restrict_manager_memory_workspace(&request.workspace);
    let range_days = if request.range_days <= 7 { 7 } else { 30 };
    let empty_dashboard = || MemoryOutcomeDashboard {
        workspace: workspace.clone(),
        range_days,
        ..MemoryOutcomeDashboard::default()
    };
    let requested_workspace = workspace.clone();
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().outcome_dashboard(&requested_workspace, range_days)
    })
    .await;
    match computed {
        Ok(Ok(dashboard)) => ok(
            "盘古记忆成果看板已加载。",
            MemoryOutcomeDashboardPayload { dashboard },
        ),
        Ok(Err(error)) => failed(
            &format!("加载盘古记忆成果看板失败：{error}"),
            MemoryOutcomeDashboardPayload {
                dashboard: empty_dashboard(),
            },
        ),
        Err(error) => failed(
            &format!("盘古记忆成果看板任务失败：{error}"),
            MemoryOutcomeDashboardPayload {
                dashboard: empty_dashboard(),
            },
        ),
    }
}

#[tauri::command]
pub async fn load_memory_new_project_guide() -> CommandResult<MemoryNewProjectGuidePayload> {
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().new_project_guide()
    })
    .await;
    match computed {
        Ok(Ok(guide)) => ok(
            "新项目启动指南已生成。",
            MemoryNewProjectGuidePayload { guide },
        ),
        Ok(Err(error)) => failed(
            &format!("生成新项目启动指南失败：{error}"),
            MemoryNewProjectGuidePayload {
                guide: MemoryNewProjectGuide::default(),
            },
        ),
        Err(error) => failed(
            &format!("新项目启动指南任务失败：{error}"),
            MemoryNewProjectGuidePayload {
                guide: MemoryNewProjectGuide::default(),
            },
        ),
    }
}

#[tauri::command]
pub async fn list_memory_assist_items(
    mut request: MemoryQueryRequest,
) -> CommandResult<MemoryAssistItemsPayload> {
    // SQLite read; keep it off the UI thread.
    restrict_manager_memory_query(&mut request);
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().list_items(request)
    })
    .await;
    match computed {
        Ok(Ok(items)) => ok(
            &format!("已加载 {} 条记忆条目。", items.len()),
            MemoryAssistItemsPayload { items },
        ),
        Ok(Err(error)) => failed(
            &format!("加载记忆列表失败：{error}"),
            MemoryAssistItemsPayload { items: Vec::new() },
        ),
        Err(error) => failed(
            &format!("记忆列表任务失败：{error}"),
            MemoryAssistItemsPayload { items: Vec::new() },
        ),
    }
}

#[tauri::command]
pub async fn learn_memory_assist_item(
    request: MemoryItemRequest,
) -> CommandResult<MemoryAssistItemPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "盘古记忆当前已禁用。",
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        );
    }
    // SQLite write (plus similarity scan and inject-cache rebuild); keep off UI.
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().learn_item(request)
    })
    .await;
    match computed {
        Ok(Ok(item)) => ok("记忆已保存。", MemoryAssistItemPayload { item }),
        Ok(Err(error)) => failed(
            &format!("保存记忆失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
        Err(error) => failed(
            &format!("记忆保存任务失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
    }
}

#[tauri::command]
pub async fn update_memory_assist_item(
    request: MemoryIdAndItemRequest,
) -> CommandResult<MemoryAssistItemPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "盘古记忆当前已禁用。",
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        );
    }
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().update_item(&request.id, request.item)
    })
    .await;
    match computed {
        Ok(Ok(item)) => ok("记忆已更新。", MemoryAssistItemPayload { item }),
        Ok(Err(error)) => failed(
            &format!("更新记忆失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
        Err(error) => failed(
            &format!("记忆更新任务失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
    }
}

#[tauri::command]
pub async fn delete_memory_assist_item(
    request: MemoryIdRequest,
) -> CommandResult<MemoryAssistItemPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "盘古记忆当前已禁用。",
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        );
    }
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().delete_item(&request.id)
    })
    .await;
    match computed {
        Ok(Ok(item)) => ok("记忆已删除。", MemoryAssistItemPayload { item }),
        Ok(Err(error)) => failed(
            &format!("删除记忆失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
        Err(error) => failed(
            &format!("记忆删除任务失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
    }
}

#[tauri::command]
pub async fn archive_memory_assist_item(
    request: MemoryIdRequest,
) -> CommandResult<MemoryAssistItemPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "盘古记忆当前已禁用。",
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        );
    }
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().archive_item(&request.id)
    })
    .await;
    match computed {
        Ok(Ok(item)) => ok("记忆已归档。", MemoryAssistItemPayload { item }),
        Ok(Err(error)) => failed(
            &format!("归档记忆失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
        Err(error) => failed(
            &format!("记忆归档任务失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
    }
}

#[tauri::command]
pub async fn restore_memory_assist_item(
    request: MemoryIdRequest,
) -> CommandResult<MemoryAssistItemPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "盘古记忆当前已禁用。",
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        );
    }
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().restore_item(&request.id)
    })
    .await;
    match computed {
        Ok(Ok(item)) => ok("记忆已恢复到活跃层。", MemoryAssistItemPayload { item }),
        Ok(Err(error)) => failed(
            &format!("恢复记忆失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
        Err(error) => failed(
            &format!("记忆恢复任务失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
    }
}

#[tauri::command]
pub async fn create_memory_assist_candidate(
    request: MemoryCandidateRequest,
) -> CommandResult<MemoryAssistCandidatePayload> {
    if !memory_assist_candidate_enabled() {
        return failed(
            "盘古记忆自动学习当前已禁用。",
            MemoryAssistCandidatePayload {
                candidate: empty_memory_candidate(),
            },
        );
    }
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().create_candidate(request)
    })
    .await;
    match computed {
        Ok(Ok(candidate)) => ok(
            "待确认记忆已创建。",
            MemoryAssistCandidatePayload { candidate },
        ),
        Ok(Err(error)) => failed(
            &format!("创建待确认记忆失败：{error}"),
            MemoryAssistCandidatePayload {
                candidate: empty_memory_candidate(),
            },
        ),
        Err(error) => failed(
            &format!("待确认记忆任务失败：{error}"),
            MemoryAssistCandidatePayload {
                candidate: empty_memory_candidate(),
            },
        ),
    }
}

#[tauri::command]
pub async fn list_memory_assist_candidates(
    request: MemoryCandidateListRequest,
) -> CommandResult<MemoryAssistCandidatesPayload> {
    let workspace = restrict_manager_memory_workspace(&request.workspace);
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().list_candidates(&workspace, true)
    })
    .await;
    match computed {
        Ok(Ok(candidates)) => ok(
            &format!("已加载 {} 条待确认记忆。", candidates.len()),
            MemoryAssistCandidatesPayload { candidates },
        ),
        Ok(Err(error)) => failed(
            &format!("加载待确认记忆失败：{error}"),
            MemoryAssistCandidatesPayload {
                candidates: Vec::new(),
            },
        ),
        Err(error) => failed(
            &format!("待确认记忆任务失败：{error}"),
            MemoryAssistCandidatesPayload {
                candidates: Vec::new(),
            },
        ),
    }
}

#[tauri::command]
pub async fn approve_memory_assist_candidate(
    request: MemoryIdRequest,
) -> CommandResult<MemoryAssistItemPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "盘古记忆当前已禁用。",
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        );
    }
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().approve_candidate(&request.id)
    })
    .await;
    match computed {
        Ok(Ok(item)) => ok("待确认记忆已通过。", MemoryAssistItemPayload { item }),
        Ok(Err(error)) => failed(
            &format!("通过待确认记忆失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
        Err(error) => failed(
            &format!("通过待确认记忆任务失败：{error}"),
            MemoryAssistItemPayload {
                item: empty_memory_item(),
            },
        ),
    }
}

#[tauri::command]
pub async fn reject_memory_assist_candidate(
    request: MemoryIdRequest,
) -> CommandResult<MemoryAssistCandidatePayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "盘古记忆当前已禁用。",
            MemoryAssistCandidatePayload {
                candidate: empty_memory_candidate(),
            },
        );
    }
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().reject_candidate(&request.id)
    })
    .await;
    match computed {
        Ok(Ok(candidate)) => ok(
            "待确认记忆已拒绝。",
            MemoryAssistCandidatePayload { candidate },
        ),
        Ok(Err(error)) => failed(
            &format!("拒绝待确认记忆失败：{error}"),
            MemoryAssistCandidatePayload {
                candidate: empty_memory_candidate(),
            },
        ),
        Err(error) => failed(
            &format!("拒绝待确认记忆任务失败：{error}"),
            MemoryAssistCandidatePayload {
                candidate: empty_memory_candidate(),
            },
        ),
    }
}

#[tauri::command]
pub async fn load_memory_assist_session(
    request: MemorySessionRequest,
) -> CommandResult<MemoryAssistSessionPayload> {
    // session_summary triggers Codex history backfill (SQLite scans + rollout
    // JSONL parsing) — heavy synchronous IO that must not run on the UI thread.
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().session_summary(request)
    })
    .await;
    let empty_summary = || MemorySessionSummary {
        workspace: String::new(),
        inject_summary_cache_path: MemoryAssistStore::default()
            .inject_summary_cache_path()
            .to_string_lossy()
            .to_string(),
        total_items: 0,
        pending_candidates: 0,
        injected_items: Vec::new(),
        recent_captures: Vec::new(),
        capture_summary: String::new(),
        summary: String::new(),
    };
    match computed {
        Ok(Ok(summary)) => ok(
            "记忆会话摘要已加载。",
            MemoryAssistSessionPayload { summary },
        ),
        Ok(Err(error)) => failed(
            &format!("加载记忆会话摘要失败：{error}"),
            MemoryAssistSessionPayload {
                summary: empty_summary(),
            },
        ),
        Err(error) => failed(
            &format!("加载记忆会话摘要失败：{error}"),
            MemoryAssistSessionPayload {
                summary: empty_summary(),
            },
        ),
    }
}

#[tauri::command]
pub async fn run_memory_assist_selfcheck(
    request: MemorySelfCheckRequest,
) -> CommandResult<MemoryAssistSelfCheckPayload> {
    log_manager_event(
        "manager.memory.selfcheck.start",
        json!({
            "repair": request.repair,
            "sources": ["codex_sqlite", "codex_rollout_files", "memory_assist.sqlite"],
            "historyScan": "all_visible_workspaces_and_sessions"
        }),
    );
    if !memory_assist_write_enabled() {
        log_manager_event(
            "manager.memory.selfcheck.failed",
            json!({
                "repair": request.repair,
                "reason": "memory_assist_write_disabled"
            }),
        );
        return failed(
            "盘古记忆当前已禁用。",
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
    // Phase 3 module C: when the LLM-summary gate is on, resolve a per-workspace
    // summary through the active relay *before* the synchronous consolidation.
    // This ships memory text to the relay, so it is off by default and only runs
    // on an explicit repair. Any workspace that fails (or the whole call failing)
    // degrades silently to the rule-based summarizer inside consolidation.
    let summaries = if request.repair {
        resolve_memory_llm_summaries().await
    } else {
        std::collections::BTreeMap::new()
    };

    // Self-check is the heaviest memory op: it scans every Codex SQLite DB and
    // rollout file (backfill runs with no cap) and can take seconds to minutes on
    // large histories. It must never run on the UI thread.
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().run_selfcheck_with_summaries(request, &summaries)
    })
    .await;
    let outcome = match computed {
        Ok(inner) => inner,
        Err(join_error) => Err(anyhow::anyhow!("自检任务失败：{join_error}")),
    };
    match outcome {
        Ok(report) => {
            let history_message = report
                .checks
                .iter()
                .find(|check| check.name == "history")
                .map(|check| check.message.clone())
                .unwrap_or_default();
            log_manager_event(
                "manager.memory.selfcheck.result",
                json!({
                    "status": &report.status,
                    "repaired": report.repaired,
                    "backupPath": &report.backup_path,
                    "history": history_message,
                    "checks": &report.checks
                }),
            );
            ok(
                "盘古记忆自检已完成。",
                MemoryAssistSelfCheckPayload { report },
            )
        }
        Err(error) => {
            log_manager_event(
                "manager.memory.selfcheck.failed",
                json!({
                    "reason": error.to_string()
                }),
            );
            failed(
                &format!("盘古记忆自检失败：{error}"),
                MemoryAssistSelfCheckPayload {
                    report: MemorySelfCheckResult {
                        status: "failed".to_string(),
                        repaired: false,
                        backup_path: None,
                        checks: Vec::new(),
                    },
                },
            )
        }
    }
}

#[tauri::command]
pub async fn export_memory_assist() -> CommandResult<MemoryAssistExportPayload> {
    // Exporting serializes the whole SQLite store; keep it off the UI thread.
    let computed =
        tauri::async_runtime::spawn_blocking(|| MemoryAssistStore::default().export_json()).await;
    let outcome = match computed {
        Ok(inner) => inner,
        Err(join_error) => Err(anyhow::anyhow!("导出任务失败：{join_error}")),
    };
    match outcome {
        Ok(data) => ok("盘古记忆数据已导出。", MemoryAssistExportPayload { data }),
        Err(error) => failed(
            &format!("盘古记忆导出失败：{error}"),
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
pub async fn import_memory_assist(
    request: MemoryImportRequest,
) -> CommandResult<MemoryAssistStatusPayload> {
    if !memory_assist_write_enabled() {
        return failed(
            "盘古记忆当前已禁用。",
            MemoryAssistStatusPayload {
                memory: empty_memory_status(),
            },
        );
    }
    // Import parses JSON and performs a batch of SQLite writes; run it on the
    // blocking pool so a large import cannot freeze the UI thread.
    let computed = tauri::async_runtime::spawn_blocking(move || {
        MemoryAssistStore::default().import_json(request)
    })
    .await;
    let outcome = match computed {
        Ok(inner) => inner,
        Err(join_error) => Err(anyhow::anyhow!("导入任务失败：{join_error}")),
    };
    match outcome {
        Ok(memory) => ok("盘古记忆数据已导入。", MemoryAssistStatusPayload { memory }),
        Err(error) => failed(
            &format!("盘古记忆导入失败：{error}"),
            MemoryAssistStatusPayload {
                memory: empty_memory_status(),
            },
        ),
    }
}

/// MCP server 在各客户端配置里的条目名（Claude Desktop 的 mcpServers key /
/// Codex config.toml 的 [mcp_servers.<id>]）。
const MEMORY_MCP_SERVER_ID: &str = "pangu-memory";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMcpRegisterPayload {
    pub mcp_binary_path: String,
    pub mcp_binary_exists: bool,
    pub claude_desktop_config_path: String,
    pub claude_desktop_registered: bool,
    pub codex_config_path: String,
    pub codex_registered: bool,
    pub mcp_enabled: bool,
    pub errors: Vec<String>,
}

fn empty_mcp_register_payload() -> MemoryMcpRegisterPayload {
    MemoryMcpRegisterPayload {
        mcp_binary_path: String::new(),
        mcp_binary_exists: false,
        claude_desktop_config_path: String::new(),
        claude_desktop_registered: false,
        codex_config_path: String::new(),
        codex_registered: false,
        mcp_enabled: false,
        errors: Vec::new(),
    }
}

/// 一键把盘古记忆 MCP server 注册到 Claude Desktop 与 Codex 两端配置（ADR 0002
/// 决策 B）。复用现有 `upsert_claude_desktop_mcp_entry`（写 mcpServers JSON）与
/// `upsert_context_entry_in_common_config`（写 config.toml 的 mcp_servers 表），
/// MCP exe 路径用 `companion_binary_path` 解析同目录兄弟二进制的绝对路径。
///
/// 两端各自独立成败：一端失败不阻断另一端，错误汇总回报。文件 IO 走 blocking 池。
#[tauri::command]
pub async fn register_memory_mcp_server() -> CommandResult<MemoryMcpRegisterPayload> {
    tauri::async_runtime::spawn_blocking(register_memory_mcp_server_blocking)
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("注册盘古记忆 MCP 任务失败：{join_error}"),
                empty_mcp_register_payload(),
            )
        })
}

fn register_memory_mcp_server_blocking() -> CommandResult<MemoryMcpRegisterPayload> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    let mcp_enabled = settings.memory_assist_mcp_enabled;

    let mcp_binary = claude_codex_pro_core::install::companion_binary_path(MCP_BINARY);
    let mcp_binary_path = mcp_binary.to_string_lossy().to_string();
    let mcp_binary_exists = mcp_binary.exists();

    let mut payload = empty_mcp_register_payload();
    payload.mcp_binary_path = mcp_binary_path.clone();
    payload.mcp_binary_exists = mcp_binary_exists;
    payload.mcp_enabled = mcp_enabled;

    // Claude Desktop 端：写 mcpServers JSON。
    let claude_body = json!({
        "command": mcp_binary_path,
        "args": [],
    })
    .to_string();
    match plugin_hub::upsert_claude_desktop_mcp_entry(MEMORY_MCP_SERVER_ID, &claude_body) {
        Ok(entries) => {
            payload.claude_desktop_config_path = entries.config_path;
            payload.claude_desktop_registered = true;
        }
        Err(error) => payload
            .errors
            .push(format!("注册到 Claude Desktop 失败：{error}")),
    }

    // Codex 端：写 config.toml 的 [mcp_servers.<id>]。用 toml_edit 安全构建 body，
    // 自动转义 Windows 路径反斜杠，避免手拼字符串出错。
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let codex_config_path = home.join("config.toml");
    payload.codex_config_path = codex_config_path.to_string_lossy().to_string();
    let mut body_doc = toml_edit::DocumentMut::new();
    body_doc["command"] = toml_edit::value(mcp_binary_path.clone());
    let toml_body = body_doc.to_string();
    let existing_config = std::fs::read_to_string(&codex_config_path).unwrap_or_default();
    match claude_codex_pro_core::relay_config::upsert_context_entry_in_common_config(
        &existing_config,
        "mcp",
        MEMORY_MCP_SERVER_ID,
        &toml_body,
    ) {
        Ok(updated) => {
            let write_result = codex_config_path
                .parent()
                .map(std::fs::create_dir_all)
                .unwrap_or(Ok(()))
                .and_then(|_| std::fs::write(&codex_config_path, &updated));
            match write_result {
                Ok(()) => payload.codex_registered = true,
                Err(error) => payload
                    .errors
                    .push(format!("写入 Codex config.toml 失败：{error}")),
            }
        }
        Err(error) => payload.errors.push(format!("注册到 Codex 失败：{error}")),
    }

    if !mcp_binary_exists {
        payload.errors.push(format!(
            "MCP 二进制未找到：{mcp_binary_path}（配置已写入，但需构建/安装 claude-codex-pro-mcp 后才能启动）。"
        ));
    }

    let both_ok = payload.claude_desktop_registered && payload.codex_registered;
    if both_ok && payload.errors.is_empty() {
        ok(
            "盘古记忆 MCP 已注册到 Claude Desktop 与 Codex 两端。",
            payload,
        )
    } else if payload.claude_desktop_registered || payload.codex_registered {
        let message = format!("盘古记忆 MCP 部分注册完成：{}", payload.errors.join("；"));
        ok(&message, payload)
    } else {
        let message = format!("盘古记忆 MCP 注册失败：{}", payload.errors.join("；"));
        failed(&message, payload)
    }
}

fn empty_memory_status() -> MemoryAssistStatus {
    MemoryAssistStatus {
        status: "failed".to_string(),
        db_path: claude_codex_pro_core::memory_assist::default_memory_assist_db_path()
            .to_string_lossy()
            .to_string(),
        inject_summary_cache_path: MemoryAssistStore::default()
            .inject_summary_cache_path()
            .to_string_lossy()
            .to_string(),
        total_items: 0,
        pending_candidates: 0,
        total_captures: 0,
        capture_progress: MemoryCaptureProgressStatus::default(),
        workspaces: Vec::new(),
        latest_backup_path: None,
        enabled: false,
        inject_enabled: false,
        auto_suggest_enabled: false,
        runtime_status: "failed".to_string(),
        runtime_message: "盘古记忆当前不可用。".to_string(),
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
        tier: "active".to_string(),
        strength: 1.0,
        archived_at: 0,
        retention: 1.0,
        exempt: false,
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

/// Resolve a per-workspace LLM summary through the active relay profile (phase 3
/// module C). Returns an empty map — degrading to the rule-based summarizer — when
/// the gate is off, memory is disabled, no consolidatable inputs exist, or the
/// relay call fails. Shipping memory text to the relay is privacy-sensitive, so
/// the `memoryAssistLlmSummaryEnabled` gate defaults to false.
async fn resolve_memory_llm_summaries() -> std::collections::BTreeMap<String, String> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    if !settings.memory_assist_enabled || !settings.memory_assist_llm_summary_enabled {
        return std::collections::BTreeMap::new();
    }
    let inputs = match tauri::async_runtime::spawn_blocking(|| {
        MemoryAssistStore::default().collect_consolidation_inputs()
    })
    .await
    {
        Ok(Ok(inputs)) => inputs,
        _ => return std::collections::BTreeMap::new(),
    };
    if inputs.is_empty() {
        return std::collections::BTreeMap::new();
    }
    let profile = settings.active_relay_profile();
    let mut summaries = std::collections::BTreeMap::new();
    for (workspace, source_text) in inputs {
        let prompt = format!(
            "你是记忆整合助手。请把下面同一工作区的多条经验教训合并去重，浓缩为一份不超过 10 条要点的中文\"经验教训手册\"，\
             每条以\"- \"开头，只保留可执行的项目约定、修复结论、偏好与工作流规则，剔除一次性命令输出和临时错误。\
             第一行输出\"经验教训手册：\"。\n\n原始记忆：\n{source_text}"
        );
        match claude_codex_pro_core::relay_config::summarize_memory_via_relay(&profile, &prompt)
            .await
        {
            Ok(summary) if !summary.trim().is_empty() => {
                summaries.insert(workspace, summary);
            }
            _ => {
                // Degrade silently: this workspace falls back to the rule-based
                // summarizer inside consolidation.
            }
        }
    }
    summaries
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

    let launch_started_at_ms = StatusStore::default()
        .load_latest()
        .ok()
        .flatten()
        .map(|status| status.started_at_ms);
    let heartbeat = latest_renderer_runtime_heartbeat();
    let heartbeat_is_fresh = heartbeat
        .as_ref()
        .is_some_and(|item| renderer_heartbeat_is_current(item.timestamp_ms, launch_started_at_ms));
    let runtime_snapshot = read_codex_memory_runtime_snapshot().or_else(|| {
        heartbeat
            .filter(|_| heartbeat_is_fresh)
            .and_then(|item| item.runtime)
    });

    if let Some(runtime) = runtime_snapshot {
        let normalized_runtime_status = normalize_memory_runtime_status(&runtime);
        memory.runtime_status = if runtime.injected {
            normalized_runtime_status
        } else if memory.enabled && memory.inject_enabled {
            "waiting".to_string()
        } else {
            "disabled".to_string()
        };
        memory.runtime_message = if runtime.summary.trim().is_empty() {
            "盘古记忆运行时已同步。".to_string()
        } else if runtime.injected && runtime.status == "idle" {
            "等待真实对话消息后写入盘古记忆。".to_string()
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
    } else if heartbeat_is_fresh && memory.enabled && memory.inject_enabled {
        memory.runtime_status = "ok".to_string();
        memory.runtime_message = "Codex 前端脚本已注入，正在等待盘古记忆运行时同步。".to_string();
        memory.codex_injected = true;
        memory.active = true;
        memory.active_source = "codex-script".to_string();
    } else {
        memory.runtime_status = if memory.enabled && memory.inject_enabled {
            "not_checked".to_string()
        } else {
            "disabled".to_string()
        };
        memory.runtime_message = if memory.enabled && memory.inject_enabled {
            "已启用盘古记忆并开启注入，正在等待 Codex 前端加载记忆运行时脚本。请确认 Codex 已启动并完成注入。".to_string()
        } else {
            "盘古记忆当前已禁用。".to_string()
        };
    }

    memory.claude_injected = false;
    memory
}

fn normalize_memory_runtime_status(runtime: &MemoryAssistRuntimeSnapshot) -> String {
    match runtime.status.as_str() {
        "idle" => "ok".to_string(),
        "" if runtime.injected => "ok".to_string(),
        value => value.to_string(),
    }
}

fn latest_renderer_runtime_heartbeat() -> Option<RendererRuntimeHeartbeat> {
    let path = claude_codex_pro_core::diagnostic_log::diagnostic_log_path();
    // Only the newest few records matter here, and the log is unbounded, so read a
    // bounded window from the end instead of the whole file (this function is on
    // the status-panel polling path — a full read got slower as the log grew).
    let text = read_tail(&path, 2_000).ok()?;
    let mut newest_script_loaded: Option<RendererRuntimeHeartbeat> = None;
    let mut newest_runtime: Option<RendererRuntimeHeartbeat> = None;
    for record in text
        .lines()
        .rev()
        .take(2_000)
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
            newest_runtime = Some(RendererRuntimeHeartbeat {
                timestamp_ms: record.timestamp_ms,
                runtime,
                runtime_reported: true,
            });
            break;
        }
        if newest_script_loaded.is_none() {
            newest_script_loaded = Some(RendererRuntimeHeartbeat {
                timestamp_ms: record.timestamp_ms,
                runtime: None,
                runtime_reported: false,
            });
        }
    }
    match (newest_runtime, newest_script_loaded) {
        (Some(runtime), Some(script_loaded))
            if script_loaded.timestamp_ms > runtime.timestamp_ms =>
        {
            Some(script_loaded)
        }
        (Some(runtime), _) => Some(runtime),
        (None, script_loaded) => script_loaded,
    }
}

fn renderer_heartbeat_is_fresh(timestamp_ms: u64) -> bool {
    current_time_ms().saturating_sub(timestamp_ms) <= 45_000
}

fn renderer_heartbeat_is_current(timestamp_ms: u64, launch_started_at_ms: Option<u64>) -> bool {
    launch_started_at_ms.is_some_and(|launch_started_at_ms| {
        timestamp_ms >= launch_started_at_ms && renderer_heartbeat_is_fresh(timestamp_ms)
    })
}

fn renderer_frontend_heartbeat_confirms_injection(heartbeat: &RendererRuntimeHeartbeat) -> bool {
    renderer_heartbeat_is_fresh(heartbeat.timestamp_ms)
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
            &format!("已加载 {} 个 Zed 远程项目。", projects.len()),
            ZedRemoteProjectsPayload { projects },
        );
    }
    failed(
        result
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("加载 Zed 远程项目失败。"),
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
            "Zed Remote 链接已打开。",
            ZedRemoteOpenPayload { url, strategy },
        );
    }
    failed(
        result
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("打开 Zed Remote 链接失败。"),
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
                .unwrap_or("移除 Zed 远程项目失败。"),
            ZedRemoteProjectsPayload {
                projects: Vec::new(),
            },
        );
    }
    list_zed_remote_projects()
}

#[tauri::command]
pub async fn delete_local_session(
    request: DeleteLocalSessionRequest,
) -> CommandResult<DeleteResult> {
    // SQLite deletion plus backup-file copies are blocking IO; keep them off the
    // UI thread so deleting a session never stalls the WebView.
    tauri::async_runtime::spawn_blocking(move || delete_local_session_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("删除会话任务失败：{join_error}"),
                DeleteResult {
                    status: claude_codex_pro_core::models::DeleteStatus::Failed,
                    session_id: String::new(),
                    message: format!("删除会话任务失败：{join_error}"),
                    undo_token: None,
                    backup_path: None,
                },
            )
        })
}

#[tauri::command]
pub async fn delete_claude_session(
    request: DeleteClaudeSessionRequest,
) -> CommandResult<DeleteClaudeSessionPayload> {
    tauri::async_runtime::spawn_blocking(move || delete_claude_session_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            let message = format!("删除 Claude 会话任务失败：{join_error}");
            failed(
                &message,
                DeleteClaudeSessionPayload {
                    session_id: String::new(),
                    backup_path: None,
                },
            )
        })
}

fn delete_claude_session_blocking(
    request: DeleteClaudeSessionRequest,
) -> CommandResult<DeleteClaudeSessionPayload> {
    let session_id = request.session_id.trim();
    let source_path = request.source_path.trim();
    if session_id.is_empty() || source_path.is_empty() {
        return failed(
            "Claude 会话 ID 和来源路径不能为空。",
            DeleteClaudeSessionPayload {
                session_id: session_id.to_string(),
                backup_path: None,
            },
        );
    }
    log_manager_event(
        "manager.delete_claude_session.start",
        json!({ "session_id": session_id }),
    );
    let result = claude_codex_pro_core::claude_sessions::delete_claude_session(
        &claude_codex_pro_core::paths::default_app_state_dir().join("backups"),
        session_id,
        Path::new(source_path),
    );
    match result {
        Ok(outcome) => {
            log_manager_event(
                "manager.delete_claude_session.finish",
                json!({ "session_id": session_id, "status": "ok" }),
            );
            ok(
                &outcome.message,
                DeleteClaudeSessionPayload {
                    session_id: outcome.session_id,
                    backup_path: Some(outcome.backup_path),
                },
            )
        }
        Err(error) => {
            let message = format!("删除 Claude 会话失败：{error}");
            log_manager_event(
                "manager.delete_claude_session.finish",
                json!({ "session_id": session_id, "status": "failed", "message": message }),
            );
            failed(
                &message,
                DeleteClaudeSessionPayload {
                    session_id: session_id.to_string(),
                    backup_path: None,
                },
            )
        }
    }
}

fn delete_local_session_blocking(
    request: DeleteLocalSessionRequest,
) -> CommandResult<DeleteResult> {
    delete_local_session_blocking_with_backup_store(
        request,
        claude_codex_pro_data::BackupStore::new(
            claude_codex_pro_core::paths::default_app_state_dir().join("backups"),
        ),
    )
}

fn delete_local_session_blocking_with_backup_store(
    request: DeleteLocalSessionRequest,
    backup_store: claude_codex_pro_data::BackupStore,
) -> CommandResult<DeleteResult> {
    let session_id = request.session_id.trim();
    if session_id.is_empty() {
        return failed(
            "会话 ID 不能为空。",
            DeleteResult {
                status: claude_codex_pro_core::models::DeleteStatus::Failed,
                session_id: String::new(),
                message: "会话 ID 不能为空。".to_string(),
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
        backup_store,
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

/// Candidate Codex session DBs for a migration/export request: the explicit
/// db_path first (so the caller's own choice wins), then every discovered DB.
fn session_candidate_db_paths(explicit: Option<&str>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = explicit {
        let path = PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            paths.push(path);
        }
    }
    for path in claude_codex_pro_core::codex_sqlite::codex_session_db_paths_from_home(
        &claude_codex_pro_core::codex_sqlite::default_codex_home_dir(),
    ) {
        if !paths.iter().any(|candidate| candidate == &path) {
            paths.push(path);
        }
    }
    paths
}

/// `~/.claude` — the root Claude Code stores its `projects/<slug>/<uuid>.jsonl`
/// transcripts under. Falls back to the config-dir home resolution used
/// elsewhere in this module.
fn claude_code_home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
}

#[tauri::command]
pub async fn export_session_universal(
    request: SessionExportRequest,
) -> CommandResult<SessionExportPayload> {
    // Reads a Codex SQLite DB and its rollout JSONL, then serializes — file IO
    // that must not run on the UI thread.
    tauri::async_runtime::spawn_blocking(move || export_session_universal_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("会话导出任务失败：{join_error}"),
                SessionExportPayload { export: None },
            )
        })
}

fn export_session_universal_blocking(
    request: SessionExportRequest,
) -> CommandResult<SessionExportPayload> {
    let session_id = request.session_id.trim().to_string();
    if session_id.is_empty() {
        return failed("会话 ID 不能为空。", SessionExportPayload { export: None });
    }
    let format = request.format;
    for db_path in session_candidate_db_paths(request.db_path.as_deref()) {
        match claude_codex_pro_data::export_session_universal(&db_path, &session_id, format) {
            Ok(Some(export)) => {
                log_manager_event(
                    "manager.session_export.ok",
                    json!({
                        "sessionId": session_id,
                        "format": format!("{format:?}"),
                        "messageCount": export.message_count
                    }),
                );
                return ok(
                    &format!("已导出 {} 条消息。", export.message_count),
                    SessionExportPayload {
                        export: Some(export),
                    },
                );
            }
            Ok(None) => continue,
            Err(error) => {
                return failed(
                    &format!("会话导出失败：{error}"),
                    SessionExportPayload { export: None },
                );
            }
        }
    }
    failed(
        "未在任何本地 Codex 数据库中找到该会话。",
        SessionExportPayload { export: None },
    )
}

#[tauri::command]
pub async fn migrate_session_to_claude_code(
    request: SessionMigrationRequest,
) -> CommandResult<SessionMigrationPayload> {
    tauri::async_runtime::spawn_blocking(move || migrate_session_to_claude_code_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("会话迁移任务失败：{join_error}"),
                SessionMigrationPayload {
                    migration: None,
                    claude_code_available: false,
                },
            )
        })
}

fn migrate_session_to_claude_code_blocking(
    request: SessionMigrationRequest,
) -> CommandResult<SessionMigrationPayload> {
    let session_id = request.session_id.trim().to_string();
    let claude_home = claude_code_home_dir();
    let claude_code_available =
        claude_codex_pro_data::claude_code_projects_dir(&claude_home).is_dir();
    if session_id.is_empty() {
        return failed(
            "会话 ID 不能为空。",
            SessionMigrationPayload {
                migration: None,
                claude_code_available,
            },
        );
    }
    if !claude_code_available {
        return failed(
            "未检测到 Claude Code（缺少 ~/.claude/projects 目录）。",
            SessionMigrationPayload {
                migration: None,
                claude_code_available,
            },
        );
    }

    let store = SettingsStore::default();
    let mut settings = store.load().unwrap_or_default();
    // Idempotency: reuse the recorded target UUID so re-running never writes a
    // duplicate transcript for a thread already migrated.
    let existing_uuid = settings
        .codex_session_migrations
        .iter()
        .find(|record| record.session_id == session_id)
        .map(|record| record.target_uuid.clone());

    for db_path in session_candidate_db_paths(request.db_path.as_deref()) {
        match claude_codex_pro_data::migrate_codex_thread_to_claude_code(
            &db_path,
            &session_id,
            &claude_home,
            request.target_cwd.as_deref(),
            existing_uuid.as_deref(),
        ) {
            Ok(Some(migration)) => {
                if !migration.already_migrated {
                    settings
                        .codex_session_migrations
                        .retain(|record| record.session_id != session_id);
                    settings.codex_session_migrations.push(
                        claude_codex_pro_core::settings::CodexSessionMigrationRecord {
                            session_id: session_id.clone(),
                            target_uuid: migration
                                .written_path
                                .rsplit(['/', '\\'])
                                .next()
                                .and_then(|name| name.strip_suffix(".jsonl"))
                                .unwrap_or_default()
                                .to_string(),
                            project_slug: migration.project_slug.clone(),
                        },
                    );
                    let _ = store.save(&settings);
                }
                log_manager_event(
                    "manager.session_migration.ok",
                    json!({
                        "sessionId": session_id,
                        "projectSlug": migration.project_slug,
                        "messageCount": migration.message_count,
                        "alreadyMigrated": migration.already_migrated
                    }),
                );
                let message = if migration.already_migrated {
                    "该会话已迁移到 Claude Code。".to_string()
                } else {
                    format!(
                        "已将 {} 条消息迁移到 Claude Code 项目 {}。",
                        migration.message_count, migration.project_slug
                    )
                };
                return ok(
                    &message,
                    SessionMigrationPayload {
                        migration: Some(migration),
                        claude_code_available,
                    },
                );
            }
            Ok(None) => continue,
            Err(error) => {
                return failed(
                    &format!("会话迁移失败：{error}"),
                    SessionMigrationPayload {
                        migration: None,
                        claude_code_available,
                    },
                );
            }
        }
    }
    failed(
        "在所有本地 Codex 数据库中都未找到该会话。",
        SessionMigrationPayload {
            migration: None,
            claude_code_available,
        },
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
    .map_err(|error| anyhow::anyhow!("供应商同步目标探测任务失败：{error}"));
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
                "供应商同步目标已加载。",
                serde_json::to_value(targets).unwrap_or_else(|_| json!({})),
            )
        }
        Err(error) => failed(&format!("供应商同步目标加载失败：{error}"), json!({})),
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
    .map_err(|error| anyhow::anyhow!("供应商同步任务失败：{error}"));
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
                    "已完成一次供应商同步：{} 个会话文件，{} 行 sqlite 数据，{} 个被锁定文件已跳过。",
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
        Err(error) => failed(&format!("供应商同步失败：{error}"), json!({})),
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
        Ok(payload) => ok("推荐内容已加载。", ads_payload(payload)),
        Err(error) => failed(
            &format!("推荐内容加载失败：{error}"),
            ads_payload(claude_codex_pro_core::ads::normalize_ad_payload(json!({}))),
        ),
    }
}

#[tauri::command]
pub async fn refresh_script_market() -> CommandResult<ScriptMarketPayload> {
    match script_market::fetch_market_manifest(script_market::DEFAULT_MARKET_INDEX_URL).await {
        Ok(manifest) => ok(
            "脚本市场已刷新。",
            script_market_payload_from_manifest(&manifest, "ok", "脚本市场已刷新。"),
        ),
        Err(error) => failed(
            &format!("脚本市场加载失败：{error}"),
            failed_script_market_payload(&format!("脚本市场加载失败：{error}")),
        ),
    }
}

#[tauri::command]
pub async fn install_market_script(id: String) -> CommandResult<ScriptMarketPayload> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return failed(
            "脚本 id 不能为空。",
            failed_script_market_payload("脚本 id 不能为空。"),
        );
    }
    let manifest =
        match script_market::fetch_market_manifest(script_market::DEFAULT_MARKET_INDEX_URL).await {
            Ok(manifest) => manifest,
            Err(error) => {
                return failed(
                    &format!("脚本市场加载失败：{error}"),
                    failed_script_market_payload(&format!("脚本市场加载失败：{error}")),
                );
            }
        };
    let Some(script) = manifest.scripts.iter().find(|script| script.id == trimmed) else {
        return failed(
            "在市场清单中未找到该脚本。",
            script_market_payload_from_manifest(&manifest, "failed", "在市场清单中未找到该脚本。"),
        );
    };
    let manager = default_user_script_manager();
    match script_market::install_market_script(&manager, script).await {
        Ok(()) => ok(
            "脚本已安装。",
            script_market_payload_from_manifest(&manifest, "ok", "脚本已安装。"),
        ),
        Err(error) => failed(
            &format!("脚本安装失败：{error}"),
            script_market_payload_from_manifest(
                &manifest,
                "failed",
                &format!("脚本安装失败：{error}"),
            ),
        ),
    }
}

#[tauri::command]
pub async fn load_codex_plugin_marketplace_status() -> CommandResult<CodexPluginMarketplacePayload>
{
    // status() 会扫描 CODEX_HOME 下的 marketplace 目录树并读取多个配置文件。
    // 工具与插件页挂载时与其余 6 个状态命令并发触发，若都留在主 IPC 线程会
    // 串行阻塞窗口消息泵导致"未响应"。改为 spawn_blocking 放到阻塞线程池。
    let status = tauri::async_runtime::spawn_blocking(
        claude_codex_pro_core::codex_plugin_marketplace::status,
    )
    .await
    .unwrap_or_default();
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
                &format!("修复 Codex OpenAI 插件市场失败：{error}"),
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
pub fn list_codex_custom_marketplaces() -> CommandResult<CodexCustomMarketplacesPayload> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    let marketplace = claude_codex_pro_core::codex_plugin_marketplace::status();
    ok(
        &format!(
            "已加载 {} 个自定义插件市场。",
            settings.codex_custom_marketplaces.len()
        ),
        CodexCustomMarketplacesPayload {
            custom_marketplaces: settings.codex_custom_marketplaces,
            marketplace,
        },
    )
}

#[tauri::command]
pub async fn add_codex_custom_marketplace(
    request: CodexCustomMarketplaceRequest,
) -> CommandResult<CodexCustomMarketplacesPayload> {
    // Persisting settings and writing config.toml are both synchronous file IO;
    // keep them off the UI thread.
    tauri::async_runtime::spawn_blocking(move || add_codex_custom_marketplace_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("添加自定义插件市场任务失败：{join_error}"),
                empty_codex_custom_marketplaces_payload(),
            )
        })
}

fn add_codex_custom_marketplace_blocking(
    request: CodexCustomMarketplaceRequest,
) -> CommandResult<CodexCustomMarketplacesPayload> {
    let store = SettingsStore::default();
    let mut settings = store.load().unwrap_or_default();
    let mut marketplace = request.marketplace;
    marketplace.name = marketplace.name.trim().to_string();
    marketplace.source = marketplace.source.trim().to_string();
    marketplace.source_type = marketplace.source_type.trim().to_ascii_lowercase();
    if marketplace.name.is_empty() || marketplace.source.is_empty() {
        return failed(
            "自定义插件市场的名称和来源均为必填。",
            empty_codex_custom_marketplaces_payload(),
        );
    }

    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    // Write config.toml first so a validation failure never leaves a saved
    // setting that cannot actually be applied.
    if let Err(error) =
        claude_codex_pro_core::codex_plugin_marketplace::ensure_custom_marketplace_config(
            &home,
            &marketplace,
        )
    {
        return failed(
            &format!("注册自定义插件市场失败：{error}"),
            empty_codex_custom_marketplaces_payload(),
        );
    }

    // Replace an existing entry with the same name (case-insensitive) rather than
    // appending a duplicate.
    settings
        .codex_custom_marketplaces
        .retain(|existing| !existing.name.eq_ignore_ascii_case(&marketplace.name));
    settings.codex_custom_marketplaces.push(marketplace);

    if let Err(error) = store.save(&settings) {
        return failed(
            &format!("自定义插件市场已注册，但保存设置失败：{error}"),
            empty_codex_custom_marketplaces_payload(),
        );
    }

    let status = claude_codex_pro_core::codex_plugin_marketplace::status();
    log_manager_event(
        "manager.codex_custom_marketplace.added",
        json!({ "count": settings.codex_custom_marketplaces.len() }),
    );
    ok(
        "自定义插件市场已注册。请重启 Codex 以加载。",
        CodexCustomMarketplacesPayload {
            custom_marketplaces: settings.codex_custom_marketplaces,
            marketplace: status,
        },
    )
}

#[tauri::command]
pub async fn remove_codex_custom_marketplace(
    request: CodexCustomMarketplaceRemoveRequest,
) -> CommandResult<CodexCustomMarketplacesPayload> {
    tauri::async_runtime::spawn_blocking(move || remove_codex_custom_marketplace_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("移除自定义插件市场任务失败：{join_error}"),
                empty_codex_custom_marketplaces_payload(),
            )
        })
}

fn remove_codex_custom_marketplace_blocking(
    request: CodexCustomMarketplaceRemoveRequest,
) -> CommandResult<CodexCustomMarketplacesPayload> {
    let store = SettingsStore::default();
    let mut settings = store.load().unwrap_or_default();
    let name = request.name.trim();
    if name.is_empty() {
        return failed(
            "自定义插件市场名称不能为空。",
            empty_codex_custom_marketplaces_payload(),
        );
    }
    let before = settings.codex_custom_marketplaces.len();
    settings
        .codex_custom_marketplaces
        .retain(|existing| !existing.name.eq_ignore_ascii_case(name));
    if settings.codex_custom_marketplaces.len() == before {
        return failed(
            &format!("未找到自定义插件市场 {name}。"),
            empty_codex_custom_marketplaces_payload(),
        );
    }

    // Drop the [marketplaces.<name>] section from config.toml too, so removing it
    // here actually stops Codex from seeing it.
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    if let Err(error) =
        claude_codex_pro_core::codex_plugin_marketplace::remove_marketplace_config(&home, name)
    {
        return failed(
            &format!("从 config.toml 注销自定义插件市场失败：{error}"),
            empty_codex_custom_marketplaces_payload(),
        );
    }

    if let Err(error) = store.save(&settings) {
        return failed(
            &format!("自定义插件市场已注销，但设置保存失败：{error}"),
            empty_codex_custom_marketplaces_payload(),
        );
    }

    let status = claude_codex_pro_core::codex_plugin_marketplace::status();
    log_manager_event(
        "manager.codex_custom_marketplace.removed",
        json!({ "count": settings.codex_custom_marketplaces.len() }),
    );
    ok(
        "自定义插件市场已移除。请重启 Codex 使其生效。",
        CodexCustomMarketplacesPayload {
            custom_marketplaces: settings.codex_custom_marketplaces,
            marketplace: status,
        },
    )
}

fn empty_codex_custom_marketplaces_payload() -> CodexCustomMarketplacesPayload {
    CodexCustomMarketplacesPayload {
        custom_marketplaces: Vec::new(),
        marketplace: claude_codex_pro_core::codex_plugin_marketplace::status(),
    }
}

#[tauri::command]
pub async fn refresh_plugin_hub_catalog() -> CommandResult<PluginHubPayload> {
    let catalog = plugin_hub::fetch_catalog().await;
    ok("插件中心目录已刷新。", PluginHubPayload { catalog })
}

#[tauri::command]
pub async fn get_plugin_hub_catalog() -> CommandResult<PluginHubPayload> {
    let catalog = plugin_hub::fetch_catalog().await;
    ok("插件中心目录已加载。", PluginHubPayload { catalog })
}

#[tauri::command]
pub async fn preview_plugin_hub_install(
    request: PluginHubItemRequest,
) -> CommandResult<PluginInstallPreview> {
    match plugin_hub::preview_install(request.id.trim()).await {
        Ok(preview) => ok("插件安装预览已加载。", preview),
        Err(error) => failed(
            &format!("插件安装预览失败：{error}"),
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
            let message = format!("插件安装失败：{error}");
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
                "插件已移除。如有需要请重启 Codex 或 Claude。",
                PluginHubPayload { catalog },
            )
        }
        Err(error) => {
            let catalog = plugin_hub::fetch_catalog().await;
            failed(
                &format!("插件卸载失败：{error}"),
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
            &format!("预览 Ponytail Codex hooks 失败：{error}"),
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
            &format!("信任 Ponytail Codex hooks 失败：{error}"),
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
            &format!("生成 Ponytail MCPB 失败：{error}"),
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
pub async fn load_claude_desktop_org_plugin_status() -> CommandResult<ClaudeDesktopOrgPluginPayload>
{
    // 遍历读取 Claude Desktop 组织插件目录与 profile 元数据，属磁盘 IO。
    // 与工具页其余状态命令并发挂载，统一放到阻塞线程池避免阻塞窗口消息泵。
    let status =
        tauri::async_runtime::spawn_blocking(plugin_hub::load_claude_desktop_org_plugin_status)
            .await
            .unwrap_or_default();
    ok(
        &status.message.clone(),
        ClaudeDesktopOrgPluginPayload {
            org_plugin_status: status,
        },
    )
}

#[tauri::command]
pub async fn load_claude_desktop_marketplace_status()
-> CommandResult<ClaudeDesktopMarketplacePayload> {
    // 读取并解析 Claude Desktop marketplace 配置，属磁盘 IO，移出主 IPC 线程。
    let status =
        tauri::async_runtime::spawn_blocking(plugin_hub::load_claude_desktop_marketplace_status)
            .await
            .unwrap_or_default();
    ok(
        &status.message.clone(),
        ClaudeDesktopMarketplacePayload {
            marketplace_status: status,
        },
    )
}

#[tauri::command]
pub async fn load_claude_desktop_dev_mode_status() -> CommandResult<ClaudeDesktopDevModePayload> {
    // 读取 Claude Desktop 开发者模式配置文件，属磁盘 IO，移出主 IPC 线程。
    let status =
        tauri::async_runtime::spawn_blocking(plugin_hub::load_claude_desktop_dev_mode_status)
            .await
            .unwrap_or_default();
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
                &format!("写入开发者模式前本地模型代理启动失败：{error}"),
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
                format!(" 本地模型代理已在 127.0.0.1:{proxy_port} 验证在线。")
            } else {
                format!(
                    " 本地模型代理已请求使用 127.0.0.1:{proxy_port}，但 /backend/status 暂未响应。"
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
            &format!("配置 Claude Desktop 开发者模式失败：{error}"),
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
            &format!("打开 Claude Desktop 插件市场设置失败：{error}"),
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
            &format!("修复 Claude Desktop 插件市场失败：{error}"),
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
            &format!("打开 Claude Desktop 组织插件目录失败：{error}"),
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
            &format!("安装 Ponytail Claude Desktop 组织插件失败：{error}"),
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
            &format!("安装 Claude Desktop 本地插件包失败：{error}"),
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
        return failed("脚本 key 不能为空。", fallback_settings_payload());
    }
    let manager = default_user_script_manager();
    match manager.set_script_enabled(trimmed, enabled) {
        Ok(_) => settings_payload(
            if enabled {
                "脚本已启用。"
            } else {
                "脚本已禁用。"
            },
            "脚本设置更新失败",
        ),
        Err(error) => failed(
            &format!("脚本设置更新失败：{error}"),
            fallback_settings_payload(),
        ),
    }
}

#[tauri::command]
pub fn delete_user_script(key: String) -> CommandResult<SettingsPayload> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return failed("脚本 key 不能为空。", fallback_settings_payload());
    }
    let manager = default_user_script_manager();
    match manager.delete_user_script(trimmed) {
        Ok(_) => settings_payload("脚本已删除。", "脚本删除失败"),
        Err(error) => failed(
            &format!("脚本删除失败：{error}"),
            fallback_settings_payload(),
        ),
    }
}

#[tauri::command]
pub fn open_external_url(url: String) -> CommandResult<Value> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return failed("只能打开 http 或 https 链接。", json!({}));
    }
    match open_url(trimmed) {
        Ok(()) => ok("链接已在系统浏览器中打开。", json!({ "url": trimmed })),
        Err(error) => failed(&format!("打开链接失败：{error}"), json!({ "url": trimmed })),
    }
}

#[tauri::command]
pub async fn install_entrypoints() -> InstallActionResult {
    tauri::async_runtime::spawn_blocking(install::install_entrypoints)
        .await
        .unwrap_or_else(|error| install_background_failure("安装入口", error))
}

#[tauri::command]
pub async fn uninstall_entrypoints(options: InstallOptions) -> InstallActionResult {
    tauri::async_runtime::spawn_blocking(move || install::uninstall_entrypoints(options))
        .await
        .unwrap_or_else(|error| install_background_failure("卸载入口", error))
}

#[tauri::command]
pub async fn repair_shortcuts() -> InstallActionResult {
    tauri::async_runtime::spawn_blocking(install::repair_shortcuts)
        .await
        .unwrap_or_else(|error| install_background_failure("修复快捷方式", error))
}

#[tauri::command]
pub fn repair_backend() -> CommandResult<SettingsPayload> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    let message = match claude_codex_pro_core::cli_wrapper::ensure_cli_wrapper(&settings) {
        Ok(Some(install)) => format!(
            "命令封装器已更新：{}。",
            install.real_codex.to_string_lossy()
        ),
        Ok(None) => "命令封装器已是最新。".to_string(),
        Err(error) => {
            return match settings_payload_value() {
                Ok(payload) => failed(&format!("命令封装器更新失败：{error}"), payload),
                Err((payload_error, payload)) => failed(
                    &format!("命令封装器更新失败：{error}；设置读取失败：{payload_error}"),
                    payload,
                ),
            };
        }
    };
    settings_payload(&message, "后端修复失败")
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
                    "有可用更新。".to_string()
                } else {
                    "你已经是最新版本。".to_string()
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
            &format!("检查更新失败：{error}"),
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

fn emit_update_download_progress(
    app: &tauri::AppHandle,
    phase: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) {
    let progress = claude_codex_pro_core::update::UpdateDownloadProgress::new(
        phase,
        downloaded_bytes,
        total_bytes,
    );
    let _ = app.emit("update-download-progress", progress);
}

fn require_expected_update_version(
    expected_version: Option<String>,
) -> Result<String, CommandResult<Value>> {
    expected_version
        .filter(|version| !version.trim().is_empty())
        .ok_or_else(|| {
            failed(
                "请先检查更新再安装；当前没有选中的发布版本。",
                json!({
                    "currentVersion": claude_codex_pro_core::version::VERSION,
                    "progress": 0
                }),
            )
        })
}

#[tauri::command]
pub async fn perform_update(
    app: tauri::AppHandle,
    expected_version: Option<String>,
) -> CommandResult<Value> {
    let expected_version = match require_expected_update_version(expected_version) {
        Ok(expected_version) => expected_version,
        Err(result) => return result,
    };
    emit_update_download_progress(&app, "connecting", 0, None);
    let release = match claude_codex_pro_core::update::fetch_current_release().await {
        Ok(release) if release.version == expected_version => release,
        Ok(release) => {
            emit_update_download_progress(&app, "failed", 0, None);
            return failed(
                "发布索引已更新，请重新检查版本后再安装。",
                json!({
                    "currentVersion": claude_codex_pro_core::version::VERSION,
                    "latestVersion": release.version,
                    "releaseSummary": release.body,
                    "progress": 0
                }),
            );
        }
        Err(error) => {
            emit_update_download_progress(&app, "failed", 0, None);
            return failed(
                &format!("重新读取发布索引失败：{error}"),
                json!({
                    "currentVersion": claude_codex_pro_core::version::VERSION,
                    "latestVersion": expected_version,
                    "progress": 0
                }),
            );
        }
    };
    let download_dir = claude_codex_pro_core::paths::default_app_state_dir().join("updates");
    let progress_app = app.clone();
    match claude_codex_pro_core::update::perform_update_with_progress(
        &release,
        &download_dir,
        move |progress| {
            let _ = progress_app.emit("update-download-progress", progress);
        },
    )
    .await
    {
        Ok(result) => {
            let downloaded_bytes = std::fs::metadata(&result.installer_path)
                .map(|metadata| metadata.len())
                .unwrap_or_default();
            emit_update_download_progress(
                &app,
                "complete",
                downloaded_bytes,
                Some(downloaded_bytes),
            );
            ok(
                "安装包已下载并启动。请按照安装向导提示完成更新。",
                json!({
                    "currentVersion": claude_codex_pro_core::version::VERSION,
                    "latestVersion": result.release.version,
                    "releaseSummary": result.release.body,
                    "installedPath": result.installer_path.to_string_lossy(),
                    "launched": result.launched,
                    "phase": "complete",
                    "downloadedBytes": downloaded_bytes,
                    "totalBytes": downloaded_bytes,
                    "progress": 100
                }),
            )
        }
        Err(error) => {
            emit_update_download_progress(&app, "failed", 0, None);
            failed(
                &format!("安装更新失败：{error}"),
                json!({
                    "currentVersion": claude_codex_pro_core::version::VERSION,
                    "latestVersion": release.version,
                    "releaseSummary": release.body,
                    "phase": "failed",
                    "downloadedBytes": 0,
                    "totalBytes": Value::Null,
                    "progress": 0
                }),
            )
        }
    }
}

#[tauri::command]
pub fn load_watcher_state() -> CommandResult<WatcherPayload> {
    ok("监视器状态已加载。", watcher_payload())
}

#[tauri::command]
pub fn install_watcher() -> CommandResult<WatcherPayload> {
    let launcher_path = match resolve_silent_launcher_path() {
        Ok(path) => path,
        Err(error) => {
            return failed(&format!("安装守护进程失败：{error}"), watcher_payload());
        }
    };
    match claude_codex_pro_core::watcher::install_watcher(&launcher_path, default_debug_port()) {
        Ok(()) => ok("守护进程已安装。", watcher_payload()),
        Err(error) => failed(&format!("安装守护进程失败：{error}"), watcher_payload()),
    }
}

#[tauri::command]
pub fn uninstall_watcher() -> CommandResult<WatcherPayload> {
    match claude_codex_pro_core::watcher::uninstall_watcher() {
        Ok(()) => ok("守护进程已卸载。", watcher_payload()),
        Err(error) => failed(&format!("卸载守护进程失败：{error}"), watcher_payload()),
    }
}

#[tauri::command]
pub fn enable_watcher() -> CommandResult<WatcherPayload> {
    match claude_codex_pro_core::watcher::enable_watcher() {
        Ok(()) => ok("守护进程已启用。", watcher_payload()),
        Err(error) => failed(&format!("启用守护进程失败：{error}"), watcher_payload()),
    }
}

#[tauri::command]
pub fn disable_watcher() -> CommandResult<WatcherPayload> {
    match claude_codex_pro_core::watcher::disable_watcher() {
        Ok(()) => ok("守护进程已禁用。", watcher_payload()),
        Err(error) => failed(&format!("禁用守护进程失败：{error}"), watcher_payload()),
    }
}

#[tauri::command]
pub async fn read_latest_logs(request: LogRequest) -> CommandResult<LogsPayload> {
    // Tailing the log is disk IO; keep it off the UI thread.
    let lines = request.lines;
    tauri::async_runtime::spawn_blocking(move || read_latest_logs_blocking(lines))
        .await
        .unwrap_or_else(|join_error| {
            let path = claude_codex_pro_core::paths::default_diagnostic_log_path();
            failed(
                &format!("读取日志失败：{join_error}"),
                LogsPayload {
                    path: path.to_string_lossy().to_string(),
                    text: String::new(),
                    lines,
                },
            )
        })
}

fn read_latest_logs_blocking(lines: usize) -> CommandResult<LogsPayload> {
    let path = claude_codex_pro_core::paths::default_diagnostic_log_path();
    match read_tail(&path, lines) {
        Ok(text) => ok(
            "日志已加载。",
            LogsPayload {
                path: path.to_string_lossy().to_string(),
                text,
                lines,
            },
        ),
        Err(error) => failed(
            &format!("读取日志失败：{error}"),
            LogsPayload {
                path: path.to_string_lossy().to_string(),
                text: String::new(),
                lines,
            },
        ),
    }
}

#[tauri::command]
pub async fn copy_diagnostics() -> CommandResult<DiagnosticsPayload> {
    // diagnostics_report() probes loopback TCP ports, reads HTTP responses and
    // tails the diagnostic log — all synchronous blocking IO that froze the UI
    // thread for up to a couple of seconds. Run it on the blocking pool.
    let report = tauri::async_runtime::spawn_blocking(diagnostics_report)
        .await
        .unwrap_or_else(|join_error| format!("诊断报告任务失败：{join_error}"));
    ok("诊断报告已生成。", DiagnosticsPayload { report })
}

#[tauri::command]
pub fn reset_settings() -> CommandResult<SettingsPayload> {
    let settings = BackendSettings::default();
    match SettingsStore::default().save(&settings) {
        Ok(()) => settings_payload("设置已重置为默认值。", "设置重置失败"),
        Err(error) => failed(
            &format!("设置重置失败：{error}"),
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
        Ok(()) => settings_payload("图片叠加设置已重置。", "图片叠加重置失败"),
        Err(error) => failed(
            &format!("图片叠加重置失败：{error}"),
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
        "已检测到 ChatGPT 登录状态。"
    } else {
        "未检测到 ChatGPT 登录状态。你仍可配置 Codex API 模式。"
    };
    ok(message, relay_payload(status, None))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearCredentialEnvironmentRequest {
    pub variable_name: String,
}

#[tauri::command]
pub fn diagnose_codex_credential_environment() -> CommandResult<CredentialEnvironmentDiagnostic> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    let diagnostic =
        claude_codex_pro_core::credential_environment::diagnose_codex_credential_environment(
            &settings,
        );
    let message = if diagnostic.conflict {
        format!(
            "检测到 {} 与当前 Codex 供应商凭据不一致，可能导致 401 Invalid token。",
            diagnostic.variable_name
        )
    } else if diagnostic.present {
        format!(
            "检测到 {} 环境变量，当前未发现与活动供应商的值冲突。",
            diagnostic.variable_name
        )
    } else {
        format!("未检测到 {} 环境变量。", diagnostic.variable_name)
    };
    ok(&message, diagnostic)
}

#[tauri::command]
pub fn clear_codex_user_credential_environment(
    request: ClearCredentialEnvironmentRequest,
) -> CommandResult<CredentialEnvironmentDiagnostic> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    match claude_codex_pro_core::credential_environment::clear_codex_user_credential_environment(
        &settings,
        &request.variable_name,
    ) {
        Ok(diagnostic) => ok(
            "用户级凭据环境变量已清理。当前运行中的 Codex 仍可能保留旧值，请完全退出后重新启动。",
            diagnostic,
        ),
        Err(error) => failed(
            &format!("清理用户级凭据环境变量失败：{error}"),
            claude_codex_pro_core::credential_environment::diagnose_codex_credential_environment(
                &settings,
            ),
        ),
    }
}

#[tauri::command]
pub fn read_relay_files() -> CommandResult<RelayFilesPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    match relay_files_payload_from_home(&home) {
        Ok(payload) => ok("中转文件已加载。", payload),
        Err(error) => failed(
            &format!("读取中转文件失败：{error}"),
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
        Ok(payload) => ok("中转文件已保存。", payload),
        Err(error) => failed(
            &format!("保存中转文件失败：{error}"),
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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierProfileSwitchRequest {
    pub settings: BackendSettings,
    pub target_app: String,
    pub profile_id: String,
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
            "未在 ~/.cc-switch/cc-switch.db 找到 cc-switch 数据库。",
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
                &format!("已从 cc-switch 导入 {count} 个 Codex / Claude 供应商配置。"),
                CcswitchImportPayload {
                    db_path: db_path.to_string_lossy().to_string(),
                    profiles,
                    scanned,
                },
            )
        }
        Err(error) => failed(
            &format!("读取 cc-switch 供应商失败：{error}"),
            CcswitchImportPayload {
                db_path: db_path.to_string_lossy().to_string(),
                profiles: Vec::new(),
                scanned: 0,
            },
        ),
    }
}

#[tauri::command]
pub async fn switch_relay_profile(
    request: RelayProfileSwitchRequest,
) -> CommandResult<RelaySwitchPayload> {
    // The switch performs synchronous config/auth/backup file IO. Running it on
    // the UI thread froze the WebView; move it to a blocking thread. The lock is
    // acquired *inside* the closure because a std MutexGuard is `!Send` and can
    // neither cross an await point nor move into a Send closure.
    tauri::async_runtime::spawn_blocking(move || switch_relay_profile_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            let status = claude_codex_pro_core::relay_config::default_relay_status();
            failed(
                &format!("切换供应商配置任务失败：{join_error}"),
                relay_switch_payload(
                    SettingsStore::default().load().unwrap_or_default(),
                    status,
                    None,
                ),
            )
        })
}

#[tauri::command]
pub async fn switch_supplier_profile(
    request: SupplierProfileSwitchRequest,
) -> CommandResult<SettingsPayload> {
    let target_app = match normalized_supplier_target(&request.target_app) {
        Ok(target_app) => target_app.to_string(),
        Err(error) => {
            return failed(&error.to_string(), fallback_settings_payload());
        }
    };
    if target_app == "claude-desktop" {
        let proxy_port = match ensure_claude_desktop_proxy_helper().await {
            Ok(port) => port,
            Err(error) => {
                return failed(
                    &format!("切换 Claude Desktop 供应商前启动本地代理失败：{error}"),
                    fallback_settings_payload(),
                );
            }
        };
        return tauri::async_runtime::spawn_blocking(move || {
            switch_claude_desktop_supplier_blocking(request, proxy_port)
        })
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("切换 Claude Desktop 供应商任务失败：{join_error}"),
                fallback_settings_payload(),
            )
        });
    }

    tauri::async_runtime::spawn_blocking(move || match target_app.as_str() {
        "codex" => switch_codex_supplier_blocking(request),
        "claude" => switch_claude_supplier_blocking(request),
        _ => unreachable!("target was normalized before spawning"),
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("切换供应商任务失败：{join_error}"),
            fallback_settings_payload(),
        )
    })
}

fn switch_codex_supplier_blocking(
    mut request: SupplierProfileSwitchRequest,
) -> CommandResult<SettingsPayload> {
    if let Err(error) =
        set_active_supplier_profile_for_target(&mut request.settings, "codex", &request.profile_id)
    {
        return failed(&error.to_string(), fallback_settings_payload());
    }
    let result = switch_relay_profile_blocking(RelayProfileSwitchRequest {
        settings: request.settings,
        previous_active_relay_id: request.previous_active_relay_id,
    });
    CommandResult {
        status: result.status,
        message: if result.message.is_empty() {
            "Codex 供应商已切换。".to_string()
        } else {
            result.message
        },
        payload: SettingsPayload {
            settings: result.payload.settings,
            settings_path: result.payload.settings_path,
            user_scripts: result.payload.user_scripts,
        },
    }
}

fn switch_claude_supplier_blocking(
    mut request: SupplierProfileSwitchRequest,
) -> CommandResult<SettingsPayload> {
    let store = SettingsStore::default();
    let previous = store.load().unwrap_or_default();
    request.settings = normalize_settings_before_save(request.settings);
    if let Err(error) =
        set_active_supplier_profile_for_target(&mut request.settings, "claude", &request.profile_id)
    {
        return failed(&error.to_string(), fallback_settings_payload());
    }
    let profile = request.settings.active_relay_profile_for_target("claude");
    log_manager_event(
        "manager.switch_supplier_profile.start",
        json!({ "targetApp": "claude", "profileId": profile.id }),
    );
    if let Err(error) = store.save(&request.settings) {
        return failed(
            &format!("保存 Claude 当前供应商失败：{error}"),
            fallback_settings_payload(),
        );
    }
    match claude_codex_pro_core::claude_provider::apply_claude_provider(&profile) {
        Ok(outcome) => {
            log_manager_event(
                "manager.switch_supplier_profile.ok",
                json!({
                    "targetApp": "claude",
                    "profileId": profile.id,
                    "settingsPath": outcome.settings_path,
                    "mergedEnvKeys": outcome.merged_env_keys
                }),
            );
            settings_payload("Claude 供应商已切换并新增式写入配置。", "刷新设置失败")
        }
        Err(error) => {
            let rollback_error = store.save(&previous).err();
            log_manager_event(
                "manager.switch_supplier_profile.failed",
                json!({
                    "targetApp": "claude",
                    "profileId": profile.id,
                    "error": error.to_string(),
                    "settingsRollbackFailed": rollback_error.is_some()
                }),
            );
            failed(
                &format!(
                    "切换 Claude 供应商失败：{error}{}",
                    rollback_error
                        .map(|error| format!("；管理工具设置回滚失败：{error}"))
                        .unwrap_or_default()
                ),
                fallback_settings_payload(),
            )
        }
    }
}

fn switch_claude_desktop_supplier_blocking(
    mut request: SupplierProfileSwitchRequest,
    proxy_port: u16,
) -> CommandResult<SettingsPayload> {
    let store = SettingsStore::default();
    let previous = store.load().unwrap_or_default();
    request.settings = normalize_settings_before_save(request.settings);
    if let Err(error) = set_active_supplier_profile_for_target(
        &mut request.settings,
        "claude-desktop",
        &request.profile_id,
    ) {
        return failed(&error.to_string(), fallback_settings_payload());
    }
    let profile = request
        .settings
        .active_relay_profile_for_target("claude-desktop");
    let api_key = relay_profile_resolved_api_key(&profile);
    if api_key.trim().is_empty() {
        return failed(
            "Claude Desktop 供应商缺少 API Key，未写入不完整配置。",
            fallback_settings_payload(),
        );
    }
    log_manager_event(
        "manager.switch_supplier_profile.start",
        json!({
            "targetApp": "claude-desktop",
            "profileId": profile.id,
            "modelLines": profile.model_list.lines().filter(|line| !line.trim().is_empty()).count(),
            "modelMappingEnabled": profile.model_mapping_enabled,
            "proxyPort": proxy_port
        }),
    );
    if let Err(error) = store.save(&request.settings) {
        return failed(
            &format!("保存 Claude Desktop 当前供应商失败：{error}"),
            fallback_settings_payload(),
        );
    }
    match plugin_hub::configure_claude_desktop_supplier_with_proxy_port(
        &profile,
        &request.settings,
        proxy_port,
    ) {
        Ok(outcome) => {
            log_manager_event(
                "manager.switch_supplier_profile.ok",
                json!({
                    "targetApp": "claude-desktop",
                    "profileId": profile.id,
                    "profilePath": outcome.profile_path,
                    "proxyPort": proxy_port
                }),
            );
            settings_payload(
                "Claude Desktop 供应商已切换，模型目录已写入；请完全退出并重启 Claude Desktop。",
                "刷新设置失败",
            )
        }
        Err(error) => {
            let rollback_error = store.save(&previous).err();
            log_manager_event(
                "manager.switch_supplier_profile.failed",
                json!({
                    "targetApp": "claude-desktop",
                    "profileId": profile.id,
                    "error": error.to_string(),
                    "settingsRollbackFailed": rollback_error.is_some()
                }),
            );
            failed(
                &format!(
                    "切换 Claude Desktop 供应商失败：{error}{}",
                    rollback_error
                        .map(|error| format!("；管理工具设置回滚失败：{error}"))
                        .unwrap_or_default()
                ),
                fallback_settings_payload(),
            )
        }
    }
}

fn normalized_supplier_target(target_app: &str) -> anyhow::Result<&'static str> {
    match target_app.trim().to_ascii_lowercase().as_str() {
        "codex" | "" => Ok("codex"),
        "claude" => Ok("claude"),
        "claude-desktop" | "claude_desktop" | "claudedesktop" => Ok("claude-desktop"),
        _ => bail!("不支持的供应商目标：{target_app}"),
    }
}

fn set_active_supplier_profile_for_target(
    settings: &mut BackendSettings,
    target_app: &str,
    profile_id: &str,
) -> anyhow::Result<()> {
    let target_app = normalized_supplier_target(target_app)?;
    let profile = settings
        .relay_profiles
        .iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| anyhow::anyhow!("供应商不存在：{profile_id}"))?;
    let profile_target = normalized_supplier_target(&profile.target_app)?;
    if profile_target != target_app {
        bail!(
            "供应商目标不匹配：{} 属于 {}，不能用于 {}",
            profile.name,
            profile_target,
            target_app
        );
    }
    if profile.aggregate_enabled {
        bail!("聚合供应商尚不能直接切换使用。");
    }
    if relay_profile_resolved_api_key(profile).trim().is_empty() {
        bail!("该供应商缺少 API Key，请补入后再切换。");
    }
    match target_app {
        "claude" => settings.active_claude_relay_id = profile_id.to_string(),
        "claude-desktop" => settings.active_claude_desktop_relay_id = profile_id.to_string(),
        _ => settings.active_relay_id = profile_id.to_string(),
    }
    Ok(())
}

fn switch_relay_profile_blocking(
    request: RelayProfileSwitchRequest,
) -> CommandResult<RelaySwitchPayload> {
    // `try_lock`, not `lock`: on contention std's `lock()` blocks (it only errors
    // on poisoning), so the original "already running" branch was dead code and
    // rapid double-clicks serialized behind each other's full file IO. Fail fast
    // instead so the second click returns immediately.
    let Ok(_guard) = relay_switch_mutex().try_lock() else {
        let status = claude_codex_pro_core::relay_config::default_relay_status();
        return failed(
            "正在切换供应商，请稍后再试。",
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
            if let Err(error) = sync_codex_credential_environment_after_apply(&home) {
                return failed(
                    &format!(
                        "供应商配置已切换，但同步 Codex 启动凭据失败：{error}。请修复后重新点击使用。"
                    ),
                    relay_switch_payload(result.settings, status, result.backup_path),
                );
            }
            log_manager_event(
                "manager.switch_relay_profile.ok",
                json!({
                    "targetRelayId": result.settings.active_relay_id,
                    "configured": status.configured,
                    "backupPath": result.backup_path.as_ref()
                }),
            );
            ok(
                "供应商配置已切换。",
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
                &format!("切换供应商配置失败：{error}"),
                relay_switch_payload(settings, status, None),
            )
        }
    }
}

#[tauri::command]
pub async fn preview_claude_desktop_provider(
    request: ClaudeDesktopProviderRequest,
) -> CommandResult<ClaudeDesktopProviderPreviewPayload> {
    // current_claude_desktop_proxy_port_hint does a loopback bind test plus a
    // synchronous HTTP probe — blocking IO that must stay off the UI thread.
    tauri::async_runtime::spawn_blocking(move || preview_claude_desktop_provider_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("Claude Desktop 供应商预览任务失败：{join_error}"),
                ClaudeDesktopProviderPreviewPayload {
                    preview: empty_claude_desktop_provider_preview(),
                },
            )
        })
}

fn preview_claude_desktop_provider_blocking(
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
            &format!("已生成 Claude Desktop 供应商预览，本地代理端口 {proxy_port}。"),
            ClaudeDesktopProviderPreviewPayload { preview },
        ),
        Err(error) => failed(
            &format!("生成 Claude Desktop 供应商预览失败：{error}"),
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
    if let Err(error) = plugin_hub::persist_claude_desktop_provider_request_to_settings(&request) {
        log_manager_event(
            "manager.claude_desktop_provider.apply.settings_failed",
            json!({ "error": error.to_string() }),
        );
        return failed(
            &format!("保存 Claude Desktop 当前供应商失败：{error}"),
            ClaudeDesktopProviderApplyPayload {
                outcome: empty_claude_desktop_provider_outcome(error.to_string()),
                dev_mode_status: plugin_hub::load_claude_desktop_dev_mode_status(),
            },
        );
    }
    let proxy_port = match ensure_claude_desktop_proxy_helper().await {
        Ok(port) => port,
        Err(error) => {
            log_manager_event(
                "manager.claude_desktop_provider.apply.proxy_failed",
                json!({ "error": error.to_string() }),
            );
            return failed(
                &format!("写入供应商配置前本地模型代理启动失败：{error}"),
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
                &format!("应用 Claude Desktop 供应商失败：{error}"),
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
                &format!("还原 Claude Desktop 官方供应商失败：{error}"),
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
        Ok(()) => ok("诊断事件已写入。", json!({})),
        Err(error) => failed(&format!("写入诊断事件失败：{error}"), json!({})),
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
            "未找到所选的供应商配置。请先保存后重试。",
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
                "供应商配置已从实时中转文件回填。",
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
                &format!("从实时中转文件回填供应商配置失败：{error}"),
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
            "上下文条目已加载。",
            ContextEntriesPayload {
                settings: request.settings,
                entries,
            },
        ),
        Err(error) => failed(
            &format!("加载上下文条目失败：{error}"),
            ContextEntriesPayload {
                settings: request.settings,
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub async fn read_live_context_entries() -> CommandResult<LiveContextEntriesPayload> {
    // 读取 Codex config.toml 并解析上下文条目，属磁盘 IO，移出主 IPC 线程。
    tauri::async_runtime::spawn_blocking(read_live_context_entries_blocking)
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("加载实时上下文条目任务失败：{join_error}"),
                LiveContextEntriesPayload {
                    entries: empty_context_entries(),
                },
            )
        })
}

#[tauri::command]
pub async fn scan_unified_tool_inventory() -> CommandResult<UnifiedToolInventoryPayload> {
    tauri::async_runtime::spawn_blocking(scan_unified_tool_inventory_blocking)
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("检测 Claude、Codex 工具与插件任务失败：{join_error}"),
                UnifiedToolInventoryPayload {
                    inventory: Default::default(),
                },
            )
        })
}

fn scan_unified_tool_inventory_blocking() -> CommandResult<UnifiedToolInventoryPayload> {
    let roots = claude_codex_pro_core::unified_tool_inventory::UnifiedToolInventoryRoots::default();
    match claude_codex_pro_core::unified_tool_inventory::scan_unified_tool_inventory(&roots) {
        Ok(inventory) => {
            log_manager_event(
                "manager.unified_tool_inventory.scan.ok",
                json!({
                    "total": inventory.counts.total,
                    "rawDiscoveries": inventory.counts.raw_discoveries,
                    "deduplicated": inventory.counts.deduplicated,
                    "mcp": inventory.counts.mcp,
                    "skills": inventory.counts.skills,
                    "plugins": inventory.counts.plugins,
                    "codexEnabled": inventory.counts.codex_enabled,
                    "claudeEnabled": inventory.counts.claude_enabled,
                    "diagnosticCount": inventory.diagnostics.len()
                }),
            );
            ok(
                &format!(
                    "检测完成：原始发现 {}，合并重复 {}，统一条目 {}；MCP {}、Skills {}、插件 {}；Codex 已启用 {}，Claude 已启用 {}。",
                    inventory.counts.raw_discoveries,
                    inventory.counts.deduplicated,
                    inventory.counts.total,
                    inventory.counts.mcp,
                    inventory.counts.skills,
                    inventory.counts.plugins,
                    inventory.counts.codex_enabled,
                    inventory.counts.claude_enabled
                ),
                UnifiedToolInventoryPayload { inventory },
            )
        }
        Err(error) => {
            log_manager_event(
                "manager.unified_tool_inventory.scan.failed",
                json!({ "error": error.to_string() }),
            );
            failed(
                &format!("检测 Claude、Codex 工具与插件失败：{error}"),
                UnifiedToolInventoryPayload {
                    inventory: Default::default(),
                },
            )
        }
    }
}

#[tauri::command]
pub async fn toggle_unified_tool_asset(
    request: UnifiedToolToggleRequest,
) -> CommandResult<UnifiedToolInventoryPayload> {
    tauri::async_runtime::spawn_blocking(move || toggle_unified_tool_asset_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("切换工具或插件任务失败：{join_error}"),
                UnifiedToolInventoryPayload {
                    inventory: Default::default(),
                },
            )
        })
}

fn toggle_unified_tool_asset_blocking(
    request: UnifiedToolToggleRequest,
) -> CommandResult<UnifiedToolInventoryPayload> {
    let roots = claude_codex_pro_core::unified_tool_inventory::UnifiedToolInventoryRoots::default();
    let core_request = claude_codex_pro_core::unified_tool_inventory::UnifiedToolToggleRequest {
        id: request.id.clone(),
        kind: request.kind.clone(),
        app: request.app.clone(),
        enabled: request.enabled,
    };
    match claude_codex_pro_core::unified_tool_inventory::set_unified_tool_asset_enabled(
        &roots,
        &core_request,
    ) {
        Ok(inventory) => {
            log_manager_event(
                "manager.unified_tool_inventory.toggle.ok",
                json!({
                    "assetId": request.id,
                    "kind": request.kind,
                    "app": request.app,
                    "enabled": request.enabled
                }),
            );
            ok(
                if request.enabled {
                    "已为目标应用启用该工具或插件。"
                } else {
                    "已为目标应用关闭该工具或插件，可随时恢复。"
                },
                UnifiedToolInventoryPayload { inventory },
            )
        }
        Err(error) => {
            log_manager_event(
                "manager.unified_tool_inventory.toggle.failed",
                json!({
                    "assetId": request.id,
                    "kind": request.kind,
                    "app": request.app,
                    "enabled": request.enabled,
                    "error": error.to_string()
                }),
            );
            let inventory =
                claude_codex_pro_core::unified_tool_inventory::scan_unified_tool_inventory(&roots)
                    .unwrap_or_default();
            failed(
                &format!("切换工具或插件失败：{error}"),
                UnifiedToolInventoryPayload { inventory },
            )
        }
    }
}

fn read_live_context_entries_blocking() -> CommandResult<LiveContextEntriesPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let config_path = home.join("config.toml");
    let config = read_optional_text_file(&config_path).unwrap_or_default();
    match claude_codex_pro_core::relay_config::list_context_entries_from_common_config(&config) {
        Ok(entries) => ok(
            "实时上下文条目已加载。",
            LiveContextEntriesPayload { entries },
        ),
        Err(error) => failed(
            &format!("加载实时上下文条目失败：{error}"),
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
            &format!("保存上下文条目失败：{error}"),
            ContextEntriesPayload {
                settings,
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub async fn sync_live_context_entries(
    request: ContextSettingsRequest,
) -> CommandResult<LiveContextEntriesPayload> {
    // Reads and rewrites live config.toml — synchronous file IO that must not run
    // on the UI thread.
    tauri::async_runtime::spawn_blocking(move || sync_live_context_entries_blocking(request))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("同步实时上下文条目任务失败：{join_error}"),
                LiveContextEntriesPayload {
                    entries: empty_context_entries(),
                },
            )
        })
}

fn sync_live_context_entries_blocking(
    request: ContextSettingsRequest,
) -> CommandResult<LiveContextEntriesPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let config_path = home.join("config.toml");
    let current_config = match read_optional_text_file(&config_path) {
        Ok(config) => config,
        Err(error) => {
            return failed(
                &format!("读取实时 config.toml 失败：{error}"),
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
                &format!("同步实时上下文条目失败：{error}"),
                LiveContextEntriesPayload {
                    entries: empty_context_entries(),
                },
            );
        }
    };
    if let Some(parent) = config_path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            return failed(
                &format!("创建 Codex 配置目录失败：{error}"),
                LiveContextEntriesPayload {
                    entries: empty_context_entries(),
                },
            );
        }
    }
    if let Err(error) = std::fs::write(&config_path, &updated_config) {
        return failed(
            &format!("写入实时 config.toml 失败：{error}"),
            LiveContextEntriesPayload {
                entries: empty_context_entries(),
            },
        );
    }
    match claude_codex_pro_core::relay_config::list_context_entries_from_common_config(
        &updated_config,
    ) {
        Ok(entries) => ok(
            "实时上下文条目已同步。",
            LiveContextEntriesPayload { entries },
        ),
        Err(error) => failed(
            &format!("读取已同步的实时上下文条目失败：{error}"),
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
            &format!("删除上下文条目失败：{error}"),
            ContextEntriesPayload {
                settings,
                entries: empty_context_entries(),
            },
        ),
    }
}

#[tauri::command]
pub async fn list_claude_context_entries() -> CommandResult<ClaudeContextEntriesPayload> {
    // 读取 Claude Desktop MCP 配置并扫描组织插件/插件仓库状态，均为磁盘 IO。
    // 放到阻塞线程池，避免工具与插件页挂载时并发触发导致窗口 "未响应"。
    tauri::async_runtime::spawn_blocking(list_claude_context_entries_blocking)
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("加载 Claude 上下文条目失败：{join_error}"),
                ClaudeContextEntriesPayload {
                    config_path: String::new(),
                    entries: empty_context_entries(),
                },
            )
        })
}

fn list_claude_context_entries_blocking() -> CommandResult<ClaudeContextEntriesPayload> {
    match plugin_hub::list_claude_desktop_mcp_entries() {
        Ok(mcp) => {
            let org = plugin_hub::load_claude_desktop_org_plugin_status();
            let market = plugin_hub::load_claude_desktop_marketplace_status();
            ok(
                "Claude 上下文条目已加载。",
                ClaudeContextEntriesPayload {
                    config_path: mcp.config_path,
                    entries: claude_entries_from_status(mcp.entries, org, market),
                },
            )
        }
        Err(error) => failed(
            &format!("加载 Claude 上下文条目失败：{error}"),
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
            "Claude 目前仅支持 MCP 上下文条目；技能和插件由 Claude 插件流程管理。",
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
                "Claude MCP 条目已保存。",
                ClaudeContextEntriesPayload {
                    config_path: mcp.config_path,
                    entries: claude_entries_from_status(mcp.entries, org, market),
                },
            )
        }
        Err(error) => failed(
            &format!("保存 Claude MCP 条目失败：{error}"),
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
            "Claude 目前仅支持删除 MCP 上下文条目。",
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
                "Claude MCP 条目已删除。",
                ClaudeContextEntriesPayload {
                    config_path: mcp.config_path,
                    entries: claude_entries_from_status(mcp.entries, org, market),
                },
            )
        }
        Err(error) => failed(
            &format!("删除 Claude MCP 条目失败：{error}"),
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
        Ok(payload) => ok("公共中转配置已提取。", payload),
        Err(error) => failed(
            &format!("提取公共中转配置失败：{error}"),
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
        "未命名供应商"
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
                "无响应预览。".to_string()
            } else {
                format!("预览：{preview}")
            };
            CommandResult {
                status: status.to_string(),
                message: format!(
                    "已使用模型 {test_model} 测试供应商 {profile_name}；HTTP {}；{detail}",
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
            &format!("供应商 {profile_name} 测试失败：{error}"),
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
        "未命名供应商"
    } else {
        profile.name.trim()
    };
    match claude_codex_pro_core::model_catalog::fetch_relay_profile_model_ids(&profile).await {
        Ok((models, endpoint)) => ok(
            &format!("已为供应商 {profile_name} 加载 {} 个模型。", models.len()),
            RelayProfileModelsPayload { models, endpoint },
        ),
        Err(error) => failed(
            &format!("加载供应商 {profile_name} 的模型列表失败：{error}"),
            RelayProfileModelsPayload {
                models: Vec::new(),
                endpoint: String::new(),
            },
        ),
    }
}

#[tauri::command]
pub async fn apply_relay_injection() -> CommandResult<RelayPayload> {
    // Writes config.toml/auth.json, creates backups and runs the switch rules —
    // all synchronous file IO. Keep it off the UI thread.
    tauri::async_runtime::spawn_blocking(apply_relay_injection_blocking)
        .await
        .unwrap_or_else(|join_error| {
            let status = claude_codex_pro_core::relay_config::default_relay_status();
            failed(
                &format!("注入 Relay 配置的后台任务失败：{join_error}"),
                relay_payload(status, None),
            )
        })
}

fn apply_relay_injection_blocking() -> CommandResult<RelayPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let settings = SettingsStore::default().load().unwrap_or_default();
    if !settings.relay_profiles_enabled {
        let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
        return failed(
            "供应商配置已禁用；未写入 config.toml 和 auth.json。",
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
                if let Err(error) = sync_codex_credential_environment_after_apply(&home) {
                    return failed(
                        &format!(
                            "供应商配置已写入，但同步 Codex 启动凭据失败：{error}。请修复后重新应用。"
                        ),
                        relay_payload(status, result.backup_path),
                    );
                }
                log_relay_apply_result(
                    "manager.apply_relay_injection.ok",
                    &relay,
                    &status,
                    result.backup_path.as_ref(),
                    None,
                );
                ok(
                    &format!(
                        "已按兼容规则切换供应商。{}",
                        chat_completions_proxy_warning_suffix(&relay)
                    ),
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
                    &format!("完整中转配置切换失败：{error}"),
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
            Some("未检测到 ChatGPT 登录状态".to_string()),
        );
        return failed(
            "未检测到 ChatGPT 登录状态，因此未写入中转配置。",
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
            if let Err(error) = sync_codex_credential_environment_after_apply(&home) {
                return failed(
                    &format!(
                        "Relay 配置已写入，但同步 Codex 启动凭据失败：{error}。请修复后重新应用。"
                    ),
                    relay_payload(status, result.backup_path),
                );
            }
            log_relay_apply_result(
                "manager.apply_relay_injection.ok",
                &relay,
                &status,
                result.backup_path.as_ref(),
                None,
            );
            ok(
                &format!(
                    "Relay 配置已写入。API 密钥不会在界面上显示。{}",
                    chat_completions_proxy_warning_suffix(&relay)
                ),
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
                &format!("写入 Relay 配置失败：{error}"),
                relay_payload(status, None),
            )
        }
    }
}

#[tauri::command]
pub async fn apply_pure_api_injection() -> CommandResult<RelayPayload> {
    // Writes config.toml/auth.json plus backups — synchronous file IO that must
    // not run on the UI thread.
    tauri::async_runtime::spawn_blocking(apply_pure_api_injection_blocking)
        .await
        .unwrap_or_else(|join_error| {
            let status = claude_codex_pro_core::relay_config::default_relay_status();
            failed(
                &format!("纯 API 注入任务失败：{join_error}"),
                relay_payload(status, None),
            )
        })
}

fn apply_pure_api_injection_blocking() -> CommandResult<RelayPayload> {
    let home = claude_codex_pro_core::relay_config::default_codex_home_dir();
    let settings = SettingsStore::default().load().unwrap_or_default();
    if !settings.relay_profiles_enabled {
        let status = claude_codex_pro_core::relay_config::relay_status_from_home(&home);
        return failed(
            "供应商配置已禁用；未写入 config.toml 和 auth.json。",
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
                if !status.configured {
                    return failed(
                        "已写入纯 API 配置，但未检测到完整的自定义供应商。请检查 config.toml 与供应商 API key。",
                        relay_payload(status, result.backup_path),
                    );
                }
                if let Err(error) = sync_codex_credential_environment_after_apply(&home) {
                    return failed(
                        &format!(
                            "纯 API 配置已写入，但同步 Codex 启动凭据失败：{error}。请修复后重新应用。"
                        ),
                        relay_payload(status, result.backup_path),
                    );
                }
                log_relay_apply_result(
                    "manager.apply_pure_api_injection.ok",
                    &relay,
                    &status,
                    result.backup_path.as_ref(),
                    None,
                );
                ok(
                    "已按兼容规则切换供应商。",
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
                    &format!("纯 API 配置切换失败：{error}"),
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
            if !status.configured {
                return failed(
                    "纯 API 配置已写入，但未检测到完整的自定义供应商。请检查 config.toml 与供应商 API 密钥。",
                    relay_payload(status, result.backup_path),
                );
            }
            if let Err(error) = sync_codex_credential_environment_after_apply(&home) {
                return failed(
                    &format!(
                        "纯 API 配置已写入，但同步 Codex 启动凭据失败：{error}。请修复后重新应用。"
                    ),
                    relay_payload(status, result.backup_path),
                );
            }
            log_relay_apply_result(
                "manager.apply_pure_api_injection.ok",
                &relay,
                &status,
                result.backup_path.as_ref(),
                None,
            );
            ok(
                "纯 API 模式已写入：config.toml 使用自定义供应商，auth.json 使用所选供应商。",
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
                &format!("写入纯 API 模式失败：{error}"),
                relay_payload(status, None),
            )
        }
    }
}

#[tauri::command]
pub async fn clear_relay_injection() -> CommandResult<RelayPayload> {
    // Rewrites config.toml/auth.json and writes a backup — synchronous file IO
    // that must not run on the UI thread.
    tauri::async_runtime::spawn_blocking(clear_relay_injection_blocking)
        .await
        .unwrap_or_else(|join_error| {
            let status = claude_codex_pro_core::relay_config::default_relay_status();
            failed(
                &format!("清除中转注入任务失败：{join_error}"),
                relay_payload(status, None),
            )
        })
}

fn clear_relay_injection_blocking() -> CommandResult<RelayPayload> {
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
                "自定义中转 API 模式已清除，已切回官方 ChatGPT 登录模式。",
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
                &format!("清除中转配置失败：{error}"),
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

/// ChatCompletions profiles rewrite Codex's `base_url` to the local protocol
/// proxy (`http://127.0.0.1:57321/v1`), which only works while this tool's
/// helper is serving that port. If nothing is listening when we write the
/// config, a Codex started independently will fail to reach the proxy with no
/// visible reason. Return a user-facing warning suffix in that case so the
/// applied-config message tells the user the proxy must be running, instead of
/// silently writing a config that points at a dead port.
fn chat_completions_proxy_warning_suffix(
    relay: &claude_codex_pro_core::settings::RelayProfile,
) -> String {
    if relay.protocol != claude_codex_pro_core::settings::RelayProtocol::ChatCompletions {
        return String::new();
    }
    let proxy_port = claude_codex_pro_core::protocol_proxy::DEFAULT_PROTOCOL_PROXY_PORT;
    if claude_codex_pro_core::launcher::protocol_proxy_backend_online(proxy_port) {
        return String::new();
    }
    format!(
        " 注意：ChatCompletions 协议依赖本地代理端口 {proxy_port}，当前未检测到本工具的代理在监听。请通过本工具启动 Codex（而非直接运行），否则模型请求会连不上本地代理。"
    )
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

fn sync_codex_credential_environment_after_apply(home: &Path) -> anyhow::Result<()> {
    let Some(result) = claude_codex_pro_core::credential_environment::
        sync_codex_user_credential_environment_from_home(home)?
    else {
        return Ok(());
    };
    log_manager_event(
        "manager.codex_credential_environment.synced",
        json!({
            "variableName": result.variable_name,
            "userChanged": result.user_changed,
            "processChanged": result.process_changed
        }),
    );
    Ok(())
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
            " 命令封装器已更新：{}。",
            install.real_codex.to_string_lossy()
        ),
        Ok(None) => String::new(),
        Err(error) => format!(" 命令封装器更新失败：{error}。"),
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
                format!("已安装：{}", org.ponytail_plugin_dir)
            } else {
                format!("未安装：{}", org.org_plugins_dir)
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
        other => anyhow::bail!("未知的中转文件类型：{other}"),
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
            .map_err(|error| anyhow::anyhow!("启动系统浏览器失败：{error}"))
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

/// Keys whose value carries a secret and must be masked in the diagnostics
/// report. Matched case-insensitively against JSON object keys at any depth so
/// that newly added secret-bearing fields are covered by default rather than
/// leaking until someone remembers to update this list.
const DIAGNOSTICS_SECRET_KEY_MARKERS: &[&str] = &[
    "apikey",
    "authcontents",
    "configcontents",
    "commonconfigcontents",
    "contextconfigcontents",
    "bearertoken",
    "token",
    "secret",
    "password",
];

/// Recursively replace the values of secret-bearing keys with a placeholder.
/// Empty strings are left as-is so the report still shows whether a field was
/// configured at all.
fn redact_settings_secrets(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut redacted = serde_json::Map::with_capacity(map.len());
            for (key, child) in map {
                let lower = key.to_ascii_lowercase();
                let is_secret = DIAGNOSTICS_SECRET_KEY_MARKERS
                    .iter()
                    .any(|marker| lower.contains(marker));
                if is_secret {
                    match child {
                        Value::String(text) if text.is_empty() => {
                            redacted.insert(key, Value::String(String::new()));
                        }
                        Value::Null => {
                            redacted.insert(key, Value::Null);
                        }
                        _ => {
                            redacted.insert(key, Value::String("***redacted***".to_string()));
                        }
                    }
                } else {
                    redacted.insert(key, redact_settings_secrets(child));
                }
            }
            Value::Object(redacted)
        }
        Value::Array(items) => {
            Value::Array(items.into_iter().map(redact_settings_secrets).collect())
        }
        other => other,
    }
}

fn diagnostics_report() -> String {
    let (codex_app_path, entrypoints, latest_launch) = load_overview_payload();
    let overview = ok(
        "概览已加载。",
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
    // The diagnostics report is meant to be copied into issues / support chats.
    // Serializing settings verbatim would leak relay API keys, CLI wrapper keys,
    // and the bearer tokens embedded in auth/config blobs. Redact them first.
    let redacted_settings =
        redact_settings_secrets(serde_json::to_value(&settings).unwrap_or_else(|_| json!({})));
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
        "settings": redacted_settings,
        "logs": {
            "diagnosticLogPath": claude_codex_pro_core::paths::default_diagnostic_log_path(),
            "latestStatusPath": claude_codex_pro_core::paths::default_latest_status_path()
        },
        "platform": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH
        }
    }))
    .unwrap_or_else(|error| format!("诊断报告序列化失败：{error}"))
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

/// Pure self-heal decision for the recorded helper port.
///
/// The helper port drifts: when the default port is occupied at launch,
/// `select_platform_loopback_port` falls back to a random free port and writes
/// that into `latest-status.json`. When that instance dies and a fresh backend
/// rebinds the default port, the status file still points at the dead random
/// port. Given whether the recorded port and the default port are each online,
/// this returns the `(helper_port, helper_port_online)` the overview should use:
/// prefer the recorded port when it is online, otherwise self-heal to the
/// default port when *it* is online.
fn resolve_helper_port_status(
    recorded_port: Option<u16>,
    recorded_online: bool,
    default_port: u16,
    default_online: bool,
) -> (Option<u16>, bool) {
    if recorded_online {
        return (recorded_port, true);
    }
    if default_online && recorded_port != Some(default_port) {
        return (Some(default_port), true);
    }
    (recorded_port, recorded_online)
}

fn refresh_launch_port_status(mut status: LaunchStatus) -> LaunchStatus {
    status.debug_port_online = status
        .debug_port
        .is_some_and(|port| codex_debug_port_online(port));

    let recorded_port = status.helper_port;
    let recorded_online = recorded_port.is_some_and(helper_backend_online);
    let default_port = default_helper_port();
    // Only probe the default port when the recorded port isn't already online,
    // so the common (healthy) path keeps a single probe.
    let default_online = !recorded_online
        && recorded_port != Some(default_port)
        && helper_backend_online(default_port);
    let (resolved_port, resolved_online) =
        resolve_helper_port_status(recorded_port, recorded_online, default_port, default_online);
    let helper_healed = resolved_port != recorded_port || resolved_online != recorded_online;
    status.helper_port = resolved_port;
    status.helper_port_online = resolved_online;

    if let Some(heartbeat) = latest_renderer_runtime_heartbeat().filter(|heartbeat| {
        renderer_heartbeat_is_current(heartbeat.timestamp_ms, Some(status.started_at_ms))
    }) {
        status.frontend_runtime_online = true;
        status.frontend_runtime_seen_at_ms = Some(heartbeat.timestamp_ms);
    } else {
        status.frontend_runtime_online = false;
        status.frontend_runtime_seen_at_ms = None;
    }

    // Persist the healed port so the next probe stops chasing the stale one.
    // Best-effort: a write failure must not change what the overview shows.
    if helper_healed {
        let _ = StatusStore::default().save_latest(&status);
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
    const READY_MARKER: &[u8] = b"webSocketDebuggerUrl";
    const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
    let mut response = Vec::with_capacity(4096);
    let mut buffer = [0_u8; 4096];
    while response.len() < MAX_RESPONSE_BYTES {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                response.extend_from_slice(&buffer[..read]);
                if response
                    .windows(READY_MARKER.len())
                    .any(|window| window == READY_MARKER)
                {
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(_) => return false,
        }
    }
    let response = String::from_utf8_lossy(&response);
    response.starts_with("HTTP/1.1 200") && response.contains("webSocketDebuggerUrl")
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
        message: format!("{action}后台任务失败：{error}"),
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
    // The diagnostic log is append-only and never rotated, so it grows without
    // bound. Reading the whole file into memory just to keep the last N lines got
    // slower and more memory-hungry the longer the app ran (a top suspect for the
    // "UI gets laggier over time" reports). Instead, read a bounded window from
    // the end of the file: for line-oriented log tails, the last few hundred KiB
    // always contain far more than any reasonable `max_lines`.
    use std::io::{Read, Seek, SeekFrom};

    if max_lines == 0 {
        return Ok(String::new());
    }

    // Cap how much we pull from the tail. 1 MiB comfortably holds thousands of
    // JSON log lines while keeping the read cheap regardless of total file size.
    const MAX_TAIL_BYTES: u64 = 1024 * 1024;

    let mut file = fs::File::open(path)?;
    let file_len = file.metadata()?.len();
    let read_len = file_len.min(MAX_TAIL_BYTES);
    let start = file_len - read_len;
    file.seek(SeekFrom::Start(start))?;

    let mut buffer = Vec::with_capacity(read_len as usize);
    file.take(read_len).read_to_end(&mut buffer)?;

    // The window may start mid-line when the file is larger than the cap; decode
    // lossily and drop a leading partial line so we never emit a truncated record.
    let text = String::from_utf8_lossy(&buffer);
    let mut slice: &str = &text;
    if start > 0 {
        if let Some(newline) = slice.find('\n') {
            slice = &slice[newline + 1..];
        }
    }

    let mut lines = slice.lines().rev().take(max_lines).collect::<Vec<_>>();
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
    .with_context(|| format!("无法打开 cc-switch 数据库: {}", db_path.display()))?;
    let has_meta_column = sqlite_table_has_column(&conn, "providers", "meta")?;
    let sql = if has_meta_column {
        "SELECT id, name, app_type, settings_config, COALESCE(meta, '{}')
         FROM providers
         WHERE app_type IN ('codex', 'claude', 'claude-desktop')
         ORDER BY COALESCE(sort_index, 0), name COLLATE NOCASE, id COLLATE NOCASE"
    } else {
        "SELECT id, name, app_type, settings_config, '{}'
         FROM providers
         WHERE app_type IN ('codex', 'claude', 'claude-desktop')
         ORDER BY COALESCE(sort_index, 0), name COLLATE NOCASE, id COLLATE NOCASE"
    };
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let mut profiles = Vec::new();
    let mut scanned = 0usize;
    for row in rows {
        scanned += 1;
        let (id, name, app_type, settings_config, meta) = row?;
        if let Some(profile) =
            ccswitch_profile_from_settings(&id, &name, &app_type, &settings_config, &meta)
        {
            profiles.push(profile);
        }
    }
    Ok((profiles, scanned))
}

fn sqlite_table_has_column(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
) -> anyhow::Result<bool> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        if row?.eq_ignore_ascii_case(column) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ccswitch_profile_from_settings(
    id: &str,
    name: &str,
    app_type: &str,
    settings: &str,
    meta: &str,
) -> Option<RelayProfile> {
    match app_type {
        "codex" => ccswitch_codex_profile_from_settings(id, name, settings),
        "claude" | "claude-desktop" => {
            ccswitch_claude_profile_from_settings(id, name, app_type, settings, meta)
        }
        _ => None,
    }
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
        .or_else(|| codex_config_api_key(config))
        .unwrap_or_default();
    let (codex_catalog_json, model_list) = ccswitch_codex_catalog(&parsed);
    let model = codex_config_model(config)
        .or_else(|| string_at(&parsed, &["model", "testModel"]))
        .or_else(|| model_list.lines().next().map(str::to_string))
        .unwrap_or_else(|| "gpt-5.5".to_string());
    let mut profile = RelayProfile {
        id: format!("{}-ccswitch", supplier_id_from_import(id)),
        name: format!("{name} (ccswitch)"),
        model,
        base_url: base_url.trim().to_string(),
        upstream_base_url: base_url.trim().to_string(),
        api_key: api_key.trim().to_string(),
        relay_mode: claude_codex_pro_core::settings::RelayMode::PureApi,
        user_agent: "ccswitch:codex".to_string(),
        import_source: "cc-switch".to_string(),
        target_app: "codex".to_string(),
        api_format: codex_config_api_format(config)
            .unwrap_or_else(|| "OpenAI Responses".to_string()),
        route_mode: "Codex provider config".to_string(),
        model_list,
        codex_catalog_json,
        ..RelayProfile::default()
    };
    profile.test_model = profile.model.clone();
    profile.config_contents = if config.trim().is_empty() {
        imported_supplier_config_toml(&profile)
    } else {
        config.to_string()
    };
    profile.auth_contents =
        if auth.is_null() || auth.as_object().is_some_and(|object| object.is_empty()) {
            format!(
                "{}\n",
                serde_json::to_string_pretty(&json!({ "OPENAI_API_KEY": profile.api_key }))
                    .unwrap_or_else(|_| "{}".to_string())
            )
        } else {
            format!(
                "{}\n",
                serde_json::to_string_pretty(auth).unwrap_or_else(|_| "{}".to_string())
            )
        };
    Some(profile)
}

fn ccswitch_codex_catalog(settings: &Value) -> (String, String) {
    let Some(models) = settings
        .get("modelCatalog")
        .and_then(|catalog| catalog.get("models"))
        .and_then(Value::as_array)
    else {
        return (String::new(), String::new());
    };
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    let mut model_ids = Vec::new();
    for item in models {
        let Some(model) = item
            .get("model")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|model| !model.is_empty())
        else {
            continue;
        };
        if !seen.insert(model.to_string()) {
            continue;
        }
        normalized.push(item.clone());
        model_ids.push(model.to_string());
    }
    (
        serde_json::to_string_pretty(&normalized).unwrap_or_else(|_| "[]".to_string()),
        model_ids.join("\n"),
    )
}

fn ccswitch_claude_profile_from_settings(
    id: &str,
    name: &str,
    app_type: &str,
    settings: &str,
    meta: &str,
) -> Option<RelayProfile> {
    let parsed = serde_json::from_str::<Value>(settings).ok()?;
    let parsed_meta = serde_json::from_str::<Value>(meta).unwrap_or(Value::Null);
    let env = parsed.get("env").unwrap_or(&Value::Null);
    let base_url = string_at(env, &["ANTHROPIC_BASE_URL", "CLAUDE_BASE_URL"])
        .or_else(|| string_at(&parsed, &["base_url", "baseUrl", "apiBaseUrl"]))
        .unwrap_or_default();
    if base_url.trim().is_empty() {
        return None;
    }
    let api_key = string_at(
        env,
        &[
            "ANTHROPIC_AUTH_TOKEN",
            "ANTHROPIC_API_KEY",
            "OPENROUTER_API_KEY",
            "GOOGLE_API_KEY",
            "apiKey",
        ],
    )
    .or_else(|| string_at(&parsed, &["apiKey", "api_key"]))
    .unwrap_or_default();
    let model = string_at(env, &["ANTHROPIC_MODEL", "ANTHROPIC_DEFAULT_SONNET_MODEL"])
        .map(|value| strip_ccswitch_one_m_marker(&value).0)
        .unwrap_or_else(|| "claude-sonnet".to_string());
    let routes = claude_model_routes_from_meta_or_env(&parsed_meta, env);
    let api_format = ccswitch_api_format_label(
        string_at(&parsed_meta, &["apiFormat", "api_format"])
            .or_else(|| string_at(&parsed, &["apiFormat", "api_format"])),
    );
    let claude_desktop_mode =
        string_at(&parsed_meta, &["claudeDesktopMode", "claude_desktop_mode"])
            .or_else(|| string_at(&parsed, &["claudeDesktopMode", "claude_desktop_mode"]))
            .unwrap_or_else(|| {
                if ccswitch_api_format_requires_route(&api_format) || !routes.is_empty() {
                    "proxy".to_string()
                } else {
                    "direct".to_string()
                }
            });
    let route_enabled = claude_desktop_mode.eq_ignore_ascii_case("proxy")
        || ccswitch_api_format_requires_route(&api_format);
    let model_mapping = claude_model_mapping_from_routes(&routes);
    let mut model_list = routes
        .iter()
        .filter_map(|route| {
            (!route.request_model.trim().is_empty()).then(|| route.request_model.clone())
        })
        .collect::<Vec<_>>();
    if model_list.is_empty() {
        model_list.push(model.clone());
    }
    model_list.sort();
    model_list.dedup();
    let mut profile = RelayProfile {
        id: format!("{}-ccswitch", supplier_id_from_import(id)),
        name: format!("{name} (ccswitch)"),
        model,
        base_url: base_url.trim().to_string(),
        upstream_base_url: base_url.trim().to_string(),
        api_key: api_key.trim().to_string(),
        relay_mode: claude_codex_pro_core::settings::RelayMode::PureApi,
        user_agent: format!("ccswitch:{app_type}"),
        import_source: "cc-switch".to_string(),
        target_app: app_type.to_string(),
        api_format,
        claude_desktop_mode: claude_desktop_mode.to_ascii_lowercase(),
        route_enabled,
        route_mode: if route_enabled {
            "Claude Desktop Proxy route".to_string()
        } else {
            "Claude Desktop Direct".to_string()
        },
        model_mapping: model_mapping.clone(),
        model_mapping_enabled: route_enabled || !model_mapping.trim().is_empty(),
        model_mapping_json: claude_model_mapping_json_from_routes(&routes),
        model_list: model_list.join("\n"),
        ..RelayProfile::default()
    };
    profile.test_model = profile.model.clone();
    profile.config_contents = format!(
        "{}\n",
        serde_json::to_string_pretty(&json!({
            "app_type": app_type,
            "env": env,
            "meta": parsed_meta
        }))
        .unwrap_or_else(|_| "{}".to_string())
    );
    profile.auth_contents = format!(
        "{}\n",
        serde_json::to_string_pretty(&json!({ "ANTHROPIC_AUTH_TOKEN": profile.api_key }))
            .unwrap_or_else(|_| "{}".to_string())
    );
    Some(profile)
}

#[derive(Debug, Clone)]
struct ClaudeRouteImportRow {
    role: &'static str,
    label: &'static str,
    route_id: String,
    display_name: String,
    request_model: String,
    supports_1m: bool,
}

fn claude_default_route_specs() -> [(&'static str, &'static str, &'static str, &'static str); 5] {
    [
        (
            "sonnet",
            "Sonnet",
            "claude-sonnet-4-6",
            "ANTHROPIC_DEFAULT_SONNET_MODEL",
        ),
        (
            "opus",
            "Opus",
            "claude-opus-4-8",
            "ANTHROPIC_DEFAULT_OPUS_MODEL",
        ),
        (
            "fable",
            "Fable",
            "claude-fable-5",
            "ANTHROPIC_DEFAULT_FABLE_MODEL",
        ),
        (
            "haiku",
            "Haiku",
            "claude-haiku-4-5",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        ),
        (
            "subagent",
            "Subagent",
            "claude-subagent",
            "CLAUDE_CODE_SUBAGENT_MODEL",
        ),
    ]
}

fn claude_model_routes_from_meta_or_env(meta: &Value, env: &Value) -> Vec<ClaudeRouteImportRow> {
    if let Some(routes) = meta
        .get("claudeDesktopModelRoutes")
        .or_else(|| meta.get("claude_desktop_model_routes"))
        .and_then(Value::as_object)
    {
        let mut rows = routes
            .iter()
            .filter_map(|(route_id, route)| {
                let request_model = string_at(route, &["model", "requestModel"])?;
                let (request_model, marker_supports_1m) =
                    strip_ccswitch_one_m_marker(&request_model);
                let role = claude_route_role(route_id).unwrap_or("sonnet");
                let label = claude_route_label(role);
                let display_name = string_at(route, &["labelOverride", "displayName"])
                    .unwrap_or_else(|| request_model.clone());
                Some(ClaudeRouteImportRow {
                    role,
                    label,
                    route_id: route_id.trim().to_string(),
                    display_name,
                    request_model,
                    supports_1m: route
                        .get("supports1m")
                        .or_else(|| route.get("supports_1m"))
                        .and_then(Value::as_bool)
                        .unwrap_or(marker_supports_1m),
                })
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| left.route_id.cmp(&right.route_id));
        if !rows.is_empty() {
            return rows;
        }
    }

    let mut rows = Vec::new();
    for (role, label, route_id, env_key) in claude_default_route_specs() {
        let Some(raw_model) = string_at(env, &[env_key]) else {
            continue;
        };
        let (request_model, _marker_supports_1m) = strip_ccswitch_one_m_marker(&raw_model);
        if request_model.trim().is_empty() {
            continue;
        }
        let display_name = string_at(env, &[&format!("{env_key}_NAME")]).unwrap_or_else(|| {
            if is_claude_safe_model_id(&request_model) {
                String::new()
            } else {
                request_model.clone()
            }
        });
        rows.push(ClaudeRouteImportRow {
            role,
            label,
            route_id: route_id.to_string(),
            display_name,
            request_model,
            supports_1m: true,
        });
    }
    if rows.is_empty() {
        if let Some(raw_model) = string_at(env, &["ANTHROPIC_MODEL"]) {
            let (request_model, _marker_supports_1m) = strip_ccswitch_one_m_marker(&raw_model);
            if !request_model.trim().is_empty() {
                rows.push(ClaudeRouteImportRow {
                    role: "sonnet",
                    label: "Sonnet",
                    route_id: "claude-sonnet-4-6".to_string(),
                    display_name: if is_claude_safe_model_id(&request_model) {
                        String::new()
                    } else {
                        request_model.clone()
                    },
                    request_model,
                    supports_1m: true,
                });
            }
        }
    }
    rows
}

fn strip_ccswitch_one_m_marker(value: &str) -> (String, bool) {
    let raw = value.trim();
    let marker = "[1M]";
    if raw.len() >= marker.len() && raw[raw.len() - marker.len()..].eq_ignore_ascii_case(marker) {
        (raw[..raw.len() - marker.len()].trim_end().to_string(), true)
    } else {
        (raw.to_string(), false)
    }
}

fn is_claude_safe_model_id(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.starts_with("claude-")
        && ["sonnet", "opus", "haiku", "fable"]
            .iter()
            .any(|role| value.contains(role))
}

fn claude_route_role(route_id: &str) -> Option<&'static str> {
    let lower = route_id.to_ascii_lowercase();
    if lower.contains("sonnet") {
        Some("sonnet")
    } else if lower.contains("opus") {
        Some("opus")
    } else if lower.contains("haiku") {
        Some("haiku")
    } else if lower.contains("fable") {
        Some("fable")
    } else if lower.contains("subagent") {
        Some("subagent")
    } else {
        None
    }
}

fn claude_route_label(role: &str) -> &'static str {
    match role {
        "opus" => "Opus",
        "haiku" => "Haiku",
        "fable" => "Fable",
        "subagent" => "Subagent",
        _ => "Sonnet",
    }
}

fn claude_model_mapping_from_routes(routes: &[ClaudeRouteImportRow]) -> String {
    routes
        .iter()
        .map(|route| {
            format!(
                "{} ({}): {} -> {}{}",
                route.label,
                route.route_id,
                if route.display_name.trim().is_empty() {
                    route.request_model.as_str()
                } else {
                    route.display_name.as_str()
                },
                route.request_model,
                if route.supports_1m { " [1M]" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn claude_model_mapping_json_from_routes(routes: &[ClaudeRouteImportRow]) -> String {
    let values = routes
        .iter()
        .map(|route| {
            json!({
                "role": route.role,
                "label": route.label,
                "routeId": route.route_id,
                "displayName": route.display_name,
                "requestModel": route.request_model,
                "supports1m": route.supports_1m
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&values).unwrap_or_default()
}

fn ccswitch_api_format_label(raw: Option<String>) -> String {
    match raw.unwrap_or_default().trim() {
        "anthropic" | "Anthropic Messages" | "" => "Anthropic Messages".to_string(),
        "openai_chat" | "OpenAI Chat Completions" => "OpenAI Chat Completions".to_string(),
        "openai_responses" | "OpenAI Responses" | "OpenAI Responses API" => {
            "OpenAI Responses API".to_string()
        }
        "gemini_native" | "Gemini Native" | "Gemini Native generateContent" => {
            "Gemini Native generateContent".to_string()
        }
        other => other.to_string(),
    }
}

fn ccswitch_api_format_requires_route(api_format: &str) -> bool {
    matches!(
        api_format,
        "OpenAI Chat Completions" | "OpenAI Responses API" | "Gemini Native generateContent"
    )
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

fn codex_config_api_key(config: &str) -> Option<String> {
    let doc = config.parse::<DocumentMut>().ok()?;
    if let Some(provider_id) = doc.get("model_provider").and_then(|item| item.as_str()) {
        if let Some(value) = doc
            .get("model_providers")
            .and_then(|item| item.as_table())
            .and_then(|providers| providers.get(provider_id))
            .and_then(|item| item.as_table())
            .and_then(|provider| provider.get("experimental_bearer_token"))
            .and_then(|item| item.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        {
            return Some(value);
        }
        if let Some(value) = doc
            .get("model_providers")
            .and_then(|item| item.as_table())
            .and_then(|providers| providers.get(provider_id))
            .and_then(|item| item.as_table())
            .and_then(|provider| provider.get("http_headers"))
            .and_then(|item| item.as_table())
            .and_then(|headers| headers.get("Authorization"))
            .and_then(|item| item.as_str())
            .and_then(bearer_token_from_authorization)
        {
            return Some(value);
        }
    }
    None
}

fn bearer_token_from_authorization(value: &str) -> Option<String> {
    value
        .trim()
        .strip_prefix("Bearer ")
        .or_else(|| value.trim().strip_prefix("bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn codex_config_api_format(config: &str) -> Option<String> {
    let doc = config.parse::<DocumentMut>().ok()?;
    let provider_id = doc.get("model_provider").and_then(|item| item.as_str())?;
    let wire_api = doc
        .get("model_providers")
        .and_then(|item| item.as_table())
        .and_then(|providers| providers.get(provider_id))
        .and_then(|item| item.as_table())
        .and_then(|provider| provider.get("wire_api"))
        .and_then(|item| item.as_str())
        .unwrap_or("responses");
    Some(
        match wire_api {
            "chat" | "chat_completions" | "chat-completions" => "OpenAI Chat Completions",
            "gemini" | "gemini_native" | "gemini-native" => "Gemini Native",
            _ => "OpenAI Responses",
        }
        .to_string(),
    )
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
    use std::{
        ffi::OsString,
        path::Path,
        sync::{Mutex, OnceLock},
    };

    fn test_path_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn target_supplier_selection_keeps_codex_and_claude_desktop_independent() {
        let mut settings = BackendSettings {
            active_relay_id: "codex-a".to_string(),
            relay_profiles: vec![
                RelayProfile {
                    id: "codex-a".to_string(),
                    target_app: "codex".to_string(),
                    ..RelayProfile::default()
                },
                RelayProfile {
                    id: "desktop-a".to_string(),
                    target_app: "claude-desktop".to_string(),
                    auth_contents: r#"{"ANTHROPIC_AUTH_TOKEN":"test-desktop-key"}"#.to_string(),
                    ..RelayProfile::default()
                },
            ],
            ..BackendSettings::default()
        };

        set_active_supplier_profile_for_target(&mut settings, "claude-desktop", "desktop-a")
            .unwrap();

        assert_eq!(settings.active_relay_id, "codex-a");
        assert_eq!(settings.active_claude_desktop_relay_id, "desktop-a");
    }

    struct TestEnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl Drop for TestEnvVarGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(previous) = &self.previous {
                    std::env::set_var(self.key, previous);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    fn set_test_codex_home(path: &Path) -> TestEnvVarGuard {
        let previous = std::env::var_os("CODEX_HOME");
        std::fs::create_dir_all(path).unwrap();
        unsafe {
            std::env::set_var("CODEX_HOME", path);
        }
        TestEnvVarGuard {
            key: "CODEX_HOME",
            previous,
        }
    }

    #[test]
    fn resolve_helper_port_prefers_recorded_when_online() {
        // Recorded port online → keep it, never mind the default.
        assert_eq!(
            resolve_helper_port_status(Some(55957), true, 57321, false),
            (Some(55957), true)
        );
        // Recorded port online AND equals default → still just the recorded one.
        assert_eq!(
            resolve_helper_port_status(Some(57321), true, 57321, false),
            (Some(57321), true)
        );
    }

    #[test]
    fn resolve_helper_port_self_heals_to_default_when_recorded_dead() {
        // The real bug: recorded 55957 is dead, default 57321 answers.
        assert_eq!(
            resolve_helper_port_status(Some(55957), false, 57321, true),
            (Some(57321), true)
        );
        // No recorded port at all, default answers → adopt the default.
        assert_eq!(
            resolve_helper_port_status(None, false, 57321, true),
            (Some(57321), true)
        );
    }

    #[test]
    fn resolve_helper_port_reports_offline_when_neither_answers() {
        assert_eq!(
            resolve_helper_port_status(Some(55957), false, 57321, false),
            (Some(55957), false)
        );
        assert_eq!(
            resolve_helper_port_status(None, false, 57321, false),
            (None, false)
        );
    }

    #[test]
    fn resolve_helper_port_does_not_flip_default_when_recorded_is_default() {
        // Recorded already is the default and it's offline; default_online can
        // never be true for the same port (caller guards it), so stay offline.
        assert_eq!(
            resolve_helper_port_status(Some(57321), false, 57321, false),
            (Some(57321), false)
        );
    }

    #[test]
    fn codex_debug_json_ready_accepts_valid_response_without_eof() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let response = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 59\r\n\r\n[{\"webSocketDebuggerUrl\":\"ws://127.0.0.1/devtools/page/1\"}]";
            stream.write_all(response).unwrap();
            std::thread::sleep(Duration::from_millis(750));
        });

        assert!(codex_debug_json_ready(port));
        server.join().unwrap();
    }

    #[test]
    fn codex_debug_json_ready_rejects_response_without_websocket_target() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n[]")
                .unwrap();
        });

        assert!(!codex_debug_json_ready(port));
        server.join().unwrap();
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
    fn diagnostics_report_redacts_settings_secrets() {
        // Simulates the settings blob that diagnostics_report serializes. The
        // report is meant to be copied into issues, so every secret-bearing
        // field must be masked while non-secret fields survive verbatim.
        let settings = json!({
            "relayApiKey": "sk-live-should-not-leak",
            "cliWrapperApiKey": "wrapper-key-should-not-leak",
            "relayBaseUrl": "https://relay.example.com/v1",
            "relayProfiles": [
                {
                    "id": "p1",
                    "name": "prod",
                    "authContents": "OPENAI_API_KEY=sk-should-not-leak",
                    "configContents": "experimental_bearer_token = \"sk-nope\"",
                    "upstreamBaseUrl": "https://up.example.com"
                }
            ],
            "relayCommonConfigContents": "token bearing blob"
        });

        let redacted = redact_settings_secrets(settings);

        assert_eq!(redacted["relayApiKey"], json!("***redacted***"));
        assert_eq!(redacted["cliWrapperApiKey"], json!("***redacted***"));
        assert_eq!(
            redacted["relayCommonConfigContents"],
            json!("***redacted***")
        );
        assert_eq!(
            redacted["relayProfiles"][0]["authContents"],
            json!("***redacted***")
        );
        assert_eq!(
            redacted["relayProfiles"][0]["configContents"],
            json!("***redacted***")
        );
        // Non-secret fields must be preserved so the report stays useful.
        assert_eq!(
            redacted["relayBaseUrl"],
            json!("https://relay.example.com/v1")
        );
        assert_eq!(redacted["relayProfiles"][0]["name"], json!("prod"));
        assert_eq!(
            redacted["relayProfiles"][0]["upstreamBaseUrl"],
            json!("https://up.example.com")
        );
    }

    #[test]
    fn ccswitch_import_reads_api_key_from_config_bearer_token() {
        let settings = json!({
            "config": r#"
model = "gpt-5.5"
model_provider = "relay"

[model_providers.relay]
name = "relay"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://relay.example/v1"
experimental_bearer_token = "sk-from-config"
"#,
            "auth": {}
        });

        let profile = ccswitch_codex_profile_from_settings(
            "Relay Provider",
            "Relay Provider",
            &settings.to_string(),
        )
        .expect("profile imported from config token");

        assert_eq!(profile.base_url, "https://relay.example/v1");
        assert_eq!(profile.api_key, "sk-from-config");
        assert!(profile.auth_contents.contains("sk-from-config"));
    }

    #[test]
    fn ccswitch_import_preserves_codex_config_and_reads_authorization_header() {
        let settings = json!({
            "config": r#"
model = "gpt-5.5"
model_provider = "relay"
model_instructions_file = "./custom-instructions.md"

[model_providers.relay]
name = "relay"
wire_api = "chat"
requires_openai_auth = true
base_url = "https://relay.example/v1"

[model_providers.relay.http_headers]
Authorization = "Bearer sk-from-header"

[plugins.example]
enabled = true
"#,
            "auth": {},
            "modelCatalog": {
                "models": [
                    {
                        "displayName": "DeepSeek V4 Flash",
                        "model": "deepseek-v4-flash",
                        "contextWindow": 128000
                    },
                    {
                        "displayName": "Qwen 3 Coder",
                        "model": "qwen3-coder",
                        "contextWindow": "200000"
                    }
                ]
            }
        });

        let profile = ccswitch_codex_profile_from_settings(
            "Relay Provider",
            "Relay Provider",
            &settings.to_string(),
        )
        .expect("profile imported from header token");

        assert_eq!(profile.api_key, "sk-from-header");
        assert_eq!(profile.api_format, "OpenAI Chat Completions");
        assert!(profile.config_contents.contains("model_instructions_file"));
        assert!(profile.config_contents.contains("[plugins.example]"));
        assert!(profile.auth_contents.contains("sk-from-header"));
        assert_eq!(profile.model_list, "deepseek-v4-flash\nqwen3-coder");
        let catalog: Value = serde_json::from_str(&profile.codex_catalog_json).unwrap();
        assert_eq!(catalog[0]["displayName"], "DeepSeek V4 Flash");
        assert_eq!(catalog[0]["model"], "deepseek-v4-flash");
        assert_eq!(catalog[0]["contextWindow"], 128000);
        assert_eq!(catalog[1]["displayName"], "Qwen 3 Coder");
        assert_eq!(catalog[1]["model"], "qwen3-coder");
        assert_eq!(catalog[1]["contextWindow"], "200000");
    }

    #[test]
    fn ccswitch_import_reads_claude_env_provider() {
        let settings = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://claude-relay.example",
                "ANTHROPIC_AUTH_TOKEN": "sk-claude-test",
                "ANTHROPIC_MODEL": "claude-fable-5",
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-5[1M]",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-5",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "claude-haiku-5",
                "ANTHROPIC_DEFAULT_FABLE_MODEL": "claude-fable-5"
            }
        });

        let profile = ccswitch_claude_profile_from_settings(
            "Claude Provider",
            "Claude Provider",
            "claude",
            &settings.to_string(),
            "{}",
        )
        .expect("profile imported from claude env");

        assert_eq!(profile.base_url, "https://claude-relay.example");
        assert_eq!(profile.api_key, "sk-claude-test");
        assert_eq!(profile.target_app, "claude");
        assert_eq!(profile.api_format, "Anthropic Messages");
        assert_eq!(profile.claude_desktop_mode, "proxy");
        assert!(profile.route_enabled);
        assert!(profile.model_mapping_enabled);
        assert!(
            profile
                .model_mapping
                .contains("Sonnet (claude-sonnet-4-6): claude-sonnet-5 -> claude-sonnet-5 [1M]")
        );
        assert!(
            profile
                .model_mapping
                .contains("Opus (claude-opus-4-8): claude-opus-5 -> claude-opus-5")
        );
        assert!(profile.model_mapping_json.contains("\"role\": \"sonnet\""));
        assert!(
            profile
                .model_mapping_json
                .contains("\"routeId\": \"claude-sonnet-4-6\"")
        );
        assert!(
            profile
                .model_mapping_json
                .contains("\"requestModel\": \"claude-sonnet-5\"")
        );
        assert!(profile.model_mapping_json.contains("\"supports1m\": true"));
        assert!(profile.model_list.contains("claude-fable-5"));
    }

    #[test]
    fn ccswitch_import_preserves_claude_desktop_proxy_routes_from_meta() {
        let settings = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://openai-compatible.example/v1",
                "ANTHROPIC_AUTH_TOKEN": "sk-claude-proxy",
                "ANTHROPIC_MODEL": "gpt-5.5"
            }
        });
        let meta = json!({
            "apiFormat": "openai_responses",
            "claudeDesktopMode": "proxy",
            "claudeDesktopModelRoutes": {
                "claude-sonnet-4-6": {
                    "model": "gpt-5.5[1M]",
                    "labelOverride": "GPT 5.5 Sonnet",
                    "supports1m": true
                },
                "claude-opus-4-8": {
                    "model": "gpt-5.5-pro",
                    "labelOverride": "GPT 5.5 Pro"
                },
                "claude-subagent": {
                    "model": "gpt-5.4-mini[1M]",
                    "labelOverride": "GPT 5.4 Mini",
                    "supports1m": true
                }
            }
        });

        let profile = ccswitch_claude_profile_from_settings(
            "Claude Desktop Provider",
            "Claude Desktop Provider",
            "claude-desktop",
            &settings.to_string(),
            &meta.to_string(),
        )
        .expect("profile imported from claude desktop proxy");

        assert_eq!(profile.target_app, "claude-desktop");
        assert_eq!(profile.api_format, "OpenAI Responses API");
        assert_eq!(profile.claude_desktop_mode, "proxy");
        assert!(profile.route_enabled);
        assert!(profile.model_mapping_enabled);
        assert!(
            profile
                .model_mapping_json
                .contains("\"routeId\": \"claude-sonnet-4-6\"")
        );
        assert!(
            profile
                .model_mapping_json
                .contains("\"displayName\": \"GPT 5.5 Sonnet\"")
        );
        assert!(
            profile
                .model_mapping_json
                .contains("\"requestModel\": \"gpt-5.5\"")
        );
        assert!(
            profile
                .model_mapping_json
                .contains("\"routeId\": \"claude-opus-4-8\"")
        );
        assert!(
            profile
                .model_mapping_json
                .contains("\"routeId\": \"claude-subagent\"")
        );
        assert!(
            profile
                .model_mapping_json
                .contains("\"requestModel\": \"gpt-5.4-mini\"")
        );
        assert!(profile.model_list.contains("gpt-5.5"));
        assert!(profile.model_list.contains("gpt-5.5-pro"));
        assert!(profile.model_list.contains("gpt-5.4-mini"));
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
        let result = require_expected_update_version(None).unwrap_err();

        assert!(result.message.contains("请先检查更新"));
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
        let result = tauri::async_runtime::block_on(preview_claude_desktop_provider(
            ClaudeDesktopProviderRequest {
                name: "TopoReduce".to_string(),
                base_url: "https://api.toporeduce.cn".to_string(),
                api_key: "sk-manager-secret".to_string(),
                model_list:
                    claude_codex_pro_core::protocol_proxy::claude_desktop_default_model_list(),
            },
        ));

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
        let result = tauri::async_runtime::block_on(read_latest_logs(LogRequest { lines: 25 }));

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
        let _guard = test_path_lock();
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path().join("codex-home");
        let _codex_home = set_test_codex_home(&codex_home);
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

        let result = delete_local_session_blocking_with_backup_store(
            DeleteLocalSessionRequest {
                session_id: "t1".to_string(),
                title: "Active Thread".to_string(),
                db_path: Some(stale_db.to_string_lossy().to_string()),
            },
            claude_codex_pro_data::BackupStore::new(temp.path().join("backups")),
        );

        assert_eq!(result.status, "ok", "{}", result.message);
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
        let _guard = test_path_lock();
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path().join("codex-home");
        let _codex_home = set_test_codex_home(&codex_home);
        let sqlite_dir = codex_home.join("sqlite");
        std::fs::create_dir_all(&sqlite_dir).unwrap();
        let current_db = sqlite_dir.join("state_5.sqlite");
        let legacy_db = codex_home.join("state_5.sqlite");
        create_minimal_thread_db(&current_db, "t1", "Current Copy", 100);
        create_minimal_thread_db(&legacy_db, "t1", "Legacy Copy", 200);

        let result = tauri::async_runtime::block_on(list_local_sessions());

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
        let _guard = test_path_lock();
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path().join("codex-home");
        let _codex_home = set_test_codex_home(&codex_home);
        let sqlite_dir = codex_home.join("sqlite");
        std::fs::create_dir_all(&sqlite_dir).unwrap();
        let current_db = sqlite_dir.join("state_5.sqlite");
        let legacy_db = codex_home.join("state_5.sqlite");
        create_minimal_thread_db(&current_db, "t1", "Current Copy", 100);
        create_minimal_thread_db(&legacy_db, "t1", "Legacy Copy", 200);

        let result = delete_local_session_blocking_with_backup_store(
            DeleteLocalSessionRequest {
                session_id: "t1".to_string(),
                title: "Legacy Copy".to_string(),
                db_path: Some(legacy_db.to_string_lossy().to_string()),
            },
            claude_codex_pro_data::BackupStore::new(temp.path().join("backups")),
        );

        assert_eq!(result.status, "ok", "{}", result.message);
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
        let _codex_home = set_test_codex_home(&temp.path().join("codex-home"));
        let settings_path = temp.path().join("settings.json");
        let memory_path = temp.path().join("memory.sqlite");
        let isolated_memory_path = memory_path.clone();
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

        let learned = tauri::async_runtime::block_on(learn_memory_assist_item(MemoryItemRequest {
            text: "should not persist".to_string(),
            workspace: "repo-a".to_string(),
            category: "manual".to_string(),
            tags: Vec::new(),
            source: "manager".to_string(),
            source_session_id: String::new(),
        }));
        let candidate = tauri::async_runtime::block_on(create_memory_assist_candidate(
            MemoryCandidateRequest {
                text: "should not become candidate".to_string(),
                workspace: "repo-a".to_string(),
                category: "preference".to_string(),
                tags: Vec::new(),
                source: "manager".to_string(),
                reason: "test".to_string(),
                source_session_id: String::new(),
            },
        ));
        let status =
            claude_codex_pro_core::memory_assist::MemoryAssistStore::new(isolated_memory_path)
                .status_from_codex_home(&temp.path().join("codex-home"))
                .unwrap();

        claude_codex_pro_core::memory_assist::set_memory_assist_db_path_for_tests(previous_memory);
        claude_codex_pro_core::paths::set_settings_path_for_tests(previous_settings);

        assert_eq!(learned.status, "failed");
        assert!(!learned.message.is_empty());
        assert_eq!(candidate.status, "failed");
        assert!(!candidate.message.is_empty());
        assert_eq!(status.total_items, 0);
        assert_eq!(status.pending_candidates, 0);
    }

    #[test]
    fn memory_assist_candidate_command_respects_auto_suggest_disabled() {
        let _guard = test_path_lock();
        let temp = tempfile::tempdir().unwrap();
        let _codex_home = set_test_codex_home(&temp.path().join("codex-home"));
        let settings_path = temp.path().join("settings.json");
        let memory_path = temp.path().join("memory.sqlite");
        let isolated_memory_path = memory_path.clone();
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

        let learned = tauri::async_runtime::block_on(learn_memory_assist_item(MemoryItemRequest {
            text: "manual memory still works".to_string(),
            workspace: "repo-a".to_string(),
            category: "manual".to_string(),
            tags: Vec::new(),
            source: "manager".to_string(),
            source_session_id: String::new(),
        }));
        let candidate = tauri::async_runtime::block_on(create_memory_assist_candidate(
            MemoryCandidateRequest {
                text: "auto suggestion should not persist".to_string(),
                workspace: "repo-a".to_string(),
                category: "preference".to_string(),
                tags: Vec::new(),
                source: "manager".to_string(),
                reason: "test".to_string(),
                source_session_id: String::new(),
            },
        ));
        let status =
            claude_codex_pro_core::memory_assist::MemoryAssistStore::new(isolated_memory_path)
                .status_from_codex_home(&temp.path().join("codex-home"))
                .unwrap();

        claude_codex_pro_core::memory_assist::set_memory_assist_db_path_for_tests(previous_memory);
        claude_codex_pro_core::paths::set_settings_path_for_tests(previous_settings);

        assert_eq!(learned.status, "ok");
        assert_eq!(candidate.status, "failed");
        assert!(!candidate.message.is_empty());
        assert_eq!(status.total_items, 1);
        assert_eq!(status.pending_candidates, 0);
    }

    #[test]
    fn memory_runtime_idle_status_is_treated_as_available() {
        let runtime = MemoryAssistRuntimeSnapshot {
            enabled: true,
            injected: true,
            status: "idle".to_string(),
            workspace: "codex:path:test".to_string(),
            summary: "waiting".to_string(),
            ..MemoryAssistRuntimeSnapshot::default()
        };

        assert_eq!(normalize_memory_runtime_status(&runtime), "ok");
    }

    #[test]
    fn repair_debug_port_keeps_preferred_port_when_bindable() {
        let selected = select_repair_debug_port_with(9230, |port| port == 9230, || 9311);

        assert_eq!(selected, 9230);
    }

    #[test]
    fn repair_debug_port_uses_available_fallback_when_preferred_is_busy() {
        let selected = select_repair_debug_port_with(9230, |_| false, || 9311);

        assert_eq!(selected, 9311);
    }

    #[test]
    fn repair_launch_status_rejects_stale_status_for_another_port() {
        let request = LaunchRequest {
            app_path: "codex.exe".to_string(),
            debug_port: 9311,
            helper_port: 46227,
        };
        let stale = LaunchStatus {
            status: "ok".to_string(),
            message: "stale".to_string(),
            started_at_ms: 111,
            codex_app: None,
            debug_port: Some(9230),
            helper_port: Some(46227),
            debug_port_online: true,
            helper_port_online: true,
            frontend_runtime_online: false,
            frontend_runtime_seen_at_ms: None,
        };

        assert!(repair_launch_status(&request, Some(stale), false, false, 222).is_none());
    }

    #[test]
    fn repair_launch_status_accepts_requested_port_while_status_file_is_stale() {
        let request = LaunchRequest {
            app_path: "codex.exe".to_string(),
            debug_port: 9311,
            helper_port: 46227,
        };
        let stale = LaunchStatus {
            status: "ok".to_string(),
            message: "stale".to_string(),
            started_at_ms: 111,
            codex_app: None,
            debug_port: Some(9230),
            helper_port: Some(46227),
            debug_port_online: true,
            helper_port_online: true,
            frontend_runtime_online: false,
            frontend_runtime_seen_at_ms: None,
        };

        let detected = repair_launch_status(&request, Some(stale), true, true, 222)
            .expect("requested repair port should win over stale status");

        assert_eq!(detected.debug_port, Some(9311));
        assert_eq!(detected.helper_port, Some(46227));
        assert_eq!(detected.started_at_ms, 222);
        assert!(detected.debug_port_online);
        assert!(detected.helper_port_online);
    }

    #[test]
    fn renderer_heartbeat_rejects_previous_launch_generation() {
        let now = current_time_ms();

        assert!(!renderer_heartbeat_is_current(
            now.saturating_sub(1),
            Some(now)
        ));
        assert!(!renderer_heartbeat_is_current(now, None));
    }

    #[test]
    fn renderer_heartbeat_accepts_current_launch_generation() {
        let now = current_time_ms();

        assert!(renderer_heartbeat_is_current(now, Some(now)));
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
        assert!(result.message.contains("只能打开 http 或 https"));
    }
}
