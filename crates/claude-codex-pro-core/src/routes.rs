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
    AutopilotRunTransition, CodexMulticaExecutionBinding, CodexMulticaTaskMessage,
    ExecutionReservation, MulticaExecutionBindingState, MulticaExecutionCommandKind,
    MulticaExecutionCommandState, MulticaExecutionKind, MulticaExecutionStore, QueueTransition,
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
    async fn multica_execution_dispatch(
        &self,
        _request: MulticaExecutionDispatchRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_unavailable")
    }
    async fn dispatch_pending_assignment(
        &self,
        _binding_id: &str,
        _expected_revision: u64,
        _lease_token: &str,
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
    async fn multica_execution_lease_claim(
        &self,
        _request: MulticaExecutionLeaseClaimRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_lease_unavailable")
    }
    async fn multica_execution_lease_renew(
        &self,
        _request: MulticaExecutionLeaseRenewRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_lease_unavailable")
    }
    async fn multica_execution_lease_release(
        &self,
        _request: MulticaExecutionLeaseReleaseRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_lease_unavailable")
    }
    async fn multica_execution_message_append(
        &self,
        _request: MulticaExecutionMessageAppendRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_message_unavailable")
    }
    async fn multica_execution_message_list(
        &self,
        _request: MulticaExecutionMessageListRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_message_unavailable")
    }
    async fn multica_task_queue_transition(
        &self,
        _request: MulticaTaskQueueTransitionRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_execution_unavailable")
    }
    async fn multica_autopilot_runs(
        &self,
        _request: MulticaAutopilotRunsRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_autopilot_unavailable")
    }
    async fn multica_autopilot_run(
        &self,
        _request: MulticaAutopilotRunRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_autopilot_unavailable")
    }
    async fn multica_autopilot_trigger(
        &self,
        _request: MulticaAutopilotTriggerRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_autopilot_unavailable")
    }
    async fn multica_autopilot_transition(
        &self,
        _request: MulticaAutopilotTransitionRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_autopilot_unavailable")
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
        "/multica/executions/dispatch" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_dispatch(parse_multica_execution_dispatch(&payload)?)
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
        "/multica/executions/lease/claim" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_lease_claim(parse_multica_execution_lease_claim(&payload)?)
                    .await
            }
            .await
        }
        "/multica/executions/lease/renew" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_lease_renew(parse_multica_execution_lease_claim(&payload)?)
                    .await
            }
            .await
        }
        "/multica/executions/lease/release" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_lease_release(parse_multica_execution_lease_release(
                        &payload,
                    )?)
                    .await
            }
            .await
        }
        "/multica/executions/messages" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_message_append(parse_multica_execution_message_append(
                        &payload,
                    )?)
                    .await
            }
            .await
        }
        "/multica/executions/messages/list" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_execution_message_list(parse_multica_execution_message_list(&payload)?)
                    .await
            }
            .await
        }
        "/multica/tasks/queue/transition" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_task_queue_transition(parse_multica_task_queue_transition(&payload)?)
                    .await
            }
            .await
        }
        "/multica/autopilots/runs" => {
            async {
                ctx.runtime
                    .multica_autopilot_runs(parse_multica_autopilot_runs(&payload)?)
                    .await
            }
            .await
        }
        "/multica/autopilots/run" => {
            async {
                ctx.runtime
                    .multica_autopilot_run(parse_multica_autopilot_run(&payload)?)
                    .await
            }
            .await
        }
        "/multica/autopilots/trigger" => {
            async {
                ctx.runtime
                    .multica_autopilot_trigger(parse_multica_autopilot_trigger(&payload)?)
                    .await
            }
            .await
        }
        "/multica/autopilots/transition" => {
            async {
                ctx.runtime
                    .multica_autopilot_transition(parse_multica_autopilot_transition(&payload)?)
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
            | MulticaWorkspaceResourceKey::Comments
            | MulticaWorkspaceResourceKey::Labels
            | MulticaWorkspaceResourceKey::Subscribers
            | MulticaWorkspaceResourceKey::Projects
            | MulticaWorkspaceResourceKey::ProjectResources
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
    pub execution_kind: Option<MulticaExecutionKind>,
    #[serde(default)]
    pub parent_thread_id: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub bindings: SkillBindings,
}

/// Claim and dispatch one assignment-created queued binding. The renderer
/// supplies a lease token so a retry from another page cannot create a second
/// native Codex thread.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionDispatchRequest {
    pub binding_id: String,
    pub expected_revision: u64,
    pub lease_token: String,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionLeaseClaimRequest {
    pub binding_id: String,
    pub expected_revision: u64,
    pub lease_token: String,
    pub lease_duration_ms: u64,
}

pub type MulticaExecutionLeaseRenewRequest = MulticaExecutionLeaseClaimRequest;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionLeaseReleaseRequest {
    pub binding_id: String,
    pub expected_revision: u64,
    pub lease_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionMessageAppendRequest {
    pub message: CodexMulticaTaskMessage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionMessageListRequest {
    pub binding_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaTaskQueueTransitionRequest {
    pub binding_id: String,
    pub expected_revision: u64,
    #[serde(default)]
    pub lease_token: Option<String>,
    pub status: String,
    #[serde(default)]
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaAutopilotRunsRequest {
    pub autopilot_id: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaAutopilotRunRequest {
    pub autopilot_id: String,
    pub run_id: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaAutopilotTriggerRequest {
    pub autopilot_id: String,
    #[serde(default)]
    pub trigger_id: Option<String>,
    #[serde(default = "default_manual_source")]
    pub source: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaAutopilotTransitionRequest {
    pub autopilot_id: String,
    pub run_id: String,
    pub expected_revision: u64,
    pub status: String,
    #[serde(default)]
    pub issue_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub failure_reason: Option<String>,
    #[serde(default)]
    pub reason_code: Option<String>,
}
fn default_manual_source() -> String {
    "manual".into()
}
fn parse_multica_autopilot_runs(payload: &Value) -> anyhow::Result<MulticaAutopilotRunsRequest> {
    Ok(serde_json::from_value(payload.clone())?)
}
fn parse_multica_autopilot_run(payload: &Value) -> anyhow::Result<MulticaAutopilotRunRequest> {
    Ok(serde_json::from_value(payload.clone())?)
}
fn parse_multica_autopilot_trigger(
    payload: &Value,
) -> anyhow::Result<MulticaAutopilotTriggerRequest> {
    Ok(serde_json::from_value(payload.clone())?)
}
fn parse_multica_autopilot_transition(
    payload: &Value,
) -> anyhow::Result<MulticaAutopilotTransitionRequest> {
    Ok(serde_json::from_value(payload.clone())?)
}

fn parse_multica_task_queue_transition(
    payload: &Value,
) -> anyhow::Result<MulticaTaskQueueTransitionRequest> {
    let request: MulticaTaskQueueTransitionRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.binding_id)?;
    let _ = MulticaExecutionBindingState::from_queue_status(&request.status)?;
    if request.expected_revision == 0 {
        anyhow::bail!("multica_execution_revision_invalid");
    }
    if let Some(token) = request.lease_token.as_deref() {
        validate_multica_execution_id(token)?;
    }
    if let Some(reason) = request.failure_reason.as_deref() {
        validate_multica_execution_id(reason)?;
    }
    Ok(request)
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
    if let Some(parent) = request.parent_thread_id.as_deref() {
        validate_multica_execution_id(parent)?;
    }
    if let Some(agent) = request.agent_id.as_deref() {
        validate_multica_execution_id(agent)?;
    }
    if request.execution_kind == Some(MulticaExecutionKind::Subagent)
        && (request.parent_thread_id.is_none() || request.agent_id.is_none())
    {
        anyhow::bail!("subagent_parent_or_agent_required");
    }
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

fn parse_multica_execution_dispatch(
    payload: &Value,
) -> anyhow::Result<MulticaExecutionDispatchRequest> {
    let request: MulticaExecutionDispatchRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.binding_id)?;
    validate_multica_execution_id(&request.lease_token)?;
    if request.expected_revision == 0 {
        anyhow::bail!("multica_execution_revision_invalid");
    }
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

fn parse_multica_execution_lease_claim(
    payload: &Value,
) -> anyhow::Result<MulticaExecutionLeaseClaimRequest> {
    let request: MulticaExecutionLeaseClaimRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.binding_id)?;
    validate_multica_execution_id(&request.lease_token)?;
    if request.expected_revision == 0 {
        anyhow::bail!("multica_execution_revision_invalid");
    }
    Ok(request)
}

fn parse_multica_execution_lease_release(
    payload: &Value,
) -> anyhow::Result<MulticaExecutionLeaseReleaseRequest> {
    let request: MulticaExecutionLeaseReleaseRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.binding_id)?;
    validate_multica_execution_id(&request.lease_token)?;
    if request.expected_revision == 0 {
        anyhow::bail!("multica_execution_revision_invalid");
    }
    Ok(request)
}

fn parse_multica_execution_message_append(
    payload: &Value,
) -> anyhow::Result<MulticaExecutionMessageAppendRequest> {
    let request: MulticaExecutionMessageAppendRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.message.binding_id)?;
    validate_multica_execution_id(&request.message.message_id)?;
    Ok(request)
}

fn parse_multica_execution_message_list(
    payload: &Value,
) -> anyhow::Result<MulticaExecutionMessageListRequest> {
    let request: MulticaExecutionMessageListRequest = parse_multica_execution_payload(payload)?;
    validate_multica_execution_id(&request.binding_id)?;
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
        let queue = if request.resource == MulticaWorkspaceResourceKey::Issues
            && entity.get("assignee_type").and_then(Value::as_str) == Some("agent")
        {
            let agent_id = entity
                .get("assignee_id")
                .and_then(Value::as_str)
                .filter(|id| !id.trim().is_empty());
            if let Some(agent_id) = agent_id {
                let issue_id = entity
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("multica_workspace_entity_invalid"))?;
                let key = format!("issue-assignment:{workspace_id}:{issue_id}:{agent_id}");
                let reservation =
                    self.multica_execution_store
                        .reserve_execution(ExecutionReservation {
                            workspace_id: workspace_id.clone(),
                            issue_id: issue_id.to_string(),
                            agent_id: Some(agent_id.to_string()),
                            execution_kind: MulticaExecutionKind::Thread,
                            parent_thread_id: None,
                            parent_attempt_id: None,
                            idempotency_key: key,
                            now_ms: unix_now_ms(),
                        })?;
                let auto_dispatch = self
                    .dispatch_pending_assignment(
                        &reservation.binding.binding_id,
                        reservation.binding.revision,
                        &format!("auto-{}", reservation.binding.binding_id),
                    )
                    .await;
                let dispatched = match auto_dispatch {
                    Ok(value) => Some(value),
                    Err(error) => Some(json!({
                        "status": "queued",
                        "diagnostic": stable_execution_error_code(&error),
                    })),
                };
                Some(json!({
                    "binding_id": reservation.binding.binding_id,
                    "status": dispatched
                        .as_ref()
                        .and_then(|value| value.get("binding"))
                        .and_then(|binding| binding.get("state"))
                        .and_then(Value::as_str)
                        .unwrap_or("queued"),
                    "replay": reservation.replay,
                    "dispatch": dispatched,
                }))
            } else {
                None
            }
        } else {
            None
        };
        Ok(json!({"status": "ok", "entity": entity, "queue": queue}))
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
        if deleted && request.resource == MulticaWorkspaceResourceKey::Issues {
            self.multica_execution_store.cancel_active_for_issue(
                &workspace_id,
                &entity_id,
                None,
                unix_now_ms(),
            )?;
        }
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
                execution_kind: request
                    .execution_kind
                    .unwrap_or(MulticaExecutionKind::Thread),
                agent_id: request.agent_id.clone(),
                parent_thread_id: request.parent_thread_id.clone(),
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
        let handle = match if reserved.binding.execution_kind == MulticaExecutionKind::Subagent {
            service
                .create_subagent(
                    reserved.binding.parent_thread_id.as_deref().unwrap(),
                    native_request,
                    &request.idempotency_key,
                )
                .await
        } else {
            service
                .create_thread(native_request, &request.idempotency_key)
                .await
        } {
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

    async fn multica_execution_dispatch(
        &self,
        request: MulticaExecutionDispatchRequest,
    ) -> anyhow::Result<Value> {
        self.dispatch_pending_assignment(
            &request.binding_id,
            request.expected_revision,
            &request.lease_token,
        )
        .await
    }

    /// Dispatch only a binding already created by Agent assignment. This is
    /// deliberately separate from explicit execution creation because its
    /// prompt comes only from persisted Issue and Agent fields.
    async fn dispatch_pending_assignment(
        &self,
        binding_id: &str,
        expected_revision: u64,
        lease_token: &str,
    ) -> anyhow::Result<Value> {
        let binding = self.multica_execution_store.get_execution(binding_id)?;
        if binding.revision != expected_revision {
            anyhow::bail!("execution_revision_conflict");
        }
        if binding.state == MulticaExecutionBindingState::Dispatched {
            let handle = execution_handle_from_binding(&binding, &binding.idempotency_key)?;
            return Ok(execution_handle_response(binding, handle));
        }
        if binding.state != MulticaExecutionBindingState::BindingPending {
            anyhow::bail!("execution_not_dispatchable");
        }

        let claimed = self.multica_execution_store.claim_execution_lease(
            binding_id,
            expected_revision,
            lease_token,
            unix_now_ms(),
            30_000,
        )?;
        let release = |revision| {
            self.multica_execution_store.release_execution_lease(
                binding_id,
                revision,
                lease_token,
                unix_now_ms(),
            )
        };

        let result = async {
            let service = self.codex_execution_service()?;
            let agent_id = claimed
                .agent_id
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("execution_agent_unavailable"))?;
            let issues = self
                .multica_workspace_store
                .list(&claimed.workspace_id, MulticaWorkspaceResourceKey::Issues)?;
            let issue = issues
                .into_iter()
                .find(|issue| {
                    issue.get("id").and_then(Value::as_str) == Some(claimed.issue_id.as_str())
                })
                .ok_or_else(|| anyhow::anyhow!("execution_issue_unavailable"))?;
            if issue.get("assignee_type").and_then(Value::as_str) != Some("agent")
                || issue.get("assignee_id").and_then(Value::as_str) != Some(agent_id)
            {
                anyhow::bail!("execution_assignment_changed");
            }
            let agents = self
                .multica_workspace_store
                .list(&claimed.workspace_id, MulticaWorkspaceResourceKey::Agents)?;
            let agent = agents
                .into_iter()
                .find(|agent| agent.get("id").and_then(Value::as_str) == Some(agent_id))
                .ok_or_else(|| anyhow::anyhow!("execution_agent_unavailable"))?;
            let native_request = CodexThreadRequest {
                workspace_id: claimed.workspace_id.clone(),
                issue_id: claimed.issue_id.clone(),
                prompt: assignment_prompt(&issue, &agent)?,
                cwd: None,
                skill_request: None,
            };
            native_request.validate()?;
            let handle = service
                .create_thread(native_request, &claimed.idempotency_key)
                .await
                .map_err(|error| anyhow::anyhow!(stable_execution_error_code(&error)))?;
            let binding = match self.multica_execution_store.commit_execution(
                binding_id,
                claimed.revision,
                &handle,
                unix_now_ms(),
            ) {
                Ok(binding) => binding,
                Err(_) => {
                    // The native thread already exists but its durable
                    // mapping is ambiguous. Fail closed instead of leaving
                    // a queued binding that could create a second thread.
                    let _ = self.multica_execution_store.fail_execution(
                        binding_id,
                        claimed.revision,
                        "execution_mapping_pending",
                        false,
                        unix_now_ms(),
                    );
                    anyhow::bail!("execution_mapping_pending");
                }
            };
            Ok::<_, anyhow::Error>((binding, handle))
        }
        .await;

        match result {
            Ok((binding, handle)) => {
                let released = release(binding.revision)?;
                Ok(execution_handle_response(released, handle))
            }
            Err(error) => {
                // A failed preflight or host call is retryable: leave the
                // assignment binding queued and make the lease available.
                let _ = release(claimed.revision);
                Err(error)
            }
        }
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

    async fn multica_execution_lease_claim(
        &self,
        request: MulticaExecutionLeaseClaimRequest,
    ) -> anyhow::Result<Value> {
        let binding = self.multica_execution_store.claim_execution_lease(
            &request.binding_id,
            request.expected_revision,
            &request.lease_token,
            unix_now_ms(),
            request.lease_duration_ms,
        )?;
        Ok(json!({"status":"ok", "binding": binding}))
    }

    async fn multica_autopilot_runs(
        &self,
        request: MulticaAutopilotRunsRequest,
    ) -> anyhow::Result<Value> {
        let items = self
            .multica_execution_store
            .list_autopilot_runs(&request.autopilot_id)?;
        Ok(json!({"status":"ok", "runs": items, "total": items.len()}))
    }

    async fn multica_autopilot_run(
        &self,
        request: MulticaAutopilotRunRequest,
    ) -> anyhow::Result<Value> {
        let run = self
            .multica_execution_store
            .get_autopilot_run(&request.run_id)?;
        if run.autopilot_id != request.autopilot_id {
            anyhow::bail!("autopilot_run_not_found");
        }
        Ok(json!({"status":"ok", "run": run}))
    }

    async fn multica_autopilot_trigger(
        &self,
        request: MulticaAutopilotTriggerRequest,
    ) -> anyhow::Result<Value> {
        let run = self.multica_execution_store.trigger_autopilot_run(
            request.autopilot_id,
            request.trigger_id,
            request.source,
            unix_now_ms(),
        )?;
        Ok(json!({"status":"ok", "run": run, "execution":"pending"}))
    }

    async fn multica_autopilot_transition(
        &self,
        request: MulticaAutopilotTransitionRequest,
    ) -> anyhow::Result<Value> {
        let run =
            self.multica_execution_store
                .transition_autopilot_run(AutopilotRunTransition {
                    autopilot_id: request.autopilot_id,
                    run_id: request.run_id,
                    expected_revision: request.expected_revision,
                    next_status: request.status,
                    issue_id: request.issue_id,
                    task_id: request.task_id,
                    failure_reason: request.failure_reason,
                    reason_code: request.reason_code,
                    now_ms: unix_now_ms(),
                })?;
        Ok(json!({"status":"ok", "run": run}))
    }

    async fn multica_execution_lease_renew(
        &self,
        request: MulticaExecutionLeaseRenewRequest,
    ) -> anyhow::Result<Value> {
        let binding = self.multica_execution_store.renew_execution_lease(
            &request.binding_id,
            request.expected_revision,
            &request.lease_token,
            unix_now_ms(),
            request.lease_duration_ms,
        )?;
        Ok(json!({"status":"ok", "binding": binding}))
    }

    async fn multica_execution_lease_release(
        &self,
        request: MulticaExecutionLeaseReleaseRequest,
    ) -> anyhow::Result<Value> {
        let binding = self.multica_execution_store.release_execution_lease(
            &request.binding_id,
            request.expected_revision,
            &request.lease_token,
            unix_now_ms(),
        )?;
        Ok(json!({"status":"ok", "binding": binding}))
    }

    async fn multica_execution_message_append(
        &self,
        request: MulticaExecutionMessageAppendRequest,
    ) -> anyhow::Result<Value> {
        let message = self
            .multica_execution_store
            .append_task_message(request.message)?;
        Ok(json!({"status":"ok", "message": message}))
    }

    async fn multica_execution_message_list(
        &self,
        request: MulticaExecutionMessageListRequest,
    ) -> anyhow::Result<Value> {
        let messages = self
            .multica_execution_store
            .list_task_messages(&request.binding_id)?;
        Ok(json!({"status":"ok", "items": messages}))
    }

    async fn multica_task_queue_transition(
        &self,
        request: MulticaTaskQueueTransitionRequest,
    ) -> anyhow::Result<Value> {
        let next_state = MulticaExecutionBindingState::from_queue_status(&request.status)?;
        let binding = self
            .multica_execution_store
            .transition_queue(QueueTransition {
                binding_id: request.binding_id,
                expected_revision: request.expected_revision,
                lease_token: request.lease_token,
                next_state,
                failure_reason: request.failure_reason,
                now_ms: unix_now_ms(),
            })?;
        Ok(json!({"status":"ok", "binding": binding}))
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

fn assignment_prompt(issue: &Value, agent: &Value) -> anyhow::Result<String> {
    let title = issue
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("execution_issue_title_unavailable"))?;
    let description = issue
        .get("description")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let instructions = agent
        .get("instructions")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let mut prompt = format!("任务标题：\n{title}");
    if let Some(description) = description {
        prompt.push_str("\n\n任务描述：\n");
        prompt.push_str(description);
    }
    if let Some(instructions) = instructions {
        prompt.push_str("\n\n智能体指令：\n");
        prompt.push_str(instructions);
    }
    Ok(prompt)
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
    if value.contains("function_call_output requires call_id on HTTP requests")
        || value.contains(
            "continuation via previous_response_id is only supported on Responses WebSocket v2",
        )
    {
        return "codex_host_transport_call_id_required".to_string();
    }
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use anyhow::bail;
    use async_trait::async_trait;
    use serde_json::json;

    use crate::codex_execution::{
        CodexExecutionEvent, CodexRuntimeCapabilities, CodexSkill, CodexThreadRequest,
    };
    use crate::multica_execution::CodexSkillExecutionRequest;
    use crate::multica_execution_store::{ExecutionReservation, MulticaExecutionKind};
    use crate::multica_workspace::{
        LocalMulticaWorkspaceStore, LocalWorkspaceEntityUpsert, MulticaWorkspaceResourceKey,
    };
    use crate::status::StatusStore;

    use super::{
        BridgeRuntimeService, CodexExecutionHandle, CodexExecutionService, CodexExecutionStatus,
        CoreRuntimeService, MulticaExecutionBindingState, MulticaExecutionDispatchRequest,
        MulticaExecutionStore, assignment_prompt, stable_execution_error_code,
    };

    #[derive(Default)]
    struct RecordingCodexHost {
        requests: Mutex<Vec<CodexThreadRequest>>,
    }

    #[async_trait]
    impl CodexExecutionService for RecordingCodexHost {
        async fn capabilities(&self) -> anyhow::Result<CodexRuntimeCapabilities> {
            bail!("unused")
        }

        async fn list_skills(&self) -> anyhow::Result<Vec<CodexSkill>> {
            bail!("unused")
        }

        async fn resolve_skills(
            &self,
            _request: CodexSkillExecutionRequest,
        ) -> anyhow::Result<CodexSkillExecutionRequest> {
            bail!("unused")
        }

        async fn create_thread(
            &self,
            request: CodexThreadRequest,
            idempotency_key: &str,
        ) -> anyhow::Result<CodexExecutionHandle> {
            self.requests.lock().unwrap().push(request);
            Ok(CodexExecutionHandle {
                runtime_id: "codex-current-page".to_string(),
                thread_id: "native-thread-1".to_string(),
                execution_id: Some("native-turn-1".to_string()),
                parent_thread_id: None,
                idempotency_key: idempotency_key.to_string(),
            })
        }

        async fn create_subagent(
            &self,
            _parent_thread_id: &str,
            _request: CodexThreadRequest,
            _idempotency_key: &str,
        ) -> anyhow::Result<CodexExecutionHandle> {
            bail!("unused")
        }

        async fn open_thread(&self, _thread_id: &str) -> anyhow::Result<CodexExecutionHandle> {
            bail!("unused")
        }

        async fn continue_thread(
            &self,
            _thread_id: &str,
            _request: CodexThreadRequest,
            _idempotency_key: &str,
        ) -> anyhow::Result<CodexExecutionHandle> {
            bail!("unused")
        }

        async fn cancel_execution(
            &self,
            _thread_id: &str,
            _execution_id: &str,
        ) -> anyhow::Result<CodexExecutionStatus> {
            bail!("unused")
        }

        async fn execution_status(
            &self,
            _thread_id: &str,
            _execution_id: &str,
        ) -> anyhow::Result<CodexExecutionStatus> {
            bail!("unused")
        }

        async fn subscribe_events(
            &self,
            _cursor: Option<&str>,
        ) -> anyhow::Result<Vec<CodexExecutionEvent>> {
            bail!("unused")
        }
    }

    fn dispatch_fixture() -> (
        tempfile::TempDir,
        MulticaExecutionStore,
        LocalMulticaWorkspaceStore,
        String,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let workspace_id = "local-test".to_string();
        let workspace = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        workspace
            .upsert(
                &workspace_id,
                LocalWorkspaceEntityUpsert {
                    resource: MulticaWorkspaceResourceKey::Agents,
                    entity: json!({
                        "id": "agent-a",
                        "name": "修复智能体",
                        "instructions": "先检查日志"
                    }),
                    expected_revision: None,
                },
                1,
            )
            .unwrap();
        workspace
            .upsert(
                &workspace_id,
                LocalWorkspaceEntityUpsert {
                    resource: MulticaWorkspaceResourceKey::Issues,
                    entity: json!({
                        "id": "issue-a",
                        "title": "同步失败",
                        "description": "修复当前页面 host 连接",
                        "assignee_type": "agent",
                        "assignee_id": "agent-a"
                    }),
                    expected_revision: None,
                },
                2,
            )
            .unwrap();
        let executions = MulticaExecutionStore::new(dir.path().join("execution.json"));
        (dir, executions, workspace, workspace_id)
    }

    fn queued_assignment(
        store: &MulticaExecutionStore,
        workspace_id: &str,
    ) -> crate::multica_execution_store::CodexMulticaExecutionBinding {
        store
            .reserve_execution(ExecutionReservation {
                workspace_id: workspace_id.to_string(),
                issue_id: "issue-a".to_string(),
                agent_id: Some("agent-a".to_string()),
                execution_kind: MulticaExecutionKind::Thread,
                parent_thread_id: None,
                parent_attempt_id: None,
                idempotency_key: "issue-assignment:local-test:issue-a:agent-a".to_string(),
                now_ms: 3,
            })
            .unwrap()
            .binding
    }

    #[test]
    fn known_codex_host_call_id_transport_error_gets_stable_code() {
        let error = anyhow::anyhow!(
            "function_call_output requires call_id on HTTP requests; continuation via previous_response_id is only supported on Responses WebSocket v2"
        );
        assert_eq!(
            stable_execution_error_code(&error),
            "codex_host_transport_call_id_required"
        );
    }

    #[test]
    fn assignment_prompt_uses_only_persisted_issue_and_agent_fields() {
        let prompt = assignment_prompt(
            &json!({
                "title": "修复任务同步",
                "description": "排查 host 连接状态",
                "untrusted": "must not be included"
            }),
            &json!({
                "instructions": "先收集日志，再提交最小修复",
                "secret": "must not be included"
            }),
        )
        .unwrap();

        assert_eq!(
            prompt,
            "任务标题：\n修复任务同步\n\n任务描述：\n排查 host 连接状态\n\n智能体指令：\n先收集日志，再提交最小修复"
        );
    }

    #[test]
    fn assignment_prompt_omits_empty_optional_sections() {
        let prompt = assignment_prompt(
            &json!({"title": "仅标题", "description": "  "}),
            &json!({"instructions": ""}),
        )
        .unwrap();

        assert_eq!(prompt, "任务标题：\n仅标题");
    }

    #[tokio::test]
    async fn queued_assignment_dispatches_once_to_the_current_codex_host() {
        let (_dir, executions, workspace, workspace_id) = dispatch_fixture();
        let queued = queued_assignment(&executions, &workspace_id);
        let host = Arc::new(RecordingCodexHost::default());
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_codex_execution_service(host.clone())
            .with_multica_execution_store(executions.clone())
            .with_multica_workspace_store(workspace);
        let request = MulticaExecutionDispatchRequest {
            binding_id: queued.binding_id.clone(),
            expected_revision: queued.revision,
            lease_token: "dispatch-lease-a".to_string(),
        };

        let response = runtime.multica_execution_dispatch(request).await.unwrap();
        assert_eq!(
            response["binding"]["state"],
            json!(MulticaExecutionBindingState::Dispatched)
        );
        assert_eq!(host.requests.lock().unwrap().len(), 1);
        assert_eq!(
            host.requests.lock().unwrap()[0].prompt,
            "任务标题：\n同步失败\n\n任务描述：\n修复当前页面 host 连接\n\n智能体指令：\n先检查日志"
        );

        let dispatched = executions.get_execution(&queued.binding_id).unwrap();
        let replay = runtime
            .multica_execution_dispatch(MulticaExecutionDispatchRequest {
                binding_id: queued.binding_id,
                expected_revision: dispatched.revision,
                lease_token: "dispatch-lease-b".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(replay["handle"]["threadId"], "native-thread-1");
        assert_eq!(host.requests.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn unavailable_host_keeps_assignment_queued_and_releases_lease() {
        let (_dir, executions, workspace, workspace_id) = dispatch_fixture();
        let queued = queued_assignment(&executions, &workspace_id);
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_execution_store(executions.clone())
            .with_multica_workspace_store(workspace);

        let error = runtime
            .multica_execution_dispatch(MulticaExecutionDispatchRequest {
                binding_id: queued.binding_id.clone(),
                expected_revision: queued.revision,
                lease_token: "dispatch-lease-a".to_string(),
            })
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "codex_page_host_unavailable");
        let current = executions.get_execution(&queued.binding_id).unwrap();
        assert_eq!(current.state, MulticaExecutionBindingState::BindingPending);
        assert_eq!(current.lease_token, None);
    }
}
