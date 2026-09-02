//! Private persistence for Multica Skill bindings and attempt evidence.
//!
//! This store is intentionally independent from `settings.json`, Codex
//! configuration, and the remote Multica database. It records stable IDs and
//! bounded Skill audit data only; prompts, bodies, paths, commands and
//! credentials never enter this file.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{anyhow, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::codex_execution::{CodexExecutionHandle, CodexExecutionState, CodexExecutionStatus};
use crate::multica_execution::{
    SkillBindingScope, SkillReference, SkillResolutionAudit, record_runtime_loaded,
};

const STORE_VERSION: u32 = 1;
const MAX_BINDINGS: usize = 512;
const MAX_SNAPSHOTS: usize = 2048;
const MAX_EXECUTION_BINDINGS: usize = 4096;
const MAX_EXECUTION_COMMANDS: usize = 8192;
const MAX_ID_LENGTH: usize = 240;
const MAX_SOURCE_LENGTH: usize = 96;
const MAX_ERROR_LENGTH: usize = 96;
const MAX_MESSAGE_SUMMARY_LENGTH: usize = 512;
const MAX_TASK_MESSAGES: usize = 16_384;
const MAX_AUTOPILOT_RUNS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexMulticaAutopilotRun {
    pub id: String,
    pub autopilot_id: String,
    #[serde(default)]
    pub trigger_id: Option<String>,
    pub source: String,
    pub status: String,
    #[serde(default)]
    pub issue_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    pub triggered_at_ms: u64,
    #[serde(default)]
    pub completed_at_ms: Option<u64>,
    #[serde(default)]
    pub failure_reason: Option<String>,
    #[serde(default)]
    pub reason_code: Option<String>,
    pub created_at_ms: u64,
    #[serde(default)]
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutopilotRunTransition {
    pub autopilot_id: String,
    pub run_id: String,
    pub expected_revision: u64,
    pub next_status: String,
    pub issue_id: Option<String>,
    pub task_id: Option<String>,
    pub failure_reason: Option<String>,
    pub reason_code: Option<String>,
    pub now_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexMulticaSkillBinding {
    pub binding_id: String,
    pub workspace_id: String,
    pub scope_kind: SkillBindingScope,
    pub scope_id: String,
    pub skill_ref: SkillReference,
    pub source_kind: String,
    pub trust_state: String,
    pub enabled: bool,
    pub revision: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexMulticaAttemptSkillSnapshot {
    pub snapshot_id: String,
    pub execution_binding_id: String,
    pub attempt_no: u32,
    pub requested_skill_refs: Vec<SkillReference>,
    pub resolved_skill_refs: Vec<SkillReference>,
    pub resolved_manifest_digest: String,
    pub runtime_loaded_skill_refs: Vec<SkillReference>,
    pub resolution_status: String,
    #[serde(default)]
    pub resolution_error_code: Option<String>,
    pub created_at_ms: u64,
    #[serde(default)]
    pub loaded_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MulticaExecutionBindingState {
    BindingPending,
    Dispatched,
    WaitingLocalDirectory,
    Running,
    Completed,
    Failed,
    Cancelled,
    CancelPending,
    Stale,
    Orphaned,
    Reconciling,
}

impl MulticaExecutionBindingState {
    /// Upstream queue states accepted by the local control-plane transition
    /// endpoint. `waiting_local_directory` is retained as CCP's documented
    /// preparation state and is not collapsed into `running`.
    pub fn from_queue_status(value: &str) -> anyhow::Result<Self> {
        match value {
            "queued" => Ok(Self::BindingPending),
            "dispatched" => Ok(Self::Dispatched),
            "waiting_local_directory" => Ok(Self::WaitingLocalDirectory),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => bail!("agent_task_queue_status_invalid"),
        }
    }

    fn queue_transition_allowed(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::BindingPending, Self::Dispatched | Self::Cancelled)
                | (
                    Self::Dispatched,
                    Self::WaitingLocalDirectory | Self::Running | Self::Failed | Self::Cancelled
                )
                | (
                    Self::WaitingLocalDirectory,
                    Self::Running | Self::Failed | Self::Cancelled
                )
                | (
                    Self::Running,
                    Self::Completed | Self::Failed | Self::Cancelled
                )
        )
    }
}

impl MulticaExecutionBindingState {
    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MulticaExecutionKind {
    Thread,
    Subagent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MulticaExecutionCommandKind {
    Create,
    Continue,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MulticaExecutionCommandState {
    Reserved,
    Committed,
    Failed,
}

/// Stable execution mapping. It deliberately contains no prompt, body, cwd,
/// command text, Skill path, provider setting, URL, or credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexMulticaExecutionBinding {
    pub binding_id: String,
    pub workspace_id: String,
    pub issue_id: String,
    #[serde(default)]
    pub agent_id: Option<String>,
    pub multica_run_id: String,
    #[serde(default)]
    pub codex_runtime_id: Option<String>,
    #[serde(default)]
    pub codex_thread_id: Option<String>,
    #[serde(default)]
    pub codex_execution_id: Option<String>,
    #[serde(default)]
    pub parent_thread_id: Option<String>,
    #[serde(default)]
    pub parent_attempt_id: Option<String>,
    pub execution_kind: MulticaExecutionKind,
    pub attempt_no: u32,
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    pub idempotency_key: String,
    pub state: MulticaExecutionBindingState,
    pub revision: u64,
    #[serde(default)]
    pub codex_revision: u64,
    #[serde(default)]
    pub last_event_id: Option<String>,
    #[serde(default)]
    pub last_error_code: Option<String>,
    pub retryable: bool,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(default)]
    pub completed_at_ms: Option<u64>,
    #[serde(default)]
    pub lease_token: Option<String>,
    #[serde(default)]
    pub lease_expires_at_ms: Option<u64>,
    #[serde(default)]
    pub last_heartbeat_at_ms: Option<u64>,
}

fn default_max_attempts() -> u32 {
    2
}

/// Bounded task transcript metadata. Full Codex message bodies remain in the
/// native history store; this index mirrors only the ordering/audit surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexMulticaTaskMessage {
    pub message_id: String,
    pub binding_id: String,
    pub seq: u32,
    pub message_type: String,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexMulticaExecutionCommand {
    pub command_id: String,
    pub binding_id: String,
    pub kind: MulticaExecutionCommandKind,
    pub state: MulticaExecutionCommandState,
    #[serde(default)]
    pub codex_execution_id: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    pub revision: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReservation {
    pub workspace_id: String,
    pub issue_id: String,
    pub agent_id: Option<String>,
    pub execution_kind: MulticaExecutionKind,
    pub parent_thread_id: Option<String>,
    pub parent_attempt_id: Option<String>,
    pub idempotency_key: String,
    pub now_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReservationResult {
    pub binding: CodexMulticaExecutionBinding,
    pub replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionCommandReservationResult {
    pub command: CodexMulticaExecutionCommand,
    pub replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueTransition {
    pub binding_id: String,
    pub expected_revision: u64,
    pub lease_token: Option<String>,
    pub next_state: MulticaExecutionBindingState,
    pub failure_reason: Option<String>,
    pub now_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaExecutionState {
    pub version: u32,
    #[serde(default)]
    pub skill_bindings: Vec<CodexMulticaSkillBinding>,
    #[serde(default)]
    pub attempt_skill_snapshots: Vec<CodexMulticaAttemptSkillSnapshot>,
    #[serde(default)]
    pub execution_bindings: Vec<CodexMulticaExecutionBinding>,
    #[serde(default)]
    pub execution_commands: Vec<CodexMulticaExecutionCommand>,
    #[serde(default)]
    pub task_messages: Vec<CodexMulticaTaskMessage>,
    #[serde(default)]
    pub autopilot_runs: Vec<CodexMulticaAutopilotRun>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MulticaExecutionStore {
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillBindingUpsert {
    pub binding_id: String,
    pub workspace_id: String,
    pub scope_kind: SkillBindingScope,
    pub scope_id: String,
    pub skill_ref: SkillReference,
    pub source_kind: String,
    pub trust_state: String,
    pub enabled: bool,
    pub expected_revision: Option<u64>,
    pub now_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillBindingReplaceAll {
    pub workspace_id: String,
    pub scope_kind: SkillBindingScope,
    pub scope_id: String,
    pub bindings: Vec<SkillBindingUpsert>,
    pub expected_revision: Option<u64>,
    pub now_ms: u64,
}

impl Default for MulticaExecutionStore {
    fn default() -> Self {
        Self::new(crate::paths::default_multica_state_dir().join("execution.json"))
    }
}

impl MulticaExecutionStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> anyhow::Result<MulticaExecutionState> {
        load_state(&self.path)
    }

    pub fn list_bindings(
        &self,
        workspace_id: &str,
        scope_kind: Option<SkillBindingScope>,
        scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<CodexMulticaSkillBinding>> {
        validate_id(workspace_id, "workspace_id")?;
        if let Some(scope_id) = scope_id {
            validate_id(scope_id, "scope_id")?;
        }
        let state = self.load()?;
        Ok(state
            .skill_bindings
            .into_iter()
            .filter(|binding| {
                binding.workspace_id == workspace_id
                    && scope_kind.is_none_or(|kind| binding.scope_kind == kind)
                    && scope_id.is_none_or(|id| binding.scope_id == id)
            })
            .collect())
    }

    pub fn list_autopilot_runs(
        &self,
        autopilot_id: &str,
    ) -> anyhow::Result<Vec<CodexMulticaAutopilotRun>> {
        validate_id(autopilot_id, "autopilot_id")?;
        let mut runs = self.load()?.autopilot_runs;
        runs.retain(|run| run.autopilot_id == autopilot_id);
        runs.sort_by_key(|run| std::cmp::Reverse(run.created_at_ms));
        Ok(runs)
    }

    pub fn get_autopilot_run(&self, run_id: &str) -> anyhow::Result<CodexMulticaAutopilotRun> {
        validate_id(run_id, "run_id")?;
        self.load()?
            .autopilot_runs
            .into_iter()
            .find(|run| run.id == run_id)
            .ok_or_else(|| anyhow!("autopilot_run_not_found"))
    }

    pub fn trigger_autopilot_run(
        &self,
        autopilot_id: String,
        trigger_id: Option<String>,
        source: String,
        now_ms: u64,
    ) -> anyhow::Result<CodexMulticaAutopilotRun> {
        validate_id(&autopilot_id, "autopilot_id")?;
        if !matches!(source.as_str(), "manual" | "schedule" | "webhook" | "api") {
            bail!("autopilot_run_source_invalid");
        }
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        if state.autopilot_runs.len() >= MAX_AUTOPILOT_RUNS {
            bail!("autopilot_runs_too_large");
        }
        let run = CodexMulticaAutopilotRun {
            id: format!("autopilot-run-{now_ms}-{}", state.autopilot_runs.len()),
            autopilot_id,
            trigger_id,
            source,
            status: "pending".into(),
            issue_id: None,
            task_id: None,
            triggered_at_ms: now_ms,
            completed_at_ms: None,
            failure_reason: None,
            reason_code: None,
            created_at_ms: now_ms,
            revision: 1,
        };
        state.autopilot_runs.push(run.clone());
        save_state_locked(&self.path, &state)?;
        Ok(run)
    }

    pub fn transition_autopilot_run(
        &self,
        input: AutopilotRunTransition,
    ) -> anyhow::Result<CodexMulticaAutopilotRun> {
        validate_id(&input.autopilot_id, "autopilot_id")?;
        validate_id(&input.run_id, "run_id")?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let run = state
            .autopilot_runs
            .iter_mut()
            .find(|run| run.id == input.run_id && run.autopilot_id == input.autopilot_id)
            .ok_or_else(|| anyhow!("autopilot_run_not_found"))?;
        if run.revision != input.expected_revision {
            bail!("autopilot_run_revision_conflict");
        }
        let allowed = matches!(
            (run.status.as_str(), input.next_status.as_str()),
            (
                "pending",
                "issue_created" | "running" | "skipped" | "failed"
            ) | ("issue_created", "running" | "failed")
                | ("running", "completed" | "failed")
        );
        if !allowed {
            bail!("autopilot_run_transition_invalid");
        }
        run.status = input.next_status;
        if input.issue_id.is_some() {
            run.issue_id = input.issue_id;
        }
        if input.task_id.is_some() {
            run.task_id = input.task_id;
        }
        run.failure_reason = input.failure_reason;
        run.reason_code = input.reason_code;
        if matches!(run.status.as_str(), "completed" | "failed" | "skipped") {
            run.completed_at_ms = Some(input.now_ms);
        }
        run.revision = run.revision.saturating_add(1);
        run.created_at_ms = run.created_at_ms.min(input.now_ms);
        let result = run.clone();
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    pub fn upsert_binding(
        &self,
        input: SkillBindingUpsert,
    ) -> anyhow::Result<CodexMulticaSkillBinding> {
        validate_binding_input(&input)?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let key = binding_key(
            &input.workspace_id,
            input.scope_kind,
            &input.scope_id,
            &input.skill_ref.id,
        );
        let existing = state
            .skill_bindings
            .iter_mut()
            .find(|binding| binding_key_for(binding) == key);
        let binding = if let Some(existing) = existing {
            if input
                .expected_revision
                .is_some_and(|revision| revision != existing.revision)
            {
                bail!("skill_binding_revision_conflict");
            }
            existing.revision = existing.revision.saturating_add(1);
            existing.skill_ref = input.skill_ref;
            existing.source_kind = input.source_kind;
            existing.trust_state = input.trust_state;
            existing.enabled = input.enabled;
            existing.updated_at_ms = input.now_ms;
            existing.clone()
        } else {
            if state.skill_bindings.len() >= MAX_BINDINGS {
                bail!("skill_bindings_too_large");
            }
            let binding = CodexMulticaSkillBinding {
                binding_id: input.binding_id,
                workspace_id: input.workspace_id,
                scope_kind: input.scope_kind,
                scope_id: input.scope_id,
                skill_ref: input.skill_ref,
                source_kind: input.source_kind,
                trust_state: input.trust_state,
                enabled: input.enabled,
                revision: 1,
                created_at_ms: input.now_ms,
                updated_at_ms: input.now_ms,
            };
            state.skill_bindings.push(binding.clone());
            binding
        };
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(binding)
    }

    pub fn remove_binding(
        &self,
        workspace_id: &str,
        scope_kind: SkillBindingScope,
        scope_id: &str,
        skill_id: &str,
        expected_revision: Option<u64>,
    ) -> anyhow::Result<bool> {
        validate_id(workspace_id, "workspace_id")?;
        validate_id(scope_id, "scope_id")?;
        validate_id(skill_id, "skill_id")?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let key = binding_key(workspace_id, scope_kind, scope_id, skill_id);
        let Some(index) = state
            .skill_bindings
            .iter()
            .position(|binding| binding_key_for(binding) == key)
        else {
            return Ok(false);
        };
        if expected_revision
            .is_some_and(|revision| revision != state.skill_bindings[index].revision)
        {
            bail!("skill_binding_revision_conflict");
        }
        state.skill_bindings.remove(index);
        save_state_locked(&self.path, &state)?;
        Ok(true)
    }

    /// Atomically replace every binding in one scope. This mirrors Multica's
    /// replace-all Agent/Skill junction update while keeping the local trust
    /// and manifest checks in the caller.
    pub fn replace_bindings(
        &self,
        input: SkillBindingReplaceAll,
    ) -> anyhow::Result<Vec<CodexMulticaSkillBinding>> {
        validate_id(&input.workspace_id, "workspace_id")?;
        validate_id(&input.scope_id, "scope_id")?;
        if input.bindings.len() > MAX_BINDINGS {
            bail!("skill_bindings_too_large");
        }
        let mut seen = BTreeSet::new();
        for binding in &input.bindings {
            validate_binding_input(binding)?;
            if binding.workspace_id != input.workspace_id
                || binding.scope_kind != input.scope_kind
                || binding.scope_id != input.scope_id
            {
                bail!("skill_binding_scope_mismatch");
            }
            if !seen.insert(binding.skill_ref.id.clone()) {
                bail!("skill_binding_duplicate");
            }
        }
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let current_revision = state
            .skill_bindings
            .iter()
            .filter(|binding| {
                binding.workspace_id == input.workspace_id
                    && binding.scope_kind == input.scope_kind
                    && binding.scope_id == input.scope_id
            })
            .map(|binding| binding.revision)
            .max()
            .unwrap_or(0);
        if input
            .expected_revision
            .is_some_and(|revision| revision != current_revision)
        {
            bail!("skill_binding_revision_conflict");
        }
        state.skill_bindings.retain(|binding| {
            !(binding.workspace_id == input.workspace_id
                && binding.scope_kind == input.scope_kind
                && binding.scope_id == input.scope_id)
        });
        let mut result = Vec::with_capacity(input.bindings.len());
        for binding in input.bindings {
            let value = CodexMulticaSkillBinding {
                binding_id: binding.binding_id,
                workspace_id: binding.workspace_id,
                scope_kind: binding.scope_kind,
                scope_id: binding.scope_id,
                skill_ref: binding.skill_ref,
                source_kind: binding.source_kind,
                trust_state: binding.trust_state,
                enabled: binding.enabled,
                revision: 1,
                created_at_ms: input.now_ms,
                updated_at_ms: input.now_ms,
            };
            state.skill_bindings.push(value.clone());
            result.push(value);
        }
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    pub fn reserve_attempt_snapshot(
        &self,
        execution_binding_id: &str,
        attempt_no: u32,
        audit: &SkillResolutionAudit,
        created_at_ms: u64,
    ) -> anyhow::Result<CodexMulticaAttemptSkillSnapshot> {
        validate_id(execution_binding_id, "execution_binding_id")?;
        if attempt_no == 0 {
            bail!("attempt_no_invalid");
        }
        validate_resolution_audit(audit)?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        if let Some(existing) = state.attempt_skill_snapshots.iter().find(|snapshot| {
            snapshot.execution_binding_id == execution_binding_id
                && snapshot.attempt_no == attempt_no
        }) {
            if same_resolution_snapshot(existing, execution_binding_id, attempt_no, audit) {
                return Ok(existing.clone());
            }
            bail!("attempt_skill_snapshot_conflict");
        }
        if state.attempt_skill_snapshots.len() >= MAX_SNAPSHOTS {
            bail!("attempt_skill_snapshots_too_large");
        }
        let snapshot = CodexMulticaAttemptSkillSnapshot {
            snapshot_id: stable_snapshot_id(execution_binding_id, attempt_no, audit),
            execution_binding_id: execution_binding_id.to_string(),
            attempt_no,
            requested_skill_refs: audit.requested_skill_refs.clone(),
            resolved_skill_refs: audit.resolved_skill_refs.clone(),
            resolved_manifest_digest: audit.resolved_manifest_digest.clone(),
            runtime_loaded_skill_refs: Vec::new(),
            resolution_status: audit.resolution_status.clone(),
            resolution_error_code: audit.resolution_error_code.clone(),
            created_at_ms,
            loaded_at_ms: None,
        };
        state.attempt_skill_snapshots.push(snapshot.clone());
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(snapshot)
    }

    pub fn record_runtime_loaded(
        &self,
        snapshot_id: &str,
        loaded: Vec<SkillReference>,
        loaded_at_ms: u64,
    ) -> anyhow::Result<CodexMulticaAttemptSkillSnapshot> {
        validate_id(snapshot_id, "snapshot_id")?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let snapshot = state
            .attempt_skill_snapshots
            .iter_mut()
            .find(|snapshot| snapshot.snapshot_id == snapshot_id)
            .ok_or_else(|| anyhow!("attempt_skill_snapshot_unknown"))?;
        if snapshot.loaded_at_ms.is_some() {
            if snapshot.runtime_loaded_skill_refs == loaded {
                return Ok(snapshot.clone());
            }
            bail!("attempt_skill_snapshot_conflict");
        }
        let audit = SkillResolutionAudit {
            requested_skill_refs: snapshot.requested_skill_refs.clone(),
            resolved_skill_refs: snapshot.resolved_skill_refs.clone(),
            resolved_manifest_digest: snapshot.resolved_manifest_digest.clone(),
            protocol: "stored".to_string(),
            resolution_status: snapshot.resolution_status.clone(),
            resolution_error_code: snapshot.resolution_error_code.clone(),
        };
        let loaded_audit = record_runtime_loaded(&audit, loaded)?;
        snapshot.runtime_loaded_skill_refs = loaded_audit.runtime_loaded_skill_refs;
        snapshot.resolution_status = loaded_audit.resolution_status;
        snapshot.loaded_at_ms = Some(loaded_at_ms);
        let result = snapshot.clone();
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    pub fn list_attempt_snapshots(
        &self,
        execution_binding_id: &str,
    ) -> anyhow::Result<Vec<CodexMulticaAttemptSkillSnapshot>> {
        validate_id(execution_binding_id, "execution_binding_id")?;
        Ok(self
            .load()?
            .attempt_skill_snapshots
            .into_iter()
            .filter(|snapshot| snapshot.execution_binding_id == execution_binding_id)
            .collect())
    }

    pub fn reserve_execution(
        &self,
        input: ExecutionReservation,
    ) -> anyhow::Result<ExecutionReservationResult> {
        validate_execution_reservation(&input)?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        if let Some(existing) = state
            .execution_bindings
            .iter()
            .find(|binding| binding.idempotency_key == input.idempotency_key)
        {
            if existing.workspace_id == input.workspace_id
                && existing.issue_id == input.issue_id
                && existing.agent_id == input.agent_id
                && existing.execution_kind == input.execution_kind
                && existing.parent_thread_id == input.parent_thread_id
                && existing.parent_attempt_id == input.parent_attempt_id
            {
                return Ok(ExecutionReservationResult {
                    binding: existing.clone(),
                    replay: true,
                });
            }
            bail!("execution_idempotency_conflict");
        }
        if state.execution_bindings.len() >= MAX_EXECUTION_BINDINGS {
            bail!("execution_bindings_too_large");
        }
        if state.execution_commands.len() >= MAX_EXECUTION_COMMANDS {
            bail!("execution_commands_too_large");
        }
        if state.execution_bindings.iter().any(|binding| {
            binding.workspace_id == input.workspace_id
                && binding.issue_id == input.issue_id
                && binding.agent_id == input.agent_id
                && !binding.state.is_terminal()
        }) {
            bail!("execution_active_attempt_conflict");
        }
        let attempt_no = state
            .execution_bindings
            .iter()
            .filter(|binding| {
                binding.workspace_id == input.workspace_id && binding.issue_id == input.issue_id
            })
            .map(|binding| binding.attempt_no)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| anyhow!("execution_attempt_overflow"))?;
        let binding_id = stable_execution_id("binding", &input.idempotency_key);
        let binding = CodexMulticaExecutionBinding {
            multica_run_id: stable_execution_id("run", &input.idempotency_key),
            binding_id,
            workspace_id: input.workspace_id,
            issue_id: input.issue_id,
            agent_id: input.agent_id,
            codex_runtime_id: None,
            codex_thread_id: None,
            codex_execution_id: None,
            parent_thread_id: input.parent_thread_id,
            parent_attempt_id: input.parent_attempt_id,
            execution_kind: input.execution_kind,
            attempt_no,
            max_attempts: 2,
            idempotency_key: input.idempotency_key,
            state: MulticaExecutionBindingState::BindingPending,
            revision: 1,
            codex_revision: 0,
            last_event_id: None,
            last_error_code: None,
            retryable: true,
            created_at_ms: input.now_ms,
            updated_at_ms: input.now_ms,
            completed_at_ms: None,
            lease_token: None,
            lease_expires_at_ms: None,
            last_heartbeat_at_ms: None,
        };
        state.execution_commands.push(CodexMulticaExecutionCommand {
            command_id: binding.idempotency_key.clone(),
            binding_id: binding.binding_id.clone(),
            kind: MulticaExecutionCommandKind::Create,
            state: MulticaExecutionCommandState::Reserved,
            codex_execution_id: None,
            error_code: None,
            revision: 1,
            created_at_ms: input.now_ms,
            updated_at_ms: input.now_ms,
        });
        state.execution_bindings.push(binding.clone());
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(ExecutionReservationResult {
            binding,
            replay: false,
        })
    }

    pub fn commit_execution(
        &self,
        binding_id: &str,
        expected_revision: u64,
        handle: &CodexExecutionHandle,
        now_ms: u64,
    ) -> anyhow::Result<CodexMulticaExecutionBinding> {
        validate_id(binding_id, "binding_id")?;
        validate_execution_handle(handle)?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let binding_index = state
            .execution_bindings
            .iter()
            .position(|binding| binding.binding_id == binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))?;
        let binding = &state.execution_bindings[binding_index];
        if binding.codex_thread_id.as_deref() == Some(handle.thread_id.as_str())
            && binding.codex_execution_id == handle.execution_id
            && binding.idempotency_key == handle.idempotency_key
        {
            return Ok(binding.clone());
        }
        if binding.revision != expected_revision {
            bail!("execution_revision_conflict");
        }
        if binding.idempotency_key != handle.idempotency_key {
            bail!("execution_idempotency_conflict");
        }
        if state
            .execution_bindings
            .iter()
            .enumerate()
            .any(|(index, other)| {
                index != binding_index
                    && other.codex_thread_id.as_deref() == Some(handle.thread_id.as_str())
            })
        {
            bail!("execution_thread_conflict");
        }
        let binding = &mut state.execution_bindings[binding_index];
        binding.codex_thread_id = Some(handle.thread_id.clone());
        binding.codex_runtime_id = Some(handle.runtime_id.clone());
        binding.codex_execution_id = handle.execution_id.clone();
        binding.parent_thread_id = handle
            .parent_thread_id
            .clone()
            .or_else(|| binding.parent_thread_id.clone());
        binding.state = MulticaExecutionBindingState::Dispatched;
        binding.revision = binding.revision.saturating_add(1);
        binding.codex_revision = binding.codex_revision.saturating_add(1);
        binding.retryable = false;
        binding.last_error_code = None;
        binding.updated_at_ms = now_ms;
        let result = binding.clone();
        commit_command_in_state(
            &mut state,
            &result.idempotency_key,
            handle.execution_id.clone(),
            now_ms,
        )?;
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    pub fn fail_execution(
        &self,
        binding_id: &str,
        expected_revision: u64,
        error_code: &str,
        retryable: bool,
        now_ms: u64,
    ) -> anyhow::Result<CodexMulticaExecutionBinding> {
        validate_id(binding_id, "binding_id")?;
        validate_error_code(error_code)?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let binding = state
            .execution_bindings
            .iter_mut()
            .find(|binding| binding.binding_id == binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))?;
        if binding.state == MulticaExecutionBindingState::Failed
            && binding.last_error_code.as_deref() == Some(error_code)
            && binding.retryable == retryable
        {
            return Ok(binding.clone());
        }
        if binding.revision != expected_revision {
            bail!("execution_revision_conflict");
        }
        binding.state = MulticaExecutionBindingState::Failed;
        binding.last_error_code = Some(error_code.to_string());
        binding.retryable = retryable;
        binding.revision = binding.revision.saturating_add(1);
        binding.updated_at_ms = now_ms;
        binding.completed_at_ms = Some(now_ms);
        let result = binding.clone();
        fail_command_in_state(&mut state, &result.idempotency_key, error_code, now_ms)?;
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    pub fn get_execution(&self, binding_id: &str) -> anyhow::Result<CodexMulticaExecutionBinding> {
        validate_id(binding_id, "binding_id")?;
        self.load()?
            .execution_bindings
            .into_iter()
            .find(|binding| binding.binding_id == binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))
    }

    pub fn list_executions(
        &self,
        workspace_id: &str,
        issue_id: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> anyhow::Result<(Vec<CodexMulticaExecutionBinding>, usize)> {
        validate_id(workspace_id, "workspace_id")?;
        if let Some(issue_id) = issue_id {
            validate_id(issue_id, "issue_id")?;
        }
        if limit == 0 || limit > 100 || offset > 100_000 {
            bail!("execution_pagination_invalid");
        }
        let mut all = self
            .load()?
            .execution_bindings
            .into_iter()
            .filter(|binding| {
                binding.workspace_id == workspace_id
                    && issue_id.is_none_or(|issue_id| binding.issue_id == issue_id)
            })
            .collect::<Vec<_>>();
        all.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
        let total = all.len();
        Ok((all.into_iter().skip(offset).take(limit).collect(), total))
    }

    /// Cancel queued/running attempts when an Issue is deleted. Reassignment
    /// deliberately does not call this: upstream keeps existing attempts
    /// running while it enqueues work for the new assignee.
    pub fn cancel_active_for_issue(
        &self,
        workspace_id: &str,
        issue_id: &str,
        keep_agent_id: Option<&str>,
        now_ms: u64,
    ) -> anyhow::Result<usize> {
        validate_id(workspace_id, "workspace_id")?;
        validate_id(issue_id, "issue_id")?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let mut cancelled = 0;
        for binding in &mut state.execution_bindings {
            if binding.workspace_id != workspace_id
                || binding.issue_id != issue_id
                || binding.state.is_terminal()
                || keep_agent_id.is_some_and(|id| binding.agent_id.as_deref() == Some(id))
            {
                continue;
            }
            binding.state = MulticaExecutionBindingState::Cancelled;
            binding.retryable = false;
            binding.completed_at_ms = Some(now_ms);
            binding.updated_at_ms = now_ms;
            binding.revision = binding.revision.saturating_add(1);
            cancelled += 1;
        }
        if cancelled > 0 {
            validate_state(&state)?;
            save_state_locked(&self.path, &state)?;
        }
        Ok(cancelled)
    }

    pub fn reserve_command(
        &self,
        binding_id: &str,
        kind: MulticaExecutionCommandKind,
        command_id: &str,
        expected_binding_revision: u64,
        now_ms: u64,
    ) -> anyhow::Result<ExecutionCommandReservationResult> {
        validate_id(binding_id, "binding_id")?;
        validate_id(command_id, "command_id")?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        if let Some(existing) = state
            .execution_commands
            .iter()
            .find(|command| command.command_id == command_id)
        {
            if existing.binding_id == binding_id && existing.kind == kind {
                return Ok(ExecutionCommandReservationResult {
                    command: existing.clone(),
                    replay: true,
                });
            }
            bail!("execution_command_idempotency_conflict");
        }
        let binding = state
            .execution_bindings
            .iter()
            .find(|binding| binding.binding_id == binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))?;
        if binding.revision != expected_binding_revision {
            bail!("execution_revision_conflict");
        }
        if state.execution_commands.len() >= MAX_EXECUTION_COMMANDS {
            bail!("execution_commands_too_large");
        }
        let command = CodexMulticaExecutionCommand {
            command_id: command_id.to_string(),
            binding_id: binding_id.to_string(),
            kind,
            state: MulticaExecutionCommandState::Reserved,
            codex_execution_id: None,
            error_code: None,
            revision: 1,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };
        state.execution_commands.push(command.clone());
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(ExecutionCommandReservationResult {
            command,
            replay: false,
        })
    }

    pub fn get_command(&self, command_id: &str) -> anyhow::Result<CodexMulticaExecutionCommand> {
        validate_id(command_id, "command_id")?;
        self.load()?
            .execution_commands
            .into_iter()
            .find(|command| command.command_id == command_id)
            .ok_or_else(|| anyhow!("execution_command_unknown"))
    }

    pub fn commit_continue(
        &self,
        command_id: &str,
        expected_binding_revision: u64,
        handle: &CodexExecutionHandle,
        now_ms: u64,
    ) -> anyhow::Result<(CodexMulticaExecutionCommand, CodexMulticaExecutionBinding)> {
        validate_execution_handle(handle)?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let command_before = state
            .execution_commands
            .iter()
            .find(|command| command.command_id == command_id)
            .ok_or_else(|| anyhow!("execution_command_unknown"))?
            .clone();
        if command_before.kind != MulticaExecutionCommandKind::Continue {
            bail!("execution_command_kind_conflict");
        }
        let binding_index = state
            .execution_bindings
            .iter()
            .position(|binding| binding.binding_id == command_before.binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))?;
        if command_before.state == MulticaExecutionCommandState::Committed {
            if command_before.codex_execution_id == handle.execution_id {
                return Ok((
                    command_before,
                    state.execution_bindings[binding_index].clone(),
                ));
            }
            bail!("execution_command_conflict");
        }
        let binding = &state.execution_bindings[binding_index];
        if binding.revision != expected_binding_revision {
            bail!("execution_revision_conflict");
        }
        if binding.codex_thread_id.as_deref() != Some(handle.thread_id.as_str()) {
            bail!("execution_thread_conflict");
        }
        let command =
            commit_command_in_state(&mut state, command_id, handle.execution_id.clone(), now_ms)?;
        let binding = &mut state.execution_bindings[binding_index];
        binding.codex_runtime_id = Some(handle.runtime_id.clone());
        binding.codex_execution_id = handle.execution_id.clone();
        binding.state = MulticaExecutionBindingState::Dispatched;
        binding.codex_revision = binding.codex_revision.saturating_add(1);
        binding.revision = binding.revision.saturating_add(1);
        binding.updated_at_ms = now_ms;
        binding.completed_at_ms = None;
        binding.last_error_code = None;
        let binding = binding.clone();
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok((command, binding))
    }

    pub fn commit_cancel(
        &self,
        command_id: &str,
        expected_binding_revision: u64,
        status: &CodexExecutionStatus,
        now_ms: u64,
    ) -> anyhow::Result<(CodexMulticaExecutionCommand, CodexMulticaExecutionBinding)> {
        validate_id(&status.runtime_id, "runtime_id")?;
        validate_id(&status.thread_id, "thread_id")?;
        validate_id(&status.execution_id, "execution_id")?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let command_before = state
            .execution_commands
            .iter()
            .find(|command| command.command_id == command_id)
            .ok_or_else(|| anyhow!("execution_command_unknown"))?
            .clone();
        if command_before.kind != MulticaExecutionCommandKind::Cancel {
            bail!("execution_command_kind_conflict");
        }
        let binding_index = state
            .execution_bindings
            .iter()
            .position(|binding| binding.binding_id == command_before.binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))?;
        if command_before.state == MulticaExecutionCommandState::Committed {
            return Ok((
                command_before,
                state.execution_bindings[binding_index].clone(),
            ));
        }
        let binding = &state.execution_bindings[binding_index];
        if binding.revision != expected_binding_revision {
            bail!("execution_revision_conflict");
        }
        if binding.codex_thread_id.as_deref() != Some(status.thread_id.as_str()) {
            bail!("execution_thread_conflict");
        }
        let command = commit_command_in_state(
            &mut state,
            command_id,
            Some(status.execution_id.clone()),
            now_ms,
        )?;
        let binding = &mut state.execution_bindings[binding_index];
        binding.codex_runtime_id = Some(status.runtime_id.clone());
        binding.codex_execution_id = Some(status.execution_id.clone());
        binding.state = binding_state_from_codex(&status.state);
        binding.codex_revision = binding.codex_revision.saturating_add(1);
        binding.revision = binding.revision.saturating_add(1);
        binding.updated_at_ms = now_ms;
        if binding.state.is_terminal() {
            binding.completed_at_ms = Some(now_ms);
        }
        let binding = binding.clone();
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok((command, binding))
    }

    pub fn fail_command(
        &self,
        command_id: &str,
        error_code: &str,
        now_ms: u64,
    ) -> anyhow::Result<CodexMulticaExecutionCommand> {
        validate_error_code(error_code)?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let result = fail_command_in_state(&mut state, command_id, error_code, now_ms)?;
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    pub fn record_status(
        &self,
        binding_id: &str,
        expected_revision: u64,
        status: &CodexExecutionStatus,
        now_ms: u64,
    ) -> anyhow::Result<CodexMulticaExecutionBinding> {
        validate_id(binding_id, "binding_id")?;
        validate_id(&status.thread_id, "thread_id")?;
        validate_id(&status.execution_id, "execution_id")?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let binding = state
            .execution_bindings
            .iter_mut()
            .find(|binding| binding.binding_id == binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))?;
        if binding.revision != expected_revision {
            bail!("execution_revision_conflict");
        }
        if binding.codex_thread_id.as_deref() != Some(status.thread_id.as_str()) {
            bail!("execution_thread_conflict");
        }
        binding.codex_execution_id = Some(status.execution_id.clone());
        binding.state = binding_state_from_codex(&status.state);
        binding.codex_revision = binding.codex_revision.saturating_add(1);
        binding.revision = binding.revision.saturating_add(1);
        binding.updated_at_ms = now_ms;
        if binding.state.is_terminal() {
            binding.completed_at_ms = Some(now_ms);
        }
        let result = binding.clone();
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    /// Apply one upstream-style queue transition with revision and lease CAS.
    /// This is intentionally local and explicit; it never claims a remote
    /// runtime or starts work by itself.
    pub fn transition_queue(
        &self,
        input: QueueTransition,
    ) -> anyhow::Result<CodexMulticaExecutionBinding> {
        validate_id(&input.binding_id, "binding_id")?;
        if let Some(reason) = input.failure_reason.as_deref() {
            validate_error_code(reason)?;
        }
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let binding = state
            .execution_bindings
            .iter_mut()
            .find(|binding| binding.binding_id == input.binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))?;
        if binding.revision != input.expected_revision {
            bail!("execution_revision_conflict");
        }
        if !binding.state.queue_transition_allowed(input.next_state) {
            if binding.state == input.next_state {
                return Ok(binding.clone());
            }
            bail!("agent_task_queue_transition_invalid");
        }
        if !matches!(
            input.next_state,
            MulticaExecutionBindingState::BindingPending
        ) && binding.lease_token.is_some()
            && binding.lease_token != input.lease_token
        {
            bail!("execution_lease_conflict");
        }
        binding.state = input.next_state;
        binding.last_error_code = input.failure_reason;
        binding.last_heartbeat_at_ms = Some(input.now_ms);
        binding.updated_at_ms = input.now_ms;
        if binding.state == MulticaExecutionBindingState::Dispatched {
            binding.retryable = false;
        }
        if binding.state.is_terminal() {
            binding.completed_at_ms = Some(input.now_ms);
            binding.lease_expires_at_ms = None;
            binding.lease_token = None;
        }
        binding.revision = binding.revision.saturating_add(1);
        let result = binding.clone();
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    /// Atomically claim or take over an execution lease. This mirrors the
    /// upstream ClaimAgentTask predicate while remaining local and explicit.
    pub fn claim_execution_lease(
        &self,
        binding_id: &str,
        expected_revision: u64,
        lease_token: &str,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> anyhow::Result<CodexMulticaExecutionBinding> {
        validate_id(binding_id, "binding_id")?;
        validate_id(lease_token, "lease_token")?;
        if lease_duration_ms == 0 || lease_duration_ms > 86_400_000 {
            bail!("execution_lease_duration_invalid");
        }
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let binding = state
            .execution_bindings
            .iter_mut()
            .find(|binding| binding.binding_id == binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))?;
        if binding.state.is_terminal() {
            bail!("execution_lease_terminal");
        }
        if binding.revision != expected_revision {
            bail!("execution_revision_conflict");
        }
        if binding
            .lease_expires_at_ms
            .is_some_and(|expires| expires > now_ms)
            && binding.lease_token.as_deref() != Some(lease_token)
        {
            bail!("execution_lease_conflict");
        }
        binding.lease_token = Some(lease_token.to_string());
        binding.lease_expires_at_ms = Some(now_ms.saturating_add(lease_duration_ms));
        binding.last_heartbeat_at_ms = Some(now_ms);
        binding.revision = binding.revision.saturating_add(1);
        binding.updated_at_ms = now_ms;
        let result = binding.clone();
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    pub fn renew_execution_lease(
        &self,
        binding_id: &str,
        expected_revision: u64,
        lease_token: &str,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> anyhow::Result<CodexMulticaExecutionBinding> {
        validate_id(binding_id, "binding_id")?;
        validate_id(lease_token, "lease_token")?;
        if lease_duration_ms == 0 || lease_duration_ms > 86_400_000 {
            bail!("execution_lease_duration_invalid");
        }
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let binding = state
            .execution_bindings
            .iter_mut()
            .find(|binding| binding.binding_id == binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))?;
        if binding.revision != expected_revision {
            bail!("execution_revision_conflict");
        }
        if binding.state.is_terminal() || binding.lease_token.as_deref() != Some(lease_token) {
            bail!("execution_lease_conflict");
        }
        if binding
            .lease_expires_at_ms
            .is_some_and(|expires| expires <= now_ms)
        {
            bail!("execution_lease_expired");
        }
        binding.lease_expires_at_ms = Some(now_ms.saturating_add(lease_duration_ms));
        binding.last_heartbeat_at_ms = Some(now_ms);
        binding.revision = binding.revision.saturating_add(1);
        binding.updated_at_ms = now_ms;
        let result = binding.clone();
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    pub fn release_execution_lease(
        &self,
        binding_id: &str,
        expected_revision: u64,
        lease_token: &str,
        now_ms: u64,
    ) -> anyhow::Result<CodexMulticaExecutionBinding> {
        validate_id(binding_id, "binding_id")?;
        validate_id(lease_token, "lease_token")?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        let binding = state
            .execution_bindings
            .iter_mut()
            .find(|binding| binding.binding_id == binding_id)
            .ok_or_else(|| anyhow!("execution_binding_unknown"))?;
        if binding.revision != expected_revision {
            bail!("execution_revision_conflict");
        }
        if binding.lease_token.as_deref() != Some(lease_token) {
            bail!("execution_lease_conflict");
        }
        binding.lease_token = None;
        binding.lease_expires_at_ms = None;
        binding.last_heartbeat_at_ms = Some(now_ms);
        binding.revision = binding.revision.saturating_add(1);
        binding.updated_at_ms = now_ms;
        let result = binding.clone();
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(result)
    }

    pub fn append_task_message(
        &self,
        message: CodexMulticaTaskMessage,
    ) -> anyhow::Result<CodexMulticaTaskMessage> {
        validate_task_message(&message)?;
        let _guard = store_lock(&self.path)?;
        let mut state = load_state(&self.path)?;
        if !state
            .execution_bindings
            .iter()
            .any(|b| b.binding_id == message.binding_id)
        {
            bail!("execution_binding_unknown");
        }
        if let Some(existing) = state
            .task_messages
            .iter()
            .find(|m| m.binding_id == message.binding_id && m.seq == message.seq)
        {
            if existing == &message {
                return Ok(existing.clone());
            }
            bail!("task_message_conflict");
        }
        if state.task_messages.len() >= MAX_TASK_MESSAGES {
            bail!("task_messages_too_large");
        }
        state.task_messages.push(message.clone());
        validate_state(&state)?;
        save_state_locked(&self.path, &state)?;
        Ok(message)
    }

    pub fn list_task_messages(
        &self,
        binding_id: &str,
    ) -> anyhow::Result<Vec<CodexMulticaTaskMessage>> {
        validate_id(binding_id, "binding_id")?;
        let mut messages = self
            .load()?
            .task_messages
            .into_iter()
            .filter(|m| m.binding_id == binding_id)
            .collect::<Vec<_>>();
        messages.sort_by_key(|m| m.seq);
        Ok(messages)
    }
}

fn load_state(path: &Path) -> anyhow::Result<MulticaExecutionState> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MulticaExecutionState {
                version: STORE_VERSION,
                ..Default::default()
            });
        }
        Err(_) => bail!("multica_execution_store_read_failed"),
    };
    if bytes.len() > 4 * 1024 * 1024 {
        bail!("multica_execution_store_too_large");
    }
    let state: MulticaExecutionState =
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("multica_execution_store_invalid"))?;
    validate_state(&state)?;
    Ok(state)
}

fn save_state_locked(path: &Path, state: &MulticaExecutionState) -> anyhow::Result<()> {
    let bytes =
        serde_json::to_vec_pretty(state).map_err(|_| anyhow!("multica_execution_store_invalid"))?;
    crate::settings::atomic_write(path, &bytes)
        .map_err(|_| anyhow!("multica_execution_store_write_failed"))
}

fn store_lock(path: &Path) -> anyhow::Result<StoreGuard> {
    static PROCESS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = PROCESS_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("multica_execution_store_lock_unavailable"))?;
    let lock_path = PathBuf::from(format!("{}.lock", path.to_string_lossy()));
    if let Some(parent) = lock_path.parent() {
        crate::settings::create_private_dir_all(parent)
            .map_err(|_| anyhow!("multica_execution_store_lock_unavailable"))?;
    }
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(|_| anyhow!("multica_execution_store_lock_unavailable"))?;
    file.lock_exclusive()
        .map_err(|_| anyhow!("multica_execution_store_lock_unavailable"))?;
    Ok(StoreGuard {
        _process: guard,
        file,
    })
}

struct StoreGuard {
    _process: std::sync::MutexGuard<'static, ()>,
    file: fs::File,
}

impl Drop for StoreGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn validate_state(state: &MulticaExecutionState) -> anyhow::Result<()> {
    if state.version != STORE_VERSION
        || state.skill_bindings.len() > MAX_BINDINGS
        || state.attempt_skill_snapshots.len() > MAX_SNAPSHOTS
        || state.execution_bindings.len() > MAX_EXECUTION_BINDINGS
        || state.execution_commands.len() > MAX_EXECUTION_COMMANDS
    {
        bail!("multica_execution_store_invalid");
    }
    let mut binding_keys = BTreeSet::new();
    for binding in &state.skill_bindings {
        validate_id(&binding.binding_id, "binding_id")?;
        validate_id(&binding.workspace_id, "workspace_id")?;
        validate_id(&binding.scope_id, "scope_id")?;
        validate_reference(&binding.skill_ref)?;
        validate_text(&binding.source_kind, MAX_SOURCE_LENGTH, "source_kind")?;
        validate_text(&binding.trust_state, MAX_SOURCE_LENGTH, "trust_state")?;
        if binding.revision == 0 || !binding_keys.insert(binding_key_for(binding)) {
            bail!("skill_binding_conflict");
        }
    }
    let mut snapshot_keys = BTreeSet::new();
    for snapshot in &state.attempt_skill_snapshots {
        validate_id(&snapshot.snapshot_id, "snapshot_id")?;
        validate_id(&snapshot.execution_binding_id, "execution_binding_id")?;
        if snapshot.attempt_no == 0 {
            bail!("attempt_no_invalid");
        }
        validate_reference_list(&snapshot.requested_skill_refs)?;
        validate_reference_list(&snapshot.resolved_skill_refs)?;
        validate_reference_list(&snapshot.runtime_loaded_skill_refs)?;
        validate_manifest_digest(
            &snapshot.resolved_manifest_digest,
            "resolved_manifest_digest",
        )?;
        validate_text(
            &snapshot.resolution_status,
            MAX_SOURCE_LENGTH,
            "resolution_status",
        )?;
        if let Some(error) = snapshot.resolution_error_code.as_deref() {
            validate_text(error, MAX_ERROR_LENGTH, "resolution_error_code")?;
        }
        if !snapshot_keys.insert((snapshot.execution_binding_id.clone(), snapshot.attempt_no)) {
            bail!("attempt_skill_snapshot_conflict");
        }
    }
    let mut execution_ids = BTreeSet::new();
    let mut idempotency_keys = BTreeSet::new();
    let mut attempts = BTreeSet::new();
    let mut thread_ids = BTreeSet::new();
    for binding in &state.execution_bindings {
        validate_execution_binding(binding)?;
        if !execution_ids.insert(binding.binding_id.to_ascii_lowercase())
            || !idempotency_keys.insert(binding.idempotency_key.to_ascii_lowercase())
            || !attempts.insert((
                binding.workspace_id.to_ascii_lowercase(),
                binding.issue_id.to_ascii_lowercase(),
                binding.attempt_no,
            ))
        {
            bail!("execution_binding_conflict");
        }
        if let Some(thread_id) = binding.codex_thread_id.as_deref()
            && !thread_ids.insert(thread_id.to_ascii_lowercase())
        {
            bail!("execution_thread_conflict");
        }
    }
    let mut message_keys = BTreeSet::new();
    for message in &state.task_messages {
        validate_task_message(message)?;
        if !execution_ids.contains(&message.binding_id.to_ascii_lowercase())
            || !message_keys.insert((message.binding_id.to_ascii_lowercase(), message.seq))
        {
            bail!("task_message_conflict");
        }
    }
    let mut command_ids = BTreeSet::new();
    for command in &state.execution_commands {
        validate_execution_command(command)?;
        if !command_ids.insert(command.command_id.to_ascii_lowercase()) {
            bail!("execution_command_conflict");
        }
        if !execution_ids.contains(&command.binding_id.to_ascii_lowercase()) {
            bail!("execution_command_binding_unknown");
        }
    }
    Ok(())
}

fn validate_execution_reservation(input: &ExecutionReservation) -> anyhow::Result<()> {
    validate_id(&input.workspace_id, "workspace_id")?;
    validate_id(&input.issue_id, "issue_id")?;
    validate_id(&input.idempotency_key, "idempotency_key")?;
    if let Some(parent) = input.parent_thread_id.as_deref() {
        validate_id(parent, "parent_thread_id")?;
    }
    if let Some(parent) = input.parent_attempt_id.as_deref() {
        validate_id(parent, "parent_attempt_id")?;
    }
    if let Some(agent) = input.agent_id.as_deref() {
        validate_id(agent, "agent_id")?;
    }
    if input.execution_kind == MulticaExecutionKind::Subagent && input.parent_thread_id.is_none() {
        bail!("parent_thread_id_required");
    }
    Ok(())
}

fn validate_execution_binding(binding: &CodexMulticaExecutionBinding) -> anyhow::Result<()> {
    validate_id(&binding.binding_id, "binding_id")?;
    validate_id(&binding.workspace_id, "workspace_id")?;
    validate_id(&binding.issue_id, "issue_id")?;
    if let Some(agent) = binding.agent_id.as_deref() {
        validate_id(agent, "agent_id")?;
    }
    validate_id(&binding.multica_run_id, "multica_run_id")?;
    validate_id(&binding.idempotency_key, "idempotency_key")?;
    if binding.attempt_no == 0
        || binding.max_attempts == 0
        || binding.attempt_no > binding.max_attempts
        || binding.revision == 0
    {
        bail!("execution_binding_invalid");
    }
    for (value, field) in [
        (binding.codex_thread_id.as_deref(), "thread_id"),
        (binding.codex_runtime_id.as_deref(), "runtime_id"),
        (binding.codex_execution_id.as_deref(), "execution_id"),
        (binding.parent_thread_id.as_deref(), "parent_thread_id"),
        (binding.parent_attempt_id.as_deref(), "parent_attempt_id"),
        (binding.last_event_id.as_deref(), "event_id"),
    ] {
        if let Some(value) = value {
            validate_id(value, field)?;
        }
    }
    if let Some(error) = binding.last_error_code.as_deref() {
        validate_error_code(error)?;
    }
    if let Some(token) = binding.lease_token.as_deref() {
        validate_id(token, "lease_token")?;
        if binding.lease_expires_at_ms.is_none() {
            bail!("execution_lease_invalid");
        }
    } else if binding.lease_expires_at_ms.is_some() {
        bail!("execution_lease_invalid");
    }
    Ok(())
}

fn validate_task_message(message: &CodexMulticaTaskMessage) -> anyhow::Result<()> {
    validate_id(&message.message_id, "message_id")?;
    validate_id(&message.binding_id, "binding_id")?;
    if message.seq == 0 {
        bail!("task_message_seq_invalid");
    }
    validate_text(&message.message_type, MAX_SOURCE_LENGTH, "message_type")?;
    if let Some(tool) = message.tool.as_deref() {
        validate_text(tool, MAX_SOURCE_LENGTH, "message_tool")?;
    }
    if let Some(summary) = message.summary.as_deref() {
        validate_text(summary, MAX_MESSAGE_SUMMARY_LENGTH, "message_summary")?;
    }
    Ok(())
}

fn validate_execution_command(command: &CodexMulticaExecutionCommand) -> anyhow::Result<()> {
    validate_id(&command.command_id, "command_id")?;
    validate_id(&command.binding_id, "binding_id")?;
    if command.revision == 0 {
        bail!("execution_command_invalid");
    }
    if let Some(id) = command.codex_execution_id.as_deref() {
        validate_id(id, "execution_id")?;
    }
    if let Some(error) = command.error_code.as_deref() {
        validate_error_code(error)?;
    }
    Ok(())
}

fn validate_execution_handle(handle: &CodexExecutionHandle) -> anyhow::Result<()> {
    validate_id(&handle.runtime_id, "runtime_id")?;
    validate_id(&handle.thread_id, "thread_id")?;
    validate_id(&handle.idempotency_key, "idempotency_key")?;
    if let Some(execution_id) = handle.execution_id.as_deref() {
        validate_id(execution_id, "execution_id")?;
    }
    if let Some(parent_thread_id) = handle.parent_thread_id.as_deref() {
        validate_id(parent_thread_id, "parent_thread_id")?;
    }
    Ok(())
}

fn validate_error_code(error_code: &str) -> anyhow::Result<()> {
    validate_text(error_code, MAX_ERROR_LENGTH, "execution_error_code")?;
    if !error_code
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        bail!("execution_error_code_invalid");
    }
    Ok(())
}

fn stable_execution_id(prefix: &str, idempotency_key: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(idempotency_key.as_bytes());
    format!(
        "{prefix}:{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7]
    )
}

fn commit_command_in_state(
    state: &mut MulticaExecutionState,
    command_id: &str,
    codex_execution_id: Option<String>,
    now_ms: u64,
) -> anyhow::Result<CodexMulticaExecutionCommand> {
    validate_id(command_id, "command_id")?;
    if let Some(id) = codex_execution_id.as_deref() {
        validate_id(id, "execution_id")?;
    }
    let command = state
        .execution_commands
        .iter_mut()
        .find(|command| command.command_id == command_id)
        .ok_or_else(|| anyhow!("execution_command_unknown"))?;
    if command.state == MulticaExecutionCommandState::Committed {
        if command.codex_execution_id == codex_execution_id {
            return Ok(command.clone());
        }
        bail!("execution_command_conflict");
    }
    if command.state != MulticaExecutionCommandState::Reserved {
        bail!("execution_command_state_conflict");
    }
    command.state = MulticaExecutionCommandState::Committed;
    command.codex_execution_id = codex_execution_id;
    command.error_code = None;
    command.revision = command.revision.saturating_add(1);
    command.updated_at_ms = now_ms;
    Ok(command.clone())
}

fn fail_command_in_state(
    state: &mut MulticaExecutionState,
    command_id: &str,
    error_code: &str,
    now_ms: u64,
) -> anyhow::Result<CodexMulticaExecutionCommand> {
    validate_id(command_id, "command_id")?;
    validate_error_code(error_code)?;
    let command = state
        .execution_commands
        .iter_mut()
        .find(|command| command.command_id == command_id)
        .ok_or_else(|| anyhow!("execution_command_unknown"))?;
    if command.state == MulticaExecutionCommandState::Failed
        && command.error_code.as_deref() == Some(error_code)
    {
        return Ok(command.clone());
    }
    if command.state != MulticaExecutionCommandState::Reserved {
        bail!("execution_command_state_conflict");
    }
    command.state = MulticaExecutionCommandState::Failed;
    command.error_code = Some(error_code.to_string());
    command.revision = command.revision.saturating_add(1);
    command.updated_at_ms = now_ms;
    Ok(command.clone())
}

fn binding_state_from_codex(state: &CodexExecutionState) -> MulticaExecutionBindingState {
    match state {
        CodexExecutionState::Unknown => MulticaExecutionBindingState::Stale,
        CodexExecutionState::Queued => MulticaExecutionBindingState::Dispatched,
        CodexExecutionState::Running => MulticaExecutionBindingState::Running,
        CodexExecutionState::Completed => MulticaExecutionBindingState::Completed,
        CodexExecutionState::Failed => MulticaExecutionBindingState::Failed,
        CodexExecutionState::Cancelled => MulticaExecutionBindingState::Cancelled,
        CodexExecutionState::CancelPending => MulticaExecutionBindingState::CancelPending,
    }
}

fn validate_binding_input(input: &SkillBindingUpsert) -> anyhow::Result<()> {
    validate_id(&input.binding_id, "binding_id")?;
    validate_id(&input.workspace_id, "workspace_id")?;
    validate_id(&input.scope_id, "scope_id")?;
    validate_reference(&input.skill_ref)?;
    validate_text(&input.source_kind, MAX_SOURCE_LENGTH, "source_kind")?;
    validate_text(&input.trust_state, MAX_SOURCE_LENGTH, "trust_state")?;
    Ok(())
}

fn validate_resolution_audit(audit: &SkillResolutionAudit) -> anyhow::Result<()> {
    validate_reference_list(&audit.requested_skill_refs)?;
    validate_reference_list(&audit.resolved_skill_refs)?;
    validate_manifest_digest(&audit.resolved_manifest_digest, "resolved_manifest_digest")?;
    validate_text(&audit.protocol, MAX_SOURCE_LENGTH, "protocol")?;
    validate_text(
        &audit.resolution_status,
        MAX_SOURCE_LENGTH,
        "resolution_status",
    )?;
    if let Some(error) = audit.resolution_error_code.as_deref() {
        validate_text(error, MAX_ERROR_LENGTH, "resolution_error_code")?;
    }
    Ok(())
}

fn validate_reference_list(refs: &[SkillReference]) -> anyhow::Result<()> {
    if refs.len() > 64 {
        bail!("skill_refs_too_large");
    }
    let mut ids = BTreeSet::new();
    for reference in refs {
        validate_reference(reference)?;
        if !ids.insert(reference.id.to_ascii_lowercase()) {
            bail!("skill_reference_conflict");
        }
    }
    Ok(())
}

fn validate_reference(reference: &SkillReference) -> anyhow::Result<()> {
    validate_id(&reference.id, "skill_id")?;
    if let Some(digest) = reference.manifest_digest.as_deref() {
        validate_manifest_digest(digest, "manifest_digest")?;
    }
    Ok(())
}

fn validate_manifest_digest(value: &str, field: &str) -> anyhow::Result<()> {
    let valid = if let Some(hex) = value.strip_prefix("sha256:") {
        hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    } else {
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || matches!(byte, b':' | b'-' | b'_'))
    };
    if !valid {
        bail!("{field}_invalid");
    }
    Ok(())
}

fn validate_id(value: &str, field: &str) -> anyhow::Result<()> {
    if value.is_empty()
        || value.len() > MAX_ID_LENGTH
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'@')
        })
    {
        bail!("{field}_invalid");
    }
    Ok(())
}

fn validate_text(value: &str, max_len: usize, field: &str) -> anyhow::Result<()> {
    if value.is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        bail!("{field}_invalid");
    }
    Ok(())
}

fn binding_key(
    workspace_id: &str,
    scope_kind: SkillBindingScope,
    scope_id: &str,
    skill_id: &str,
) -> (String, SkillBindingScope, String, String) {
    (
        workspace_id.to_ascii_lowercase(),
        scope_kind,
        scope_id.to_ascii_lowercase(),
        skill_id.to_ascii_lowercase(),
    )
}

fn binding_key_for(
    binding: &CodexMulticaSkillBinding,
) -> (String, SkillBindingScope, String, String) {
    binding_key(
        &binding.workspace_id,
        binding.scope_kind,
        &binding.scope_id,
        &binding.skill_ref.id,
    )
}

fn same_resolution_snapshot(
    existing: &CodexMulticaAttemptSkillSnapshot,
    execution_binding_id: &str,
    attempt_no: u32,
    audit: &SkillResolutionAudit,
) -> bool {
    existing.execution_binding_id == execution_binding_id
        && existing.attempt_no == attempt_no
        && existing.requested_skill_refs == audit.requested_skill_refs
        && existing.resolved_skill_refs == audit.resolved_skill_refs
        && existing.resolved_manifest_digest == audit.resolved_manifest_digest
        && existing.resolution_status == audit.resolution_status
        && existing.resolution_error_code == audit.resolution_error_code
}

fn stable_snapshot_id(
    execution_binding_id: &str,
    attempt_no: u32,
    audit: &SkillResolutionAudit,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(execution_binding_id.as_bytes());
    hasher.update(attempt_no.to_le_bytes());
    hasher.update(serde_json::to_vec(audit).unwrap_or_default());
    let digest = hasher.finalize();
    format!(
        "attempt:{}:{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        attempt_no,
        digest[0],
        digest[1],
        digest[2],
        digest[3],
        digest[4],
        digest[5],
        digest[6],
        digest[7]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multica_execution::SkillBindings;

    fn reference(id: &str) -> SkillReference {
        SkillReference {
            id: id.to_string(),
            manifest_digest: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            ),
        }
    }

    fn audit() -> SkillResolutionAudit {
        SkillResolutionAudit {
            requested_skill_refs: vec![reference("skill:a")],
            resolved_skill_refs: vec![reference("skill:a")],
            resolved_manifest_digest:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            protocol: "skill-bundles-v1".to_string(),
            resolution_status: "resolved".to_string(),
            resolution_error_code: None,
        }
    }

    #[test]
    fn bindings_use_cas_and_reject_duplicate_scope_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let input = SkillBindingUpsert {
            binding_id: "binding-a".to_string(),
            workspace_id: "workspace-a".to_string(),
            scope_kind: SkillBindingScope::Agent,
            scope_id: "agent-a".to_string(),
            skill_ref: reference("skill:a"),
            source_kind: "explicit_review".to_string(),
            trust_state: "trusted".to_string(),
            enabled: true,
            expected_revision: None,
            now_ms: 1,
        };
        let first = store.upsert_binding(input.clone()).unwrap();
        assert_eq!(first.revision, 1);
        let error = store
            .upsert_binding(SkillBindingUpsert {
                expected_revision: Some(0),
                ..input.clone()
            })
            .unwrap_err();
        assert_eq!(error.to_string(), "skill_binding_revision_conflict");
        let updated = store
            .upsert_binding(SkillBindingUpsert {
                expected_revision: Some(1),
                enabled: false,
                now_ms: 2,
                ..input
            })
            .unwrap();
        assert_eq!(updated.revision, 2);
        assert!(!updated.enabled);
    }

    #[test]
    fn replace_bindings_is_atomic_supports_clear_and_scope_cas() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let make = |id: &str| SkillBindingUpsert {
            binding_id: format!("binding-{id}"),
            workspace_id: "workspace-a".to_string(),
            scope_kind: SkillBindingScope::Agent,
            scope_id: "agent-a".to_string(),
            skill_ref: reference(id),
            source_kind: "explicit_review".to_string(),
            trust_state: "trusted".to_string(),
            enabled: true,
            expected_revision: None,
            now_ms: 1,
        };
        let first = store
            .replace_bindings(SkillBindingReplaceAll {
                workspace_id: "workspace-a".to_string(),
                scope_kind: SkillBindingScope::Agent,
                scope_id: "agent-a".to_string(),
                bindings: vec![make("skill:a"), make("skill:b")],
                expected_revision: Some(0),
                now_ms: 1,
            })
            .unwrap();
        assert_eq!(first.len(), 2);
        let cleared = store
            .replace_bindings(SkillBindingReplaceAll {
                workspace_id: "workspace-a".to_string(),
                scope_kind: SkillBindingScope::Agent,
                scope_id: "agent-a".to_string(),
                bindings: vec![],
                expected_revision: Some(1),
                now_ms: 2,
            })
            .unwrap();
        assert!(cleared.is_empty());
        assert!(
            store
                .list_bindings(
                    "workspace-a",
                    Some(SkillBindingScope::Agent),
                    Some("agent-a")
                )
                .unwrap()
                .is_empty()
        );
        let error = store
            .replace_bindings(SkillBindingReplaceAll {
                workspace_id: "workspace-a".to_string(),
                scope_kind: SkillBindingScope::Agent,
                scope_id: "agent-a".to_string(),
                bindings: vec![make("skill:a"), make("skill:a")],
                expected_revision: Some(0),
                now_ms: 3,
            })
            .unwrap_err();
        assert_eq!(error.to_string(), "skill_binding_duplicate");
        assert!(
            store
                .list_bindings(
                    "workspace-a",
                    Some(SkillBindingScope::Agent),
                    Some("agent-a")
                )
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn snapshots_are_idempotent_immutable_and_loaded_set_is_separate() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let audit = audit();
        let first = store
            .reserve_attempt_snapshot("execution-a", 1, &audit, 1)
            .unwrap();
        let replay = store
            .reserve_attempt_snapshot("execution-a", 1, &audit, 2)
            .unwrap();
        assert_eq!(first, replay);
        let loaded = store
            .record_runtime_loaded(&first.snapshot_id, vec![reference("skill:a")], 3)
            .unwrap();
        assert_eq!(loaded.resolution_status, "loaded");
        assert_eq!(loaded.requested_skill_refs, audit.requested_skill_refs);
        let conflict = store
            .reserve_attempt_snapshot(
                "execution-a",
                1,
                &SkillResolutionAudit {
                    requested_skill_refs: vec![],
                    ..audit
                },
                4,
            )
            .unwrap_err();
        assert_eq!(conflict.to_string(), "attempt_skill_snapshot_conflict");
        let encoded = std::fs::read_to_string(store.path()).unwrap();
        assert!(!encoded.contains("prompt"));
        assert!(!encoded.contains("C:\\\\Users"));
        let _ = SkillBindings::default();
    }

    #[test]
    fn execution_reservation_commit_status_and_retry_are_persistent_and_cas_guarded() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let reservation = ExecutionReservation {
            workspace_id: "workspace-a".to_string(),
            issue_id: "issue-a".to_string(),
            agent_id: None,
            execution_kind: MulticaExecutionKind::Thread,
            parent_thread_id: None,
            parent_attempt_id: None,
            idempotency_key: "create-a".to_string(),
            now_ms: 1,
        };
        let first = store.reserve_execution(reservation.clone()).unwrap();
        assert!(!first.replay);
        assert_eq!(first.binding.attempt_no, 1);
        let replay = store.reserve_execution(reservation).unwrap();
        assert!(replay.replay);
        assert_eq!(first.binding.binding_id, replay.binding.binding_id);

        let handle = CodexExecutionHandle {
            runtime_id: "codex-current-page".to_string(),
            thread_id: "thread-a".to_string(),
            execution_id: Some("turn-a".to_string()),
            parent_thread_id: None,
            idempotency_key: "create-a".to_string(),
        };
        assert_eq!(
            store
                .commit_execution(&first.binding.binding_id, 0, &handle, 2)
                .unwrap_err()
                .to_string(),
            "execution_revision_conflict"
        );
        let committed = store
            .commit_execution(&first.binding.binding_id, 1, &handle, 2)
            .unwrap();
        assert_eq!(committed.state, MulticaExecutionBindingState::Dispatched);
        assert_eq!(committed.revision, 2);
        assert_eq!(
            store
                .commit_execution(&first.binding.binding_id, 1, &handle, 3)
                .unwrap(),
            committed
        );

        let completed = store
            .record_status(
                &first.binding.binding_id,
                2,
                &CodexExecutionStatus {
                    runtime_id: "codex-current-page".to_string(),
                    thread_id: "thread-a".to_string(),
                    execution_id: "turn-a".to_string(),
                    state: CodexExecutionState::Completed,
                    diagnostic: Some("not persisted".to_string()),
                },
                4,
            )
            .unwrap();
        assert_eq!(completed.state, MulticaExecutionBindingState::Completed);

        let retry = store
            .reserve_execution(ExecutionReservation {
                idempotency_key: "create-b".to_string(),
                now_ms: 5,
                ..ExecutionReservation {
                    workspace_id: "workspace-a".to_string(),
                    issue_id: "issue-a".to_string(),
                    agent_id: None,
                    execution_kind: MulticaExecutionKind::Thread,
                    parent_thread_id: Some("thread-a".to_string()),
                    parent_attempt_id: Some(first.binding.binding_id.clone()),
                    idempotency_key: String::new(),
                    now_ms: 0,
                }
            })
            .unwrap();
        assert_eq!(retry.binding.attempt_no, 2);
        let encoded = std::fs::read_to_string(store.path()).unwrap();
        assert!(!encoded.contains("not persisted"));
        assert!(!encoded.contains("prompt"));
        assert!(!encoded.contains("cwd"));
    }

    #[test]
    fn execution_commands_are_idempotent_and_do_not_store_command_bodies() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let reserved = store
            .reserve_execution(ExecutionReservation {
                workspace_id: "workspace-a".to_string(),
                issue_id: "issue-a".to_string(),
                agent_id: None,
                execution_kind: MulticaExecutionKind::Thread,
                parent_thread_id: None,
                parent_attempt_id: None,
                idempotency_key: "create-a".to_string(),
                now_ms: 1,
            })
            .unwrap();
        let handle = CodexExecutionHandle {
            runtime_id: "codex-current-page".to_string(),
            thread_id: "thread-a".to_string(),
            execution_id: Some("turn-a".to_string()),
            parent_thread_id: None,
            idempotency_key: "create-a".to_string(),
        };
        let binding = store
            .commit_execution(&reserved.binding.binding_id, 1, &handle, 2)
            .unwrap();
        let command = store
            .reserve_command(
                &binding.binding_id,
                MulticaExecutionCommandKind::Continue,
                "continue-a",
                binding.revision,
                3,
            )
            .unwrap();
        assert!(!command.replay);
        assert!(
            store
                .reserve_command(
                    &binding.binding_id,
                    MulticaExecutionCommandKind::Continue,
                    "continue-a",
                    binding.revision,
                    4,
                )
                .unwrap()
                .replay
        );
        let continued = CodexExecutionHandle {
            execution_id: Some("turn-b".to_string()),
            idempotency_key: "continue-a".to_string(),
            ..handle
        };
        let (_, updated) = store
            .commit_continue("continue-a", binding.revision, &continued, 5)
            .unwrap();
        assert_eq!(updated.codex_execution_id.as_deref(), Some("turn-b"));
        assert_eq!(
            store.get_command("continue-a").unwrap().state,
            MulticaExecutionCommandState::Committed
        );
    }

    #[test]
    fn queue_transition_enforces_upstream_lifecycle_and_lease_cas() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let reserved = store
            .reserve_execution(ExecutionReservation {
                workspace_id: "workspace-a".into(),
                issue_id: "issue-a".into(),
                agent_id: None,
                execution_kind: MulticaExecutionKind::Thread,
                parent_thread_id: None,
                parent_attempt_id: None,
                idempotency_key: "queue-a".into(),
                now_ms: 1,
            })
            .unwrap();
        let dispatched = store
            .transition_queue(QueueTransition {
                binding_id: reserved.binding.binding_id.clone(),
                expected_revision: 1,
                lease_token: None,
                next_state: MulticaExecutionBindingState::Dispatched,
                failure_reason: None,
                now_ms: 2,
            })
            .unwrap();
        assert_eq!(dispatched.revision, 2);
        let invalid = store
            .transition_queue(QueueTransition {
                binding_id: dispatched.binding_id.clone(),
                expected_revision: dispatched.revision,
                lease_token: None,
                next_state: MulticaExecutionBindingState::Completed,
                failure_reason: None,
                now_ms: 3,
            })
            .unwrap_err();
        assert_eq!(invalid.to_string(), "agent_task_queue_transition_invalid");
        let claimed = store
            .claim_execution_lease(
                &dispatched.binding_id,
                dispatched.revision,
                "lease-a",
                4,
                100,
            )
            .unwrap();
        let running = store
            .transition_queue(QueueTransition {
                binding_id: claimed.binding_id.clone(),
                expected_revision: claimed.revision,
                lease_token: Some("lease-a".into()),
                next_state: MulticaExecutionBindingState::Running,
                failure_reason: None,
                now_ms: 5,
            })
            .unwrap();
        assert_eq!(running.state, MulticaExecutionBindingState::Running);
        let completed = store
            .transition_queue(QueueTransition {
                binding_id: running.binding_id,
                expected_revision: running.revision,
                lease_token: Some("lease-a".into()),
                next_state: MulticaExecutionBindingState::Completed,
                failure_reason: None,
                now_ms: 6,
            })
            .unwrap();
        assert_eq!(completed.state, MulticaExecutionBindingState::Completed);
        assert!(completed.lease_token.is_none());
    }

    #[test]
    fn reassignment_keeps_old_active_agent_attempts_and_allows_new_agent() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        store
            .reserve_execution(ExecutionReservation {
                workspace_id: "workspace-a".into(),
                issue_id: "issue-a".into(),
                agent_id: Some("agent-a".into()),
                execution_kind: MulticaExecutionKind::Thread,
                parent_thread_id: None,
                parent_attempt_id: None,
                idempotency_key: "assignment-a".into(),
                now_ms: 1,
            })
            .unwrap();
        store
            .reserve_execution(ExecutionReservation {
                workspace_id: "workspace-a".into(),
                issue_id: "issue-a".into(),
                agent_id: Some("agent-b".into()),
                execution_kind: MulticaExecutionKind::Thread,
                parent_thread_id: None,
                parent_attempt_id: None,
                idempotency_key: "assignment-b".into(),
                now_ms: 2,
            })
            .unwrap();
        let (bindings, _) = store
            .list_executions("workspace-a", Some("issue-a"), 10, 0)
            .unwrap();
        assert_eq!(
            bindings.iter().filter(|b| !b.state.is_terminal()).count(),
            2
        );
        assert_eq!(
            store
                .cancel_active_for_issue("workspace-a", "issue-a", None, 3)
                .unwrap(),
            2
        );
    }

    #[test]
    fn execution_lease_is_token_guarded_and_takeover_after_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let reserved = store
            .reserve_execution(ExecutionReservation {
                workspace_id: "workspace-a".into(),
                issue_id: "issue-a".into(),
                agent_id: None,
                execution_kind: MulticaExecutionKind::Thread,
                parent_thread_id: None,
                parent_attempt_id: None,
                idempotency_key: "lease-a".into(),
                now_ms: 1,
            })
            .unwrap();
        let claimed = store
            .claim_execution_lease(&reserved.binding.binding_id, 1, "token-a", 2, 100)
            .unwrap();
        assert_eq!(claimed.lease_token.as_deref(), Some("token-a"));
        let err = store
            .claim_execution_lease(&claimed.binding_id, claimed.revision, "token-b", 50, 100)
            .unwrap_err();
        assert_eq!(err.to_string(), "execution_lease_conflict");
        let renewed = store
            .renew_execution_lease(&claimed.binding_id, claimed.revision, "token-a", 50, 100)
            .unwrap();
        let taken = store
            .claim_execution_lease(&renewed.binding_id, renewed.revision, "token-b", 200, 100)
            .unwrap();
        assert_eq!(taken.lease_token.as_deref(), Some("token-b"));
        assert!(
            store
                .release_execution_lease(&taken.binding_id, taken.revision, "token-b", 201)
                .is_ok()
        );
    }

    #[test]
    fn task_messages_are_ordered_idempotent_and_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let reserved = store
            .reserve_execution(ExecutionReservation {
                workspace_id: "workspace-a".into(),
                issue_id: "issue-a".into(),
                agent_id: None,
                execution_kind: MulticaExecutionKind::Thread,
                parent_thread_id: None,
                parent_attempt_id: None,
                idempotency_key: "msg-a".into(),
                now_ms: 1,
            })
            .unwrap();
        let make = |seq: u32, summary: &str| CodexMulticaTaskMessage {
            message_id: format!("message-{seq}"),
            binding_id: reserved.binding.binding_id.clone(),
            seq,
            message_type: "assistant".into(),
            tool: None,
            summary: Some(summary.into()),
            created_at_ms: seq as u64,
        };
        let second = make(2, "second");
        store.append_task_message(second.clone()).unwrap();
        store.append_task_message(make(1, "first")).unwrap();
        assert_eq!(store.append_task_message(second.clone()).unwrap(), second);
        let conflict = store.append_task_message(make(2, "changed")).unwrap_err();
        assert_eq!(conflict.to_string(), "task_message_conflict");
        let messages = store
            .list_task_messages(&reserved.binding.binding_id)
            .unwrap();
        assert_eq!(
            messages.iter().map(|m| m.seq).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn autopilot_run_is_persisted_and_source_guarded() {
        let dir = tempfile::tempdir().unwrap();
        let store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let run = store
            .trigger_autopilot_run("auto-1".into(), None, "manual".into(), 10)
            .unwrap();
        assert_eq!(run.status, "pending");
        assert_eq!(
            store.list_autopilot_runs("auto-1").unwrap(),
            vec![run.clone()]
        );
        assert_eq!(store.get_autopilot_run(&run.id).unwrap(), run);
        assert_eq!(
            store
                .trigger_autopilot_run("auto-1".into(), None, "bogus".into(), 11)
                .unwrap_err()
                .to_string(),
            "autopilot_run_source_invalid"
        );
    }
}
