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
use rusqlite::{Connection, OpenFlags};
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
    Comments,
    Labels,
    Subscribers,
    Reactions,
    Activities,
    Projects,
    ProjectResources,
    Autopilots,
    Agents,
    Squads,
    Statistics,
    Runtimes,
    Skills,
    Settings,
}

impl MulticaWorkspaceResourceKey {
    pub const ALL: [Self; 16] = [
        Self::MyTasks,
        Self::Issues,
        Self::Comments,
        Self::Labels,
        Self::Subscribers,
        Self::Reactions,
        Self::Activities,
        Self::Projects,
        Self::ProjectResources,
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
            Self::Comments => "comments",
            Self::Labels => "labels",
            Self::Subscribers => "subscribers",
            Self::Reactions => "reactions",
            Self::Activities => "activities",
            Self::Projects => "projects",
            Self::ProjectResources => "project_resources",
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
    #[serde(default)]
    pub skills_inventory_supported: bool,
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
    pub comments: Vec<Value>,
    #[serde(default)]
    pub labels: Vec<Value>,
    #[serde(default)]
    pub subscribers: Vec<Value>,
    #[serde(default)]
    pub reactions: Vec<Value>,
    #[serde(default)]
    pub activities: Vec<Value>,
    #[serde(default)]
    pub projects: Vec<Value>,
    #[serde(default)]
    pub project_resources: Vec<Value>,
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
            comments: Vec::new(),
            labels: Vec::new(),
            subscribers: Vec::new(),
            reactions: Vec::new(),
            activities: Vec::new(),
            projects: Vec::new(),
            project_resources: Vec::new(),
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
            MulticaWorkspaceResourceKey::Comments => Ok(&self.comments),
            MulticaWorkspaceResourceKey::Labels => Ok(&self.labels),
            MulticaWorkspaceResourceKey::Subscribers => Ok(&self.subscribers),
            MulticaWorkspaceResourceKey::Reactions => Ok(&self.reactions),
            MulticaWorkspaceResourceKey::Activities => Ok(&self.activities),
            MulticaWorkspaceResourceKey::Projects => Ok(&self.projects),
            MulticaWorkspaceResourceKey::ProjectResources => Ok(&self.project_resources),
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
            MulticaWorkspaceResourceKey::Comments => Ok(&mut self.comments),
            MulticaWorkspaceResourceKey::Labels => Ok(&mut self.labels),
            MulticaWorkspaceResourceKey::Subscribers => Ok(&mut self.subscribers),
            MulticaWorkspaceResourceKey::Reactions => Ok(&mut self.reactions),
            // Activities are derived/audit records. They are queryable but
            // must never be fabricated through the generic mutation path.
            MulticaWorkspaceResourceKey::Activities => {
                bail!("multica_workspace_resource_read_only")
            }
            MulticaWorkspaceResourceKey::Projects => Ok(&mut self.projects),
            MulticaWorkspaceResourceKey::ProjectResources => Ok(&mut self.project_resources),
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
        validate_local_entity(&value, workspace_id, command.resource)?;
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
    // Native Codex state is a separate, read-only projection.  It is not
    // folded into editable Multica entities because the SQLite schema and
    // lifecycle are owned by Codex itself.
    for (key, items) in codex_native_inventory() {
        let total = items.len() as u64;
        collections.insert(
            key.to_string(),
            collection(
                &workspace,
                MulticaWorkspaceResourceKey::Projects,
                items,
                total,
                100,
                0,
            ),
        );
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

fn codex_native_inventory() -> [(&'static str, Vec<Value>); 3] {
    let mut threads = Vec::new();
    let mut projects = Vec::new();
    let mut project_paths = Vec::new();
    let mut tool_calls = Vec::new();
    let home = crate::codex_sqlite::default_codex_home_dir();
    for path in crate::codex_sqlite::codex_session_db_paths_from_home(&home) {
        let Ok(db) = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
            continue;
        };
        if sqlite_has_table(&db, "threads") {
            let columns = sqlite_columns_safe(&db, "threads");
            let title = if columns.iter().any(|c| c == "title") {
                "title"
            } else {
                "''"
            };
            let cwd = if columns.iter().any(|c| c == "cwd") {
                "cwd"
            } else {
                "''"
            };
            let updated = if columns.iter().any(|c| c == "updated_at_ms") {
                "updated_at_ms"
            } else {
                "0"
            };
            let sql = format!(
                "SELECT id, {title}, {cwd}, {updated} FROM threads ORDER BY COALESCE({updated}, 0) DESC LIMIT 100"
            );
            let _ = db
                .prepare(&sql)
                .and_then(|mut stmt| {
                    let rows = stmt.query_map([], |row| {
                        let id: String = row.get(0)?;
                        let title = row.get::<_, Option<String>>(1)?.unwrap_or_default();
                        let cwd = row.get::<_, Option<String>>(2)?.unwrap_or_default();
                        let updated = row.get::<_, Option<i64>>(3)?.unwrap_or_default();
                        Ok((id, title, cwd, updated))
                    })?;
                    for item in rows.flatten() {
                        let (id, title, cwd, updated) = item;
                        threads.push(json!({"id": id, "title": title, "cwd": cwd, "updated_at_ms": updated, "source": "codex_native"}));
                    }
                    Ok(())
                });
            if sqlite_has_table(&db, "thread_spawn_edges") {
                let cols = sqlite_columns_safe(&db, "thread_spawn_edges");
                let child = if cols.iter().any(|c| c == "child_thread_id") {
                    "child_thread_id"
                } else {
                    "child_id"
                };
                let parent = if cols.iter().any(|c| c == "parent_thread_id") {
                    "parent_thread_id"
                } else {
                    "parent_id"
                };
                if cols.iter().any(|c| c == child) && cols.iter().any(|c| c == parent) {
                    if let Ok(mut stmt) =
                        db.prepare(&format!("SELECT {child}, {parent} FROM thread_spawn_edges"))
                    {
                        if let Ok(rows) = stmt.query_map([], |row| {
                            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                        }) {
                            for pair in rows.flatten() {
                                if let Some(thread) = threads.iter_mut().find(|v| {
                                    v.get("id").and_then(Value::as_str) == Some(pair.0.as_str())
                                }) {
                                    thread["parent_thread_id"] = Value::String(pair.1);
                                    thread["is_subagent"] = Value::Bool(true);
                                }
                            }
                        }
                    }
                }
            }
            if sqlite_has_table(&db, "thread_dynamic_tools") {
                let cols = sqlite_columns_safe(&db, "thread_dynamic_tools");
                let thread_col = if cols.iter().any(|c| c == "thread_id") {
                    "thread_id"
                } else {
                    "threadId"
                };
                if cols.iter().any(|c| c == thread_col) {
                    if let Ok(mut stmt) = db.prepare(&format!("SELECT {thread_col}, COUNT(*) FROM thread_dynamic_tools GROUP BY {thread_col}")) {
                        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))) {
                            for pair in rows.flatten() {
                                if let Some(thread) = threads.iter_mut().find(|v| v.get("id").and_then(Value::as_str) == Some(pair.0.as_str())) {
                                    thread["dynamic_tool_call_count"] = Value::from(pair.1);
                                }
                            }
                        }
                    }
                    let name_col = ["name", "tool_name", "toolName", "slug"]
                        .into_iter()
                        .find(|c| cols.iter().any(|x| x == c));
                    if let Some(name_col) = name_col {
                        let id_col = ["id", "call_id", "callId"]
                            .into_iter()
                            .find(|c| cols.iter().any(|x| x == c))
                            .unwrap_or(thread_col);
                        if let Ok(mut stmt) = db.prepare(&format!(
                            "SELECT {thread_col}, {name_col}, {id_col} FROM thread_dynamic_tools LIMIT 500"
                        )) {
                            if let Ok(rows) = stmt.query_map([], |row| {
                                Ok(json!({
                                    "id": row.get::<_, String>(2).unwrap_or_default(),
                                    "thread_id": row.get::<_, String>(0).unwrap_or_default(),
                                    "name": row.get::<_, String>(1).unwrap_or_default(),
                                    "source": "codex_native"
                                }))
                            }) {
                                tool_calls.extend(rows.flatten());
                            }
                        }
                    }
                }
            }
        }
        if sqlite_has_table(&db, "project_roots") {
            let _ = db
                .prepare("SELECT path FROM project_roots WHERE COALESCE(path, '') <> '' LIMIT 100")
                .and_then(|mut stmt| {
                    let rows = stmt.query_map([], |row| {
                        let path: String = row.get(0)?;
                        Ok(json!({"id": format!("codex-project:{}", path), "path": path, "source": "codex_native"}))
                    })?;
                    for project in rows.flatten() {
                        if let Some(path) = project.get("path").and_then(Value::as_str) {
                            project_paths.push(path.to_string());
                        }
                        projects.push(project);
                    }
                    Ok(())
                });
        }
    }
    // Native execution events live in the read-only history projection, not
    // in state_5.sqlite. Project only bounded metadata so arguments and
    // message bodies are never copied into the Multica control plane.
    let history_path = home.join("thread_history_1.sqlite");
    if let Ok(db) = Connection::open_with_flags(&history_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        && sqlite_has_table(&db, "thread_items")
    {
        if let Ok(mut stmt) = db.prepare(
            "SELECT thread_id, item_id, item_type, item_json FROM thread_items ORDER BY created_at_ms DESC LIMIT 1000",
        ) {
            if let Ok(rows) = stmt.query_map([], |row| {
                let thread_id: String = row.get(0)?;
                let item_id: String = row.get(1)?;
                let item_type: String = row.get(2)?;
                let item_json: String = row.get(3)?;
                Ok((thread_id, item_id, item_type, item_json))
            }) {
                for (thread_id, item_id, item_type, item_json) in rows.flatten() {
                    if !matches!(
                        item_type.as_str(),
                        "subAgentActivity" | "mcpToolCall" | "dynamicToolCall"
                    ) {
                        continue;
                    }
                    let parsed = serde_json::from_str::<Value>(&item_json).ok();
                    let name = parsed.as_ref().and_then(|value| {
                        ["name", "tool_name", "toolName", "command"]
                            .into_iter()
                            .find_map(|key| value.get(key).and_then(Value::as_str))
                    });
                    let mut entry = json!({
                        "id": item_id,
                        "thread_id": thread_id,
                        "item_type": item_type,
                        "source": "codex_native",
                    });
                    if let Some(name) = name {
                        entry["name"] = Value::String(name.to_string());
                    }
                    tool_calls.push(entry);
                }
            }
        }
    }
    // `threads.project_id` is not present or reliable in every Codex schema. Resolve
    // the owning project from the read-only project root paths and the thread cwd.
    for thread in &mut threads {
        let Some(cwd) = thread.get("cwd").and_then(Value::as_str) else {
            continue;
        };
        if let Some(project_path) = longest_project_path_match(cwd, &project_paths) {
            thread["project_path"] = Value::String(project_path.clone());
            thread["project_id"] = Value::String(format!("codex-project:{project_path}"));
        }
    }
    threads.sort_by(|a, b| {
        b.get("updated_at_ms")
            .and_then(Value::as_i64)
            .cmp(&a.get("updated_at_ms").and_then(Value::as_i64))
    });
    threads.truncate(100);
    projects.sort_by(|a, b| {
        a.get("path")
            .and_then(Value::as_str)
            .cmp(&b.get("path").and_then(Value::as_str))
    });
    projects.dedup_by(|a, b| a.get("path") == b.get("path"));
    projects.truncate(100);
    tool_calls.truncate(500);
    [
        ("codex_native_threads", threads),
        ("codex_native_projects", projects),
        ("codex_native_tool_calls", tool_calls),
    ]
}

fn normalize_project_path(path: &str) -> String {
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn longest_project_path_match(cwd: &str, project_paths: &[String]) -> Option<String> {
    let cwd = normalize_project_path(cwd);
    project_paths
        .iter()
        .filter_map(|path| {
            let normalized = normalize_project_path(path);
            if normalized.is_empty()
                || !(cwd == normalized || cwd.starts_with(&(normalized.clone() + "\\")))
            {
                return None;
            }
            Some((normalized.len(), path))
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, path)| path.clone())
}

fn sqlite_has_table(db: &Connection, table: &str) -> bool {
    db.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
        [table],
        |_| Ok(()),
    )
    .is_ok()
}

fn sqlite_columns_safe(db: &Connection, table: &str) -> Vec<String> {
    let Ok(mut stmt) = db.prepare(&format!("PRAGMA table_info({table})")) else {
        return Vec::new();
    };
    stmt.query_map([], |row| row.get::<_, String>(1))
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
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
    if !capabilities.skills_inventory_supported {
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
        .map(|skill| {
            runtime_codex_skill_item(
                skill,
                &capabilities.runtime_id,
                &trust,
                capabilities.skills_supported,
            )
        })
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
        | MulticaWorkspaceResourceKey::Comments
        | MulticaWorkspaceResourceKey::Labels
        | MulticaWorkspaceResourceKey::Subscribers
        | MulticaWorkspaceResourceKey::Reactions
        | MulticaWorkspaceResourceKey::Activities
        | MulticaWorkspaceResourceKey::Projects
        | MulticaWorkspaceResourceKey::ProjectResources
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
    if matches!(
        resource,
        MulticaWorkspaceResourceKey::Issues | MulticaWorkspaceResourceKey::MyTasks
    ) {
        let state = store.load(&workspace.id)?;
        project_issue_collaboration(&mut all_items, &state);
    }
    if resource == MulticaWorkspaceResourceKey::Autopilots {
        project_autopilot_contract(&mut all_items);
    }
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

/// Normalize embedded automation detail into the list-endpoint fields used by
/// Multica. This is strictly derived from persisted trigger/run rows; when an
/// older local entity has no detail arrays, optional fields remain absent.
fn project_autopilot_contract(autopilots: &mut [Value]) {
    for autopilot in autopilots {
        let Some(object) = autopilot.as_object_mut() else {
            continue;
        };
        if !object.contains_key("subscribers") {
            object.insert("subscribers".to_string(), Value::Array(Vec::new()));
        }
        if let Some(triggers) = object.get("triggers").and_then(Value::as_array).cloned() {
            let mut kinds = BTreeSet::new();
            let mut next_run: Option<&str> = None;
            for trigger in &triggers {
                if trigger.get("enabled").and_then(Value::as_bool) == Some(true) {
                    if let Some(kind) = trigger.get("kind").and_then(Value::as_str) {
                        kinds.insert(kind.to_string());
                    }
                    if let Some(value) = trigger
                        .get("next_run_at")
                        .or_else(|| trigger.get("nextRunAt"))
                        .and_then(Value::as_str)
                    {
                        if next_run.is_none_or(|current| value < current) {
                            next_run = Some(value);
                        }
                    }
                }
            }
            object.insert(
                "trigger_kinds".to_string(),
                Value::Array(kinds.into_iter().map(Value::String).collect()),
            );
            if let Some(value) = next_run {
                object.insert("next_run_at".to_string(), Value::String(value.to_string()));
            }
        }
        if let Some(runs) = object.get("runs").and_then(Value::as_array).cloned() {
            let latest = runs
                .iter()
                .filter_map(|run| {
                    let timestamp = run
                        .get("triggered_at")
                        .or_else(|| run.get("created_at"))
                        .or_else(|| run.get("triggeredAt"))
                        .and_then(Value::as_str)?;
                    let status = run.get("status").and_then(Value::as_str)?;
                    Some((timestamp, status))
                })
                .max_by(|left, right| left.0.cmp(right.0));
            if let Some((_, status)) = latest {
                object.insert(
                    "last_run_status".to_string(),
                    Value::String(status.to_string()),
                );
            }
        }
    }
}

/// Project the local collaboration resources onto Issue reads in the same
/// shape as the upstream Issue DTO. This is derived at query time so the
/// canonical entities and their revisions remain untouched.
fn project_issue_collaboration(issues: &mut [Value], state: &LocalMulticaWorkspaceState) {
    for issue in issues {
        let Some(issue_id) = issue.get("id").and_then(Value::as_str) else {
            continue;
        };
        let labels: Vec<Value> = state
            .labels
            .iter()
            .filter(|label| {
                let direct_issue = label
                    .get("issue_id")
                    .or_else(|| label.get("issueId"))
                    .and_then(Value::as_str)
                    == Some(issue_id);
                let linked_issue = label
                    .get("issue_ids")
                    .or_else(|| label.get("issueIds"))
                    .and_then(Value::as_array)
                    .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(issue_id)));
                let issue_label_id = issue
                    .get("label_ids")
                    .or_else(|| issue.get("labelIds"))
                    .and_then(Value::as_array)
                    .is_some_and(|ids| {
                        label.get("id").and_then(Value::as_str).is_some_and(|id| {
                            ids.iter().any(|candidate| candidate.as_str() == Some(id))
                        })
                    });
                direct_issue || linked_issue || issue_label_id
            })
            .cloned()
            .collect();
        let comments: Vec<Value> = state
            .comments
            .iter()
            .filter(|comment| {
                comment
                    .get("issue_id")
                    .or_else(|| comment.get("issueId"))
                    .and_then(Value::as_str)
                    == Some(issue_id)
            })
            .cloned()
            .collect();
        let comment_ids = comments
            .iter()
            .filter_map(|comment| comment.get("id").and_then(Value::as_str))
            .collect::<BTreeSet<_>>();
        let reactions: Vec<Value> = state
            .reactions
            .iter()
            .filter(|reaction| {
                let direct_issue = reaction
                    .get("issue_id")
                    .or_else(|| reaction.get("issueId"))
                    .and_then(Value::as_str)
                    == Some(issue_id);
                let comment_reaction = reaction
                    .get("comment_id")
                    .or_else(|| reaction.get("commentId"))
                    .and_then(Value::as_str)
                    .is_some_and(|comment_id| comment_ids.contains(comment_id));
                direct_issue || comment_reaction
            })
            .cloned()
            .collect();
        let activities: Vec<&Value> = state
            .activities
            .iter()
            .filter(|activity| {
                activity
                    .get("issue_id")
                    .or_else(|| activity.get("issueId"))
                    .and_then(Value::as_str)
                    == Some(issue_id)
            })
            .collect();
        // Upstream Multica exposes a unified issue timeline. Keep the local
        // collections canonical, but project a read-only merged view so the
        // task surface can render comments and activities in one stream.
        let mut timeline = Vec::with_capacity(comments.len() + activities.len());
        for comment in &comments {
            let mut entry = comment.clone();
            if let Some(object) = entry.as_object_mut() {
                object.insert("type".to_string(), Value::String("comment".to_string()));
            }
            timeline.push(entry);
        }
        for activity in &activities {
            let mut entry = (*activity).clone();
            if let Some(object) = entry.as_object_mut() {
                object.insert("type".to_string(), Value::String("activity".to_string()));
            }
            timeline.push(entry);
        }
        timeline.sort_by(|left, right| {
            let left_time = left
                .get("created_at")
                .or_else(|| left.get("createdAt"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let right_time = right
                .get("created_at")
                .or_else(|| right.get("createdAt"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            left_time.cmp(right_time).then_with(|| {
                left.get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .cmp(right.get("id").and_then(Value::as_str).unwrap_or_default())
            })
        });
        let latest = comments
            .iter()
            .chain(activities.iter().copied())
            .filter_map(|item| {
                item.get("created_at")
                    .or_else(|| item.get("createdAt"))
                    .and_then(Value::as_str)
            })
            .max()
            .map(str::to_string);
        issue["labels"] = Value::Array(labels);
        issue["reactions"] = Value::Array(reactions);
        issue["comment_count"] = Value::from(comments.len() as u64);
        issue["activity_count"] = Value::from(activities.len() as u64);
        issue["timeline"] = Value::Array(timeline);
        if let Some(latest) = latest {
            issue["last_activity_at"] = Value::String(latest);
        }
    }
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
    let bindings = execution_state
        .execution_bindings
        .iter()
        .filter(|binding| binding.workspace_id == workspace.id)
        .collect::<Vec<_>>();
    let mut execution_statuses = BTreeMap::<String, u64>::new();
    let mut failure_codes = BTreeMap::<String, u64>::new();
    let mut duration_total_ms = 0u64;
    let mut duration_count = 0u64;
    let mut retryable_failures = 0u64;
    for binding in &bindings {
        let status = serde_json::to_value(binding.state)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".to_string());
        *execution_statuses.entry(status).or_default() += 1;
        if binding.state == crate::multica_execution_store::MulticaExecutionBindingState::Failed {
            if let Some(code) = binding.last_error_code.as_deref() {
                *failure_codes.entry(code.to_string()).or_default() += 1;
            }
            if binding.retryable {
                retryable_failures += 1;
            }
        }
        if let Some(completed_at_ms) = binding.completed_at_ms {
            if completed_at_ms >= binding.created_at_ms {
                duration_total_ms = duration_total_ms
                    .saturating_add(completed_at_ms.saturating_sub(binding.created_at_ms));
                duration_count += 1;
            }
        }
    }
    let execution_total = bindings.len() as u64;
    let terminal_total = bindings
        .iter()
        .filter(|binding| {
            matches!(
                binding.state,
                crate::multica_execution_store::MulticaExecutionBindingState::Completed
                    | crate::multica_execution_store::MulticaExecutionBindingState::Failed
                    | crate::multica_execution_store::MulticaExecutionBindingState::Cancelled
            )
        })
        .count() as u64;
    let successful_total = bindings
        .iter()
        .filter(|binding| {
            binding.state == crate::multica_execution_store::MulticaExecutionBindingState::Completed
        })
        .count() as u64;
    let issue_statuses = workspace_state
        .issues
        .iter()
        .filter_map(|issue| issue.get("status").and_then(Value::as_str))
        .fold(BTreeMap::<String, u64>::new(), |mut counts, status| {
            *counts.entry(status.to_string()).or_default() += 1;
            counts
        });
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
            "execution_total": execution_total,
            "execution_terminal_total": terminal_total,
            "execution_successful_total": successful_total,
            "execution_success_rate": if terminal_total == 0 { Value::Null } else { json!(successful_total as f64 / terminal_total as f64) },
            "execution_statuses": execution_statuses,
            "failure_codes": failure_codes,
            "retryable_failures": retryable_failures,
            "average_execution_duration_ms": if duration_count == 0 { Value::Null } else { json!(duration_total_ms / duration_count) },
            "issue_statuses": issue_statuses,
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
        && bindings.is_empty()
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
            "skills_inventory_supported": runtime.skills_inventory_supported,
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
        skills_inventory_supported: capabilities.skills_inventory_supported,
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
        skills_inventory_supported: false,
        skill_protocol: None,
        multi_agent_supported: false,
    }
}

fn runtime_codex_skill_item(
    skill: &CodexSkill,
    runtime_id: &str,
    trust: &crate::multica_skill_trust::LocalSkillTrustSnapshot,
    execution_supported: bool,
) -> Value {
    let local = trust.get(&skill.id);
    let digest_matches = skill
        .manifest_digest
        .as_deref()
        .zip(local.and_then(|entry| entry.manifest_digest.as_deref()))
        .is_some_and(|(runtime_digest, local_digest)| runtime_digest == local_digest);
    let dispatch_allowed = execution_supported
        && skill.enabled
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
        "compatible": execution_supported && skill.enabled && skill.manifest_digest.is_some(),
        "execution_supported": execution_supported,
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
    for (resource, entities) in [
        (MulticaWorkspaceResourceKey::Issues, &state.issues),
        (MulticaWorkspaceResourceKey::Comments, &state.comments),
        (MulticaWorkspaceResourceKey::Labels, &state.labels),
        (MulticaWorkspaceResourceKey::Subscribers, &state.subscribers),
        (MulticaWorkspaceResourceKey::Reactions, &state.reactions),
        (MulticaWorkspaceResourceKey::Activities, &state.activities),
        (MulticaWorkspaceResourceKey::Projects, &state.projects),
        (
            MulticaWorkspaceResourceKey::ProjectResources,
            &state.project_resources,
        ),
        (MulticaWorkspaceResourceKey::Agents, &state.agents),
        (MulticaWorkspaceResourceKey::Squads, &state.squads),
        (MulticaWorkspaceResourceKey::Autopilots, &state.autopilots),
    ] {
        if entities.len() > MAX_LOCAL_ENTITIES_PER_RESOURCE {
            bail!("multica_workspace_collection_too_large");
        }
        let mut ids = BTreeSet::new();
        for entity in entities {
            validate_local_entity(entity, &state.workspace_id, resource)?;
            let id = entity
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("multica_workspace_entity_invalid"))?;
            if !ids.insert(id.to_ascii_lowercase()) {
                bail!("multica_workspace_entity_conflict");
            }
        }
    }
    let project_ids = state
        .projects
        .iter()
        .filter_map(|project| project.get("id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    for resource in &state.project_resources {
        let project_id = resource
            .get("project_id")
            .or_else(|| resource.get("projectId"))
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("multica_workspace_project_resource_invalid"))?;
        if !project_ids.contains(project_id) {
            bail!("multica_workspace_project_resource_project_missing");
        }
    }
    let issue_ids = state
        .issues
        .iter()
        .filter_map(|issue| issue.get("id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let comments_by_id = state
        .comments
        .iter()
        .filter_map(|comment| {
            comment
                .get("id")
                .and_then(Value::as_str)
                .map(|id| (id, comment))
        })
        .collect::<BTreeMap<_, _>>();
    for comment in &state.comments {
        let issue_id = comment
            .get("issue_id")
            .or_else(|| comment.get("issueId"))
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("multica_workspace_comment_invalid"))?;
        if !issue_ids.contains(issue_id) {
            bail!("multica_workspace_comment_issue_missing");
        }
        if let Some(parent_id) = comment
            .get("parent_id")
            .or_else(|| comment.get("parentId"))
            .and_then(Value::as_str)
        {
            if parent_id
                == comment
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            {
                bail!("multica_workspace_comment_parent_invalid");
            }
            let parent = comments_by_id
                .get(parent_id)
                .ok_or_else(|| anyhow!("multica_workspace_comment_parent_missing"))?;
            let parent_issue = parent
                .get("issue_id")
                .or_else(|| parent.get("issueId"))
                .and_then(Value::as_str);
            if parent_issue != Some(issue_id) {
                bail!("multica_workspace_comment_parent_issue_mismatch");
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

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.as_bytes().first() == Some(&b'#')
        && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}

fn validate_local_entity(
    entity: &Value,
    workspace_id: &str,
    resource: MulticaWorkspaceResourceKey,
) -> anyhow::Result<()> {
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
    validate_entity_contract(object, resource)?;
    Ok(())
}

fn validate_entity_contract(
    object: &serde_json::Map<String, Value>,
    resource: MulticaWorkspaceResourceKey,
) -> anyhow::Result<()> {
    let bounded_arrays = [
        "resources",
        "members",
        "memberAgentIds",
        "member_agent_ids",
        "skills",
        "disabled_runtime_skills",
        "conversation_starters",
        "subscribers",
        "triggers",
        "runs",
        "collaborators",
        "invocation_targets",
    ];
    for key in bounded_arrays {
        if let Some(value) = object.get(key) {
            let Some(items) = value.as_array() else {
                bail!("multica_workspace_entity_invalid");
            };
            if items.len() > 256 {
                bail!("multica_workspace_entity_too_large");
            }
        }
    }
    for key in ["label_ids", "labelIds"] {
        if let Some(value) = object.get(key) {
            let Some(items) = value.as_array() else {
                bail!("multica_workspace_label_ids_invalid");
            };
            if items.len() > 128 {
                bail!("multica_workspace_label_ids_invalid");
            }
            for item in items {
                let Some(id) = item.as_str() else {
                    bail!("multica_workspace_label_ids_invalid");
                };
                if id.trim().is_empty() || id.len() > 240 {
                    bail!("multica_workspace_label_ids_invalid");
                }
                validate_local_entity_id(id)
                    .map_err(|_| anyhow!("multica_workspace_label_ids_invalid"))?;
            }
        }
    }
    if resource == MulticaWorkspaceResourceKey::ProjectResources {
        let project_id = object
            .get("project_id")
            .or_else(|| object.get("projectId"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("multica_workspace_project_resource_invalid"))?;
        validate_local_entity_id(project_id)
            .map_err(|_| anyhow!("multica_workspace_project_resource_invalid"))?;
        let resource_type = object
            .get("resource_type")
            .or_else(|| object.get("resourceType"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let reference = object
            .get("resource_ref")
            .or_else(|| object.get("resourceRef"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("multica_workspace_project_resource_invalid"))?;
        match resource_type {
            "github_repo" => {
                let url = reference
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !is_supported_git_url(url) {
                    bail!("multica_workspace_project_resource_invalid");
                }
            }
            "local_directory" => {
                let path = reference
                    .get("local_path")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let daemon_id = reference
                    .get("daemon_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if path.trim().is_empty()
                    || path.len() > 1024
                    || daemon_id.trim().is_empty()
                    || daemon_id.len() > 240
                {
                    bail!("multica_workspace_project_resource_invalid");
                }
                if let Some(mode) = reference.get("execution_mode").and_then(Value::as_str)
                    && !matches!(mode, "in_place" | "worktree")
                {
                    bail!("multica_workspace_project_resource_invalid");
                }
            }
            _ => bail!("multica_workspace_project_resource_invalid"),
        }
        return Ok(());
    }
    if resource == MulticaWorkspaceResourceKey::Comments {
        let issue_id = object
            .get("issue_id")
            .or_else(|| object.get("issueId"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("multica_workspace_comment_invalid"))?;
        validate_local_entity_id(issue_id)
            .map_err(|_| anyhow!("multica_workspace_comment_invalid"))?;
        let content = object
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("multica_workspace_comment_invalid"))?;
        if content.trim().is_empty() || content.contains('\0') || content.chars().count() > 64_000 {
            bail!("multica_workspace_comment_invalid");
        }
        let comment_type = object
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("comment");
        if !matches!(
            comment_type,
            "comment" | "status_change" | "progress_update"
        ) {
            bail!("multica_workspace_comment_type_invalid");
        }
        if let Some(parent) = object.get("parent_id").or_else(|| object.get("parentId"))
            && !parent.is_null()
            && parent.as_str().is_none()
        {
            bail!("multica_workspace_comment_invalid");
        }
        return Ok(());
    }
    if resource == MulticaWorkspaceResourceKey::Labels {
        let name = object
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if name.trim().is_empty() || name.chars().count() > 128 {
            bail!("multica_workspace_label_invalid");
        }
        let kind = object
            .get("resource_type")
            .or_else(|| object.get("resourceType"))
            .and_then(Value::as_str)
            .unwrap_or("issue");
        if !matches!(kind, "issue" | "agent" | "skill") {
            bail!("multica_workspace_label_invalid");
        }
        if let Some(color) = object.get("color").and_then(Value::as_str)
            && !is_hex_color(color)
        {
            bail!("multica_workspace_label_color_invalid");
        }
        return Ok(());
    }
    if resource == MulticaWorkspaceResourceKey::Subscribers {
        for key in ["issue_id", "user_id"] {
            if object.get(key).and_then(Value::as_str).is_none() {
                bail!("multica_workspace_subscriber_invalid");
            }
        }
        let user_type = object
            .get("user_type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(user_type, "member" | "agent") {
            bail!("multica_workspace_subscriber_invalid");
        }
        return Ok(());
    }
    if resource == MulticaWorkspaceResourceKey::Reactions {
        if object.get("actor_id").and_then(Value::as_str).is_none()
            || (object.get("comment_id").and_then(Value::as_str).is_none()
                && object.get("issue_id").and_then(Value::as_str).is_none())
        {
            bail!("multica_workspace_reaction_invalid");
        }
        let emoji = object
            .get("emoji")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if emoji.trim().is_empty() || emoji.chars().count() > 32 {
            bail!("multica_workspace_reaction_invalid");
        }
        return Ok(());
    }
    if resource == MulticaWorkspaceResourceKey::Autopilots {
        let status = object
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("active");
        if !matches!(status, "active" | "paused" | "archived") {
            bail!("multica_workspace_autopilot_status_invalid");
        }
        if let Some(mode) = object.get("execution_mode").and_then(Value::as_str)
            && !matches!(mode, "create_issue" | "run_only")
        {
            bail!("multica_workspace_autopilot_execution_mode_invalid");
        }
        if let Some(kind) = object.get("assignee_type").and_then(Value::as_str)
            && !matches!(kind, "agent" | "squad")
        {
            bail!("multica_workspace_autopilot_assignee_type_invalid");
        }
        if let Some(triggers) = object.get("triggers") {
            for trigger in triggers.as_array().expect("validated array") {
                let Some(trigger) = trigger.as_object() else {
                    bail!("multica_workspace_autopilot_trigger_invalid");
                };
                let kind = trigger
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let trigger_id = trigger
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if trigger_id.trim().is_empty() || validate_local_entity_id(trigger_id).is_err() {
                    bail!("multica_workspace_autopilot_trigger_invalid");
                }
                if !matches!(kind, "schedule" | "webhook" | "api") {
                    bail!("multica_workspace_autopilot_trigger_invalid");
                }
                if trigger.get("enabled").and_then(Value::as_bool).is_none() {
                    bail!("multica_workspace_autopilot_trigger_invalid");
                }
                if kind == "schedule"
                    && trigger
                        .get("cron_expression")
                        .and_then(Value::as_str)
                        .is_none()
                {
                    bail!("multica_workspace_autopilot_trigger_invalid");
                }
            }
        }
        if let Some(collaborators) = object.get("collaborators") {
            for collaborator in collaborators.as_array().expect("validated array") {
                let Some(collaborator) = collaborator.as_object() else {
                    bail!("multica_workspace_autopilot_collaborator_invalid");
                };
                let user_id = collaborator
                    .get("user_id")
                    .or_else(|| collaborator.get("userId"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let role = collaborator
                    .get("role")
                    .and_then(Value::as_str)
                    .unwrap_or("collaborator");
                if user_id.trim().is_empty()
                    || validate_local_entity_id(user_id).is_err()
                    || !matches!(role, "collaborator" | "owner")
                {
                    bail!("multica_workspace_autopilot_collaborator_invalid");
                }
            }
        }
        if let Some(runs) = object.get("runs") {
            for run in runs.as_array().expect("validated array") {
                let Some(run) = run.as_object() else {
                    bail!("multica_workspace_autopilot_run_invalid");
                };
                let status = run
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !matches!(
                    status,
                    "issue_created"
                        | "running"
                        | "completed"
                        | "failed"
                        | "skipped"
                        | "unsupported"
                ) {
                    bail!("multica_workspace_autopilot_run_invalid");
                }
            }
        }
        return Ok(());
    }
    if resource != MulticaWorkspaceResourceKey::Projects {
        return Ok(());
    }
    let Some(resources) = object.get("resources") else {
        return Ok(());
    };
    for resource in resources.as_array().expect("validated array") {
        let Some(resource) = resource.as_object() else {
            bail!("multica_workspace_project_resource_invalid");
        };
        let resource_type = resource
            .get("resource_type")
            .or_else(|| resource.get("resourceType"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let reference = resource
            .get("resource_ref")
            .or_else(|| resource.get("resourceRef"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("multica_workspace_project_resource_invalid"))?;
        match resource_type {
            "github_repo" => {
                let url = reference
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !is_supported_git_url(url) {
                    bail!("multica_workspace_project_resource_invalid");
                }
            }
            "local_directory" => {
                if reference
                    .get("local_path")
                    .and_then(Value::as_str)
                    .is_none_or(|path| path.trim().is_empty() || path.len() > 1024)
                    || reference
                        .get("daemon_id")
                        .and_then(Value::as_str)
                        .is_none_or(|id| id.trim().is_empty() || id.len() > 240)
                {
                    bail!("multica_workspace_project_resource_invalid");
                }
                if let Some(mode) = reference.get("execution_mode").and_then(Value::as_str)
                    && !matches!(mode, "in_place" | "worktree")
                {
                    bail!("multica_workspace_project_resource_invalid");
                }
            }
            _ => bail!("multica_workspace_project_resource_invalid"),
        }
    }
    Ok(())
}

fn is_supported_git_url(url: &str) -> bool {
    let value = url.trim();
    (value.starts_with("https://") || value.starts_with("http://") || value.starts_with("ssh://"))
        && value.len() <= 2048
        && !value.chars().any(char::is_whitespace)
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
                "comments",
                "labels",
                "subscribers",
                "reactions",
                "activities",
                "projects",
                "project_resources",
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
    fn project_resource_contract_matches_upstream_shapes() {
        let valid = json!({
            "id": "project-1",
            "workspace_id": "local-test",
            "revision": 1,
            "resources": [{
                "id": "resource-1",
                "resource_type": "github_repo",
                "resource_ref": {"url": "https://github.com/multica-ai/multica", "ref": "main"}
            }, {
                "id": "resource-2",
                "resource_type": "local_directory",
                "resource_ref": {"local_path": "D:/work", "daemon_id": "daemon-1", "execution_mode": "worktree"}
            }]
        });
        validate_local_entity(&valid, "local-test", MulticaWorkspaceResourceKey::Projects).unwrap();

        let invalid_type = json!({
            "id": "project-1", "workspace_id": "local-test", "revision": 1,
            "resources": [{"resource_type": "s3_bucket", "resource_ref": {}}]
        });
        assert!(
            validate_local_entity(
                &invalid_type,
                "local-test",
                MulticaWorkspaceResourceKey::Projects
            )
            .is_err()
        );

        let invalid_mode = json!({
            "id": "project-1", "workspace_id": "local-test", "revision": 1,
            "resources": [{"resource_type": "local_directory", "resource_ref": {
                "local_path": "D:/work", "daemon_id": "daemon-1", "execution_mode": "parallel"
            }}]
        });
        assert!(
            validate_local_entity(
                &invalid_mode,
                "local-test",
                MulticaWorkspaceResourceKey::Projects
            )
            .is_err()
        );

        let standalone = json!({
            "id": "resource-1",
            "workspace_id": "local-test",
            "revision": 1,
            "project_id": "project-1",
            "resource_type": "github_repo",
            "resource_ref": {"url": "ssh://git@github.com/multica-ai/multica.git", "ref": "main"},
            "label": "Upstream"
        });
        validate_local_entity(
            &standalone,
            "local-test",
            MulticaWorkspaceResourceKey::ProjectResources,
        )
        .unwrap();
    }

    #[test]
    fn label_id_relations_are_bounded_and_string_typed() {
        let valid = json!({
            "id": "issue-1", "workspace_id": "local-test", "revision": 1,
            "label_ids": ["label-1", "label-2"]
        });
        validate_local_entity(&valid, "local-test", MulticaWorkspaceResourceKey::Issues).unwrap();

        for value in [
            json!({"id":"issue-1","workspace_id":"local-test","revision":1,"label_ids":"label-1"}),
            json!({"id":"issue-1","workspace_id":"local-test","revision":1,"label_ids":[1]}),
            json!({"id":"issue-1","workspace_id":"local-test","revision":1,"label_ids":["bad id"]}),
        ] {
            assert_eq!(
                validate_local_entity(&value, "local-test", MulticaWorkspaceResourceKey::Issues)
                    .unwrap_err()
                    .to_string(),
                "multica_workspace_label_ids_invalid"
            );
        }
    }

    #[test]
    fn activities_are_read_only_for_generic_store_mutations() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
        let result = store.upsert(
            "local-test",
            LocalWorkspaceEntityUpsert {
                resource: MulticaWorkspaceResourceKey::Activities,
                entity: json!({"id":"activity-1"}),
                expected_revision: Some(0),
            },
            1,
        );
        assert_eq!(
            result.unwrap_err().to_string(),
            "multica_workspace_resource_read_only"
        );
    }

    #[test]
    fn autopilot_trigger_and_collaborator_shapes_are_validated() {
        let valid = json!({
            "id":"autopilot-1", "workspace_id":"local-test", "revision":1,
            "triggers":[{"id":"trigger-1","kind":"api","enabled":true}],
            "collaborators":[{"user_id":"user-1","role":"collaborator"}]
        });
        validate_local_entity(
            &valid,
            "local-test",
            MulticaWorkspaceResourceKey::Autopilots,
        )
        .unwrap();
        let bad_trigger = json!({
            "id":"autopilot-1", "workspace_id":"local-test", "revision":1,
            "triggers":[{"id":"","kind":"api","enabled":true}]
        });
        assert!(
            validate_local_entity(
                &bad_trigger,
                "local-test",
                MulticaWorkspaceResourceKey::Autopilots
            )
            .is_err()
        );
        let bad_collaborator = json!({
            "id":"autopilot-1", "workspace_id":"local-test", "revision":1,
            "collaborators":[{"user_id":"user-1","role":"admin"}]
        });
        assert!(
            validate_local_entity(
                &bad_collaborator,
                "local-test",
                MulticaWorkspaceResourceKey::Autopilots
            )
            .is_err()
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
            skills_inventory_supported: true,
            skill_protocol: Some("agent-skill-v1".to_string()),
            subagents_supported: true,
        };
        let summary = runtime_summary_from_capabilities(&capabilities);
        assert!(summary.available);
        assert!(summary.skills_supported);
        assert!(summary.skills_inventory_supported);
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
            true,
        );
        let encoded = serde_json::to_string(&item).unwrap();
        assert!(!encoded.contains("C:\\\\Users"));
        assert!(!encoded.contains("\"scope\""));
        assert!(encoded.contains("\"inventory_source\":\"codex_page_host\""));
        assert!(encoded.contains("\"execution_supported\":true"));
    }

    #[test]
    fn inventory_only_skill_projection_is_not_dispatchable() {
        let skill = CodexSkill {
            id: "skill:review".to_string(),
            name: "Review".to_string(),
            summary: None,
            scope: None,
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
            false,
        );
        assert_eq!(item["execution_supported"], false);
        assert_eq!(item["compatible"], false);
        assert_eq!(item["dispatch_allowed"], false);
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

    #[test]
    fn project_path_match_is_case_and_separator_insensitive() {
        let roots = vec![r"C:\Work\Repo".to_string()];
        assert_eq!(
            longest_project_path_match("c:/work/repo/src", &roots),
            Some(r"C:\Work\Repo".to_string())
        );
    }

    #[test]
    fn project_path_match_prefers_the_longest_nested_root() {
        let roots = vec![r"C:\Work".to_string(), r"C:\Work\Repo".to_string()];
        assert_eq!(
            longest_project_path_match(r"C:\Work\Repo\src", &roots),
            Some(r"C:\Work\Repo".to_string())
        );
    }

    #[test]
    fn project_path_match_rejects_prefix_without_directory_boundary() {
        let roots = vec![r"C:\Work\Repo".to_string()];
        assert_eq!(
            longest_project_path_match(r"C:\Work\Repository", &roots),
            None
        );
    }

    #[test]
    fn issue_projection_merges_only_matching_collaboration_entities() {
        let mut issues = vec![
            json!({"id":"issue-a","workspace_id":"local-test","revision":1,"label_ids":["label-b"]}),
        ];
        let mut state = LocalMulticaWorkspaceState::empty("local-test");
        state.comments.push(json!({"id":"comment-a","issue_id":"issue-a","created_at":"2026-01-02T00:00:00Z","content":"ok"}));
        state.comments.push(
            json!({"id":"comment-b","issue_id":"issue-other","created_at":"2026-01-03T00:00:00Z"}),
        );
        state.activities.push(
            json!({"id":"activity-a","issue_id":"issue-a","created_at":"2026-01-04T00:00:00Z"}),
        );
        state
            .reactions
            .push(json!({"id":"reaction-a","comment_id":"comment-a","emoji":"+1"}));
        state
            .reactions
            .push(json!({"id":"reaction-b","comment_id":"comment-b","emoji":"-1"}));
        state.reactions.push(
            json!({"id":"reaction-issue","issue_id":"issue-a","actor_id":"actor","emoji":"heart"}),
        );
        state
            .labels
            .push(json!({"id":"label-a","issue_id":"issue-a","name":"bug"}));
        state.labels.push(json!({"id":"label-b","name":"feature"}));
        project_issue_collaboration(&mut issues, &state);
        assert_eq!(issues[0]["comment_count"], 1);
        assert_eq!(issues[0]["activity_count"], 1);
        assert_eq!(issues[0]["labels"].as_array().map(Vec::len), Some(2));
        assert_eq!(issues[0]["reactions"].as_array().map(Vec::len), Some(2));
        assert_eq!(issues[0]["last_activity_at"], "2026-01-04T00:00:00Z");
        let timeline = issues[0]["timeline"].as_array().expect("timeline");
        assert_eq!(timeline.len(), 2);
        assert_eq!(timeline[0]["type"], "comment");
        assert_eq!(timeline[1]["type"], "activity");
        assert_eq!(issues[0]["revision"], 1);
    }

    #[test]
    fn autopilot_projection_derives_list_summary_from_detail_rows() {
        let mut autopilots = vec![json!({
            "id": "ap-1",
            "triggers": [
                {"kind":"webhook","enabled":true},
                {"kind":"schedule","enabled":true,"next_run_at":"2026-02-02T00:00:00Z"},
                {"kind":"schedule","enabled":false,"next_run_at":"2026-01-01T00:00:00Z"}
            ],
            "runs": [
                {"status":"completed","triggered_at":"2026-01-01T00:00:00Z"},
                {"status":"failed","triggered_at":"2026-01-03T00:00:00Z"}
            ]
        })];
        project_autopilot_contract(&mut autopilots);
        assert_eq!(
            autopilots[0]["trigger_kinds"],
            json!(["schedule", "webhook"])
        );
        assert_eq!(autopilots[0]["next_run_at"], "2026-02-02T00:00:00Z");
        assert_eq!(autopilots[0]["last_run_status"], "failed");
        assert_eq!(autopilots[0]["subscribers"], json!([]));
    }

    #[test]
    fn autopilot_contract_rejects_unknown_trigger_and_run_status() {
        let workspace = "local-test";
        let mut state = LocalMulticaWorkspaceState::empty(workspace);
        state.autopilots.push(json!({
            "id":"ap-1", "workspace_id":workspace, "revision":1,
            "triggers":[{"kind":"timer","enabled":true}], "runs":[]
        }));
        let error = validate_local_workspace_state(&state).unwrap_err();
        assert_eq!(
            error.to_string(),
            "multica_workspace_autopilot_trigger_invalid"
        );

        state.autopilots[0]["triggers"] = json!([]);
        state.autopilots[0]["runs"] = json!([{"status":"done"}]);
        let error = validate_local_workspace_state(&state).unwrap_err();
        assert_eq!(error.to_string(), "multica_workspace_autopilot_run_invalid");
    }

    #[test]
    fn comment_parent_must_exist_on_same_issue() {
        let mut state = LocalMulticaWorkspaceState::empty("local-test");
        state
            .issues
            .push(json!({"id":"issue-a","workspace_id":"local-test","revision":1,"title":"A"}));
        state
            .issues
            .push(json!({"id":"issue-b","workspace_id":"local-test","revision":1,"title":"B"}));
        state.comments.push(json!({"id":"comment-a","workspace_id":"local-test","revision":1,"issue_id":"issue-a","content":"root","parent_id":"missing"}));
        assert_eq!(
            validate_local_workspace_state(&state)
                .unwrap_err()
                .to_string(),
            "multica_workspace_comment_parent_missing"
        );
        state.comments[0]["parent_id"] = Value::String("comment-b".into());
        state.comments.push(
            json!({"id":"comment-b","workspace_id":"local-test","revision":1,"issue_id":"issue-b","content":"other"}),
        );
        assert_eq!(
            validate_local_workspace_state(&state)
                .unwrap_err()
                .to_string(),
            "multica_workspace_comment_parent_issue_mismatch"
        );
    }
}
