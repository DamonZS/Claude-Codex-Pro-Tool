//! Local Multica workspace projections for the Codex renderer bridge.
//!
//! The workspace is an embedded control plane. It reads CCP-owned local state
//! and, when supplied by the caller, projects capabilities from the current
//! Codex page host. It never reads a managed Multica profile, starts a daemon,
//! registers a runtime, or falls back to a CLI/app-server transport.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{anyhow, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::codex_execution::{CodexExecutionService, CodexRuntimeCapabilities, CodexSkill};
use crate::multica_execution::{
    SkillBindingScope, SkillBindingSelection, SkillInventoryEntry, SkillReference,
    SkillResolutionRequest, resolve_skill_bindings as resolve_skill_bindings_policy,
};
use crate::multica_execution_store::{MulticaExecutionStore, SkillBindingUpsert};
use crate::multica_skill_trust::{
    LocalSkillTrustEntry, TRUST_STATE_REVIEW_REQUIRED, read_local_skill_trust_snapshot,
};
use crate::settings::SettingsStore;
use crate::unified_tool_inventory::UnifiedToolInventoryRoots;

const MAX_COLLECTION_ITEMS: usize = 100;
const DEFAULT_COLLECTION_LIMIT: u16 = 50;
const LOCAL_CONTROL_PLANE_EMPTY: &str = "local_control_plane_empty";
const CODEX_PAGE_HOST_UNAVAILABLE: &str = "codex_page_host_unavailable";
const MULTICA_WORKSPACE_DISABLED: &str = "multica_workspace_disabled";
const LOCAL_WORKSPACE_STORE_VERSION: u32 = 1;
const MAX_LOCAL_ENTITIES_PER_RESOURCE: usize = 2_048;
const MAX_LOCAL_ENTITY_BYTES: usize = 128 * 1024;
const MAX_LOCAL_WORKSPACE_STORE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MulticaWorkspaceResourceKey {
    MyTasks,
    Issues,
    Projects,
    Autopilots,
    Agents,
    Squads,
    Statistics,
    Runtimes,
    Skills,
    Settings,
}

impl MulticaWorkspaceResourceKey {
    pub const ALL: [Self; 10] = [
        Self::MyTasks,
        Self::Issues,
        Self::Projects,
        Self::Autopilots,
        Self::Agents,
        Self::Squads,
        Self::Statistics,
        Self::Runtimes,
        Self::Skills,
        Self::Settings,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::MyTasks => "my_tasks",
            Self::Issues => "issues",
            Self::Projects => "projects",
            Self::Autopilots => "autopilots",
            Self::Agents => "agents",
            Self::Squads => "squads",
            Self::Statistics => "statistics",
            Self::Runtimes => "runtimes",
            Self::Skills => "skills",
            Self::Settings => "settings",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaWorkspaceQuery {
    pub resource: MulticaWorkspaceResourceKey,
    #[serde(default = "default_collection_limit")]
    pub limit: u16,
    #[serde(default)]
    pub offset: u32,
}

impl MulticaWorkspaceQuery {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.limit == 0 || usize::from(self.limit) > MAX_COLLECTION_ITEMS {
            bail!("multica_workspace_limit_invalid");
        }
        if self.offset > 100_000 {
            bail!("multica_workspace_offset_invalid");
        }
        Ok(())
    }
}

fn default_collection_limit() -> u16 {
    DEFAULT_COLLECTION_LIMIT
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MulticaWorkspaceIdentity {
    pub id: String,
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MulticaWorkspaceCollection {
    pub workspace_id: String,
    pub resource: MulticaWorkspaceResourceKey,
    pub items: Vec<Value>,
    pub total: u64,
    pub limit: u16,
    pub offset: u32,
    pub fetched_at_ms: u64,
    #[serde(default)]
    pub stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MulticaCodexRuntimeSummary {
    pub available: bool,
    pub runtime_id: Option<String>,
    pub provider: Option<String>,
    pub status: Option<String>,
    pub capabilities: Vec<String>,
    pub skills_supported: bool,
    pub skill_protocol: Option<String>,
    pub multi_agent_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MulticaWorkspaceBootstrap {
    pub status: String,
    pub fetched_at_ms: u64,
    pub workspace: MulticaWorkspaceIdentity,
    pub user: Value,
    pub runtime: MulticaCodexRuntimeSummary,
    pub modules: Vec<String>,
    pub collections: BTreeMap<String, MulticaWorkspaceCollection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MulticaSkillBindingCommand {
    pub scope_kind: SkillBindingScope,
    pub scope_id: String,
    pub skill_ref: SkillReference,
    pub enabled: bool,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MulticaSkillBindingRemoveCommand {
    pub scope_kind: SkillBindingScope,
    pub scope_id: String,
    pub skill_id: String,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MulticaSkillBindingsQuery {
    pub scope_kind: Option<SkillBindingScope>,
    pub scope_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalWorkspaceEntityUpsert {
    pub resource: MulticaWorkspaceResourceKey,
    pub entity: Value,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWorkspaceEntityDelete {
    pub resource: MulticaWorkspaceResourceKey,
    pub entity_id: String,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalMulticaWorkspaceState {
    pub version: u32,
    pub workspace_id: String,
    #[serde(default)]
    pub issues: Vec<Value>,
    #[serde(default)]
    pub projects: Vec<Value>,
    #[serde(default)]
    pub agents: Vec<Value>,
    #[serde(default)]
    pub squads: Vec<Value>,
    #[serde(default)]
    pub autopilots: Vec<Value>,
}

impl LocalMulticaWorkspaceState {
    fn empty(workspace_id: &str) -> Self {
        Self {
            version: LOCAL_WORKSPACE_STORE_VERSION,
            workspace_id: workspace_id.to_string(),
            issues: Vec::new(),
            projects: Vec::new(),
            agents: Vec::new(),
            squads: Vec::new(),
            autopilots: Vec::new(),
        }
    }

    fn collection(&self, resource: MulticaWorkspaceResourceKey) -> anyhow::Result<&Vec<Value>> {
        match resource {
            MulticaWorkspaceResourceKey::Issues | MulticaWorkspaceResourceKey::MyTasks => {
                Ok(&self.issues)
            }
            MulticaWorkspaceResourceKey::Projects => Ok(&self.projects),
            MulticaWorkspaceResourceKey::Agents => Ok(&self.agents),
            MulticaWorkspaceResourceKey::Squads => Ok(&self.squads),
            MulticaWorkspaceResourceKey::Autopilots => Ok(&self.autopilots),
            _ => bail!("multica_workspace_resource_not_persisted"),
        }
    }

    fn collection_mut(
        &mut self,
        resource: MulticaWorkspaceResourceKey,
    ) -> anyhow::Result<&mut Vec<Value>> {
        match resource {
            MulticaWorkspaceResourceKey::Issues => Ok(&mut self.issues),
            MulticaWorkspaceResourceKey::Projects => Ok(&mut self.projects),
            MulticaWorkspaceResourceKey::Agents => Ok(&mut self.agents),
            MulticaWorkspaceResourceKey::Squads => Ok(&mut self.squads),
            MulticaWorkspaceResourceKey::Autopilots => Ok(&mut self.autopilots),
            _ => bail!("multica_workspace_resource_not_persisted"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMulticaWorkspaceStore {
    path: PathBuf,
}

impl Default for LocalMulticaWorkspaceStore {
    fn default() -> Self {
        Self::new(crate::paths::default_multica_state_dir().join("workspace.json"))
    }
}

impl LocalMulticaWorkspaceStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self, workspace_id: &str) -> anyhow::Result<LocalMulticaWorkspaceState> {
        load_local_workspace_state(&self.path, workspace_id)
    }

    pub fn save(&self, state: &LocalMulticaWorkspaceState) -> anyhow::Result<()> {
        validate_local_workspace_state(state)?;
        let _guard = local_workspace_store_lock(&self.path)?;
        save_local_workspace_state_locked(&self.path, state)
    }

    pub fn list(
        &self,
        workspace_id: &str,
        resource: MulticaWorkspaceResourceKey,
    ) -> anyhow::Result<Vec<Value>> {
        Ok(self.load(workspace_id)?.collection(resource)?.clone())
    }

    pub fn upsert(
        &self,
        workspace_id: &str,
        command: LocalWorkspaceEntityUpsert,
        updated_at_ms: u64,
    ) -> anyhow::Result<Value> {
        validate_local_workspace_id(workspace_id)?;
        let mut entity = command
            .entity
            .as_object()
            .cloned()
            .ok_or_else(|| anyhow!("multica_workspace_entity_invalid"))?;
        let entity_id = entity
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("multica_workspace_entity_invalid"))?
            .to_string();
        validate_local_entity_id(&entity_id)?;
        entity.remove("workspaceId");
        let _guard = local_workspace_store_lock(&self.path)?;
        let mut state = load_local_workspace_state(&self.path, workspace_id)?;
        let entities = state.collection_mut(command.resource)?;
        let existing_index = entities.iter().position(|candidate| {
            candidate.get("id").and_then(Value::as_str) == Some(entity_id.as_str())
        });

        let revision = if let Some(index) = existing_index {
            let current_revision = entities[index]
                .get("revision")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow!("multica_workspace_store_invalid"))?;
            if command.expected_revision != Some(current_revision) {
                bail!("multica_workspace_revision_conflict");
            }
            current_revision.saturating_add(1)
        } else {
            if command
                .expected_revision
                .is_some_and(|revision| revision != 0)
            {
                bail!("multica_workspace_revision_conflict");
            }
            if entities.len() >= MAX_LOCAL_ENTITIES_PER_RESOURCE {
                bail!("multica_workspace_collection_too_large");
            }
            1
        };

        entity.insert("workspace_id".to_string(), json!(workspace_id));
        entity.insert("revision".to_string(), json!(revision));
        entity.insert("updated_at_ms".to_string(), json!(updated_at_ms));
        if existing_index.is_none() {
            entity.insert("created_at_ms".to_string(), json!(updated_at_ms));
        } else if let Some(created_at_ms) = existing_index
            .and_then(|index| entities[index].get("created_at_ms"))
            .cloned()
        {
            entity.insert("created_at_ms".to_string(), created_at_ms);
        }
        let value = Value::Object(entity);
        validate_local_entity(&value, workspace_id)?;
        if let Some(index) = existing_index {
            entities[index] = value.clone();
        } else {
            entities.push(value.clone());
        }
        validate_local_workspace_state(&state)?;
        save_local_workspace_state_locked(&self.path, &state)?;
        Ok(value)
    }

    pub fn delete(
        &self,
        workspace_id: &str,
        command: LocalWorkspaceEntityDelete,
    ) -> anyhow::Result<bool> {
        validate_local_workspace_id(workspace_id)?;
        validate_local_entity_id(&command.entity_id)?;
        let _guard = local_workspace_store_lock(&self.path)?;
        let mut state = load_local_workspace_state(&self.path, workspace_id)?;
        let entities = state.collection_mut(command.resource)?;
        let Some(index) = entities.iter().position(|candidate| {
            candidate.get("id").and_then(Value::as_str) == Some(command.entity_id.as_str())
        }) else {
            return Ok(false);
        };
        let current_revision = entities[index]
            .get("revision")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("multica_workspace_store_invalid"))?;
        if current_revision != command.expected_revision {
            bail!("multica_workspace_revision_conflict");
        }
        entities.remove(index);
        save_local_workspace_state_locked(&self.path, &state)?;
        Ok(true)
    }
}

/// Build the local control-plane snapshot without an execution host.
pub async fn workspace_bootstrap() -> anyhow::Result<MulticaWorkspaceBootstrap> {
    local_workspace_bootstrap(None).await
}

/// Build the local snapshot and project the current Codex page host. The
/// service is supplied by the page bridge; no runtime transport is discovered
/// or registered here.
pub async fn workspace_bootstrap_with_codex_runtime(
    runtime: Arc<dyn CodexExecutionService>,
) -> anyhow::Result<MulticaWorkspaceBootstrap> {
    local_workspace_bootstrap(Some(runtime)).await
}

async fn local_workspace_bootstrap(
    runtime: Option<Arc<dyn CodexExecutionService>>,
) -> anyhow::Result<MulticaWorkspaceBootstrap> {
    let workspace = local_workspace_identity();
    let enabled = local_workspace_enabled()?;
    let execution_store = MulticaExecutionStore::default();
    let workspace_store = LocalMulticaWorkspaceStore::default();
    let mut collections = BTreeMap::new();

    for resource in MulticaWorkspaceResourceKey::ALL {
        let query = MulticaWorkspaceQuery {
            resource,
            limit: DEFAULT_COLLECTION_LIMIT,
            offset: 0,
        };
        let value = match (enabled, resource, runtime.as_ref()) {
            (false, _, _) => query_local_collection(
                &workspace,
                &execution_store,
                &workspace_store,
                enabled,
                query,
            ),
            (true, MulticaWorkspaceResourceKey::Skills, Some(service))
            | (true, MulticaWorkspaceResourceKey::Runtimes, Some(service)) => {
                query_host_collection(&workspace, query, Arc::clone(service)).await
            }
            _ => query_local_collection(
                &workspace,
                &execution_store,
                &workspace_store,
                enabled,
                query,
            ),
        }
        .unwrap_or_else(|error| {
            unavailable_collection(
                &workspace,
                resource,
                DEFAULT_COLLECTION_LIMIT,
                0,
                diagnostic_code(&error),
            )
        });
        collections.insert(resource.key().to_string(), value);
    }

    let runtime_summary = match (enabled, runtime.as_ref()) {
        (false, _) => unavailable_runtime_summary(MULTICA_WORKSPACE_DISABLED),
        (true, Some(service)) => match service.capabilities().await {
            Ok(capabilities) => runtime_summary_from_capabilities(&capabilities),
            Err(error) => unavailable_runtime_summary(diagnostic_code(&error)),
        },
        (true, None) => unavailable_runtime_summary(CODEX_PAGE_HOST_UNAVAILABLE),
    };

    Ok(MulticaWorkspaceBootstrap {
        status: if runtime_summary.available {
            "ok"
        } else {
            "degraded"
        }
        .to_string(),
        fetched_at_ms: now_ms(),
        workspace: workspace.clone(),
        user: local_user_projection(&workspace),
        runtime: runtime_summary,
        modules: MulticaWorkspaceResourceKey::ALL
            .into_iter()
            .map(|resource| resource.key().to_string())
            .collect(),
        collections,
    })
}

/// Query CCP-owned local state only. No managed profile, server, daemon, or
/// runtime registry is consulted by this compatibility entry point.
pub async fn workspace_query(
    query: MulticaWorkspaceQuery,
) -> anyhow::Result<MulticaWorkspaceCollection> {
    query.validate()?;
    let workspace = local_workspace_identity();
    query_local_collection(
        &workspace,
        &MulticaExecutionStore::default(),
        &LocalMulticaWorkspaceStore::default(),
        local_workspace_enabled()?,
        query,
    )
}

/// Query page-authoritative modules from the supplied Codex page host. All
/// other modules continue to use the embedded local control plane.
pub async fn workspace_query_with_codex_runtime(
    query: MulticaWorkspaceQuery,
    runtime: Arc<dyn CodexExecutionService>,
) -> anyhow::Result<MulticaWorkspaceCollection> {
    query.validate()?;
    let workspace = local_workspace_identity();
    let enabled = local_workspace_enabled()?;
    if !enabled {
        return query_local_collection(
            &workspace,
            &MulticaExecutionStore::default(),
            &LocalMulticaWorkspaceStore::default(),
            false,
            query,
        );
    }
    if matches!(
        query.resource,
        MulticaWorkspaceResourceKey::Skills | MulticaWorkspaceResourceKey::Runtimes
    ) {
        return query_host_collection(&workspace, query, runtime).await;
    }
    query_local_collection(
        &workspace,
        &MulticaExecutionStore::default(),
        &LocalMulticaWorkspaceStore::default(),
        enabled,
        query,
    )
}

async fn query_host_collection(
    workspace: &MulticaWorkspaceIdentity,
    query: MulticaWorkspaceQuery,
    runtime: Arc<dyn CodexExecutionService>,
) -> anyhow::Result<MulticaWorkspaceCollection> {
    match query.resource {
        MulticaWorkspaceResourceKey::Skills => {
            query_codex_skills(workspace, query.limit, query.offset, runtime).await
        }
        MulticaWorkspaceResourceKey::Runtimes => {
            let capabilities = match runtime.capabilities().await {
                Ok(capabilities) => capabilities,
                Err(error) => {
                    let code = diagnostic_code(&error);
                    return Ok(runtime_collection(
                        workspace,
                        &unavailable_runtime_summary(code),
                        Some(code),
                    ));
                }
            };
            Ok(runtime_collection(
                workspace,
                &runtime_summary_from_capabilities(&capabilities),
                None,
            ))
        }
        _ => bail!("multica_workspace_resource_invalid"),
    }
}

async fn query_codex_skills(
    workspace: &MulticaWorkspaceIdentity,
    limit: u16,
    offset: u32,
    runtime: Arc<dyn CodexExecutionService>,
) -> anyhow::Result<MulticaWorkspaceCollection> {
    let capabilities = match runtime.capabilities().await {
        Ok(capabilities) => capabilities,
        Err(error) => {
            return Ok(unavailable_collection(
                workspace,
                MulticaWorkspaceResourceKey::Skills,
                limit,
                offset,
                diagnostic_code(&error),
            ));
        }
    };
    if !capabilities.skills_supported {
        return Ok(unavailable_collection(
            workspace,
            MulticaWorkspaceResourceKey::Skills,
            limit,
            offset,
            "runtime_skills_unsupported",
        ));
    }
    let skills = match runtime.list_skills().await {
        Ok(skills) => skills,
        Err(error) => {
            return Ok(unavailable_collection(
                workspace,
                MulticaWorkspaceResourceKey::Skills,
                limit,
                offset,
                diagnostic_code(&error),
            ));
        }
    };
    let trust = read_local_skill_trust_snapshot(&UnifiedToolInventoryRoots::default());
    let all_items = skills
        .iter()
        .map(|skill| runtime_codex_skill_item(skill, &capabilities.runtime_id, &trust))
        .collect::<Vec<_>>();
    let items = paginate(&all_items, limit, offset);
    Ok(collection(
        workspace,
        MulticaWorkspaceResourceKey::Skills,
        items,
        all_items.len() as u64,
        limit,
        offset,
    ))
}

/// Resolution requires the current page host inventory. The host-less API is
/// retained for compatibility but intentionally fails closed.
pub async fn resolve_skill_bindings(_selection: SkillBindingSelection) -> anyhow::Result<Value> {
    bail!(CODEX_PAGE_HOST_UNAVAILABLE)
}

pub async fn resolve_skill_bindings_with_codex_runtime(
    selection: SkillBindingSelection,
    runtime: Arc<dyn CodexExecutionService>,
) -> anyhow::Result<Value> {
    selection.validate()?;
    let capabilities = runtime.capabilities().await?;
    if !capabilities.skills_supported {
        bail!("runtime_skills_unsupported");
    }
    let skills = runtime.list_skills().await?;
    let trust = read_local_skill_trust_snapshot(&UnifiedToolInventoryRoots::default());
    let inventory = skills
        .iter()
        .map(|skill| {
            let local = trust.get(&skill.id);
            let digest_matches = skill
                .manifest_digest
                .as_deref()
                .zip(local.and_then(|entry| entry.manifest_digest.as_deref()))
                .is_some_and(|(runtime_digest, local_digest)| runtime_digest == local_digest);
            SkillInventoryEntry {
                id: skill.id.clone(),
                name: skill.name.clone(),
                version: None,
                summary: skill.summary.clone(),
                installed: skill.enabled,
                trusted: skill.enabled
                    && digest_matches
                    && local.is_some_and(LocalSkillTrustEntry::dispatch_allowed),
                compatible: skill.enabled && skill.manifest_digest.is_some(),
                manifest_digest: skill.manifest_digest.clone(),
            }
        })
        .collect::<Vec<_>>();
    let request = SkillResolutionRequest {
        bindings: selection.bindings,
        runtime_capabilities: capabilities.capabilities,
        inventory,
    };
    let audit = resolve_skill_bindings_policy(&request)?;
    Ok(json!({
        "status": "ok",
        "runtime_id": capabilities.runtime_id,
        "audit": audit,
    }))
}

/// Persist a local Skill binding after an explicit CCP trust decision. The
/// binding belongs to the stable embedded workspace, never a remote tenant.
pub async fn upsert_skill_binding(command: MulticaSkillBindingCommand) -> anyhow::Result<Value> {
    validate_binding_scope_id(&command.scope_id)?;
    let workspace_id = local_workspace_id();
    let trust_snapshot = read_local_skill_trust_snapshot(&UnifiedToolInventoryRoots::default());
    let trust = trust_snapshot
        .get(&command.skill_ref.id)
        .ok_or_else(|| anyhow!("skill_unknown"))?;
    if !trust.dispatch_allowed() {
        bail!("skill_not_trusted");
    }
    let manifest_digest = trust
        .manifest_digest
        .clone()
        .ok_or_else(|| anyhow!("skill_manifest_unavailable"))?;
    if let Some(expected) = command.skill_ref.manifest_digest.as_deref()
        && expected != manifest_digest
    {
        bail!("skill_manifest_conflict");
    }
    let skill_ref = SkillReference {
        id: command.skill_ref.id,
        manifest_digest: Some(manifest_digest),
    };
    let binding_id = binding_id(
        &workspace_id,
        command.scope_kind,
        &command.scope_id,
        &skill_ref.id,
    );
    let binding = MulticaExecutionStore::default().upsert_binding(SkillBindingUpsert {
        binding_id,
        workspace_id,
        scope_kind: command.scope_kind,
        scope_id: command.scope_id,
        skill_ref,
        source_kind: trust.source_kind.clone(),
        trust_state: trust.trust_state.clone(),
        enabled: command.enabled,
        expected_revision: command.expected_revision,
        now_ms: now_ms(),
    })?;
    Ok(json!({"status": "ok", "binding": binding}))
}

pub async fn upsert_skill_binding_with_codex_runtime(
    command: MulticaSkillBindingCommand,
    runtime: Arc<dyn CodexExecutionService>,
) -> anyhow::Result<Value> {
    let capabilities = runtime.capabilities().await?;
    if !capabilities.skills_supported {
        bail!("runtime_skills_unsupported");
    }
    let skills = runtime.list_skills().await?;
    let skill = skills
        .iter()
        .find(|skill| skill.id == command.skill_ref.id)
        .ok_or_else(|| anyhow!("skill_unknown"))?;
    if !skill.enabled {
        bail!("skill_not_installed");
    }
    if let Some(expected) = command.skill_ref.manifest_digest.as_deref()
        && skill.manifest_digest.as_deref() != Some(expected)
    {
        bail!("skill_manifest_conflict");
    }
    upsert_skill_binding(command).await
}

pub async fn remove_skill_binding(
    command: MulticaSkillBindingRemoveCommand,
) -> anyhow::Result<Value> {
    validate_binding_scope_id(&command.scope_id)?;
    let removed = MulticaExecutionStore::default().remove_binding(
        &local_workspace_id(),
        command.scope_kind,
        &command.scope_id,
        &command.skill_id,
        command.expected_revision,
    )?;
    Ok(json!({"status": "ok", "removed": removed}))
}

pub async fn list_skill_bindings(query: MulticaSkillBindingsQuery) -> anyhow::Result<Value> {
    if let Some(scope_id) = query.scope_id.as_deref() {
        validate_binding_scope_id(scope_id)?;
    }
    let workspace_id = local_workspace_id();
    let bindings = MulticaExecutionStore::default().list_bindings(
        &workspace_id,
        query.scope_kind,
        query.scope_id.as_deref(),
    )?;
    Ok(json!({"status": "ok", "workspaceId": workspace_id, "bindings": bindings}))
}

fn query_local_collection(
    workspace: &MulticaWorkspaceIdentity,
    execution_store: &MulticaExecutionStore,
    workspace_store: &LocalMulticaWorkspaceStore,
    enabled: bool,
    query: MulticaWorkspaceQuery,
) -> anyhow::Result<MulticaWorkspaceCollection> {
    if !enabled && query.resource != MulticaWorkspaceResourceKey::Settings {
        return Ok(unavailable_collection(
            workspace,
            query.resource,
            query.limit,
            query.offset,
            MULTICA_WORKSPACE_DISABLED,
        ));
    }
    match query.resource {
        MulticaWorkspaceResourceKey::Settings => Ok(settings_collection(workspace, enabled)),
        MulticaWorkspaceResourceKey::Statistics => {
            statistics_collection(workspace, execution_store, workspace_store)
        }
        MulticaWorkspaceResourceKey::Runtimes => Ok(runtime_collection(
            workspace,
            &unavailable_runtime_summary(CODEX_PAGE_HOST_UNAVAILABLE),
            Some(CODEX_PAGE_HOST_UNAVAILABLE),
        )),
        MulticaWorkspaceResourceKey::Skills => Ok(unavailable_collection(
            workspace,
            query.resource,
            query.limit,
            query.offset,
            CODEX_PAGE_HOST_UNAVAILABLE,
        )),
        MulticaWorkspaceResourceKey::MyTasks
        | MulticaWorkspaceResourceKey::Issues
        | MulticaWorkspaceResourceKey::Projects
        | MulticaWorkspaceResourceKey::Autopilots
        | MulticaWorkspaceResourceKey::Agents
        | MulticaWorkspaceResourceKey::Squads => local_entity_collection(
            workspace,
            workspace_store,
            query.resource,
            query.limit,
            query.offset,
        ),
    }
}

fn local_entity_collection(
    workspace: &MulticaWorkspaceIdentity,
    store: &LocalMulticaWorkspaceStore,
    resource: MulticaWorkspaceResourceKey,
    limit: u16,
    offset: u32,
) -> anyhow::Result<MulticaWorkspaceCollection> {
    let mut all_items = store.list(&workspace.id, resource)?;
    if resource == MulticaWorkspaceResourceKey::MyTasks {
        let user_id = local_user_id(workspace);
        all_items.retain(|entity| {
            entity
                .get("assignee_id")
                .or_else(|| entity.get("assigneeId"))
                .and_then(Value::as_str)
                == Some(user_id.as_str())
        });
    }
    let items = paginate(&all_items, limit, offset);
    let mut value = collection(
        workspace,
        resource,
        items,
        all_items.len() as u64,
        limit,
        offset,
    );
    if all_items.is_empty() {
        value.diagnostic = Some(LOCAL_CONTROL_PLANE_EMPTY.to_string());
    }
    Ok(value)
}

fn statistics_collection(
    workspace: &MulticaWorkspaceIdentity,
    execution_store: &MulticaExecutionStore,
    workspace_store: &LocalMulticaWorkspaceStore,
) -> anyhow::Result<MulticaWorkspaceCollection> {
    let execution_state = execution_store.load()?;
    let workspace_state = workspace_store.load(&workspace.id)?;
    let enabled_bindings = execution_state
        .skill_bindings
        .iter()
        .filter(|binding| binding.workspace_id == workspace.id && binding.enabled)
        .count();
    let binding_total = execution_state
        .skill_bindings
        .iter()
        .filter(|binding| binding.workspace_id == workspace.id)
        .count();
    let loaded_attempts = execution_state
        .attempt_skill_snapshots
        .iter()
        .filter(|snapshot| snapshot.loaded_at_ms.is_some())
        .count();
    let mut statuses = BTreeMap::<String, u64>::new();
    for snapshot in &execution_state.attempt_skill_snapshots {
        *statuses
            .entry(snapshot.resolution_status.clone())
            .or_default() += 1;
    }
    let attempt_total = execution_state.attempt_skill_snapshots.len();
    let mut value = collection(
        workspace,
        MulticaWorkspaceResourceKey::Statistics,
        vec![json!({
            "skill_binding_total": binding_total,
            "enabled_skill_bindings": enabled_bindings,
            "attempt_total": attempt_total,
            "loaded_attempts": loaded_attempts,
            "resolution_statuses": statuses,
            "issue_total": workspace_state.issues.len(),
            "project_total": workspace_state.projects.len(),
            "agent_total": workspace_state.agents.len(),
            "squad_total": workspace_state.squads.len(),
            "autopilot_total": workspace_state.autopilots.len(),
        })],
        1,
        1,
        0,
    );
    if binding_total == 0
        && attempt_total == 0
        && workspace_state.issues.is_empty()
        && workspace_state.projects.is_empty()
        && workspace_state.agents.is_empty()
        && workspace_state.squads.is_empty()
        && workspace_state.autopilots.is_empty()
    {
        value.diagnostic = Some(LOCAL_CONTROL_PLANE_EMPTY.to_string());
    }
    Ok(value)
}

fn settings_collection(
    workspace: &MulticaWorkspaceIdentity,
    enabled: bool,
) -> MulticaWorkspaceCollection {
    collection(
        workspace,
        MulticaWorkspaceResourceKey::Settings,
        vec![json!({
            "workspace_id": workspace.id,
            "enabled": enabled,
            "mode": "local",
            "managed": false,
            "control_plane": "embedded",
            "execution_source": "codex_page_host",
        })],
        1,
        1,
        0,
    )
}

fn runtime_collection(
    workspace: &MulticaWorkspaceIdentity,
    runtime: &MulticaCodexRuntimeSummary,
    diagnostic: Option<&str>,
) -> MulticaWorkspaceCollection {
    let mut items = vec![json!({
        "id": "local-control-plane",
        "workspace_id": workspace.id,
        "kind": "control_plane",
        "provider": "multica",
        "status": "ready",
        "managed": false,
        "process_required": false,
    })];
    if runtime.available {
        items.push(json!({
            "id": runtime.runtime_id,
            "workspace_id": workspace.id,
            "kind": "codex_page_host",
            "provider": runtime.provider,
            "status": runtime.status,
            "capabilities": runtime.capabilities,
            "skills_supported": runtime.skills_supported,
            "skill_protocol": runtime.skill_protocol,
            "multi_agent_supported": runtime.multi_agent_supported,
            "registered": false,
        }));
    }
    let total = items.len() as u64;
    let mut value = collection(
        workspace,
        MulticaWorkspaceResourceKey::Runtimes,
        items,
        total,
        DEFAULT_COLLECTION_LIMIT,
        0,
    );
    value.diagnostic = diagnostic.map(str::to_string);
    value.stale = diagnostic.is_some();
    value
}

fn runtime_summary_from_capabilities(
    capabilities: &CodexRuntimeCapabilities,
) -> MulticaCodexRuntimeSummary {
    MulticaCodexRuntimeSummary {
        available: true,
        runtime_id: Some(capabilities.runtime_id.clone()),
        provider: Some(capabilities.provider.clone()),
        status: Some("available".to_string()),
        capabilities: capabilities.capabilities.clone(),
        skills_supported: capabilities.skills_supported,
        skill_protocol: capabilities.skill_protocol.clone(),
        multi_agent_supported: capabilities.subagents_supported,
    }
}

fn unavailable_runtime_summary(diagnostic: &str) -> MulticaCodexRuntimeSummary {
    MulticaCodexRuntimeSummary {
        available: false,
        runtime_id: None,
        provider: Some("codex".to_string()),
        status: Some(diagnostic.to_string()),
        capabilities: Vec::new(),
        skills_supported: false,
        skill_protocol: None,
        multi_agent_supported: false,
    }
}

fn runtime_codex_skill_item(
    skill: &CodexSkill,
    runtime_id: &str,
    trust: &crate::multica_skill_trust::LocalSkillTrustSnapshot,
) -> Value {
    let local = trust.get(&skill.id);
    let digest_matches = skill
        .manifest_digest
        .as_deref()
        .zip(local.and_then(|entry| entry.manifest_digest.as_deref()))
        .is_some_and(|(runtime_digest, local_digest)| runtime_digest == local_digest);
    let dispatch_allowed = skill.enabled
        && digest_matches
        && local.is_some_and(LocalSkillTrustEntry::dispatch_allowed);
    json!({
        "id": skill.id,
        "runtime_id": runtime_id,
        "name": skill.name,
        "description": skill.summary,
        "provider": "codex",
        "inventory_source": "codex_page_host",
        "installed": skill.enabled,
        "compatible": skill.enabled && skill.manifest_digest.is_some(),
        "trust_state": local.map(|entry| entry.trust_state.as_str()).unwrap_or(TRUST_STATE_REVIEW_REQUIRED),
        "dispatch_allowed": dispatch_allowed,
        "trust_source": local.map(|entry| entry.source_kind.as_str()).unwrap_or("none"),
        "manifest_digest": skill.manifest_digest,
        "runtime_loaded": "unknown",
        "binding_scope": ["task", "agent"],
    })
}

fn validate_binding_scope_id(value: &str) -> anyhow::Result<()> {
    if value.is_empty()
        || value.len() > 240
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'@')
        })
    {
        bail!("skill_binding_scope_invalid");
    }
    Ok(())
}

fn binding_id(
    workspace_id: &str,
    scope_kind: SkillBindingScope,
    scope_id: &str,
    skill_id: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(workspace_id.as_bytes());
    hasher.update([0]);
    hasher.update(scope_kind.as_str().as_bytes());
    hasher.update([0]);
    hasher.update(scope_id.as_bytes());
    hasher.update([0]);
    hasher.update(skill_id.as_bytes());
    let digest = hasher.finalize();
    format!(
        "binding-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7]
    )
}

fn load_local_workspace_state(
    path: &Path,
    workspace_id: &str,
) -> anyhow::Result<LocalMulticaWorkspaceState> {
    validate_local_workspace_id(workspace_id)?;
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LocalMulticaWorkspaceState::empty(workspace_id));
        }
        Err(_) => bail!("multica_workspace_store_read_failed"),
    };
    if bytes.len() > MAX_LOCAL_WORKSPACE_STORE_BYTES {
        bail!("multica_workspace_store_too_large");
    }
    let state: LocalMulticaWorkspaceState =
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("multica_workspace_store_invalid"))?;
    validate_local_workspace_state(&state)?;
    if state.workspace_id != workspace_id {
        bail!("multica_workspace_tenant_mismatch");
    }
    Ok(state)
}

fn save_local_workspace_state_locked(
    path: &Path,
    state: &LocalMulticaWorkspaceState,
) -> anyhow::Result<()> {
    validate_local_workspace_state(state)?;
    let bytes =
        serde_json::to_vec_pretty(state).map_err(|_| anyhow!("multica_workspace_store_invalid"))?;
    if bytes.len() > MAX_LOCAL_WORKSPACE_STORE_BYTES {
        bail!("multica_workspace_store_too_large");
    }
    crate::settings::atomic_write(path, &bytes)
        .map_err(|_| anyhow!("multica_workspace_store_write_failed"))
}

fn validate_local_workspace_state(state: &LocalMulticaWorkspaceState) -> anyhow::Result<()> {
    if state.version != LOCAL_WORKSPACE_STORE_VERSION {
        bail!("multica_workspace_store_invalid");
    }
    validate_local_workspace_id(&state.workspace_id)?;
    for entities in [
        &state.issues,
        &state.projects,
        &state.agents,
        &state.squads,
        &state.autopilots,
    ] {
        if entities.len() > MAX_LOCAL_ENTITIES_PER_RESOURCE {
            bail!("multica_workspace_collection_too_large");
        }
        let mut ids = BTreeSet::new();
        for entity in entities {
            validate_local_entity(entity, &state.workspace_id)?;
            let id = entity
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("multica_workspace_entity_invalid"))?;
            if !ids.insert(id.to_ascii_lowercase()) {
                bail!("multica_workspace_entity_conflict");
            }
        }
    }
    Ok(())
}

fn validate_local_workspace_id(value: &str) -> anyhow::Result<()> {
    if !value.starts_with("local-") {
        bail!("multica_workspace_id_invalid");
    }
    validate_local_entity_id(value).map_err(|_| anyhow!("multica_workspace_id_invalid"))
}

fn validate_local_entity_id(value: &str) -> anyhow::Result<()> {
    if value.is_empty()
        || value.len() > 240
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'@')
        })
    {
        bail!("multica_workspace_entity_id_invalid");
    }
    Ok(())
}

fn validate_local_entity(entity: &Value, workspace_id: &str) -> anyhow::Result<()> {
    let encoded =
        serde_json::to_vec(entity).map_err(|_| anyhow!("multica_workspace_entity_invalid"))?;
    if encoded.len() > MAX_LOCAL_ENTITY_BYTES {
        bail!("multica_workspace_entity_too_large");
    }
    let object = entity
        .as_object()
        .ok_or_else(|| anyhow!("multica_workspace_entity_invalid"))?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("multica_workspace_entity_invalid"))?;
    validate_local_entity_id(id)?;
    if object.get("workspace_id").and_then(Value::as_str) != Some(workspace_id) {
        bail!("multica_workspace_tenant_mismatch");
    }
    if object.get("revision").and_then(Value::as_u64) == Some(0)
        || object.get("revision").and_then(Value::as_u64).is_none()
    {
        bail!("multica_workspace_entity_invalid");
    }
    if contains_sensitive_workspace_field(entity) {
        bail!("multica_workspace_sensitive_field_rejected");
    }
    Ok(())
}

fn contains_sensitive_workspace_field(value: &Value) -> bool {
    let mut pending = vec![value];
    while let Some(next) = pending.pop() {
        match next {
            Value::Object(object) => {
                for (key, child) in object {
                    let normalized = key.to_ascii_lowercase().replace(['-', ' '], "_");
                    if matches!(
                        normalized.as_str(),
                        "authorization"
                            | "api_key"
                            | "apikey"
                            | "access_token"
                            | "refresh_token"
                            | "bearer_token"
                            | "password"
                            | "secret"
                    ) {
                        return true;
                    }
                    pending.push(child);
                }
            }
            Value::Array(values) => pending.extend(values),
            _ => {}
        }
    }
    false
}

fn local_workspace_store_lock(path: &Path) -> anyhow::Result<LocalWorkspaceStoreGuard> {
    static PROCESS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let process = PROCESS_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("multica_workspace_store_lock_unavailable"))?;
    let lock_path = PathBuf::from(format!("{}.lock", path.to_string_lossy()));
    if let Some(parent) = lock_path.parent() {
        crate::settings::create_private_dir_all(parent)
            .map_err(|_| anyhow!("multica_workspace_store_lock_unavailable"))?;
    }
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(|_| anyhow!("multica_workspace_store_lock_unavailable"))?;
    file.lock_exclusive()
        .map_err(|_| anyhow!("multica_workspace_store_lock_unavailable"))?;
    Ok(LocalWorkspaceStoreGuard {
        _process: process,
        file,
    })
}

struct LocalWorkspaceStoreGuard {
    _process: std::sync::MutexGuard<'static, ()>,
    file: fs::File,
}

impl Drop for LocalWorkspaceStoreGuard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

fn local_workspace_enabled() -> anyhow::Result<bool> {
    Ok(SettingsStore::default().load()?.multica_workspace_enabled)
}

fn local_workspace_identity() -> MulticaWorkspaceIdentity {
    let id = local_workspace_id();
    MulticaWorkspaceIdentity {
        id: id.clone(),
        slug: id,
        name: "Local Multica Workspace".to_string(),
    }
}

fn local_workspace_id() -> String {
    local_workspace_id_for_path(&crate::paths::default_multica_state_dir())
}

fn local_workspace_id_for_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let stable = if cfg!(windows) {
        raw.replace('\\', "/").to_ascii_lowercase()
    } else {
        raw.to_string()
    };
    let digest = Sha256::digest(stable.as_bytes());
    format!(
        "local-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7]
    )
}

fn local_user_projection(workspace: &MulticaWorkspaceIdentity) -> Value {
    json!({
        "id": local_user_id(workspace),
        "kind": "local_control_plane",
    })
}

fn local_user_id(workspace: &MulticaWorkspaceIdentity) -> String {
    format!("{}-user", workspace.id)
}

fn paginate(items: &[Value], limit: u16, offset: u32) -> Vec<Value> {
    let start = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(items.len());
    let end = start.saturating_add(usize::from(limit)).min(items.len());
    items[start..end].to_vec()
}

fn collection(
    workspace: &MulticaWorkspaceIdentity,
    resource: MulticaWorkspaceResourceKey,
    items: Vec<Value>,
    total: u64,
    limit: u16,
    offset: u32,
) -> MulticaWorkspaceCollection {
    MulticaWorkspaceCollection {
        workspace_id: workspace.id.clone(),
        resource,
        items,
        total,
        limit,
        offset,
        fetched_at_ms: now_ms(),
        stale: false,
        diagnostic: None,
    }
}

fn unavailable_collection(
    workspace: &MulticaWorkspaceIdentity,
    resource: MulticaWorkspaceResourceKey,
    limit: u16,
    offset: u32,
    diagnostic: &str,
) -> MulticaWorkspaceCollection {
    let mut value = collection(workspace, resource, Vec::new(), 0, limit, offset);
    value.stale = true;
    value.diagnostic = Some(diagnostic.to_string());
    value
}

fn diagnostic_code(error: &anyhow::Error) -> &'static str {
    match error
        .to_string()
        .split_whitespace()
        .next()
        .unwrap_or_default()
    {
        "codex_page_host_unavailable" => CODEX_PAGE_HOST_UNAVAILABLE,
        "multica_workspace_disabled" => MULTICA_WORKSPACE_DISABLED,
        "runtime_skills_unsupported" => "runtime_skills_unsupported",
        "managed_runtime_rpc_unavailable" => CODEX_PAGE_HOST_UNAVAILABLE,
        _ => "codex_page_host_error",
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_query_rejects_unbounded_pagination() {
        assert!(
            MulticaWorkspaceQuery {
                resource: MulticaWorkspaceResourceKey::Issues,
                limit: 101,
                offset: 0,
            }
            .validate()
            .is_err()
        );
        assert!(
            MulticaWorkspaceQuery {
                resource: MulticaWorkspaceResourceKey::Skills,
                limit: 25,
                offset: 100_001,
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn module_order_is_fixed_and_contains_skills() {
        let keys = MulticaWorkspaceResourceKey::ALL
            .into_iter()
            .map(|resource| resource.key())
            .collect::<Vec<_>>();
        assert_eq!(
            keys,
            vec![
                "my_tasks",
                "issues",
                "projects",
                "autopilots",
                "agents",
                "squads",
                "statistics",
                "runtimes",
                "skills",
                "settings",
            ]
        );
    }

    #[test]
    fn local_workspace_id_is_stable_private_and_path_scoped() {
        let first = local_workspace_id_for_path(Path::new("C:\\Users\\fixture\\.ccp\\multica"));
        let replay = local_workspace_id_for_path(Path::new("C:\\Users\\fixture\\.ccp\\multica"));
        let other = local_workspace_id_for_path(Path::new("C:\\Users\\other\\.ccp\\multica"));
        assert_eq!(first, replay);
        assert_ne!(first, other);
        assert!(first.starts_with("local-"));
        assert!(!first.contains("fixture"));
        assert!(!first.contains("Users"));
    }

    #[test]
    fn local_collections_do_not_require_a_managed_profile() {
        let workspace = local_workspace_identity();
        let dir = tempfile::tempdir().unwrap();
        let execution_store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let workspace_store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        let issues = query_local_collection(
            &workspace,
            &execution_store,
            &workspace_store,
            true,
            MulticaWorkspaceQuery {
                resource: MulticaWorkspaceResourceKey::Issues,
                limit: 25,
                offset: 0,
            },
        )
        .unwrap();
        assert_eq!(issues.workspace_id, workspace.id);
        assert!(issues.items.is_empty());
        assert_eq!(
            issues.diagnostic.as_deref(),
            Some(LOCAL_CONTROL_PLANE_EMPTY)
        );

        let runtimes = query_local_collection(
            &workspace,
            &execution_store,
            &workspace_store,
            true,
            MulticaWorkspaceQuery {
                resource: MulticaWorkspaceResourceKey::Runtimes,
                limit: 25,
                offset: 0,
            },
        )
        .unwrap();
        assert_eq!(runtimes.items.len(), 1);
        assert_eq!(
            runtimes.diagnostic.as_deref(),
            Some(CODEX_PAGE_HOST_UNAVAILABLE)
        );
    }

    #[test]
    fn local_store_persists_entities_and_enforces_revision_cas() {
        let workspace = local_workspace_identity();
        let dir = tempfile::tempdir().unwrap();
        let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        let created = store
            .upsert(
                &workspace.id,
                LocalWorkspaceEntityUpsert {
                    resource: MulticaWorkspaceResourceKey::Issues,
                    entity: json!({"id": "issue-a", "title": "First"}),
                    expected_revision: None,
                },
                10,
            )
            .unwrap();
        assert_eq!(created["revision"], 1);
        assert_eq!(created["workspace_id"], workspace.id);

        let conflict = store
            .upsert(
                &workspace.id,
                LocalWorkspaceEntityUpsert {
                    resource: MulticaWorkspaceResourceKey::Issues,
                    entity: json!({"id": "issue-a", "title": "Blind overwrite"}),
                    expected_revision: None,
                },
                11,
            )
            .unwrap_err();
        assert_eq!(conflict.to_string(), "multica_workspace_revision_conflict");

        let updated = store
            .upsert(
                &workspace.id,
                LocalWorkspaceEntityUpsert {
                    resource: MulticaWorkspaceResourceKey::Issues,
                    entity: json!({"id": "issue-a", "title": "Updated"}),
                    expected_revision: Some(1),
                },
                12,
            )
            .unwrap();
        assert_eq!(updated["revision"], 2);
        assert_eq!(updated["created_at_ms"], 10);
        assert_eq!(
            store
                .list(&workspace.id, MulticaWorkspaceResourceKey::Issues)
                .unwrap()
                .len(),
            1
        );

        let delete_conflict = store
            .delete(
                &workspace.id,
                LocalWorkspaceEntityDelete {
                    resource: MulticaWorkspaceResourceKey::Issues,
                    entity_id: "issue-a".to_string(),
                    expected_revision: 1,
                },
            )
            .unwrap_err();
        assert_eq!(
            delete_conflict.to_string(),
            "multica_workspace_revision_conflict"
        );
        assert!(
            store
                .delete(
                    &workspace.id,
                    LocalWorkspaceEntityDelete {
                        resource: MulticaWorkspaceResourceKey::Issues,
                        entity_id: "issue-a".to_string(),
                        expected_revision: 2,
                    },
                )
                .unwrap()
        );
        assert!(
            store
                .list(&workspace.id, MulticaWorkspaceResourceKey::Issues)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn local_query_reads_persisted_entities_and_filters_my_tasks() {
        let workspace = local_workspace_identity();
        let dir = tempfile::tempdir().unwrap();
        let execution_store = MulticaExecutionStore::new(dir.path().join("execution.json"));
        let workspace_store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        for (id, assignee_id) in [
            ("issue-mine", local_user_id(&workspace)),
            ("issue-other", "other-user".to_string()),
        ] {
            workspace_store
                .upsert(
                    &workspace.id,
                    LocalWorkspaceEntityUpsert {
                        resource: MulticaWorkspaceResourceKey::Issues,
                        entity: json!({"id": id, "title": id, "assignee_id": assignee_id}),
                        expected_revision: None,
                    },
                    1,
                )
                .unwrap();
        }
        let issues = query_local_collection(
            &workspace,
            &execution_store,
            &workspace_store,
            true,
            MulticaWorkspaceQuery {
                resource: MulticaWorkspaceResourceKey::Issues,
                limit: 50,
                offset: 0,
            },
        )
        .unwrap();
        assert_eq!(issues.total, 2);
        assert_eq!(issues.items.len(), 2);

        let mine = query_local_collection(
            &workspace,
            &execution_store,
            &workspace_store,
            true,
            MulticaWorkspaceQuery {
                resource: MulticaWorkspaceResourceKey::MyTasks,
                limit: 50,
                offset: 0,
            },
        )
        .unwrap();
        assert_eq!(mine.total, 1);
        assert_eq!(mine.items[0]["id"], "issue-mine");
    }

    #[test]
    fn runtime_summary_is_a_projection_of_page_host_capabilities() {
        let capabilities = CodexRuntimeCapabilities {
            runtime_id: "page-host-a".to_string(),
            provider: "codex".to_string(),
            protocol_version: Some("1".to_string()),
            server_version: None,
            capabilities: vec!["agent-skill-v1".to_string(), "thread-fork".to_string()],
            skills_supported: true,
            skill_protocol: Some("agent-skill-v1".to_string()),
            subagents_supported: true,
        };
        let summary = runtime_summary_from_capabilities(&capabilities);
        assert!(summary.available);
        assert!(summary.skills_supported);
        assert!(summary.multi_agent_supported);
        assert_eq!(summary.runtime_id.as_deref(), Some("page-host-a"));
    }

    #[test]
    fn settings_projection_contains_only_the_local_workspace_contract() {
        let workspace = local_workspace_identity();
        let value = settings_collection(&workspace, false);
        let encoded = serde_json::to_string(&value).unwrap();
        assert!(encoded.contains("\"enabled\":false"));
        assert!(encoded.contains("\"mode\":\"local\""));
        assert!(!encoded.contains("server_url"));
        assert!(!encoded.contains("token"));
        assert!(!encoded.contains("daemon"));
        assert!(!encoded.contains("profile"));
    }

    #[test]
    fn skill_projection_keeps_runtime_paths_out_of_renderer_values() {
        let skill = CodexSkill {
            id: "skill:review".to_string(),
            name: "Review".to_string(),
            summary: Some("Review changes".to_string()),
            scope: Some("C:\\Users\\fixture\\.codex\\skills".to_string()),
            enabled: true,
            manifest_digest: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            ),
        };
        let item = runtime_codex_skill_item(
            &skill,
            "page-host-a",
            &crate::multica_skill_trust::LocalSkillTrustSnapshot::default(),
        );
        let encoded = serde_json::to_string(&item).unwrap();
        assert!(!encoded.contains("C:\\\\Users"));
        assert!(!encoded.contains("\"scope\""));
        assert!(encoded.contains("\"inventory_source\":\"codex_page_host\""));
    }

    #[tokio::test]
    async fn hostless_skill_resolution_fails_closed() {
        let error = resolve_skill_bindings(SkillBindingSelection {
            bindings: Default::default(),
        })
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), CODEX_PAGE_HOST_UNAVAILABLE);
    }
}
