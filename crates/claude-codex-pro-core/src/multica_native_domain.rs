//! Native-only quick actions and truthful local admission/usage projections.
//! Registered as a child of multica_workspace to share its atomic store lock.

use super::*;
use crate::codex_execution::CodexThreadRequest;
use crate::multica_execution::{SkillBindingScope, SkillBindings};
use crate::multica_execution_store::MulticaExecutionStore;
use crate::routes::{BridgeRuntimeService, MulticaExecutionCreateRequest};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuickActionRenderRequest {
    pub issue_id: String,
    pub quick_action_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuickActionRunRequest {
    pub issue_id: String,
    pub quick_action_id: String,
    pub expected_issue_revision: u64,
    pub expected_action_revision: u64,
    pub command_id: String,
    pub command_signature: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueTriggerPreviewRequest {
    #[serde(default)]
    pub issue_ids: Vec<String>,
    #[serde(default)]
    pub is_create: bool,
    pub assignee_type: Option<String>,
    pub assignee_id: Option<String>,
    pub status: Option<String>,
}

/// CCP has no Multica Cloud issue-count entitlement gate. This is NOT an
/// assertion about the user's Codex subscription, balance or model quota.
pub fn issue_limit_usage() -> Value {
    json!({"usage":null,"source":"local_cloud_gate_not_configured"})
}

pub fn autopilot_usage() -> Value {
    json!({"source":"local_cloud_gate_not_configured","usage":{
        "action":"off","used":null,"reserved":null,"total":null,"limit":null,
        "reached":null,"period_start":null,"period_end":null,"reset_at":null,"blocked_counts":null,
    }})
}

struct ActionContext {
    issue: Value,
    action: Value,
    agent: Value,
    content: String,
}

// Mirrors quick_action.go at pinned 9fce92f427694d7d303258aa281b05c902a95ba9.
pub(super) fn quick_action_template_token(prompt: &str) -> bool {
    prompt.match_indices("{{").any(|(start, _)| {
        prompt[start + 2..]
            .find('}')
            .is_some_and(|end| prompt[start + 2 + end..].starts_with("}}"))
    })
}

pub(super) fn validated_quick_action_prompt(prompt: &str) -> anyhow::Result<&str> {
    let prompt = prompt.trim();
    if prompt.is_empty() || prompt.chars().count() > 4000 || prompt.contains('\0') {
        bail!("multica_workspace_quick_action_invalid");
    }
    if quick_action_template_token(prompt) {
        bail!("multica_workspace_quick_action_template_invalid");
    }
    if ["agent", "squad", "member", "all"]
        .iter()
        .any(|kind| prompt.contains(&format!("mention://{kind}/")))
    {
        bail!("multica_workspace_quick_action_mention_invalid");
    }
    Ok(prompt)
}

fn context(
    state: &LocalMulticaWorkspaceState,
    issue_id: &str,
    action_id: &str,
) -> anyhow::Result<ActionContext> {
    validate_local_entity_id(issue_id)?;
    validate_local_entity_id(action_id)?;
    let issue = state
        .issues
        .iter()
        .find(|v| v["id"] == issue_id)
        .ok_or_else(|| anyhow!("native_action_issue_not_found"))?;
    let action = state
        .quick_actions
        .iter()
        .find(|v| v["id"] == action_id)
        .ok_or_else(|| anyhow!("native_action_not_found"))?;
    let caller = format!("{}-user", state.workspace_id);
    if action["visibility"] != "public" && action["created_by_id"] != caller {
        bail!("native_action_access_denied");
    }
    // A squad's leader cannot be inferred from its first member. Until a
    // native squad dispatch contract exists, do not execute a different target.
    if action["assignee_type"] != "agent" {
        bail!("native_action_squad_unavailable");
    }
    let agent = state
        .agents
        .iter()
        .find(|v| v["id"] == action["assignee_id"])
        .ok_or_else(|| anyhow!("native_action_target_not_found"))?;
    if !metadata::agent_invocable(agent, &state.workspace_id) {
        bail!("multica_workspace_agent_access_denied");
    }
    if agent["archived"] == true
        || agent["status"] == "archived"
        || agent.get("archived_at").is_some_and(|v| !v.is_null())
    {
        bail!("execution_agent_archived");
    }
    let name = agent["name"]
        .as_str()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| anyhow!("native_action_target_invalid"))?;
    let agent_id = agent["id"]
        .as_str()
        .ok_or_else(|| anyhow!("native_action_target_invalid"))?;
    // Upstream deliberately leaves prompt text literal; no handlebars, shell,
    // filesystem or environment expansion is part of quick-action rendering.
    let prompt = validated_quick_action_prompt(
        action["prompt"]
            .as_str()
            .ok_or_else(|| anyhow!("native_action_prompt_invalid"))?,
    )?;
    let content = format!("[@{name}](mention://agent/{agent_id})\n\n{prompt}");
    Ok(ActionContext {
        issue: issue.clone(),
        action: action.clone(),
        agent: agent.clone(),
        content,
    })
}

pub fn render_quick_action(
    store: &LocalMulticaWorkspaceStore,
    workspace_id: &str,
    request: &QuickActionRenderRequest,
) -> anyhow::Result<Value> {
    let state = store.load(workspace_id)?;
    let ctx = context(&state, &request.issue_id, &request.quick_action_id)?;
    Ok(json!({"content":ctx.content}))
}

/// Recover only durable native results. This entry never initiates dispatch.
pub async fn recover_quick_action(
    store: &LocalMulticaWorkspaceStore,
    executions: &MulticaExecutionStore,
    executor: &dyn BridgeRuntimeService,
    workspace_id: &str,
    command: &WorkspaceCommand,
    now_ms: u64,
) -> anyhow::Result<Value> {
    let state = store.load(workspace_id)?;
    replay_workspace_command(&state, Some(command))?;
    let receipt = state
        .command_receipts
        .iter()
        .find(|r| r.command.id == command.id)
        .ok_or_else(|| anyhow!("native_action_journal_invalid"))?;
    if receipt.complete {
        return Ok(receipt.result.clone());
    }
    let pending = &receipt.result;
    if pending["dispatch_started"] != true {
        bail!("native_action_dispatch_recovery_required");
    }
    let request: QuickActionRunRequest = if let Some(request) = pending.get("request") {
        serde_json::from_value(request.clone())
            .map_err(|_| anyhow!("native_action_journal_invalid"))?
    } else {
        // Legacy receipts lack the action revision snapshot. A candidate from
        // current state is usable only if the original payload hash proves it.
        let comment = &pending["comment"];
        let issue_id = comment["issue_id"]
            .as_str()
            .ok_or_else(|| anyhow!("native_action_journal_invalid"))?;
        let action_id = comment["quick_action_id"]
            .as_str()
            .ok_or_else(|| anyhow!("native_action_journal_invalid"))?;
        let prior = |value: &Value| {
            value
                .as_u64()
                .and_then(|n| n.checked_sub(1))
                .ok_or_else(|| anyhow!("native_action_dispatch_recovery_required"))
        };
        let action = state
            .quick_actions
            .iter()
            .find(|v| v["id"] == action_id)
            .ok_or_else(|| anyhow!("native_action_dispatch_recovery_required"))?;
        QuickActionRunRequest {
            issue_id: issue_id.into(),
            quick_action_id: action_id.into(),
            expected_issue_revision: prior(&comment["issue_revision"])?,
            expected_action_revision: prior(&action["revision"])?,
            command_id: command.id.clone(),
            command_signature: command.signature.clone(),
        }
    };
    let reconstructed = WorkspaceCommand::new(
        request.command_id.clone(),
        request.command_signature.clone(),
        format!(
            "quick-action:{}:{}",
            request.issue_id, request.quick_action_id
        ),
        serde_json::to_value(&request)?,
    )?;
    if reconstructed != *command {
        bail!("native_action_dispatch_recovery_required");
    }
    run_quick_action(store, executions, executor, workspace_id, &request, now_ms).await
}

pub async fn run_quick_action(
    store: &LocalMulticaWorkspaceStore,
    executions: &MulticaExecutionStore,
    executor: &dyn BridgeRuntimeService,
    workspace_id: &str,
    request: &QuickActionRunRequest,
    now_ms: u64,
) -> anyhow::Result<Value> {
    let command = WorkspaceCommand::new(
        request.command_id.clone(),
        request.command_signature.clone(),
        format!(
            "quick-action:{}:{}",
            request.issue_id, request.quick_action_id
        ),
        serde_json::to_value(request)?,
    )?;
    let pending = prepare_run(store, workspace_id, request, &command, now_ms)?;
    if pending.get("dispatch_started").is_none() {
        return Ok(pending);
    }
    let comment = pending["comment"].clone();
    let agent_id = pending["agent_id"]
        .as_str()
        .ok_or_else(|| anyhow!("native_action_journal_invalid"))?;
    let key = format!(
        "quick-action:{}",
        comment["id"]
            .as_str()
            .ok_or_else(|| anyhow!("native_action_journal_invalid"))?
    );
    let claimed = {
        let _guard = local_workspace_store_lock(store.path())?;
        let mut state = load_local_workspace_state(store.path(), workspace_id)?;
        replay_workspace_command(&state, Some(&command))?;
        let receipt = state
            .command_receipts
            .iter_mut()
            .find(|v| v.command.id == command.id)
            .ok_or_else(|| anyhow!("native_action_journal_invalid"))?;
        if receipt.complete {
            return Ok(receipt.result.clone());
        }
        if receipt.result["dispatch_started"] == true {
            false
        } else {
            receipt.result["dispatch_started"] = json!(true);
            save_local_workspace_state_locked(store.path(), &state)?;
            true
        }
    };
    if !claimed {
        // A durable receipt and native binding may have committed before a
        // process died. Recover that result, never repeat an ambiguous dispatch.
        let binding = executions.load()?.execution_bindings.into_iter().find(|b| {
            b.workspace_id == workspace_id
                && b.issue_id.as_deref() == Some(request.issue_id.as_str())
                && b.agent_id.as_deref() == Some(agent_id)
                && b.idempotency_key == key
        });
        if let Some(binding) =
            binding.filter(|b| b.codex_thread_id.is_some() && b.codex_execution_id.is_some())
        {
            let outcome = json!({"target_type":"agent","target_id":agent_id,"status":"queued","reason_code":"",
                "task_id":binding.binding_id,"native_thread_id":binding.codex_thread_id,"native_turn_id":binding.codex_execution_id});
            return finish_run(store, workspace_id, &command, comment, outcome);
        }
        bail!("native_action_dispatch_recovery_required");
    }
    let result = async {
        let agent = executions
            .list_bindings(workspace_id, Some(SkillBindingScope::Agent), Some(agent_id))?
            .into_iter()
            .filter(|v| v.enabled)
            .map(|v| v.skill_ref)
            .collect();
        executor
            .multica_execution_create(MulticaExecutionCreateRequest {
                workspace_id: workspace_id.into(),
                issue_id: request.issue_id.clone(),
                prompt: pending["prompt"]
                    .as_str()
                    .ok_or_else(|| anyhow!("native_action_journal_invalid"))?
                    .into(),
                cwd: None,
                idempotency_key: key,
                execution_kind: None,
                parent_thread_id: None,
                agent_id: Some(agent_id.into()),
                bindings: SkillBindings {
                    task: vec![],
                    agent,
                },
            })
            .await
    }
    .await;
    let outcome = match result {
        Ok(value) => {
            let binding = &value["binding"];
            let task = binding["bindingId"].as_str();
            let thread = binding["codexThreadId"].as_str();
            let turn = binding["codexExecutionId"].as_str();
            if task.is_none()
                || thread.is_none()
                || turn.is_none()
                || binding["workspaceId"] != workspace_id
                || binding["issueId"] != request.issue_id
                || binding["agentId"] != agent_id
            {
                bail!("native_action_dispatch_recovery_required");
            }
            json!({"target_type":"agent","target_id":agent_id,"status":"queued","reason_code":"",
                "task_id":task,"native_thread_id":thread,"native_turn_id":turn})
        }
        Err(error) => {
            let text = error.to_string();
            let code = if text.len() <= 128
                && text
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            {
                text.as_str()
            } else {
                "native_action_execution_failed"
            };
            json!({"target_type":"agent","target_id":agent_id,"status":"blocked","reason_code":code})
        }
    };
    finish_run(store, workspace_id, &command, comment, outcome)
}

fn prepare_run(
    store: &LocalMulticaWorkspaceStore,
    workspace_id: &str,
    request: &QuickActionRunRequest,
    command: &WorkspaceCommand,
    now_ms: u64,
) -> anyhow::Result<Value> {
    validate_local_workspace_id(workspace_id)?;
    let _guard = local_workspace_store_lock(store.path())?;
    let mut state = load_local_workspace_state(store.path(), workspace_id)?;
    if let Some(result) = replay_workspace_command(&state, Some(command))? {
        return Ok(result);
    }
    let ctx = context(&state, &request.issue_id, &request.quick_action_id)?;
    if ctx.action["status"] != "active" {
        bail!("native_action_archived");
    }
    if ctx.issue["revision"].as_u64() != Some(request.expected_issue_revision)
        || ctx.action["revision"].as_u64() != Some(request.expected_action_revision)
    {
        bail!("multica_workspace_revision_conflict");
    }
    let prompt = format!(
        "Agent instructions:\n{}\n\nIssue title:\n{}\n\nIssue description:\n{}\n\nUser quick action:\n{}",
        ctx.agent["instructions"].as_str().unwrap_or(""),
        ctx.issue["title"].as_str().unwrap_or(""),
        ctx.issue["description"].as_str().unwrap_or(""),
        ctx.content
    );
    CodexThreadRequest {
        workspace_id: workspace_id.into(),
        issue_id: Some(request.issue_id.clone()),
        prompt: prompt.clone(),
        cwd: None,
        skill_request: None,
    }
    .validate()?;
    if state.comments.len() >= MAX_LOCAL_ENTITIES_PER_RESOURCE {
        bail!("multica_workspace_collection_too_large");
    }
    let created_at = i64::try_from(now_ms)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .ok_or_else(|| anyhow!("native_action_timestamp_invalid"))?
        .to_rfc3339();
    let comment_id = uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_URL,
        format!("ccp:quick-action:{workspace_id}:{}", request.command_id).as_bytes(),
    )
    .to_string();
    let issue_revision = request
        .expected_issue_revision
        .checked_add(1)
        .ok_or_else(|| anyhow!("multica_workspace_revision_conflict"))?;
    let comment = json!({"id":comment_id,"workspace_id":workspace_id,"issue_id":request.issue_id,
        "author_type":"member","author_id":format!("{workspace_id}-user"),"content":ctx.content,"type":"comment",
        "quick_action_id":request.quick_action_id,"revision":1,"issue_revision":issue_revision,
        "created_at":created_at,"updated_at":created_at,"created_at_ms":now_ms,"updated_at_ms":now_ms});
    validate_local_entity(
        &comment,
        workspace_id,
        MulticaWorkspaceResourceKey::Comments,
    )?;
    state.comments.push(comment.clone());
    let issue = state
        .issues
        .iter_mut()
        .find(|v| v["id"] == request.issue_id)
        .unwrap();
    issue["revision"] = json!(issue_revision);
    issue["updated_at_ms"] = json!(now_ms);
    let action = state
        .quick_actions
        .iter_mut()
        .find(|v| v["id"] == request.quick_action_id)
        .unwrap();
    action["use_count"] = json!(
        action["use_count"]
            .as_u64()
            .unwrap_or_default()
            .checked_add(1)
            .ok_or_else(|| anyhow!("native_action_usage_overflow"))?
    );
    action["last_used_at"] = json!(created_at);
    action["updated_at_ms"] = json!(now_ms);
    action["revision"] = json!(
        request
            .expected_action_revision
            .checked_add(1)
            .ok_or_else(|| anyhow!("multica_workspace_revision_conflict"))?
    );
    let pending = json!({"dispatch_started":false,"comment":comment,"agent_id":ctx.agent["id"],"prompt":prompt,"request":request});
    record_workspace_command(&mut state, Some(command), pending.clone(), false)?;
    save_local_workspace_state_locked(store.path(), &state)?;
    Ok(pending)
}

fn finish_run(
    store: &LocalMulticaWorkspaceStore,
    workspace_id: &str,
    command: &WorkspaceCommand,
    mut comment: Value,
    outcome: Value,
) -> anyhow::Result<Value> {
    let _guard = local_workspace_store_lock(store.path())?;
    let mut state = load_local_workspace_state(store.path(), workspace_id)?;
    replay_workspace_command(&state, Some(command))?;
    let index = state
        .command_receipts
        .iter()
        .position(|v| v.command.id == command.id)
        .ok_or_else(|| anyhow!("native_action_journal_invalid"))?;
    if state.command_receipts[index].complete {
        return Ok(state.command_receipts[index].result.clone());
    }
    comment["trigger_outcomes"] = json!([outcome]);
    if let Some(stored) = state.comments.iter_mut().find(|v| v["id"] == comment["id"]) {
        stored["trigger_outcomes"] = comment["trigger_outcomes"].clone();
    }
    state.command_receipts[index].result = comment.clone();
    state.command_receipts[index].complete = true;
    save_local_workspace_state_locked(store.path(), &state)?;
    Ok(comment)
}

/// Forecast the current local queue's reservation, not native completion or
/// Cloud admission. Read-only and deliberately mirrors queue_issue_assignment.
pub fn preview_issue_trigger(
    store: &LocalMulticaWorkspaceStore,
    executions: &MulticaExecutionStore,
    workspace_id: &str,
    request: &IssueTriggerPreviewRequest,
) -> anyhow::Result<Value> {
    if request.issue_ids.len() > 100
        || request.issue_ids.iter().collect::<BTreeSet<_>>().len() != request.issue_ids.len()
        || request.is_create && !request.issue_ids.is_empty()
        || request.assignee_type.is_some() != request.assignee_id.is_some()
    {
        bail!("native_trigger_preview_invalid");
    }
    if request
        .assignee_type
        .as_deref()
        .is_some_and(|kind| !matches!(kind, "agent" | "member" | "squad"))
    {
        bail!("native_trigger_preview_invalid");
    }
    let state = store.load(workspace_id)?;
    if let Some(status) = &request.status {
        if !state
            .issue_statuses
            .iter()
            .any(|s| s["key"] == *status && s["archived"] != true)
        {
            bail!("multica_workspace_issue_status_invalid");
        }
    }
    let mut candidates = vec![];
    if request.is_create {
        candidates.push(json!({"id":""}));
    }
    for issue_id in &request.issue_ids {
        validate_local_entity_id(issue_id)?;
        candidates.push(
            state
                .issues
                .iter()
                .find(|v| v["id"] == *issue_id)
                .cloned()
                .ok_or_else(|| anyhow!("native_action_issue_not_found"))?,
        );
    }
    let bindings = executions.load()?.execution_bindings;
    let mut triggers = vec![];
    for issue in candidates {
        let kind = request
            .assignee_type
            .as_deref()
            .or_else(|| issue["assignee_type"].as_str());
        if kind == Some("squad") {
            bail!("native_action_squad_unavailable");
        }
        if kind != Some("agent") {
            continue;
        }
        let agent_id = request
            .assignee_id
            .as_deref()
            .or_else(|| issue["assignee_id"].as_str())
            .ok_or_else(|| anyhow!("native_trigger_preview_invalid"))?;
        validate_local_entity_id(agent_id)?;
        let agent = state
            .agents
            .iter()
            .find(|v| v["id"] == agent_id)
            .ok_or_else(|| anyhow!("execution_agent_unavailable"))?;
        if !metadata::agent_invocable(agent, workspace_id) {
            bail!("multica_workspace_agent_access_denied");
        }
        let mut next = issue.clone();
        next["assignee_type"] = json!(kind);
        next["assignee_id"] = json!(agent_id);
        if let Some(status) = &request.status {
            next["status"] = json!(status);
        }
        let Some(source) = super::issue_run_controls::trigger_source(
            &state,
            (!request.is_create).then_some(&issue),
            &next,
        ) else {
            continue;
        };
        let issue_id = issue["id"].as_str().unwrap_or_default();
        let key = format!("issue-assignment:{workspace_id}:{issue_id}:{agent_id}");
        if !request.is_create
            && (state
                .issue_run_intents
                .iter()
                .any(|intent| intent.key == key)
                || bindings.iter().any(|b| {
                    b.workspace_id == workspace_id
                        && (b.idempotency_key == key
                            || b.issue_id.as_deref() == Some(issue_id)
                                && b.agent_id.as_deref() == Some(agent_id)
                                && !b.state.is_terminal())
                }))
        {
            continue;
        }
        triggers.push(json!({"issue_id":issue_id,"agent_id":agent_id,"source":source,"handoff_supported":true}));
    }
    Ok(json!({"total_count":triggers.len(),"triggers":triggers}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_execution::{
        CodexPageExecutionClient, CodexPageHostMethod, CodexRuntimeBinding,
        FakeCodexPageHostTransport,
    };
    use crate::routes::CoreRuntimeService;
    use crate::status::StatusStore;
    use std::sync::Arc;

    const WORKSPACE: &str = "local-test";

    fn fixture() -> (
        tempfile::TempDir,
        LocalMulticaWorkspaceStore,
        MulticaExecutionStore,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        for (resource, entity) in [
            (
                MulticaWorkspaceResourceKey::Agents,
                json!({"id":"agent-a","name":"Reviewer","instructions":"Inspect the diff"}),
            ),
            (
                MulticaWorkspaceResourceKey::Issues,
                json!({"id":"issue-a","title":"Review","description":"Current changes","assignee_type":"agent","assignee_id":"agent-a"}),
            ),
            (
                MulticaWorkspaceResourceKey::QuickActions,
                json!({"id":"action-a","name":"Review now","prompt":"Review $LITERAL","assignee_type":"agent","assignee_id":"agent-a"}),
            ),
        ] {
            store
                .upsert(
                    WORKSPACE,
                    LocalWorkspaceEntityUpsert {
                        resource,
                        entity,
                        expected_revision: Some(0),
                    },
                    1000,
                )
                .unwrap();
        }
        let executions = MulticaExecutionStore::new(dir.path().join("execution.json"));
        (dir, store, executions)
    }

    fn request() -> QuickActionRunRequest {
        QuickActionRunRequest {
            issue_id: "issue-a".into(),
            quick_action_id: "action-a".into(),
            expected_issue_revision: 1,
            expected_action_revision: 1,
            command_id: "action-command".into(),
            command_signature: "a".repeat(64),
        }
    }

    fn runtime(
        store: &LocalMulticaWorkspaceStore,
        executions: &MulticaExecutionStore,
    ) -> (CoreRuntimeService, FakeCodexPageHostTransport) {
        let transport = FakeCodexPageHostTransport::default();
        let service = CodexPageExecutionClient::new(
            transport.clone(),
            CodexRuntimeBinding {
                runtime_id: "codex-current-page".into(),
                provider: "codex".into(),
                app_server_version: None,
                declared_capabilities: vec![],
            },
        )
        .unwrap();
        (
            CoreRuntimeService::new(0, StatusStore::default())
                .with_multica_workspace_store(store.clone())
                .with_multica_execution_store(executions.clone())
                .with_codex_execution_service(Arc::new(service)),
            transport,
        )
    }

    #[test]
    fn native_domain_render_literal_and_access() {
        let (_dir, store, _) = fixture();
        let request = QuickActionRenderRequest {
            issue_id: "issue-a".into(),
            quick_action_id: "action-a".into(),
        };
        assert_eq!(
            render_quick_action(&store, WORKSPACE, &request).unwrap()["content"],
            "[@Reviewer](mention://agent/agent-a)\n\nReview $LITERAL"
        );
        let mut state = store.load(WORKSPACE).unwrap();
        state.quick_actions[0]["created_by_id"] = json!("another-user");
        store.save(&state).unwrap();
        assert_eq!(
            render_quick_action(&store, WORKSPACE, &request)
                .unwrap_err()
                .to_string(),
            "native_action_access_denied"
        );
        state.quick_actions[0]["created_by_id"] = json!("local-test-user");
        state.agents[0]["archived"] = json!(true);
        store.save(&state).unwrap();
        assert_eq!(
            render_quick_action(&store, WORKSPACE, &request)
                .unwrap_err()
                .to_string(),
            "execution_agent_archived"
        );
    }

    #[test]
    fn native_domain_quick_action_prompt_matches_pinned_validation() {
        for value in [
            "{{issue.title}}",
            "{{}}",
            "{{line\nbreak}}",
            "broken {{a} plus {{valid}}",
        ] {
            assert_eq!(
                validated_quick_action_prompt(value)
                    .unwrap_err()
                    .to_string(),
                "multica_workspace_quick_action_template_invalid"
            );
        }
        // A stray single closing brace interrupts the pinned [^}]* pattern.
        assert_eq!(
            validated_quick_action_prompt("{{a} text }}").unwrap(),
            "{{a} text }}"
        );
        for kind in ["agent", "squad", "member", "all"] {
            assert_eq!(
                validated_quick_action_prompt(&format!("literal mention://{kind}/id"))
                    .unwrap_err()
                    .to_string(),
                "multica_workspace_quick_action_mention_invalid"
            );
        }
        let literal = "See [task](mention://issue/related), ask @someone; $VALUE stays literal.";
        assert_eq!(
            validated_quick_action_prompt(&format!(" \n{literal}\t")).unwrap(),
            literal
        );
        let unicode = "\u{754c}".repeat(4000);
        assert!(validated_quick_action_prompt(&unicode).is_ok());
        assert!(validated_quick_action_prompt(&(unicode + "x")).is_err());
    }

    #[test]
    fn native_domain_quick_action_writes_trim_bound_and_reject_side_effect_mentions() {
        let (_dir, store, _) = fixture();
        let mut action = store.load(WORKSPACE).unwrap().quick_actions[0].clone();
        let before = std::fs::read(store.path()).unwrap();
        for (field, value) in [
            ("name", "x".repeat(33)),
            ("description", "x".repeat(201)),
            ("prompt", "x".repeat(4001)),
            ("prompt", "{{issue.title}}".into()),
            ("prompt", "mention://agent/other".into()),
            ("prompt", "mention://member/person".into()),
            ("prompt", "mention://squad/team".into()),
            ("prompt", "mention://all/all".into()),
        ] {
            let mut invalid = action.clone();
            invalid[field] = json!(value);
            assert!(
                store
                    .upsert(
                        WORKSPACE,
                        LocalWorkspaceEntityUpsert {
                            resource: MulticaWorkspaceResourceKey::QuickActions,
                            entity: invalid,
                            expected_revision: Some(1),
                        },
                        2000
                    )
                    .is_err()
            );
            assert_eq!(std::fs::read(store.path()).unwrap(), before);
        }
        action["name"] = json!(format!(" {} ", "\u{754c}".repeat(32)));
        action["description"] = json!(format!(" {} ", "\u{754c}".repeat(200)));
        action["prompt"] = json!("  Review $LITERAL\nSee [related](mention://issue/other).  ");
        let saved = store
            .upsert(
                WORKSPACE,
                LocalWorkspaceEntityUpsert {
                    resource: MulticaWorkspaceResourceKey::QuickActions,
                    entity: action,
                    expected_revision: Some(1),
                },
                2000,
            )
            .unwrap();
        assert_eq!(saved["name"].as_str().unwrap().chars().count(), 32);
        assert_eq!(saved["description"].as_str().unwrap().chars().count(), 200);
        assert_eq!(
            saved["prompt"],
            "Review $LITERAL\nSee [related](mention://issue/other)."
        );
        let persisted = std::fs::read(store.path()).unwrap();
        let rendered = render_quick_action(
            &store,
            WORKSPACE,
            &QuickActionRenderRequest {
                issue_id: "issue-a".into(),
                quick_action_id: "action-a".into(),
            },
        )
        .unwrap();
        assert_eq!(
            rendered["content"],
            "[@Reviewer](mention://agent/agent-a)\n\nReview $LITERAL\nSee [related](mention://issue/other)."
        );
        assert_eq!(std::fs::read(store.path()).unwrap(), persisted);
    }

    #[tokio::test]
    async fn native_domain_quick_action_invalid_legacy_prompt_never_posts_or_dispatches() {
        let (_dir, store, executions) = fixture();
        let (runtime, transport) = runtime(&store, &executions);
        for prompt in ["mention://agent/other".to_string(), "x".repeat(4001)] {
            let mut legacy = store.load(WORKSPACE).unwrap();
            legacy.quick_actions[0]["prompt"] = json!(prompt);
            store.save(&legacy).unwrap();
            let before = std::fs::read(store.path()).unwrap();
            assert!(
                run_quick_action(&store, &executions, &runtime, WORKSPACE, &request(), 2000)
                    .await
                    .is_err()
            );
            assert!(
                render_quick_action(
                    &store,
                    WORKSPACE,
                    &QuickActionRenderRequest {
                        issue_id: "issue-a".into(),
                        quick_action_id: "action-a".into(),
                    }
                )
                .is_err()
            );
            assert_eq!(std::fs::read(store.path()).unwrap(), before);
            assert!(transport.calls().is_empty());
            assert!(executions.load().unwrap().execution_bindings.is_empty());
        }
    }

    #[tokio::test]
    async fn native_domain_quick_action_archived_render_is_read_only_but_run_is_blocked() {
        let (_dir, store, executions) = fixture();
        let (runtime, transport) = runtime(&store, &executions);
        let mut state = store.load(WORKSPACE).unwrap();
        state.quick_actions[0]["status"] = json!("archived");
        store.save(&state).unwrap();
        let before = std::fs::read(store.path()).unwrap();
        assert_eq!(
            render_quick_action(
                &store,
                WORKSPACE,
                &QuickActionRenderRequest {
                    issue_id: "issue-a".into(),
                    quick_action_id: "action-a".into(),
                }
            )
            .unwrap()["content"],
            "[@Reviewer](mention://agent/agent-a)\n\nReview $LITERAL"
        );
        assert_eq!(
            run_quick_action(&store, &executions, &runtime, WORKSPACE, &request(), 2000)
                .await
                .unwrap_err()
                .to_string(),
            "native_action_archived"
        );
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(transport.calls().is_empty());
    }

    #[tokio::test]
    async fn native_domain_run_dispatches_existing_service_and_replays_after_reload() {
        let (_dir, store, executions) = fixture();
        let (runtime, transport) = runtime(&store, &executions);
        let request = request();
        let result = run_quick_action(&store, &executions, &runtime, WORKSPACE, &request, 2000)
            .await
            .unwrap();
        assert_eq!(result["type"], "comment");
        assert_eq!(result["quick_action_id"], "action-a");
        assert_eq!(result["trigger_outcomes"][0]["status"], "queued");
        let reloaded = LocalMulticaWorkspaceStore::new(store.path().into());
        assert_eq!(
            run_quick_action(&reloaded, &executions, &runtime, WORKSPACE, &request, 3000)
                .await
                .unwrap(),
            result
        );
        assert_eq!(
            transport
                .calls()
                .iter()
                .filter(|r| r.method == CodexPageHostMethod::TurnStart)
                .count(),
            1
        );
        let turn = transport
            .calls()
            .into_iter()
            .find(|r| r.method == CodexPageHostMethod::TurnStart)
            .unwrap();
        let params = turn.params.to_string();
        assert!(params.contains("Inspect the diff"));
        assert!(params.contains("Review $LITERAL"));
        assert!(turn.params.get("env").is_none());
        assert!(turn.params.get("model").is_none());
        let state = reloaded.load(WORKSPACE).unwrap();
        assert_eq!(state.comments.len(), 1);
        assert_eq!(state.quick_actions[0]["use_count"], 1);
        assert_eq!(state.issues[0]["revision"], 2);
        let mut conflicting = request.clone();
        conflicting.expected_issue_revision = 2;
        assert_eq!(
            run_quick_action(&store, &executions, &runtime, WORKSPACE, &conflicting, 3000)
                .await
                .unwrap_err()
                .to_string(),
            "multica_workspace_idempotency_conflict"
        );
    }

    #[tokio::test]
    async fn native_domain_stale_revision_has_no_side_effect() {
        let (_dir, store, executions) = fixture();
        let (runtime, transport) = runtime(&store, &executions);
        let before = std::fs::read(store.path()).unwrap();
        let mut request = request();
        request.expected_action_revision = 0;
        assert_eq!(
            run_quick_action(&store, &executions, &runtime, WORKSPACE, &request, 2000)
                .await
                .unwrap_err()
                .to_string(),
            "multica_workspace_revision_conflict"
        );
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(transport.calls().is_empty());
    }

    #[tokio::test]
    async fn native_domain_failure_is_persisted_not_reported_as_queued() {
        let (_dir, store, executions) = fixture();
        let runtime = CoreRuntimeService::new(0, StatusStore::default())
            .with_multica_workspace_store(store.clone())
            .with_multica_execution_store(executions.clone());
        let result = run_quick_action(&store, &executions, &runtime, WORKSPACE, &request(), 2000)
            .await
            .unwrap();
        assert_eq!(result["trigger_outcomes"][0]["status"], "blocked");
        assert!(
            result["trigger_outcomes"][0]
                .get("native_thread_id")
                .is_none()
        );
        assert_eq!(
            store.load(WORKSPACE).unwrap().comments[0]["trigger_outcomes"],
            result["trigger_outcomes"]
        );
    }

    #[tokio::test]
    async fn native_domain_ambiguous_journal_never_redispatches() {
        let (_dir, store, executions) = fixture();
        let (runtime, transport) = runtime(&store, &executions);
        let request = request();
        let command = WorkspaceCommand::new(
            request.command_id.clone(),
            request.command_signature.clone(),
            "quick-action:issue-a:action-a".into(),
            serde_json::to_value(&request).unwrap(),
        )
        .unwrap();
        prepare_run(&store, WORKSPACE, &request, &command, 2000).unwrap();
        let mut state = store.load(WORKSPACE).unwrap();
        state.command_receipts[0].result["dispatch_started"] = json!(true);
        store.save(&state).unwrap();
        assert_eq!(
            run_quick_action(&store, &executions, &runtime, WORKSPACE, &request, 3000)
                .await
                .unwrap_err()
                .to_string(),
            "native_action_dispatch_recovery_required"
        );
        assert!(transport.calls().is_empty());
        assert_eq!(store.load(WORKSPACE).unwrap().comments.len(), 1);
        assert_eq!(
            recover_quick_action(&store, &executions, &runtime, WORKSPACE, &command, 3000)
                .await
                .unwrap_err()
                .to_string(),
            "native_action_dispatch_recovery_required"
        );
        assert!(transport.calls().is_empty());
    }

    #[tokio::test]
    async fn native_domain_recovers_committed_binding_after_receipt_interruption() {
        let (_dir, store, executions) = fixture();
        let (runtime, transport) = runtime(&store, &executions);
        let request = request();
        let command = WorkspaceCommand::new(
            request.command_id.clone(),
            request.command_signature.clone(),
            "quick-action:issue-a:action-a".into(),
            serde_json::to_value(&request).unwrap(),
        )
        .unwrap();
        let mut pending = prepare_run(&store, WORKSPACE, &request, &command, 2000).unwrap();
        let result = run_quick_action(&store, &executions, &runtime, WORKSPACE, &request, 2000)
            .await
            .unwrap();
        pending["dispatch_started"] = json!(true);
        let mut state = store.load(WORKSPACE).unwrap();
        state.command_receipts[0].complete = false;
        state.command_receipts[0].result = pending;
        store.save(&state).unwrap();
        let calls = transport.calls().len();
        assert_eq!(
            run_quick_action(&store, &executions, &runtime, WORKSPACE, &request, 3000)
                .await
                .unwrap(),
            result
        );
        assert_eq!(transport.calls().len(), calls);
        assert!(store.load(WORKSPACE).unwrap().command_receipts[0].complete);

        // Exercise the route recovery entry with both old and new journals.
        for legacy in [true, false] {
            let mut receipt_state = state.clone();
            if legacy {
                receipt_state.command_receipts[0]
                    .result
                    .as_object_mut()
                    .unwrap()
                    .remove("request");
            } else {
                // New journals stay recoverable after the action was edited.
                receipt_state.quick_actions[0]["revision"] = json!(99);
            }
            store.save(&receipt_state).unwrap();
            assert_eq!(
                recover_quick_action(&store, &executions, &runtime, WORKSPACE, &command, 3000)
                    .await
                    .unwrap(),
                result
            );
            assert_eq!(transport.calls().len(), calls);
            assert!(store.load(WORKSPACE).unwrap().command_receipts[0].complete);
        }
    }

    #[tokio::test]
    async fn native_domain_recovery_rejects_unstarted_and_unverifiable_receipts() {
        let (_dir, store, executions) = fixture();
        let (runtime, transport) = runtime(&store, &executions);
        let request = request();
        let command = WorkspaceCommand::new(
            request.command_id.clone(),
            request.command_signature.clone(),
            "quick-action:issue-a:action-a".into(),
            serde_json::to_value(&request).unwrap(),
        )
        .unwrap();
        prepare_run(&store, WORKSPACE, &request, &command, 2000).unwrap();
        assert_eq!(
            recover_quick_action(&store, &executions, &runtime, WORKSPACE, &command, 3000)
                .await
                .unwrap_err()
                .to_string(),
            "native_action_dispatch_recovery_required"
        );
        let mut state = store.load(WORKSPACE).unwrap();
        state.command_receipts[0].result["dispatch_started"] = json!(true);
        state.command_receipts[0].result["request"]["issueId"] = json!("different-issue");
        store.save(&state).unwrap();
        assert_eq!(
            recover_quick_action(&store, &executions, &runtime, WORKSPACE, &command, 3000)
                .await
                .unwrap_err()
                .to_string(),
            "native_action_dispatch_recovery_required"
        );
        state.command_receipts[0]
            .result
            .as_object_mut()
            .unwrap()
            .remove("request");
        state.quick_actions[0]["revision"] = json!(99);
        store.save(&state).unwrap();
        assert_eq!(
            recover_quick_action(&store, &executions, &runtime, WORKSPACE, &command, 3000)
                .await
                .unwrap_err()
                .to_string(),
            "native_action_dispatch_recovery_required"
        );
        assert!(transport.calls().is_empty());
        assert!(!store.load(WORKSPACE).unwrap().command_receipts[0].complete);
    }

    #[test]
    fn native_domain_preview_uses_the_same_assignment_idempotency_scope() {
        use crate::multica_execution_store::{ExecutionReservation, MulticaExecutionKind};
        let (_dir, store, executions) = fixture();
        let request: IssueTriggerPreviewRequest =
            serde_json::from_value(json!({"issueIds":["issue-a"]})).unwrap();
        executions
            .reserve_execution(ExecutionReservation {
                workspace_id: WORKSPACE.into(),
                issue_id: Some("issue-a".into()),
                agent_id: Some("agent-a".into()),
                execution_kind: MulticaExecutionKind::Thread,
                parent_thread_id: None,
                parent_attempt_id: None,
                idempotency_key: format!("issue-assignment:{WORKSPACE}:issue-a:agent-a"),
                now_ms: 2000,
            })
            .unwrap();
        assert_eq!(
            preview_issue_trigger(&store, &executions, WORKSPACE, &request).unwrap()["total_count"],
            0
        );
        let mut create = request;
        create.is_create = true;
        create.issue_ids.clear();
        create.assignee_type = Some("agent".into());
        create.assignee_id = Some("agent-a".into());
        assert_eq!(
            preview_issue_trigger(&store, &executions, WORKSPACE, &create).unwrap()["total_count"],
            1
        );
    }

    #[test]
    fn native_domain_preview_and_usage_are_read_only_and_no_quota_is_fabricated() {
        let (_dir, store, executions) = fixture();
        let before = std::fs::read(store.path()).unwrap();
        let request: IssueTriggerPreviewRequest = serde_json::from_value(
            json!({"isCreate":true,"assigneeType":"agent","assigneeId":"agent-a"}),
        )
        .unwrap();
        let preview = preview_issue_trigger(&store, &executions, WORKSPACE, &request).unwrap();
        assert_eq!(preview["total_count"], 1);
        assert_eq!(preview["triggers"][0]["handoff_supported"], true);
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(executions.load().unwrap().execution_bindings.is_empty());
        assert!(issue_limit_usage()["usage"].is_null());
        assert_eq!(autopilot_usage()["usage"]["action"], "off");
        for field in ["used", "reserved", "total", "limit", "reached"] {
            assert!(autopilot_usage()["usage"][field].is_null());
        }
    }

    #[test]
    fn native_domain_requests_do_not_accept_execution_overrides() {
        let base = serde_json::to_value(request()).unwrap();
        for (field, value) in [
            ("prompt", json!("replacement")),
            ("env", json!({"EXAMPLE":"value"})),
            ("model", json!("replacement")),
            ("cwd", json!("C:/other")),
            ("bindings", json!({"agent":["untrusted"]})),
        ] {
            let mut payload = base.clone();
            payload[field] = value;
            assert!(serde_json::from_value::<QuickActionRunRequest>(payload).is_err());
        }
    }
}
