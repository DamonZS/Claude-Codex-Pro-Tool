//! Narrow adapter for the request client owned by the current Codex page.
//!
//! The production transport invokes the already-open renderer through CDP.
//! It never starts or registers another runtime, reads provider settings,
//! rewrites URLs, or accepts arbitrary request method names.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use crate::multica_execution::{CodexSkillExecutionRequest, SkillReference};

const MAX_ID_LENGTH: usize = 240;
const MAX_PROMPT_LENGTH: usize = 32 * 1024;
const MAX_SKILLS: usize = 256;
const MAX_SKILL_PATH_LENGTH: usize = 4096;
const MAX_CAPABILITIES: usize = 128;
const MAX_PAGE_HOST_PARAMS_BYTES: usize = 256 * 1024;

/// Methods used through the current Codex page's request client. Keep this
/// enum closed so the renderer cannot turn the adapter into an arbitrary
/// request proxy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodexPageHostMethod {
    Initialize,
    SkillsList,
    ThreadStart,
    ThreadRead,
    TurnStart,
    TurnInterrupt,
    ThreadFork,
}

impl CodexPageHostMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::SkillsList => "skills/list",
            Self::ThreadStart => "thread/start",
            Self::ThreadRead => "thread/read",
            Self::TurnStart => "turn/start",
            Self::TurnInterrupt => "turn/interrupt",
            Self::ThreadFork => "thread/fork",
        }
    }
}

/// A typed request handed to the current page host transport.
#[derive(Debug, Clone, PartialEq)]
pub struct CodexPageHostRequest {
    pub id: u64,
    pub method: CodexPageHostMethod,
    pub params: Value,
}

/// Identity of the current Codex page host. Provider is fixed before any
/// request is sent, so another model runtime cannot be selected here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexRuntimeBinding {
    pub runtime_id: String,
    pub provider: String,
    #[serde(default)]
    pub app_server_version: Option<String>,
    /// Reserved for persisted compatibility metadata. Production capability
    /// negotiation never trusts this field; only the live page host response
    /// can enable an execution feature.
    #[serde(default)]
    pub declared_capabilities: Vec<String>,
}

impl CodexRuntimeBinding {
    pub fn validate(&self) -> anyhow::Result<()> {
        validate_id(&self.runtime_id, "runtime_id")?;
        if !self.provider.eq_ignore_ascii_case("codex") {
            bail!("runtime_provider_mismatch");
        }
        if let Some(version) = self.app_server_version.as_deref() {
            validate_text(version, MAX_ID_LENGTH, "runtime_version")?;
        }
        if self.declared_capabilities.len() > MAX_CAPABILITIES {
            bail!("runtime_capabilities_invalid");
        }
        for capability in &self.declared_capabilities {
            if !is_capability(capability) {
                bail!("runtime_capabilities_invalid");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRuntimeCapabilities {
    pub runtime_id: String,
    pub provider: String,
    #[serde(default)]
    pub protocol_version: Option<String>,
    #[serde(default)]
    pub server_version: Option<String>,
    pub capabilities: Vec<String>,
    pub skills_supported: bool,
    /// The current page can enumerate its Skills, even when it does not
    /// advertise a protocol that permits dispatching them in native turns.
    #[serde(default)]
    pub skills_inventory_supported: bool,
    pub skill_protocol: Option<String>,
    pub subagents_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSkill {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    pub enabled: bool,
    /// The runtime may provide an immutable digest.  A local path is never
    /// copied into this DTO.
    #[serde(default)]
    pub manifest_digest: Option<String>,
}

/// The native `skills/list` metadata needed to build a Codex
/// `UserInput::Skill`. The path stays private to Core: it is read from the
/// already-authoritative current-page inventory and never crosses the bridge or
/// gets serialized in a renderer-facing DTO.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeCodexSkill {
    skill: CodexSkill,
    path: String,
}

/// A Skill item accepted by Codex's native `turn/start` input protocol. The
/// wire type is always `skill`; callers provide only the validated metadata
/// returned by `skills/list`, never an arbitrary path from the renderer.
fn native_skill_input(skill: &NativeCodexSkill) -> Value {
    json!({
        "type": "skill",
        "name": skill.skill.name,
        "path": skill.path,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexThreadRequest {
    pub workspace_id: String,
    pub issue_id: String,
    pub prompt: String,
    #[serde(default)]
    pub cwd: Option<String>,
    /// Resolved, digest-pinned Skills selected for this native execution.
    /// The request carries references only; Skill contents and local paths
    /// never cross the page-host boundary.
    #[serde(default)]
    pub skill_request: Option<CodexSkillExecutionRequest>,
}

impl CodexThreadRequest {
    pub fn validate(&self) -> anyhow::Result<()> {
        validate_id(&self.workspace_id, "workspace_id")?;
        validate_id(&self.issue_id, "issue_id")?;
        validate_text(&self.prompt, MAX_PROMPT_LENGTH, "prompt")?;
        if let Some(cwd) = self.cwd.as_deref() {
            validate_path(cwd)?;
        }
        if let Some(skill_request) = self.skill_request.as_ref() {
            validate_skill_execution_request(skill_request)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexExecutionHandle {
    pub runtime_id: String,
    pub thread_id: String,
    #[serde(default)]
    pub execution_id: Option<String>,
    #[serde(default)]
    pub parent_thread_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexExecutionState {
    Unknown,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    CancelPending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexExecutionStatus {
    pub runtime_id: String,
    pub thread_id: String,
    pub execution_id: String,
    pub state: CodexExecutionState,
    #[serde(default)]
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexExecutionEvent {
    pub event_id: String,
    pub cursor: String,
    pub thread_id: String,
    #[serde(default)]
    pub execution_id: Option<String>,
    pub state: CodexExecutionState,
}

/// The transport is deliberately injected and only carries one allowlisted
/// request to the current Codex page at a time.
#[async_trait]
pub trait CodexPageHostRequestTransport: Send + Sync {
    fn generation(&self) -> u64 {
        0
    }

    async fn request(&self, request: CodexPageHostRequest) -> anyhow::Result<Value>;
}

/// Transport backed by the request client already owned by the current Codex
/// renderer page. It talks to that page through CDP and never starts,
/// registers, or manages another Codex runtime or execution process.
#[derive(Clone, Default)]
pub struct CodexPageHostTransport {
    websocket_url: Arc<Mutex<Option<String>>>,
    generation: Arc<AtomicU64>,
}

impl CodexPageHostTransport {
    pub fn new(websocket_url: Arc<Mutex<Option<String>>>) -> Self {
        Self {
            websocket_url,
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn websocket_state(&self) -> Arc<Mutex<Option<String>>> {
        Arc::clone(&self.websocket_url)
    }

    pub fn set_websocket_url(&self, websocket_url: impl Into<String>) -> anyhow::Result<()> {
        let websocket_url = websocket_url.into();
        if websocket_url.trim().is_empty() {
            bail!("codex_page_host_unavailable");
        }
        let mut current = self
            .websocket_url
            .lock()
            .map_err(|_| anyhow!("codex_page_host_unavailable"))?;
        // A renderer reload/reinjection can preserve the same CDP target URL
        // while replacing the page-owned request client. Every successful
        // binding therefore starts a new generation, even when the URL text
        // itself is unchanged.
        *current = Some(websocket_url);
        self.generation.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    async fn page_request(
        &self,
        method: CodexPageHostMethod,
        params: &Value,
    ) -> anyhow::Result<Value> {
        let websocket_url = self
            .websocket_url
            .lock()
            .map_err(|_| anyhow!("codex_page_host_unavailable"))?
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("codex_page_host_unavailable"))?;
        let method = serde_json::to_string(method.as_str())
            .map_err(|_| anyhow!("codex_page_host_request_invalid"))?;
        let params = serde_json::to_string(params)
            .map_err(|_| anyhow!("codex_page_host_request_invalid"))?;
        if params.len() > MAX_PAGE_HOST_PARAMS_BYTES {
            bail!("codex_page_host_params_too_large");
        }
        let script = format!(
            r#"(async () => {{
              const request = window.__claudeCodexProCodexPageHostRequest;
              if (typeof request !== "function") throw new Error("codex_page_host_unavailable");
              return await request({method}, {params});
            }})()"#
        );
        let response =
            crate::bridge::evaluate_script_with_await_promise(&websocket_url, &script, true)
                .await
                .map_err(|error| {
                    anyhow!(
                        "codex_page_host_request_failed: {}",
                        bounded_host_error(&error.to_string())
                    )
                })?;
        if let Some(details) = response.pointer("/result/exceptionDetails") {
            bail!(
                "codex_page_host_request_failed: {}",
                exception_detail_message(details)
            );
        }
        response
            .pointer("/result/result/value")
            .cloned()
            .ok_or_else(|| anyhow!("codex_page_host_response_invalid"))
    }
}

const MAX_HOST_ERROR_LENGTH: usize = 256;

fn bounded_host_error(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(MAX_HOST_ERROR_LENGTH).collect()
}

fn exception_detail_message(details: &Value) -> String {
    let candidates = [
        details.pointer("/exception/description"),
        details.pointer("/exception/value"),
        details.get("text"),
    ];
    candidates
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(bounded_host_error)
        .find(|value| !value.is_empty())
        .unwrap_or_else(|| "runtime_exception".to_string())
}

#[async_trait]
impl CodexPageHostRequestTransport for CodexPageHostTransport {
    fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    async fn request(&self, request: CodexPageHostRequest) -> anyhow::Result<Value> {
        self.page_request(request.method, &request.params).await
    }
}

/// Build an execution service over the current page's existing Host API.
/// Callers keep the returned transport handle only to update the CDP target
/// after a Codex renderer reload.
pub fn codex_page_execution_service(
    websocket_url: Arc<Mutex<Option<String>>>,
) -> anyhow::Result<(Arc<CodexPageExecutionClient>, CodexPageHostTransport)> {
    let transport = CodexPageHostTransport::new(websocket_url);
    let client = CodexPageExecutionClient::new(
        transport.clone(),
        CodexRuntimeBinding {
            runtime_id: "codex-current-page".to_string(),
            provider: "codex".to_string(),
            app_server_version: None,
            declared_capabilities: Vec::new(),
        },
    )?;
    Ok((Arc::new(client), transport))
}

#[async_trait]
pub trait CodexExecutionService: Send + Sync {
    async fn capabilities(&self) -> anyhow::Result<CodexRuntimeCapabilities>;
    async fn list_skills(&self) -> anyhow::Result<Vec<CodexSkill>>;
    async fn resolve_skills(
        &self,
        request: CodexSkillExecutionRequest,
    ) -> anyhow::Result<CodexSkillExecutionRequest>;
    async fn create_thread(
        &self,
        request: CodexThreadRequest,
        idempotency_key: &str,
    ) -> anyhow::Result<CodexExecutionHandle>;
    async fn create_subagent(
        &self,
        parent_thread_id: &str,
        request: CodexThreadRequest,
        idempotency_key: &str,
    ) -> anyhow::Result<CodexExecutionHandle>;
    async fn open_thread(&self, thread_id: &str) -> anyhow::Result<CodexExecutionHandle>;
    async fn continue_thread(
        &self,
        thread_id: &str,
        request: CodexThreadRequest,
        idempotency_key: &str,
    ) -> anyhow::Result<CodexExecutionHandle>;
    async fn cancel_execution(
        &self,
        thread_id: &str,
        execution_id: &str,
    ) -> anyhow::Result<CodexExecutionStatus>;
    async fn execution_status(
        &self,
        thread_id: &str,
        execution_id: &str,
    ) -> anyhow::Result<CodexExecutionStatus>;
    /// Read the Skill inputs persisted by Codex for one turn. The returned
    /// references are stable IDs/digests only; native filesystem paths never
    /// leave the Core adapter.
    async fn execution_loaded_skills(
        &self,
        _thread_id: &str,
        _execution_id: &str,
    ) -> anyhow::Result<Vec<SkillReference>> {
        bail!("unsupported");
    }
    async fn subscribe_events(
        &self,
        _cursor: Option<&str>,
    ) -> anyhow::Result<Vec<CodexExecutionEvent>> {
        bail!("unsupported");
    }
}

/// Production adapter. It talks only to the current page-host transport and
/// keeps small process-local idempotency records.
pub struct CodexPageExecutionClient {
    transport: Arc<dyn CodexPageHostRequestTransport>,
    binding: CodexRuntimeBinding,
    next_request_id: AtomicU64,
    observed_generation: Mutex<u64>,
    capabilities: Mutex<Option<(u64, CodexRuntimeCapabilities)>>,
    capabilities_lock: AsyncMutex<()>,
    idempotency: Mutex<HashMap<String, CodexExecutionHandle>>,
    partial_threads: Mutex<HashMap<String, PartialThreadRecord>>,
    idempotency_locks: AsyncMutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PartialThreadRecord {
    generation: u64,
    thread_id: String,
    parent_thread_id: Option<String>,
}

impl CodexPageExecutionClient {
    pub fn new<T>(transport: T, binding: CodexRuntimeBinding) -> anyhow::Result<Self>
    where
        T: CodexPageHostRequestTransport + 'static,
    {
        binding.validate()?;
        let observed_generation = transport.generation();
        Ok(Self {
            transport: Arc::new(transport),
            binding,
            next_request_id: AtomicU64::new(1),
            observed_generation: Mutex::new(observed_generation),
            capabilities: Mutex::new(None),
            capabilities_lock: AsyncMutex::new(()),
            idempotency: Mutex::new(HashMap::new()),
            partial_threads: Mutex::new(HashMap::new()),
            idempotency_locks: AsyncMutex::new(HashMap::new()),
        })
    }

    pub fn from_arc(
        transport: Arc<dyn CodexPageHostRequestTransport>,
        binding: CodexRuntimeBinding,
    ) -> anyhow::Result<Self> {
        binding.validate()?;
        let observed_generation = transport.generation();
        Ok(Self {
            transport,
            binding,
            next_request_id: AtomicU64::new(1),
            observed_generation: Mutex::new(observed_generation),
            capabilities: Mutex::new(None),
            capabilities_lock: AsyncMutex::new(()),
            idempotency: Mutex::new(HashMap::new()),
            partial_threads: Mutex::new(HashMap::new()),
            idempotency_locks: AsyncMutex::new(HashMap::new()),
        })
    }

    fn next_id(&self) -> u64 {
        self.next_request_id.fetch_add(1, Ordering::Relaxed)
    }

    fn sync_generation(&self) -> anyhow::Result<u64> {
        let current = self.transport.generation();
        let mut observed = self
            .observed_generation
            .lock()
            .map_err(|_| anyhow!("codex_execution_state_unavailable"))?;
        if *observed != current {
            *self
                .capabilities
                .lock()
                .map_err(|_| anyhow!("codex_execution_state_unavailable"))? = None;
            self.idempotency
                .lock()
                .map_err(|_| anyhow!("codex_execution_state_unavailable"))?
                .clear();
            self.partial_threads
                .lock()
                .map_err(|_| anyhow!("codex_execution_state_unavailable"))?
                .clear();
            *observed = current;
        }
        Ok(current)
    }

    fn ensure_generation(&self, expected: u64) -> anyhow::Result<()> {
        if self.sync_generation()? != expected {
            bail!("codex_page_host_generation_changed");
        }
        Ok(())
    }

    async fn request_host_at(
        &self,
        generation: u64,
        method: CodexPageHostMethod,
        params: Value,
    ) -> anyhow::Result<Value> {
        self.ensure_generation(generation)?;
        let params_bytes =
            serde_json::to_vec(&params).map_err(|_| anyhow!("codex_page_host_params_invalid"))?;
        if params_bytes.len() > MAX_PAGE_HOST_PARAMS_BYTES {
            bail!("codex_page_host_params_too_large");
        }
        let response = self
            .transport
            .request(CodexPageHostRequest {
                id: self.next_id(),
                method,
                params,
            })
            .await?;
        self.ensure_generation(generation)?;
        Ok(response)
    }

    fn cached_capabilities(&self, generation: u64) -> Option<CodexRuntimeCapabilities> {
        self.capabilities
            .lock()
            .ok()
            .and_then(|value| value.clone())
            .and_then(|(cached_generation, capabilities)| {
                (cached_generation == generation).then_some(capabilities)
            })
    }

    fn set_capabilities(&self, generation: u64, value: CodexRuntimeCapabilities) {
        if let Ok(mut cached) = self.capabilities.lock() {
            *cached = Some((generation, value));
        }
    }

    async fn capabilities_for_generation(
        &self,
        generation: u64,
    ) -> anyhow::Result<CodexRuntimeCapabilities> {
        self.ensure_generation(generation)?;
        if let Some(cached) = self.cached_capabilities(generation) {
            return Ok(cached);
        }
        let _guard = self.capabilities_lock.lock().await;
        self.ensure_generation(generation)?;
        if let Some(cached) = self.cached_capabilities(generation) {
            return Ok(cached);
        }
        let response = self
            .request_host_at(
                generation,
                CodexPageHostMethod::Initialize,
                json!({
                    "clientInfo": {
                        "name": "claude-codex-pro-tool",
                        "version": crate::version::VERSION
                    }
                }),
            )
            .await
            .map_err(|error| {
                let _ = crate::diagnostic_log::append_diagnostic_log(
                    "codex_runtime_capabilities_failed",
                    json!({ "method": "initialize", "error": error.to_string() }),
                );
                error
            })?;
        let capabilities = parse_capabilities(&self.binding, &response)?;
        self.ensure_generation(generation)?;
        self.set_capabilities(generation, capabilities.clone());
        Ok(capabilities)
    }

    fn idempotency_record_key(namespace: &str, key: &str) -> anyhow::Result<String> {
        validate_idempotency_key(key)?;
        Ok(format!("{namespace}:{key}"))
    }

    async fn idempotency_guard(
        &self,
        namespace: &str,
        key: &str,
    ) -> anyhow::Result<OwnedMutexGuard<()>> {
        let key = Self::idempotency_record_key(namespace, key)?;
        let lock = {
            let mut locks = self.idempotency_locks.lock().await;
            Arc::clone(
                locks
                    .entry(key)
                    .or_insert_with(|| Arc::new(AsyncMutex::new(()))),
            )
        };
        Ok(lock.lock_owned().await)
    }

    fn idempotent(
        &self,
        namespace: &str,
        key: &str,
    ) -> anyhow::Result<Option<CodexExecutionHandle>> {
        let key = Self::idempotency_record_key(namespace, key)?;
        self.idempotency
            .lock()
            .map(|records| records.get(&key).cloned())
            .map_err(|_| anyhow!("codex_execution_state_unavailable"))
    }

    fn remember_idempotency(
        &self,
        namespace: &str,
        key: &str,
        value: CodexExecutionHandle,
    ) -> anyhow::Result<()> {
        let key = Self::idempotency_record_key(namespace, key)?;
        self.idempotency
            .lock()
            .map_err(|_| anyhow!("codex_execution_state_unavailable"))?
            .insert(key.clone(), value);
        self.partial_threads
            .lock()
            .map_err(|_| anyhow!("codex_execution_state_unavailable"))?
            .remove(&key);
        Ok(())
    }

    fn partial_thread(
        &self,
        namespace: &str,
        key: &str,
        generation: u64,
    ) -> anyhow::Result<Option<PartialThreadRecord>> {
        let key = Self::idempotency_record_key(namespace, key)?;
        self.partial_threads
            .lock()
            .map(|records| {
                records
                    .get(&key)
                    .filter(|record| record.generation == generation)
                    .cloned()
            })
            .map_err(|_| anyhow!("codex_execution_state_unavailable"))
    }

    fn remember_partial_thread(
        &self,
        namespace: &str,
        key: &str,
        value: PartialThreadRecord,
    ) -> anyhow::Result<()> {
        let key = Self::idempotency_record_key(namespace, key)?;
        self.partial_threads
            .lock()
            .map_err(|_| anyhow!("codex_execution_state_unavailable"))?
            .insert(key, value);
        Ok(())
    }

    async fn ensure_subagent_capability_at(&self, generation: u64) -> anyhow::Result<()> {
        let capabilities = self.capabilities_for_generation(generation).await?;
        if !capabilities.subagents_supported {
            bail!("unsupported");
        }
        Ok(())
    }

    async fn resolve_request_skills_at(
        &self,
        generation: u64,
        request: &CodexThreadRequest,
    ) -> anyhow::Result<Vec<NativeCodexSkill>> {
        let Some(skill_request) = request.skill_request.clone() else {
            return Ok(Vec::new());
        };
        // Resolve against the live Codex inventory immediately before the
        // native create/turn call. This prevents a stale renderer snapshot or
        // a changed manifest from authorizing execution.
        self.resolve_native_skills_at(generation, skill_request, request.cwd.as_deref())
            .await
    }

    async fn resolve_native_skills_at(
        &self,
        generation: u64,
        request: CodexSkillExecutionRequest,
        cwd: Option<&str>,
    ) -> anyhow::Result<Vec<NativeCodexSkill>> {
        validate_skill_execution_request(&request)?;
        if let Some(cwd) = cwd {
            validate_path(cwd)?;
        }
        let capabilities = self.capabilities_for_generation(generation).await?;
        if capabilities.skill_protocol.as_deref() != Some(request.protocol.as_str()) {
            bail!("runtime_skills_unsupported");
        }
        let inventory = self.fetch_native_skills_at(generation, cwd, true).await?;
        resolve_native_skill_refs(&inventory, &request)
    }

    async fn fetch_native_skills_at(
        &self,
        generation: u64,
        cwd: Option<&str>,
        force_reload: bool,
    ) -> anyhow::Result<Vec<NativeCodexSkill>> {
        if let Some(cwd) = cwd {
            validate_path(cwd)?;
        }
        let cwds = cwd.into_iter().collect::<Vec<_>>();
        let response = self
            .request_host_at(
                generation,
                CodexPageHostMethod::SkillsList,
                json!({ "cwds": cwds, "forceReload": force_reload }),
            )
            .await?;
        parse_native_skills(&response)
    }

    async fn start_turn_at(
        &self,
        generation: u64,
        thread_id: &str,
        request: &CodexThreadRequest,
        idempotency_key: &str,
        native_skills: &[NativeCodexSkill],
    ) -> anyhow::Result<String> {
        let response = self
            .request_host_at(
                generation,
                CodexPageHostMethod::TurnStart,
                turn_start_params(thread_id, request, idempotency_key, native_skills),
            )
            .await?;
        extract_execution_id(&response)
    }
}

#[async_trait]
impl CodexExecutionService for CodexPageExecutionClient {
    async fn capabilities(&self) -> anyhow::Result<CodexRuntimeCapabilities> {
        let generation = self.sync_generation()?;
        self.capabilities_for_generation(generation).await
    }

    async fn list_skills(&self) -> anyhow::Result<Vec<CodexSkill>> {
        let generation = self.sync_generation()?;
        let capabilities = self.capabilities_for_generation(generation).await?;
        if !capabilities.skills_inventory_supported {
            bail!("unsupported");
        }
        Ok(self
            .fetch_native_skills_at(generation, None, false)
            .await?
            .into_iter()
            .map(|native| native.skill)
            .collect())
    }

    async fn resolve_skills(
        &self,
        request: CodexSkillExecutionRequest,
    ) -> anyhow::Result<CodexSkillExecutionRequest> {
        let generation = self.sync_generation()?;
        let _ = self
            .resolve_native_skills_at(generation, request.clone(), None)
            .await?;
        Ok(request)
    }

    async fn create_thread(
        &self,
        request: CodexThreadRequest,
        idempotency_key: &str,
    ) -> anyhow::Result<CodexExecutionHandle> {
        request.validate()?;
        let _guard = self.idempotency_guard("create", idempotency_key).await?;
        let generation = self.sync_generation()?;
        if let Some(existing) = self.idempotent("create", idempotency_key)? {
            return Ok(existing);
        }
        self.capabilities_for_generation(generation).await?;
        let native_skills = self.resolve_request_skills_at(generation, &request).await?;
        let thread_id =
            if let Some(partial) = self.partial_thread("create", idempotency_key, generation)? {
                if partial.parent_thread_id.is_some() {
                    bail!("codex_execution_state_unavailable");
                }
                partial.thread_id
            } else {
                let response = self
                    .request_host_at(
                        generation,
                        CodexPageHostMethod::ThreadStart,
                        thread_start_params(&request, &native_skills),
                    )
                    .await?;
                let thread_id = extract_thread_id(&response)?;
                self.ensure_generation(generation)?;
                self.remember_partial_thread(
                    "create",
                    idempotency_key,
                    PartialThreadRecord {
                        generation,
                        thread_id: thread_id.clone(),
                        parent_thread_id: None,
                    },
                )?;
                thread_id
            };
        let execution_id = self
            .start_turn_at(
                generation,
                &thread_id,
                &request,
                idempotency_key,
                &native_skills,
            )
            .await?;
        self.ensure_generation(generation)?;
        let handle = CodexExecutionHandle {
            runtime_id: self.binding.runtime_id.clone(),
            thread_id,
            execution_id: Some(execution_id),
            parent_thread_id: None,
            idempotency_key: idempotency_key.to_string(),
        };
        self.remember_idempotency("create", idempotency_key, handle.clone())?;
        Ok(handle)
    }

    async fn create_subagent(
        &self,
        parent_thread_id: &str,
        request: CodexThreadRequest,
        idempotency_key: &str,
    ) -> anyhow::Result<CodexExecutionHandle> {
        validate_id(parent_thread_id, "parent_thread_id")?;
        request.validate()?;
        let _guard = self.idempotency_guard("subagent", idempotency_key).await?;
        let generation = self.sync_generation()?;
        if let Some(existing) = self.idempotent("subagent", idempotency_key)? {
            return Ok(existing);
        }
        self.ensure_subagent_capability_at(generation).await?;
        let native_skills = self.resolve_request_skills_at(generation, &request).await?;
        // The current Codex page exposes fork as the stable
        // parent/child primitive.  We only use it after the runtime explicitly
        // advertises `subagent-v1`; otherwise this method returns `unsupported`
        // instead of simulating a subagent with another process.
        let thread_id =
            if let Some(partial) = self.partial_thread("subagent", idempotency_key, generation)? {
                if partial.parent_thread_id.as_deref() != Some(parent_thread_id) {
                    bail!("codex_idempotency_conflict");
                }
                partial.thread_id
            } else {
                let response = self
                    .request_host_at(
                        generation,
                        CodexPageHostMethod::ThreadFork,
                        json!({
                            "threadId": parent_thread_id,
                            "threadSource": "multica-subagent",
                            "cwd": request.cwd,
                        }),
                    )
                    .await?;
                let thread_id = extract_thread_id(&response)?;
                self.ensure_generation(generation)?;
                self.remember_partial_thread(
                    "subagent",
                    idempotency_key,
                    PartialThreadRecord {
                        generation,
                        thread_id: thread_id.clone(),
                        parent_thread_id: Some(parent_thread_id.to_string()),
                    },
                )?;
                thread_id
            };
        let execution_id = self
            .start_turn_at(
                generation,
                &thread_id,
                &request,
                idempotency_key,
                &native_skills,
            )
            .await?;
        self.ensure_generation(generation)?;
        let handle = CodexExecutionHandle {
            runtime_id: self.binding.runtime_id.clone(),
            thread_id,
            execution_id: Some(execution_id),
            parent_thread_id: Some(parent_thread_id.to_string()),
            idempotency_key: idempotency_key.to_string(),
        };
        self.remember_idempotency("subagent", idempotency_key, handle.clone())?;
        Ok(handle)
    }

    async fn open_thread(&self, thread_id: &str) -> anyhow::Result<CodexExecutionHandle> {
        validate_id(thread_id, "thread_id")?;
        let generation = self.sync_generation()?;
        self.capabilities_for_generation(generation).await?;
        let response = self
            .request_host_at(
                generation,
                CodexPageHostMethod::ThreadRead,
                json!({ "threadId": thread_id }),
            )
            .await?;
        let returned_id = extract_thread_id(&response)?;
        if returned_id != thread_id {
            bail!("codex_thread_id_mismatch");
        }
        Ok(CodexExecutionHandle {
            runtime_id: self.binding.runtime_id.clone(),
            thread_id: returned_id,
            execution_id: None,
            parent_thread_id: response
                .pointer("/thread/parentThreadId")
                .and_then(Value::as_str)
                .map(str::to_string),
            idempotency_key: format!("open:{thread_id}"),
        })
    }

    async fn continue_thread(
        &self,
        thread_id: &str,
        request: CodexThreadRequest,
        idempotency_key: &str,
    ) -> anyhow::Result<CodexExecutionHandle> {
        validate_id(thread_id, "thread_id")?;
        request.validate()?;
        let _guard = self.idempotency_guard("continue", idempotency_key).await?;
        let generation = self.sync_generation()?;
        if let Some(existing) = self.idempotent("continue", idempotency_key)? {
            return Ok(existing);
        }
        self.capabilities_for_generation(generation).await?;
        let native_skills = self.resolve_request_skills_at(generation, &request).await?;
        let response = self
            .request_host_at(
                generation,
                CodexPageHostMethod::TurnStart,
                turn_start_params(thread_id, &request, idempotency_key, &native_skills),
            )
            .await?;
        let execution_id = extract_execution_id(&response)?;
        self.ensure_generation(generation)?;
        let handle = CodexExecutionHandle {
            runtime_id: self.binding.runtime_id.clone(),
            thread_id: thread_id.to_string(),
            execution_id: Some(execution_id),
            parent_thread_id: None,
            idempotency_key: idempotency_key.to_string(),
        };
        self.remember_idempotency("continue", idempotency_key, handle.clone())?;
        Ok(handle)
    }

    async fn cancel_execution(
        &self,
        thread_id: &str,
        execution_id: &str,
    ) -> anyhow::Result<CodexExecutionStatus> {
        validate_id(thread_id, "thread_id")?;
        validate_id(execution_id, "execution_id")?;
        let generation = self.sync_generation()?;
        self.capabilities_for_generation(generation).await?;
        let response = self
            .request_host_at(
                generation,
                CodexPageHostMethod::TurnInterrupt,
                json!({ "threadId": thread_id, "turnId": execution_id }),
            )
            .await?;
        let state = response
            .pointer("/turn/status")
            .and_then(Value::as_str)
            .map(parse_execution_state)
            .unwrap_or(CodexExecutionState::CancelPending);
        Ok(CodexExecutionStatus {
            runtime_id: self.binding.runtime_id.clone(),
            thread_id: thread_id.to_string(),
            execution_id: execution_id.to_string(),
            state,
            diagnostic: None,
        })
    }

    async fn execution_status(
        &self,
        thread_id: &str,
        execution_id: &str,
    ) -> anyhow::Result<CodexExecutionStatus> {
        validate_id(thread_id, "thread_id")?;
        validate_id(execution_id, "execution_id")?;
        let generation = self.sync_generation()?;
        self.capabilities_for_generation(generation).await?;
        let response = self
            .request_host_at(
                generation,
                CodexPageHostMethod::ThreadRead,
                json!({ "threadId": thread_id }),
            )
            .await?;
        let status = response
            .pointer("/turn/status")
            .or_else(|| response.pointer("/thread/status/type"))
            .and_then(Value::as_str)
            .map(parse_execution_state)
            .unwrap_or(CodexExecutionState::Unknown);
        Ok(CodexExecutionStatus {
            runtime_id: self.binding.runtime_id.clone(),
            thread_id: thread_id.to_string(),
            execution_id: execution_id.to_string(),
            state: status,
            diagnostic: None,
        })
    }
}

/// Alias used by contract tests and callers that need a named fake service.
pub type FakeCodexExecutionService = CodexPageExecutionClient;

/// A deterministic in-memory transport.  It never starts a process and is
/// intended for contract tests of the bridge/execution orchestration.
#[derive(Clone, Default)]
pub struct FakeCodexPageHostTransport {
    calls: Arc<Mutex<Vec<CodexPageHostRequest>>>,
    responses: Arc<Mutex<HashMap<CodexPageHostMethod, VecDeque<Result<Value, String>>>>>,
    next_thread: Arc<AtomicU64>,
    next_turn: Arc<AtomicU64>,
    generation: Arc<AtomicU64>,
    delay_millis: Arc<AtomicU64>,
}

impl FakeCodexPageHostTransport {
    pub fn push_response(&self, method: CodexPageHostMethod, response: anyhow::Result<Value>) {
        let response = response.map_err(|error| error.to_string());
        if let Ok(mut responses) = self.responses.lock() {
            responses.entry(method).or_default().push_back(response);
        }
    }

    pub fn calls(&self) -> Vec<CodexPageHostRequest> {
        self.calls
            .lock()
            .map(|calls| calls.clone())
            .unwrap_or_default()
    }

    pub fn bump_generation(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
    }

    pub fn set_delay_millis(&self, delay_millis: u64) {
        self.delay_millis.store(delay_millis, Ordering::Release);
    }
}

#[async_trait]
impl CodexPageHostRequestTransport for FakeCodexPageHostTransport {
    fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    async fn request(&self, request: CodexPageHostRequest) -> anyhow::Result<Value> {
        let delay_millis = self.delay_millis.load(Ordering::Acquire);
        if delay_millis > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay_millis)).await;
        }
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(request.clone());
        }
        if let Ok(mut responses) = self.responses.lock()
            && let Some(queue) = responses.get_mut(&request.method)
            && let Some(response) = queue.pop_front()
        {
            return response.map_err(anyhow::Error::msg);
        }
        Ok(self.default_response(&request))
    }
}

impl FakeCodexPageHostTransport {
    fn default_response(&self, request: &CodexPageHostRequest) -> Value {
        match request.method {
            CodexPageHostMethod::Initialize => json!({
                "provider": "codex",
                "serverVersion": "fake",
                "capabilities": ["skill-bundles-v1", "subagent-v1"]
            }),
            CodexPageHostMethod::SkillsList => json!({ "data": [] }),
            CodexPageHostMethod::ThreadStart => {
                let id = self.next_thread.fetch_add(1, Ordering::Relaxed);
                json!({ "thread": { "id": format!("thread-fake-{id}") } })
            }
            CodexPageHostMethod::ThreadFork => {
                let id = self.next_thread.fetch_add(1, Ordering::Relaxed);
                json!({ "thread": { "id": format!("subagent-fake-{id}") } })
            }
            CodexPageHostMethod::TurnStart => {
                let id = self.next_turn.fetch_add(1, Ordering::Relaxed);
                json!({ "turn": { "id": format!("turn-fake-{id}"), "status": "inProgress" } })
            }
            CodexPageHostMethod::TurnInterrupt => json!({}),
            CodexPageHostMethod::ThreadRead => {
                let id = request
                    .params
                    .get("threadId")
                    .and_then(Value::as_str)
                    .unwrap_or("thread-fake");
                json!({ "thread": { "id": id, "status": { "type": "idle" } } })
            }
        }
    }
}

fn parse_capabilities(
    binding: &CodexRuntimeBinding,
    response: &Value,
) -> anyhow::Result<CodexRuntimeCapabilities> {
    let provider = response
        .get("provider")
        .or_else(|| response.pointer("/serverInfo/provider"))
        .and_then(Value::as_str)
        .unwrap_or(&binding.provider)
        .to_string();
    if !provider.eq_ignore_ascii_case("codex") {
        bail!("runtime_provider_mismatch");
    }
    let mut capabilities = Vec::new();
    for source in [
        response.get("capabilities"),
        response.get("serverCapabilities"),
    ] {
        if let Some(values) = source.and_then(Value::as_array) {
            for value in values.iter().filter_map(Value::as_str) {
                if is_capability(value) && !capabilities.iter().any(|item| item == value) {
                    capabilities.push(value.to_string());
                }
            }
        }
    }
    if capabilities.len() > MAX_CAPABILITIES {
        bail!("runtime_capabilities_invalid");
    }
    let skill_protocol = if capabilities.iter().any(|value| value == "agent-skill-v1") {
        Some("agent-skill-v1".to_string())
    } else if capabilities.iter().any(|value| value == "skill-bundles-v1") {
        Some("skill-bundles-v1".to_string())
    } else {
        None
    };
    let skills_inventory_supported = skill_protocol.is_some()
        || response
            .pointer("/pageHostProbe/skillsList")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    Ok(CodexRuntimeCapabilities {
        runtime_id: binding.runtime_id.clone(),
        provider,
        protocol_version: response
            .get("protocolVersion")
            .and_then(Value::as_str)
            .map(str::to_string),
        server_version: response
            .get("serverVersion")
            .or_else(|| response.pointer("/serverInfo/version"))
            .and_then(Value::as_str)
            .map(str::to_string),
        skills_supported: skill_protocol.is_some(),
        skills_inventory_supported,
        skill_protocol,
        subagents_supported: capabilities.iter().any(|value| {
            matches!(
                value.as_str(),
                "subagent-v1" | "collab-agent-v1" | "multi-agent-v1"
            )
        }),
        capabilities,
    })
}

fn parse_native_skills(response: &Value) -> anyhow::Result<Vec<NativeCodexSkill>> {
    let entries = response
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("codex_skills_response_invalid"))?;
    let mut skills = Vec::new();
    for entry in entries {
        let Some(values) = entry.get("skills").and_then(Value::as_array) else {
            continue;
        };
        for value in values {
            if skills.len() >= MAX_SKILLS {
                bail!("codex_skills_too_large");
            }
            let name = value
                .get("name")
                .or_else(|| value.get("displayName"))
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("codex_skill_invalid"))?;
            validate_text(name, MAX_ID_LENGTH, "skill_name")?;
            let path = value
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("codex_skill_path_missing"))?;
            validate_skill_path(path)?;
            let id = value
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("codex:{name}"));
            validate_id(&id, "skill_id")?;
            let digest = value
                .get("manifestDigest")
                .or_else(|| value.get("manifest_digest"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| skill_manifest_digest_from_path(path));
            if let Some(digest) = digest.as_deref() {
                validate_digest(digest)?;
            }
            skills.push(NativeCodexSkill {
                skill: CodexSkill {
                    id,
                    name: name.to_string(),
                    summary: value
                        .get("description")
                        .or_else(|| value.get("shortDescription"))
                        .and_then(Value::as_str)
                        .map(|summary| truncate_text(summary, 512)),
                    scope: value
                        .get("scope")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    enabled: value
                        .get("enabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    manifest_digest: digest,
                },
                path: path.to_string(),
            });
        }
    }
    Ok(skills)
}

fn skill_manifest_digest_from_path(path: &str) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > 512 * 1024 {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    Some(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn thread_start_params(request: &CodexThreadRequest, native_skills: &[NativeCodexSkill]) -> Value {
    let mut params = Map::new();
    params.insert("threadSource".to_string(), json!("multica"));
    if let Some(cwd) = request.cwd.as_deref() {
        params.insert("cwd".to_string(), json!(cwd));
    }
    if let Some(skill_request) = request.skill_request.as_ref() {
        // Keep the thread-level metadata path-only-free.  The actual native
        // `type=skill` inputs are added to turn/start; this block is the
        // immutable protocol envelope used by the page host for
        // auditing and capability negotiation.
        let skill_refs = native_skills
            .iter()
            .map(|skill| {
                json!({
                    "id": skill.skill.id,
                    "manifestDigest": skill.skill.manifest_digest,
                })
            })
            .collect::<Vec<_>>();
        params.insert(
            "skills".to_string(),
            json!({
                "protocol": skill_request.protocol,
                "skillRefs": skill_refs,
                "manifestDigest": skill_request.manifest_digest,
            }),
        );
    }
    Value::Object(params)
}

fn turn_start_params(
    thread_id: &str,
    request: &CodexThreadRequest,
    idempotency_key: &str,
    native_skills: &[NativeCodexSkill],
) -> Value {
    let mut input = vec![json!({
        "type": "text",
        "text": request.prompt,
    })];
    input.extend(native_skills.iter().map(native_skill_input));
    json!({
        "threadId": thread_id,
        "clientUserMessageId": idempotency_key,
        "input": input,
    })
}

fn resolve_native_skill_refs(
    inventory: &[NativeCodexSkill],
    request: &CodexSkillExecutionRequest,
) -> anyhow::Result<Vec<NativeCodexSkill>> {
    let mut resolved = Vec::with_capacity(request.skill_refs.len());
    for reference in &request.skill_refs {
        let Some(skill) = inventory
            .iter()
            .find(|skill| skill.skill.id == reference.id)
        else {
            bail!("skill_unknown");
        };
        let Some(expected) = reference.manifest_digest.as_deref() else {
            // A resolved Skill reference must carry the immutable manifest
            // pin; an inventory entry without a matching digest cannot be
            // used to authorize a native dispatch.
            bail!("skill_manifest_conflict");
        };
        if skill.skill.manifest_digest.as_deref() != Some(expected) {
            bail!("skill_manifest_conflict");
        }
        resolved.push(skill.clone());
    }
    Ok(resolved)
}

fn extract_thread_id(response: &Value) -> anyhow::Result<String> {
    let id = response
        .pointer("/thread/id")
        .or_else(|| response.get("threadId"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("codex_thread_response_invalid"))?;
    validate_id(id, "thread_id")?;
    Ok(id.to_string())
}

fn extract_execution_id(response: &Value) -> anyhow::Result<String> {
    let id = response
        .pointer("/turn/id")
        .or_else(|| response.get("executionId"))
        .or_else(|| response.get("turnId"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("codex_execution_response_invalid"))?;
    validate_id(id, "execution_id")?;
    Ok(id.to_string())
}

fn validate_skill_execution_request(request: &CodexSkillExecutionRequest) -> anyhow::Result<()> {
    if !matches!(
        request.protocol.as_str(),
        "agent-skill-v1" | "skill-bundles-v1"
    ) {
        bail!("runtime_skills_unsupported");
    }
    validate_digest(&request.manifest_digest)?;
    if request.skill_refs.len() > MAX_SKILLS {
        bail!("skill_refs_too_large");
    }
    for reference in &request.skill_refs {
        validate_id(&reference.id, "skill_id")?;
        if let Some(digest) = reference.manifest_digest.as_deref() {
            validate_digest(digest)?;
        }
    }
    Ok(())
}

fn parse_execution_state(value: &str) -> CodexExecutionState {
    match value {
        "queued" | "pending" => CodexExecutionState::Queued,
        "inProgress" | "running" | "active" => CodexExecutionState::Running,
        "completed" | "complete" | "succeeded" => CodexExecutionState::Completed,
        "failed" | "errored" | "error" => CodexExecutionState::Failed,
        "interrupted" | "cancelled" | "canceled" => CodexExecutionState::Cancelled,
        _ => CodexExecutionState::Unknown,
    }
}

fn validate_id(value: &str, field: &str) -> anyhow::Result<()> {
    if value.trim().is_empty()
        || value.len() > MAX_ID_LENGTH
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'@')
        })
    {
        bail!("codex_{field}_invalid");
    }
    Ok(())
}

fn validate_idempotency_key(value: &str) -> anyhow::Result<()> {
    validate_id(value, "idempotency_key")
}

fn validate_text(value: &str, max: usize, field: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        bail!("codex_{field}_invalid");
    }
    Ok(())
}

fn validate_path(value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty()
        || value.len() > MAX_ID_LENGTH
        || value
            .bytes()
            .any(|byte| byte == 0 || byte == b'\r' || byte == b'\n')
    {
        bail!("codex_cwd_invalid");
    }
    // Windows drive paths and UNC paths are valid even when this crate is
    // unit-tested on another host.  Unix absolute paths are accepted too.
    let windows_absolute = value.as_bytes().get(1) == Some(&b':') || value.starts_with("\\\\");
    if !windows_absolute && !value.starts_with('/') {
        bail!("codex_cwd_invalid");
    }
    Ok(())
}

fn validate_skill_path(value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty()
        || value.len() > MAX_SKILL_PATH_LENGTH
        || value.chars().any(char::is_control)
    {
        bail!("codex_skill_path_invalid");
    }
    // The current-page schema promises an absolute path. Accept Windows drive
    // and UNC forms even when the Core tests run on a Unix host.
    let bytes = value.as_bytes();
    let drive_absolute = bytes.len() >= 3 && bytes[1] == b':' && matches!(bytes[2], b'/' | b'\\');
    let windows_absolute = drive_absolute || value.starts_with("\\\\");
    if !windows_absolute && !value.starts_with('/') {
        bail!("codex_skill_path_invalid");
    }
    Ok(())
}

fn validate_digest(value: &str) -> anyhow::Result<()> {
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
        bail!("skill_manifest_digest_invalid");
    }
    Ok(())
}

fn is_capability(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn truncate_text(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding() -> CodexRuntimeBinding {
        CodexRuntimeBinding {
            runtime_id: "runtime-codex".to_string(),
            provider: "codex".to_string(),
            app_server_version: Some("fake".to_string()),
            declared_capabilities: Vec::new(),
        }
    }

    fn request(prompt: &str) -> CodexThreadRequest {
        CodexThreadRequest {
            workspace_id: "workspace-1".to_string(),
            issue_id: "issue-1".to_string(),
            prompt: prompt.to_string(),
            cwd: None,
            skill_request: None,
        }
    }

    #[test]
    fn rebinding_same_cdp_target_starts_a_new_page_generation() {
        let state = Arc::new(Mutex::new(None));
        let transport = CodexPageHostTransport::new(state);
        assert_eq!(transport.generation(), 0);
        transport.set_websocket_url("ws://page-1").unwrap();
        assert_eq!(transport.generation(), 1);
        transport.set_websocket_url("ws://page-1").unwrap();
        assert_eq!(transport.generation(), 2);
    }

    #[test]
    fn host_exception_detail_is_bounded_and_prefers_runtime_description() {
        let details = serde_json::json!({
            "text": "fallback",
            "exception": {"description": "function_call_output requires call_id on HTTP requests; continuation via previous_response_id is only supported on Responses WebSocket v2"}
        });
        let message = exception_detail_message(&details);
        assert!(message.starts_with("function_call_output requires call_id"));
        assert!(message.len() <= MAX_HOST_ERROR_LENGTH);
    }

    #[test]
    fn host_transport_error_is_compacted_without_payload_dump() {
        let value = bounded_host_error("  websocket   failed\nwith details  ");
        assert_eq!(value, "websocket failed with details");
    }

    #[tokio::test]
    async fn fake_service_capabilities_skills_and_thread_lifecycle_are_native_calls() {
        let transport = FakeCodexPageHostTransport::default();
        let client = CodexPageExecutionClient::new(transport.clone(), binding()).unwrap();
        let caps = client.capabilities().await.unwrap();
        assert_eq!(caps.provider, "codex");
        assert!(caps.skills_supported);
        assert!(caps.skills_inventory_supported);
        assert!(caps.subagents_supported);

        let created = client
            .create_thread(request("first"), "command-1")
            .await
            .unwrap();
        let opened = client.open_thread(&created.thread_id).await.unwrap();
        assert_eq!(opened.thread_id, created.thread_id);
        let continued = client
            .continue_thread(&created.thread_id, request("next"), "command-2")
            .await
            .unwrap();
        assert_eq!(continued.thread_id, created.thread_id);
        assert!(continued.execution_id.is_some());
        let status = client
            .cancel_execution(
                &created.thread_id,
                continued.execution_id.as_deref().unwrap(),
            )
            .await
            .unwrap();
        assert!(matches!(status.state, CodexExecutionState::CancelPending));

        let methods = transport
            .calls()
            .into_iter()
            .map(|call| call.method)
            .collect::<Vec<_>>();
        assert_eq!(
            methods,
            vec![
                CodexPageHostMethod::Initialize,
                CodexPageHostMethod::ThreadStart,
                CodexPageHostMethod::TurnStart,
                CodexPageHostMethod::ThreadRead,
                CodexPageHostMethod::TurnStart,
                CodexPageHostMethod::TurnInterrupt,
            ]
        );
    }

    #[tokio::test]
    async fn native_thread_and_turn_include_digest_pinned_skill_request() {
        let transport = FakeCodexPageHostTransport::default();
        let digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let skills = json!({
            "data": [{"skills": [{
                "id": "codex:review-helper",
                "name": "review-helper",
                "path": "C:/codex/skills/review-helper/SKILL.md",
                "enabled": true,
                "manifestDigest": digest
            }]}]
        });
        // The adapter revalidates the live inventory immediately before both
        // create and continue, so each native operation gets its own read.
        transport.push_response(CodexPageHostMethod::SkillsList, Ok(skills.clone()));
        transport.push_response(CodexPageHostMethod::SkillsList, Ok(skills));
        let client = CodexPageExecutionClient::new(transport.clone(), binding()).unwrap();
        let skill_request = CodexSkillExecutionRequest {
            protocol: "skill-bundles-v1".to_string(),
            skill_refs: vec![SkillReference {
                id: "codex:review-helper".to_string(),
                manifest_digest: Some(digest.to_string()),
            }],
            manifest_digest: digest.to_string(),
        };
        let mut first = request("first");
        first.cwd = Some("C:/workspace".to_string());
        first.skill_request = Some(skill_request.clone());
        let created = client.create_thread(first, "skill-create").await.unwrap();
        let mut next = request("next");
        next.cwd = Some("C:/workspace".to_string());
        next.skill_request = Some(skill_request);
        client
            .continue_thread(&created.thread_id, next, "skill-turn")
            .await
            .unwrap();

        let calls = transport.calls();
        let thread_start = calls
            .iter()
            .find(|call| call.method == CodexPageHostMethod::ThreadStart)
            .expect("thread/start call");
        let turn_start = calls
            .iter()
            .rev()
            .find(|call| call.method == CodexPageHostMethod::TurnStart)
            .expect("turn/start call");
        assert_eq!(
            thread_start.params,
            json!({
                "threadSource": "multica",
                "cwd": "C:/workspace",
                "skills": {
                    "protocol": "skill-bundles-v1",
                    "skillRefs": [{
                        "id": "codex:review-helper",
                        "manifestDigest": digest,
                    }],
                    "manifestDigest": digest,
                },
            })
        );
        assert_eq!(turn_start.params.as_object().unwrap().len(), 3);
        let input = turn_start.params["input"].as_array().unwrap();
        assert_eq!(input.len(), 2);
        assert_eq!(input[0], json!({"type": "text", "text": "next"}));
        assert_eq!(
            input[1],
            json!({
                "type": "skill",
                "name": "review-helper",
                "path": "C:/codex/skills/review-helper/SKILL.md"
            })
        );
        assert_eq!(input[1].as_object().unwrap().len(), 3);
        assert!(turn_start.params.get("skills").is_none());
        let skill_calls = calls
            .iter()
            .filter(|call| call.method == CodexPageHostMethod::SkillsList)
            .collect::<Vec<_>>();
        assert_eq!(skill_calls.len(), 2);
        assert!(skill_calls.iter().all(|call| {
            call.params == json!({ "cwds": ["C:/workspace"], "forceReload": true })
        }));
    }

    #[tokio::test]
    async fn create_and_continue_are_idempotent_without_duplicate_native_calls() {
        let transport = FakeCodexPageHostTransport::default();
        let client = CodexPageExecutionClient::new(transport.clone(), binding()).unwrap();
        let first = client
            .create_thread(request("first"), "same")
            .await
            .unwrap();
        let second = client
            .create_thread(request("different"), "same")
            .await
            .unwrap();
        assert_eq!(first, second);
        let first_turn = client
            .continue_thread(&first.thread_id, request("next"), "turn-same")
            .await
            .unwrap();
        let second_turn = client
            .continue_thread(&first.thread_id, request("different"), "turn-same")
            .await
            .unwrap();
        assert_eq!(first_turn, second_turn);
        let methods = transport
            .calls()
            .into_iter()
            .map(|call| call.method)
            .collect::<Vec<_>>();
        assert_eq!(
            methods,
            vec![
                CodexPageHostMethod::Initialize,
                CodexPageHostMethod::ThreadStart,
                CodexPageHostMethod::TurnStart,
                CodexPageHostMethod::TurnStart,
            ]
        );
    }

    #[tokio::test]
    async fn concurrent_create_is_single_flight() {
        let transport = FakeCodexPageHostTransport::default();
        transport.set_delay_millis(20);
        let client = Arc::new(CodexPageExecutionClient::new(transport.clone(), binding()).unwrap());
        let (first, second, third) = tokio::join!(
            client.create_thread(request("first"), "concurrent-same"),
            client.create_thread(request("second"), "concurrent-same"),
            client.create_thread(request("third"), "concurrent-same"),
        );
        let first = first.unwrap();
        assert_eq!(second.unwrap(), first);
        assert_eq!(third.unwrap(), first);
        let calls = transport.calls();
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.method == CodexPageHostMethod::ThreadStart)
                .count(),
            1
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.method == CodexPageHostMethod::TurnStart)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn turn_failure_reuses_the_thread_created_for_the_same_key() {
        let transport = FakeCodexPageHostTransport::default();
        transport.push_response(
            CodexPageHostMethod::TurnStart,
            Err(anyhow!("turn temporarily unavailable")),
        );
        let client = CodexPageExecutionClient::new(transport.clone(), binding()).unwrap();
        assert_eq!(
            client
                .create_thread(request("first"), "partial-create")
                .await
                .unwrap_err()
                .to_string(),
            "turn temporarily unavailable"
        );
        let recovered = client
            .create_thread(request("first"), "partial-create")
            .await
            .unwrap();
        assert!(recovered.execution_id.is_some());
        let calls = transport.calls();
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.method == CodexPageHostMethod::ThreadStart)
                .count(),
            1
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.method == CodexPageHostMethod::TurnStart)
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn page_generation_change_invalidates_live_capability_cache() {
        let transport = FakeCodexPageHostTransport::default();
        let client = CodexPageExecutionClient::new(transport.clone(), binding()).unwrap();
        assert!(client.capabilities().await.unwrap().skills_supported);
        transport.push_response(
            CodexPageHostMethod::Initialize,
            Ok(json!({"provider":"codex","capabilities":[]})),
        );
        transport.bump_generation();
        let refreshed = client.capabilities().await.unwrap();
        assert!(!refreshed.skills_supported);
        assert!(!refreshed.skills_inventory_supported);
        assert!(!refreshed.subagents_supported);
        assert_eq!(
            transport
                .calls()
                .iter()
                .filter(|call| call.method == CodexPageHostMethod::Initialize)
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn declared_runtime_capabilities_do_not_override_live_host() {
        let transport = FakeCodexPageHostTransport::default();
        transport.push_response(
            CodexPageHostMethod::Initialize,
            Ok(json!({"provider":"codex"})),
        );
        let mut runtime = binding();
        runtime.declared_capabilities = vec!["skill-bundles-v1".to_string()];
        let client = CodexPageExecutionClient::new(transport, runtime).unwrap();
        let capabilities = client.capabilities().await.unwrap();
        assert!(!capabilities.skills_supported);
        assert!(!capabilities.skills_inventory_supported);
        assert_eq!(capabilities.skill_protocol, None);
    }

    #[tokio::test]
    async fn inventory_only_skills_are_read_only_and_cannot_dispatch() {
        let transport = FakeCodexPageHostTransport::default();
        transport.push_response(
            CodexPageHostMethod::Initialize,
            Ok(json!({
                "provider": "codex",
                "capabilities": [],
                "pageHostProbe": { "skillsList": true }
            })),
        );
        let digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        transport.push_response(
            CodexPageHostMethod::SkillsList,
            Ok(json!({"data": [{"skills": [{
                "id": "codex:review-helper",
                "name": "review-helper",
                "path": "C:/codex/skills/review-helper/SKILL.md",
                "enabled": true,
                "manifestDigest": digest
            }]}]})),
        );
        let client = CodexPageExecutionClient::new(transport.clone(), binding()).unwrap();

        let capabilities = client.capabilities().await.unwrap();
        assert!(capabilities.skills_inventory_supported);
        assert!(!capabilities.skills_supported);
        assert_eq!(capabilities.skill_protocol, None);
        assert_eq!(
            client.list_skills().await.unwrap()[0].id,
            "codex:review-helper"
        );

        let skill_request = CodexSkillExecutionRequest {
            protocol: "agent-skill-v1".to_string(),
            skill_refs: vec![SkillReference {
                id: "codex:review-helper".to_string(),
                manifest_digest: Some(digest.to_string()),
            }],
            manifest_digest: digest.to_string(),
        };
        assert_eq!(
            client
                .resolve_skills(skill_request.clone())
                .await
                .unwrap_err()
                .to_string(),
            "runtime_skills_unsupported"
        );
        let mut thread = request("inventory-only");
        thread.skill_request = Some(skill_request);
        assert_eq!(
            client
                .create_thread(thread, "inventory-only")
                .await
                .unwrap_err()
                .to_string(),
            "runtime_skills_unsupported"
        );

        let methods = transport
            .calls()
            .into_iter()
            .map(|call| call.method)
            .collect::<Vec<_>>();
        assert_eq!(
            methods,
            vec![
                CodexPageHostMethod::Initialize,
                CodexPageHostMethod::SkillsList,
            ]
        );
    }

    #[tokio::test]
    async fn provider_mismatch_and_unadvertised_subagent_are_fail_closed() {
        let transport = FakeCodexPageHostTransport::default();
        transport.push_response(
            CodexPageHostMethod::Initialize,
            Ok(json!({"provider":"anthropic","capabilities":[]})),
        );
        let client = CodexPageExecutionClient::new(transport.clone(), binding()).unwrap();
        assert_eq!(
            client.capabilities().await.unwrap_err().to_string(),
            "runtime_provider_mismatch"
        );

        let transport = FakeCodexPageHostTransport::default();
        transport.push_response(
            CodexPageHostMethod::Initialize,
            Ok(json!({"provider":"codex","capabilities":[]})),
        );
        let client = CodexPageExecutionClient::new(transport.clone(), binding()).unwrap();
        assert_eq!(
            client
                .create_subagent("parent-1", request("member"), "member-1")
                .await
                .unwrap_err()
                .to_string(),
            "unsupported"
        );
        assert_eq!(transport.calls().len(), 1);
    }

    #[tokio::test]
    async fn skill_inventory_discards_paths_and_resolves_immutable_digest() {
        let transport = FakeCodexPageHostTransport::default();
        let skills_response = json!({"data":[{"cwd":"C:/secret","skills":[{
            "name":"review-helper",
            "description":"review files",
            "path":"C:/secret/SKILL.md",
            "enabled":true,
            "manifestDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        }]}]});
        // `resolve_skills` intentionally re-reads the runtime inventory so a
        // stale list cannot authorize a dispatch after the runtime changes.
        transport.push_response(CodexPageHostMethod::SkillsList, Ok(skills_response.clone()));
        transport.push_response(CodexPageHostMethod::SkillsList, Ok(skills_response));
        let client = CodexPageExecutionClient::new(transport, binding()).unwrap();
        let skills = client.list_skills().await.unwrap();
        assert_eq!(skills[0].id, "codex:review-helper");
        let serialized = serde_json::to_string(&skills).unwrap();
        assert!(!serialized.contains("C:/secret"));
        let digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let request = CodexSkillExecutionRequest {
            protocol: "skill-bundles-v1".to_string(),
            skill_refs: vec![SkillReference {
                id: "codex:review-helper".to_string(),
                manifest_digest: Some(digest.to_string()),
            }],
            manifest_digest: digest.to_string(),
        };
        assert_eq!(
            client.resolve_skills(request.clone()).await.unwrap(),
            request
        );
    }

    #[test]
    fn request_validation_does_not_accept_relative_or_control_paths() {
        let mut request = request("ok");
        request.cwd = Some("relative/path".to_string());
        assert_eq!(
            request.validate().unwrap_err().to_string(),
            "codex_cwd_invalid"
        );
        request.cwd = Some("C:/workspace\n".to_string());
        assert_eq!(
            request.validate().unwrap_err().to_string(),
            "codex_cwd_invalid"
        );
    }

    #[test]
    fn native_skill_metadata_requires_valid_name_and_absolute_path() {
        let mut valid = json!({
            "data": [{"skills": [{
                "name": "review-helper",
                "path": "C:/codex/skills/review-helper/SKILL.md",
                "enabled": true
            }]}]
        });
        let parsed = parse_native_skills(&valid).unwrap();
        assert_eq!(parsed[0].skill.name, "review-helper");
        assert_eq!(parsed[0].path, "C:/codex/skills/review-helper/SKILL.md");

        valid["data"][0]["skills"][0]
            .as_object_mut()
            .unwrap()
            .remove("path");
        assert_eq!(
            parse_native_skills(&valid).unwrap_err().to_string(),
            "codex_skill_path_missing"
        );

        for path in [
            "relative/SKILL.md",
            "C:relative/SKILL.md",
            "C:/codex/skills/\nSKILL.md",
        ] {
            let invalid = json!({
                "data": [{"skills": [{
                    "name": "review-helper",
                    "path": path,
                    "enabled": true
                }]}]
            });
            assert_eq!(
                parse_native_skills(&invalid).unwrap_err().to_string(),
                "codex_skill_path_invalid"
            );
        }

        for name in ["", "review\u{0007}helper"] {
            let invalid = json!({
                "data": [{"skills": [{
                    "name": name,
                    "path": "/opt/codex/skills/review-helper/SKILL.md",
                    "enabled": true
                }]}]
            });
            assert_eq!(
                parse_native_skills(&invalid).unwrap_err().to_string(),
                "codex_skill_name_invalid"
            );
        }
    }
}
