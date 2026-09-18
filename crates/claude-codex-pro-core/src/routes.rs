use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::codex_execution::{
    CodexExecutionHandle, CodexExecutionService, CodexExecutionStatus,
    CodexPageHostRequestTransport, CodexThreadRequest,
};
use crate::models::{DeleteResult, DeleteStatus, ExportResult, ExportStatus, SessionRef};
use crate::multica_builder::{BuilderRequest, MulticaBuilderStore};
use crate::multica_execution::{
    SkillBindingScope, SkillBindingSelection, SkillBindings, SkillReference, SkillResolutionAudit,
};
use crate::multica_execution_store::{
    AutopilotRunTransition, CodexMulticaExecutionBinding, CodexMulticaTaskMessage,
    ExecutionReservation, MulticaExecutionBindingState, MulticaExecutionCommandKind,
    MulticaExecutionCommandState, MulticaExecutionKind, MulticaExecutionStore, QueueTransition,
};
use crate::multica_skill_trust::review_local_skill;
use crate::multica_webhooks::{MulticaWebhookStore, WebhookTarget};
use crate::multica_workspace::{
    LocalMulticaWorkspaceStore, LocalWorkspaceEntityDelete, LocalWorkspaceEntityUpsert,
    LocalWorkspaceIssueMove, LocalWorkspaceIssueStatusReorder, MulticaAgentCreateCommand,
    MulticaSkillBindingCommand, MulticaSkillBindingRemoveCommand, MulticaSkillBindingsQuery,
    MulticaWorkspaceQuery, MulticaWorkspaceResourceKey, WorkspaceCommand,
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
    diagnostics_enabled: bool,
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
            diagnostics_enabled: true,
        }
    }

    /// Keeps route-level diagnostics out of the user's persistent log when a
    /// test exercises synthetic bridge traffic.
    pub fn without_diagnostics(mut self) -> Self {
        self.diagnostics_enabled = false;
        self
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
    async fn multica_builder(&self, _request: BuilderRequest) -> anyhow::Result<Value> {
        anyhow::bail!("multica_builder_unavailable")
    }
    async fn multica_native_domain(
        &self,
        _request: MulticaNativeDomainRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_native_domain_unavailable")
    }
    async fn multica_webhooks(&self, _request: MulticaWebhookRequest) -> anyhow::Result<Value> {
        anyhow::bail!("multica_webhooks_unavailable")
    }
    async fn multica_workspace_reorder_statuses(
        &self,
        _request: MulticaStatusReorderRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_workspace_mutation_unavailable")
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
    async fn multica_workspace_move_issue(
        &self,
        _request: MulticaWorkspaceMoveIssueRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_workspace_mutation_unavailable")
    }
    async fn multica_workspace_delete(
        &self,
        _request: MulticaWorkspaceDeleteRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_workspace_mutation_unavailable")
    }
    async fn multica_workspace_command(
        &self,
        _request: MulticaWorkspaceCommandRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_workspace_command_unavailable")
    }
    async fn multica_agent_create(
        &self,
        _request: MulticaAgentCreateRequest,
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
    async fn multica_skill_bindings_replace(
        &self,
        _request: MulticaSkillBindingsReplaceAllRequest,
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
    async fn multica_autopilot_tick(&self) -> anyhow::Result<Value> {
        anyhow::bail!("multica_autopilot_unavailable")
    }
    async fn multica_autopilot_transition(
        &self,
        _request: MulticaAutopilotTransitionRequest,
    ) -> anyhow::Result<Value> {
        anyhow::bail!("multica_autopilot_unavailable")
    }
}

#[async_trait]
pub trait BridgeDataService: Send + Sync {
    async fn session_availability(&self, session_ids: Vec<String>) -> anyhow::Result<Vec<String>>;
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
    if ctx.diagnostics_enabled {
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
    }
    let result = match path {
        "/session-availability" => {
            let session_ids = payload
                .get("session_ids")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            ctx.data
                .session_availability(session_ids)
                .await
                .map(|available| {
                    json!({
                        "status": "ok",
                        "available_session_ids": available
                    })
                })
        }
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
        "/multica/agents/env"
        | "/multica/issues/limit-usage"
        | "/multica/autopilots/usage"
        | "/multica/issues/preview-trigger"
        | "/multica/quick-actions/render"
        | "/multica/quick-actions/run" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_native_domain(parse_multica_native_domain(path, &payload)?)
                    .await
            }
            .await
        }
        "/multica/builder" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ensure_multica_payload_size(&payload)?;
                let request = serde_json::from_value::<BuilderRequest>(payload.clone())
                    .map_err(|_| anyhow::anyhow!("builder_request_invalid"))?;
                ctx.runtime.multica_builder(request).await
            }
            .await
        }
        "/multica/webhooks"
        | "/multica/webhooks/provision"
        | "/multica/webhooks/trigger"
        | "/multica/webhooks/rotate"
        | "/multica/webhooks/deliveries"
        | "/multica/webhooks/delivery"
        | "/multica/webhooks/replay"
        | "/multica/webhooks/revoke" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                let request = parse_multica_webhook(path, &payload)?;
                ctx.runtime.multica_webhooks(request).await
            }
            .await
        }
        "/multica/workspace/reorder-statuses" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ensure_multica_payload_size(&payload)?;
                let request =
                    serde_json::from_value::<MulticaStatusReorderRequest>(payload.clone())
                        .map_err(|_| {
                            anyhow::anyhow!("multica_workspace_issue_status_reorder_invalid")
                        })?;
                ctx.runtime
                    .multica_workspace_reorder_statuses(request)
                    .await
            }
            .await
        }
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
        "/multica/workspace/command" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                let request: MulticaWorkspaceCommandRequest =
                    serde_json::from_value(payload.clone())
                        .map_err(|_| anyhow::anyhow!("multica_workspace_command_invalid"))?;
                ctx.runtime.multica_workspace_command(request).await
            }
            .await
        }
        "/multica/workspace/move-issue" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_workspace_move_issue(parse_multica_workspace_move_issue(&payload)?)
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
        "/multica/agents/create" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_agent_create(parse_multica_agent_create(&payload)?)
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
        "/multica/skills/bindings/replace" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_skill_bindings_replace(parse_multica_skill_bindings_replace(&payload)?)
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
                ensure_multica_workspace_enabled(&ctx).await?;
                ctx.runtime
                    .multica_autopilot_trigger(parse_multica_autopilot_trigger(&payload)?)
                    .await
            }
            .await
        }
        "/multica/autopilots/cron-preview" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                let request: MulticaCronPreviewRequest = serde_json::from_value(payload.clone())
                    .map_err(|_| anyhow::anyhow!("autopilot_cron_preview_payload_invalid"))?;
                autopilot_cron_preview(request, unix_now_ms())
            }
            .await
        }
        "/multica/autopilots/tick" => {
            async {
                ensure_multica_workspace_enabled(&ctx).await?;
                if payload.as_object().is_none_or(|object| !object.is_empty()) {
                    anyhow::bail!("autopilot_tick_payload_invalid");
                }
                ctx.runtime.multica_autopilot_tick().await
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
            if ctx.diagnostics_enabled {
                let _ = crate::diagnostic_log::append_diagnostic_log(
                    "bridge.unknown_path",
                    json!({
                        "path": path
                    }),
                );
            }
            return json!({
                "status": "failed",
                "session_id": "",
                "message": "Unknown bridge path"
            });
        }
    };

    let mut response = result.unwrap_or_else(|error| failed_from_error(&payload, error));
    if path == "/multica/workspace/query"
        && response.get("status").and_then(Value::as_str).is_none()
    {
        if let Some(object) = response.as_object_mut() {
            object.insert("status".to_string(), json!("ok"));
        }
    }
    if ctx.diagnostics_enabled {
        let _ = crate::diagnostic_log::append_diagnostic_log(
            "bridge.response",
            json!({
                "path": path,
                "elapsed_ms": started.elapsed().as_millis() as u64,
                "status": response.get("status").and_then(Value::as_str).unwrap_or("")
            }),
        );
    }
    response
}

async fn ensure_multica_workspace_enabled(ctx: &BridgeContext) -> anyhow::Result<()> {
    if ctx.settings.get_settings().await?.multica_workspace_enabled {
        Ok(())
    } else {
        anyhow::bail!("multica_workspace_disabled")
    }
}

const MAX_MULTICA_BRIDGE_PAYLOAD_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MulticaNativeDomainRequest {
    GetEnv {
        agent_id: String,
    },
    SetEnv {
        agent_id: String,
        custom_env: std::collections::BTreeMap<String, String>,
        expected_revision: u64,
        command_id: String,
        command_signature: String,
    },
    IssueUsage,
    AutopilotUsage,
    Preview {
        issue_ids: Vec<String>,
        is_create: bool,
        #[serde(default)]
        assignee_type: Option<String>,
        #[serde(default)]
        assignee_id: Option<String>,
        #[serde(default)]
        status: Option<String>,
    },
    Render {
        issue_id: String,
        quick_action_id: String,
    },
    Run {
        issue_id: String,
        quick_action_id: String,
        expected_issue_revision: u64,
        expected_action_revision: u64,
        command_id: String,
        command_signature: String,
    },
}

fn parse_multica_native_domain(
    path: &str,
    payload: &Value,
) -> anyhow::Result<MulticaNativeDomainRequest> {
    ensure_multica_payload_size(payload)?;
    let mut value = payload.clone();
    let object = value
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("multica_native_request_invalid"))?;
    let operation = match path {
        "/multica/agents/env" => match object
            .remove("operation")
            .and_then(|v| v.as_str().map(str::to_owned))
            .as_deref()
        {
            Some("get") => "get_env",
            Some("set") => "set_env",
            _ => anyhow::bail!("multica_native_request_invalid"),
        },
        other => {
            if object.contains_key("operation") {
                anyhow::bail!("multica_native_request_invalid");
            }
            match other {
                "/multica/issues/limit-usage" => "issue_usage",
                "/multica/autopilots/usage" => "autopilot_usage",
                "/multica/issues/preview-trigger" => "preview",
                "/multica/quick-actions/render" => "render",
                "/multica/quick-actions/run" => "run",
                _ => anyhow::bail!("multica_native_request_invalid"),
            }
        }
    };
    object.insert("operation".into(), json!(operation));
    serde_json::from_value(value).map_err(|_| anyhow::anyhow!("multica_native_request_invalid"))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaStatusReorderRequest {
    pub category: String,
    pub ids: Vec<String>,
    pub expected_revisions: std::collections::BTreeMap<String, u64>,
    pub command_id: String,
    pub command_signature: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MulticaWebhookRequest {
    Provision {
        autopilot_id: String,
        trigger_id: String,
        command_id: String,
    },
    Trigger {
        autopilot_id: String,
        trigger_id: String,
    },
    Rotate {
        autopilot_id: String,
        trigger_id: String,
        expected_revision: u64,
        command_id: String,
        #[serde(default)]
        command_signature: Option<String>,
    },
    Revoke {
        autopilot_id: String,
        trigger_id: String,
        expected_revision: u64,
    },
    Deliveries {
        autopilot_id: String,
        limit: usize,
        offset: usize,
    },
    Delivery {
        autopilot_id: String,
        delivery_id: String,
    },
    Replay {
        autopilot_id: String,
        delivery_id: String,
        command_id: String,
    },
}

fn parse_multica_webhook(path: &str, payload: &Value) -> anyhow::Result<MulticaWebhookRequest> {
    ensure_multica_payload_size(payload)?;
    let mut payload = payload.clone();
    let object = payload
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("webhook_request_invalid"))?;
    if let Some(operation) = path.strip_prefix("/multica/webhooks/") {
        if object.contains_key("operation") {
            anyhow::bail!("webhook_request_invalid");
        }
        object.insert("operation".into(), json!(operation));
    }
    serde_json::from_value(payload).map_err(|_| anyhow::anyhow!("webhook_request_invalid"))
}

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
    #[serde(default)]
    pub suppress_run: bool,
    #[serde(default)]
    pub handoff_note: Option<String>,
    #[serde(default)]
    pub command_id: Option<String>,
    #[serde(default)]
    pub command_signature: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaWorkspaceMoveIssueRequest {
    pub issue_id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub assignee_type: Option<Option<String>>,
    #[serde(default)]
    pub assignee_id: Option<Option<String>>,
    #[serde(default)]
    pub parent_issue_id: Option<Option<String>>,
    #[serde(default)]
    pub project_id: Option<Option<String>>,
    pub before_id: Option<String>,
    pub after_id: Option<String>,
    pub expected_revision: u64,
    #[serde(default, skip_serializing)]
    pub command_id: Option<String>,
    #[serde(default, skip_serializing)]
    pub command_signature: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaWorkspaceDeleteRequest {
    pub resource: MulticaWorkspaceResourceKey,
    pub entity_id: String,
    pub expected_revision: u64,
    #[serde(default)]
    pub command_id: Option<String>,
    #[serde(default)]
    pub command_signature: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaWorkspaceCommandRequest {
    pub command_id: String,
    pub command_signature: String,
}

fn workspace_command(
    id: Option<String>,
    signature: Option<String>,
    operation: String,
    payload: Value,
) -> anyhow::Result<Option<WorkspaceCommand>> {
    match (id, signature) {
        (None, None) => Ok(None),
        (Some(id), Some(signature)) => Ok(Some(WorkspaceCommand::new(
            id, signature, operation, payload,
        )?)),
        _ => anyhow::bail!("multica_workspace_command_invalid"),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaAgentCreateRequest {
    pub entity: Value,
    #[serde(default)]
    pub skills: Vec<SkillReference>,
    #[serde(default)]
    pub command_id: Option<String>,
    #[serde(default)]
    pub command_signature: Option<String>,
}

fn parse_multica_agent_create(payload: &Value) -> anyhow::Result<MulticaAgentCreateRequest> {
    ensure_multica_payload_size(payload)?;
    let request: MulticaAgentCreateRequest = serde_json::from_value(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_agent_create_invalid"))?;
    if !request.entity.is_object() || request.skills.len() > 512 {
        anyhow::bail!("multica_agent_create_invalid");
    }
    for skill in &request.skills {
        if skill.id.trim().is_empty() || skill.id.len() > 240 {
            anyhow::bail!("multica_agent_create_invalid");
        }
    }
    Ok(request)
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
    crate::multica_workspace::issue_run_controls::IssueRunControls {
        suppress_run: request.suppress_run,
        handoff_note: request.handoff_note.clone(),
    }
    .validate(request.resource, &request.entity)?;
    Ok(request)
}

fn parse_multica_workspace_move_issue(
    payload: &Value,
) -> anyhow::Result<MulticaWorkspaceMoveIssueRequest> {
    ensure_multica_payload_size(payload)?;
    let object = payload
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("multica_workspace_move_invalid"))?;
    for key in ["beforeId", "afterId"] {
        if !object.contains_key(key) {
            anyhow::bail!("multica_workspace_move_invalid");
        }
    }
    let mut request: MulticaWorkspaceMoveIssueRequest = serde_json::from_value(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_workspace_move_invalid"))?;
    // Serde's nested Option treats null like a missing field by default.
    for (key, field) in [
        ("assigneeType", &mut request.assignee_type),
        ("assigneeId", &mut request.assignee_id),
        ("parentIssueId", &mut request.parent_issue_id),
        ("projectId", &mut request.project_id),
    ] {
        if object.get(key).is_some_and(Value::is_null) {
            *field = Some(None);
        }
    }
    validate_multica_execution_id(&request.issue_id)
        .map_err(|_| anyhow::anyhow!("multica_workspace_move_invalid"))?;
    if request.expected_revision == 0 {
        anyhow::bail!("multica_workspace_revision_invalid");
    }
    for value in request
        .status
        .as_deref()
        .into_iter()
        .chain(request.assignee_type.as_ref().and_then(Option::as_deref))
        .chain(request.assignee_id.as_ref().and_then(Option::as_deref))
        .chain(request.parent_issue_id.as_ref().and_then(Option::as_deref))
        .chain(request.project_id.as_ref().and_then(Option::as_deref))
        .chain(request.before_id.as_deref())
        .chain(request.after_id.as_deref())
    {
        validate_multica_execution_id(value)
            .map_err(|_| anyhow::anyhow!("multica_workspace_move_invalid"))?;
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
            | MulticaWorkspaceResourceKey::Reactions
            | MulticaWorkspaceResourceKey::Properties
            | MulticaWorkspaceResourceKey::IssueViewPreferences
            | MulticaWorkspaceResourceKey::QuickActions
            | MulticaWorkspaceResourceKey::Projects
            | MulticaWorkspaceResourceKey::ProjectResources
            | MulticaWorkspaceResourceKey::Agents
            | MulticaWorkspaceResourceKey::Squads
            | MulticaWorkspaceResourceKey::Autopilots
            | MulticaWorkspaceResourceKey::IssueViews
            | MulticaWorkspaceResourceKey::IssueStatuses
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaAutopilotTriggerRequest {
    pub autopilot_id: String,
    #[serde(default)]
    pub trigger_id: Option<String>,
    #[serde(default = "default_manual_source")]
    pub source: String,
    #[serde(default)]
    pub occurrence_id: Option<String>,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MulticaCronPreviewRequest {
    expr: String,
    tz: String,
}

fn autopilot_cron_preview(
    request: MulticaCronPreviewRequest,
    now_ms: u64,
) -> anyhow::Result<Value> {
    let mut after_ms = now_ms;
    let mut next_runs = Vec::with_capacity(5);
    for _ in 0..5 {
        after_ms = crate::multica_execution_store::next_autopilot_occurrence(
            &request.expr,
            &request.tz,
            after_ms,
        )?;
        let next = i64::try_from(after_ms)
            .ok()
            .and_then(chrono::DateTime::from_timestamp_millis)
            .ok_or_else(|| anyhow::anyhow!("autopilot_schedule_time_invalid"))?;
        next_runs.push(next.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    }
    Ok(json!({"status":"ok", "next_runs":next_runs}))
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
    let request: MulticaAutopilotTriggerRequest = serde_json::from_value(payload.clone())?;
    validate_multica_execution_id(&request.autopilot_id)?;
    if let Some(id) = request.trigger_id.as_deref() {
        validate_multica_execution_id(id)?;
    }
    if let Some(id) = request.occurrence_id.as_deref() {
        validate_multica_execution_id(id)?;
    }
    if !matches!(request.source.as_str(), "manual" | "webhook" | "api") {
        anyhow::bail!("autopilot_run_source_invalid");
    }
    if request.source != "manual"
        && (request.trigger_id.is_none() || request.occurrence_id.is_none())
    {
        anyhow::bail!("autopilot_occurrence_required");
    }
    Ok(request)
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
        issue_id: Some(request.issue_id.clone()),
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
        issue_id: Some("issue".to_string()),
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaSkillBindingsReplaceAllRequest {
    pub scope_kind: SkillBindingScope,
    pub scope_id: String,
    #[serde(default)]
    pub skills: Vec<SkillReference>,
    #[serde(default)]
    pub expected_revision: Option<u64>,
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

fn parse_multica_skill_bindings_replace(
    payload: &Value,
) -> anyhow::Result<MulticaSkillBindingsReplaceAllRequest> {
    ensure_multica_payload_size(payload)?;
    let request = serde_json::from_value::<MulticaSkillBindingsReplaceAllRequest>(payload.clone())
        .map_err(|_| anyhow::anyhow!("multica_skill_binding_invalid"))?;
    validate_skill_binding_scope(&request.scope_id)?;
    if request.skills.len() > 512 {
        anyhow::bail!("skill_bindings_too_large");
    }
    for skill in &request.skills {
        if skill.id.trim().is_empty() || skill.id.len() > 240 {
            anyhow::bail!("multica_skill_binding_invalid");
        }
        if let Some(digest) = skill.manifest_digest.as_deref()
            && (digest.is_empty() || digest.len() > 128)
        {
            anyhow::bail!("multica_skill_binding_invalid");
        }
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
    codex_execution: Option<Arc<dyn CodexExecutionService>>,
    codex_page_transport: Option<Arc<dyn CodexPageHostRequestTransport>>,
    multica_builder_store: MulticaBuilderStore,
    multica_webhook_store: MulticaWebhookStore,
    multica_execution_store: MulticaExecutionStore,
    multica_workspace_store: LocalMulticaWorkspaceStore,
    autopilot_trigger_lock: Arc<tokio::sync::Mutex<()>>,
    workspace_mutation_lock: Arc<tokio::sync::Mutex<()>>,
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
            codex_execution: None,
            codex_page_transport: None,
            multica_builder_store: MulticaBuilderStore::default(),
            multica_webhook_store: MulticaWebhookStore::default(),
            multica_execution_store: MulticaExecutionStore::default(),
            multica_workspace_store: LocalMulticaWorkspaceStore::default(),
            autopilot_trigger_lock: Arc::new(tokio::sync::Mutex::new(())),
            workspace_mutation_lock: Arc::new(tokio::sync::Mutex::new(())),
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

    /// Attach the current Codex page's native execution adapter used by the
    /// Multica workspace. The bridge never registers or starts a Codex
    /// runtime; production callers must provide the already-open page host.
    pub fn with_codex_execution_service(mut self, service: Arc<dyn CodexExecutionService>) -> Self {
        self.codex_execution = Some(service);
        self
    }

    /// Must be the same current-page transport backing the execution service.
    pub fn with_codex_page_transport(
        mut self,
        transport: Arc<dyn CodexPageHostRequestTransport>,
    ) -> Self {
        self.codex_page_transport = Some(transport);
        self
    }

    pub fn with_multica_builder_store(mut self, store: MulticaBuilderStore) -> Self {
        self.multica_builder_store = store;
        self
    }

    pub fn with_multica_webhook_store(mut self, store: MulticaWebhookStore) -> Self {
        self.multica_webhook_store = store;
        self
    }

    pub fn with_multica_execution_store(mut self, store: MulticaExecutionStore) -> Self {
        self.multica_execution_store = store;
        self
    }

    pub fn with_multica_workspace_store(mut self, store: LocalMulticaWorkspaceStore) -> Self {
        self.multica_builder_store =
            MulticaBuilderStore::new(store.path().with_file_name("builder.json"));
        self.multica_webhook_store =
            MulticaWebhookStore::new(store.path().with_file_name("webhooks.json"));
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
    async fn multica_native_domain(
        &self,
        request: MulticaNativeDomainRequest,
    ) -> anyhow::Result<Value> {
        let workspace_id = crate::multica_workspace::local_workspace_id();
        match request {
            MulticaNativeDomainRequest::GetEnv { agent_id } => {
                self.multica_workspace_store
                    .require_agent_owner(&workspace_id, &agent_id)?;
                let agent = self.workspace_entity(
                    &workspace_id,
                    MulticaWorkspaceResourceKey::Agents,
                    &agent_id,
                )?;
                Ok(
                    json!({"agent_id":agent_id,"custom_env":agent.get("custom_env").cloned().unwrap_or_else(||json!({})),"revision":agent["revision"],"execution_supported":false}),
                )
            }
            MulticaNativeDomainRequest::SetEnv {
                agent_id,
                custom_env,
                expected_revision,
                command_id,
                command_signature,
            } => {
                let _guard = self.workspace_mutation_lock.lock().await;
                self.multica_workspace_store
                    .require_agent_owner(&workspace_id, &agent_id)?;
                if !custom_env.is_empty() {
                    anyhow::bail!("execution_agent_environment_unsupported");
                }
                let receipt = WorkspaceCommand::new(
                    command_id,
                    command_signature,
                    format!("agent-env:{agent_id}"),
                    json!({"agentId":agent_id,"customEnv":custom_env,"expectedRevision":expected_revision}),
                )?;
                if let Some(result) = self
                    .multica_workspace_store
                    .replay_command(&workspace_id, &receipt)?
                {
                    if result.get("custom_env").is_some() {
                        return Ok(result);
                    }
                    return self.multica_workspace_store.complete_command(
                        &workspace_id,
                        Some(&receipt),
                        environment_result(&result["entity"]),
                    );
                }
                let mut agent = self.workspace_entity(
                    &workspace_id,
                    MulticaWorkspaceResourceKey::Agents,
                    &agent_id,
                )?;
                agent["custom_env"] = json!(custom_env);
                let saved = self.multica_workspace_store.upsert_with_command(
                    &workspace_id,
                    LocalWorkspaceEntityUpsert {
                        resource: MulticaWorkspaceResourceKey::Agents,
                        entity: agent,
                        expected_revision: Some(expected_revision),
                    },
                    unix_now_ms(),
                    Some(&receipt),
                )?;
                self.multica_workspace_store.complete_command(
                    &workspace_id,
                    Some(&receipt),
                    environment_result(&saved),
                )
            }
            MulticaNativeDomainRequest::IssueUsage => {
                Ok(crate::multica_workspace::native_domain::issue_limit_usage())
            }
            MulticaNativeDomainRequest::AutopilotUsage => {
                Ok(crate::multica_workspace::native_domain::autopilot_usage())
            }
            MulticaNativeDomainRequest::Preview {
                issue_ids,
                is_create,
                assignee_type,
                assignee_id,
                status,
            } => crate::multica_workspace::native_domain::preview_issue_trigger(
                &self.multica_workspace_store,
                &self.multica_execution_store,
                &workspace_id,
                &crate::multica_workspace::native_domain::IssueTriggerPreviewRequest {
                    issue_ids,
                    is_create,
                    assignee_type,
                    assignee_id,
                    status,
                },
            ),
            MulticaNativeDomainRequest::Render {
                issue_id,
                quick_action_id,
            } => crate::multica_workspace::native_domain::render_quick_action(
                &self.multica_workspace_store,
                &workspace_id,
                &crate::multica_workspace::native_domain::QuickActionRenderRequest {
                    issue_id,
                    quick_action_id,
                },
            ),
            MulticaNativeDomainRequest::Run {
                issue_id,
                quick_action_id,
                expected_issue_revision,
                expected_action_revision,
                command_id,
                command_signature,
            } => {
                let _guard = self.workspace_mutation_lock.lock().await;
                crate::multica_workspace::native_domain::run_quick_action(
                    &self.multica_workspace_store,
                    &self.multica_execution_store,
                    self,
                    &workspace_id,
                    &crate::multica_workspace::native_domain::QuickActionRunRequest {
                        issue_id,
                        quick_action_id,
                        expected_issue_revision,
                        expected_action_revision,
                        command_id,
                        command_signature,
                    },
                    unix_now_ms(),
                )
                .await
            }
        }
    }

    async fn multica_builder(&self, request: BuilderRequest) -> anyhow::Result<Value> {
        let service = self.codex_execution_service()?;
        let workspace_id = crate::multica_workspace::local_workspace_id();
        self.multica_builder_store
            .handle(
                service.as_ref(),
                self.codex_page_transport.as_deref(),
                &workspace_id,
                &format!("{workspace_id}-user"),
                request,
                unix_now_ms(),
            )
            .await
    }

    async fn multica_workspace_reorder_statuses(
        &self,
        request: MulticaStatusReorderRequest,
    ) -> anyhow::Result<Value> {
        let _guard = self.workspace_mutation_lock.lock().await;
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let reorder = LocalWorkspaceIssueStatusReorder {
            category: request.category,
            ordered_ids: request.ids,
            expected_revisions: request.expected_revisions,
        };
        let receipt = WorkspaceCommand::new(
            request.command_id,
            request.command_signature,
            format!("reorder-statuses:{}", reorder.category),
            serde_json::to_value(&reorder)?,
        )?;
        let statuses = self.multica_workspace_store.reorder_issue_statuses(
            &workspace_id,
            &reorder,
            unix_now_ms(),
            Some(&receipt),
        )?;
        self.multica_workspace_store.complete_command(
            &workspace_id,
            Some(&receipt),
            json!({"status":"ok","statuses":statuses}),
        )
    }

    async fn multica_webhooks(&self, request: MulticaWebhookRequest) -> anyhow::Result<Value> {
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let autopilot_id = match &request {
            MulticaWebhookRequest::Provision { autopilot_id, .. }
            | MulticaWebhookRequest::Trigger { autopilot_id, .. }
            | MulticaWebhookRequest::Rotate { autopilot_id, .. }
            | MulticaWebhookRequest::Revoke { autopilot_id, .. }
            | MulticaWebhookRequest::Deliveries { autopilot_id, .. }
            | MulticaWebhookRequest::Delivery { autopilot_id, .. }
            | MulticaWebhookRequest::Replay { autopilot_id, .. } => autopilot_id,
        };
        let autopilot = self.webhook_autopilot(&workspace_id, autopilot_id)?;
        let user_id = format!("{workspace_id}-user");
        let creator = autopilot
            .get("created_by_id")
            .or_else(|| autopilot.get("createdById"))
            .and_then(Value::as_str)
            .unwrap_or(&user_id)
            == user_id;
        let collaborator = autopilot["collaborators"].as_array().is_some_and(|items| {
            items
                .iter()
                .any(|item| item["user_id"] == user_id || item["userId"] == user_id)
        });
        let manage = matches!(
            &request,
            MulticaWebhookRequest::Provision { .. }
                | MulticaWebhookRequest::Rotate { .. }
                | MulticaWebhookRequest::Revoke { .. }
        );
        if !creator && (manage || !collaborator) {
            anyhow::bail!("webhook_access_denied");
        }
        match request {
            MulticaWebhookRequest::Provision {
                trigger_id,
                command_id,
                ..
            } => {
                let target = WebhookTarget::from_autopilot(&workspace_id, &autopilot, &trigger_id)?;
                self.multica_webhook_store
                    .provision(&target, &command_id, unix_now_ms())
            }
            MulticaWebhookRequest::Trigger { trigger_id, .. } => {
                let target = WebhookTarget::from_autopilot(&workspace_id, &autopilot, &trigger_id)?;
                self.multica_webhook_store.trigger(&target)
            }
            MulticaWebhookRequest::Rotate {
                trigger_id,
                expected_revision,
                command_id,
                command_signature,
                ..
            } => {
                let target = WebhookTarget::from_autopilot(&workspace_id, &autopilot, &trigger_id)?;
                match command_signature {
                    Some(signature) => self.multica_webhook_store.rotate_with_signature(
                        &target,
                        expected_revision,
                        &command_id,
                        &signature,
                        unix_now_ms(),
                    ),
                    None => self.multica_webhook_store.rotate(
                        &target,
                        expected_revision,
                        &command_id,
                        unix_now_ms(),
                    ),
                }
            }
            MulticaWebhookRequest::Revoke {
                trigger_id,
                expected_revision,
                ..
            } => {
                let target = WebhookTarget::from_autopilot(&workspace_id, &autopilot, &trigger_id)?;
                self.multica_webhook_store
                    .revoke(&target, expected_revision)?;
                Ok(json!({"status":"ok"}))
            }
            MulticaWebhookRequest::Deliveries {
                autopilot_id,
                limit,
                offset,
            } => self
                .multica_webhook_store
                .list(&workspace_id, &autopilot_id, limit, offset),
            MulticaWebhookRequest::Delivery {
                autopilot_id,
                delivery_id,
            } => self
                .multica_webhook_store
                .get(&workspace_id, &autopilot_id, &delivery_id),
            MulticaWebhookRequest::Replay {
                autopilot_id,
                delivery_id,
                command_id,
            } => {
                let delivery =
                    self.multica_webhook_store
                        .get(&workspace_id, &autopilot_id, &delivery_id)?;
                let trigger_id = delivery["trigger_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("webhook_store_invalid"))?;
                let target = WebhookTarget::from_autopilot(&workspace_id, &autopilot, trigger_id)?;
                let replay = self.multica_webhook_store.replay(
                    &target,
                    &delivery_id,
                    &command_id,
                    unix_now_ms(),
                )?;
                let id = replay["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("webhook_store_invalid"))?;
                self.multica_webhook_store.enqueue(
                    &self.multica_execution_store,
                    &target,
                    id,
                    unix_now_ms(),
                )
            }
        }
    }

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
        let mut value = serde_json::to_value(collection)?;
        // Workspace collections use the same bridge envelope as mutations.
        // Keep an explicit success status even when the collection is empty.
        if let Some(object) = value.as_object_mut() {
            object.insert("status".to_string(), json!("ok"));
        }
        Ok(value)
    }

    async fn multica_workspace_upsert(
        &self,
        request: MulticaWorkspaceUpsertRequest,
    ) -> anyhow::Result<Value> {
        let _guard = self.workspace_mutation_lock.lock().await;
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let controls = crate::multica_workspace::issue_run_controls::IssueRunControls {
            suppress_run: request.suppress_run,
            handoff_note: request.handoff_note.clone(),
        };
        controls.validate(request.resource, &request.entity)?;
        let command = workspace_command(
            request.command_id,
            request.command_signature,
            format!(
                "upsert:{:?}:{}",
                request.resource,
                request
                    .entity
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            ),
            json!({"entity":request.entity,"expectedRevision":request.expected_revision,
                "suppressRun":request.suppress_run,"handoffNote":request.handoff_note}),
        )?;
        if let Some(command) = &command {
            if let Some(result) = self
                .multica_workspace_store
                .replay_command(&workspace_id, command)?
            {
                return Ok(result);
            }
        }
        let entity = self.multica_workspace_store.upsert_with_run_controls(
            &workspace_id,
            LocalWorkspaceEntityUpsert {
                resource: request.resource,
                entity: request.entity,
                expected_revision: request.expected_revision,
            },
            unix_now_ms(),
            command.as_ref(),
            Some((&controls, &self.multica_execution_store)),
        )?;
        let queue = if request.resource == MulticaWorkspaceResourceKey::Issues
            && self
                .multica_workspace_store
                .issue_run_eligible(&workspace_id, &entity)?
        {
            self.queue_issue_assignment(&workspace_id, &entity).await?
        } else {
            None
        };
        self.multica_workspace_store.complete_command(
            &workspace_id,
            command.as_ref(),
            json!({"status": "ok", "entity": entity, "queue": queue}),
        )
    }

    async fn multica_workspace_move_issue(
        &self,
        request: MulticaWorkspaceMoveIssueRequest,
    ) -> anyhow::Result<Value> {
        let _guard = self.workspace_mutation_lock.lock().await;
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let command = workspace_command(
            request.command_id.clone(),
            request.command_signature.clone(),
            format!("move:{}", request.issue_id),
            serde_json::to_value(&request)?,
        )?;
        if let Some(command) = &command {
            if let Some(result) = self
                .multica_workspace_store
                .replay_command(&workspace_id, command)?
            {
                return Ok(result);
            }
        }
        let entity = self
            .multica_workspace_store
            .move_issue_with_execution_store(
                &workspace_id,
                LocalWorkspaceIssueMove {
                    issue_id: request.issue_id,
                    status: request.status,
                    assignee_type: request.assignee_type,
                    assignee_id: request.assignee_id,
                    parent_issue_id: request.parent_issue_id,
                    project_id: request.project_id,
                    before_id: request.before_id,
                    after_id: request.after_id,
                    expected_revision: request.expected_revision,
                },
                unix_now_ms(),
                command.as_ref(),
                Some(&self.multica_execution_store),
            )?;
        let queue = if self
            .multica_workspace_store
            .issue_run_eligible(&workspace_id, &entity)?
        {
            self.queue_issue_assignment(&workspace_id, &entity).await?
        } else {
            None
        };
        self.multica_workspace_store.complete_command(
            &workspace_id,
            command.as_ref(),
            json!({"status": "ok", "entity": entity, "queue": queue}),
        )
    }

    async fn multica_workspace_delete(
        &self,
        request: MulticaWorkspaceDeleteRequest,
    ) -> anyhow::Result<Value> {
        let _guard = self.workspace_mutation_lock.lock().await;
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let command = workspace_command(
            request.command_id,
            request.command_signature,
            format!("delete:{:?}:{}", request.resource, request.entity_id),
            json!({"entityId":request.entity_id,"expectedRevision":request.expected_revision}),
        )?;
        if let Some(command) = &command {
            if let Some(result) = self
                .multica_workspace_store
                .replay_command(&workspace_id, command)?
            {
                return Ok(result);
            }
        }
        let entity_id = request.entity_id;
        let deleted = self.multica_workspace_store.delete_with_command(
            &workspace_id,
            LocalWorkspaceEntityDelete {
                resource: request.resource,
                entity_id: entity_id.clone(),
                expected_revision: request.expected_revision,
            },
            command.as_ref(),
        )?;
        if deleted && request.resource == MulticaWorkspaceResourceKey::Issues {
            self.multica_execution_store.cancel_active_for_issue(
                &workspace_id,
                &entity_id,
                None,
                unix_now_ms(),
            )?;
        }
        self.multica_workspace_store.complete_command(
            &workspace_id,
            command.as_ref(),
            json!({"status": "ok", "deleted": deleted, "entityId": entity_id}),
        )
    }

    async fn multica_workspace_command(
        &self,
        request: MulticaWorkspaceCommandRequest,
    ) -> anyhow::Result<Value> {
        let _guard = self.workspace_mutation_lock.lock().await;
        let workspace_id = crate::multica_workspace::local_workspace_id();
        crate::multica_workspace::recover_pending_agent_create(
            &self.multica_workspace_store,
            &self.multica_execution_store,
        )?;
        if let Some((command, mut result)) = self.multica_workspace_store.pending_command(
            &workspace_id,
            &request.command_id,
            &request.command_signature,
        )? {
            if command.operation.starts_with("upsert:") || command.operation.starts_with("move:") {
                let queue = if (command.operation.starts_with("upsert:Issues:")
                    || command.operation.starts_with("move:"))
                    && result.get("issueRunEligible") != Some(&json!(false))
                {
                    self.queue_issue_assignment(&workspace_id, &result["entity"])
                        .await?
                } else {
                    None
                };
                result["queue"] = json!(queue);
            } else if command.operation.starts_with("delete:") {
                if command.operation.starts_with("delete:Issues:") && result["deleted"] == true {
                    let id = result["entityId"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("multica_workspace_command_invalid"))?;
                    self.multica_execution_store.cancel_active_for_issue(
                        &workspace_id,
                        id,
                        None,
                        unix_now_ms(),
                    )?;
                }
            } else if command.operation.starts_with("quick-action:") {
                result = crate::multica_workspace::native_domain::recover_quick_action(
                    &self.multica_workspace_store,
                    &self.multica_execution_store,
                    self,
                    &workspace_id,
                    &command,
                    unix_now_ms(),
                )
                .await?;
            } else if command.operation.starts_with("agent-env:") {
                result = environment_result(&result["entity"]);
            } else if command.operation.starts_with("reorder-statuses:") {
                // The reorder and receipt were committed by one workspace save.
            } else {
                anyhow::bail!("multica_workspace_command_pending");
            }
            self.multica_workspace_store
                .complete_command(&workspace_id, Some(&command), result)?;
        }
        let result = self.multica_workspace_store.command_result(
            &workspace_id,
            &request.command_id,
            &request.command_signature,
        )?;
        Ok(json!({"status":"ok", "found":result.is_some(), "result":result}))
    }

    async fn multica_agent_create(
        &self,
        request: MulticaAgentCreateRequest,
    ) -> anyhow::Result<Value> {
        let _guard = self.workspace_mutation_lock.lock().await;
        let command = workspace_command(
            request.command_id,
            request.command_signature,
            format!(
                "agent-create:{}",
                request
                    .entity
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            ),
            json!({"entity":request.entity,"skills":request.skills}),
        )?;
        crate::multica_workspace::create_agent_with_skill_bindings_idempotent(
            MulticaAgentCreateCommand {
                entity: request.entity,
                skills: request.skills,
            },
            &self.multica_workspace_store,
            &self.multica_execution_store,
            self.codex_execution.clone(),
            command,
        )
        .await
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

    async fn multica_skill_bindings_replace(
        &self,
        request: MulticaSkillBindingsReplaceAllRequest,
    ) -> anyhow::Result<Value> {
        let service = self
            .codex_execution
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("codex_page_host_unavailable"))?;
        crate::multica_workspace::replace_skill_bindings_with_codex_runtime(
            crate::multica_workspace::MulticaSkillBindingsReplaceAllCommand {
                scope_kind: request.scope_kind,
                scope_id: request.scope_id,
                skills: request.skills,
                expected_revision: request.expected_revision,
            },
            Arc::clone(service),
        )
        .await
    }

    async fn multica_execution_create(
        &self,
        request: MulticaExecutionCreateRequest,
    ) -> anyhow::Result<Value> {
        self.validate_agent_runtime_selection(&request.workspace_id, request.agent_id.as_deref())?;
        let service = self.codex_execution_service()?;
        let now_ms = unix_now_ms();
        let mut reserved =
            self.multica_execution_store
                .reserve_execution(ExecutionReservation {
                    workspace_id: request.workspace_id.clone(),
                    issue_id: Some(request.issue_id.clone()),
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
        reserved.binding = self
            .multica_execution_store
            .claim_execution_lease_with_capacity(
                &reserved.binding.binding_id,
                reserved.binding.revision,
                &format!("create-{}", reserved.binding.binding_id),
                now_ms,
                30_000,
                self.agent_concurrency_limit(&reserved.binding)?,
            )?;
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
            issue_id: Some(request.issue_id),
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
        self.require_execution_agent_access(&binding)?;
        if binding.revision != expected_revision {
            anyhow::bail!("execution_revision_conflict");
        }
        if binding.state == MulticaExecutionBindingState::Dispatched {
            let handle = execution_handle_from_binding(&binding, &binding.idempotency_key)?;
            self.multica_workspace_store.audit_issue_handoff(
                &binding.workspace_id,
                &binding.idempotency_key,
                &binding.binding_id,
                unix_now_ms(),
            )?;
            return Ok(execution_handle_response(binding, handle));
        }
        if binding.state != MulticaExecutionBindingState::BindingPending {
            anyhow::bail!("execution_not_dispatchable");
        }

        let claimed = self
            .multica_execution_store
            .claim_execution_lease_with_capacity(
                binding_id,
                expected_revision,
                lease_token,
                unix_now_ms(),
                30_000,
                self.agent_concurrency_limit(&binding)?,
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
            let issue = claimed.issue_id.as_deref().and_then(|issue_id| {
                issues
                    .into_iter()
                    .find(|issue| issue.get("id").and_then(Value::as_str) == Some(issue_id))
            });
            let issue = if let Some(issue) = issue {
                if issue.get("assignee_type").and_then(Value::as_str) != Some("agent")
                    || issue.get("assignee_id").and_then(Value::as_str) != Some(agent_id)
                {
                    anyhow::bail!("execution_assignment_changed");
                }
                issue
            } else {
                // `run_only` has no Issue by design. Resolve the owning run
                // through its persisted task binding; never invent an Issue
                // identifier merely to satisfy the dispatch path.
                let run = self
                    .multica_execution_store
                    .get_autopilot_run_by_task_id(&claimed.binding_id)
                    .map_err(|_| anyhow::anyhow!("execution_autopilot_unavailable"))?;
                let autopilots = self.multica_workspace_store.list(
                    &claimed.workspace_id,
                    MulticaWorkspaceResourceKey::Autopilots,
                )?;
                let autopilot = autopilots
                    .into_iter()
                    .find(|item| {
                        item.get("id").and_then(Value::as_str) == Some(run.autopilot_id.as_str())
                    })
                    .ok_or_else(|| anyhow::anyhow!("execution_autopilot_unavailable"))?;
                json!({
                    "id": run.id,
                    "title": autopilot.get("title").or_else(|| autopilot.get("name")),
                    "description": autopilot.get("description"),
                    "assignee_type": "agent",
                    "assignee_id": agent_id,
                    "origin_type": "autopilot",
                    "origin_id": run.autopilot_id
                })
            };
            let agents = self
                .multica_workspace_store
                .list(&claimed.workspace_id, MulticaWorkspaceResourceKey::Agents)?;
            let agent = agents
                .into_iter()
                .find(|agent| agent.get("id").and_then(Value::as_str) == Some(agent_id))
                .ok_or_else(|| anyhow::anyhow!("execution_agent_unavailable"))?;
            self.validate_agent_runtime_selection(&claimed.workspace_id, Some(agent_id))?;
            // Agent bindings are authoritative only after the current page
            // host resolves their pinned digests against its live inventory.
            // Do not silently create a thread without requested Skills when
            // a persisted binding is no longer dispatchable.
            let bindings = agent_skill_bindings(
                &self.multica_execution_store,
                &claimed.workspace_id,
                agent_id,
            )?;
            let (skill_request, skill_audit) =
                resolve_execution_skills(Arc::clone(&service), bindings).await?;
            if let Some(audit) = skill_audit.as_ref() {
                self.multica_execution_store.reserve_attempt_snapshot(
                    &claimed.binding_id,
                    claimed.attempt_no,
                    audit,
                    unix_now_ms(),
                )?;
            }
            let mut prompt = assignment_prompt(&issue, &agent)?;
            if let Some(note) = self
                .multica_workspace_store
                .issue_handoff_note(&claimed.workspace_id, &claimed.idempotency_key)?
            {
                prompt.push_str("\n\nHandoff note:\n");
                prompt.push_str(&note);
            }
            let native_request = CodexThreadRequest {
                workspace_id: claimed.workspace_id.clone(),
                issue_id: claimed.issue_id.clone(),
                prompt,
                cwd: None,
                skill_request,
            };
            native_request.validate()?;
            // Preserve the execution kind recorded at reservation time.  A
            // subagent binding must fork the verified parent thread; falling
            // back to thread/start would silently lose the native parent /
            // child relationship while still reporting success.
            let handle = if claimed.execution_kind == MulticaExecutionKind::Subagent {
                let parent_thread_id = claimed
                    .parent_thread_id
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("subagent_parent_or_agent_required"))?;
                service
                    .create_subagent(parent_thread_id, native_request, &claimed.idempotency_key)
                    .await
            } else {
                service
                    .create_thread(native_request, &claimed.idempotency_key)
                    .await
            }
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
                self.multica_workspace_store.audit_issue_handoff(
                    &binding.workspace_id,
                    &binding.idempotency_key,
                    &binding.binding_id,
                    unix_now_ms(),
                )?;
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
        self.require_execution_agent_access(&binding)?;
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
        self.require_execution_agent_access(&binding)?;
        use sha2::{Digest, Sha256};
        let request_hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&request)?));
        if let Some(command) = self.multica_execution_store.replay_command_request(
            &binding.binding_id,
            MulticaExecutionCommandKind::Continue,
            &request.idempotency_key,
            &request_hash,
        )? {
            return execution_command_replay(command);
        }
        self.validate_agent_runtime_selection(&binding.workspace_id, binding.agent_id.as_deref())?;
        if binding.revision != request.expected_revision {
            anyhow::bail!("execution_revision_conflict");
        }
        if !matches!(
            binding.state,
            MulticaExecutionBindingState::Completed | MulticaExecutionBindingState::Stale
        ) {
            anyhow::bail!("execution_not_continuable");
        }
        let thread_id = binding
            .codex_thread_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_binding_pending"))?;
        let command = self.multica_execution_store.reserve_command_request(
            &binding.binding_id,
            MulticaExecutionCommandKind::Continue,
            &request.idempotency_key,
            request.expected_revision,
            unix_now_ms(),
            self.agent_concurrency_limit(&binding)?,
            &request_hash,
        )?;
        if command.replay {
            return execution_command_replay(command.command);
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
        self.require_execution_agent_access(&binding)?;
        use sha2::{Digest, Sha256};
        let request_hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&request)?));
        if let Some(command) = self.multica_execution_store.replay_command_request(
            &binding.binding_id,
            MulticaExecutionCommandKind::Cancel,
            &request.idempotency_key,
            &request_hash,
        )? {
            return execution_command_replay(command);
        }
        let thread_id = binding
            .codex_thread_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_binding_pending"))?;
        let execution_id = binding
            .codex_execution_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("execution_id_unavailable"))?;
        let command = self.multica_execution_store.reserve_command_request(
            &binding.binding_id,
            MulticaExecutionCommandKind::Cancel,
            &request.idempotency_key,
            request.expected_revision,
            unix_now_ms(),
            None,
            &request_hash,
        )?;
        if command.replay {
            return execution_command_replay(command.command);
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
        self.require_execution_agent_access(&binding)?;
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
        let candidate = self
            .multica_execution_store
            .get_execution(&request.binding_id)?;
        let binding = self
            .multica_execution_store
            .claim_execution_lease_with_capacity(
                &request.binding_id,
                request.expected_revision,
                &request.lease_token,
                unix_now_ms(),
                request.lease_duration_ms,
                self.agent_concurrency_limit(&candidate)?,
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
        self.materialize_autopilot_run(request, true).await
    }

    async fn multica_autopilot_tick(&self) -> anyhow::Result<Value> {
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let mut webhook_diagnostics = self.recover_pending_webhooks(&workspace_id);
        let autopilots = self
            .multica_workspace_store
            .list(&workspace_id, MulticaWorkspaceResourceKey::Autopilots)?;
        let batch = self
            .multica_execution_store
            .tick_autopilots(&autopilots, unix_now_ms())?;
        let mut runs = Vec::new();
        let mut diagnostics = batch.diagnostics;
        diagnostics.append(&mut webhook_diagnostics);
        for run in batch.runs {
            let result = self
                .materialize_autopilot_run(
                    MulticaAutopilotTriggerRequest {
                        autopilot_id: run.autopilot_id.clone(),
                        trigger_id: run.trigger_id.clone(),
                        source: run.source.clone(),
                        occurrence_id: run.occurrence_id.clone(),
                    },
                    false,
                )
                .await;
            match result {
                Ok(value) => runs.push(value),
                Err(error) => {
                    let code = stable_execution_error_code(&error);
                    // Invalid persisted definitions must not monopolize the bounded retry batch.
                    if matches!(
                        code.as_str(),
                        "autopilot_assignee_unavailable"
                            | "autopilot_execution_mode_invalid"
                            | "autopilot_occurrence_invalid"
                    ) {
                        let current = self.multica_execution_store.get_autopilot_run(&run.id)?;
                        if current.status == "pending" && current.task_id.is_none() {
                            self.multica_execution_store.transition_autopilot_run(
                                AutopilotRunTransition {
                                    autopilot_id: current.autopilot_id,
                                    run_id: current.id,
                                    expected_revision: current.revision,
                                    next_status: "failed".into(),
                                    issue_id: None,
                                    task_id: None,
                                    failure_reason: Some(code.clone()),
                                    reason_code: Some(code.clone()),
                                    now_ms: unix_now_ms(),
                                },
                            )?;
                        }
                    }
                    diagnostics.push(json!({"autopilotId":run.autopilot_id,"triggerId":run.trigger_id,"code":code}));
                }
            }
        }
        diagnostics.extend(self.recover_pending_webhooks(&workspace_id));
        Ok(json!({"status":"ok", "runs":runs, "diagnostics":diagnostics, "hasMore":batch.has_more}))
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
}

impl CoreRuntimeService {
    fn workspace_entity(
        &self,
        workspace_id: &str,
        resource: MulticaWorkspaceResourceKey,
        id: &str,
    ) -> anyhow::Result<Value> {
        self.multica_workspace_store
            .list(workspace_id, resource)?
            .into_iter()
            .find(|row| row["id"] == id)
            .ok_or_else(|| anyhow::anyhow!("multica_workspace_entity_not_found"))
    }

    fn require_execution_agent_access(
        &self,
        binding: &CodexMulticaExecutionBinding,
    ) -> anyhow::Result<()> {
        if let Some(agent_id) = binding.agent_id.as_deref() {
            self.multica_workspace_store
                .require_agent_invocation(&binding.workspace_id, agent_id)?;
        }
        Ok(())
    }

    fn webhook_autopilot(&self, workspace_id: &str, autopilot_id: &str) -> anyhow::Result<Value> {
        validate_multica_execution_id(autopilot_id)?;
        self.multica_workspace_store
            .list(workspace_id, MulticaWorkspaceResourceKey::Autopilots)?
            .into_iter()
            .find(|row| row["id"] == autopilot_id)
            .ok_or_else(|| anyhow::anyhow!("webhook_target_not_found"))
    }

    fn recover_pending_webhooks(&self, workspace_id: &str) -> Vec<Value> {
        let pending = match self.multica_webhook_store.pending(100) {
            Ok(rows) => rows,
            Err(error) => return vec![json!({"code":stable_execution_error_code(&error)})],
        };
        let mut diagnostics = Vec::new();
        for row in pending {
            if row["workspace_id"] != workspace_id {
                continue;
            }
            let result = (|| -> anyhow::Result<()> {
                let autopilot = self.webhook_autopilot(
                    workspace_id,
                    row["autopilot_id"].as_str().unwrap_or_default(),
                )?;
                let target = WebhookTarget::from_autopilot(
                    workspace_id,
                    &autopilot,
                    row["trigger_id"].as_str().unwrap_or_default(),
                )?;
                self.multica_webhook_store.enqueue(
                    &self.multica_execution_store,
                    &target,
                    row["id"].as_str().unwrap_or_default(),
                    unix_now_ms(),
                )?;
                Ok(())
            })();
            if let Err(error) = result {
                diagnostics.push(
                    json!({"deliveryId":row["id"],"code":stable_execution_error_code(&error)}),
                );
            }
        }
        diagnostics
    }

    fn agent_concurrency_limit(
        &self,
        binding: &crate::multica_execution_store::CodexMulticaExecutionBinding,
    ) -> anyhow::Result<Option<u64>> {
        let Some(agent_id) = binding.agent_id.as_deref() else {
            return Ok(None);
        };
        self.multica_workspace_store
            .require_agent_invocation(&binding.workspace_id, agent_id)?;
        let agent = self
            .multica_workspace_store
            .list(&binding.workspace_id, MulticaWorkspaceResourceKey::Agents)?
            .into_iter()
            .find(|agent| agent.get("id").and_then(Value::as_str) == Some(agent_id))
            .ok_or_else(|| anyhow::anyhow!("execution_agent_unavailable"))?;
        Ok(Some(
            agent
                .get("max_concurrent_tasks")
                .and_then(Value::as_u64)
                .unwrap_or(1),
        ))
    }

    fn validate_agent_runtime_selection(
        &self,
        workspace_id: &str,
        agent_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let Some(agent_id) = agent_id else {
            return Ok(());
        };
        self.multica_workspace_store
            .require_agent_invocation(workspace_id, agent_id)?;
        if let Some(agent) = self
            .multica_workspace_store
            .list(workspace_id, MulticaWorkspaceResourceKey::Agents)?
            .into_iter()
            .find(|agent| agent.get("id").and_then(Value::as_str) == Some(agent_id))
        {
            if agent["status"] == "archived"
                || agent["archived"] == true
                || agent
                    .get("archived_at")
                    .is_some_and(|value| !value.is_null())
            {
                anyhow::bail!("execution_agent_archived");
            }
            // The native selector remains authoritative; saved overrides are not silently ignored.
            if agent
                .get("custom_env")
                .and_then(Value::as_object)
                .is_some_and(|env| !env.is_empty())
            {
                anyhow::bail!("execution_agent_environment_unsupported");
            }
            if ["model", "thinking_level", "service_tier"]
                .iter()
                .any(|field| {
                    agent
                        .get(*field)
                        .and_then(Value::as_str)
                        .is_some_and(|value| !value.trim().is_empty())
                })
            {
                anyhow::bail!("execution_agent_runtime_override_unsupported");
            }
        }
        Ok(())
    }
    async fn queue_issue_assignment(
        &self,
        workspace_id: &str,
        entity: &Value,
    ) -> anyhow::Result<Option<Value>> {
        if entity.get("assignee_type").and_then(Value::as_str) != Some("agent") {
            return Ok(None);
        }
        let Some(agent_id) = entity
            .get("assignee_id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
        else {
            return Ok(None);
        };
        let issue_id = entity
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("multica_workspace_entity_invalid"))?;
        let reservation = self
            .multica_execution_store
            .reserve_execution(ExecutionReservation {
                workspace_id: workspace_id.to_string(),
                issue_id: Some(issue_id.to_string()),
                agent_id: Some(agent_id.to_string()),
                execution_kind: MulticaExecutionKind::Thread,
                parent_thread_id: None,
                parent_attempt_id: None,
                idempotency_key: format!("issue-assignment:{workspace_id}:{issue_id}:{agent_id}"),
                now_ms: unix_now_ms(),
            })?;
        let dispatched = match self
            .dispatch_pending_assignment(
                &reservation.binding.binding_id,
                reservation.binding.revision,
                &format!("auto-{}", reservation.binding.binding_id),
            )
            .await
        {
            Ok(value) => value,
            Err(error) => json!({
                "status": "queued",
                "diagnostic": stable_execution_error_code(&error),
            }),
        };
        Ok(Some(json!({
            "binding_id": reservation.binding.binding_id,
            "status": dispatched.get("binding")
                .and_then(|binding| binding.get("state"))
                .and_then(Value::as_str)
                .unwrap_or("queued"),
            "replay": reservation.replay,
            "dispatch": dispatched,
        })))
    }

    async fn materialize_autopilot_run(
        &self,
        request: MulticaAutopilotTriggerRequest,
        dispatch_now: bool,
    ) -> anyhow::Result<Value> {
        let _guard = self.autopilot_trigger_lock.lock().await;
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let autopilot = self
            .multica_workspace_store
            .list(&workspace_id, MulticaWorkspaceResourceKey::Autopilots)?
            .into_iter()
            .find(|item| {
                item.get("id").and_then(Value::as_str) == Some(request.autopilot_id.as_str())
            })
            .ok_or_else(|| anyhow::anyhow!("autopilot_not_found"))?;
        let status = autopilot
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("active");
        if status != "active" {
            anyhow::bail!("autopilot_not_active");
        }
        if !matches!(
            request.source.as_str(),
            "manual" | "schedule" | "webhook" | "api"
        ) {
            anyhow::bail!("autopilot_run_source_invalid");
        }
        if request.source != "manual" || request.trigger_id.is_some() {
            let trigger = autopilot
                .get("triggers")
                .and_then(Value::as_array)
                .and_then(|triggers| {
                    triggers.iter().find(|trigger| {
                        trigger.get("id").and_then(Value::as_str) == request.trigger_id.as_deref()
                    })
                })
                .ok_or_else(|| anyhow::anyhow!("autopilot_trigger_unknown"))?;
            if trigger.get("enabled").and_then(Value::as_bool) != Some(true) {
                anyhow::bail!("autopilot_trigger_disabled");
            }
            if request.source != "manual"
                && (trigger.get("kind").and_then(Value::as_str) != Some(request.source.as_str())
                    || request.occurrence_id.is_none())
            {
                anyhow::bail!("autopilot_occurrence_invalid");
            }
        }
        let mode = autopilot
            .get("execution_mode")
            .and_then(Value::as_str)
            .unwrap_or("create_issue");
        if !matches!(mode, "create_issue" | "run_only") {
            anyhow::bail!("autopilot_execution_mode_invalid");
        }
        let agent_id = autopilot
            .get("assignee_id")
            .or_else(|| autopilot.get("agent_id"))
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("autopilot_assignee_unavailable"))?;
        let agent_exists = self
            .multica_workspace_store
            .list(&workspace_id, MulticaWorkspaceResourceKey::Agents)?
            .iter()
            .any(|agent| agent.get("id").and_then(Value::as_str) == Some(agent_id));
        if !agent_exists {
            anyhow::bail!("autopilot_assignee_unavailable");
        }
        let now = unix_now_ms();
        let run = self.multica_execution_store.reserve_autopilot_occurrence(
            request.autopilot_id.clone(),
            request.trigger_id.clone(),
            request.source,
            Some(
                request
                    .occurrence_id
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            ),
            now,
        )?;
        if run.task_id.is_some() || run.status != "pending" {
            let binding = run
                .task_id
                .as_deref()
                .map(|id| self.multica_execution_store.get_execution(id))
                .transpose()?;
            return Ok(json!({"status":"ok", "run": run, "binding": binding, "replay": true}));
        }
        let issue_id = format!("autopilot-issue-{}", run.id);
        let binding = if mode == "create_issue" {
            let issue = json!({
                "id": issue_id,
                "title": autopilot.get("title").or_else(|| autopilot.get("name")).and_then(Value::as_str).unwrap_or("工作流任务"),
                "description": autopilot.get("description").and_then(Value::as_str).unwrap_or(""),
                "assignee_type": "agent", "assignee_id": agent_id,
                "origin_type": "autopilot", "origin_id": run.autopilot_id,
                "status": "todo"
            });
            let existing = self
                .multica_workspace_store
                .list(&workspace_id, MulticaWorkspaceResourceKey::Issues)?
                .into_iter()
                .find(|issue| issue.get("id").and_then(Value::as_str) == Some(issue_id.as_str()));
            let saved = if let Some(existing) = existing {
                existing
            } else {
                self.multica_workspace_store.upsert(
                    &workspace_id,
                    LocalWorkspaceEntityUpsert {
                        resource: MulticaWorkspaceResourceKey::Issues,
                        entity: issue,
                        expected_revision: Some(0),
                    },
                    now,
                )?
            };
            let id = saved.get("id").and_then(Value::as_str).unwrap_or_default();
            let key = format!("issue-assignment:{}:{}:{}", workspace_id, id, agent_id);
            let reserved =
                self.multica_execution_store
                    .reserve_execution(ExecutionReservation {
                        workspace_id: workspace_id.clone(),
                        issue_id: Some(id.to_string()),
                        agent_id: Some(agent_id.to_string()),
                        execution_kind: MulticaExecutionKind::Thread,
                        parent_thread_id: None,
                        parent_attempt_id: None,
                        idempotency_key: key,
                        now_ms: now,
                    })?;
            let _ =
                self.multica_execution_store
                    .transition_autopilot_run(AutopilotRunTransition {
                        autopilot_id: run.autopilot_id.clone(),
                        run_id: run.id.clone(),
                        expected_revision: run.revision,
                        next_status: "issue_created".into(),
                        issue_id: Some(id.to_string()),
                        task_id: Some(reserved.binding.binding_id.clone()),
                        failure_reason: None,
                        reason_code: None,
                        now_ms: now,
                    })?;
            reserved.binding
        } else if mode == "run_only" {
            let key = format!("autopilot-run:{}", run.id);
            let reserved =
                self.multica_execution_store
                    .reserve_execution(ExecutionReservation {
                        workspace_id: workspace_id.clone(),
                        issue_id: None,
                        agent_id: Some(agent_id.to_string()),
                        execution_kind: MulticaExecutionKind::Thread,
                        parent_thread_id: None,
                        parent_attempt_id: None,
                        idempotency_key: key,
                        now_ms: now,
                    })?;
            let _ =
                self.multica_execution_store
                    .transition_autopilot_run(AutopilotRunTransition {
                        autopilot_id: run.autopilot_id.clone(),
                        run_id: run.id.clone(),
                        expected_revision: run.revision,
                        next_status: "pending".into(),
                        issue_id: None,
                        task_id: Some(reserved.binding.binding_id.clone()),
                        failure_reason: None,
                        reason_code: None,
                        now_ms: now,
                    })?;
            reserved.binding
        } else {
            anyhow::bail!("autopilot_execution_mode_invalid");
        };
        if !dispatch_now {
            let run = self.multica_execution_store.get_autopilot_run(&run.id)?;
            return Ok(
                json!({"status":"ok", "run":run, "binding":binding, "execution":{"status":"queued"}}),
            );
        }
        drop(_guard);
        let dispatch = self
            .dispatch_pending_assignment(
                &binding.binding_id,
                binding.revision,
                &format!("auto-{}", binding.binding_id),
            )
            .await;
        let binding = self
            .multica_execution_store
            .get_execution(&binding.binding_id)?;
        if let Err(error) = &dispatch {
            if binding.state.is_terminal() {
                anyhow::bail!("{}", stable_execution_error_code(error));
            }
        }
        let execution = dispatch.unwrap_or_else(
            |error| json!({"status":"queued", "diagnostic": stable_execution_error_code(&error)}),
        );
        let final_run = self.multica_execution_store.get_autopilot_run(&run.id)?;
        Ok(json!({"status":"ok", "run": final_run, "execution": execution, "binding": binding}))
    }
}

struct UnavailableDataService;

#[async_trait]
impl BridgeDataService for UnavailableDataService {
    async fn session_availability(&self, _session_ids: Vec<String>) -> anyhow::Result<Vec<String>> {
        anyhow::bail!("Session availability service is not wired in core launcher hooks")
    }

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
    crate::multica_workspace::issue_run_controls::assignment_prompt(issue, agent)
}

/// Build an execution selection exclusively from bindings persisted for one
/// Agent. The generic workspace Agent JSON is intentionally not consulted:
/// its `skills` field is a read-only projection and is never authoritative for
/// execution.
fn agent_skill_bindings(
    store: &MulticaExecutionStore,
    workspace_id: &str,
    agent_id: &str,
) -> anyhow::Result<SkillBindings> {
    let bindings =
        store.list_bindings(workspace_id, Some(SkillBindingScope::Agent), Some(agent_id))?;
    Ok(SkillBindings {
        task: Vec::new(),
        agent: bindings
            .into_iter()
            .filter(|binding| binding.enabled)
            .map(|binding| binding.skill_ref)
            .collect(),
    })
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

fn execution_command_replay(
    command: crate::multica_execution_store::CodexMulticaExecutionCommand,
) -> anyhow::Result<Value> {
    match command.state {
        MulticaExecutionCommandState::Committed => command
            .result
            .ok_or_else(|| anyhow::anyhow!("execution_command_result_unavailable")),
        MulticaExecutionCommandState::Failed => anyhow::bail!(
            "{}",
            command
                .error_code
                .as_deref()
                .unwrap_or("codex_execution_failed")
        ),
        MulticaExecutionCommandState::Reserved => anyhow::bail!("execution_command_in_progress"),
    }
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

fn environment_result(agent: &Value) -> Value {
    json!({"agent_id":agent["id"], "custom_env":agent.get("custom_env").cloned().unwrap_or_else(||json!({})),
        "revision":agent["revision"], "execution_supported":false})
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
    use crate::multica_execution::{CodexSkillExecutionRequest, SkillBindingScope, SkillReference};
    use crate::multica_execution_store::{
        ExecutionReservation, MulticaExecutionKind, SkillBindingUpsert,
    };
    use crate::multica_workspace::{
        LocalMulticaWorkspaceStore, LocalWorkspaceEntityUpsert, MulticaWorkspaceResourceKey,
    };
    use crate::status::StatusStore;

    use super::{
        BridgeRuntimeService, CodexExecutionHandle, CodexExecutionService, CodexExecutionStatus,
        CoreRuntimeService, MulticaAgentCreateRequest, MulticaExecutionBindingState,
        MulticaExecutionDispatchRequest, MulticaExecutionStore, agent_skill_bindings,
        assignment_prompt, stable_execution_error_code,
    };

    struct RecordingCodexHost {
        requests: Mutex<Vec<CodexThreadRequest>>,
        subagent_parents: Mutex<Vec<String>>,
        skills: Vec<CodexSkill>,
    }

    impl Default for RecordingCodexHost {
        fn default() -> Self {
            Self {
                requests: Mutex::new(Vec::new()),
                subagent_parents: Mutex::new(Vec::new()),
                skills: Vec::new(),
            }
        }
    }

    #[async_trait]
    impl CodexExecutionService for RecordingCodexHost {
        async fn capabilities(&self) -> anyhow::Result<CodexRuntimeCapabilities> {
            let skills_supported = !self.skills.is_empty();
            Ok(CodexRuntimeCapabilities {
                runtime_id: "codex-current-page".to_string(),
                provider: "codex".to_string(),
                protocol_version: None,
                server_version: None,
                capabilities: if skills_supported {
                    vec!["skill-bundles-v1".to_string()]
                } else {
                    Vec::new()
                },
                skills_supported,
                native_task_host_supported: true,
                skills_inventory_supported: skills_supported,
                skill_protocol: skills_supported.then(|| "skill-bundles-v1".to_string()),
                subagents_supported: true,
            })
        }

        async fn list_skills(&self) -> anyhow::Result<Vec<CodexSkill>> {
            Ok(self.skills.clone())
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
            let mut requests = self.requests.lock().unwrap();
            requests.push(request);
            let sequence = requests.len();
            Ok(CodexExecutionHandle {
                runtime_id: "codex-current-page".to_string(),
                thread_id: format!("native-thread-{sequence}"),
                execution_id: Some(format!("native-turn-{sequence}")),
                parent_thread_id: None,
                idempotency_key: idempotency_key.to_string(),
            })
        }

        async fn create_subagent(
            &self,
            parent_thread_id: &str,
            request: CodexThreadRequest,
            idempotency_key: &str,
        ) -> anyhow::Result<CodexExecutionHandle> {
            self.subagent_parents
                .lock()
                .unwrap()
                .push(parent_thread_id.to_string());
            self.requests.lock().unwrap().push(request);
            Ok(CodexExecutionHandle {
                runtime_id: "codex-current-page".to_string(),
                thread_id: "native-subagent-1".to_string(),
                execution_id: Some("native-subagent-turn-1".to_string()),
                parent_thread_id: Some(parent_thread_id.to_string()),
                idempotency_key: idempotency_key.to_string(),
            })
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
            thread_id: &str,
            execution_id: &str,
        ) -> anyhow::Result<CodexExecutionStatus> {
            Ok(CodexExecutionStatus {
                runtime_id: "codex-current-page".into(),
                thread_id: thread_id.into(),
                execution_id: execution_id.into(),
                state: crate::codex_execution::CodexExecutionState::Completed,
                diagnostic: None,
            })
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

    fn autopilot_fixture(
        mode: &str,
    ) -> (
        tempfile::TempDir,
        MulticaExecutionStore,
        LocalMulticaWorkspaceStore,
        serde_json::Value,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let workspace = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        let autopilot = json!({
            "id":"auto-a", "title":"Scheduled task", "status":"active",
            "execution_mode":mode, "assignee_id":"agent-a",
            "triggers":[
                {"id":"schedule-a","kind":"schedule","enabled":true,"cron_expression":"* * * * *","timezone":"UTC"},
                {"id":"webhook-a","kind":"webhook","enabled":true},
                {"id":"api-a","kind":"api","enabled":true},
                {"id":"disabled-a","kind":"api","enabled":false}
            ]
        });
        for (resource, entity) in [
            (
                MulticaWorkspaceResourceKey::Agents,
                json!({"id":"agent-a","name":"Worker"}),
            ),
            (MulticaWorkspaceResourceKey::Autopilots, autopilot.clone()),
        ] {
            workspace
                .upsert(
                    &workspace_id,
                    LocalWorkspaceEntityUpsert {
                        resource,
                        entity,
                        expected_revision: Some(0),
                    },
                    1,
                )
                .unwrap();
        }
        let executions = MulticaExecutionStore::new(dir.path().join("execution.json"));
        (dir, executions, workspace, autopilot)
    }

    #[tokio::test]
    async fn webhook_rotate_route_replays_after_refresh_and_preserves_cas() {
        let (dir, _, workspace, _) = autopilot_fixture("run_only");
        let path = dir.path().join("webhooks.json");
        let runtime = || {
            CoreRuntimeService::new(0, StatusStore::default())
                .with_multica_workspace_store(workspace.clone())
                .with_multica_webhook_store(super::MulticaWebhookStore::new(path.clone()))
        };
        let provision = super::parse_multica_webhook(
            "/multica/webhooks/provision",
            &json!({"autopilotId":"auto-a","triggerId":"webhook-a","commandId":"provision"}),
        )
        .unwrap();
        runtime().multica_webhooks(provision).await.unwrap();
        let mut payload = json!({
            "autopilotId":"auto-a","triggerId":"webhook-a","commandId":"rotate",
            "expectedRevision":1,"commandSignature":"a".repeat(64)
        });
        let parse = |value: &serde_json::Value| {
            super::parse_multica_webhook("/multica/webhooks/rotate", value).unwrap()
        };
        let first = runtime().multica_webhooks(parse(&payload)).await.unwrap();
        assert_eq!(first["credential_revision"], 2);
        let token = first["webhook_token"].as_str().unwrap();
        payload["expectedRevision"] = json!(2);
        let replay = runtime().multica_webhooks(parse(&payload)).await.unwrap();
        assert_eq!(replay["credential_revision"], 2);
        assert_eq!(replay["credential_replay"], true);
        assert!(replay["webhook_token"].is_null());

        payload["commandSignature"] = json!("b".repeat(64));
        let error = runtime()
            .multica_webhooks(parse(&payload))
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "webhook_command_conflict");
        assert!(
            !super::failed_from_error(&payload, error)
                .to_string()
                .contains(token)
        );
        payload["commandId"] = json!("new-command");
        payload["expectedRevision"] = json!(1);
        assert_eq!(
            runtime()
                .multica_webhooks(parse(&payload))
                .await
                .unwrap_err()
                .to_string(),
            "webhook_revision_conflict"
        );
        payload["commandSignature"] = json!("invalid");
        assert_eq!(
            runtime()
                .multica_webhooks(parse(&payload))
                .await
                .unwrap_err()
                .to_string(),
            "webhook_command_signature_invalid"
        );
        payload.as_object_mut().unwrap().remove("commandSignature");
        payload["expectedRevision"] = json!(2);
        let legacy = runtime().multica_webhooks(parse(&payload)).await.unwrap();
        assert_eq!(legacy["credential_revision"], 3);
        payload["token"] = json!("unexpected");
        assert!(super::parse_multica_webhook("/multica/webhooks/rotate", &payload).is_err());
        assert!(!std::fs::read_to_string(path).unwrap().contains(token));
    }

    #[tokio::test]
    async fn autopilot_tick_queues_run_only_then_dispatches_and_completes() {
        let (_dir, executions, workspace, autopilot) = autopilot_fixture("run_only");
        let minute = super::unix_now_ms() / 60_000 * 60_000;
        executions
            .tick_autopilots(&[autopilot], minute - 60_000)
            .unwrap();
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_execution_store(executions.clone())
            .with_multica_workspace_store(workspace.clone());
        let response = runtime.multica_autopilot_tick().await.unwrap();
        assert_eq!(response["runs"].as_array().unwrap().len(), 1);
        assert_eq!(response["runs"][0]["execution"]["status"], "queued");
        assert_eq!(response["runs"][0]["run"]["status"], "pending");
        assert!(
            workspace
                .list(
                    &crate::multica_workspace::local_workspace_id(),
                    MulticaWorkspaceResourceKey::Issues
                )
                .unwrap()
                .is_empty()
        );
        let run = executions.list_autopilot_runs("auto-a").unwrap().remove(0);
        let binding = executions
            .get_execution(run.task_id.as_deref().unwrap())
            .unwrap();
        assert!(binding.issue_id.is_none());
        let host = Arc::new(RecordingCodexHost::default());
        let runtime = runtime.with_codex_execution_service(host.clone());
        runtime
            .multica_execution_dispatch(MulticaExecutionDispatchRequest {
                binding_id: binding.binding_id.clone(),
                expected_revision: binding.revision,
                lease_token: "scheduler-test".into(),
            })
            .await
            .unwrap();
        assert_eq!(
            executions.get_autopilot_run(&run.id).unwrap().status,
            "running"
        );
        runtime
            .multica_execution_status(super::MulticaExecutionBindingRequest {
                binding_id: binding.binding_id,
            })
            .await
            .unwrap();
        assert_eq!(
            executions.get_autopilot_run(&run.id).unwrap().status,
            "completed"
        );
        assert_eq!(host.requests.lock().unwrap().len(), 1);
        assert!(
            runtime.multica_autopilot_tick().await.unwrap()["runs"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn autopilot_tick_recovers_partial_issue_without_calling_host_or_overwriting_it() {
        let (_dir, executions, workspace, _) = autopilot_fixture("create_issue");
        let run = executions
            .reserve_autopilot_occurrence(
                "auto-a".into(),
                Some("api-a".into()),
                "api".into(),
                Some("delivery-a".into()),
                1,
            )
            .unwrap();
        let issue_id = format!("autopilot-issue-{}", run.id);
        let workspace_id = crate::multica_workspace::local_workspace_id();
        workspace.upsert(&workspace_id, LocalWorkspaceEntityUpsert {
            resource:MulticaWorkspaceResourceKey::Issues,
            entity:json!({"id":issue_id,"title":"Edited after reservation","assignee_type":"agent","assignee_id":"agent-a"}),
            expected_revision:Some(0),
        }, 2).unwrap();
        let host = Arc::new(RecordingCodexHost::default());
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_execution_store(executions.clone())
            .with_multica_workspace_store(workspace.clone())
            .with_codex_execution_service(host.clone());
        let response = runtime.multica_autopilot_tick().await.unwrap();
        assert_eq!(response["runs"][0]["run"]["id"], run.id);
        assert_eq!(response["runs"][0]["execution"]["status"], "queued");
        assert!(host.requests.lock().unwrap().is_empty());
        assert!(
            runtime.multica_autopilot_tick().await.unwrap()["runs"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(executions.list_autopilot_runs("auto-a").unwrap().len(), 1);
        let issues = workspace
            .list(&workspace_id, MulticaWorkspaceResourceKey::Issues)
            .unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0]["title"], "Edited after reservation");
    }

    #[tokio::test]
    async fn autopilot_manual_run_only_occurrences_do_not_share_retry_budget() {
        let (_dir, executions, workspace, _) = autopilot_fixture("run_only");
        let host = Arc::new(RecordingCodexHost::default());
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_execution_store(executions.clone())
            .with_multica_workspace_store(workspace)
            .with_codex_execution_service(host.clone());
        for index in 0..4 {
            // The original UI sends manual + occurrenceId, with no triggerId.
            let request = super::parse_multica_autopilot_trigger(&json!({
                "autopilotId":"auto-a", "source":"manual", "occurrenceId":format!("manual-{index}")
            }))
            .unwrap();
            let response = runtime
                .multica_autopilot_trigger(request.clone())
                .await
                .unwrap();
            assert_eq!(response["run"]["status"], "running");
            assert!(response["run"]["issue_id"].is_null());
            let run = executions
                .get_autopilot_run(response["run"]["id"].as_str().unwrap())
                .unwrap();
            let binding = executions
                .get_execution(run.task_id.as_deref().unwrap())
                .unwrap();
            assert!(binding.issue_id.is_none());
            runtime
                .multica_execution_status(super::MulticaExecutionBindingRequest {
                    binding_id: binding.binding_id,
                })
                .await
                .unwrap();
            let replay = runtime.multica_autopilot_trigger(request).await.unwrap();
            assert_eq!(replay["run"]["id"], response["run"]["id"]);
            assert_eq!(host.requests.lock().unwrap().len(), index + 1);
        }
        assert_eq!(executions.list_autopilot_runs("auto-a").unwrap().len(), 4);
        assert!(
            executions
                .load()
                .unwrap()
                .execution_bindings
                .iter()
                .all(|binding| binding.attempt_no == 1)
        );
    }

    #[tokio::test]
    async fn autopilot_delivery_retries_create_one_native_thread_and_issue() {
        for source in ["webhook", "api"] {
            let (_dir, executions, workspace, _) = autopilot_fixture("create_issue");
            let host = Arc::new(RecordingCodexHost::default());
            let runtime = CoreRuntimeService::new(0, StatusStore::default())
                .with_multica_execution_store(executions.clone())
                .with_multica_workspace_store(workspace.clone())
                .with_codex_execution_service(host.clone());
            let request = super::MulticaAutopilotTriggerRequest {
                autopilot_id: "auto-a".into(),
                trigger_id: Some(format!("{source}-a")),
                source: source.into(),
                occurrence_id: Some("delivery-a".into()),
            };
            let (first, second, third) = tokio::join!(
                runtime.multica_autopilot_trigger(request.clone()),
                runtime.multica_autopilot_trigger(request.clone()),
                runtime.multica_autopilot_trigger(request.clone()),
            );
            let first = first.unwrap();
            assert_eq!(second.unwrap()["run"]["id"], first["run"]["id"]);
            assert_eq!(third.unwrap()["run"]["id"], first["run"]["id"]);
            assert_eq!(first["run"]["status"], "running");
            assert_eq!(host.requests.lock().unwrap().len(), 1);
            assert_eq!(executions.list_autopilot_runs("auto-a").unwrap().len(), 1);
            assert_eq!(
                workspace
                    .list(
                        &crate::multica_workspace::local_workspace_id(),
                        MulticaWorkspaceResourceKey::Issues
                    )
                    .unwrap()
                    .len(),
                1
            );
            let next = runtime
                .multica_autopilot_trigger(super::MulticaAutopilotTriggerRequest {
                    occurrence_id: Some("delivery-b".into()),
                    ..request
                })
                .await
                .unwrap();
            assert_eq!(host.requests.lock().unwrap().len(), 1);
            let first_run = executions
                .get_autopilot_run(first["run"]["id"].as_str().unwrap())
                .unwrap();
            runtime
                .multica_execution_status(super::MulticaExecutionBindingRequest {
                    binding_id: first_run.task_id.unwrap(),
                })
                .await
                .unwrap();
            let next_run = executions
                .get_autopilot_run(next["run"]["id"].as_str().unwrap())
                .unwrap();
            let pending = executions
                .get_execution(&next_run.task_id.unwrap())
                .unwrap();
            assert_eq!(pending.state, MulticaExecutionBindingState::BindingPending);
            runtime
                .multica_execution_dispatch(MulticaExecutionDispatchRequest {
                    binding_id: pending.binding_id,
                    expected_revision: pending.revision,
                    lease_token: "next-slot".into(),
                })
                .await
                .unwrap();
            assert_eq!(host.requests.lock().unwrap().len(), 2);
        }
    }

    #[tokio::test]
    async fn autopilot_rejects_unknown_disabled_and_mismatched_triggers_before_reservation() {
        let (_dir, executions, workspace, _) = autopilot_fixture("run_only");
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_execution_store(executions.clone())
            .with_multica_workspace_store(workspace);
        for (trigger, code) in [
            ("missing", "autopilot_trigger_unknown"),
            ("disabled-a", "autopilot_trigger_disabled"),
            ("webhook-a", "autopilot_occurrence_invalid"),
        ] {
            let error = runtime
                .multica_autopilot_trigger(super::MulticaAutopilotTriggerRequest {
                    autopilot_id: "auto-a".into(),
                    trigger_id: Some(trigger.into()),
                    source: "api".into(),
                    occurrence_id: Some("delivery-a".into()),
                })
                .await
                .unwrap_err();
            assert_eq!(error.to_string(), code);
        }
        assert!(executions.list_autopilot_runs("auto-a").unwrap().is_empty());
    }

    #[test]
    fn autopilot_trigger_parser_limits_ingress_to_delivery_identity() {
        let valid = json!({"autopilotId":"auto-a","triggerId":"api-a","source":"api","occurrenceId":"delivery-a"});
        assert!(super::parse_multica_autopilot_trigger(&valid).is_ok());
        for field in ["url", "headers", "payload", "nowMs"] {
            let mut value = valid.clone();
            value[field] = json!("unexpected");
            assert!(super::parse_multica_autopilot_trigger(&value).is_err());
        }
        let mut value = valid.clone();
        value["source"] = json!("schedule");
        assert!(super::parse_multica_autopilot_trigger(&value).is_err());
        value = valid;
        value.as_object_mut().unwrap().remove("occurrenceId");
        assert!(super::parse_multica_autopilot_trigger(&value).is_err());
    }

    #[test]
    fn autopilot_cron_preview_returns_five_utc_instants_with_strict_limits() {
        let at = chrono::DateTime::parse_from_rfc3339("2026-09-18T00:00:00Z")
            .unwrap()
            .timestamp_millis() as u64;
        let response = super::autopilot_cron_preview(
            super::MulticaCronPreviewRequest {
                expr: "0 9 * * *".into(),
                tz: "Asia/Shanghai".into(),
            },
            at,
        )
        .unwrap();
        assert_eq!(response["next_runs"].as_array().unwrap().len(), 5);
        assert_eq!(response["next_runs"][0], "2026-09-18T01:00:00Z");
        assert_eq!(response["next_runs"][4], "2026-09-22T01:00:00Z");
        for (expr, tz, code) in [
            (" ".repeat(257), "UTC".into(), "autopilot_cron_invalid"),
            ("61 * * * *".into(), "UTC".into(), "autopilot_cron_invalid"),
            (
                "* * * * *".into(),
                "x".repeat(129),
                "autopilot_timezone_invalid",
            ),
            (
                "* * * * *".into(),
                "Unknown/Zone".into(),
                "autopilot_timezone_invalid",
            ),
        ] {
            let error =
                super::autopilot_cron_preview(super::MulticaCronPreviewRequest { expr, tz }, at)
                    .unwrap_err();
            assert_eq!(error.to_string(), code);
        }
    }

    #[tokio::test]
    async fn autopilot_tick_finishes_invalid_unlinked_runs_instead_of_retrying_forever() {
        let (_dir, executions, workspace, mut autopilot) = autopilot_fixture("run_only");
        autopilot["assignee_id"] = json!("missing-agent");
        workspace
            .upsert(
                &crate::multica_workspace::local_workspace_id(),
                LocalWorkspaceEntityUpsert {
                    resource: MulticaWorkspaceResourceKey::Autopilots,
                    entity: autopilot,
                    expected_revision: Some(1),
                },
                2,
            )
            .unwrap();
        let run = executions
            .reserve_autopilot_occurrence(
                "auto-a".into(),
                Some("api-a".into()),
                "api".into(),
                Some("delivery-a".into()),
                1,
            )
            .unwrap();
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_execution_store(executions.clone())
            .with_multica_workspace_store(workspace);
        let response = runtime.multica_autopilot_tick().await.unwrap();
        assert_eq!(
            response["diagnostics"][0]["code"],
            "autopilot_assignee_unavailable"
        );
        assert_eq!(
            executions.get_autopilot_run(&run.id).unwrap().status,
            "failed"
        );
        let retry = runtime.multica_autopilot_tick().await.unwrap();
        assert!(retry["diagnostics"].as_array().unwrap().is_empty());
        assert!(retry["runs"].as_array().unwrap().is_empty());
    }

    fn queued_assignment(
        store: &MulticaExecutionStore,
        workspace_id: &str,
    ) -> crate::multica_execution_store::CodexMulticaExecutionBinding {
        store
            .reserve_execution(ExecutionReservation {
                workspace_id: workspace_id.to_string(),
                issue_id: Some("issue-a".to_string()),
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

    #[tokio::test]
    async fn issue_run_controls_suppress_replay_and_preserve_handoff_until_dispatch() {
        let (_dir, executions, workspace, _) = autopilot_fixture("run_only");
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let offline = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_workspace_store(workspace.clone())
            .with_multica_execution_store(executions.clone());
        let suppressed = json!({"resource":"issues","entity":{"id":"suppressed","title":"Saved only","assignee_type":"agent","assignee_id":"agent-a"},"expectedRevision":0,"suppressRun":true,"handoffNote":"Discarded","commandId":"suppressed-command","commandSignature":"a".repeat(64)});
        let result = offline
            .multica_workspace_upsert(super::parse_multica_workspace_upsert(&suppressed).unwrap())
            .await
            .unwrap();
        assert!(result["queue"].is_null());
        assert!(executions.load().unwrap().execution_bindings.is_empty());
        let mut changed = suppressed.clone();
        changed["suppressRun"] = json!(false);
        assert_eq!(
            offline
                .multica_workspace_upsert(super::parse_multica_workspace_upsert(&changed).unwrap())
                .await
                .unwrap_err()
                .to_string(),
            "multica_workspace_idempotency_conflict"
        );
        assert_eq!(
            offline
                .multica_workspace_upsert(
                    super::parse_multica_workspace_upsert(&suppressed).unwrap()
                )
                .await
                .unwrap(),
            result
        );
        let pending = json!({"resource":"issues","entity":{"id":"handoff","title":"Run with context","assignee_type":"agent","assignee_id":"agent-a"},"expectedRevision":0,"handoffNote":"Remember the selected approach","commandId":"handoff-command","commandSignature":"b".repeat(64)});
        let queued = offline
            .multica_workspace_upsert(super::parse_multica_workspace_upsert(&pending).unwrap())
            .await
            .unwrap();
        assert_eq!(queued["queue"]["status"], "queued");
        let host = Arc::new(RecordingCodexHost::default());
        let online = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_workspace_store(LocalMulticaWorkspaceStore::new(
                workspace.path().to_path_buf(),
            ))
            .with_multica_execution_store(executions.clone())
            .with_codex_execution_service(host.clone());
        let binding = executions.load().unwrap().execution_bindings[0].clone();
        online
            .dispatch_pending_assignment(&binding.binding_id, binding.revision, "handoff-lease")
            .await
            .unwrap();
        assert_eq!(host.requests.lock().unwrap().len(), 1);
        assert_eq!(
            host.requests.lock().unwrap()[0]
                .prompt
                .matches("Remember the selected approach")
                .count(),
            1
        );
        let mut edit = pending.clone();
        edit["entity"] = queued["entity"].clone();
        edit["entity"]["title"] = json!("Edited only");
        edit["expectedRevision"] = json!(1);
        edit["commandId"] = json!("edit-command");
        edit["handoffNote"] = json!("Do not inject this edit");
        let edited = online
            .multica_workspace_upsert(super::parse_multica_workspace_upsert(&edit).unwrap())
            .await
            .unwrap();
        assert!(edited["queue"].is_null());
        assert_eq!(host.requests.lock().unwrap().len(), 1);
        assert_eq!(
            workspace
                .list(&workspace_id, MulticaWorkspaceResourceKey::Activities)
                .unwrap()
                .iter()
                .filter(|v| v["type"] == "handoff_note")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn issue_run_controls_backlog_and_invalid_controls_do_not_dispatch() {
        let (_dir, executions, workspace, _) = autopilot_fixture("run_only");
        let host = Arc::new(RecordingCodexHost::default());
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_workspace_store(workspace.clone())
            .with_multica_execution_store(executions.clone())
            .with_codex_execution_service(host.clone());
        let payload = json!({"resource":"issues","entity":{"id":"backlog-issue","title":"Later","status":"backlog","assignee_type":"agent","assignee_id":"agent-a"},"expectedRevision":0});
        let saved = runtime
            .multica_workspace_upsert(super::parse_multica_workspace_upsert(&payload).unwrap())
            .await
            .unwrap();
        assert!(saved["queue"].is_null());
        assert!(host.requests.lock().unwrap().is_empty());
        for (field, value) in [("suppressRun", json!("yes")), ("handoffNote", json!(42))] {
            let mut invalid = payload.clone();
            invalid[field] = value;
            assert!(super::parse_multica_workspace_upsert(&invalid).is_err());
        }
        let mut invalid = payload.clone();
        invalid["entity"]["suppress_run"] = json!(true);
        assert!(super::parse_multica_workspace_upsert(&invalid).is_err());
        let moved = runtime.multica_workspace_move_issue(super::parse_multica_workspace_move_issue(&json!({"issueId":"backlog-issue","status":"todo","expectedRevision":1,"beforeId":null,"afterId":null})).unwrap()).await.unwrap();
        assert!(moved["queue"].is_object());
        assert_eq!(host.requests.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn issue_run_controls_pending_suppressed_receipt_recovers_without_dispatch() {
        let (_dir, executions, workspace, _) = autopilot_fixture("run_only");
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let entity = json!({"id":"pending-suppressed","title":"Only save","assignee_type":"agent","assignee_id":"agent-a"});
        let command = crate::multica_workspace::WorkspaceCommand::new(
            "pending-suppressed".into(),
            "f".repeat(64),
            "upsert:Issues:pending-suppressed".into(),
            json!({"entity":entity,"expectedRevision":0,"suppressRun":true,"handoffNote":null}),
        )
        .unwrap();
        workspace
            .upsert_with_run_controls(
                &workspace_id,
                LocalWorkspaceEntityUpsert {
                    resource: MulticaWorkspaceResourceKey::Issues,
                    entity,
                    expected_revision: Some(0),
                },
                1,
                Some(&command),
                Some((
                    &crate::multica_workspace::issue_run_controls::IssueRunControls {
                        suppress_run: true,
                        handoff_note: None,
                    },
                    &executions,
                )),
            )
            .unwrap();
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_workspace_store(workspace.clone())
            .with_multica_execution_store(executions.clone());
        let response = runtime
            .multica_workspace_command(super::MulticaWorkspaceCommandRequest {
                command_id: command.id,
                command_signature: command.signature,
            })
            .await
            .unwrap();
        assert_eq!(response["found"], true);
        assert!(response["result"]["queue"].is_null());
        assert_eq!(response["result"]["entity"]["revision"], 1);
        assert!(executions.load().unwrap().execution_bindings.is_empty());
    }

    #[tokio::test]
    async fn workspace_commands_replay_after_runtime_recreation_and_reject_changed_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        let runtime = || {
            CoreRuntimeService::new(0, StatusStore::default())
                .with_multica_workspace_store(LocalMulticaWorkspaceStore::new(
                    store.path().to_path_buf(),
                ))
                .with_multica_execution_store(MulticaExecutionStore::new(
                    dir.path().join("executions.json"),
                ))
        };
        let create = json!({"resource":"issues","entity":{"id":"durable-issue","title":"Original"},"expectedRevision":0,"commandId":"create-a","commandSignature":"a".repeat(64)});
        let saved = runtime()
            .multica_workspace_upsert(serde_json::from_value(create.clone()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            runtime()
                .multica_workspace_upsert(serde_json::from_value(create.clone()).unwrap())
                .await
                .unwrap(),
            saved
        );
        let mut changed = create.clone();
        changed["entity"]["title"] = json!("Changed under same claimed signature");
        assert_eq!(
            runtime()
                .multica_workspace_upsert(serde_json::from_value(changed).unwrap())
                .await
                .unwrap_err()
                .to_string(),
            "multica_workspace_idempotency_conflict"
        );
        let update = json!({"resource":"issues","entity":{"id":"durable-issue","title":"Updated"},"expectedRevision":1,"commandId":"update-a","commandSignature":"b".repeat(64)});
        let updated = runtime()
            .multica_workspace_upsert(serde_json::from_value(update.clone()).unwrap())
            .await
            .unwrap();
        assert_eq!(updated["entity"]["revision"], 2);
        assert_eq!(
            runtime()
                .multica_workspace_upsert(serde_json::from_value(update).unwrap())
                .await
                .unwrap(),
            updated
        );
        let delete = json!({"resource":"issues","entityId":"durable-issue","expectedRevision":2,"commandId":"delete-a","commandSignature":"c".repeat(64)});
        let deleted = runtime()
            .multica_workspace_delete(serde_json::from_value(delete.clone()).unwrap())
            .await
            .unwrap();
        assert_eq!(deleted["deleted"], true);
        assert_eq!(
            runtime()
                .multica_workspace_delete(serde_json::from_value(delete.clone()).unwrap())
                .await
                .unwrap(),
            deleted
        );
        let mut changed = delete;
        changed["entityId"] = json!("other-issue");
        assert_eq!(
            runtime()
                .multica_workspace_delete(serde_json::from_value(changed).unwrap())
                .await
                .unwrap_err()
                .to_string(),
            "multica_workspace_idempotency_conflict"
        );
        assert_eq!(
            runtime()
                .multica_workspace_upsert(serde_json::from_value(create).unwrap())
                .await
                .unwrap(),
            saved
        );
        assert!(
            store
                .list(
                    &crate::multica_workspace::local_workspace_id(),
                    MulticaWorkspaceResourceKey::Issues
                )
                .unwrap()
                .is_empty()
        );
        let lookup = runtime()
            .multica_workspace_command(super::MulticaWorkspaceCommandRequest {
                command_id: "delete-a".into(),
                command_signature: "c".repeat(64),
            })
            .await
            .unwrap();
        assert_eq!(lookup["found"], true);
        assert_eq!(lookup["result"], deleted);
    }

    #[tokio::test]
    async fn workspace_agent_create_replay_is_durable_and_binds_acl_and_skills_payload() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = || {
            CoreRuntimeService::new(0, StatusStore::default())
                .with_multica_workspace_store(LocalMulticaWorkspaceStore::new(
                    dir.path().join("workspace.json"),
                ))
                .with_multica_execution_store(MulticaExecutionStore::new(
                    dir.path().join("executions.json"),
                ))
        };
        let request = json!({"entity":{"id":"durable-agent","name":"Worker","permission_mode":"private","invocation_targets":[]},"skills":[],"commandId":"agent-command","commandSignature":"d".repeat(64)});
        let first = runtime()
            .multica_agent_create(serde_json::from_value(request.clone()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            runtime()
                .multica_agent_create(serde_json::from_value(request.clone()).unwrap())
                .await
                .unwrap(),
            first
        );
        let mut changed = request.clone();
        changed["entity"]["permission_mode"] = json!("public_to");
        changed["entity"]["invocation_targets"] = json!([{"target_type":"workspace"}]);
        assert_eq!(
            runtime()
                .multica_agent_create(serde_json::from_value(changed).unwrap())
                .await
                .unwrap_err()
                .to_string(),
            "multica_workspace_idempotency_conflict"
        );
        let mut changed = request;
        changed["skills"] = json!([{"id":"different-skill"}]);
        assert_eq!(
            runtime()
                .multica_agent_create(serde_json::from_value(changed).unwrap())
                .await
                .unwrap_err()
                .to_string(),
            "multica_workspace_idempotency_conflict"
        );
    }

    #[tokio::test]
    async fn workspace_command_lookup_recovers_committed_entity_without_reapplying_it() {
        let (_dir, executions, workspace, _) = autopilot_fixture("run_only");
        let workspace_id = crate::multica_workspace::local_workspace_id();
        let entity = json!({"id":"recovery-issue","title":"Recover assignment","assignee_type":"agent","assignee_id":"agent-a"});
        let command = crate::multica_workspace::WorkspaceCommand::new(
            "recover-command".into(),
            "e".repeat(64),
            "upsert:Issues:recovery-issue".into(),
            json!({"entity":entity,"expectedRevision":0}),
        )
        .unwrap();
        workspace
            .upsert_with_command(
                &workspace_id,
                LocalWorkspaceEntityUpsert {
                    resource: MulticaWorkspaceResourceKey::Issues,
                    entity,
                    expected_revision: Some(0),
                },
                1,
                Some(&command),
            )
            .unwrap();
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_workspace_store(workspace.clone())
            .with_multica_execution_store(executions.clone());
        let request = super::MulticaWorkspaceCommandRequest {
            command_id: command.id,
            command_signature: command.signature,
        };
        let first = runtime
            .multica_workspace_command(request.clone())
            .await
            .unwrap();
        assert_eq!(first["found"], true);
        assert_eq!(first["result"]["entity"]["revision"], 1);
        assert_eq!(first["result"]["queue"]["status"], "queued");
        assert_eq!(
            runtime.multica_workspace_command(request).await.unwrap(),
            first
        );
        assert_eq!(executions.load().unwrap().execution_bindings.len(), 1);
        assert_eq!(
            workspace
                .list(&workspace_id, MulticaWorkspaceResourceKey::Issues)
                .unwrap()[0]["revision"],
            1
        );
    }

    #[tokio::test]
    async fn agent_nondefault_runtime_fields_fail_before_native_dispatch() {
        for field in ["model", "thinking_level", "service_tier"] {
            let (_dir, executions, workspace, workspace_id) = dispatch_fixture();
            let mut agent = workspace
                .list(&workspace_id, MulticaWorkspaceResourceKey::Agents)
                .unwrap()
                .remove(0);
            agent[field] = json!("nondefault");
            workspace
                .upsert(
                    &workspace_id,
                    LocalWorkspaceEntityUpsert {
                        resource: MulticaWorkspaceResourceKey::Agents,
                        entity: agent,
                        expected_revision: Some(1),
                    },
                    4,
                )
                .unwrap();
            let queued = queued_assignment(&executions, &workspace_id);
            let host = Arc::new(RecordingCodexHost::default());
            let runtime = CoreRuntimeService::new(0, StatusStore::default())
                .with_codex_execution_service(host.clone())
                .with_multica_workspace_store(workspace)
                .with_multica_execution_store(executions.clone());
            let error = runtime
                .multica_execution_dispatch(MulticaExecutionDispatchRequest {
                    binding_id: queued.binding_id.clone(),
                    expected_revision: queued.revision,
                    lease_token: "override-check".into(),
                })
                .await
                .unwrap_err();
            assert_eq!(
                error.to_string(),
                "execution_agent_runtime_override_unsupported"
            );
            assert!(host.requests.lock().unwrap().is_empty());
            assert_eq!(
                executions.get_execution(&queued.binding_id).unwrap().state,
                MulticaExecutionBindingState::BindingPending
            );
        }
    }

    #[test]
    fn workflow_mutation_parser_preserves_clear_fields_and_custom_status_writes() {
        let omitted = super::parse_multica_workspace_move_issue(&json!({
            "issueId":"issue-a", "beforeId":null, "afterId":null, "expectedRevision":1,
        }))
        .unwrap();
        assert_eq!(omitted.assignee_id, None);
        let cleared = super::parse_multica_workspace_move_issue(&json!({
            "issueId":"issue-a", "beforeId":null, "afterId":null, "expectedRevision":1,
            "assigneeType":null, "assigneeId":null, "parentIssueId":null, "projectId":null,
        }))
        .unwrap();
        assert_eq!(cleared.assignee_type, Some(None));
        assert_eq!(cleared.assignee_id, Some(None));
        assert_eq!(cleared.parent_issue_id, Some(None));
        assert_eq!(cleared.project_id, Some(None));
        assert!(
            super::parse_multica_workspace_upsert(&json!({
                "resource":"issue_statuses", "expectedRevision":0,
                "entity":{"id":"status-a", "key":"triage", "name":"Triage", "category":"todo"},
            }))
            .is_ok()
        );
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

    #[test]
    fn assignment_uses_only_enabled_persisted_agent_skill_bindings() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let binding = |id: &str, enabled: bool| SkillBindingUpsert {
            binding_id: format!("binding-{id}"),
            workspace_id: "workspace-a".to_string(),
            scope_kind: SkillBindingScope::Agent,
            scope_id: "agent-a".to_string(),
            skill_ref: SkillReference {
                id: id.to_string(),
                manifest_digest: Some(
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_string(),
                ),
            },
            source_kind: "local".to_string(),
            trust_state: "trusted".to_string(),
            enabled,
            expected_revision: None,
            now_ms: 1,
        };
        store
            .upsert_binding(binding("codex:enabled", true))
            .unwrap();
        store
            .upsert_binding(binding("codex:disabled", false))
            .unwrap();
        store
            .upsert_binding(SkillBindingUpsert {
                scope_id: "agent-b".to_string(),
                ..binding("codex:other-agent", true)
            })
            .unwrap();

        let selected = agent_skill_bindings(&store, "workspace-a", "agent-a").unwrap();
        assert!(selected.task.is_empty());
        assert_eq!(selected.agent.len(), 1);
        assert_eq!(selected.agent[0].id, "codex:enabled");
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
    async fn moved_issue_assignment_dispatches_once_and_survives_host_unavailability() {
        for online in [true, false] {
            let (_dir, executions, workspace, _) = autopilot_fixture("create_issue");
            let workspace_id = crate::multica_workspace::local_workspace_id();
            let saved = workspace
                .upsert(
                    &workspace_id,
                    LocalWorkspaceEntityUpsert {
                        resource: MulticaWorkspaceResourceKey::Issues,
                        entity: json!({"id":"move-issue", "title":"Move task", "status":"todo"}),
                        expected_revision: Some(0),
                    },
                    2,
                )
                .unwrap();
            let host = Arc::new(RecordingCodexHost::default());
            let mut runtime = CoreRuntimeService::new(0, StatusStore::default())
                .with_multica_execution_store(executions.clone())
                .with_multica_workspace_store(workspace.clone());
            if online {
                runtime = runtime.with_codex_execution_service(host.clone());
            }
            let mut revision = saved["revision"].as_u64().unwrap();
            let mut first_binding = None;
            for index in 0..2 {
                let request = serde_json::from_value(json!({
                    "issueId":"move-issue", "assigneeType":"agent", "assigneeId":"agent-a",
                    "beforeId":null, "afterId":null, "expectedRevision":revision,
                }))
                .unwrap();
                let response = runtime.multica_workspace_move_issue(request).await.unwrap();
                revision = response["entity"]["revision"].as_u64().unwrap();
                if index == 1 {
                    // Repeating an unchanged assignment is an ordinary edit.
                    assert!(response.get("queue").is_none_or(|queue| queue.is_null()));
                    continue;
                }
                let binding = response["queue"]["binding_id"]
                    .as_str()
                    .unwrap()
                    .to_string();
                assert_eq!(
                    response["queue"]["status"],
                    if online { "dispatched" } else { "queued" }
                );
                if let Some(first) = &first_binding {
                    assert_eq!(&binding, first);
                } else {
                    first_binding = Some(binding);
                }
            }
            assert_eq!(executions.load().unwrap().execution_bindings.len(), 1);
            assert_eq!(host.requests.lock().unwrap().len(), usize::from(online));
            let stale = serde_json::from_value(json!({
                "issueId":"move-issue", "assigneeType":"agent", "assigneeId":"agent-a",
                "beforeId":null, "afterId":null, "expectedRevision":1,
            }))
            .unwrap();
            assert!(runtime.multica_workspace_move_issue(stale).await.is_err());
            assert_eq!(host.requests.lock().unwrap().len(), usize::from(online));
        }
    }

    #[tokio::test]
    async fn queued_subagent_assignment_forks_the_persisted_parent_thread() {
        let (_dir, executions, workspace, workspace_id) = dispatch_fixture();
        let queued = executions
            .reserve_execution(ExecutionReservation {
                workspace_id: workspace_id.clone(),
                issue_id: Some("issue-a".to_string()),
                agent_id: Some("agent-a".to_string()),
                execution_kind: MulticaExecutionKind::Subagent,
                parent_thread_id: Some("parent-native-thread".to_string()),
                parent_attempt_id: None,
                idempotency_key: "issue-assignment:subagent".to_string(),
                now_ms: 3,
            })
            .unwrap()
            .binding;
        let host = Arc::new(RecordingCodexHost::default());
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_codex_execution_service(host.clone())
            .with_multica_execution_store(executions.clone())
            .with_multica_workspace_store(workspace);

        let response = runtime
            .multica_execution_dispatch(MulticaExecutionDispatchRequest {
                binding_id: queued.binding_id.clone(),
                expected_revision: queued.revision,
                lease_token: "dispatch-subagent".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(
            response["handle"]["parentThreadId"],
            json!("parent-native-thread")
        );
        assert_eq!(
            host.subagent_parents.lock().unwrap().as_slice(),
            ["parent-native-thread"]
        );
        assert_eq!(response["handle"]["threadId"], json!("native-subagent-1"));
    }

    #[tokio::test]
    async fn agent_create_uses_dedicated_path_and_rejects_untrusted_skills_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        let executions = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let host = Arc::new(RecordingCodexHost {
            requests: Mutex::new(Vec::new()),
            subagent_parents: Mutex::new(Vec::new()),
            skills: vec![CodexSkill {
                id: "codex:untrusted".to_string(),
                name: "Untrusted".to_string(),
                summary: None,
                scope: None,
                manifest_digest: Some(
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_string(),
                ),
                enabled: true,
            }],
        });
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_codex_execution_service(host)
            .with_multica_execution_store(executions.clone())
            .with_multica_workspace_store(workspace.clone());
        let error = runtime
            .multica_agent_create(MulticaAgentCreateRequest {
                command_id: None,
                command_signature: None,
                entity: json!({"id": "agent-a", "name": "Agent A"}),
                skills: vec![SkillReference {
                    id: "codex:untrusted".to_string(),
                    manifest_digest: None,
                }],
            })
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "skill_unknown");
        assert!(
            workspace
                .list("local-test", MulticaWorkspaceResourceKey::Agents)
                .unwrap()
                .is_empty()
        );
        assert!(
            executions
                .list_bindings("local-test", Some(SkillBindingScope::Agent), None)
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn agent_create_persists_agent_and_empty_real_binding_set() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        let executions = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let host = Arc::new(RecordingCodexHost {
            requests: Mutex::new(Vec::new()),
            subagent_parents: Mutex::new(Vec::new()),
            skills: vec![CodexSkill {
                id: "codex:available".to_string(),
                name: "Available".to_string(),
                summary: None,
                scope: None,
                manifest_digest: Some(
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_string(),
                ),
                enabled: true,
            }],
        });
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_codex_execution_service(host)
            .with_multica_execution_store(executions)
            .with_multica_workspace_store(workspace.clone());
        let response = runtime
            .multica_agent_create(MulticaAgentCreateRequest {
                command_id: None,
                command_signature: None,
                entity: json!({"id": "agent-a", "name": "Agent A"}),
                skills: Vec::new(),
            })
            .await
            .unwrap();
        assert_eq!(response["agent"]["id"], json!("agent-a"));
        assert!(response["bindings"].as_array().unwrap().is_empty());
        let workspace_id = response["workspaceId"].as_str().unwrap();
        assert_eq!(
            workspace
                .list(workspace_id, MulticaWorkspaceResourceKey::Agents)
                .unwrap()
                .len(),
            1
        );
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
