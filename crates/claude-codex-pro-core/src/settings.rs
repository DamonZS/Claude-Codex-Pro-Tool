use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};

use anyhow::Context;
use fs2::FileExt;
use serde::Deserialize;
use serde_json::{Map, Value};
use toml_edit::{DocumentMut, Item};

use crate::zed_remote::ZedOpenStrategy;

static SETTINGS_WRITE_TURN: Mutex<u64> = Mutex::new(0);
static SETTINGS_WRITE_READY: Condvar = Condvar::new();
static NEXT_SETTINGS_WRITE_TICKET: AtomicU64 = AtomicU64::new(0);
static NEXT_ATOMIC_WRITE_ID: AtomicU64 = AtomicU64::new(0);

struct SettingsWriteGuard {
    turn: MutexGuard<'static, u64>,
}

impl Drop for SettingsWriteGuard {
    fn drop(&mut self) {
        *self.turn += 1;
        SETTINGS_WRITE_READY.notify_all();
    }
}

fn lock_settings_writes() -> anyhow::Result<SettingsWriteGuard> {
    let ticket = NEXT_SETTINGS_WRITE_TICKET.fetch_add(1, Ordering::Relaxed);
    let mut turn = SETTINGS_WRITE_TURN
        .lock()
        .map_err(|_| anyhow::anyhow!("settings write lock is poisoned"))?;
    while *turn != ticket {
        turn = SETTINGS_WRITE_READY
            .wait(turn)
            .map_err(|_| anyhow::anyhow!("settings write lock is poisoned"))?;
    }
    Ok(SettingsWriteGuard { turn })
}

struct SettingsFileLock {
    file: File,
}

impl SettingsFileLock {
    fn acquire(settings_path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = settings_path.parent() {
            create_private_dir_all(parent)?;
        }
        let lock_path = settings_lock_path(settings_path);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("failed to open settings lock {}", lock_path.display()))?;
        file.lock_exclusive()
            .with_context(|| format!("failed to lock settings {}", settings_path.display()))?;
        Ok(Self { file })
    }
}

impl Drop for SettingsFileLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LaunchMode {
    #[default]
    Patch,
    Relay,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayContextSelection {
    #[serde(default)]
    pub mcp_servers: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub plugins: Vec<String>,
}

impl Default for RelayContextSelection {
    fn default() -> Self {
        Self {
            mcp_servers: Vec::new(),
            skills: Vec::new(),
            plugins: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayProfile {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing)]
    pub model: String,
    #[serde(default = "default_relay_base_url", skip_serializing)]
    pub base_url: String,
    #[serde(rename = "upstreamBaseUrl", default)]
    pub upstream_base_url: String,
    #[serde(
        default,
        skip_serializing,
        deserialize_with = "deserialize_profile_api_key"
    )]
    pub api_key: String,
    #[serde(rename = "apiKeyExplicit", default, skip_serializing)]
    pub api_key_explicit: bool,
    #[serde(default)]
    pub protocol: RelayProtocol,
    #[serde(rename = "relayMode", default)]
    pub relay_mode: RelayMode,
    #[serde(rename = "officialMixApiKey", default)]
    pub official_mix_api_key: bool,
    #[serde(rename = "testModel", default)]
    pub test_model: String,
    #[serde(rename = "configContents", default)]
    pub config_contents: String,
    #[serde(rename = "authContents", default)]
    pub auth_contents: String,
    #[serde(rename = "useCommonConfig", default = "default_true")]
    pub use_common_config: bool,
    #[serde(rename = "contextSelection", default)]
    pub context_selection: RelayContextSelection,
    #[serde(rename = "contextSelectionInitialized", default)]
    pub context_selection_initialized: bool,
    #[serde(rename = "contextWindow", default)]
    pub context_window: String,
    #[serde(rename = "autoCompactLimit", default)]
    pub auto_compact_limit: String,
    #[serde(rename = "modelInsertMode", default)]
    pub model_insert_mode: RelayModelInsertMode,
    #[serde(rename = "modelList", default)]
    pub model_list: String,
    #[serde(
        rename = "codexCatalogJson",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub codex_catalog_json: String,
    #[serde(
        rename = "userAgent",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub user_agent: String,
    #[serde(rename = "notes", default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    #[serde(
        rename = "websiteUrl",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub website_url: String,
    #[serde(
        rename = "authField",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub auth_field: String,
    #[serde(
        rename = "headerOverride",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub header_override: String,
    #[serde(
        rename = "bodyOverride",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub body_override: String,
    #[serde(rename = "hideAiSignature", default, skip_serializing_if = "is_false")]
    pub hide_ai_signature: bool,
    #[serde(rename = "teammatesMode", default, skip_serializing_if = "is_false")]
    pub teammates_mode: bool,
    #[serde(
        rename = "toolSearchEnabled",
        default,
        skip_serializing_if = "is_false"
    )]
    pub tool_search_enabled: bool,
    #[serde(
        rename = "maxThinkingEnabled",
        default,
        skip_serializing_if = "is_false"
    )]
    pub max_thinking_enabled: bool,
    #[serde(
        rename = "disableAutoUpdate",
        default,
        skip_serializing_if = "is_false"
    )]
    pub disable_auto_update: bool,
    #[serde(
        rename = "importSource",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub import_source: String,
    #[serde(
        rename = "targetApp",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub target_app: String,
    #[serde(
        rename = "apiFormat",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub api_format: String,
    #[serde(
        rename = "claudeDesktopMode",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub claude_desktop_mode: String,
    #[serde(rename = "routeEnabled", default, skip_serializing_if = "is_false")]
    pub route_enabled: bool,
    #[serde(
        rename = "routeMode",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub route_mode: String,
    #[serde(
        rename = "modelMapping",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub model_mapping: String,
    #[serde(
        rename = "modelMappingEnabled",
        default,
        skip_serializing_if = "is_false"
    )]
    pub model_mapping_enabled: bool,
    #[serde(
        rename = "modelMappingJson",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub model_mapping_json: String,
    #[serde(rename = "aggregateEnabled", default, skip_serializing_if = "is_false")]
    pub aggregate_enabled: bool,
    #[serde(
        rename = "aggregateStrategy",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub aggregate_strategy: String,
    #[serde(
        rename = "aggregateMembers",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub aggregate_members: Vec<String>,
}

impl Default for RelayProfile {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "默认中转".to_string(),
            model: String::new(),
            base_url: default_relay_base_url(),
            upstream_base_url: String::new(),
            api_key: String::new(),
            api_key_explicit: false,
            protocol: RelayProtocol::Responses,
            relay_mode: RelayMode::Official,
            official_mix_api_key: false,
            test_model: String::new(),
            config_contents: String::new(),
            auth_contents: String::new(),
            use_common_config: true,
            context_selection: RelayContextSelection::default(),
            context_selection_initialized: false,
            context_window: String::new(),
            auto_compact_limit: String::new(),
            model_insert_mode: RelayModelInsertMode::Patch,
            model_list: String::new(),
            codex_catalog_json: String::new(),
            user_agent: String::new(),
            notes: String::new(),
            website_url: String::new(),
            auth_field: String::new(),
            header_override: String::new(),
            body_override: String::new(),
            hide_ai_signature: false,
            teammates_mode: false,
            tool_search_enabled: false,
            max_thinking_enabled: false,
            disable_auto_update: false,
            import_source: String::new(),
            target_app: String::new(),
            api_format: String::new(),
            claude_desktop_mode: String::new(),
            route_enabled: false,
            route_mode: String::new(),
            model_mapping: String::new(),
            model_mapping_enabled: false,
            model_mapping_json: String::new(),
            aggregate_enabled: false,
            aggregate_strategy: String::new(),
            aggregate_members: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum RelayModelInsertMode {
    ModelCatalog,
    #[default]
    Patch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum RelayProtocol {
    #[default]
    Responses,
    ChatCompletions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum RelayMode {
    Official,
    #[default]
    MixedApi,
    PureApi,
}

/// A user-defined Codex plugin marketplace. Until this existed the tool only
/// knew about three hard-coded built-in repositories and had no write channel
/// for user repos at all, so "add a third-party marketplace" was impossible.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexCustomMarketplace {
    /// Marketplace name/key written under `[marketplaces.<name>]` in config.toml.
    pub name: String,
    /// Either "git" or "local".
    #[serde(rename = "sourceType", default = "default_marketplace_source_type")]
    pub source_type: String,
    /// Git URL (for git) or filesystem path (for local).
    pub source: String,
    /// Git ref (branch/tag/commit). Only meaningful for git sources.
    #[serde(rename = "ref", default)]
    pub git_ref: String,
    /// Optional sparse-checkout paths for git sources.
    #[serde(rename = "sparsePaths", default)]
    pub sparse_paths: Vec<String>,
}

fn default_marketplace_source_type() -> String {
    "git".to_string()
}

/// One recorded Codex → Claude Code migration, used to keep re-runs idempotent:
/// if the recorded target file still exists we reuse it instead of writing a
/// duplicate transcript.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionMigrationRecord {
    /// Codex thread id that was migrated.
    pub session_id: String,
    /// UUID of the Claude Code transcript file that was written.
    pub target_uuid: String,
    /// Claude Code project slug the transcript landed in.
    #[serde(default)]
    pub project_slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BackendSettings {
    #[serde(rename = "codexAppPath", default)]
    pub codex_app_path: String,
    #[serde(rename = "codexExtraArgs", default)]
    pub codex_extra_args: Vec<String>,
    #[serde(rename = "providerSyncEnabled", default)]
    pub provider_sync_enabled: bool,
    #[serde(rename = "providerSyncSavedProviders", default)]
    pub provider_sync_saved_providers: Vec<String>,
    #[serde(rename = "providerSyncManualProviders", default)]
    pub provider_sync_manual_providers: Vec<String>,
    #[serde(rename = "providerSyncLastSelectedProvider", default)]
    pub provider_sync_last_selected_provider: String,
    #[serde(rename = "relayProfilesEnabled", default = "default_true")]
    pub relay_profiles_enabled: bool,
    #[serde(rename = "enhancementsEnabled", default = "default_true")]
    pub enhancements_enabled: bool,
    #[serde(rename = "computerUseGuardEnabled", default)]
    pub computer_use_guard_enabled: bool,
    #[serde(rename = "codexAppPluginEntryUnlock", default = "default_true")]
    pub codex_app_plugin_entry_unlock: bool,
    #[serde(rename = "codexAppPluginMarketplaceUnlock", default = "default_true")]
    pub codex_app_plugin_marketplace_unlock: bool,
    #[serde(rename = "codexAppForcePluginInstall", default = "default_true")]
    pub codex_app_force_plugin_install: bool,
    #[serde(rename = "codexAppModelWhitelistUnlock", default = "default_true")]
    pub codex_app_model_whitelist_unlock: bool,
    #[serde(rename = "codexAppSessionDelete", default = "default_true")]
    pub codex_app_session_delete: bool,
    #[serde(rename = "codexAppMarkdownExport", default = "default_true")]
    pub codex_app_markdown_export: bool,
    #[serde(rename = "codexAppProjectMove", default = "default_true")]
    pub codex_app_project_move: bool,
    #[serde(rename = "codexAppConversationTimeline", default = "default_true")]
    pub codex_app_conversation_timeline: bool,
    #[serde(rename = "codexAppConversationView", default)]
    pub codex_app_conversation_view: bool,
    #[serde(rename = "codexAppThreadScrollRestore", default = "default_true")]
    pub codex_app_thread_scroll_restore: bool,
    #[serde(rename = "codexAppZedRemoteOpen", default = "default_true")]
    pub codex_app_zed_remote_open: bool,
    #[serde(rename = "zedRemoteOpenStrategy", default)]
    pub zed_remote_open_strategy: ZedOpenStrategy,
    #[serde(rename = "zedRemoteProjectRegistryEnabled", default = "default_true")]
    pub zed_remote_project_registry_enabled: bool,
    #[serde(rename = "zedRemoteSyncToZedSettings", default)]
    pub zed_remote_sync_to_zed_settings: bool,
    #[serde(rename = "codexAppUpstreamWorktreeCreate", default = "default_true")]
    pub codex_app_upstream_worktree_create: bool,
    #[serde(rename = "codexAppNativeMenuPlacement", default = "default_true")]
    pub codex_app_native_menu_placement: bool,
    #[serde(rename = "claudeAppChineseOverlayEnabled", default)]
    pub claude_app_chinese_overlay_enabled: bool,
    #[serde(rename = "codexAppServiceTierControls", default)]
    pub codex_app_service_tier_controls: bool,
    #[serde(rename = "codexAppImageOverlayEnabled", default)]
    pub codex_app_image_overlay_enabled: bool,
    #[serde(rename = "codexAppImageOverlayPath", default)]
    pub codex_app_image_overlay_path: String,
    #[serde(
        rename = "codexAppImageOverlayOpacity",
        default = "default_image_overlay_opacity",
        deserialize_with = "deserialize_image_overlay_opacity"
    )]
    pub codex_app_image_overlay_opacity: u8,
    #[serde(rename = "codexGoalsEnabled", default)]
    pub codex_goals_enabled: bool,
    #[serde(rename = "memoryAssistEnabled", default = "default_true")]
    pub memory_assist_enabled: bool,
    #[serde(rename = "memoryAssistInjectEnabled", default = "default_true")]
    pub memory_assist_inject_enabled: bool,
    #[serde(rename = "memoryAssistAutoSuggestEnabled", default = "default_true")]
    pub memory_assist_auto_suggest_enabled: bool,
    /// Phase 3 module C: opt-in gate for sending memory text to the active relay
    /// profile to generate a consolidation summary. Defaults to false because it
    /// transmits local memory content (which may include project detail) to an
    /// external endpoint; when off, consolidation uses the local rule-based
    /// summarizer only.
    #[serde(rename = "memoryAssistLlmSummaryEnabled", default)]
    pub memory_assist_llm_summary_enabled: bool,
    /// Phase 4 module D: opt-in gate for the MCP server that exposes Pangu memory
    /// to external agents (Claude Code / Cursor / Codex CLI) over stdio. Defaults
    /// to false because it makes local memory readable/writable by any agent that
    /// spawns the server; when off, the MCP server refuses to serve. Individual
    /// tools additionally re-check `memoryAssistEnabled` at call time. See ADR 0002.
    #[serde(rename = "memoryAssistMcpEnabled", default)]
    pub memory_assist_mcp_enabled: bool,
    #[serde(
        rename = "memoryAssistMaxInjectedItems",
        default = "default_memory_assist_max_injected_items",
        deserialize_with = "deserialize_memory_assist_max_injected_items"
    )]
    pub memory_assist_max_injected_items: u8,
    #[serde(
        rename = "memoryAssistWorkspaceMode",
        default = "default_memory_assist_workspace_mode"
    )]
    pub memory_assist_workspace_mode: String,
    #[serde(rename = "memoryAssistDataDir", default)]
    pub memory_assist_data_dir: String,
    #[serde(rename = "launchMode", default)]
    pub launch_mode: LaunchMode,
    #[serde(rename = "relayBaseUrl", default = "default_relay_base_url")]
    pub relay_base_url: String,
    #[serde(rename = "relayApiKey", default)]
    pub relay_api_key: String,
    #[serde(rename = "relayProfiles", default = "default_relay_profiles")]
    pub relay_profiles: Vec<RelayProfile>,
    #[serde(rename = "relayCommonConfigContents", default)]
    pub relay_common_config_contents: String,
    #[serde(rename = "relayContextConfigContents", default)]
    pub relay_context_config_contents: String,
    #[serde(rename = "activeRelayId", default = "default_active_relay_id")]
    pub active_relay_id: String,
    #[serde(rename = "activeClaudeRelayId", default)]
    pub active_claude_relay_id: String,
    #[serde(rename = "activeClaudeDesktopRelayId", default)]
    pub active_claude_desktop_relay_id: String,
    #[serde(rename = "relayTestModel", default = "default_relay_test_model")]
    pub relay_test_model: String,
    #[serde(rename = "cliWrapperEnabled", default)]
    pub cli_wrapper_enabled: bool,
    #[serde(rename = "cliWrapperBaseUrl", default)]
    pub cli_wrapper_base_url: String,
    #[serde(rename = "cliWrapperApiKey", default)]
    pub cli_wrapper_api_key: String,
    #[serde(
        rename = "cliWrapperApiKeyEnv",
        default = "default_api_key_env",
        deserialize_with = "empty_as_default_api_key_env"
    )]
    pub cli_wrapper_api_key_env: String,
    #[serde(rename = "codexCustomMarketplaces", default)]
    pub codex_custom_marketplaces: Vec<CodexCustomMarketplace>,
    #[serde(rename = "codexSessionMigrations", default)]
    pub codex_session_migrations: Vec<CodexSessionMigrationRecord>,
}

impl Default for BackendSettings {
    fn default() -> Self {
        Self {
            codex_app_path: String::new(),
            codex_extra_args: Vec::new(),
            provider_sync_enabled: false,
            provider_sync_saved_providers: Vec::new(),
            provider_sync_manual_providers: Vec::new(),
            provider_sync_last_selected_provider: String::new(),
            relay_profiles_enabled: true,
            enhancements_enabled: true,
            computer_use_guard_enabled: false,
            codex_app_plugin_entry_unlock: true,
            codex_app_plugin_marketplace_unlock: true,
            codex_app_force_plugin_install: true,
            codex_app_model_whitelist_unlock: true,
            codex_app_session_delete: true,
            codex_app_markdown_export: true,
            codex_app_project_move: true,
            codex_app_conversation_timeline: true,
            codex_app_conversation_view: false,
            codex_app_thread_scroll_restore: true,
            codex_app_zed_remote_open: true,
            zed_remote_open_strategy: ZedOpenStrategy::AddToFocusedWorkspace,
            zed_remote_project_registry_enabled: true,
            zed_remote_sync_to_zed_settings: false,
            codex_app_upstream_worktree_create: true,
            codex_app_native_menu_placement: true,
            claude_app_chinese_overlay_enabled: false,
            codex_app_service_tier_controls: false,
            codex_app_image_overlay_enabled: false,
            codex_app_image_overlay_path: String::new(),
            codex_app_image_overlay_opacity: default_image_overlay_opacity(),
            codex_goals_enabled: false,
            memory_assist_enabled: true,
            memory_assist_inject_enabled: true,
            memory_assist_auto_suggest_enabled: true,
            memory_assist_max_injected_items: default_memory_assist_max_injected_items(),
            memory_assist_workspace_mode: default_memory_assist_workspace_mode(),
            memory_assist_data_dir: String::new(),
            memory_assist_llm_summary_enabled: false,
            memory_assist_mcp_enabled: false,
            launch_mode: LaunchMode::Patch,
            relay_base_url: default_relay_base_url(),
            relay_api_key: String::new(),
            relay_profiles: default_relay_profiles(),
            relay_common_config_contents: String::new(),
            relay_context_config_contents: String::new(),
            active_relay_id: default_active_relay_id(),
            active_claude_relay_id: String::new(),
            active_claude_desktop_relay_id: String::new(),
            relay_test_model: default_relay_test_model(),
            cli_wrapper_enabled: false,
            cli_wrapper_base_url: String::new(),
            cli_wrapper_api_key: String::new(),
            cli_wrapper_api_key_env: default_api_key_env(),
            codex_custom_marketplaces: Vec::new(),
            codex_session_migrations: Vec::new(),
        }
    }
}

impl BackendSettings {
    pub fn active_relay_id_for_target(&self, target_app: &str) -> &str {
        match normalized_target_app(target_app) {
            "claude" => self.active_claude_relay_id.as_str(),
            "claude-desktop" => self.active_claude_desktop_relay_id.as_str(),
            _ => self.active_relay_id.as_str(),
        }
    }

    pub fn active_relay_profile_for_target(&self, target_app: &str) -> RelayProfile {
        let target_app = normalized_target_app(target_app);
        if target_app == "codex" {
            return self.active_relay_profile();
        }

        let active_id = self.active_relay_id_for_target(target_app);
        let selected = if active_id.trim().is_empty() {
            let mut candidates = self
                .relay_profiles
                .iter()
                .filter(|profile| relay_profile_matches_target(profile, target_app));
            let only_candidate = candidates.next();
            if candidates.next().is_none() {
                only_candidate
            } else {
                None
            }
        } else {
            self.relay_profiles.iter().find(|profile| {
                profile.id == active_id && relay_profile_matches_target(profile, target_app)
            })
        };

        selected.cloned().unwrap_or_else(|| RelayProfile {
            id: active_id.to_string(),
            name: "未配置供应商".to_string(),
            target_app: target_app.to_string(),
            ..RelayProfile::default()
        })
    }

    pub fn active_relay_profile(&self) -> RelayProfile {
        if self.active_relay_id == default_active_relay_id()
            && self.relay_profiles.len() == 1
            && self.relay_profiles[0] == RelayProfile::default()
            && (!self.relay_api_key.is_empty() || self.relay_base_url != default_relay_base_url())
        {
            return RelayProfile {
                id: default_active_relay_id(),
                name: "默认中转".to_string(),
                model: String::new(),
                base_url: if self.relay_base_url.is_empty() {
                    default_relay_base_url()
                } else {
                    self.relay_base_url.clone()
                },
                upstream_base_url: if self.relay_base_url.is_empty() {
                    default_relay_base_url()
                } else {
                    self.relay_base_url.clone()
                },
                api_key: self.relay_api_key.clone(),
                protocol: RelayProtocol::Responses,
                relay_mode: RelayMode::MixedApi,
                official_mix_api_key: true,
                test_model: String::new(),
                config_contents: String::new(),
                auth_contents: String::new(),
                use_common_config: true,
                context_selection: RelayContextSelection::default(),
                context_selection_initialized: false,
                context_window: String::new(),
                auto_compact_limit: String::new(),
                model_insert_mode: RelayModelInsertMode::Patch,
                model_list: String::new(),
                user_agent: String::new(),
                ..RelayProfile::default()
            };
        }

        if let Some(profile) = self
            .relay_profiles
            .iter()
            .find(|profile| profile.id == self.active_relay_id)
        {
            return profile.clone();
        }

        RelayProfile {
            id: if self.active_relay_id.is_empty() {
                default_active_relay_id()
            } else {
                self.active_relay_id.clone()
            },
            name: "默认中转".to_string(),
            model: String::new(),
            base_url: if self.relay_base_url.is_empty() {
                default_relay_base_url()
            } else {
                self.relay_base_url.clone()
            },
            upstream_base_url: if self.relay_base_url.is_empty() {
                default_relay_base_url()
            } else {
                self.relay_base_url.clone()
            },
            api_key: self.relay_api_key.clone(),
            protocol: RelayProtocol::Responses,
            relay_mode: RelayMode::Official,
            official_mix_api_key: false,
            test_model: String::new(),
            config_contents: String::new(),
            auth_contents: String::new(),
            use_common_config: true,
            context_selection: RelayContextSelection::default(),
            context_selection_initialized: false,
            context_window: String::new(),
            auto_compact_limit: String::new(),
            model_insert_mode: RelayModelInsertMode::Patch,
            model_list: String::new(),
            user_agent: String::new(),
            ..RelayProfile::default()
        }
    }
}

fn normalized_target_app(target_app: &str) -> &str {
    match target_app.trim().to_ascii_lowercase().as_str() {
        "claude" => "claude",
        "claude-desktop" | "claude_desktop" | "claudedesktop" => "claude-desktop",
        _ => "codex",
    }
}

fn relay_profile_matches_target(profile: &RelayProfile, target_app: &str) -> bool {
    let profile_target = profile.target_app.trim();
    if profile_target.is_empty() {
        target_app == "codex"
    } else {
        normalized_target_app(profile_target) == target_app
    }
}

pub fn relay_profile_resolved_api_key(profile: &RelayProfile) -> String {
    resolved_relay_profile_api_key(profile)
        .map(|resolved| resolved.value)
        .unwrap_or_default()
}

pub fn relay_profile_uses_anthropic_api_key(profile: &RelayProfile) -> bool {
    resolved_relay_profile_api_key(profile).is_some_and(|resolved| resolved.anthropic_api_key)
}

fn non_empty_setting(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

struct ResolvedRelayApiKey {
    value: String,
    anthropic_api_key: bool,
}

/// 判断显式填写的 Key 是否应走 Anthropic 官方 `x-api-key` 鉴权。
///
/// `authField` 显式声明时以它为准；未声明（第三方中转的常见情况）时，
/// 只有当上游指向 Anthropic 官方域名才用 `x-api-key`，否则一律按第三方
/// 中转的约定使用 `ANTHROPIC_AUTH_TOKEN` Bearer，避免中转 Key 被错误地
/// 塞进 `x-api-key` 头导致 401。
fn explicit_key_uses_anthropic_api_key(profile: &RelayProfile, key: &str) -> bool {
    let auth_field = profile.auth_field.trim();
    if !auth_field.is_empty() {
        return auth_field == "ANTHROPIC_API_KEY";
    }
    // 非 Claude 目标（如 Codex/OpenAI 兼容）永远不用 Anthropic 头。
    if !matches!(
        normalized_target_app(&profile.target_app),
        "claude" | "claude-desktop"
    ) {
        return false;
    }
    relay_targets_official_anthropic(profile) && !key.trim().starts_with("sk-ant-oat")
}

/// 上游地址是否指向 Anthropic 官方 API 域名。
fn relay_targets_official_anthropic(profile: &RelayProfile) -> bool {
    let candidate = if profile.upstream_base_url.trim().is_empty() {
        profile.base_url.trim()
    } else {
        profile.upstream_base_url.trim()
    };
    host_is_official_anthropic(candidate)
}

fn host_is_official_anthropic(base_url: &str) -> bool {
    let without_scheme = base_url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = without_scheme
        .split(['/', ':', '?', '#'])
        .next()
        .unwrap_or("")
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    host == "api.anthropic.com" || host.ends_with(".api.anthropic.com")
}

fn resolved_relay_profile_api_key(profile: &RelayProfile) -> Option<ResolvedRelayApiKey> {
    let explicit = non_empty_setting(&profile.api_key).map(|value| {
        let anthropic_api_key = explicit_key_uses_anthropic_api_key(profile, &value);
        ResolvedRelayApiKey {
            value,
            anthropic_api_key,
        }
    });
    if profile.api_key_explicit || explicit.is_some() {
        return explicit;
    }

    let config_key = || {
        api_key_from_json_text(&profile.config_contents)
            .or_else(|| api_key_from_toml_text(&profile.config_contents))
    };
    let auth_key = || api_key_from_json_text(&profile.auth_contents);
    if matches!(
        normalized_target_app(&profile.target_app),
        "claude" | "claude-desktop"
    ) {
        config_key().or_else(auth_key)
    } else {
        auth_key().or_else(config_key)
    }
}

fn api_key_from_json_text(contents: &str) -> Option<ResolvedRelayApiKey> {
    let value = serde_json::from_str::<Value>(contents.trim()).ok()?;
    api_key_from_json_value(&value)
}

fn api_key_from_json_value(value: &Value) -> Option<ResolvedRelayApiKey> {
    let object = value.as_object()?;
    for (key, anthropic_api_key) in [
        ("OPENAI_API_KEY", false),
        ("ANTHROPIC_AUTH_TOKEN", false),
        ("ANTHROPIC_API_KEY", true),
        ("api_key", false),
        ("apiKey", false),
    ] {
        if let Some(value) = object.get(key).and_then(Value::as_str) {
            if let Some(value) = non_empty_setting(value) {
                return Some(ResolvedRelayApiKey {
                    value,
                    anthropic_api_key,
                });
            }
        }
    }
    for container in ["env", "auth", "credentials"] {
        if let Some(value) = object.get(container).and_then(api_key_from_json_value) {
            return Some(value);
        }
    }
    None
}

fn api_key_from_toml_text(contents: &str) -> Option<ResolvedRelayApiKey> {
    let document = contents.parse::<DocumentMut>().ok()?;
    for (key, anthropic_api_key) in [
        ("experimental_bearer_token", false),
        ("OPENAI_API_KEY", false),
        ("ANTHROPIC_AUTH_TOKEN", false),
        ("ANTHROPIC_API_KEY", true),
        ("api_key", false),
        ("apiKey", false),
    ] {
        if let Some(value) = document.get(key).and_then(Item::as_str) {
            if let Some(value) = non_empty_setting(value) {
                return Some(ResolvedRelayApiKey {
                    value,
                    anthropic_api_key,
                });
            }
        }
    }
    None
}

pub fn default_api_key_env() -> String {
    "CUSTOM_OPENAI_API_KEY".to_string()
}

fn default_image_overlay_opacity() -> u8 {
    35
}

fn default_memory_assist_max_injected_items() -> u8 {
    5
}

pub fn default_memory_assist_workspace_mode() -> String {
    "project_plus_global".to_string()
}

fn clamp_image_overlay_opacity(value: u8) -> u8 {
    value.clamp(1, 100)
}

fn clamp_memory_assist_max_injected_items(value: u8) -> u8 {
    value.clamp(1, 20)
}

pub fn default_true() -> bool {
    true
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub fn default_relay_base_url() -> String {
    String::new()
}

pub fn default_active_relay_id() -> String {
    "default".to_string()
}

pub fn default_relay_test_model() -> String {
    "gpt-5.4-mini".to_string()
}

pub fn default_relay_profiles() -> Vec<RelayProfile> {
    vec![RelayProfile::default()]
}

pub fn empty_as_default_api_key_env<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value
        .filter(|value| !value.is_empty())
        .unwrap_or_else(default_api_key_env))
}

fn deserialize_image_overlay_opacity<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<u8>::deserialize(deserializer)?
        .map(clamp_image_overlay_opacity)
        .unwrap_or_else(default_image_overlay_opacity))
}

fn deserialize_memory_assist_max_injected_items<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<u8>::deserialize(deserializer)?
        .map(clamp_memory_assist_max_injected_items)
        .unwrap_or_else(default_memory_assist_max_injected_items))
}

fn deserialize_profile_api_key<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

pub fn normalize_codex_extra_args(args: &[String]) -> Vec<String> {
    args.iter()
        .map(|arg| arg.trim())
        .filter(|arg| !arg.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self::new(crate::paths::default_settings_path())
    }
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> anyhow::Result<BackendSettings> {
        let contents = match self.read_contents()? {
            Some(contents) => contents,
            None => return Ok(BackendSettings::default()),
        };

        Ok(normalize_settings_config_sections(
            serde_json::from_str(&contents).unwrap_or_default(),
        ))
    }

    pub fn load_strict(&self) -> anyhow::Result<BackendSettings> {
        let contents = match self.read_contents()? {
            Some(contents) => contents,
            None => return Ok(BackendSettings::default()),
        };

        let settings = serde_json::from_str(&contents)
            .with_context(|| format!("解析设置文件失败: {}", self.path.display()))?;
        Ok(normalize_settings_config_sections(settings))
    }

    fn read_contents(&self) -> anyhow::Result<Option<String>> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read settings {}", self.path.display()));
            }
        };
        Ok(Some(contents))
    }

    pub fn save(&self, settings: &BackendSettings) -> anyhow::Result<()> {
        let _write_guard = lock_settings_writes()?;
        let _file_lock = SettingsFileLock::acquire(&self.path)?;
        let mut settings = normalize_settings_config_sections(settings.clone());
        settings.codex_extra_args = normalize_codex_extra_args(&settings.codex_extra_args);
        let bytes = serde_json::to_vec_pretty(&settings)?;
        atomic_write(&self.path, &bytes)
    }

    pub fn update(&self, payload: Value) -> anyhow::Result<BackendSettings> {
        let Value::Object(payload) = payload else {
            return self.load();
        };

        // Keep the read/merge/write transaction intact so concurrent partial
        // updates cannot both start from the same stale snapshot.
        let _write_guard = lock_settings_writes()?;
        let _file_lock = SettingsFileLock::acquire(&self.path)?;
        let mut raw = self.load_raw_object()?;
        merge_known_setting_fields(&mut raw, &payload);
        let settings = normalize_settings_config_sections(
            serde_json::from_value(Value::Object(raw.clone())).unwrap_or_default(),
        );
        raw.insert(
            "relayCommonConfigContents".to_string(),
            Value::String(settings.relay_common_config_contents.clone()),
        );
        raw.insert(
            "relayContextConfigContents".to_string(),
            Value::String(settings.relay_context_config_contents.clone()),
        );
        let bytes = serde_json::to_vec_pretty(&Value::Object(raw))?;
        atomic_write(&self.path, &bytes)?;
        Ok(settings)
    }

    fn load_raw_object(&self) -> anyhow::Result<Map<String, Value>> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(settings_to_object(&BackendSettings::default()));
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read settings {}", self.path.display()));
            }
        };

        let value = serde_json::from_str::<Value>(&contents)
            .with_context(|| format!("failed to parse settings {}", self.path.display()))?;
        value.as_object().cloned().ok_or_else(|| {
            anyhow::anyhow!(
                "settings root must be a JSON object: {}",
                self.path.display()
            )
        })
    }
}

fn merge_known_setting_fields(target: &mut Map<String, Value>, source: &Map<String, Value>) {
    if let Some(value) = source.get("codexAppPath").and_then(Value::as_str) {
        target.insert("codexAppPath".to_string(), Value::String(value.to_string()));
    }
    if let Some(value) = source.get("codexExtraArgs").and_then(Value::as_array) {
        let args = value
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        target.insert(
            "codexExtraArgs".to_string(),
            Value::Array(
                normalize_codex_extra_args(&args)
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    if let Some(value) = source.get("providerSyncEnabled").and_then(Value::as_bool) {
        target.insert("providerSyncEnabled".to_string(), Value::Bool(value));
    }
    if let Some(value) = source.get("relayProfilesEnabled").and_then(Value::as_bool) {
        target.insert("relayProfilesEnabled".to_string(), Value::Bool(value));
    }
    if let Some(value) = source.get("enhancementsEnabled").and_then(Value::as_bool) {
        target.insert("enhancementsEnabled".to_string(), Value::Bool(value));
    }
    if let Some(value) = source
        .get("computerUseGuardEnabled")
        .and_then(Value::as_bool)
    {
        target.insert("computerUseGuardEnabled".to_string(), Value::Bool(value));
    }
    merge_bool_setting(target, source, "codexAppPluginEntryUnlock");
    merge_bool_setting(target, source, "codexAppPluginMarketplaceUnlock");
    merge_bool_setting(target, source, "codexAppForcePluginInstall");
    merge_bool_setting(target, source, "codexAppModelWhitelistUnlock");
    merge_bool_setting(target, source, "codexAppSessionDelete");
    merge_bool_setting(target, source, "codexAppMarkdownExport");
    merge_bool_setting(target, source, "codexAppProjectMove");
    merge_bool_setting(target, source, "codexAppConversationTimeline");
    merge_bool_setting(target, source, "codexAppConversationView");
    merge_bool_setting(target, source, "codexAppThreadScrollRestore");
    merge_bool_setting(target, source, "codexAppZedRemoteOpen");
    if let Some(value) = source.get("zedRemoteOpenStrategy") {
        if serde_json::from_value::<ZedOpenStrategy>(value.clone()).is_ok() {
            target.insert("zedRemoteOpenStrategy".to_string(), value.clone());
        }
    }
    merge_bool_setting(target, source, "zedRemoteProjectRegistryEnabled");
    merge_bool_setting(target, source, "zedRemoteSyncToZedSettings");
    merge_bool_setting(target, source, "codexAppUpstreamWorktreeCreate");
    merge_bool_setting(target, source, "codexAppNativeMenuPlacement");
    merge_bool_setting(target, source, "claudeAppChineseOverlayEnabled");
    merge_bool_setting(target, source, "codexAppServiceTierControls");
    merge_bool_setting(target, source, "codexAppImageOverlayEnabled");
    merge_bool_setting(target, source, "memoryAssistEnabled");
    merge_bool_setting(target, source, "memoryAssistInjectEnabled");
    merge_bool_setting(target, source, "memoryAssistAutoSuggestEnabled");
    merge_bool_setting(target, source, "memoryAssistLlmSummaryEnabled");
    if let Some(value) = source.get("memoryAssistDataDir").and_then(Value::as_str) {
        target.insert(
            "memoryAssistDataDir".to_string(),
            Value::String(value.trim().to_string()),
        );
    }
    merge_bool_setting(target, source, "memoryAssistMcpEnabled");
    if let Some(value) = source
        .get("codexAppImageOverlayPath")
        .and_then(Value::as_str)
    {
        target.insert(
            "codexAppImageOverlayPath".to_string(),
            Value::String(value.to_string()),
        );
    }
    if let Some(value) = source
        .get("codexAppImageOverlayOpacity")
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
    {
        target.insert(
            "codexAppImageOverlayOpacity".to_string(),
            Value::Number(serde_json::Number::from(clamp_image_overlay_opacity(value))),
        );
    }
    if let Some(value) = source.get("codexGoalsEnabled").and_then(Value::as_bool) {
        target.insert("codexGoalsEnabled".to_string(), Value::Bool(value));
    }
    if let Some(value) = source
        .get("memoryAssistMaxInjectedItems")
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
    {
        target.insert(
            "memoryAssistMaxInjectedItems".to_string(),
            Value::Number(serde_json::Number::from(
                clamp_memory_assist_max_injected_items(value),
            )),
        );
    }
    if let Some(value) = source
        .get("memoryAssistWorkspaceMode")
        .and_then(Value::as_str)
    {
        target.insert(
            "memoryAssistWorkspaceMode".to_string(),
            Value::String(if value.trim().is_empty() {
                default_memory_assist_workspace_mode()
            } else {
                value.trim().to_string()
            }),
        );
    }
    if let Some(value) = source.get("launchMode").and_then(Value::as_str) {
        if matches!(value, "patch" | "relay") {
            target.insert("launchMode".to_string(), Value::String(value.to_string()));
        }
    }
    if let Some(value) = source.get("relayBaseUrl").and_then(Value::as_str) {
        target.insert("relayBaseUrl".to_string(), Value::String(value.to_string()));
    }
    if let Some(value) = source.get("relayApiKey").and_then(Value::as_str) {
        target.insert("relayApiKey".to_string(), Value::String(value.to_string()));
    }
    if let Some(value) = source.get("relayProfiles").and_then(Value::as_array) {
        let mut profiles = serde_json::from_value::<Vec<RelayProfile>>(Value::Array(value.clone()))
            .unwrap_or_default();
        for profile in &mut profiles {
            if profile.relay_mode == RelayMode::PureApi
                || (profile.relay_mode == RelayMode::Official && profile.official_mix_api_key)
            {
                let _ = crate::relay_config::normalize_relay_profile_for_storage(profile);
                rewrite_profile_provider_id_to_match_profile_id(profile);
            }
        }
        preserve_official_mix_bearer_tokens(&mut profiles, target);
        target.insert(
            "relayProfiles".to_string(),
            serde_json::to_value(profiles).unwrap_or_else(|_| Value::Array(Vec::new())),
        );
    }
    if let Some(value) = source
        .get("relayCommonConfigContents")
        .and_then(Value::as_str)
    {
        target.insert(
            "relayCommonConfigContents".to_string(),
            Value::String(value.to_string()),
        );
    }
    if let Some(value) = source
        .get("relayContextConfigContents")
        .and_then(Value::as_str)
    {
        target.insert(
            "relayContextConfigContents".to_string(),
            Value::String(value.to_string()),
        );
    }
    if let Some(value) = source.get("activeRelayId").and_then(Value::as_str) {
        target.insert(
            "activeRelayId".to_string(),
            Value::String(value.to_string()),
        );
    }
    if let Some(value) = source.get("activeClaudeRelayId").and_then(Value::as_str) {
        target.insert(
            "activeClaudeRelayId".to_string(),
            Value::String(value.to_string()),
        );
    }
    if let Some(value) = source
        .get("activeClaudeDesktopRelayId")
        .and_then(Value::as_str)
    {
        target.insert(
            "activeClaudeDesktopRelayId".to_string(),
            Value::String(value.to_string()),
        );
    }
    if let Some(value) = source.get("relayTestModel").and_then(Value::as_str) {
        target.insert(
            "relayTestModel".to_string(),
            Value::String(if value.trim().is_empty() {
                default_relay_test_model()
            } else {
                value.trim().to_string()
            }),
        );
    }
    if let Some(value) = source.get("cliWrapperEnabled").and_then(Value::as_bool) {
        target.insert("cliWrapperEnabled".to_string(), Value::Bool(value));
    }
    if let Some(value) = source.get("cliWrapperBaseUrl").and_then(Value::as_str) {
        target.insert(
            "cliWrapperBaseUrl".to_string(),
            Value::String(value.to_string()),
        );
    }
    if let Some(value) = source.get("cliWrapperApiKey").and_then(Value::as_str) {
        target.insert(
            "cliWrapperApiKey".to_string(),
            Value::String(value.to_string()),
        );
    }
    if let Some(value) = source.get("cliWrapperApiKeyEnv").and_then(Value::as_str) {
        target.insert(
            "cliWrapperApiKeyEnv".to_string(),
            Value::String(if value.is_empty() {
                default_api_key_env()
            } else {
                value.to_string()
            }),
        );
    }
}

fn merge_bool_setting(target: &mut Map<String, Value>, source: &Map<String, Value>, key: &str) {
    if let Some(value) = source.get(key).and_then(Value::as_bool) {
        target.insert(key.to_string(), Value::Bool(value));
    }
}

fn preserve_official_mix_bearer_tokens(
    profiles: &mut [RelayProfile],
    previous: &Map<String, Value>,
) {
    let previous_tokens = previous
        .get("relayProfiles")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| serde_json::from_value::<RelayProfile>(value.clone()).ok())
        .filter_map(|profile| {
            if profile.relay_mode != RelayMode::Official || !profile.official_mix_api_key {
                return None;
            }
            let token = experimental_bearer_token_from_config_text(&profile.config_contents)?;
            Some((profile.id, token))
        })
        .collect::<HashMap<_, _>>();

    for profile in profiles {
        if profile.relay_mode != RelayMode::Official || !profile.official_mix_api_key {
            continue;
        }
        if profile.api_key_explicit {
            continue;
        }
        if experimental_bearer_token_from_config_text(&profile.config_contents).is_some() {
            continue;
        }
        let token = if profile.api_key.trim().is_empty() {
            previous_tokens.get(&profile.id).cloned()
        } else {
            Some(profile.api_key.trim().to_string())
        };
        let Some(token) = token else {
            continue;
        };
        profile.config_contents =
            set_or_replace_experimental_bearer_token(&profile.config_contents, &token);
    }
}

fn rewrite_profile_provider_id_to_match_profile_id(profile: &mut RelayProfile) {
    let profile_id = profile.id.trim();
    if profile_id.is_empty() || profile_id == "custom" {
        return;
    }
    let Ok(mut doc) = parse_toml_document(&profile.config_contents) else {
        return;
    };
    let Some(current_provider) = active_provider_id(&doc) else {
        return;
    };
    if current_provider != "custom" {
        return;
    }
    doc["model_provider"] = toml_edit::value(profile_id);
    if let Some(providers) = doc.get_mut("model_providers").and_then(Item::as_table_mut) {
        let moved = providers.remove("custom").unwrap_or_else(toml_edit::table);
        providers.insert(profile_id, moved);
    }
    profile.config_contents = ensure_text_newline(doc.to_string());
}

fn set_or_replace_experimental_bearer_token(contents: &str, token: &str) -> String {
    let mut doc = parse_toml_document(contents).unwrap_or_else(|_| DocumentMut::new());
    let provider_id =
        active_provider_id(&doc).unwrap_or_else(|| "claude-codex-pro-relay".to_string());
    doc["model_provider"] = toml_edit::value(provider_id.as_str());
    doc["model_providers"][provider_id.as_str()]["experimental_bearer_token"] =
        toml_edit::value(token.trim());
    ensure_text_newline(doc.to_string())
}

fn ensure_text_newline(mut value: String) -> String {
    if !value.is_empty() && !value.ends_with('\n') {
        value.push('\n');
    }
    value
}

fn experimental_bearer_token_from_config_text(contents: &str) -> Option<String> {
    let doc = parse_toml_document(contents).ok()?;
    let provider_id = active_provider_id(&doc)?;
    doc.get("model_providers")
        .and_then(Item::as_table)
        .and_then(|providers| providers.get(&provider_id))
        .and_then(Item::as_table)
        .and_then(|provider| provider.get("experimental_bearer_token"))
        .and_then(Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn active_provider_id(doc: &DocumentMut) -> Option<String> {
    doc.get("model_provider")
        .and_then(Item::as_str)
        .map(str::trim)
        .filter(|provider| !provider.is_empty())
        .map(ToString::to_string)
}

fn parse_toml_document(contents: &str) -> anyhow::Result<DocumentMut> {
    if contents.trim().is_empty() {
        Ok(DocumentMut::new())
    } else {
        contents
            .parse::<DocumentMut>()
            .with_context(|| "config.toml TOML 解析失败")
    }
}

fn settings_to_object(settings: &BackendSettings) -> Map<String, Value> {
    match serde_json::to_value(settings).unwrap_or_else(|_| Value::Object(Map::new())) {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

#[derive(Debug)]
struct ClaudeMappingTextRow {
    role: String,
    label: String,
    route_id: String,
    display_name: String,
    request_model: String,
    supports_1m: bool,
}

fn claude_mapping_role(label: &str, route_id: &str) -> Option<&'static str> {
    match label.trim().to_ascii_lowercase().as_str() {
        "sonnet" => Some("sonnet"),
        "opus" => Some("opus"),
        "fable" => Some("fable"),
        "haiku" => Some("haiku"),
        "subagent" => Some("subagent"),
        _ => {
            let route_id = route_id.trim().to_ascii_lowercase();
            if route_id.contains("sonnet") {
                Some("sonnet")
            } else if route_id.contains("opus") {
                Some("opus")
            } else if route_id.contains("fable") {
                Some("fable")
            } else if route_id.contains("haiku") {
                Some("haiku")
            } else if route_id.contains("subagent") {
                Some("subagent")
            } else {
                None
            }
        }
    }
}

fn claude_mapping_label(role: &str) -> Option<&'static str> {
    match role {
        "sonnet" => Some("Sonnet"),
        "opus" => Some("Opus"),
        "fable" => Some("Fable"),
        "haiku" => Some("Haiku"),
        "subagent" => Some("Subagent"),
        _ => None,
    }
}

fn parse_claude_mapping_text(text: &str) -> Vec<ClaudeMappingTextRow> {
    text.lines()
        .filter_map(|raw_line| {
            let trimmed = raw_line.trim();
            let supports_1m = trimmed.to_ascii_lowercase().ends_with("[1m]");
            let line = if supports_1m {
                trimmed[..trimmed.len().saturating_sub(4)].trim_end()
            } else {
                trimmed
            };
            let (left, right) = line.split_once(':')?;
            let left = left.trim();
            let right = right.trim();
            let (label, route_id) = match (left.rfind('('), left.rfind(')')) {
                (Some(open), Some(close)) if open < close => {
                    (left[..open].trim(), left[open + 1..close].trim())
                }
                _ => (left, ""),
            };
            let role = claude_mapping_role(label, route_id)?;
            let (display_name, request_model) = right
                .split_once("->")
                .map(|(display, request)| (display.trim(), request.trim()))
                .unwrap_or((right, right));
            if display_name.is_empty() && request_model.is_empty() {
                return None;
            }
            Some(ClaudeMappingTextRow {
                role: role.to_string(),
                label: label.to_string(),
                route_id: route_id.to_string(),
                display_name: if display_name.is_empty() {
                    request_model.to_string()
                } else {
                    display_name.to_string()
                },
                request_model: if request_model.is_empty() {
                    display_name.to_string()
                } else {
                    request_model.to_string()
                },
                supports_1m,
            })
        })
        .collect()
}

fn claude_mapping_text_from_json_entries(entries: &[Value]) -> Option<String> {
    let lines = entries
        .iter()
        .map(|entry| {
            let object = entry.as_object()?;
            let role = object.get("role")?.as_str()?;
            let route_id = object.get("routeId")?.as_str()?.trim();
            let request_model = object
                .get("requestModel")
                .or_else(|| object.get("model"))?
                .as_str()?
                .trim();
            if route_id.is_empty() || request_model.is_empty() {
                return None;
            }
            let label = object
                .get("label")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .or_else(|| claude_mapping_label(role))?;
            let display_name = object
                .get("displayName")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(request_model);
            let supports_1m = object
                .get("supports1m")
                .and_then(Value::as_bool)
                .or_else(|| object.get("supports_1m").and_then(Value::as_bool))
                .unwrap_or(false);
            Some(format!(
                "{label} ({route_id}): {display_name} -> {request_model}{}",
                if supports_1m { " [1M]" } else { "" }
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn claude_mapping_json_from_text_rows(rows: &[ClaudeMappingTextRow]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                serde_json::json!({
                    "role": row.role,
                    "label": row.label,
                    "routeId": row.route_id,
                    "displayName": row.display_name,
                    "requestModel": row.request_model,
                    "supports1m": row.supports_1m
                })
            })
            .collect(),
    )
}

fn strip_claude_mapping_one_m_marker(value: &str) -> (&str, bool) {
    let value = value.trim();
    const MARKER: &str = "[1M]";
    if value.len() >= MARKER.len()
        && value[value.len() - MARKER.len()..].eq_ignore_ascii_case(MARKER)
    {
        (value[..value.len() - MARKER.len()].trim_end(), true)
    } else {
        (value, false)
    }
}

fn claude_mapping_rows_from_config_contents(contents: &str) -> Vec<ClaudeMappingTextRow> {
    let Ok(config) = serde_json::from_str::<Value>(contents.trim()) else {
        return Vec::new();
    };
    let route_source = config.get("meta").unwrap_or(&config);
    let Some(routes) = route_source
        .get("claudeDesktopModelRoutes")
        .or_else(|| route_source.get("claude_desktop_model_routes"))
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };

    routes
        .iter()
        .filter_map(|(route_id, route)| {
            let role = claude_mapping_role("", route_id)?;
            let (raw_request_model, display_name, configured_supports_1m) = match route {
                Value::String(model) => {
                    let model = model.trim();
                    (model, None, None)
                }
                Value::Object(object) => {
                    let request_model = object
                        .get("model")
                        .or_else(|| object.get("requestModel"))
                        .and_then(Value::as_str)?
                        .trim();
                    let display_name = object
                        .get("labelOverride")
                        .or_else(|| object.get("displayName"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty());
                    let supports_1m = object
                        .get("supports1m")
                        .or_else(|| object.get("supports_1m"))
                        .and_then(Value::as_bool);
                    (request_model, display_name, supports_1m)
                }
                _ => return None,
            };
            let (request_model, marker_supports_1m) =
                strip_claude_mapping_one_m_marker(raw_request_model);
            if request_model.is_empty() {
                return None;
            }
            Some(ClaudeMappingTextRow {
                role: role.to_string(),
                label: claude_mapping_label(role)?.to_string(),
                route_id: route_id.trim().to_string(),
                display_name: display_name.unwrap_or(request_model).to_string(),
                request_model: request_model.to_string(),
                supports_1m: configured_supports_1m.unwrap_or(marker_supports_1m),
            })
        })
        .collect()
}

fn synchronize_claude_mapping_entries(entries: &mut Vec<Value>, rows: &[ClaudeMappingTextRow]) {
    for row in rows {
        let existing = entries.iter_mut().find(|entry| {
            entry
                .get("routeId")
                .and_then(Value::as_str)
                .is_some_and(|route_id| route_id.trim() == row.route_id)
        });
        if let Some(object) = existing.and_then(Value::as_object_mut) {
            if object
                .get("role")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                object.insert("role".to_string(), Value::String(row.role.clone()));
            }
            if object
                .get("label")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                object.insert("label".to_string(), Value::String(row.label.clone()));
            }
            object.insert(
                "displayName".to_string(),
                Value::String(row.display_name.clone()),
            );
            object.insert(
                "requestModel".to_string(),
                Value::String(row.request_model.clone()),
            );
            object.insert("supports1m".to_string(), Value::Bool(row.supports_1m));
        } else {
            entries.push(serde_json::json!({
                "role": row.role,
                "label": row.label,
                "routeId": row.route_id,
                "displayName": row.display_name,
                "requestModel": row.request_model,
                "supports1m": row.supports_1m
            }));
        }
    }
}

fn store_claude_mapping_representations(profile: &mut RelayProfile, entries: Vec<Value>) {
    if let Some(text) = claude_mapping_text_from_json_entries(&entries) {
        profile.model_mapping = text;
    }
    profile.model_mapping_json = Value::Array(entries).to_string();
}

fn repair_claude_model_mapping_representation(profile: &mut RelayProfile) {
    if !matches!(
        normalized_target_app(&profile.target_app),
        "claude" | "claude-desktop"
    ) {
        return;
    }
    let text_rows = parse_claude_mapping_text(&profile.model_mapping);
    let config_rows = claude_mapping_rows_from_config_contents(&profile.config_contents);
    let raw_json = profile.model_mapping_json.trim();
    let parsed_mapping = (!raw_json.is_empty())
        .then(|| serde_json::from_str::<Value>(raw_json).ok())
        .flatten();
    let mut entries = match parsed_mapping {
        Some(Value::Array(entries)) => entries,
        Some(_) if config_rows.is_empty() => return,
        _ => Vec::new(),
    };
    let mut rebuilt = false;
    if entries.is_empty() {
        let source_rows = if text_rows.is_empty() {
            &config_rows
        } else {
            &text_rows
        };
        if source_rows.is_empty() {
            return;
        }
        entries = claude_mapping_json_from_text_rows(source_rows)
            .as_array()
            .cloned()
            .unwrap_or_default();
        rebuilt = true;
    }

    if !config_rows.is_empty() {
        synchronize_claude_mapping_entries(&mut entries, &config_rows);
        store_claude_mapping_representations(profile, entries);
        return;
    }
    if rebuilt {
        store_claude_mapping_representations(profile, entries);
        return;
    }

    let Some(explicit_haiku) = text_rows.iter().find(|row| {
        row.role == "haiku"
            && row.route_id == "claude-haiku-4-5"
            && row.display_name == "claude-opus-4-7"
            && row.request_model == "claude-opus-4-7"
    }) else {
        if let Some(text) = claude_mapping_text_from_json_entries(&entries) {
            profile.model_mapping = text;
        }
        return;
    };
    let Some(haiku) = entries.iter_mut().find_map(|entry| {
        let object = entry.as_object_mut()?;
        let role = object
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let route_id = object
            .get("routeId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let request_model = object
            .get("requestModel")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let display_name = object
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or_default();
        (role == "haiku"
            && route_id == "claude-haiku-4-5"
            && request_model == "claude-haiku-4-5"
            && (display_name.is_empty() || display_name == "claude-haiku-4-5"))
            .then_some(object)
    }) else {
        if let Some(text) = claude_mapping_text_from_json_entries(&entries) {
            profile.model_mapping = text;
        }
        return;
    };
    haiku.insert(
        "displayName".to_string(),
        Value::String(explicit_haiku.display_name.clone()),
    );
    haiku.insert(
        "requestModel".to_string(),
        Value::String(explicit_haiku.request_model.clone()),
    );
    haiku.insert(
        "supports1m".to_string(),
        Value::Bool(explicit_haiku.supports_1m),
    );

    store_claude_mapping_representations(profile, entries);
}

fn normalize_settings_config_sections(mut settings: BackendSettings) -> BackendSettings {
    let (common, extracted_context) =
        split_context_config_sections(&settings.relay_common_config_contents);
    let context = join_config_sections(&[
        settings.relay_context_config_contents.as_str(),
        extracted_context.as_str(),
    ]);
    settings.relay_common_config_contents = crate::relay_config::normalize_config_text(&common);
    settings.relay_context_config_contents = crate::relay_config::normalize_config_text(&context);
    for profile in &mut settings.relay_profiles {
        let _ = crate::relay_config::normalize_relay_profile_for_storage(profile);
        repair_claude_model_mapping_representation(profile);
    }
    settings.codex_app_image_overlay_opacity =
        clamp_image_overlay_opacity(settings.codex_app_image_overlay_opacity);
    settings.memory_assist_max_injected_items =
        clamp_memory_assist_max_injected_items(settings.memory_assist_max_injected_items);
    if settings.memory_assist_workspace_mode.trim().is_empty() {
        settings.memory_assist_workspace_mode = default_memory_assist_workspace_mode();
    }
    settings
}

fn split_context_config_sections(config: &str) -> (String, String) {
    let mut common = Vec::new();
    let mut context = Vec::new();
    let mut in_context_table = false;

    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_context_table = is_context_table_header(trimmed);
        }
        if in_context_table {
            context.push(line);
        } else {
            common.push(line);
        }
    }

    (
        normalize_text_config(common.join("\n")),
        normalize_text_config(context.join("\n")),
    )
}

fn is_context_table_header(header: &str) -> bool {
    header.starts_with("[mcp_servers.")
        || header.starts_with("[skills.")
        || header.starts_with("[plugins.")
}

fn join_config_sections(sections: &[&str]) -> String {
    let joined = sections
        .iter()
        .map(|section| section.trim())
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    normalize_text_config(joined)
}

fn normalize_text_config(contents: String) -> String {
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        create_private_dir_all(parent)?;
    }

    let temp_path = temp_path_for(path);
    write_private_file(&temp_path, bytes)?;
    let replace_result = replace_file(&temp_path, path).with_context(|| {
        format!(
            "failed to replace {} with {}",
            path.display(),
            temp_path.display()
        )
    });
    if replace_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    replace_result?;
    secure_private_path(path)?;
    Ok(())
}

pub(crate) fn create_private_dir_all(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder
            .create(path)
            .with_context(|| format!("failed to create directory {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(path)
            .with_context(|| format!("failed to create directory {}", path.display()))?;
    }
    Ok(())
}

fn write_private_file(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .with_context(|| format!("failed to write temp file {}", path.display()))?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to secure temp file {}", path.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("failed to write temp file {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        use std::io::Write;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .with_context(|| format!("failed to write temp file {}", path.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("failed to write temp file {}", path.display()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn secure_private_file(_path: &Path, file: &File) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(unix)]
fn secure_private_path(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(windows)]
fn secure_private_path(path: &Path) -> anyhow::Result<()> {
    use crate::windows_create_no_window;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Use well-known SIDs instead of USERDOMAIN/USERNAME: launchers and
    // elevated helpers can inherit environment variables for a different
    // account than the process token. OWNER RIGHTS follows the actual file
    // owner, while LocalSystem keeps elevated maintenance flows working.
    let mut command = Command::new("icacls.exe");
    command
        .arg(path)
        .args(["/inheritance:r", "/grant:r"])
        .args(["*S-1-3-4:(F)", "*S-1-5-18:(F)"])
        .creation_flags(windows_create_no_window());
    let output = command
        .output()
        .with_context(|| format!("failed to secure private file {}", path.display()))?;
    if !output.status.success() {
        anyhow::bail!(
            "failed to secure private file {} (icacls exit {})",
            path.display(),
            output.status
        );
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn secure_private_path(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> anyhow::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    use windows::core::PCWSTR;

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(anyhow::Error::from)
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> anyhow::Result<()> {
    fs::rename(source, destination).map_err(anyhow::Error::from)
}

fn temp_path_for(path: &Path) -> PathBuf {
    let mut temp_path = path.to_path_buf();
    let extension = path.extension().and_then(|value| value.to_str());
    let write_id = NEXT_ATOMIC_WRITE_ID.fetch_add(1, Ordering::Relaxed);
    temp_path.set_extension(match extension {
        Some(extension) => format!("{extension}.{}.{}.tmp", std::process::id(), write_id),
        None => format!("{}.{}.tmp", std::process::id(), write_id),
    });
    temp_path
}

fn settings_lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.to_path_buf();
    let extension = path.extension().and_then(|value| value.to_str());
    lock_path.set_extension(match extension {
        Some(extension) => format!("{extension}.lock"),
        None => "lock".to_string(),
    });
    lock_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "claude-codex-pro-core-settings-test-{}-{}",
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn settings_default_matches_expected_behavior() {
        let settings = BackendSettings::default();
        assert!(!settings.provider_sync_enabled);
        assert!(settings.relay_profiles_enabled);
        assert!(settings.enhancements_enabled);
        assert!(!settings.computer_use_guard_enabled);
        assert!(settings.codex_app_plugin_entry_unlock);
        assert!(settings.codex_app_plugin_marketplace_unlock);
        assert!(settings.codex_app_force_plugin_install);
        assert!(!settings.codex_goals_enabled);
        assert!(settings.memory_assist_enabled);
        assert!(settings.memory_assist_inject_enabled);
        assert!(settings.memory_assist_auto_suggest_enabled);
        assert_eq!(settings.memory_assist_max_injected_items, 5);
        assert_eq!(settings.memory_assist_workspace_mode, "project_plus_global");
        assert!(settings.codex_app_path.is_empty());
        assert!(settings.codex_extra_args.is_empty());
        assert_eq!(
            settings.zed_remote_open_strategy,
            ZedOpenStrategy::AddToFocusedWorkspace
        );
        assert!(settings.zed_remote_project_registry_enabled);
        assert!(!settings.zed_remote_sync_to_zed_settings);
        assert_eq!(settings.launch_mode, LaunchMode::Patch);
        assert_eq!(settings.relay_base_url, default_relay_base_url());
        assert!(settings.relay_api_key.is_empty());
        assert_eq!(settings.relay_profiles[0].relay_mode, RelayMode::Official);
        assert!(settings.relay_common_config_contents.is_empty());
        assert_eq!(settings.relay_test_model, default_relay_test_model());
        assert!(!settings.cli_wrapper_enabled);
        assert_eq!(settings.cli_wrapper_api_key_env, "CUSTOM_OPENAI_API_KEY");
    }

    #[test]
    fn settings_load_repairs_claude_mapping_divergence_without_overwriting_custom_json() {
        fn split_mapping_json() -> Value {
            json!([
                {
                    "role": "sonnet",
                    "label": "Sonnet",
                    "routeId": "claude-sonnet-4-6",
                    "displayName": "claude-opus-4-6",
                    "requestModel": "claude-opus-4-6",
                    "supports1m": true
                },
                {
                    "role": "opus",
                    "label": "Opus",
                    "routeId": "claude-opus-4-8",
                    "displayName": "claude-opus-4-8",
                    "requestModel": "claude-opus-4-8",
                    "supports1m": true
                },
                {
                    "role": "haiku",
                    "label": "Haiku",
                    "routeId": "claude-haiku-4-5",
                    "displayName": "",
                    "requestModel": "claude-haiku-4-5",
                    "supports1m": true
                }
            ])
        }

        const EXPLICIT_TEXT: &str = "Sonnet (claude-sonnet-4-6): claude-opus-4-6 -> claude-opus-4-6 [1M]\n\
Opus (claude-opus-4-8): claude-opus-4-8 -> claude-opus-4-8 [1M]\n\
Haiku (claude-haiku-4-5): claude-opus-4-7 -> claude-opus-4-7 [1M]";

        let split_json = split_mapping_json();
        let mut custom_json = split_mapping_json();
        let custom_haiku = custom_json
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["role"] == "haiku")
            .unwrap();
        custom_haiku["displayName"] = json!("custom-haiku");
        custom_haiku["requestModel"] = json!("custom-haiku");
        let unchanged_custom = custom_json.to_string();
        let expected_custom_text =
            claude_mapping_text_from_json_entries(custom_json.as_array().unwrap()).unwrap();
        let unchanged_codex = split_json.to_string();
        let generated_config_split_json = json!([
            {
                "role": "fable",
                "label": "Fable",
                "routeId": "claude-fable-5",
                "displayName": "claude-fable-5",
                "requestModel": "claude-fable-5",
                "supports1m": false
            },
            {
                "role": "haiku",
                "label": "Haiku",
                "routeId": "claude-haiku-4-5",
                "displayName": "claude-haiku-4-5",
                "requestModel": "claude-haiku-4-5",
                "supports1m": false
            },
            {
                "role": "subagent",
                "label": "Subagent",
                "routeId": "claude-subagent",
                "displayName": "custom-subagent",
                "requestModel": "custom-subagent",
                "supports1m": false
            }
        ]);
        let generated_config_contents = json!({
            "meta": {
                "claudeDesktopModelRoutes": {
                    "claude-fable-5": {
                        "model": "claude-opus-4-8",
                        "labelOverride": "claude-opus-4-8",
                        "supports1m": true
                    },
                    "claude-haiku-4-5": {
                        "model": "claude-opus-4-7",
                        "labelOverride": "claude-opus-4-7",
                        "supports1m": true
                    },
                    "claude-sonnet-5": {
                        "model": "claude-opus-4-6[1M]",
                        "labelOverride": "claude-opus-4-6"
                    },
                    "not-a-role": {
                        "model": "ignored-model"
                    }
                }
            }
        })
        .to_string();
        let invalid_config_contents = json!({
            "meta": {
                "claudeDesktopModelRoutes": {
                    "claude-haiku-4-5": {
                        "labelOverride": "missing-model"
                    }
                }
            }
        })
        .to_string();

        let dir = temp_dir();
        let path = dir.join("settings.json");
        let store = SettingsStore::new(path.clone());
        let settings = BackendSettings {
            relay_profiles: vec![
                RelayProfile {
                    id: "known-split".to_string(),
                    target_app: "claude-desktop".to_string(),
                    model_mapping_enabled: true,
                    model_mapping_json: split_json.to_string(),
                    model_mapping: EXPLICIT_TEXT.to_string(),
                    ..RelayProfile::default()
                },
                RelayProfile {
                    id: "custom-json".to_string(),
                    target_app: "claude".to_string(),
                    model_mapping_json: unchanged_custom.clone(),
                    model_mapping: EXPLICIT_TEXT.to_string(),
                    ..RelayProfile::default()
                },
                RelayProfile {
                    id: "blank-json".to_string(),
                    target_app: "claude-desktop".to_string(),
                    model_mapping_enabled: true,
                    model_mapping_json: "   ".to_string(),
                    model_mapping: EXPLICIT_TEXT.to_string(),
                    ..RelayProfile::default()
                },
                RelayProfile {
                    id: "empty-array-json".to_string(),
                    target_app: "claude-desktop".to_string(),
                    model_mapping_enabled: true,
                    model_mapping_json: "[]".to_string(),
                    model_mapping: EXPLICIT_TEXT.to_string(),
                    ..RelayProfile::default()
                },
                RelayProfile {
                    id: "codex-profile".to_string(),
                    target_app: "codex".to_string(),
                    model_mapping_json: unchanged_codex.clone(),
                    ..RelayProfile::default()
                },
                RelayProfile {
                    id: "generated-config-split".to_string(),
                    target_app: "claude-desktop".to_string(),
                    model_mapping_enabled: true,
                    model_mapping_json: generated_config_split_json.to_string(),
                    config_contents: generated_config_contents,
                    ..RelayProfile::default()
                },
                RelayProfile {
                    id: "invalid-generated-config".to_string(),
                    target_app: "claude-desktop".to_string(),
                    model_mapping_enabled: true,
                    model_mapping_json: unchanged_custom.clone(),
                    config_contents: invalid_config_contents,
                    ..RelayProfile::default()
                },
            ],
            ..BackendSettings::default()
        };
        std::fs::write(&path, serde_json::to_vec_pretty(&settings).unwrap()).unwrap();

        let loaded = store.load().unwrap();
        let repaired_entries =
            serde_json::from_str::<Value>(&loaded.relay_profiles[0].model_mapping_json).unwrap();
        let repaired_haiku = repaired_entries
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["role"] == "haiku")
            .unwrap();
        assert_eq!(repaired_haiku["requestModel"], "claude-opus-4-7");
        assert_eq!(repaired_haiku["displayName"], "claude-opus-4-7");
        assert_eq!(loaded.relay_profiles[0].model_mapping, EXPLICIT_TEXT);
        assert_eq!(
            crate::protocol_proxy::claude_desktop_model_mapping_for(
                "claude-haiku-4-5",
                &loaded.relay_profiles[0]
            )
            .as_deref(),
            Some("claude-opus-4-7")
        );

        assert_eq!(
            loaded.relay_profiles[1].model_mapping_json,
            unchanged_custom
        );
        assert_eq!(loaded.relay_profiles[1].model_mapping, expected_custom_text);

        let rebuilt_entries =
            serde_json::from_str::<Value>(&loaded.relay_profiles[2].model_mapping_json).unwrap();
        let rebuilt_haiku = rebuilt_entries
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["role"] == "haiku")
            .unwrap();
        assert_eq!(rebuilt_haiku["requestModel"], "claude-opus-4-7");
        assert_eq!(rebuilt_haiku["displayName"], "claude-opus-4-7");
        assert_eq!(loaded.relay_profiles[2].model_mapping, EXPLICIT_TEXT);

        let rebuilt_empty_entries =
            serde_json::from_str::<Value>(&loaded.relay_profiles[3].model_mapping_json).unwrap();
        let rebuilt_empty_haiku = rebuilt_empty_entries
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["role"] == "haiku")
            .unwrap();
        assert_eq!(rebuilt_empty_haiku["requestModel"], "claude-opus-4-7");
        assert_eq!(loaded.relay_profiles[3].model_mapping, EXPLICIT_TEXT);
        assert_eq!(
            crate::protocol_proxy::claude_desktop_model_mapping_for(
                "claude-haiku-4-5",
                &loaded.relay_profiles[3]
            )
            .as_deref(),
            Some("claude-opus-4-7")
        );

        assert_eq!(loaded.relay_profiles[4].model_mapping_json, unchanged_codex);

        let config_repaired_entries =
            serde_json::from_str::<Value>(&loaded.relay_profiles[5].model_mapping_json).unwrap();
        let config_repaired_entries = config_repaired_entries.as_array().unwrap();
        let request_model_for = |route_id: &str| {
            config_repaired_entries
                .iter()
                .find(|entry| entry["routeId"] == route_id)
                .and_then(|entry| entry["requestModel"].as_str())
        };
        assert_eq!(
            request_model_for("claude-haiku-4-5"),
            Some("claude-opus-4-7")
        );
        assert_eq!(request_model_for("claude-fable-5"), Some("claude-opus-4-8"));
        assert_eq!(
            request_model_for("claude-sonnet-5"),
            Some("claude-opus-4-6")
        );
        assert_eq!(
            request_model_for("claude-subagent"),
            Some("custom-subagent"),
            "config synchronization must preserve JSON-only custom routes"
        );
        assert_eq!(request_model_for("not-a-role"), None);
        assert_eq!(
            crate::protocol_proxy::claude_desktop_model_mapping_for(
                "claude-haiku-4-5",
                &loaded.relay_profiles[5]
            )
            .as_deref(),
            Some("claude-opus-4-7")
        );
        assert!(
            loaded.relay_profiles[5]
                .model_mapping
                .contains("Haiku (claude-haiku-4-5): claude-opus-4-7 -> claude-opus-4-7 [1M]")
        );

        assert_eq!(
            loaded.relay_profiles[6].model_mapping_json, unchanged_custom,
            "an invalid generated route must not overwrite an existing mapping"
        );

        assert_eq!(
            normalize_settings_config_sections(loaded.clone()),
            loaded,
            "mapping repair must be deterministic"
        );
    }

    #[test]
    fn settings_store_save_and_load_preserves_current_claude_credentials() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));
        let settings = BackendSettings {
            relay_profiles: vec![RelayProfile {
                id: "claude-current-key".to_string(),
                target_app: "claude-desktop".to_string(),
                api_key: "test-current-key".to_string(),
                upstream_base_url: "https://relay.example/v1".to_string(),
                config_contents: json!({
                    "env": {
                        "ANTHROPIC_BASE_URL": "https://relay.old.example/v1",
                        "OPENAI_API_KEY": "test-old-config-key",
                        "KEEP_ENV": "keep-env"
                    },
                    "metadata": { "keep": true }
                })
                .to_string(),
                auth_contents: json!({
                    "ANTHROPIC_API_KEY": "test-old-auth-key",
                    "keepAuth": true
                })
                .to_string(),
                ..RelayProfile::default()
            }],
            ..BackendSettings::default()
        };

        store.save(&settings).unwrap();
        let loaded = store.load().unwrap();
        let profile = &loaded.relay_profiles[0];
        let config: Value = serde_json::from_str(&profile.config_contents).unwrap();
        let auth: Value = serde_json::from_str(&profile.auth_contents).unwrap();

        assert_eq!(profile.api_key, "test-current-key");
        assert_eq!(profile.base_url, "https://relay.example/v1");
        assert_eq!(profile.upstream_base_url, "https://relay.example/v1");
        assert_eq!(config["env"]["ANTHROPIC_AUTH_TOKEN"], "test-current-key");
        assert_eq!(auth["ANTHROPIC_AUTH_TOKEN"], "test-current-key");
        assert_eq!(config["env"]["KEEP_ENV"], "keep-env");
        assert_eq!(config["metadata"]["keep"], true);
        assert_eq!(auth["keepAuth"], true);
        for alias in ["OPENAI_API_KEY", "ANTHROPIC_API_KEY", "api_key", "apiKey"] {
            assert!(config["env"].get(alias).is_none());
            assert!(auth.get(alias).is_none());
        }
    }

    #[test]
    fn legacy_settings_default_memory_assist_fields() {
        let settings: BackendSettings = serde_json::from_value(json!({
            "codexAppPath": "C:\\Portable\\Codex\\app",
            "memoryAssistMaxInjectedItems": 99
        }))
        .unwrap();

        assert!(settings.memory_assist_enabled);
        assert!(settings.memory_assist_inject_enabled);
        assert!(settings.memory_assist_auto_suggest_enabled);
        // LLM summarization sends memory content to an external relay, so it must
        // stay off unless the user explicitly opts in.
        assert!(!settings.memory_assist_llm_summary_enabled);
        // The MCP server exposes memory to external agents, so it must default off.
        assert!(!settings.memory_assist_mcp_enabled);
        assert_eq!(settings.memory_assist_max_injected_items, 20);
        assert_eq!(settings.memory_assist_workspace_mode, "project_plus_global");
    }

    #[test]
    fn settings_deserialize_uses_existing_json_keys() {
        let settings: BackendSettings = serde_json::from_str(
            r#"{"codexAppPath":"C:\\Portable\\Codex\\app","providerSyncEnabled":true,"codexGoalsEnabled":true,"cliWrapperEnabled":true,"cliWrapperBaseUrl":"https://example.test","cliWrapperApiKey":"sk-test","cliWrapperApiKeyEnv":""}"#,
        )
        .unwrap();
        assert_eq!(settings.codex_app_path, r"C:\Portable\Codex\app");
        assert!(settings.provider_sync_enabled);
        assert!(settings.codex_goals_enabled);
        assert!(settings.cli_wrapper_enabled);
        assert_eq!(settings.cli_wrapper_base_url, "https://example.test");
        assert_eq!(settings.cli_wrapper_api_key, "sk-test");
        assert_eq!(settings.cli_wrapper_api_key_env, "CUSTOM_OPENAI_API_KEY");
        assert_eq!(settings.relay_base_url, default_relay_base_url());
        assert!(settings.codex_extra_args.is_empty());
    }

    #[test]
    fn settings_deserialize_keeps_plugin_unlock_switches_independent() {
        let settings: BackendSettings = serde_json::from_str(
            r#"{
                "codexAppPluginEntryUnlock": false,
                "codexAppPluginMarketplaceUnlock": true,
                "codexAppForcePluginInstall": false
            }"#,
        )
        .unwrap();

        assert!(!settings.codex_app_plugin_entry_unlock);
        assert!(settings.codex_app_plugin_marketplace_unlock);
        assert!(!settings.codex_app_force_plugin_install);

        let legacy_settings: BackendSettings = serde_json::from_str(
            r#"{
                "codexAppPluginEntryUnlock": false,
                "codexAppForcePluginInstall": false
            }"#,
        )
        .unwrap();

        assert!(!legacy_settings.codex_app_plugin_entry_unlock);
        assert!(legacy_settings.codex_app_plugin_marketplace_unlock);
        assert!(!legacy_settings.codex_app_force_plugin_install);
    }

    #[test]
    fn settings_deserialize_reads_codex_extra_args() {
        let settings: BackendSettings = serde_json::from_str(
            r#"{"codexExtraArgs":["--force_high_performance_gpu"," --ignored-trimmed-by-ui "]}"#,
        )
        .unwrap();

        assert_eq!(
            settings.codex_extra_args,
            vec![
                "--force_high_performance_gpu".to_string(),
                " --ignored-trimmed-by-ui ".to_string(),
            ]
        );
    }

    #[test]
    fn relay_profile_official_mix_api_key_defaults_to_false() {
        let profile: RelayProfile =
            serde_json::from_str(r#"{"id":"official","name":"官方","relayMode":"official"}"#)
                .unwrap();

        assert_eq!(profile.relay_mode, RelayMode::Official);
        assert!(!profile.official_mix_api_key);
        assert!(profile.test_model.is_empty());
    }

    #[test]
    fn relay_profile_context_fields_default_to_empty() {
        let profile = RelayProfile::default();

        assert!(profile.context_selection.mcp_servers.is_empty());
        assert!(profile.context_selection.skills.is_empty());
        assert!(profile.context_selection.plugins.is_empty());
        assert!(profile.use_common_config);
        assert!(!profile.context_selection_initialized);
        assert!(profile.context_window.is_empty());
        assert!(profile.auto_compact_limit.is_empty());
        assert_eq!(profile.model_insert_mode, RelayModelInsertMode::Patch);
        assert!(profile.model_list.is_empty());
    }

    #[test]
    fn relay_profile_context_fields_deserialize_from_camel_case() {
        let profile: RelayProfile = serde_json::from_str(
            r#"{
                "id":"relay-a",
                "name":"供应商 A",
                "contextSelection":{
                    "mcpServers":["context7"],
                    "skills":["writer"],
                    "plugins":["local"]
                },
                "contextSelectionInitialized":true,
                "useCommonConfig":false,
                "contextWindow":"200000",
                "autoCompactLimit":"160000",
                "modelInsertMode":"patch",
                "modelList":"qwen3-coder\ndeepseek-coder"
            }"#,
        )
        .unwrap();

        assert_eq!(profile.context_selection.mcp_servers, vec!["context7"]);
        assert_eq!(profile.context_selection.skills, vec!["writer"]);
        assert_eq!(profile.context_selection.plugins, vec!["local"]);
        assert!(!profile.use_common_config);
        assert!(profile.context_selection_initialized);
        assert_eq!(profile.context_window, "200000");
        assert_eq!(profile.auto_compact_limit, "160000");
        assert_eq!(profile.model_insert_mode, RelayModelInsertMode::Patch);
        assert_eq!(profile.model_list, "qwen3-coder\ndeepseek-coder");
    }

    fn target_profile(id: &str, target_app: &str) -> RelayProfile {
        RelayProfile {
            id: id.to_string(),
            name: format!("供应商 {id}"),
            target_app: target_app.to_string(),
            ..RelayProfile::default()
        }
    }

    #[test]
    fn explicit_claude_key_defaults_to_bearer_for_third_party_relay() {
        let profile = RelayProfile {
            target_app: "claude".to_string(),
            base_url: "https://relay.example.com/v1".to_string(),
            api_key: "sk-relay-abc".to_string(),
            auth_field: String::new(),
            ..RelayProfile::default()
        };

        assert!(
            !relay_profile_uses_anthropic_api_key(&profile),
            "第三方中转且未声明 authField 时应走 Bearer"
        );
        assert_eq!(relay_profile_resolved_api_key(&profile), "sk-relay-abc");
    }

    #[test]
    fn explicit_claude_key_uses_x_api_key_for_official_anthropic() {
        let profile = RelayProfile {
            target_app: "claude".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            api_key: "sk-ant-api03-xyz".to_string(),
            auth_field: String::new(),
            ..RelayProfile::default()
        };

        assert!(
            relay_profile_uses_anthropic_api_key(&profile),
            "官方 Anthropic 域名的显式 Key 应走 x-api-key"
        );
    }

    #[test]
    fn official_anthropic_oauth_token_stays_bearer() {
        let profile = RelayProfile {
            target_app: "claude".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            api_key: "sk-ant-oat01-token".to_string(),
            auth_field: String::new(),
            ..RelayProfile::default()
        };

        assert!(
            !relay_profile_uses_anthropic_api_key(&profile),
            "OAuth token（sk-ant-oat）即便命中官方域名也应走 Bearer"
        );
    }

    #[test]
    fn explicit_auth_field_overrides_url_inference() {
        let bearer_on_official = RelayProfile {
            target_app: "claude".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            api_key: "sk-ant-api03-xyz".to_string(),
            auth_field: "ANTHROPIC_AUTH_TOKEN".to_string(),
            ..RelayProfile::default()
        };
        assert!(
            !relay_profile_uses_anthropic_api_key(&bearer_on_official),
            "显式 authField=ANTHROPIC_AUTH_TOKEN 应强制 Bearer"
        );

        let x_api_key_on_relay = RelayProfile {
            target_app: "claude".to_string(),
            base_url: "https://relay.example.com/v1".to_string(),
            api_key: "sk-relay-abc".to_string(),
            auth_field: "ANTHROPIC_API_KEY".to_string(),
            ..RelayProfile::default()
        };
        assert!(
            relay_profile_uses_anthropic_api_key(&x_api_key_on_relay),
            "显式 authField=ANTHROPIC_API_KEY 应强制 x-api-key"
        );
    }

    #[test]
    fn upstream_base_url_takes_priority_for_official_detection() {
        let profile = RelayProfile {
            target_app: "claude-desktop".to_string(),
            base_url: "http://127.0.0.1:57321/v1".to_string(),
            upstream_base_url: "https://api.anthropic.com".to_string(),
            api_key: "sk-ant-api03-xyz".to_string(),
            auth_field: String::new(),
            ..RelayProfile::default()
        };

        assert!(
            relay_profile_uses_anthropic_api_key(&profile),
            "官方判定应看 upstreamBaseUrl 而非本地代理 baseUrl"
        );
    }

    #[test]
    fn codex_target_never_uses_anthropic_header() {
        let profile = RelayProfile {
            target_app: "codex".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            api_key: "sk-ant-api03-xyz".to_string(),
            auth_field: String::new(),
            ..RelayProfile::default()
        };

        assert!(
            !relay_profile_uses_anthropic_api_key(&profile),
            "非 Claude 目标永远不应使用 Anthropic x-api-key"
        );
    }

    #[test]
    fn host_is_official_anthropic_matches_domain_and_subdomains() {
        assert!(host_is_official_anthropic("https://api.anthropic.com"));
        assert!(host_is_official_anthropic("https://api.anthropic.com/v1"));
        assert!(host_is_official_anthropic(
            "https://eu.api.anthropic.com/v1"
        ));
        assert!(host_is_official_anthropic("api.anthropic.com:443"));
        assert!(!host_is_official_anthropic(
            "https://api.anthropic.com.evil.example/v1"
        ));
        assert!(!host_is_official_anthropic("https://relay.example.com/v1"));
        assert!(!host_is_official_anthropic("https://anthropic.com.cn/v1"));
        assert!(!host_is_official_anthropic(""));
    }

    #[test]
    fn target_profile_requires_an_exact_active_id_when_multiple_profiles_exist() {
        let settings = BackendSettings {
            relay_profiles: vec![
                target_profile("claude-old", "claude"),
                target_profile("claude-current", "claude"),
            ],
            active_claude_relay_id: String::new(),
            ..BackendSettings::default()
        };

        let active = settings.active_relay_profile_for_target("claude");

        assert_eq!(active.name, "未配置供应商");
        assert!(active.id.is_empty());
    }

    #[test]
    fn target_profile_does_not_fall_back_when_active_id_is_stale() {
        let settings = BackendSettings {
            relay_profiles: vec![target_profile("desktop-old", "claude-desktop")],
            active_claude_desktop_relay_id: "desktop-missing".to_string(),
            ..BackendSettings::default()
        };

        let active = settings.active_relay_profile_for_target("claude-desktop");

        assert_eq!(active.name, "未配置供应商");
        assert_eq!(active.id, "desktop-missing");
    }

    #[test]
    fn target_profile_keeps_single_profile_legacy_compatibility() {
        let settings = BackendSettings {
            relay_profiles: vec![target_profile("claude-only", "claude")],
            active_claude_relay_id: String::new(),
            ..BackendSettings::default()
        };

        let active = settings.active_relay_profile_for_target("claude");

        assert_eq!(active.id, "claude-only");
    }

    #[test]
    fn target_profile_uses_exact_active_id() {
        let settings = BackendSettings {
            relay_profiles: vec![
                target_profile("desktop-old", "claude-desktop"),
                target_profile("desktop-current", "claude-desktop"),
            ],
            active_claude_desktop_relay_id: "desktop-current".to_string(),
            ..BackendSettings::default()
        };

        let active = settings.active_relay_profile_for_target("claude-desktop");

        assert_eq!(active.id, "desktop-current");
    }

    #[test]
    fn relay_profile_derived_fields_are_read_but_not_serialized() {
        let profile: RelayProfile = serde_json::from_str(
            r#"{
                "id":"relay-a",
                "name":"供应商 A",
                "model":"gpt-5.4",
                "baseUrl":"https://relay.example/v1",
                "apiKey":"sk-test",
                "configContents":"model = \"gpt-5.4\"\n",
                "authContents":"{\"OPENAI_API_KEY\":\"sk-test\"}"
            }"#,
        )
        .unwrap();

        assert_eq!(profile.model, "gpt-5.4");
        assert_eq!(profile.base_url, "https://relay.example/v1");
        assert_eq!(profile.api_key, "sk-test");

        let saved = serde_json::to_value(&profile).unwrap();
        assert!(saved.get("model").is_none());
        assert!(saved.get("baseUrl").is_none());
        assert!(saved.get("apiKey").is_none());
        assert_eq!(saved["configContents"], "model = \"gpt-5.4\"\n");
        assert_eq!(saved["authContents"], "{\"OPENAI_API_KEY\":\"sk-test\"}");
    }

    #[test]
    fn chat_protocol_profile_roundtrip_migrates_upstream_base_url_out_of_config() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));
        let settings = BackendSettings {
            relay_profiles: vec![RelayProfile {
                id: "relay-chat".to_string(),
                name: "DeepSeek".to_string(),
                protocol: RelayProtocol::ChatCompletions,
                relay_mode: RelayMode::PureApi,
                config_contents: r#"model = "deepseek-chat"
claude_codex_pro_chat_base_url = "https://api.deepseek.com"
model_provider = "custom"

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true
base_url = "http://127.0.0.1:57321/v1"
"#
                .to_string(),
                auth_contents: r#"{"OPENAI_API_KEY":"sk-test"}"#.to_string(),
                ..RelayProfile::default()
            }],
            active_relay_id: "relay-chat".to_string(),
            ..BackendSettings::default()
        };

        store.save(&settings).unwrap();
        let loaded = store.load().unwrap();
        let active = loaded.active_relay_profile();

        assert_eq!(active.protocol, RelayProtocol::ChatCompletions);
        assert_eq!(active.base_url, "https://api.deepseek.com");
        assert_eq!(active.upstream_base_url, "https://api.deepseek.com");
        assert_eq!(active.api_key, "sk-test");
        assert!(
            !active
                .config_contents
                .contains("claude_codex_pro_chat_base_url")
        );

        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap())
                .unwrap();
        let profile = &saved["relayProfiles"][0];
        assert!(profile.get("baseUrl").is_none());
        assert_eq!(profile["upstreamBaseUrl"], "https://api.deepseek.com");
        assert!(profile.get("apiKey").is_none());
        assert!(
            !profile["configContents"]
                .as_str()
                .unwrap()
                .contains("claude_codex_pro_chat_base_url")
        );
    }

    #[test]
    fn official_profile_without_mix_does_not_persist_api_config() {
        let settings = BackendSettings {
            relay_profiles: vec![RelayProfile {
                id: "official".to_string(),
                name: "官方".to_string(),
                relay_mode: RelayMode::Official,
                official_mix_api_key: false,
                model: "gpt-5.5".to_string(),
                base_url: "https://relay.example/v1".to_string(),
                api_key: "sk-test".to_string(),
                config_contents: r#"model = "gpt-5.5"
model_provider = "custom"

[model_providers.custom]
requires_openai_auth = true
"#
                .to_string(),
                auth_contents: r#"{"OPENAI_API_KEY":"sk-test"}"#.to_string(),
                ..RelayProfile::default()
            }],
            active_relay_id: "official".to_string(),
            ..BackendSettings::default()
        };

        let value = settings_to_object(&normalize_settings_config_sections(settings));
        let profile = &value["relayProfiles"][0];
        assert_eq!(profile["relayMode"], "official");
        assert_eq!(profile["officialMixApiKey"], false);
        assert_eq!(profile["configContents"], "");
        assert_eq!(profile["authContents"], "");
        assert!(profile.get("model").is_none());
        assert!(profile.get("baseUrl").is_none());
        assert!(profile.get("apiKey").is_none());
    }

    #[test]
    fn official_mix_profile_keeps_key_in_config_not_auth() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));
        let settings = BackendSettings {
            relay_profiles: vec![RelayProfile {
                id: "official-mix".to_string(),
                name: "官方混入".to_string(),
                relay_mode: RelayMode::Official,
                official_mix_api_key: true,
                model: "gpt-5.5".to_string(),
                base_url: "https://relay.example/v1".to_string(),
                api_key: "sk-mix".to_string(),
                config_contents: r#"model = "gpt-5.5"
model_provider = "custom"

[model_providers.custom]
requires_openai_auth = true
base_url = "https://relay.example/v1"
experimental_bearer_token = "sk-mix"
"#
                .to_string(),
                auth_contents: r#"{"OPENAI_API_KEY":"sk-mix","auth_mode":"chatgpt"}"#.to_string(),
                ..RelayProfile::default()
            }],
            active_relay_id: "official-mix".to_string(),
            ..BackendSettings::default()
        };

        store.save(&settings).unwrap();
        let loaded = store.load().unwrap();
        let profile = &loaded.relay_profiles[0];

        assert_eq!(profile.relay_mode, RelayMode::Official);
        assert!(profile.official_mix_api_key);
        assert_eq!(profile.api_key, "sk-mix");
        assert!(!profile.auth_contents.contains("OPENAI_API_KEY"));
        assert!(
            profile
                .config_contents
                .contains(r#"experimental_bearer_token = "sk-mix""#)
        );

        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap())
                .unwrap();
        assert!(saved["relayProfiles"][0].get("apiKey").is_none());
        assert!(
            !saved["relayProfiles"][0]["authContents"]
                .as_str()
                .unwrap()
                .contains("OPENAI_API_KEY")
        );
        assert!(
            saved["relayProfiles"][0]["configContents"]
                .as_str()
                .unwrap()
                .contains(r#"experimental_bearer_token = "sk-mix""#)
        );
    }

    #[test]
    fn settings_update_preserves_official_mix_key_when_payload_loses_it() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));
        store
            .save(&BackendSettings {
                relay_profiles: vec![RelayProfile {
                    id: "official-mix".to_string(),
                    name: "官方混入".to_string(),
                    relay_mode: RelayMode::Official,
                    official_mix_api_key: true,
                    config_contents: r#"model_provider = "custom"

[model_providers.other]
base_url = "https://other.example/v1"
experimental_bearer_token = "sk-other"

[model_providers.custom]
base_url = "https://relay.example/v1"
experimental_bearer_token = "sk-existing"
"#
                    .to_string(),
                    ..RelayProfile::default()
                }],
                active_relay_id: "official-mix".to_string(),
                ..BackendSettings::default()
            })
            .unwrap();

        let updated = store
            .update(json!({
                "relayProfiles": [{
                    "id": "official-mix",
                    "name": "官方混入",
                    "relayMode": "official",
                    "officialMixApiKey": true,
                    "configContents": "model_provider = \"custom\"\n\n[model_providers.other]\nbase_url = \"https://other.example/v1\"\nexperimental_bearer_token = \"sk-other\"\n\n[model_providers.custom]\nbase_url = \"https://relay.example/v1\"\nexperimental_bearer_token = \"\"\n",
                    "authContents": ""
                }],
                "activeRelayId": "official-mix"
            }))
            .unwrap();

        let profile = &updated.relay_profiles[0];
        assert_eq!(profile.api_key, "sk-existing");
        assert!(!profile.config_contents.contains("sk-other"));
        assert!(
            profile
                .config_contents
                .contains(r#"experimental_bearer_token = "sk-existing""#)
        );
        assert!(
            profile
                .config_contents
                .contains(r#"base_url = "https://relay.example/v1""#)
        );
    }

    #[test]
    fn official_mix_update_uses_api_key_when_config_token_missing() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "relayProfiles": [{
                    "id": "official-mix",
                    "name": "官方混入",
                    "relayMode": "official",
                    "officialMixApiKey": true,
                    "baseUrl": "https://relay.example/v1",
                    "apiKey": "sk-new",
                    "configContents": "model_provider = \"custom\"\n\n[model_providers.custom]\nbase_url = \"https://relay.example/v1\"\n",
                    "authContents": ""
                }],
                "activeRelayId": "official-mix"
            }))
            .unwrap();

        let profile = &updated.relay_profiles[0];
        assert_eq!(profile.api_key, "sk-new");
        assert!(
            profile
                .config_contents
                .contains(r#"experimental_bearer_token = "sk-new""#)
        );
        assert!(!profile.auth_contents.contains("OPENAI_API_KEY"));
    }

    #[test]
    fn settings_update_preserves_manual_official_mix_config_token() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "relayProfiles": [{
                    "id": "official-mix",
                    "name": "官方混入",
                    "relayMode": "official",
                    "officialMixApiKey": true,
                    "configContents": "model_provider = \"custom\"\n\n[model_providers.custom]\nbase_url = \"https://relay.example/v1\"\nexperimental_bearer_token = \"22222222222222222222222222222222222\"\n",
                    "authContents": ""
                }],
                "activeRelayId": "official-mix"
            }))
            .unwrap();

        let profile = &updated.relay_profiles[0];
        assert_eq!(profile.relay_mode, RelayMode::Official);
        assert!(profile.official_mix_api_key);
        assert_eq!(profile.api_key, "22222222222222222222222222222222222");
        assert!(
            profile
                .config_contents
                .contains(r#"experimental_bearer_token = "22222222222222222222222222222222222""#)
        );
        assert!(!profile.auth_contents.contains("OPENAI_API_KEY"));
    }

    #[test]
    fn settings_store_load_missing_file_returns_default() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        assert_eq!(store.load().unwrap(), BackendSettings::default());
    }

    #[test]
    fn settings_store_load_bad_json_returns_default() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        std::fs::write(&path, "{bad json").unwrap();
        let store = SettingsStore::new(path);

        assert_eq!(store.load().unwrap(), BackendSettings::default());
    }

    #[test]
    fn settings_store_save_load_roundtrip_uses_custom_path() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("nested").join("settings.json"));
        let settings = BackendSettings {
            provider_sync_enabled: true,
            cli_wrapper_enabled: true,
            cli_wrapper_base_url: "https://example.test".to_string(),
            cli_wrapper_api_key: "sk-test".to_string(),
            cli_wrapper_api_key_env: "CUSTOM_ENV".to_string(),
            codex_extra_args: vec!["--force_high_performance_gpu".to_string()],
            ..BackendSettings::default()
        };

        store.save(&settings).unwrap();

        assert_eq!(store.load().unwrap(), settings);
    }

    #[test]
    fn settings_store_update_only_mutates_present_known_fields() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));
        let initial = BackendSettings {
            provider_sync_enabled: false,
            cli_wrapper_enabled: true,
            cli_wrapper_base_url: "https://old.test".to_string(),
            cli_wrapper_api_key: "old-key".to_string(),
            cli_wrapper_api_key_env: "OLD_ENV".to_string(),
            ..BackendSettings::default()
        };
        store.save(&initial).unwrap();

        let updated = store
            .update(json!({
            "providerSyncEnabled": true,
            "codexAppPath": "C:\\Portable\\Codex\\Codex.exe",
            "enhancementsEnabled": false,
            "codexAppPluginEntryUnlock": false,
            "codexAppSessionDelete": false,
            "codexAppConversationView": true,
            "codexAppServiceTierControls": true,
            "codexGoalsEnabled": true,
            "relayBaseUrl": "https://relay.example.test/v1",
            "relayApiKey": "sk-relay",
            "codexExtraArgs": ["--force_high_performance_gpu", "", "  ", " --enable-gpu "],
            "cliWrapperApiKeyEnv": "",
            "unknownKey": "ignored"
            }))
            .unwrap();

        assert!(updated.provider_sync_enabled);
        assert_eq!(updated.codex_app_path, r"C:\Portable\Codex\Codex.exe");
        assert!(!updated.enhancements_enabled);
        assert!(!updated.codex_app_plugin_entry_unlock);
        assert!(!updated.codex_app_session_delete);
        assert!(updated.codex_app_conversation_view);
        assert!(updated.codex_app_service_tier_controls);
        assert!(updated.codex_goals_enabled);
        assert_eq!(updated.relay_base_url, "https://relay.example.test/v1");
        assert_eq!(updated.relay_api_key, "sk-relay");
        assert_eq!(
            updated.codex_extra_args,
            vec![
                "--force_high_performance_gpu".to_string(),
                "--enable-gpu".to_string(),
            ]
        );
        assert!(updated.cli_wrapper_enabled);
        assert_eq!(updated.cli_wrapper_base_url, "https://old.test");
        assert_eq!(updated.cli_wrapper_api_key, "old-key");
        assert_eq!(updated.cli_wrapper_api_key_env, "CUSTOM_OPENAI_API_KEY");
        assert_eq!(store.load().unwrap(), updated);
    }

    #[test]
    fn settings_store_update_persists_image_overlay_settings() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "codexAppImageOverlayEnabled": true,
                "codexAppImageOverlayPath": "C:\\Users\\me\\Pictures\\overlay.png",
                "codexAppImageOverlayOpacity": 42
            }))
            .unwrap();

        assert!(updated.codex_app_image_overlay_enabled);
        assert_eq!(
            updated.codex_app_image_overlay_path,
            r"C:\Users\me\Pictures\overlay.png"
        );
        assert_eq!(updated.codex_app_image_overlay_opacity, 42);
        assert_eq!(store.load().unwrap(), updated);
    }

    #[test]
    fn settings_store_update_persists_memory_assist_settings() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "memoryAssistEnabled": false,
                "memoryAssistInjectEnabled": false,
                "memoryAssistAutoSuggestEnabled": false,
                "memoryAssistMaxInjectedItems": 8,
                "memoryAssistWorkspaceMode": "global_only"
            }))
            .unwrap();

        assert!(!updated.memory_assist_enabled);
        assert!(!updated.memory_assist_inject_enabled);
        assert!(!updated.memory_assist_auto_suggest_enabled);
        assert_eq!(updated.memory_assist_max_injected_items, 8);
        assert_eq!(updated.memory_assist_workspace_mode, "global_only");
        assert_eq!(store.load().unwrap(), updated);

        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(saved["memoryAssistEnabled"], json!(false));
        assert_eq!(saved["memoryAssistInjectEnabled"], json!(false));
        assert_eq!(saved["memoryAssistAutoSuggestEnabled"], json!(false));
        assert_eq!(saved["memoryAssistMaxInjectedItems"], json!(8));
        assert_eq!(saved["memoryAssistWorkspaceMode"], json!("global_only"));
    }

    #[test]
    fn settings_store_update_persists_launch_mode() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store.update(json!({"launchMode": "relay"})).unwrap();
        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(updated.launch_mode, LaunchMode::Relay);
        assert_eq!(saved["launchMode"], json!("relay"));
    }

    #[test]
    fn settings_store_update_persists_relay_profiles_and_active_profile() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "relayProfiles": [
                    {
                        "id": "relay-a",
                        "name": "中转 A",
                        "baseUrl": "https://relay-a.example/v1",
                        "apiKey": "sk-a"
                    },
                    {
                        "id": "relay-b",
                        "name": "中转 B",
                        "baseUrl": "https://relay-b.example/v1",
                        "apiKey": "sk-b"
                    }
                ],
                "activeRelayId": "relay-b",
                "relayTestModel": "claude-sonnet-4"
            }))
            .unwrap();

        let active = updated.active_relay_profile();
        assert_eq!(updated.relay_profiles.len(), 2);
        assert_eq!(active.id, "relay-b");
        assert_eq!(active.name, "中转 B");
        assert_eq!(updated.relay_test_model, "claude-sonnet-4");

        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap())
                .unwrap();
        assert!(saved["relayProfiles"][1].get("baseUrl").is_none());
        assert!(saved["relayProfiles"][1].get("apiKey").is_none());
    }

    #[test]
    fn settings_store_update_roundtrips_aggregate_supplier_fields() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "relayProfiles": [
                    {
                        "id": "relay-a",
                        "name": "供应商 A",
                        "baseUrl": "https://relay-a.example/v1",
                        "apiKey": "sk-a"
                    },
                    {
                        "id": "aggregate-1",
                        "name": "聚合供应商1",
                        "aggregateEnabled": true,
                        "aggregateStrategy": "failover",
                        "aggregateMembers": ["relay-a"]
                    }
                ]
            }))
            .unwrap();

        let aggregate = updated
            .relay_profiles
            .iter()
            .find(|profile| profile.id == "aggregate-1")
            .expect("aggregate profile");
        assert!(aggregate.aggregate_enabled);
        assert_eq!(aggregate.aggregate_strategy, "failover");
        assert_eq!(aggregate.aggregate_members, vec!["relay-a".to_string()]);

        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(
            saved["relayProfiles"][1]["aggregateStrategy"],
            json!("failover")
        );
        assert_eq!(
            saved["relayProfiles"][1]["aggregateMembers"],
            json!(["relay-a"])
        );
    }

    #[test]
    fn settings_store_update_roundtrips_codex_catalog_json() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));
        let catalog = r#"[{"displayName":"DeepSeek V4 Flash","model":"deepseek-v4-flash","contextWindow":"128000"}]"#;

        store
            .update(json!({
                "relayProfiles": [{
                    "id": "codex-catalog",
                    "name": "Codex Catalog",
                    "targetApp": "codex",
                    "codexCatalogJson": catalog
                }]
            }))
            .unwrap();

        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(
            saved["relayProfiles"][0]["codexCatalogJson"],
            json!(catalog)
        );

        let reloaded = serde_json::to_value(store.load().unwrap()).unwrap();
        assert_eq!(
            reloaded["relayProfiles"][0]["codexCatalogJson"],
            json!(catalog)
        );

        store
            .update(json!({
                "relayProfiles": [{
                    "id": "codex-catalog",
                    "name": "Codex Catalog",
                    "targetApp": "codex",
                    "model": "legacy-model",
                    "modelList": "legacy-model",
                    "codexCatalogJson": "[]"
                }]
            }))
            .unwrap();

        let reloaded = serde_json::to_value(store.load().unwrap()).unwrap();
        assert_eq!(
            reloaded["relayProfiles"][0]["codexCatalogJson"],
            json!("[]")
        );
    }

    #[test]
    fn settings_store_update_saves_editable_supplier_as_single_record() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "relayProfilesEnabled": true,
                "relayProfiles": [{
                    "id": "gpt-plus",
                    "name": "GPT Plus",
                    "relayMode": "pureApi",
                    "protocol": "responses",
                    "model": "gpt-5.5",
                    "baseUrl": "https://api.toporeduce.cn/v1",
                    "apiKey": "sk-test",
                    "configContents": "model = \"gpt-5.5\"\nmodel_provider = \"gpt-plus\"\n\n[model_providers.gpt-plus]\nname = \"gpt-plus\"\nwire_api = \"responses\"\nrequires_openai_auth = true\nbase_url = \"https://api.toporeduce.cn/v1\"\n",
                    "authContents": "{\"OPENAI_API_KEY\":\"sk-test\"}\n"
                }],
                "activeRelayId": "gpt-plus"
            }))
            .unwrap();

        assert_eq!(updated.relay_profiles.len(), 1);
        let profile = &updated.relay_profiles[0];
        assert_eq!(profile.id, "gpt-plus");
        assert_eq!(profile.name, "GPT Plus");
        assert_eq!(profile.base_url, "https://api.toporeduce.cn/v1");
        assert_eq!(profile.api_key, "sk-test");
        assert_eq!(updated.active_relay_id, "gpt-plus");
        assert!(
            profile
                .config_contents
                .contains("model_provider = \"gpt-plus\"")
        );
        assert!(
            profile
                .config_contents
                .contains("[model_providers.gpt-plus]")
        );

        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(saved["relayProfiles"].as_array().unwrap().len(), 1);
        assert_eq!(saved["relayProfiles"][0]["id"], json!("gpt-plus"));
        assert_eq!(saved["activeRelayId"], json!("gpt-plus"));
        assert!(saved["relayProfiles"][0].get("baseUrl").is_none());
        assert!(saved["relayProfiles"][0].get("apiKey").is_none());
        assert!(
            saved["relayProfiles"][0]["configContents"]
                .as_str()
                .unwrap()
                .contains("model_provider = \"gpt-plus\"")
        );
    }

    #[test]
    fn settings_store_update_rewrites_supplier_config_to_edited_id() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "relayProfilesEnabled": true,
                "relayProfiles": [{
                    "id": "gpt-plus",
                    "name": "GPT Plus",
                    "relayMode": "pureApi",
                    "protocol": "responses",
                    "model": "gpt-5.5",
                    "baseUrl": "https://api.toporeduce.cn/v1",
                    "apiKey": "sk-new",
                    "configContents": "model = \"old-model\"\nmodel_provider = \"custom\"\n\n[model_providers.custom]\nname = \"custom\"\nwire_api = \"responses\"\nrequires_openai_auth = true\nbase_url = \"https://old.example/v1\"\n",
                    "authContents": "{\"OPENAI_API_KEY\":\"sk-old\"}\n"
                }],
                "activeRelayId": "gpt-plus"
            }))
            .unwrap();

        assert_eq!(updated.relay_profiles.len(), 1);
        let profile = &updated.relay_profiles[0];
        assert_eq!(profile.id, "gpt-plus");
        assert!(
            profile
                .config_contents
                .contains("model_provider = \"gpt-plus\"")
        );
        assert!(
            profile
                .config_contents
                .contains("[model_providers.gpt-plus]")
        );
        assert!(
            !profile
                .config_contents
                .contains("model_provider = \"custom\"")
        );
        assert!(!profile.config_contents.contains("[model_providers.custom]"));
        assert!(profile.config_contents.contains("model = \"gpt-5.5\""));
        assert!(
            profile
                .config_contents
                .contains("base_url = \"https://api.toporeduce.cn/v1\"")
        );
        assert_eq!(profile.base_url, "https://api.toporeduce.cn/v1");
        assert_eq!(profile.api_key, "sk-new");
        assert!(profile.auth_contents.contains("sk-new"));
        assert_eq!(updated.active_relay_id, "gpt-plus");
    }

    #[test]
    fn settings_store_save_roundtrips_editable_supplier_record() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));
        let settings = BackendSettings {
            relay_profiles_enabled: true,
            active_relay_id: "gpt-plus".to_string(),
            relay_profiles: vec![RelayProfile {
                id: "gpt-plus".to_string(),
                name: "GPT Plus".to_string(),
                relay_mode: RelayMode::PureApi,
                protocol: RelayProtocol::Responses,
                model: "gpt-5.5".to_string(),
                base_url: "https://api.toporeduce.cn/v1".to_string(),
                api_key: "sk-test".to_string(),
                config_contents: "model = \"gpt-5.5\"\nmodel_provider = \"gpt-plus\"\n\n[model_providers.gpt-plus]\nname = \"gpt-plus\"\nwire_api = \"responses\"\nrequires_openai_auth = true\nbase_url = \"https://api.toporeduce.cn/v1\"\n".to_string(),
                auth_contents: "{\"OPENAI_API_KEY\":\"sk-test\"}\n".to_string(),
                ..RelayProfile::default()
            }],
            ..BackendSettings::default()
        };

        store.save(&settings).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.relay_profiles.len(), 1);
        let profile = &loaded.relay_profiles[0];
        assert_eq!(profile.id, "gpt-plus");
        assert_eq!(profile.name, "GPT Plus");
        assert_eq!(profile.base_url, "https://api.toporeduce.cn/v1");
        assert_eq!(profile.api_key, "sk-test");
        assert_eq!(loaded.active_relay_id, "gpt-plus");
        assert!(
            profile
                .config_contents
                .contains("model_provider = \"gpt-plus\"")
        );
        assert!(
            profile
                .config_contents
                .contains("[model_providers.gpt-plus]")
        );
    }

    #[test]
    fn settings_store_update_does_not_persist_relay_profile_derived_fields() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "relayProfiles": [
                    {
                        "id": "relay-a",
                        "name": "供应商 A",
                        "model": "gpt-5.4",
                        "baseUrl": "https://relay.example/v1",
                        "apiKey": "sk-a",
                        "configContents": "model = \"gpt-5.4\"\n",
                        "authContents": "{\"OPENAI_API_KEY\":\"sk-a\"}"
                    }
                ],
                "activeRelayId": "relay-a"
            }))
            .unwrap();

        assert_eq!(updated.relay_profiles[0].id, "relay-a");
        assert_eq!(updated.relay_profiles[0].name, "供应商 A");

        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap())
                .unwrap();
        let saved_profile = &saved["relayProfiles"][0];
        assert!(saved_profile.get("model").is_none());
        assert!(saved_profile.get("baseUrl").is_none());
        assert!(saved_profile.get("apiKey").is_none());
        assert_eq!(saved_profile["configContents"], "model = \"gpt-5.4\"\n");
        assert_eq!(
            saved_profile["authContents"],
            "{\"OPENAI_API_KEY\":\"sk-a\"}"
        );
    }

    #[test]
    fn settings_store_update_moves_context_tables_out_of_common_config() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "relayCommonConfigContents": "[mcp_servers.context7]\ncommand = \"npx\"\n"
            }))
            .unwrap();

        assert!(updated.relay_common_config_contents.is_empty());
        assert_eq!(
            updated.relay_context_config_contents,
            "[mcp_servers.context7]\ncommand = \"npx\"\n"
        );
        assert_eq!(store.load().unwrap(), updated);
    }

    #[test]
    fn settings_store_update_extracts_context_config_from_common_config() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.join("settings.json"));

        let updated = store
            .update(json!({
                "relayCommonConfigContents": "model_reasoning_effort = \"high\"\n\n[mcp_servers.context7]\ncommand = \"npx\"\n\n[plugins.\"superpowers@openai-curated\"]\nenabled = true\n"
            }))
            .unwrap();

        assert_eq!(
            updated.relay_common_config_contents,
            "model_reasoning_effort = \"high\"\n"
        );
        assert!(
            updated
                .relay_context_config_contents
                .contains("[mcp_servers.context7]")
        );
        assert!(
            updated
                .relay_context_config_contents
                .contains("[plugins.\"superpowers@openai-curated\"]")
        );
        assert_eq!(store.load().unwrap(), updated);
    }

    #[test]
    fn active_relay_profile_uses_legacy_single_relay_when_profiles_are_default() {
        let settings = BackendSettings {
            relay_base_url: "https://legacy.example/v1".to_string(),
            relay_api_key: "sk-legacy".to_string(),
            ..BackendSettings::default()
        };

        let active = settings.active_relay_profile();

        assert_eq!(active.id, "default");
        assert_eq!(active.name, "默认中转");
        assert_eq!(active.base_url, "https://legacy.example/v1");
        assert_eq!(active.api_key, "sk-legacy");
        assert_eq!(active.relay_mode, RelayMode::MixedApi);
        assert!(active.official_mix_api_key);
    }

    #[test]
    fn settings_store_update_preserves_existing_unknown_fields() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let store = SettingsStore::new(path.clone());
        std::fs::write(
            &path,
            r#"{"providerSyncEnabled":false,"customField":{"nested":true}}"#,
        )
        .unwrap();

        let updated = store
            .update(json!({
                "providerSyncEnabled": true
            }))
            .unwrap();
        let saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        assert!(updated.provider_sync_enabled);
        assert_eq!(saved["providerSyncEnabled"], json!(true));
        assert_eq!(saved["codexExtraArgs"], Value::Null);
        assert_eq!(saved["customField"], json!({"nested": true}));
    }

    #[test]
    fn settings_store_update_persists_codex_extra_args_and_preserves_unknown_fields() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let store = SettingsStore::new(path.clone());
        std::fs::write(
            &path,
            r#"{"providerSyncEnabled":false,"customField":{"nested":true}}"#,
        )
        .unwrap();

        let updated = store
            .update(json!({
                "codexExtraArgs": ["--force_high_performance_gpu", "--enable-features=UseOzonePlatform"]
            }))
            .unwrap();
        let saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(
            updated.codex_extra_args,
            vec![
                "--force_high_performance_gpu".to_string(),
                "--enable-features=UseOzonePlatform".to_string(),
            ]
        );
        assert_eq!(
            saved["codexExtraArgs"],
            json!([
                "--force_high_performance_gpu",
                "--enable-features=UseOzonePlatform"
            ])
        );
        assert_eq!(saved["customField"], json!({"nested": true}));
    }

    #[test]
    fn settings_store_update_with_non_object_payload_does_not_write_file() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let store = SettingsStore::new(path.clone());
        let original = r#"{"providerSyncEnabled":false,"customField":"keep me"}"#;
        std::fs::write(&path, original).unwrap();

        let updated = store.update(json!(null)).unwrap();

        assert!(!updated.provider_sync_enabled);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn settings_store_update_rejects_malformed_json_without_overwriting_it() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let store = SettingsStore::new(path.clone());
        let original = r#"{"providerSyncEnabled":false"#;
        std::fs::write(&path, original).unwrap();

        let error = store
            .update(json!({"providerSyncEnabled": true}))
            .unwrap_err();

        assert!(error.to_string().contains("failed to parse settings"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn settings_store_update_rejects_non_object_root_without_overwriting_it() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let store = SettingsStore::new(path.clone());
        let original = r#"["keep", "this"]"#;
        std::fs::write(&path, original).unwrap();

        let error = store
            .update(json!({"providerSyncEnabled": true}))
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("settings root must be a JSON object")
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn settings_store_concurrent_updates_preserve_both_changes() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        std::fs::write(&path, r#"{"futureField":"preserved"}"#).unwrap();

        let first = SettingsStore::new(path.clone());
        let second = SettingsStore::new(path.clone());
        let first_update = std::thread::spawn(move || {
            first.update(json!({"providerSyncEnabled": true})).unwrap();
        });
        let second_update = std::thread::spawn(move || {
            second
                .update(json!({"memoryAssistDataDir": "D:/CCP Data"}))
                .unwrap();
        });
        first_update.join().unwrap();
        second_update.join().unwrap();

        let saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["providerSyncEnabled"], true);
        assert_eq!(saved["memoryAssistDataDir"], "D:/CCP Data");
        assert_eq!(saved["futureField"], "preserved");
    }

    #[test]
    fn atomic_write_uses_unique_temp_paths() {
        let path = Path::new("settings.json");
        let first = temp_path_for(path);
        let second = temp_path_for(path);

        assert_ne!(first, second);
        assert!(first.to_string_lossy().ends_with(".tmp"));
        assert!(second.to_string_lossy().ends_with(".tmp"));
    }

    #[test]
    fn settings_file_lock_excludes_another_writer() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let _held = SettingsFileLock::acquire(&path).unwrap();
        let lock_path = settings_lock_path(&path);
        let contender = OpenOptions::new()
            .read(true)
            .write(true)
            .open(lock_path)
            .unwrap();

        assert!(contender.try_lock_exclusive().is_err());
    }

    #[test]
    fn settings_store_save_replaces_an_existing_file() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        std::fs::write(&path, r#"{"providerSyncEnabled":false}"#).unwrap();
        let store = SettingsStore::new(path.clone());
        let settings = BackendSettings {
            provider_sync_enabled: true,
            ..BackendSettings::default()
        };

        store.save(&settings).unwrap();

        let saved: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(saved["providerSyncEnabled"], true);
    }

    #[cfg(windows)]
    #[test]
    fn atomic_write_applies_private_windows_acl() {
        use std::process::Command;

        let dir = temp_dir();
        let path = dir.join("settings.json");
        atomic_write(&path, br#"{"secret":true}"#).unwrap();

        let output = Command::new("icacls.exe").arg(&path).output().unwrap();
        assert!(output.status.success());
        let acl = String::from_utf8_lossy(&output.stdout);
        assert!(
            !acl.contains("(I)"),
            "ACL still contains inherited entries: {acl}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_creates_private_file_and_directories() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir();
        let private_dir = dir.join("private").join("nested");
        let path = private_dir.join("settings.json");

        atomic_write(&path, br#"{"secret":true}"#).unwrap();

        assert_eq!(
            std::fs::metadata(&private_dir)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn memory_assist_data_dir_round_trips_through_incremental_update() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let store = SettingsStore::new(path.clone());
        std::fs::write(&path, r#"{"futureField":"preserved"}"#).unwrap();

        let updated = store
            .update(serde_json::json!({"memoryAssistDataDir": "D:/CCP Data"}))
            .unwrap();

        assert_eq!(updated.memory_assist_data_dir, "D:/CCP Data");
        let raw: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(raw["memoryAssistDataDir"], "D:/CCP Data");
        assert_eq!(raw["futureField"], "preserved");
    }
}
