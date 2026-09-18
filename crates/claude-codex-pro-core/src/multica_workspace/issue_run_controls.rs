//! Durable Issue-write run decisions; no native execution or capacity policy.
use super::*;
use crate::codex_execution::CodexThreadRequest;

#[derive(Debug, Clone, Default)]
pub struct IssueRunControls {
    pub suppress_run: bool,
    pub handoff_note: Option<String>,
}

impl IssueRunControls {
    pub fn validate(
        &self,
        resource: MulticaWorkspaceResourceKey,
        entity: &Value,
    ) -> anyhow::Result<()> {
        if (resource != MulticaWorkspaceResourceKey::Issues
            && (self.suppress_run || self.handoff_note.is_some()))
            || ["suppress_run", "suppressRun", "handoff_note", "handoffNote"]
                .iter()
                .any(|key| entity.get(*key).is_some())
            || self
                .handoff_note
                .as_ref()
                .is_some_and(|note| note.len() > 32 * 1024)
        {
            bail!("multica_issue_run_controls_invalid");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct IssueRunIntent {
    pub(super) key: String,
    issue_id: String,
    issue_revision: u64,
    handoff_note: Option<String>,
    binding_id: Option<String>,
}

pub fn assignment_key(workspace_id: &str, issue: &Value) -> String {
    format!(
        "issue-assignment:{workspace_id}:{}:{}",
        issue["id"].as_str().unwrap_or_default(),
        issue["assignee_id"].as_str().unwrap_or_default()
    )
}

pub(crate) fn assignment_prompt(issue: &Value, agent: &Value) -> anyhow::Result<String> {
    let title = issue
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("execution_issue_title_unavailable"))?;
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

pub(super) fn trigger_source(
    state: &LocalMulticaWorkspaceState,
    previous: Option<&Value>,
    issue: &Value,
) -> Option<&'static str> {
    if issue["assignee_type"] != "agent" || issue["assignee_id"].as_str().is_none() {
        return None;
    }
    let category = |value: &Value| {
        let status = value["status"].as_str().unwrap_or("todo");
        state
            .issue_statuses
            .iter()
            .find(|s| s["key"] == status)
            .and_then(|s| s["category"].as_str())
            .unwrap_or("todo")
            .to_string()
    };
    let current = category(issue);
    let reassigned = previous.is_none_or(|old| {
        old["assignee_type"] != issue["assignee_type"] || old["assignee_id"] != issue["assignee_id"]
    });
    if reassigned {
        return (current != "backlog").then_some("assign");
    }
    previous
        .filter(|old| {
            category(old) == "backlog"
                && !matches!(current.as_str(), "backlog" | "done" | "cancelled")
        })
        .map(|_| "status")
}

pub(super) fn prepare_intent(
    state: &mut LocalMulticaWorkspaceState,
    previous: Option<&Value>,
    issue: &Value,
    controls: &IssueRunControls,
    executions: &MulticaExecutionStore,
) -> anyhow::Result<bool> {
    if controls.suppress_run || trigger_source(state, previous, issue).is_none() {
        return Ok(false);
    }
    let key = assignment_key(&state.workspace_id, issue);
    if state
        .issue_run_intents
        .iter()
        .any(|intent| intent.key == key)
        || executions.load()?.execution_bindings.iter().any(|binding| {
            binding.workspace_id == state.workspace_id
                && (binding.idempotency_key == key
                    || binding.issue_id.as_deref() == issue["id"].as_str()
                        && binding.agent_id.as_deref() == issue["assignee_id"].as_str()
                        && !binding.state.is_terminal())
        })
    {
        return Ok(false);
    }
    let agent = state
        .agents
        .iter()
        .find(|agent| agent["id"] == issue["assignee_id"])
        .ok_or_else(|| anyhow!("execution_agent_unavailable"))?;
    if !metadata::agent_invocable(agent, &state.workspace_id) {
        bail!("multica_workspace_agent_access_denied");
    }
    let handoff_note = controls
        .handoff_note
        .clone()
        .filter(|note| !note.trim().is_empty());
    let mut prompt = assignment_prompt(issue, agent)?;
    if let Some(note) = &handoff_note {
        prompt.push_str("\n\nHandoff note:\n");
        prompt.push_str(note);
    }
    // Validate the combined native request before persisting the Issue or its intent.
    CodexThreadRequest {
        workspace_id: state.workspace_id.clone(),
        issue_id: issue["id"].as_str().map(str::to_owned),
        prompt,
        cwd: None,
        skill_request: None,
    }
    .validate()?;
    state.issue_run_intents.push(IssueRunIntent {
        key,
        issue_id: issue["id"].as_str().unwrap_or_default().into(),
        issue_revision: issue["revision"].as_u64().unwrap_or_default(),
        handoff_note,
        binding_id: None,
    });
    Ok(true)
}

impl LocalMulticaWorkspaceStore {
    pub fn issue_run_eligible(&self, workspace_id: &str, issue: &Value) -> anyhow::Result<bool> {
        let key = assignment_key(workspace_id, issue);
        Ok(self
            .load(workspace_id)?
            .issue_run_intents
            .iter()
            .any(|intent| {
                intent.key == key && Some(intent.issue_revision) == issue["revision"].as_u64()
            }))
    }

    pub fn issue_handoff_note(
        &self,
        workspace_id: &str,
        key: &str,
    ) -> anyhow::Result<Option<String>> {
        Ok(self
            .load(workspace_id)?
            .issue_run_intents
            .iter()
            .find(|intent| intent.key == key)
            .and_then(|intent| intent.handoff_note.clone()))
    }

    pub fn audit_issue_handoff(
        &self,
        workspace_id: &str,
        key: &str,
        binding_id: &str,
        now_ms: u64,
    ) -> anyhow::Result<()> {
        let _guard = local_workspace_store_lock(&self.path)?;
        let mut state = load_local_workspace_state(&self.path, workspace_id)?;
        let Some(intent) = state
            .issue_run_intents
            .iter_mut()
            .find(|intent| intent.key == key)
        else {
            return Ok(());
        };
        if intent.binding_id.is_some() {
            return Ok(());
        }
        intent.binding_id = Some(binding_id.into());
        if let Some(note) = &intent.handoff_note {
            state.activities.push(json!({
                "id":format!("handoff:{binding_id}"), "workspace_id":workspace_id,
                "issue_id":intent.issue_id, "binding_id":binding_id,
                "type":"handoff_note", "handoff_note":note,
                "created_at_ms":now_ms, "revision":1,
            }));
        }
        save_local_workspace_state_locked(&self.path, &state)
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_domain::{IssueTriggerPreviewRequest, preview_issue_trigger};
    use super::*;

    const WORKSPACE: &str = "local-run-controls";

    fn fixture() -> (
        tempfile::TempDir,
        LocalMulticaWorkspaceStore,
        MulticaExecutionStore,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        let executions = MulticaExecutionStore::new(dir.path().join("executions.json"));
        store.upsert(WORKSPACE, LocalWorkspaceEntityUpsert {
            resource: MulticaWorkspaceResourceKey::Agents,
            entity: json!({"id":"agent-a", "name":"Reviewer", "instructions":"Use tests", "permission_mode":"private"}),
            expected_revision: Some(0),
        }, 1).unwrap();
        (dir, store, executions)
    }

    fn issue(status: &str) -> Value {
        json!({"id":"issue-a", "title":"Review", "status":status,
            "assignee_type":"agent", "assignee_id":"agent-a"})
    }

    fn write(
        store: &LocalMulticaWorkspaceStore,
        executions: &MulticaExecutionStore,
        entity: Value,
        revision: u64,
        controls: &IssueRunControls,
    ) -> anyhow::Result<Value> {
        let command = WorkspaceCommand::new(
            format!("write-{revision}"),
            "a".repeat(64),
            "upsert:Issues:issue-a".into(),
            json!({"entity":entity,"expectedRevision":revision,
                "suppressRun":controls.suppress_run,"handoffNote":controls.handoff_note}),
        )?;
        store.upsert_with_run_controls(
            WORKSPACE,
            LocalWorkspaceEntityUpsert {
                resource: MulticaWorkspaceResourceKey::Issues,
                entity,
                expected_revision: Some(revision),
            },
            revision + 2,
            Some(&command),
            Some((controls, executions)),
        )
    }

    fn preview(
        store: &LocalMulticaWorkspaceStore,
        executions: &MulticaExecutionStore,
        request: Value,
    ) -> Value {
        let request: IssueTriggerPreviewRequest = serde_json::from_value(request).unwrap();
        preview_issue_trigger(store, executions, WORKSPACE, &request).unwrap()
    }

    #[test]
    fn custom_backlog_creation_edit_and_promotion_match_preview() {
        let (_dir, store, executions) = fixture();
        for (key, category) in [
            ("custom-backlog", "backlog"),
            ("custom-active", "in_progress"),
        ] {
            store
                .upsert(
                    WORKSPACE,
                    LocalWorkspaceEntityUpsert {
                        resource: MulticaWorkspaceResourceKey::IssueStatuses,
                        entity: json!({"id":key,"key":key,"name":key,"category":category,
                    "color":"#888888","position":1,"is_system":false}),
                        expected_revision: Some(0),
                    },
                    1,
                )
                .unwrap();
        }
        let controls = IssueRunControls {
            suppress_run: false,
            handoff_note: Some("Only on promotion".into()),
        };
        assert_eq!(
            preview(
                &store,
                &executions,
                json!({"isCreate":true,
            "assigneeType":"agent","assigneeId":"agent-a","status":"custom-backlog"})
            )["total_count"],
            0
        );
        let created = write(&store, &executions, issue("custom-backlog"), 0, &controls).unwrap();
        assert_eq!(created["revision"], 1);
        assert!(store.load(WORKSPACE).unwrap().issue_run_intents.is_empty());
        assert_eq!(
            store
                .issue_handoff_note(WORKSPACE, &assignment_key(WORKSPACE, &created))
                .unwrap(),
            None
        );
        let mut edited = created;
        edited["title"] = json!("Edit without dispatch");
        assert_eq!(
            preview(&store, &executions, json!({"issueIds":["issue-a"]}))["total_count"],
            0
        );
        let mut edited = write(&store, &executions, edited, 1, &controls).unwrap();
        assert!(store.load(WORKSPACE).unwrap().issue_run_intents.is_empty());
        for terminal in ["done", "cancelled"] {
            assert_eq!(
                preview(
                    &store,
                    &executions,
                    json!({"issueIds":["issue-a"],"status":terminal})
                )["total_count"],
                0
            );
        }
        let forecast = preview(
            &store,
            &executions,
            json!({"issueIds":["issue-a"],"status":"custom-active"}),
        );
        assert_eq!(forecast["total_count"], 1);
        assert_eq!(forecast["triggers"][0]["source"], "status");
        assert_eq!(forecast["triggers"][0]["handoff_supported"], true);
        edited["status"] = json!("custom-active");
        let mut promoted = write(&store, &executions, edited, 2, &controls).unwrap();
        assert!(store.issue_run_eligible(WORKSPACE, &promoted).unwrap());
        assert_eq!(store.load(WORKSPACE).unwrap().issue_run_intents.len(), 1);
        let key = assignment_key(WORKSPACE, &promoted);
        assert_eq!(
            store.issue_handoff_note(WORKSPACE, &key).unwrap(),
            controls.handoff_note
        );
        promoted["title"] = json!("Ordinary active edit");
        assert_eq!(
            preview(&store, &executions, json!({"issueIds":["issue-a"]}))["total_count"],
            0
        );
        let edited = write(
            &store,
            &executions,
            promoted,
            3,
            &IssueRunControls {
                handoff_note: Some("Discard this edit".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!store.issue_run_eligible(WORKSPACE, &edited).unwrap());
        assert_eq!(store.load(WORKSPACE).unwrap().issue_run_intents.len(), 1);
        assert_eq!(
            store.issue_handoff_note(WORKSPACE, &key).unwrap(),
            controls.handoff_note
        );
        assert!(!executions.path().exists());
    }

    #[test]
    fn cas_failure_preserves_issue_revision_receipts_and_intents() {
        for existing in [false, true] {
            let (_dir, store, executions) = fixture();
            let mut entity = if existing {
                write(
                    &store,
                    &executions,
                    issue("backlog"),
                    0,
                    &IssueRunControls::default(),
                )
                .unwrap()
            } else {
                issue("todo")
            };
            entity["status"] = json!("todo");
            let before = fs::read(store.path()).unwrap();
            let error = write(
                &store,
                &executions,
                entity,
                99,
                &IssueRunControls {
                    handoff_note: Some("Must not be stored".into()),
                    ..Default::default()
                },
            )
            .unwrap_err();
            assert_eq!(error.to_string(), "multica_workspace_revision_conflict");
            assert_eq!(fs::read(store.path()).unwrap(), before);
            let state = store.load(WORKSPACE).unwrap();
            assert!(state.issue_run_intents.is_empty());
            if existing {
                assert_eq!(state.issues[0]["revision"], 1);
            } else {
                assert!(state.issues.is_empty());
            }
            assert!(!executions.path().exists());
        }
    }

    #[test]
    fn denied_agent_create_and_update_do_not_write_even_when_suppressed() {
        for existing in [false, true] {
            for suppress_run in [false, true] {
                let (_dir, store, executions) = fixture();
                let mut entity = if existing {
                    write(
                        &store,
                        &executions,
                        issue("backlog"),
                        0,
                        &IssueRunControls::default(),
                    )
                    .unwrap()
                } else {
                    issue("todo")
                };
                let mut state = store.load(WORKSPACE).unwrap();
                state.agents[0]["owner_id"] = json!("another-user");
                store.save(&state).unwrap();
                entity["status"] = json!("todo");
                let before = fs::read(store.path()).unwrap();
                let error = write(
                    &store,
                    &executions,
                    entity,
                    u64::from(existing),
                    &IssueRunControls {
                        suppress_run,
                        handoff_note: Some("Must not be stored".into()),
                    },
                )
                .unwrap_err();
                assert_eq!(error.to_string(), "multica_workspace_agent_access_denied");
                assert_eq!(fs::read(store.path()).unwrap(), before);
                assert!(store.load(WORKSPACE).unwrap().issue_run_intents.is_empty());
                assert!(!executions.path().exists());
            }
        }
    }

    #[test]
    fn final_prompt_byte_limit_is_checked_before_issue_or_receipt_write() {
        // Independent literal fixes the dispatch format and counts multibyte labels/notes.
        let note = "交接".repeat(300);
        let overhead =
            "任务标题：\nReview\n\n任务描述：\n\n\n智能体指令：\nUse tests\n\nHandoff note:\n"
                .len();
        for existing in [false, true] {
            for extra_bytes in [0, 1] {
                let (_dir, store, executions) = fixture();
                let mut entity = if existing {
                    write(
                        &store,
                        &executions,
                        issue("backlog"),
                        0,
                        &IssueRunControls::default(),
                    )
                    .unwrap()
                } else {
                    issue("todo")
                };
                entity["status"] = json!("todo");
                entity["description"] =
                    json!("d".repeat(32 * 1024 - overhead - note.len() + extra_bytes));
                let before = fs::read(store.path()).unwrap();
                let result = write(
                    &store,
                    &executions,
                    entity,
                    u64::from(existing),
                    &IssueRunControls {
                        suppress_run: false,
                        handoff_note: Some(note.clone()),
                    },
                );
                if extra_bytes == 0 {
                    let saved = result.unwrap();
                    assert_eq!(saved["revision"], u64::from(existing) + 1);
                    assert!(store.issue_run_eligible(WORKSPACE, &saved).unwrap());
                    assert_eq!(
                        store
                            .issue_handoff_note(WORKSPACE, &assignment_key(WORKSPACE, &saved))
                            .unwrap(),
                        Some(note.clone())
                    );
                } else {
                    assert_eq!(result.unwrap_err().to_string(), "codex_prompt_invalid");
                    assert_eq!(fs::read(store.path()).unwrap(), before);
                    assert!(store.load(WORKSPACE).unwrap().issue_run_intents.is_empty());
                }
                assert!(!executions.path().exists());
            }
        }
    }

    #[test]
    fn suppressed_oversized_assignment_saves_without_a_native_intent() {
        let (_dir, store, executions) = fixture();
        let mut entity = issue("todo");
        entity["description"] = json!("d".repeat(32 * 1024));
        let saved = write(
            &store,
            &executions,
            entity,
            0,
            &IssueRunControls {
                suppress_run: true,
                handoff_note: Some("Discarded".into()),
            },
        )
        .unwrap();
        assert_eq!(saved["revision"], 1);
        assert!(!store.issue_run_eligible(WORKSPACE, &saved).unwrap());
        assert!(store.load(WORKSPACE).unwrap().issue_run_intents.is_empty());
        assert!(!executions.path().exists());
    }
}
