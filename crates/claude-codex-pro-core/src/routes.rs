use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::codex_execution::{
    CodexExecutionHandle, CodexExecutionService, CodexExecutionState, CodexExecutionStatus,
    CodexThreadRequest,
};
use crate::memory_assist::{
    MemoryAssistStore, MemoryCandidateRequest, MemoryCaptureRequest, MemoryItemRequest,
    MemoryQueryRequest, MemorySelfCheckRequest, MemorySessionRequest,
};
use crate::models::{DeleteResult, DeleteStatus, ExportResult, ExportStatus, SessionRef};
use crate::multica_execution::{
    SkillBindingScope, SkillBindingSelection, SkillBindings, SkillReference, SkillResolutionAudit,
};
use crate::multica_execution_store::{
    CodexMulticaExecutionBinding, ExecutionReservation, MulticaExecutionBindingState,
    MulticaExecutionCommandKind, MulticaExecutionCommandState, MulticaExecutionKind,
    MulticaExecutionStore,
};
use crate::multica_skill_trust::review_local_skill;
use crate::multica_workspace::{
    LocalMulticaWorkspaceStore, LocalWorkspaceEntityDelete, LocalWorkspaceEntityUpsert,
    MulticaSkillBindingCommand, MulticaSkillBindingRemoveCommand, MulticaSkillBindingsQuery,
    MulticaWorkspaceQuery, MulticaWorkspaceResourceKey,
};
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
    async fn multica_workspace_bootstrap(&self) -> anyhow::Result<Value> {
        anyhow::bail!("multica_workspace_unavailable")
    }
    async fn multica_workspace_query(
        &self,
        _query: MulticaWorkspaceQuery,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_workspace_unavailable")
    }
    async fn multica_workspace_upsert(
        &self,
        _request: MulticaWorkspaceUpsertRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_workspace_mutation_unavailable")
    }
    async fn multica_workspace_delete(
        &self,
        _request: MulticaWorkspaceDeleteRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_workspace_mutation_unavailable")
    }
    async fn multica_skill_resolve(
        &self,
        _selection: SkillBindingSelection,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_skill_resolution_unavailable")
    }
    async fn multica_skill_review(
        &self,
        _request: MulticaSkillReviewRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_skill_review_unavailable")
    }
    async fn multica_skill_bind(
        &self,
        _request: MulticaSkillBindingRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_skill_binding_unavailable")
    }
    async fn multica_skill_unbind(
        &self,
        _request: MulticaSkillBindingRemoveRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_skill_binding_unavailable")
    }
    async fn multica_skill_bindings(
        &self,
        _request: MulticaSkillBindingsQueryRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_skill_binding_unavailable")
    }
    async fn multica_execution_create(
        &self,
        _request: MulticaExecutionCreateRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_unavailable")
    }
    async fn multica_execution_open(
        &self,
        _request: MulticaExecutionBindingRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_unavailable")
    }
    async fn multica_execution_continue(
        &self,
        _request: MulticaExecutionContinueRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_unavailable")
    }
    async fn multica_execution_cancel(
        &self,
        _request: MulticaExecutionCancelRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_unavailable")
    }
    async fn multica_execution_status(
        &self,
        _request: MulticaExecutionBindingRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_unavailable")
    }
    async fn multica_execution_list(
        &self,
        _request: MulticaExecutionListRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_unavailable")
    }
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
        "/multica/workspace/bootstrap" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                parse_empty_multica_payload(&payload)?;
                ctx.runtime.multica_workspace_bootstrap().await
            }
            .await
        }
        "/multica/workspace/query" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_workspace_query(parse_multica_workspace_query(&payload)?)
                    .await
            }
            .await
        }
        "/multica/workspace/upsert" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_workspace_upsert(parse_multica_workspace_upsert(&payload)?)
                    .await
            }
            .await
        }
        "/multica/workspace/delete" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_workspace_delete(parse_multica_workspace_delete(&payload)?)
                    .await
            }
            .await
        }
        "/multica/skills/resolve" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_skill_resolve(parse_multica_skill_selection(&payload)?)
                    .await
            }
            .await
        }
        "/multica/skills/review" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_skill_review(parse_multica_skill_review(&payload)?)
                    .await
            }
            .await
        }
        "/multica/skills/bind" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_skill_bind(parse_multica_skill_binding(&payload)?)
                    .await
            }
            .await
        }
        "/multica/skills/unbind" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_skill_unbind(parse_multica_skill_binding_remove(&payload)?)
                    .await
            }
            .await
        }
        "/multica/skills/bindings" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_skill_bindings(parse_multica_skill_bindings_query(&payload)?)
                    .await
            }
            .await
        }
        "/multica/executions/create" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_create(parse_multica_execution_create(&payload)?)
                    .await
            }
            .await
        }
        "/multica/executions/open" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_open(parse_multica_execution_binding(&payload)?)
                    .await
            }
            .await
        }
        "/multica/executions/continue" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_continue(parse_multica_execution_continue(&payload)?)
                    .await
            }
            .await
        }
        "/multica/executions/cancel" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_cancel(parse_multica_execution_cancel(&payload)?)
                    .await
            }
            .await
        }
        "/multica/executions/status" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_status(parse_multica_execution_binding(&payload)?)
                    .await
            }
            .await
        }
        "/multica/executions/list" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_list(parse_multica_execution_list(&payload)?)
                    .await
            }
            .await
        }
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

async fn ensure_multica_workspace_enabled(ctx: &BridgeContext) -> anyhow::Result<()> {
    if ctx.settings.get_settings().await?.multica_workspace_enabled {
        Ok(())
    } else {
        anyhow::bail!("multica_workspace_disabled")
    }
}

const MAX_MULTICA_BRIDGE_PAYLOAD_BYTES: usize = 32 * 1024;

fn ensure_multica_payload_size(payload: &Value) -> anyhow::Result<()> {
    let bytes =
        serde_json::to_vec(payload).map_err(|_| anyhow::anyhow!("multica_payload_invalid"))?;
    if bytes.len() > MAX_MULTICA_BRIDGE_PAYLOAD_BYTES {
        anyhow::bail!("multica_payload_too_large");
    }
    Ok(())
}

fn parse_empty_multica_payload(payload: &Value) -> anyhow::Result<()> {
    ensure_multica_payload_size(payload)?;
    if payload.as_object().is_some_and(Map::is_empty) {
        Ok(())
    } else {
        anyhow::bail!("multica_payload_invalid")
    }
}

fn parse_multica_workspace_query(payload: &Value) -> anyhow::Result<MulticaWorkspaceQuery> {
    ensure_multica_payload_size(payload)?;
    let query = serde_json::from_value::<MulticaWorkspaceQuery>(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_workspace_query_invalid"))?;
    query.validate()?;
    Ok(query)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaWorkspaceUpsertRequest {
    pub resource: MulticaWorkspaceResourceKey,
    pub entity: Value,
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaWorkspaceDeleteRequest {
    pub resource: MulticaWorkspaceResourceKey,
    pub entity_id: String,
    pub expected_revision: u64,
}

fn parse_multica_workspace_upsert(
    payload: &Value,
) -> anyhow::Result<MulticaWorkspaceUpsertRequest> {
    ensure_multica_payload_size(payload)?;
    let request: MulticaWorkspaceUpsertRequest = serde_json::from_value(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_workspace_mutation_invalid"))?;
    validate_mutable_workspace_resource(request.resource)?;
    if !request.entity.is_object() {
        anyhow::bail!("multica_workspace_entity_invalid");
    }
    Ok(request)
}

fn parse_multica_workspace_delete(
    payload: &Value,
) -> anyhow::Result<MulticaWorkspaceDeleteRequest> {
    ensure_multica_payload_size(payload)?;
    let request: MulticaWorkspaceDeleteRequest = serde_json::from_value(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_workspace_mutation_invalid"))?;
    validate_mutable_workspace_resource(request.resource)?;
    validate_multica_execution_id(&request.entity_id)
        .map_err(|_| anyhow::anyhow!("multica_workspace_entity_invalid"))?;
    if request.expected_revision == 0 {
        anyhow::bail!("multica_workspace_revision_invalid");
    }
    Ok(request)
}

fn validate_mutable_workspace_resource(
    resource: MulticaWorkspaceResourceKey,
) -> anyhow::Result<()> {
    if matches!(
        resource,
        MulticaWorkspaceResourceKey::Issues
            | MulticaWorkspaceResourceKey::Projects
            | MulticaWorkspaceResourceKey::Agents
            | MulticaWorkspaceResourceKey::Squads
            | MulticaWorkspaceResourceKey::Autopilots
    ) {
        Ok(())
    } else {
        anyhow::bail!("multica_workspace_resource_not_mutable")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionCreateRequest {
    pub workspace_id: String,
    pub issue_id: String,
    pub prompt: String,
    #[serde(default)]
    pub cwd: Option<String>,
    pub idempotency_key: String,
    #[serde(default)]
    pub bindings: SkillBindings,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionBindingRequest {
    pub binding_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionContinueRequest {
    pub binding_id: String,
    pub prompt: String,
    #[serde(default)]
    pub cwd: Option<String>,
    pub idempotency_key: String,
    #[serde(default)]
    pub bindings: SkillBindings,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionCancelRequest {
    pub binding_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionListRequest {
    pub workspace_id: String,
    #[serde(default)]
    pub issue_id: Option<String>,
    #[serde(default = "default_multica_execution_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_multica_execution_limit() -> usize {
    50
}

fn parse_multica_execution_create(
    payload: &Value,
) -> anyhow::Result<MulticaExecutionCreateRequest> {
    let request: MulticaExecutionCreateRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.workspace_id)?;
    validate_multica_execution_id(&request.issue_id)?;
    validate_multica_execution_id(&request.idempotency_key)?;
    SkillBindingSelection {
        bindings: request.bindings.clone(),
    }
    .validate()?;
    CodexThreadRequest {
        workspace_id: request.workspace_id.clone(),
        issue_id: request.issue_id.clone(),
        prompt: request.prompt.clone(),
        cwd: request.cwd.clone(),
        skill_request: None,
    }
    .validate()?;
    Ok(request)
}

fn parse_multica_execution_binding(
    payload: &Value,
) -> anyhow::Result<MulticaExecutionBindingRequest> {
    let request: MulticaExecutionBindingRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.binding_id)?;
    Ok(request)
}

fn parse_multica_execution_continue(
    payload: &Value,
) -> anyhow::Result<MulticaExecutionContinueRequest> {
    let request: MulticaExecutionContinueRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.binding_id)?;
    validate_multica_execution_id(&request.idempotency_key)?;
    SkillBindingSelection {
        bindings: request.bindings.clone(),
    }
    .validate()?;
    CodexThreadRequest {
        workspace_id: "local".to_string(),
        issue_id: "issue".to_string(),
        prompt: request.prompt.clone(),
        cwd: request.cwd.clone(),
        skill_request: None,
    }
    .validate()?;
    if request.expected_revision == 0 {
        anyhow::bail!("multica_execution_revision_invalid");
    }
    Ok(request)
}

fn parse_multica_execution_cancel(
    payload: &Value,
) -> anyhow::Result<MulticaExecutionCancelRequest> {
    let request: MulticaExecutionCancelRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.binding_id)?;
    validate_multica_execution_id(&request.idempotency_key)?;
    if request.expected_revision == 0 {
        anyhow::bail!("multica_execution_revision_invalid");
    }
    Ok(request)
}

fn parse_multica_execution_list(payload: &Value) -> anyhow::Result<MulticaExecutionListRequest> {
    let request: MulticaExecutionListRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.workspace_id)?;
    if let Some(issue_id) = request.issue_id.as_deref() {
        validate_multica_execution_id(issue_id)?;
    }
    if request.limit == 0 || request.limit > 100 || request.offset > 100_000 {
        anyhow::bail!("multica_execution_pagination_invalid");
    }
    Ok(request)
}

fn parse_multica_execution_payload<T>(payload: &Value) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    ensure_multica_payload_size(payload)?;
    serde_json::from_value(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_execution_payload_invalid"))
}

fn validate_multica_execution_id(value: &str) -> anyhow::Result<()> {
    if value.is_empty()
        || value.len() > 240
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'@')
        })
    {
        anyhow::bail!("multica_execution_id_invalid");
    }
    Ok(())
}

fn parse_multica_skill_selection(payload: &Value) -> anyhow::Result<SkillBindingSelection> {
    ensure_multica_payload_size(payload)?;
    let selection = serde_json::from_value::<SkillBindingSelection>(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_skill_selection_invalid"))?;
    selection.validate()?;
    Ok(selection)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaSkillReviewRequest {
    pub id: String,
    pub trusted: bool,
    #[serde(default)]
    pub manifest_digest: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaSkillBindingRequest {
    pub scope_kind: SkillBindingScope,
    pub scope_id: String,
    pub skill_ref: SkillReference,
    #[serde(default = "default_skill_binding_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaSkillBindingRemoveRequest {
    pub scope_kind: SkillBindingScope,
    pub scope_id: String,
    pub skill_id: String,
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaSkillBindingsQueryRequest {
    #[serde(default)]
    pub scope_kind: Option<SkillBindingScope>,
    #[serde(default)]
    pub scope_id: Option<String>,
}

fn default_skill_binding_enabled() -> bool {
    true
}

fn parse_multica_skill_review(payload: &Value) -> anyhow::Result<MulticaSkillReviewRequest> {
    ensure_multica_payload_size(payload)?;
    let request = serde_json::from_value::<MulticaSkillReviewRequest>(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_skill_review_invalid"))?;
    if request.id.trim().is_empty() || request.id.len() > 240 {
        anyhow::bail!("multica_skill_review_invalid");
    }
    if let Some(digest) = request.manifest_digest.as_deref()
        && (digest.is_empty() || digest.len() > 128)
    {
        anyhow::bail!("multica_skill_review_invalid");
    }
    Ok(request)
}

fn parse_multica_skill_binding(payload: &Value) -> anyhow::Result<MulticaSkillBindingRequest> {
    ensure_multica_payload_size(payload)?;
    let request = serde_json::from_value::<MulticaSkillBindingRequest>(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_skill_binding_invalid"))?;
    validate_skill_binding_scope(&request.scope_id)?;
    if request.skill_ref.id.trim().is_empty() {
        anyhow::bail!("multica_skill_binding_invalid");
    }
    Ok(request)
}

fn parse_multica_skill_binding_remove(
    payload: &Value,
) -> anyhow::Result<MulticaSkillBindingRemoveRequest> {
    ensure_multica_payload_size(payload)?;
    let request = serde_json::from_value::<MulticaSkillBindingRemoveRequest>(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_skill_binding_invalid"))?;
    validate_skill_binding_scope(&request.scope_id)?;
    if request.skill_id.trim().is_empty() {
        anyhow::bail!("multica_skill_binding_invalid");
    }
    Ok(request)
}

fn parse_multica_skill_bindings_query(
    payload: &Value,
) -> anyhow::Result<MulticaSkillBindingsQueryRequest> {
    ensure_multica_payload_size(payload)?;
    let request = serde_json::from_value::<MulticaSkillBindingsQueryRequest>(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_skill_binding_invalid"))?;
    if let Some(scope_id) = request.scope_id.as_deref() {
        validate_skill_binding_scope(scope_id)?;
    }
    Ok(request)
}

fn validate_skill_binding_scope(value: &str) -> anyhow::Result<()> {
    if value.is_empty()
        || value.len() > 240
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'@')
        })
    {
        anyhow::bail!("multica_skill_binding_invalid");
    }
    Ok(())
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
    codex_execution: Option<Arc<dyn CodexExecutionService>>,
    multica_execution_store: MulticaExecutionStore,
    multica_workspace_store: LocalMulticaWorkspaceStore,
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
            codex_execution: None,
            multica_execution_store: MulticaExecutionStore::default(),
            multica_workspace_store: LocalMulticaWorkspaceStore::default(),
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

    /// Attach the current Codex page's native execution adapter used by the
    /// Multica workspace. The bridge never registers or starts a Codex
    /// runtime; production callers must provide the already-open page host.
    pub fn with_codex_execution_service(mut self, service: Arc<dyn CodexExecutionService>) -> Self {
        self.codex_execution = Some(service);
        self
    }

    pub fn with_multica_execution_store(mut self, store: MulticaExecutionStore) -> Self {
        self.multica_execution_store = store;
        self
    }

    pub fn with_multica_workspace_store(mut self, store: LocalMulticaWorkspaceStore) -> Self {
        self.multica_workspace_store = store;
        self
    }

    fn codex_execution_service(&self) -> anyhow::Result<Arc<dyn CodexExecutionService>> {
        self.codex_execution
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("codex_page_host_unavailable"))
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

    async fn multica_workspace_bootstrap(&self) -> anyhow::Result<Value> {
        let bootstrap = match self.codex_execution.as_ref() {
            Some(service) => {
                crate::multica_workspace::workspace_bootstrap_with_codex_runtime(Arc::clone(
                    service,
                ))
                .await?
            }
            None => crate::multica_workspace::workspace_bootstrap().await?,
        };
        Ok(serde_json::to_value(bootstrap)?)
    }

    async fn multica_workspace_query(&self, query: MulticaWorkspaceQuery) -> anyhow::Result<Value> {
        let collection = match self.codex_execution.as_ref() {
            Some(service) => {
                crate::multica_workspace::workspace_query_with_codex_runtime(
                    query,
                    Arc::clone(service),
                )
                .await?
            }
            None if matches!(
                query.resource,
                crate::multica_workspace::MulticaWorkspaceResourceKey::Skills
                    | crate::multica_workspace::MulticaWorkspaceResourceKey::Runtimes
            ) =>
            {
                anyhow::bail!("codex_page_host_unavailable")
            }
            None => crate::multica_workspace::workspace_query(query).await?,
        };
        Ok(serde_json::to_value(collection)?)
    }

    async fn multica_workspace_upsert(
        &self,
        request: MulticaWorkspaceUpsertRequest,
    ) -> anyhow::Result<Value> {
        let workspace_id = crate::multica_workspace::workspace_bootstrap()
            .await?
            .workspace
            .id;
        let entity = self.multica_workspace_store.upsert(
            &workspace_id,
            LocalWorkspaceEntityUpsert {
                resource: request.resource,
                entity: request.entity,
                expected_revision: request.expected_revision,
            },
            unix_now_ms(),
        )?;
        Ok(json!({"status": "ok", "entity": entity}))
    }

    async fn multica_workspace_delete(
        &self,
        request: MulticaWorkspaceDeleteRequest,
    ) -> anyhow::Result<Value> {
        let workspace_id = crate::multica_workspace::workspace_bootstrap()
            .await?
            .workspace
            .id;
        let entity_id = request.entity_id;
        let deleted = self.multica_workspace_store.delete(
            &workspace_id,
            LocalWorkspaceEntityDelete {
                resource: request.resource,
                entity_id: entity_id.clone(),
                expected_revision: request.expected_revision,
            },
        )?;
        Ok(json!({"status": "ok", "deleted": deleted, "entityId": entity_id}))
    }

    async fn multica_skill_resolve(
        &self,
        selection: SkillBindingSelection,
    ) -> anyhow::Result<Value> {
        let service = self
            .codex_execution
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("codex_page_host_unavailable"))?;
        crate::multica_workspace::resolve_skill_bindings_with_codex_runtime(
            selection,
            Arc::clone(service),
        )
        .await
    }

    async fn multica_skill_review(
        &self,
        request: MulticaSkillReviewRequest,
    ) -> anyhow::Result<Value> {
        review_local_skill(
            &request.id,
            request.trusted,
            request.manifest_digest.as_deref(),
        )
    }

    async fn multica_skill_bind(
        &self,
        request: MulticaSkillBindingRequest,
    ) -> anyhow::Result<Value> {
        let command = MulticaSkillBindingCommand {
            scope_kind: request.scope_kind,
            scope_id: request.scope_id,
            skill_ref: request.skill_ref,
            enabled: request.enabled,
            expected_revision: request.expected_revision,
        };
        let service = self
            .codex_execution
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("codex_page_host_unavailable"))?;
        crate::multica_workspace::upsert_skill_binding_with_codex_runtime(
            command,
            Arc::clone(service),
        )
        .await
    }

    async fn multica_skill_unbind(
        &self,
        request: MulticaSkillBindingRemoveRequest,
    ) -> anyhow::Result<Value> {
        crate::multica_workspace::remove_skill_binding(MulticaSkillBindingRemoveCommand {
            scope_kind: request.scope_kind,
            scope_id: request.scope_id,
            skill_id: request.skill_id,
            expected_revision: request.expected_revision,
        })
        .await
    }

    async fn multica_skill_bindings(
        &self,
        request: MulticaSkillBindingsQueryRequest,
    ) -> anyhow::Result<Value> {
        crate::multica_workspace::list_skill_bindings(MulticaSkillBindingsQuery {
            scope_kind: request.scope_kind,
            scope_id: request.scope_id,
        })
        .await
    }

    async fn multica_execution_create(
        &self,
        request: MulticaExecutionCreateRequest,
    ) -> anyhow::Result<Value> {
        let service = self.codex_execution_service()?;
        let now_ms = unix_now_ms();
        let reserved = self
            .multica_execution_store
            .reserve_execution(ExecutionReservation {
                workspace_id: request.workspace_id.clone(),
                issue_id: request.issue_id.clone(),
                execution_kind: MulticaExecutionKind::Thread,
                parent_thread_id: None,
                parent_attempt_id: None,
                idempotency_key: request.idempotency_key.clone(),
                now_ms,
            })?;
        if reserved.replay {
            if reserved.binding.codex_thread_id.is_some() {
                let handle = execution_handle_from_binding(
                    &reserved.binding,
                    &reserved.binding.idempotency_key,
                )?;
                return Ok(execution_handle_response(reserved.binding, handle));
            }
            if reserved.binding.state == MulticaExecutionBindingState::Failed {
                anyhow::bail!(
                    "{}",
                    reserved
                        .binding
                        .last_error_code
                        .as_deref()
                        .unwrap_or("codex_execution_failed")
                );
            }
        }
        let (skill_request, skill_audit) =
            match resolve_execution_skills(Arc::clone(&service), request.bindings).await {
                Ok(value) => value,
                Err(error) => {
                    let code = stable_execution_error_code(&error);
                    let _ = self.multica_execution_store.fail_execution(
                        &reserved.binding.binding_id,
                        reserved.binding.revision,
                        &code,
                        true,
                        unix_now_ms(),
                    );
                    return Err(error);
                }
            };
        if let Some(audit) = skill_audit.as_ref() {
            self.multica_execution_store.reserve_attempt_snapshot(
                &reserved.binding.binding_id,
                reserved.binding.attempt_no,
                audit,
                now_ms,
            )?;
        }
        let native_request = CodexThreadRequest {
            workspace_id: request.workspace_id,
            issue_id: request.issue_id,
            prompt: request.prompt,
            cwd: request.cwd,
            skill_request,
        };
        let handle = match service
            .create_thread(native_request, &request.idempotency_key)
            .await
        {
            Ok(handle) => handle,
            Err(error) => {
                let code = stable_execution_error_code(&error);
                let _ = self.multica_execution_store.fail_execution(
                    &reserved.binding.binding_id,
                    reserved.binding.revision,
                    &code,
                    true,
                    unix_now_ms(),
                );
                return Err(anyhow::anyhow!(code));
            }
        };
        let binding = self.multica_execution_store.commit_execution(
            &reserved.binding.binding_id,
            reserved.binding.revision,
            &handle,
            unix_now_ms(),
        )?;
        Ok(execution_handle_response(binding, handle))
    }

    async fn multica_execution_open(
        &self,
        request: MulticaExecutionBindingRequest,
    ) -> anyhow::Result<Value> {
        let service = self.codex_execution_service()?;
        let binding = self
            .multica_execution_store
            .get_execution(&request.binding_id)?;
        if binding.state == MulticaExecutionBindingState::Orphaned {
            anyhow::bail!("execution_thread_orphaned");
        }
        let thread_id = binding
            .codex_thread_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("execution_binding_pending"))?;
        let handle = service.open_thread(thread_id).await?;
        Ok(execution_handle_response(binding, handle))
    }

    async fn multica_execution_continue(
        &self,
        request: MulticaExecutionContinueRequest,
    ) -> anyhow::Result<Value> {
        let service = self.codex_execution_service()?;
        let binding = self
            .multica_execution_store
            .get_execution(&request.binding_id)?;
        if binding.revision != request.expected_revision {
            anyhow::bail!("execution_revision_conflict");
        }
        if !matches!(
            binding.state,
            MulticaExecutionBindingState::Dispatched
                | MulticaExecutionBindingState::Running
                | MulticaExecutionBindingState::Stale
        ) {
            anyhow::bail!("execution_not_continuable");
        }
        let thread_id = binding
            .codex_thread_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_binding_pending"))?;
        let command = self.multica_execution_store.reserve_command(
            &binding.binding_id,
            MulticaExecutionCommandKind::Continue,
            &request.idempotency_key,
            request.expected_revision,
            unix_now_ms(),
        )?;
        if command.replay {
            match command.command.state {
                MulticaExecutionCommandState::Committed => {
                    let mut handle =
                        execution_handle_from_binding(&binding, &request.idempotency_key)?;
                    handle.execution_id = command.command.codex_execution_id;
                    return Ok(execution_handle_response(binding, handle));
                }
                MulticaExecutionCommandState::Failed => anyhow::bail!(
                    "{}",
                    command
                        .command
                        .error_code
                        .as_deref()
                        .unwrap_or("codex_execution_failed")
                ),
                MulticaExecutionCommandState::Reserved => {}
            }
        }
        let (skill_request, _) =
            match resolve_execution_skills(Arc::clone(&service), request.bindings).await {
                Ok(value) => value,
                Err(error) => {
                    let code = stable_execution_error_code(&error);
                    let _ = self.multica_execution_store.fail_command(
                        &request.idempotency_key,
                        &code,
                        unix_now_ms(),
                    );
                    return Err(error);
                }
            };
        let native_request = CodexThreadRequest {
            workspace_id: binding.workspace_id.clone(),
            issue_id: binding.issue_id.clone(),
            prompt: request.prompt,
            cwd: request.cwd,
            skill_request,
        };
        native_request.validate()?;
        let handle = match service
            .continue_thread(&thread_id, native_request, &request.idempotency_key)
            .await
        {
            Ok(handle) => handle,
            Err(error) => {
                let code = stable_execution_error_code(&error);
                let _ = self.multica_execution_store.fail_command(
                    &request.idempotency_key,
                    &code,
                    unix_now_ms(),
                );
                return Err(anyhow::anyhow!(code));
            }
        };
        let (_, binding) = self.multica_execution_store.commit_continue(
            &request.idempotency_key,
            request.expected_revision,
            &handle,
            unix_now_ms(),
        )?;
        Ok(execution_handle_response(binding, handle))
    }

    async fn multica_execution_cancel(
        &self,
        request: MulticaExecutionCancelRequest,
    ) -> anyhow::Result<Value> {
        let service = self.codex_execution_service()?;
        let binding = self
            .multica_execution_store
            .get_execution(&request.binding_id)?;
        let thread_id = binding
            .codex_thread_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_binding_pending"))?;
        let execution_id = binding
            .codex_execution_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_id_unavailable"))?;
        let command = self.multica_execution_store.reserve_command(
            &binding.binding_id,
            MulticaExecutionCommandKind::Cancel,
            &request.idempotency_key,
            request.expected_revision,
            unix_now_ms(),
        )?;
        if command.replay {
            match command.command.state {
                MulticaExecutionCommandState::Committed => {
                    let status = execution_status_from_binding(&binding)?;
                    return Ok(execution_status_response(binding, status));
                }
                MulticaExecutionCommandState::Failed => anyhow::bail!(
                    "{}",
                    command
                        .command
                        .error_code
                        .as_deref()
                        .unwrap_or("codex_execution_failed")
                ),
                MulticaExecutionCommandState::Reserved => {}
            }
        }
        let status = match service.cancel_execution(&thread_id, &execution_id).await {
            Ok(status) => status,
            Err(error) => {
                let code = stable_execution_error_code(&error);
                let _ = self.multica_execution_store.fail_command(
                    &request.idempotency_key,
                    &code,
                    unix_now_ms(),
                );
                return Err(anyhow::anyhow!(code));
            }
        };
        let (_, binding) = self.multica_execution_store.commit_cancel(
            &request.idempotency_key,
            request.expected_revision,
            &status,
            unix_now_ms(),
        )?;
        Ok(execution_status_response(binding, status))
    }

    async fn multica_execution_status(
        &self,
        request: MulticaExecutionBindingRequest,
    ) -> anyhow::Result<Value> {
        let service = self.codex_execution_service()?;
        let binding = self
            .multica_execution_store
            .get_execution(&request.binding_id)?;
        let thread_id = binding
            .codex_thread_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("execution_binding_pending"))?;
        let execution_id = binding
            .codex_execution_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("execution_id_unavailable"))?;
        let status = service.execution_status(thread_id, execution_id).await?;
        let binding = self.multica_execution_store.record_status(
            &binding.binding_id,
            binding.revision,
            &status,
            unix_now_ms(),
        )?;
        Ok(execution_status_response(binding, status))
    }

    async fn multica_execution_list(
        &self,
        request: MulticaExecutionListRequest,
    ) -> anyhow::Result<Value> {
        let (items, total) = self.multica_execution_store.list_executions(
            &request.workspace_id,
            request.issue_id.as_deref(),
            request.limit,
            request.offset,
        )?;
        Ok(json!({
            "status": "ok",
            "items": items,
            "total": total,
            "limit": request.limit,
            "offset": request.offset,
        }))
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

async fn resolve_execution_skills(
    service: Arc<dyn CodexExecutionService>,
    bindings: SkillBindings,
) -> anyhow::Result<(
    Option<crate::multica_execution::CodexSkillExecutionRequest>,
    Option<SkillResolutionAudit>,
)> {
    if bindings.task.is_empty() && bindings.agent.is_empty() {
        return Ok((None, None));
    }
    let value = crate::multica_workspace::resolve_skill_bindings_with_codex_runtime(
        SkillBindingSelection { bindings },
        service,
    )
    .await?;
    let audit: SkillResolutionAudit = serde_json::from_value(
        value
            .get("audit")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("skill_resolution_invalid"))?,
    )
    .map_err(|_| anyhow::anyhow!("skill_resolution_invalid"))?;
    let request = audit.execution_request()?;
    Ok((Some(request), Some(audit)))
}

fn execution_handle_from_binding(
    binding: &CodexMulticaExecutionBinding,
    idempotency_key: &str,
) -> anyhow::Result<CodexExecutionHandle> {
    Ok(CodexExecutionHandle {
        runtime_id: binding
            .codex_runtime_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_runtime_id_unavailable"))?,
        thread_id: binding
            .codex_thread_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_binding_pending"))?,
        execution_id: binding.codex_execution_id.clone(),
        parent_thread_id: binding.parent_thread_id.clone(),
        idempotency_key: idempotency_key.to_string(),
    })
}

fn execution_status_from_binding(
    binding: &CodexMulticaExecutionBinding,
) -> anyhow::Result<CodexExecutionStatus> {
    let state = match binding.state {
        MulticaExecutionBindingState::BindingPending => CodexExecutionState::Queued,
        MulticaExecutionBindingState::Dispatched => CodexExecutionState::Queued,
        MulticaExecutionBindingState::Running => CodexExecutionState::Running,
        MulticaExecutionBindingState::Completed => CodexExecutionState::Completed,
        MulticaExecutionBindingState::Failed => CodexExecutionState::Failed,
        MulticaExecutionBindingState::Cancelled => CodexExecutionState::Cancelled,
        MulticaExecutionBindingState::CancelPending => CodexExecutionState::CancelPending,
        MulticaExecutionBindingState::WaitingLocalDirectory
        | MulticaExecutionBindingState::Stale
        | MulticaExecutionBindingState::Orphaned
        | MulticaExecutionBindingState::Reconciling => CodexExecutionState::Unknown,
    };
    Ok(CodexExecutionStatus {
        runtime_id: binding
            .codex_runtime_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_runtime_id_unavailable"))?,
        thread_id: binding
            .codex_thread_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_binding_pending"))?,
        execution_id: binding
            .codex_execution_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_id_unavailable"))?,
        state,
        diagnostic: None,
    })
}

fn execution_handle_response(
    binding: CodexMulticaExecutionBinding,
    handle: CodexExecutionHandle,
) -> Value {
    json!({"status": "ok", "binding": binding, "handle": handle})
}

fn execution_status_response(
    binding: CodexMulticaExecutionBinding,
    execution_status: CodexExecutionStatus,
) -> Value {
    json!({"status": "ok", "binding": binding, "executionStatus": execution_status})
}

fn stable_execution_error_code(error: &anyhow::Error) -> String {
    let value = error.to_string();
    if !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        value
    } else {
        "codex_execution_failed".to_string()
    }
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
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
