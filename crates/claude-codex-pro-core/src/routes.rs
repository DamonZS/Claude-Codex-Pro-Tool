use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use rusqlite::Connection;
use serde_json::{Value, json};

use crate::memory_assist::{
    MemoryAssistStore, MemoryCandidateRequest, MemoryCaptureRequest, MemoryItemRequest,
    MemoryQueryRequest, MemorySelfCheckRequest, MemorySessionRequest,
};
use crate::models::{DeleteResult, DeleteStatus, ExportResult, ExportStatus, SessionRef};
use crate::settings::{BackendSettings, SettingsStore};
use crate::status::StatusStore;
use crate::user_scripts::UserScriptManager;

pub type UserScriptEvaluator = Arc<dyn Fn(&str, &str) -> anyhow::Result<Value> + Send + Sync>;
pub type DevtoolsOpener = Arc<dyn Fn(&str) -> anyhow::Result<()> + Send + Sync>;

#[derive(Clone)]
pub struct BridgeContext {
    settings: Arc<dyn BridgeSettingsService>,
    runtime: Arc<dyn BridgeRuntimeService>,
    data: Arc<dyn BridgeDataService>,
}

impl BridgeContext {
    pub fn new(
        settings: Arc<dyn BridgeSettingsService>,
        runtime: Arc<dyn BridgeRuntimeService>,
        data: Arc<dyn BridgeDataService>,
    ) -> Self {
        Self {
            settings,
            runtime,
            data,
        }
    }

    pub fn core(runtime: Arc<dyn BridgeRuntimeService>) -> Self {
        Self::core_with_data(runtime, Arc::new(UnavailableDataService))
    }

    pub fn core_with_data(
        runtime: Arc<dyn BridgeRuntimeService>,
        data: Arc<dyn BridgeDataService>,
    ) -> Self {
        Self::new(Arc::new(CoreSettingsService::default()), runtime, data)
    }

    pub fn core_with_data_and_app_dir(
        runtime: Arc<dyn BridgeRuntimeService>,
        data: Arc<dyn BridgeDataService>,
        app_dir: PathBuf,
    ) -> Self {
        Self::new(
            Arc::new(CoreSettingsService::with_app_dir(app_dir)),
            runtime,
            data,
        )
    }
}

#[async_trait]
pub trait BridgeSettingsService: Send + Sync {
    async fn get_settings(&self) -> anyhow::Result<BackendSettings>;
    async fn set_settings(&self, payload: Value) -> anyhow::Result<BackendSettings>;

    async fn codex_app_version(&self) -> anyhow::Result<String> {
        Ok(String::new())
    }
}

#[async_trait]
pub trait BridgeRuntimeService: Send + Sync {
    async fn user_script_inventory(&self) -> anyhow::Result<Value>;
    async fn set_user_scripts_enabled(&self, enabled: bool) -> anyhow::Result<Value>;
    async fn set_user_script_enabled(&self, key: String, enabled: bool) -> anyhow::Result<Value>;
    async fn delete_user_script(&self, key: String) -> anyhow::Result<Value>;
    async fn reload_user_scripts(&self) -> anyhow::Result<Value>;
    async fn open_devtools(&self) -> anyhow::Result<Value>;
    async fn open_manager(&self) -> anyhow::Result<Value>;
    async fn backend_status(&self) -> anyhow::Result<Value>;
    async fn repair_backend(&self) -> anyhow::Result<Value>;
    async fn claude_desktop_status(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::status_response())
    }
    async fn claude_desktop_integrity(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::integrity_response())
    }
    async fn claude_desktop_focus(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::focus_response())
    }
    async fn claude_desktop_verify(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::verify_response())
    }
    async fn claude_desktop_open_devtools(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::open_devtools_response())
    }
    async fn claude_desktop_open(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::open_response())
    }
    async fn claude_desktop_new_chat(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::new_chat_response())
    }
    async fn claude_desktop_paste_draft(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::draft_response(&payload))
    }
    async fn claude_desktop_submit(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::submit_response(&payload))
    }
    async fn codex_model_catalog(&self) -> anyhow::Result<Value>;
    async fn ads(&self) -> anyhow::Result<Value>;
    async fn zed_remote_status(&self) -> anyhow::Result<Value>;
    async fn resolve_zed_remote_host(&self, payload: Value) -> anyhow::Result<Value>;
    async fn fallback_zed_remote_request(&self, payload: Value) -> anyhow::Result<Value>;
    async fn open_zed_remote(&self, payload: Value) -> anyhow::Result<Value>;
    async fn list_zed_remote_projects(&self, payload: Value) -> anyhow::Result<Value>;
    async fn remember_zed_remote_project(&self, payload: Value) -> anyhow::Result<Value>;
    async fn forget_zed_remote_project(&self, payload: Value) -> anyhow::Result<Value>;
    async fn upstream_worktree_status(&self) -> anyhow::Result<Value>;
    async fn upstream_worktree_defaults(&self, payload: Value) -> anyhow::Result<Value>;
    async fn upstream_worktree_prepare(&self, payload: Value) -> anyhow::Result<Value>;
    async fn upstream_worktree_create(&self, payload: Value) -> anyhow::Result<Value>;
    async fn memory_status(&self) -> anyhow::Result<Value> {
        Ok(json!({"status": "failed", "message": "盘古记忆尚未接线"}))
    }
    async fn memory_session(&self, _payload: Value) -> anyhow::Result<Value> {
        Ok(json!({"status": "failed", "message": "盘古记忆尚未接线"}))
    }
    async fn memory_search(&self, _payload: Value) -> anyhow::Result<Value> {
        Ok(json!({"status": "failed", "message": "盘古记忆尚未接线", "results": []}))
    }
    async fn memory_learn(&self, _payload: Value) -> anyhow::Result<Value> {
        Ok(json!({"status": "failed", "message": "盘古记忆尚未接线"}))
    }
    async fn memory_candidates(&self, _payload: Value) -> anyhow::Result<Value> {
        Ok(json!({"status": "failed", "message": "盘古记忆尚未接线", "candidates": []}))
    }
    async fn memory_capture(&self, _payload: Value) -> anyhow::Result<Value> {
        Ok(json!({"status": "failed", "message": "盘古记忆采集尚未接线"}))
    }
    async fn memory_resolve_workspace(&self, _payload: Value) -> anyhow::Result<Value> {
        Ok(json!({"status": "failed", "message": "盘古记忆 workspace 解析尚未接线"}))
    }
    async fn memory_approve(&self, _payload: Value) -> anyhow::Result<Value> {
        Ok(json!({"status": "failed", "message": "盘古记忆尚未接线"}))
    }
    async fn memory_reject(&self, _payload: Value) -> anyhow::Result<Value> {
        Ok(json!({"status": "failed", "message": "盘古记忆尚未接线"}))
    }
    async fn memory_selfcheck(&self, _payload: Value) -> anyhow::Result<Value> {
        Ok(json!({"status": "failed", "message": "盘古记忆尚未接线"}))
    }
}

#[async_trait]
pub trait BridgeDataService: Send + Sync {
    async fn delete(&self, session: SessionRef) -> anyhow::Result<DeleteResult>;
    async fn undo(&self, undo_token: String) -> anyhow::Result<DeleteResult>;
    async fn export_markdown(&self, session: SessionRef) -> anyhow::Result<ExportResult>;
    async fn thread_usage_history(&self, session: SessionRef) -> anyhow::Result<Value>;
    async fn find_archived_thread_by_title(
        &self,
        title: String,
    ) -> anyhow::Result<Option<SessionRef>>;
    async fn move_thread_workspace(
        &self,
        session: SessionRef,
        target_cwd: String,
    ) -> anyhow::Result<Value>;
    async fn thread_sort_key(&self, session: SessionRef) -> anyhow::Result<Value>;
    async fn thread_sort_keys(&self, sessions: Vec<SessionRef>) -> anyhow::Result<Value>;
}

pub async fn handle_bridge_request(
    ctx: BridgeContext,
    path: &str,
    payload: Value,
) -> serde_json::Value {
    let started = Instant::now();
    let _ = crate::diagnostic_log::append_diagnostic_log(
        "bridge.request",
        json!({
            "path": path,
            "payload_keys": payload
                .as_object()
                .map(|object| object.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        }),
    );
    let result = match path {
        "/settings/get" => settings_value(&ctx, ctx.settings.get_settings().await).await,
        "/settings/set" => {
            settings_value(&ctx, ctx.settings.set_settings(payload.clone()).await).await
        }
        "/user-scripts/list" => ctx.runtime.user_script_inventory().await,
        "/user-scripts/set-enabled" => {
            let enabled = payload
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            ctx.runtime.set_user_scripts_enabled(enabled).await
        }
        "/user-scripts/set-script-enabled" => {
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let enabled = payload
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            ctx.runtime.set_user_script_enabled(key, enabled).await
        }
        "/user-scripts/delete" => {
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            ctx.runtime.delete_user_script(key).await
        }
        "/user-scripts/reload" => ctx.runtime.reload_user_scripts().await,
        "/devtools/open" => ctx.runtime.open_devtools().await,
        "/manager/open" => ctx.runtime.open_manager().await,
        "/backend/status" => ctx.runtime.backend_status().await,
        "/backend/repair" => ctx.runtime.repair_backend().await,
        "/claude-desktop/status" => ctx.runtime.claude_desktop_status().await,
        "/claude-desktop/integrity" => ctx.runtime.claude_desktop_integrity().await,
        "/claude-desktop/focus" => ctx.runtime.claude_desktop_focus().await,
        "/claude-desktop/verify" => ctx.runtime.claude_desktop_verify().await,
        "/claude-desktop/open-devtools" => ctx.runtime.claude_desktop_open_devtools().await,
        "/claude-desktop/open" => ctx.runtime.claude_desktop_open().await,
        "/claude-desktop/new-chat" => ctx.runtime.claude_desktop_new_chat().await,
        "/claude-desktop/paste-draft" => {
            ctx.runtime
                .claude_desktop_paste_draft(payload.clone())
                .await
        }
        "/claude-desktop/submit" => ctx.runtime.claude_desktop_submit(payload.clone()).await,
        "/codex-model-catalog" | "/codex-config-model" => ctx.runtime.codex_model_catalog().await,
        "/diagnostics/log" => diagnostic_log_value(payload.clone()),
        "/ads" => ctx.runtime.ads().await,
        "/zed-remote/status" => ctx.runtime.zed_remote_status().await,
        "/zed-remote/resolve-host" => ctx.runtime.resolve_zed_remote_host(payload.clone()).await,
        "/zed-remote/fallback-request" => {
            ctx.runtime
                .fallback_zed_remote_request(payload.clone())
                .await
        }
        "/zed-remote/open" => ctx.runtime.open_zed_remote(payload.clone()).await,
        "/zed-remote/projects" => ctx.runtime.list_zed_remote_projects(payload.clone()).await,
        "/zed-remote/remember-project" => {
            ctx.runtime
                .remember_zed_remote_project(payload.clone())
                .await
        }
        "/zed-remote/forget-project" => {
            ctx.runtime.forget_zed_remote_project(payload.clone()).await
        }
        "/upstream-worktree/status" => ctx.runtime.upstream_worktree_status().await,
        "/upstream-worktree/defaults" => {
            ctx.runtime
                .upstream_worktree_defaults(payload.clone())
                .await
        }
        "/upstream-worktree/prepare" => {
            ctx.runtime.upstream_worktree_prepare(payload.clone()).await
        }
        "/upstream-worktree/create" => ctx.runtime.upstream_worktree_create(payload.clone()).await,
        "/memory/status" => ctx.runtime.memory_status().await,
        "/memory/session" => match ensure_memory_enabled(&ctx).await {
            Ok(()) => ctx.runtime.memory_session(payload.clone()).await,
            Err(err) => Err(err),
        },
        "/memory/search" => match ensure_memory_enabled(&ctx).await {
            Ok(()) => ctx.runtime.memory_search(payload.clone()).await,
            Err(err) => Err(err),
        },
        "/memory/learn" => match ensure_memory_enabled(&ctx).await {
            Ok(()) => ctx.runtime.memory_learn(payload.clone()).await,
            Err(err) => Err(err),
        },
        "/memory/candidates" => match ensure_memory_candidates_allowed(&ctx, &payload).await {
            Ok(()) => ctx.runtime.memory_candidates(payload.clone()).await,
            Err(err) => Err(err),
        },
        "/memory/capture" => match ensure_memory_enabled(&ctx).await {
            Ok(()) => ctx.runtime.memory_capture(payload.clone()).await,
            Err(err) => Err(err),
        },
        "/memory/resolve-workspace" => match ensure_memory_enabled(&ctx).await {
            Ok(()) => ctx.runtime.memory_resolve_workspace(payload.clone()).await,
            Err(err) => Err(err),
        },
        "/memory/approve" => match ensure_memory_enabled(&ctx).await {
            Ok(()) => ctx.runtime.memory_approve(payload.clone()).await,
            Err(err) => Err(err),
        },
        "/memory/reject" => match ensure_memory_enabled(&ctx).await {
            Ok(()) => ctx.runtime.memory_reject(payload.clone()).await,
            Err(err) => Err(err),
        },
        "/memory/selfcheck" => match ensure_memory_enabled(&ctx).await {
            Ok(()) => ctx.runtime.memory_selfcheck(payload.clone()).await,
            Err(err) => Err(err),
        },
        "/delete" => result_value(ctx.data.delete(session_from_payload(&payload)).await),
        "/undo" => {
            let undo_token = payload
                .get("undo_token")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            result_value(ctx.data.undo(undo_token).await)
        }
        "/export-markdown" => result_value(
            ctx.data
                .export_markdown(session_from_payload(&payload))
                .await,
        ),
        "/thread-usage-history" => {
            ctx.data
                .thread_usage_history(session_from_payload(&payload))
                .await
        }
        "/archived-thread" => {
            let title = payload
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            archived_thread_value(ctx.data.find_archived_thread_by_title(title).await)
        }
        "/move-thread-workspace" => {
            let target_cwd = payload
                .get("target_cwd")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            ctx.data
                .move_thread_workspace(session_from_payload(&payload), target_cwd)
                .await
        }
        "/thread-sort-key" => {
            ctx.data
                .thread_sort_key(session_from_payload(&payload))
                .await
        }
        "/thread-sort-keys" => {
            ctx.data
                .thread_sort_keys(sessions_from_payload(&payload))
                .await
        }
        _ => {
            let _ = crate::diagnostic_log::append_diagnostic_log(
                "bridge.unknown_path",
                json!({
                    "path": path
                }),
            );
            return json!({
                "status": "failed",
                "session_id": "",
                "message": "Unknown bridge path"
            });
        }
    };

    let response = result.unwrap_or_else(|error| failed_from_error(&payload, error));
    let _ = crate::diagnostic_log::append_diagnostic_log(
        "bridge.response",
        json!({
            "path": path,
            "elapsed_ms": started.elapsed().as_millis() as u64,
            "status": response.get("status").and_then(Value::as_str).unwrap_or("")
        }),
    );
    response
}

async fn ensure_memory_enabled(ctx: &BridgeContext) -> anyhow::Result<()> {
    let settings = ctx.settings.get_settings().await?;
    if settings.memory_assist_enabled {
        Ok(())
    } else {
        anyhow::bail!("盘古记忆已禁用")
    }
}

async fn ensure_memory_candidates_allowed(
    ctx: &BridgeContext,
    payload: &Value,
) -> anyhow::Result<()> {
    ensure_memory_enabled(ctx).await?;
    let creates_candidate = payload
        .get("text")
        .and_then(Value::as_str)
        .map(|text| !text.trim().is_empty())
        .unwrap_or(false);
    if !creates_candidate {
        return Ok(());
    }
    let settings = ctx.settings.get_settings().await?;
    if settings.memory_assist_auto_suggest_enabled {
        Ok(())
    } else {
        anyhow::bail!("盘古记忆自动学习已禁用")
    }
}

#[derive(Default)]
pub struct CoreSettingsService {
    store: SettingsStore,
    app_dir: Option<PathBuf>,
}

impl CoreSettingsService {
    fn with_app_dir(app_dir: PathBuf) -> Self {
        Self {
            store: SettingsStore::default(),
            app_dir: Some(app_dir),
        }
    }
}

#[async_trait]
impl BridgeSettingsService for CoreSettingsService {
    async fn get_settings(&self) -> anyhow::Result<BackendSettings> {
        self.store.load()
    }

    async fn set_settings(&self, payload: Value) -> anyhow::Result<BackendSettings> {
        self.store.update(payload)
    }

    async fn codex_app_version(&self) -> anyhow::Result<String> {
        if let Some(app_dir) = self.app_dir.as_deref() {
            return Ok(crate::app_paths::codex_app_version(app_dir).unwrap_or_default());
        }
        let settings = self.store.load().unwrap_or_default();
        let app_dir = crate::app_paths::resolve_codex_app_dir_with_saved(
            None,
            Some(settings.codex_app_path.as_str()),
        );
        Ok(app_dir
            .as_deref()
            .and_then(crate::app_paths::codex_app_version)
            .unwrap_or_default())
    }
}

#[derive(Clone)]
pub struct CoreRuntimeService {
    debug_port: u16,
    status_store: StatusStore,
    user_scripts: Option<UserScriptManager>,
    websocket_url: Option<String>,
    user_script_evaluator: Option<UserScriptEvaluator>,
    devtools_opener: Option<DevtoolsOpener>,
    devtools_target_id: Option<String>,
    memory_store: MemoryAssistStore,
}

impl CoreRuntimeService {
    pub fn new(debug_port: u16, status_store: StatusStore) -> Self {
        Self {
            debug_port,
            status_store,
            user_scripts: None,
            websocket_url: None,
            user_script_evaluator: None,
            devtools_opener: None,
            devtools_target_id: None,
            memory_store: MemoryAssistStore::default(),
        }
    }

    pub fn with_user_scripts(mut self, user_scripts: UserScriptManager) -> Self {
        self.user_scripts = Some(user_scripts);
        self
    }

    pub fn with_websocket_url(mut self, websocket_url: impl Into<String>) -> Self {
        self.websocket_url = Some(websocket_url.into());
        self
    }

    pub fn with_user_script_evaluator(mut self, evaluator: UserScriptEvaluator) -> Self {
        self.user_script_evaluator = Some(evaluator);
        self
    }

    pub fn with_devtools_opener(mut self, opener: DevtoolsOpener) -> Self {
        self.devtools_opener = Some(opener);
        self
    }

    pub fn with_devtools_target_id(mut self, target_id: impl Into<String>) -> Self {
        self.devtools_target_id = Some(target_id.into());
        self
    }

    pub fn with_memory_store(mut self, memory_store: MemoryAssistStore) -> Self {
        self.memory_store = memory_store;
        self
    }
}

#[async_trait]
impl BridgeRuntimeService for CoreRuntimeService {
    async fn user_script_inventory(&self) -> anyhow::Result<Value> {
        match &self.user_scripts {
            Some(user_scripts) => user_scripts.inventory(),
            None => Ok(empty_user_script_inventory()),
        }
    }

    async fn set_user_scripts_enabled(&self, enabled: bool) -> anyhow::Result<Value> {
        match &self.user_scripts {
            Some(user_scripts) => {
                user_scripts.set_global_enabled(enabled)?;
                user_scripts.inventory()
            }
            None => {
                let mut inventory = empty_user_script_inventory();
                inventory["enabled"] = json!(enabled);
                Ok(inventory)
            }
        }
    }

    async fn set_user_script_enabled(&self, key: String, enabled: bool) -> anyhow::Result<Value> {
        match &self.user_scripts {
            Some(user_scripts) => {
                user_scripts.set_script_enabled(&key, enabled)?;
                user_scripts.inventory()
            }
            None => Ok(empty_user_script_inventory()),
        }
    }

    async fn delete_user_script(&self, key: String) -> anyhow::Result<Value> {
        match &self.user_scripts {
            Some(user_scripts) => {
                user_scripts.delete_user_script(&key)?;
                user_scripts.inventory()
            }
            None => Ok(empty_user_script_inventory()),
        }
    }

    async fn reload_user_scripts(&self) -> anyhow::Result<Value> {
        if let (Some(user_scripts), Some(websocket_url), Some(evaluator)) = (
            &self.user_scripts,
            self.websocket_url.as_deref(),
            &self.user_script_evaluator,
        ) {
            let bundle = user_scripts.build_enabled_bundle()?;
            if !bundle.trim().is_empty() {
                evaluator(websocket_url, &bundle)?;
            }
        }
        self.user_script_inventory().await
    }

    async fn open_devtools(&self) -> anyhow::Result<Value> {
        let target_id = self
            .devtools_target_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("No DevTools target configured"))?;
        let url = devtools_url(self.debug_port, target_id);
        if let Some(opener) = &self.devtools_opener {
            opener(&url)?;
        }
        Ok(json!({
            "status": "ok",
            "target_id": target_id,
            "url": url
        }))
    }

    async fn open_manager(&self) -> anyhow::Result<Value> {
        let manager_path = manager_exe_path();
        if !manager_path.exists() {
            anyhow::bail!("未找到管理工具：{}", manager_path.display());
        }
        spawn_manager(&manager_path)?;
        Ok(json!({
            "status": "ok",
            "path": manager_path.to_string_lossy()
        }))
    }

    async fn backend_status(&self) -> anyhow::Result<Value> {
        let _ = self.status_store.load_latest();
        let _ = crate::diagnostic_log::append_diagnostic_log(
            "bridge.backend_status_ok",
            json!({
                "debug_port": self.debug_port,
                "version": crate::version::VERSION
            }),
        );
        Ok(json!({"status": "ok", "message": "后端已连接", "version": crate::version::VERSION}))
    }

    async fn repair_backend(&self) -> anyhow::Result<Value> {
        self.backend_status().await
    }

    async fn claude_desktop_status(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::status_response())
    }

    async fn claude_desktop_integrity(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::integrity_response())
    }

    async fn claude_desktop_focus(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::focus_response())
    }

    async fn claude_desktop_verify(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::verify_response())
    }

    async fn claude_desktop_open_devtools(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::open_devtools_response())
    }

    async fn claude_desktop_open(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::open_response())
    }

    async fn claude_desktop_new_chat(&self) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::new_chat_response())
    }

    async fn claude_desktop_paste_draft(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::draft_response(&payload))
    }

    async fn claude_desktop_submit(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::claude_desktop::submit_response(&payload))
    }

    async fn codex_model_catalog(&self) -> anyhow::Result<Value> {
        Ok(crate::model_catalog::read_codex_model_catalog().await)
    }

    async fn ads(&self) -> anyhow::Result<Value> {
        crate::ads::fetch_ad_list().await
    }

    async fn zed_remote_status(&self) -> anyhow::Result<Value> {
        Ok(crate::zed_remote::zed_remote_status())
    }

    async fn resolve_zed_remote_host(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::zed_remote::resolve_ssh_target_response(&payload))
    }

    async fn fallback_zed_remote_request(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::zed_remote::fallback_open_request_response(&payload))
    }

    async fn open_zed_remote(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::zed_remote::open_zed_remote(&payload))
    }

    async fn list_zed_remote_projects(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::zed_remote::list_zed_remote_projects_response(
            &payload,
        ))
    }

    async fn remember_zed_remote_project(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::zed_remote::remember_zed_remote_project_response(
            &payload,
        ))
    }

    async fn forget_zed_remote_project(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::zed_remote::forget_zed_remote_project_response(
            &payload,
        ))
    }

    async fn upstream_worktree_status(&self) -> anyhow::Result<Value> {
        Ok(crate::upstream_worktree::status_response())
    }

    async fn upstream_worktree_defaults(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::upstream_worktree::defaults_response(&payload))
    }

    async fn upstream_worktree_prepare(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::upstream_worktree::prepare_response(&payload))
    }

    async fn upstream_worktree_create(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(crate::upstream_worktree::create_response(&payload))
    }

    async fn memory_status(&self) -> anyhow::Result<Value> {
        let mut value = serde_json::to_value(self.memory_store.status()?)?;
        value["status"] = json!("ok");
        Ok(value)
    }

    async fn memory_session(&self, payload: Value) -> anyhow::Result<Value> {
        let request: MemorySessionRequest =
            serde_json::from_value(payload).unwrap_or(MemorySessionRequest {
                workspace: String::new(),
                query: String::new(),
                max_items: 5,
            });
        let mut value = serde_json::to_value(self.memory_store.session_summary(request)?)?;
        value["status"] = json!("ok");
        Ok(value)
    }

    async fn memory_search(&self, payload: Value) -> anyhow::Result<Value> {
        let request: MemoryQueryRequest = serde_json::from_value(payload)?;
        let mut value = serde_json::to_value(self.memory_store.query(request)?)?;
        value["status"] = json!("ok");
        Ok(value)
    }

    async fn memory_learn(&self, payload: Value) -> anyhow::Result<Value> {
        let request: MemoryItemRequest = serde_json::from_value(payload)?;
        let mut value = serde_json::to_value(self.memory_store.learn_item(request)?)?;
        value["status"] = json!("ok");
        Ok(value)
    }

    async fn memory_candidates(&self, payload: Value) -> anyhow::Result<Value> {
        if payload
            .get("text")
            .and_then(Value::as_str)
            .map(|text| !text.trim().is_empty())
            .unwrap_or(false)
        {
            let request: MemoryCandidateRequest = serde_json::from_value(payload)?;
            let mut value = serde_json::to_value(self.memory_store.create_candidate(request)?)?;
            value["status"] = json!("ok");
            return Ok(value);
        }
        let workspace = payload
            .get("workspace")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let include_global = payload
            .get("includeGlobal")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        Ok(json!({
            "status": "ok",
            "candidates": self.memory_store.list_candidates(workspace, include_global)?
        }))
    }

    async fn memory_capture(&self, payload: Value) -> anyhow::Result<Value> {
        let request: MemoryCaptureRequest = serde_json::from_value(payload)?;
        let mut value = serde_json::to_value(self.memory_store.record_capture(request)?)?;
        value["status"] = json!("ok");
        Ok(value)
    }

    async fn memory_resolve_workspace(&self, payload: Value) -> anyhow::Result<Value> {
        Ok(resolve_codex_memory_workspace_response(&payload))
    }

    async fn memory_approve(&self, payload: Value) -> anyhow::Result<Value> {
        let id = payload
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let mut value = serde_json::to_value(self.memory_store.approve_candidate(id)?)?;
        value["status"] = json!("ok");
        Ok(value)
    }

    async fn memory_reject(&self, payload: Value) -> anyhow::Result<Value> {
        let id = payload
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let mut value = serde_json::to_value(self.memory_store.reject_candidate(id)?)?;
        value["status"] = json!("ok");
        Ok(value)
    }

    async fn memory_selfcheck(&self, payload: Value) -> anyhow::Result<Value> {
        let request: MemorySelfCheckRequest =
            serde_json::from_value(payload).unwrap_or(MemorySelfCheckRequest { repair: false });
        let mut value = serde_json::to_value(self.memory_store.run_selfcheck(request)?)?;
        value["status"] = json!("ok");
        Ok(value)
    }
}

pub fn resolve_codex_memory_workspace_response(payload: &Value) -> Value {
    let current_workspace = payload
        .get("workspace")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if !current_workspace.is_empty() && !current_workspace.starts_with("codex:path:") {
        return json!({
            "status": "ok",
            "resolved": false,
            "workspace": current_workspace,
            "source": "already_stable"
        });
    }

    let project_label = payload
        .get("projectLabel")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let thread_title = payload
        .get("threadTitle")
        .or_else(|| payload.get("title"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let url = payload
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match resolve_codex_workspace_from_local_sessions(project_label, thread_title, url) {
        Some((workspace, source)) => json!({
            "status": "ok",
            "resolved": true,
            "workspace": workspace,
            "source": source
        }),
        None => json!({
            "status": "ok",
            "resolved": false,
            "workspace": current_workspace,
            "source": "unresolved"
        }),
    }
}

fn resolve_codex_workspace_from_local_sessions(
    project_label: &str,
    thread_title: &str,
    url: &str,
) -> Option<(String, String)> {
    let project_label = normalize_match_text(project_label);
    let thread_title = normalize_match_text(thread_title);
    let thread_id = extract_uuidish(url);
    let codex_home = crate::codex_sqlite::default_codex_home_dir();
    for db_path in crate::codex_sqlite::codex_session_db_paths_from_home(&codex_home) {
        if !db_path.is_file() {
            continue;
        }
        let Ok(db) =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        else {
            continue;
        };
        if let Some(workspace) =
            resolve_workspace_from_threads(&db, &project_label, &thread_title, &thread_id)
        {
            return Some((workspace, "codex_threads".to_string()));
        }
        if let Some(workspace) = resolve_workspace_from_local_catalog(&db, &project_label) {
            return Some((workspace, "codex_local_thread_catalog".to_string()));
        }
    }
    None
}

fn resolve_workspace_from_threads(
    db: &Connection,
    project_label: &str,
    thread_title: &str,
    thread_id: &str,
) -> Option<String> {
    if !sqlite_table_has_columns(db, "threads", &["id", "cwd"]).ok()? {
        return None;
    }
    let columns = sqlite_columns(db, "threads").ok()?;
    let title_expr = if columns.iter().any(|column| column == "title") {
        "title"
    } else {
        "''"
    };
    let updated = if columns.iter().any(|column| column == "updated_at_ms") {
        "updated_at_ms"
    } else if columns.iter().any(|column| column == "updated_at") {
        "updated_at * 1000"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT id, {title_expr}, cwd FROM threads
         WHERE COALESCE(cwd, '') <> ''
         ORDER BY COALESCE({updated}, 0) DESC, id DESC
         LIMIT 500"
    );
    let mut stmt = db.prepare(&sql).ok()?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            ))
        })
        .ok()?;
    let mut fallback_by_label = None;
    for row in rows.flatten() {
        let (id, title, cwd) = row;
        let cwd = cwd.trim().to_string();
        if cwd.is_empty() {
            continue;
        }
        let cwd_label = normalize_match_text(
            Path::new(&cwd)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&cwd),
        );
        let normalized_title = normalize_match_text(&title);
        if !thread_id.is_empty() && normalize_match_text(&id).contains(thread_id) {
            return Some(cwd);
        }
        if !thread_title.is_empty()
            && (!normalized_title.is_empty()
                && (normalized_title.contains(&thread_title)
                    || thread_title.contains(&normalized_title)))
        {
            return Some(cwd);
        }
        if !project_label.is_empty()
            && !cwd_label.is_empty()
            && (cwd_label == project_label
                || cwd_label.contains(project_label)
                || project_label.contains(&cwd_label))
        {
            fallback_by_label.get_or_insert(cwd);
        }
    }
    fallback_by_label
}

fn resolve_workspace_from_local_catalog(db: &Connection, project_label: &str) -> Option<String> {
    if project_label.is_empty()
        || !sqlite_table_has_columns(db, "local_thread_catalog", &["path"]).ok()?
    {
        return None;
    }
    let mut stmt = db
        .prepare("SELECT path FROM local_thread_catalog WHERE COALESCE(path, '') <> '' LIMIT 500")
        .ok()?;
    let rows = stmt
        .query_map([], |row| row.get::<_, Option<String>>(0))
        .ok()?;
    for path in rows.flatten().flatten() {
        let label = normalize_match_text(
            Path::new(&path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&path),
        );
        if label == project_label || label.contains(project_label) || project_label.contains(&label)
        {
            return Some(path);
        }
    }
    None
}

fn sqlite_table_has_columns(
    db: &Connection,
    table: &str,
    required: &[&str],
) -> rusqlite::Result<bool> {
    let columns = sqlite_columns(db, table)?;
    Ok(required
        .iter()
        .all(|required| columns.iter().any(|column| column == required)))
}

fn sqlite_columns(db: &Connection, table: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = db.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    rows.collect()
}

fn normalize_match_text(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn extract_uuidish(text: &str) -> String {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-'))
        .find(|part| part.len() >= 16 && part.contains('-'))
        .map(normalize_match_text)
        .unwrap_or_default()
}

struct UnavailableDataService;

#[async_trait]
impl BridgeDataService for UnavailableDataService {
    async fn delete(&self, session: SessionRef) -> anyhow::Result<DeleteResult> {
        Ok(DeleteResult {
            status: DeleteStatus::Failed,
            session_id: session.session_id,
            message: "Delete service is not wired in core launcher hooks".to_string(),
            undo_token: None,
            backup_path: None,
        })
    }

    async fn undo(&self, undo_token: String) -> anyhow::Result<DeleteResult> {
        Ok(DeleteResult {
            status: DeleteStatus::Failed,
            session_id: String::new(),
            message: "Undo service is not wired in core launcher hooks".to_string(),
            undo_token: Some(undo_token),
            backup_path: None,
        })
    }

    async fn export_markdown(&self, session: SessionRef) -> anyhow::Result<ExportResult> {
        Ok(ExportResult {
            status: ExportStatus::Failed,
            session_id: session.session_id,
            message: "Markdown export service is not wired in core launcher hooks".to_string(),
            filename: None,
            markdown: None,
        })
    }

    async fn thread_usage_history(&self, session: SessionRef) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "failed",
            "session_id": session.session_id,
            "message": "Thread usage history service is not wired in core launcher hooks",
            "history": []
        }))
    }

    async fn find_archived_thread_by_title(
        &self,
        _title: String,
    ) -> anyhow::Result<Option<SessionRef>> {
        Ok(None)
    }

    async fn move_thread_workspace(
        &self,
        session: SessionRef,
        _target_cwd: String,
    ) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "failed",
            "session_id": session.session_id,
            "message": "Move workspace service is not wired in core launcher hooks"
        }))
    }

    async fn thread_sort_key(&self, session: SessionRef) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "failed",
            "session_id": session.session_id,
            "message": "Thread sort service is not wired in core launcher hooks"
        }))
    }

    async fn thread_sort_keys(&self, _sessions: Vec<SessionRef>) -> anyhow::Result<Value> {
        Ok(json!({
            "status": "failed",
            "message": "Thread sort service is not wired in core launcher hooks",
            "sort_keys": []
        }))
    }
}

fn manager_exe_path() -> PathBuf {
    crate::install::option_or_current_exe(&None, crate::install::MANAGER_BINARY)
}

fn spawn_manager(manager_path: &Path) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(manager_path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::windows_create_no_window());
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| anyhow::anyhow!("启动管理工具失败：{error}"))
}

fn settings_payload_value(
    settings: BackendSettings,
    codex_app_version: String,
) -> anyhow::Result<Value> {
    let mut value = serde_json::to_value(settings)?;
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "codexAppVersion".to_string(),
            Value::String(codex_app_version),
        );
    }
    Ok(value)
}

async fn settings_value(
    ctx: &BridgeContext,
    result: anyhow::Result<BackendSettings>,
) -> anyhow::Result<Value> {
    let settings = result?;
    let codex_app_version = ctx.settings.codex_app_version().await.unwrap_or_default();
    settings_payload_value(settings, codex_app_version)
}

fn result_value<T>(result: anyhow::Result<T>) -> anyhow::Result<Value>
where
    T: serde::Serialize,
{
    Ok(serde_json::to_value(result?)?)
}

fn diagnostic_log_value(payload: Value) -> anyhow::Result<Value> {
    let event = payload
        .get("event")
        .and_then(Value::as_str)
        .map(sanitize_diagnostic_event)
        .unwrap_or_else(|| "event".to_string());
    crate::diagnostic_log::append_diagnostic_log(&format!("renderer.{event}"), payload)?;
    Ok(json!({
        "status": "ok",
        "message": "日志已记录"
    }))
}

fn sanitize_diagnostic_event(event: &str) -> String {
    let sanitized = event
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "event".to_string()
    } else {
        sanitized
    }
}

fn archived_thread_value(result: anyhow::Result<Option<SessionRef>>) -> anyhow::Result<Value> {
    Ok(match result? {
        Some(session) => json!({"session_id": session.session_id, "title": session.title}),
        None => json!({"session_id": "", "title": ""}),
    })
}

fn failed_from_error(payload: &Value, error: anyhow::Error) -> Value {
    json!({
        "status": "failed",
        "session_id": payload
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "message": error.to_string()
    })
}

fn session_from_payload(payload: &Value) -> SessionRef {
    SessionRef {
        session_id: payload
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: payload
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    }
}

fn sessions_from_payload(payload: &Value) -> Vec<SessionRef> {
    payload
        .get("sessions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_object())
                .map(|item| SessionRef {
                    session_id: item
                        .get("session_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    title: item
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn devtools_url(debug_port: u16, target_id: &str) -> String {
    format!(
        "http://127.0.0.1:{debug_port}/devtools/inspector.html?ws=127.0.0.1:{debug_port}/devtools/page/{target_id}"
    )
}

fn empty_user_script_inventory() -> Value {
    json!({
        "enabled": true,
        "scripts": []
    })
}
