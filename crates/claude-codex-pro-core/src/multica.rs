//! Isolated Multica runtime adapter.
//!
//! This module deliberately has no dependency on the relay/profile settings
//! layer.  Multica is an external control plane: the adapter owns only its
//! connection records, read-only snapshots, and explicitly configured
//! sidecar processes.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::future::Future;
use std::io::{Cursor, Read, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, anyhow, bail};
use flate2::read::GzDecoder;
use fs2::FileExt;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderValue};
use reqwest::{Client, Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};

#[cfg(windows)]
use crate::multica_managed_job::ManagedProcessJob;

const CONNECTIONS_FILE: &str = "connections.json";
const SNAPSHOTS_FILE: &str = "snapshots.json";
const SIDECAR_LIFECYCLE_LOG_FILE: &str = "sidecar-lifecycle.jsonl";
const SIDECAR_LIFECYCLE_LOG_ROTATED_FILE: &str = "sidecar-lifecycle.jsonl.1";
const MAX_SIDECAR_LIFECYCLE_LOG_BYTES: u64 = 256 * 1024;
const MAX_COLLECTION_ITEMS: usize = 100;
const MAX_TEXT_LENGTH: usize = 240;
const MAX_PUBLIC_TEXT_INPUT_LENGTH: usize = 4096;
/// Never buffer more than this amount from an upstream response.  Multica
/// endpoints used by this adapter are small status documents; a bounded body
/// protects the manager from an unexpectedly large or hostile response.
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const HEALTH_TOTAL_TIMEOUT: Duration = Duration::from_secs(20);
const SNAPSHOT_TOTAL_TIMEOUT: Duration = Duration::from_secs(20);
const SNAPSHOT_CONCURRENCY_LIMIT: usize = 4;
const DAEMON_HEALTH_TIMEOUT: Duration = Duration::from_secs(3);
const DAEMON_STARTUP_PROBE_ATTEMPTS: usize = 4;
const DAEMON_STARTUP_PROBE_RETRY_DELAY: Duration = Duration::from_millis(250);
const SIDECAR_STOP_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const SIDECAR_STOP_POLL_INTERVAL: Duration = Duration::from_millis(20);
const DEFAULT_DAEMON_HEALTH_PORT: u16 = 19514;
const FORBIDDEN_PORTS: &[&str] = &["57321", "57331", "57320", "9230"];
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

// The managed runtime is intentionally pinned.  Updating this table is a
// release operation: the asset name and digest must be changed together and
// are verified again against the official checksums file before activation.
const MANAGED_RUNTIME_VERSION: &str = "0.4.36";
const MANAGED_RUNTIME_RELEASE_TAG: &str = "v0.4.36";
const MANAGED_RUNTIME_REPOSITORY_OWNER: &str = "multica-ai";
const MANAGED_RUNTIME_REPOSITORY_NAME: &str = "multica";
const MANAGED_RUNTIME_CONNECTION_ID: &str = "managed-multica";
const MANAGED_RUNTIME_DISPLAY_NAME: &str = "内置 Multica Runtime";
const MANAGED_RUNTIME_SERVER_URL: &str = "https://api.multica.ai";
const MANAGED_RUNTIME_PROFILE: &str = "ccp-managed";
const MANAGED_PROFILE_CONFIG_FILE: &str = "config.json";
const MANAGED_PROFILE_MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MANAGED_WORKSPACE_MAX_CURSOR_LENGTH: usize = 512;
const MANAGED_WORKSPACE_MAX_TOKEN_LENGTH: usize = 8 * 1024;
const MANAGED_WORKSPACE_SKILL_POLL_ATTEMPTS: usize = 60;
const MANAGED_WORKSPACE_SKILL_POLL_DELAY: Duration = Duration::from_millis(500);
const MANAGED_CONNECTION_INIT_ERROR_CODE: &str = "managed_connection_init_failed";
const MANAGED_CONNECTION_RESERVED_ERROR: &str = "managed_connection_reserved";
const MANAGED_RUNTIME_MAX_ARCHIVE_BYTES: usize = 96 * 1024 * 1024;
const MANAGED_RUNTIME_MAX_BINARY_BYTES: usize = 64 * 1024 * 1024;
const MANAGED_RUNTIME_MAX_ARCHIVE_ENTRIES: usize = 32;
const MANAGED_RUNTIME_MAX_REDIRECTS: usize = 3;
const MANAGED_RUNTIME_MAX_DOWNLOAD_ATTEMPTS: usize = 3;
const MANAGED_RUNTIME_DOWNLOAD_RETRY_BACKOFFS: [Duration; 2] =
    [Duration::from_millis(250), Duration::from_millis(500)];
const MANAGED_RUNTIME_MAX_METADATA_BYTES: usize = 16 * 1024;
const MANAGED_RUNTIME_CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(25);
const MANAGED_RUNTIME_STATE_DIR: &str = "managed-runtime";
const MANAGED_RUNTIME_CURRENT_FILE: &str = "current.json";
const MANAGED_RUNTIME_PREVIOUS_FILE: &str = "previous.json";
const MANAGED_RUNTIME_INSTALL_FAILURE_FILE: &str = "install-failure.json";
const MANAGED_RUNTIME_VERSIONS_DIR: &str = "versions";
const MANAGED_RUNTIME_LOCK_FILE: &str = "install.lock";
const MANAGED_RUNTIME_LIFECYCLE_LOCK_FILE: &str = "lifecycle.lock";
const MANAGED_RUNTIME_OWNER_FILE: &str = "owner.json";
const MANAGED_RUNTIME_MAX_OWNER_BYTES: u64 = 16 * 1024;
const MANAGED_RUNTIME_STALE_OWNER_WAIT: Duration = Duration::from_secs(1);
// The supervised daemon is intentionally conservative: a temporary process
// failure gets three bounded recovery attempts, then control returns to the
// user.  These values are kept local so ordinary manually configured sidecars
// retain their existing one-shot lifecycle semantics.
const MANAGED_SUPERVISOR_MAX_RESTARTS: u8 = 3;
const MANAGED_SUPERVISOR_BACKOFFS: [Duration; MANAGED_SUPERVISOR_MAX_RESTARTS as usize] = [
    Duration::from_millis(100),
    Duration::from_millis(250),
    Duration::from_millis(500),
];
const MANAGED_SUPERVISOR_POLL_INTERVAL: Duration = Duration::from_millis(100);
const MANAGED_SUPERVISOR_HEALTH_PROBE_INTERVAL: Duration = Duration::from_secs(5);

const MANAGED_RUNTIME_WINDOWS_AMD64_SHA256: &str =
    "b96bc1df13824ed1bcb733351eb29ae570cdf3bae1f004dba45215cd011c744c";
const MANAGED_RUNTIME_WINDOWS_ARM64_SHA256: &str =
    "819e4839fab86a1c50af8fb755c3d5eafc78e8655931a22a2486264e0fd58ac0";
const MANAGED_RUNTIME_DARWIN_AMD64_SHA256: &str =
    "76d0e286b085cbb3f716c7ee5cfce7aee4ac223589620b0cdc5d86d5de7e8803";
const MANAGED_RUNTIME_DARWIN_ARM64_SHA256: &str =
    "ca7b62877628444bb08f8109008220616fefb275927ad741ad372114ee2f7d62";
const MANAGED_RUNTIME_LINUX_AMD64_SHA256: &str =
    "bdee5c7f574202e43d9cafe23914a384ad4e86098b98f59432faed6fdc92bfa2";
const MANAGED_RUNTIME_LINUX_ARM64_SHA256: &str =
    "e6cd65111f2a98f22d602d1db53aa506cacf906fc34ac59dc204525e34594f60";

const KNOWN_ITEM_STATUSES: &[&str] = &[
    "unknown",
    "pending",
    "queued",
    "starting",
    "initializing",
    "initialized",
    "initialised",
    "running",
    "ready",
    "healthy",
    "degraded",
    "stopped",
    "failed",
    "error",
    "completed",
    "complete",
    "cancelled",
    "canceled",
    "success",
    "succeeded",
    "paused",
    "waiting",
    "idle",
    "active",
    "inactive",
    "unavailable",
    "terminated",
    "connected",
    "disconnected",
];

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
static STORE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static SNAPSHOT_CACHE: OnceLock<Mutex<HashMap<String, MulticaRuntimeSnapshot>>> = OnceLock::new();
static SIDECARS: OnceLock<Mutex<HashMap<String, SidecarProcess>>> = OnceLock::new();
static ACTIVE_REQUESTS: OnceLock<Mutex<HashMap<String, ActiveRequest>>> = OnceLock::new();
static SNAPSHOT_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
static SIDECAR_LIFECYCLE_LOG_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
/// The in-process managed-runtime install cancellation token. There can be
/// only one installer in this process because the cross-process file lock is
/// held for the whole operation.
static MANAGED_INSTALL_CANCEL: OnceLock<Mutex<Option<Arc<AtomicBool>>>> = OnceLock::new();
/// In-process progress for the one managed-runtime installer.  The state is
/// deliberately ephemeral and redacted: it contains only fixed release
/// metadata and byte counters, never a URL, path, response body, or token.
static MANAGED_INSTALL_PROGRESS: OnceLock<Mutex<Option<ManagedInstallProgress>>> = OnceLock::new();
/// Runtime-only lifecycle state for the one fixed managed daemon.  It is not
/// persisted: a new CCP process must establish fresh ownership of a child it
/// starts itself rather than guessing from a stale PID.
static MANAGED_SUPERVISOR: OnceLock<Mutex<ManagedSupervisorState>> = OnceLock::new();
#[cfg(test)]
static SIDECAR_LIFECYCLE_LOG_PATH_OVERRIDE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
/// Set before manager shutdown starts.  The flag is coordinated with the
/// sidecar map lock so an auto-start operation cannot insert a newly spawned
/// child after the final cleanup snapshot has been taken.
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
struct ManagedSupervisorState {
    generation: u64,
    worker_running: bool,
    /// Set by an explicit Stop, disable, or application shutdown.  A worker
    /// observes this before every delay and before every spawn.
    stop_requested: bool,
    restart_attempts: u8,
    restart_exhausted: bool,
    /// A generation can fall back to its verified previous version at most
    /// once. This prevents a failing current/previous pair from oscillating.
    rollback_attempted: bool,
    last_terminal_status: Option<MulticaDaemonStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedSupervisorAction {
    Observe,
    Recover,
    StopUnsafe,
}

#[derive(Debug, Clone, Default)]
struct ManagedInstallProgress {
    phase: String,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    progress_percent: Option<u8>,
    version: Option<String>,
    target_triple: Option<String>,
    asset_name: Option<String>,
    updated_at_ms: u64,
}

impl Default for ManagedSupervisorState {
    fn default() -> Self {
        Self {
            generation: 0,
            worker_running: false,
            stop_requested: false,
            restart_attempts: 0,
            restart_exhausted: false,
            rollback_attempted: false,
            last_terminal_status: None,
        }
    }
}

fn managed_supervisor() -> &'static Mutex<ManagedSupervisorState> {
    MANAGED_SUPERVISOR.get_or_init(|| Mutex::new(ManagedSupervisorState::default()))
}

fn store_lock() -> &'static Mutex<()> {
    STORE_LOCK.get_or_init(|| Mutex::new(()))
}

fn snapshot_cache() -> &'static Mutex<HashMap<String, MulticaRuntimeSnapshot>> {
    SNAPSHOT_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn sidecars() -> &'static Mutex<HashMap<String, SidecarProcess>> {
    SIDECARS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Close the sidecar admission gate.  Holding the same mutex used for child
/// insertion makes the transition atomic with respect to `start_sidecar`:
/// either the child is already tracked and cleanup sees it, or the starter
/// observes the flag and tears down its untracked child before returning.
pub fn request_shutdown() {
    let _guard = sidecars()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
    // Invalidate the managed worker before the caller snapshots tracked
    // children for cleanup.  The worker never receives a chance to replace a
    // child after shutdown has begun.
    invalidate_managed_supervisor(true, None);
}

fn shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

fn active_requests() -> &'static Mutex<HashMap<String, ActiveRequest>> {
    ACTIVE_REQUESTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn managed_install_cancel_slot() -> &'static Mutex<Option<Arc<AtomicBool>>> {
    MANAGED_INSTALL_CANCEL.get_or_init(|| Mutex::new(None))
}

fn managed_install_progress_slot() -> &'static Mutex<Option<ManagedInstallProgress>> {
    MANAGED_INSTALL_PROGRESS.get_or_init(|| Mutex::new(None))
}

fn managed_install_in_progress() -> bool {
    managed_install_cancel_slot()
        .lock()
        .map(|slot| slot.is_some())
        .unwrap_or(false)
}

fn progress_percent(downloaded_bytes: u64, total_bytes: Option<u64>) -> Option<u8> {
    let total = total_bytes?;
    if total == 0 {
        return Some(0);
    }
    Some(
        downloaded_bytes
            .saturating_mul(100)
            .checked_div(total)
            .unwrap_or(0)
            .min(100) as u8,
    )
}

/// Publish a fixed, sanitized installation phase.  Calls made by unit-test
/// fixture installers (which do not own `ManagedInstallGuard`) are ignored so
/// one test cannot leak progress into another test's process-global status.
fn set_managed_install_progress(
    asset: Option<MulticaRuntimeAsset>,
    phase: &'static str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    percent_override: Option<u8>,
) {
    if !managed_install_in_progress() {
        return;
    }
    let (version, target_triple, asset_name) = asset
        .map(|asset| {
            (
                Some(asset.version.to_string()),
                Some(asset.target_triple.to_string()),
                Some(asset.asset_name.to_string()),
            )
        })
        .unwrap_or((None, None, None));
    let mut progress = managed_install_progress_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *progress = Some(ManagedInstallProgress {
        phase: phase.to_string(),
        downloaded_bytes,
        total_bytes,
        progress_percent: percent_override
            .or_else(|| progress_percent(downloaded_bytes, total_bytes)),
        version,
        target_triple,
        asset_name,
        updated_at_ms: now_ms(),
    });
}

fn clear_managed_install_progress() {
    if let Ok(mut progress) = managed_install_progress_slot().lock() {
        *progress = None;
    }
}

struct ManagedInstallGuard {
    token: Arc<AtomicBool>,
}

impl ManagedInstallGuard {
    fn begin() -> Self {
        let mut slot = managed_install_cancel_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(token) = slot.as_ref() {
            return Self {
                token: token.clone(),
            };
        }
        let token = Arc::new(AtomicBool::new(false));
        *slot = Some(token.clone());
        Self { token }
    }
}

impl Drop for ManagedInstallGuard {
    fn drop(&mut self) {
        let mut slot = managed_install_cancel_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot
            .as_ref()
            .is_some_and(|active| Arc::ptr_eq(active, &self.token))
        {
            *slot = None;
        }
        clear_managed_install_progress();
    }
}

fn check_managed_install_cancelled(cancel: Option<&AtomicBool>) -> anyhow::Result<()> {
    let globally_cancelled = managed_install_cancel_slot()
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().cloned())
        .is_some_and(|token| token.load(Ordering::Acquire));
    if globally_cancelled || cancel.is_some_and(|token| token.load(Ordering::Acquire)) {
        bail!("managed_runtime_install_cancelled");
    }
    Ok(())
}

/// Reqwest's request and streaming-body futures have no cancellation-token
/// parameter. Racing them with this future makes dropping either wait prompt
/// while preserving the normal reqwest timeout/error classification.
async fn wait_for_managed_install_cancellation(cancel: Option<&AtomicBool>) {
    loop {
        if check_managed_install_cancelled(cancel).is_err() {
            return;
        }
        tokio::time::sleep(MANAGED_RUNTIME_CANCEL_POLL_INTERVAL).await;
    }
}

enum ManagedDownloadWaitError {
    Cancelled,
    Transport(reqwest::Error),
}

async fn await_managed_download<T, F>(
    future: F,
    cancel: Option<&AtomicBool>,
) -> Result<T, ManagedDownloadWaitError>
where
    F: Future<Output = Result<T, reqwest::Error>>,
{
    tokio::pin!(future);
    tokio::select! {
        result = &mut future => result.map_err(ManagedDownloadWaitError::Transport),
        _ = wait_for_managed_install_cancellation(cancel) => Err(ManagedDownloadWaitError::Cancelled),
    }
}

fn snapshot_semaphore() -> Arc<Semaphore> {
    SNAPSHOT_SEMAPHORE
        .get_or_init(|| Arc::new(Semaphore::new(SNAPSHOT_CONCURRENCY_LIMIT)))
        .clone()
}

struct ActiveRequest {
    sequence: u64,
    cancel: watch::Sender<bool>,
}

/// A connection-scoped request lease. Starting another request of the same
/// kind for the connection cancels the previous lease. The sequence check
/// prevents an older request from writing a cache entry after it was
/// superseded by a newer refresh.
#[derive(Clone)]
struct RequestGuard {
    key: String,
    sequence: u64,
    cancel: watch::Receiver<bool>,
}

impl RequestGuard {
    fn begin(kind: &str, connection_id: &str) -> Self {
        let key = format!("{kind}:{connection_id}");
        let sequence = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        let (cancel, receiver) = watch::channel(false);
        let mut requests = active_requests()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(previous) = requests.insert(
            key.clone(),
            ActiveRequest {
                sequence,
                cancel: cancel.clone(),
            },
        ) {
            let _ = previous.cancel.send(true);
        }
        Self {
            key,
            sequence,
            cancel: receiver,
        }
    }

    fn is_current(&self) -> bool {
        let requests = active_requests()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        requests
            .get(&self.key)
            .is_some_and(|request| request.sequence == self.sequence)
    }

    fn is_cancelled(&self) -> bool {
        *self.cancel.borrow()
    }

    async fn run<F, T>(&self, future: F, cancel_code: &'static str) -> anyhow::Result<T>
    where
        F: Future<Output = T>,
    {
        if self.is_cancelled() || !self.is_current() {
            return Err(anyhow!(cancel_code));
        }
        let mut cancellation = self.cancel.clone();
        let output = tokio::select! {
            _ = wait_for_request_cancel(&mut cancellation) => return Err(anyhow!(cancel_code)),
            output = future => output,
        };
        if self.is_cancelled() || !self.is_current() {
            return Err(anyhow!(cancel_code));
        }
        Ok(output)
    }

    fn finish(self) {
        let mut requests = active_requests()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if requests
            .get(&self.key)
            .is_some_and(|request| request.sequence == self.sequence)
        {
            requests.remove(&self.key);
        }
    }
}

async fn wait_for_request_cancel(receiver: &mut watch::Receiver<bool>) {
    if *receiver.borrow() {
        return;
    }
    while receiver.changed().await.is_ok() {
        if *receiver.borrow() {
            return;
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

/// A compile-time allowlisted Multica CLI release.  The fields are borrowed
/// from static data so callers cannot alter the release URL or asset name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaRuntimeAsset {
    pub version: &'static str,
    pub target_triple: &'static str,
    pub asset_name: &'static str,
    pub binary_name: &'static str,
    pub expected_sha256: &'static str,
}

/// Public, redacted state returned to the manager.  In particular, the
/// executable path is represented by its file name only; absolute paths and
/// command lines never cross the IPC boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MulticaRuntimeInstallStatus {
    pub install_state: String,
    /// A fixed, renderer-safe phase such as `downloading_archive` or
    /// `activating`. It is `None` when no installation is active.
    pub install_phase: Option<String>,
    /// Bytes downloaded/copied for the current archive operation. This is a
    /// counter only; no source URL or file path is exposed.
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub progress_percent: Option<u8>,
    pub installed_version: Option<String>,
    pub target_triple: Option<String>,
    pub asset_name: Option<String>,
    pub asset_source: Option<String>,
    pub sha256: Option<String>,
    pub sha256_verified: bool,
    pub executable_name: Option<String>,
    pub previous_version: Option<String>,
    pub last_install_error_code: Option<String>,
    pub updated_at_ms: Option<u64>,
    pub diagnostic: Option<String>,
}

/// Redacted authentication state for the managed Multica profile.  The CLI
/// itself owns the credential material in `~/.multica/profiles/ccp-managed`;
/// this DTO intentionally contains no token, URL, command line, or local
/// path.  A successful process exit is not enough to claim authentication:
/// `multica auth status` also exits successfully when no token is configured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MulticaManagedAuthStatus {
    pub status: String,
    pub checked_at_ms: Option<u64>,
    pub diagnostic: Option<String>,
}

/// Managed connection defaults are kept separate from supplier/profile data.
/// `profile` is passed only to the managed Multica process and is never used
/// as a CCP relay profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaManagedConnection {
    pub connection_id: String,
    pub display_name: String,
    pub server_url: String,
    pub profile: String,
    pub enabled: bool,
    pub auto_start: bool,
    pub supervise: bool,
}

impl Default for MulticaManagedConnection {
    fn default() -> Self {
        Self {
            connection_id: MANAGED_RUNTIME_CONNECTION_ID.to_string(),
            display_name: MANAGED_RUNTIME_DISPLAY_NAME.to_string(),
            server_url: MANAGED_RUNTIME_SERVER_URL.to_string(),
            profile: MANAGED_RUNTIME_PROFILE.to_string(),
            enabled: true,
            auto_start: true,
            supervise: true,
        }
    }
}

/// The editable, managed-only connection view.  Unlike a regular connection
/// view this deliberately retains the exact saved server URL so the Runtime
/// form can show and edit the user's value without reconstructing it from a
/// redacted display string.  It is only returned by the managed Runtime IPC
/// path, never by the generic connection list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaManagedConnectionView {
    pub connection_id: String,
    pub display_name: String,
    pub server_url: String,
    pub enabled: bool,
    pub profile: String,
    pub sidecar_configured: bool,
    pub sidecar_auto_start: bool,
}

impl MulticaManagedConnectionView {
    fn from_connection(connection: &MulticaConnectionConfig) -> Self {
        let (sidecar_configured, sidecar_auto_start) = connection
            .sidecar
            .as_ref()
            .map(|sidecar| (true, sidecar.auto_start))
            .unwrap_or((false, false));
        Self {
            connection_id: connection.connection_id.clone(),
            display_name: connection.display_name.clone(),
            server_url: connection.server_url.clone(),
            enabled: connection.enabled,
            profile: MANAGED_RUNTIME_PROFILE.to_string(),
            sidecar_configured,
            sidecar_auto_start,
        }
    }
}

/// The only mutable fields exposed by the managed Runtime surface.  Keeping
/// this separate from `MulticaConnectionInput` prevents generic URL
/// preservation and validation behavior from being applied to this record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaManagedConnectionUpdate {
    pub display_name: String,
    pub server_url: String,
    pub enabled: bool,
}

/// The complete allowlist for the first read-only managed workspace client.
/// Callers can select a resource, but cannot supply an HTTP method, URL,
/// header, or path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MulticaWorkspaceReadResource {
    Me,
    Issues,
    Projects,
    Agents,
    Runtimes,
    Skills,
    Squads,
    Autopilots,
}

impl MulticaWorkspaceReadResource {
    fn path(self) -> &'static str {
        match self {
            Self::Me => "/api/me",
            Self::Issues => "/api/issues",
            Self::Projects => "/api/projects",
            Self::Agents => "/api/agents",
            Self::Runtimes => "/api/runtimes",
            Self::Skills => "/api/skills",
            Self::Squads => "/api/squads",
            Self::Autopilots => "/api/autopilots",
        }
    }
}

fn default_managed_workspace_limit() -> usize {
    MAX_COLLECTION_ITEMS
}

/// Renderer-safe paging input. Unknown properties are rejected so this type
/// cannot become an accidental carrier for arbitrary request configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MulticaWorkspaceListRequest {
    pub workspace_id: String,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default = "default_managed_workspace_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MulticaWorkspaceUser {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MulticaWorkspaceIssue {
    pub id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub number: Option<u64>,
    #[serde(default)]
    pub identifier: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub status_name: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub assignee_type: Option<String>,
    #[serde(default)]
    pub assignee_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub parent_issue_id: Option<String>,
    #[serde(default)]
    pub revision: Option<u64>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MulticaWorkspaceProject {
    pub id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub issue_count: Option<u64>,
    #[serde(default)]
    pub done_count: Option<u64>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MulticaWorkspaceAgent {
    pub id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub runtime_id: Option<String>,
    #[serde(default)]
    pub runtime_mode: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub archived_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MulticaWorkspaceRuntime {
    pub id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub custom_name: Option<String>,
    #[serde(default)]
    pub runtime_mode: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub last_seen_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing)]
    metadata: Value,
}

impl MulticaWorkspaceRuntime {
    pub fn capabilities(&self) -> Vec<String> {
        self.metadata
            .get("capabilities")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 80
                    && value.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
                    })
            })
            .take(32)
            .map(str::to_string)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MulticaWorkspaceSkill {
    pub id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MulticaRuntimeLocalSkillSummary {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub plugin: Option<String>,
    #[serde(default)]
    pub can_disable: bool,
    #[serde(default)]
    pub file_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaRuntimeLocalSkillInventory {
    pub workspace_id: String,
    pub runtime_id: String,
    pub supported: bool,
    pub skills: Vec<MulticaRuntimeLocalSkillSummary>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ManagedRuntimeLocalSkillRequest {
    id: String,
    runtime_id: String,
    status: String,
    #[serde(default)]
    skills: Vec<MulticaRuntimeLocalSkillSummary>,
    #[serde(default)]
    supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MulticaWorkspaceSquad {
    pub id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub leader_id: Option<String>,
    #[serde(default)]
    pub member_count: Option<u64>,
    #[serde(default)]
    pub archived_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MulticaWorkspaceAutopilot {
    pub id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub execution_mode: Option<String>,
    #[serde(default)]
    pub assignee_type: Option<String>,
    #[serde(default)]
    pub assignee_id: Option<String>,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub next_run_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaWorkspaceMeResponse {
    pub workspace_id: String,
    pub user: MulticaWorkspaceUser,
}

macro_rules! managed_workspace_list_response {
    ($name:ident, $field:ident, $item:ty) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            pub workspace_id: String,
            pub $field: Vec<$item>,
            pub total: u64,
            pub next_cursor: Option<String>,
        }
    };
}

managed_workspace_list_response!(
    MulticaWorkspaceIssuesResponse,
    issues,
    MulticaWorkspaceIssue
);
managed_workspace_list_response!(
    MulticaWorkspaceProjectsResponse,
    projects,
    MulticaWorkspaceProject
);
managed_workspace_list_response!(
    MulticaWorkspaceAgentsResponse,
    agents,
    MulticaWorkspaceAgent
);
managed_workspace_list_response!(
    MulticaWorkspaceRuntimesResponse,
    runtimes,
    MulticaWorkspaceRuntime
);
managed_workspace_list_response!(
    MulticaWorkspaceSkillsResponse,
    skills,
    MulticaWorkspaceSkill
);
managed_workspace_list_response!(
    MulticaWorkspaceSquadsResponse,
    squads,
    MulticaWorkspaceSquad
);
managed_workspace_list_response!(
    MulticaWorkspaceAutopilotsResponse,
    autopilots,
    MulticaWorkspaceAutopilot
);

/// Credential-bearing workspace client. It intentionally implements neither
/// `Debug` nor `Serialize`; the authorization header remains Core-private and
/// is marked sensitive for reqwest/header diagnostics.
pub struct MulticaManagedWorkspaceClient {
    client: Client,
    server_origin: Url,
    _app_origin: Url,
    workspace_id: String,
    authorization: HeaderValue,
}

/// The persisted portion of managed runtime state.  It deliberately contains
/// no credentials, user URLs, process IDs, or complete local paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedRuntimeMetadata {
    version: String,
    target_triple: String,
    asset_name: String,
    binary_name: String,
    /// Digest of the extracted executable. `sha256` below remains the digest
    /// of the published archive so the release checksum can be audited.
    #[serde(default)]
    binary_sha256: String,
    sha256: String,
    asset_source: String,
    directory_name: String,
    updated_at_ms: u64,
}

/// Persist only a stable error code and timestamp. Raw anyhow messages can
/// contain URLs, paths, response fragments, or operating-system details and
/// therefore never cross this boundary or reach the renderer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedInstallFailureRecord {
    code: String,
    updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedRuntimeOwner {
    pid: u32,
    version: String,
    executable: String,
    profile: String,
    connection_id: String,
    started_at_ms: u64,
}

#[derive(Debug)]
struct ManagedLifecycleLock {
    file: fs::File,
    owner_path: PathBuf,
}

struct ManagedLifecycleLease {
    file: fs::File,
    owner_path: PathBuf,
    owner: ManagedRuntimeOwner,
}

impl ManagedLifecycleLock {
    fn activate(self, owner: ManagedRuntimeOwner) -> anyhow::Result<ManagedLifecycleLease> {
        validate_managed_owner(&owner)?;
        let bytes = serde_json::to_vec_pretty(&owner)
            .map_err(|_| anyhow!("managed_runtime_owner_serialize_failed"))?;
        crate::settings::atomic_write(&self.owner_path, &bytes)
            .map_err(|_| anyhow!("managed_runtime_owner_write_failed"))?;
        Ok(ManagedLifecycleLease {
            file: self.file,
            owner_path: self.owner_path,
            owner,
        })
    }
}

impl Drop for ManagedLifecycleLease {
    fn drop(&mut self) {
        let remove =
            read_managed_owner(&self.owner_path).ok().flatten().as_ref() == Some(&self.owner);
        if remove {
            let _ = fs::remove_file(&self.owner_path);
        }
        let _ = FileExt::unlock(&self.file);
    }
}

fn managed_asset_for_target(target_triple: &str) -> Option<MulticaRuntimeAsset> {
    let (target_triple, asset_name, digest) = match target_triple {
        "x86_64-pc-windows-msvc" => (
            "x86_64-pc-windows-msvc",
            "multica-cli-0.4.36-windows-amd64.zip",
            MANAGED_RUNTIME_WINDOWS_AMD64_SHA256,
        ),
        "aarch64-pc-windows-msvc" => (
            "aarch64-pc-windows-msvc",
            "multica-cli-0.4.36-windows-arm64.zip",
            MANAGED_RUNTIME_WINDOWS_ARM64_SHA256,
        ),
        "x86_64-apple-darwin" => (
            "x86_64-apple-darwin",
            "multica-cli-0.4.36-darwin-amd64.tar.gz",
            MANAGED_RUNTIME_DARWIN_AMD64_SHA256,
        ),
        "aarch64-apple-darwin" => (
            "aarch64-apple-darwin",
            "multica-cli-0.4.36-darwin-arm64.tar.gz",
            MANAGED_RUNTIME_DARWIN_ARM64_SHA256,
        ),
        "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl" => (
            "x86_64-unknown-linux-gnu",
            "multica-cli-0.4.36-linux-amd64.tar.gz",
            MANAGED_RUNTIME_LINUX_AMD64_SHA256,
        ),
        "aarch64-unknown-linux-gnu" | "aarch64-unknown-linux-musl" => (
            "aarch64-unknown-linux-gnu",
            "multica-cli-0.4.36-linux-arm64.tar.gz",
            MANAGED_RUNTIME_LINUX_ARM64_SHA256,
        ),
        _ => return None,
    };
    Some(MulticaRuntimeAsset {
        version: MANAGED_RUNTIME_VERSION,
        target_triple,
        asset_name,
        binary_name: if target_triple.contains("windows") {
            "multica.exe"
        } else {
            "multica"
        },
        expected_sha256: digest,
    })
}

fn current_target_triple() -> &'static str {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "aarch64-pc-windows-msvc"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-unknown-linux-gnu"
    } else {
        "unsupported"
    }
}

/// Return the fixed asset for this build target, if supported.
pub fn managed_runtime_asset() -> Option<MulticaRuntimeAsset> {
    managed_asset_for_target(current_target_triple())
}

/// Return the managed connection defaults without touching disk.  This is
/// useful to render an unconfigured state and is intentionally side-effect
/// free.
pub fn managed_connection_defaults() -> MulticaManagedConnection {
    MulticaManagedConnection::default()
}

fn managed_runtime_root() -> PathBuf {
    crate::paths::default_multica_state_dir().join(MANAGED_RUNTIME_STATE_DIR)
}

/// Private installation root, exposed for diagnostics only through a stable
/// directory name.  Callers should use `managed_runtime_status` rather than
/// depending on this path.
pub fn managed_runtime_state_dir() -> PathBuf {
    managed_runtime_root()
}

fn managed_current_path(root: &Path) -> PathBuf {
    root.join(MANAGED_RUNTIME_CURRENT_FILE)
}

fn managed_previous_path(root: &Path) -> PathBuf {
    root.join(MANAGED_RUNTIME_PREVIOUS_FILE)
}

fn managed_install_failure_path(root: &Path) -> PathBuf {
    root.join(MANAGED_RUNTIME_INSTALL_FAILURE_FILE)
}

fn managed_lock_path(root: &Path) -> PathBuf {
    root.join(MANAGED_RUNTIME_LOCK_FILE)
}

fn managed_lifecycle_lock_path(root: &Path) -> PathBuf {
    root.join(MANAGED_RUNTIME_LIFECYCLE_LOCK_FILE)
}

fn managed_owner_path(root: &Path) -> PathBuf {
    root.join(MANAGED_RUNTIME_OWNER_FILE)
}

fn managed_versions_dir(root: &Path) -> PathBuf {
    root.join(MANAGED_RUNTIME_VERSIONS_DIR)
}

fn validate_managed_owner(owner: &ManagedRuntimeOwner) -> anyhow::Result<()> {
    if owner.pid < 2
        || owner.started_at_ms == 0
        || !managed_metadata_component_is_valid(&owner.version)
        || owner.profile != MANAGED_RUNTIME_PROFILE
        || owner.connection_id != MANAGED_RUNTIME_CONNECTION_ID
        || owner.executable.is_empty()
        || !Path::new(&owner.executable).is_absolute()
    {
        bail!("managed_runtime_owner_invalid");
    }
    Ok(())
}

fn read_managed_owner(path: &Path) -> anyhow::Result<Option<ManagedRuntimeOwner>> {
    match fs::metadata(path) {
        Ok(metadata) if !metadata.is_file() || metadata.len() > MANAGED_RUNTIME_MAX_OWNER_BYTES => {
            bail!("managed_runtime_owner_invalid");
        }
        Ok(_) => {
            let bytes = fs::read(path).map_err(|_| anyhow!("managed_runtime_owner_read_failed"))?;
            let owner: ManagedRuntimeOwner = serde_json::from_slice(&bytes)
                .map_err(|_| anyhow!("managed_runtime_owner_invalid"))?;
            validate_managed_owner(&owner)?;
            Ok(Some(owner))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => bail!("managed_runtime_owner_read_failed"),
    }
}

#[cfg(windows)]
fn managed_owner_process_exists(process_id: u32) -> bool {
    crate::windows_process_exists(process_id)
}

#[cfg(unix)]
fn managed_owner_process_exists(process_id: u32) -> bool {
    let Ok(process_id) = i32::try_from(process_id) else {
        return false;
    };
    if process_id < 2 {
        return false;
    }
    unsafe extern "C" {
        fn kill(process: i32, signal: i32) -> i32;
    }
    if unsafe { kill(process_id, 0) } == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(1)
}

#[cfg(not(any(windows, unix)))]
fn managed_owner_process_exists(_process_id: u32) -> bool {
    false
}

fn cleanup_stale_managed_owner_with(
    root: &Path,
    mut process_exists: impl FnMut(u32) -> bool,
    mut query_executable: impl FnMut(u32) -> Option<PathBuf>,
    mut terminate_process_tree: impl FnMut(u32) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let owner_path = managed_owner_path(root);
    let Some(owner) = read_managed_owner(&owner_path)? else {
        return Ok(());
    };
    if !process_exists(owner.pid) {
        fs::remove_file(&owner_path)
            .map_err(|_| anyhow!("managed_runtime_owner_cleanup_failed"))?;
        return Ok(());
    }

    let Some(actual_executable) = query_executable(owner.pid) else {
        bail!("managed_runtime_owner_process_unverified");
    };
    if !same_executable_path(Path::new(&owner.executable), &actual_executable) {
        fs::remove_file(&owner_path)
            .map_err(|_| anyhow!("managed_runtime_owner_cleanup_failed"))?;
        return Ok(());
    }

    // The caller has already acquired the exclusive lifecycle lock. A live
    // process recorded here therefore survived its owning manager rather than
    // belonging to another healthy CCP instance. Kill only the revalidated
    // process group and wait for a bounded confirmation before removing the
    // owner record.
    terminate_process_tree(owner.pid)
        .map_err(|_| anyhow!("managed_runtime_stale_owner_stop_failed"))?;
    let deadline = Instant::now() + MANAGED_RUNTIME_STALE_OWNER_WAIT;
    while process_exists(owner.pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    if process_exists(owner.pid) {
        bail!("managed_runtime_stale_owner_stop_failed");
    }
    fs::remove_file(&owner_path).map_err(|_| anyhow!("managed_runtime_owner_cleanup_failed"))?;
    Ok(())
}

fn cleanup_stale_managed_owner(root: &Path) -> anyhow::Result<()> {
    cleanup_stale_managed_owner_with(
        root,
        managed_owner_process_exists,
        query_process_executable_path,
        terminate_sidecar_process_tree,
    )
}

fn acquire_managed_lifecycle_lock_at(root: &Path) -> anyhow::Result<ManagedLifecycleLock> {
    crate::settings::create_private_dir_all(root)
        .map_err(|_| anyhow!("managed_runtime_lifecycle_lock_unavailable"))?;
    let file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(managed_lifecycle_lock_path(root))
        .map_err(|_| anyhow!("managed_runtime_lifecycle_lock_unavailable"))?;
    file.try_lock_exclusive()
        .map_err(|_| anyhow!("managed_runtime_owned_by_other_manager"))?;
    cleanup_stale_managed_owner(root)?;
    Ok(ManagedLifecycleLock {
        file,
        owner_path: managed_owner_path(root),
    })
}

fn acquire_managed_lifecycle_lock() -> anyhow::Result<ManagedLifecycleLock> {
    acquire_managed_lifecycle_lock_at(&managed_runtime_root())
}

fn managed_owner_for_process(
    executable: &Path,
    pid: u32,
    started_at_ms: u64,
) -> anyhow::Result<ManagedRuntimeOwner> {
    let root = managed_runtime_root();
    let metadata = managed_metadata(&managed_current_path(&root))?
        .ok_or_else(|| anyhow!("managed_runtime_metadata_missing"))?;
    if !managed_metadata_is_verified(&root, &metadata) {
        bail!("managed_runtime_owner_version_unverified");
    }
    let expected = fs::canonicalize(managed_executable_for_metadata(&root, &metadata))
        .map_err(|_| anyhow!("managed_runtime_owner_executable_unavailable"))?;
    if !same_executable_path(&expected, executable) {
        bail!("managed_runtime_owner_executable_mismatch");
    }
    Ok(ManagedRuntimeOwner {
        pid,
        version: metadata.version,
        executable: executable.to_string_lossy().to_string(),
        profile: MANAGED_RUNTIME_PROFILE.to_string(),
        connection_id: MANAGED_RUNTIME_CONNECTION_ID.to_string(),
        started_at_ms,
    })
}

fn managed_metadata(path: &Path) -> anyhow::Result<Option<ManagedRuntimeMetadata>> {
    match fs::metadata(path) {
        Ok(metadata)
            if !metadata.is_file()
                || metadata.len() > MANAGED_RUNTIME_MAX_METADATA_BYTES as u64 =>
        {
            bail!("managed_runtime_metadata_invalid");
        }
        Ok(_) => {
            let bytes = fs::read(path).context("读取托管 Multica 安装元数据失败。")?;
            Ok(Some(
                serde_json::from_slice(&bytes).context("托管 Multica 安装元数据格式无效。")?,
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("读取托管 Multica 安装元数据失败。"),
    }
}

fn managed_executable_for_metadata(root: &Path, metadata: &ManagedRuntimeMetadata) -> PathBuf {
    managed_versions_dir(root)
        .join(&metadata.directory_name)
        .join(&metadata.binary_name)
}

fn managed_metadata_component_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

/// Validate the self-contained shape of persisted metadata without comparing
/// it to this build's pinned release. Historical entries were already created
/// from an allowlisted install, and must remain usable after that allowlist is
/// advanced to a newer version.
fn managed_metadata_shape_is_valid(metadata: &ManagedRuntimeMetadata) -> bool {
    managed_metadata_component_is_valid(&metadata.version)
        && managed_metadata_component_is_valid(&metadata.target_triple)
        && managed_metadata_component_is_valid(&metadata.asset_name)
        && managed_metadata_component_is_valid(&metadata.binary_name)
        && metadata
            .directory_name
            .starts_with(&format!("{}-", metadata.version))
        && managed_metadata_component_is_valid(&metadata.directory_name)
        && metadata.sha256.len() == 64
        && metadata.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        && metadata.binary_sha256.len() == 64
        && metadata
            .binary_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        && matches!(metadata.asset_source.as_str(), "bundled" | "github_release")
}

/// Only automatic install/upgrade decisions use the current build allowlist.
/// Status reads and explicit rollback deliberately use the metadata's own
/// release identity and recorded binary digest instead.
fn managed_metadata_matches_asset(
    metadata: &ManagedRuntimeMetadata,
    asset: MulticaRuntimeAsset,
) -> bool {
    managed_metadata_shape_is_valid(metadata)
        && managed_metadata_matches_target(metadata, asset)
        && metadata.version == asset.version
        && metadata.asset_name == asset.asset_name
        && metadata.sha256.eq_ignore_ascii_case(asset.expected_sha256)
}

fn managed_metadata_matches_target(
    metadata: &ManagedRuntimeMetadata,
    asset: MulticaRuntimeAsset,
) -> bool {
    metadata.target_triple == asset.target_triple && metadata.binary_name == asset.binary_name
}

fn managed_metadata_is_verified(root: &Path, metadata: &ManagedRuntimeMetadata) -> bool {
    managed_metadata_shape_is_valid(metadata)
        && sha256_file(
            &managed_executable_for_metadata(root, metadata),
            MANAGED_RUNTIME_MAX_BINARY_BYTES,
        )
        .map(|(hash, _)| hash.eq_ignore_ascii_case(&metadata.binary_sha256))
        .unwrap_or(false)
}

fn managed_metadata_error_code(metadata: &ManagedRuntimeMetadata) -> &'static str {
    if metadata.binary_sha256.trim().is_empty() {
        "managed_runtime_metadata_binary_digest_missing"
    } else if !managed_metadata_shape_is_valid(metadata) {
        "managed_runtime_metadata_invalid"
    } else {
        "verification_failed"
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sha256_file(path: &Path, limit: usize) -> anyhow::Result<(String, u64)> {
    let mut file = fs::File::open(path).with_context(|| "托管 Multica 可执行文件无法读取。")?;
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or_else(|| anyhow!("managed_runtime_size_limit"))?;
        if total > limit as u64 {
            bail!("managed_runtime_size_limit");
        }
        hasher.update(&buffer[..read]);
    }
    Ok((
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        total,
    ))
}

fn managed_status_from_metadata(
    root: &Path,
    asset: Option<MulticaRuntimeAsset>,
    metadata: Option<ManagedRuntimeMetadata>,
) -> MulticaRuntimeInstallStatus {
    let Some(asset) = asset else {
        return MulticaRuntimeInstallStatus {
            install_state: "unsupported_platform".to_string(),
            install_phase: Some("unsupported".to_string()),
            progress_percent: Some(0),
            target_triple: Some(current_target_triple().to_string()),
            last_install_error_code: Some("unsupported_platform".to_string()),
            updated_at_ms: Some(now_ms()),
            ..Default::default()
        };
    };
    let Some(metadata) = metadata else {
        return MulticaRuntimeInstallStatus {
            install_state: "not_installed".to_string(),
            install_phase: Some("idle".to_string()),
            progress_percent: Some(0),
            installed_version: None,
            target_triple: Some(asset.target_triple.to_string()),
            asset_name: Some(asset.asset_name.to_string()),
            asset_source: None,
            sha256: None,
            sha256_verified: false,
            executable_name: Some(asset.binary_name.to_string()),
            updated_at_ms: None,
            ..Default::default()
        };
    };
    let valid_metadata = managed_metadata_matches_target(&metadata, asset)
        && managed_metadata_is_verified(root, &metadata);
    let error_code = (!valid_metadata).then(|| managed_metadata_error_code(&metadata));
    MulticaRuntimeInstallStatus {
        install_state: if valid_metadata {
            "ready".to_string()
        } else {
            "verification_failed".to_string()
        },
        install_phase: Some(if valid_metadata {
            "ready".to_string()
        } else {
            "failed".to_string()
        }),
        downloaded_bytes: 0,
        total_bytes: None,
        progress_percent: Some(if valid_metadata { 100 } else { 0 }),
        installed_version: Some(metadata.version),
        target_triple: Some(metadata.target_triple),
        asset_name: Some(metadata.asset_name),
        asset_source: Some(metadata.asset_source),
        sha256: Some(metadata.sha256),
        sha256_verified: valid_metadata,
        executable_name: Some(metadata.binary_name),
        previous_version: None,
        last_install_error_code: error_code.map(str::to_string),
        updated_at_ms: Some(metadata.updated_at_ms),
        diagnostic: error_code.map(|code| match code {
            "managed_runtime_metadata_binary_digest_missing" => code.to_string(),
            _ => "managed_runtime_artifact_invalid".to_string(),
        }),
    }
}

fn overlay_managed_install_progress(
    mut status: MulticaRuntimeInstallStatus,
) -> MulticaRuntimeInstallStatus {
    let progress = managed_install_progress_slot()
        .lock()
        .ok()
        .and_then(|value| value.clone());
    let Some(progress) = progress else {
        return status;
    };

    status.install_state = "installing".to_string();
    status.install_phase = Some(progress.phase);
    status.downloaded_bytes = progress.downloaded_bytes;
    status.total_bytes = progress.total_bytes;
    status.progress_percent = progress.progress_percent;
    status.installed_version = progress.version.or(status.installed_version);
    status.target_triple = progress.target_triple.or(status.target_triple);
    status.asset_name = progress.asset_name.or(status.asset_name);
    status.asset_source = Some("pending".to_string());
    status.sha256_verified = false;
    status.updated_at_ms = Some(progress.updated_at_ms);
    status
}

/// Read the current managed runtime pointer and verify the binary digest.
/// This function performs no installation or network access.
pub fn managed_runtime_status() -> anyhow::Result<MulticaRuntimeInstallStatus> {
    let root = managed_runtime_root();
    managed_runtime_status_at(&root)
}

fn managed_runtime_status_at(root: &Path) -> anyhow::Result<MulticaRuntimeInstallStatus> {
    let asset = managed_runtime_asset();
    let mut status = match managed_metadata(&managed_current_path(root)) {
        Ok(metadata) => managed_status_from_metadata(root, asset, metadata),
        Err(_) => {
            let mut invalid =
                managed_install_status_error("managed_runtime_metadata_invalid", asset);
            invalid.install_state = "verification_failed".to_string();
            invalid
        }
    };
    if let Some(previous) = managed_metadata(&managed_previous_path(root))
        .ok()
        .flatten()
        .filter(|previous| {
            asset.is_some_and(|asset| {
                managed_metadata_matches_target(previous, asset)
                    && managed_metadata_is_verified(root, previous)
            })
        })
    {
        status.previous_version = Some(previous.version);
    }
    if let Some(failure) = read_managed_install_failure(root)? {
        status = apply_managed_install_failure(status, &failure);
    }
    Ok(overlay_managed_install_progress(status))
}

/// Alias used by the Manager command layer.
pub fn get_managed_runtime_status() -> anyhow::Result<MulticaRuntimeInstallStatus> {
    managed_runtime_status()
}

fn managed_release_url(asset_name: &str) -> anyhow::Result<Url> {
    // The path is built from compile-time constants and an allowlisted asset;
    // callers cannot supply an arbitrary download URL.
    let asset = managed_runtime_asset().ok_or_else(|| anyhow!("unsupported_platform"))?;
    if asset.asset_name != asset_name {
        bail!("managed_runtime_asset_not_allowed");
    }
    Url::parse(&format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        MANAGED_RUNTIME_REPOSITORY_OWNER,
        MANAGED_RUNTIME_REPOSITORY_NAME,
        MANAGED_RUNTIME_RELEASE_TAG,
        asset.asset_name
    ))
    .map_err(|_| anyhow!("managed_runtime_url_invalid"))
}

fn managed_checksums_url() -> anyhow::Result<Url> {
    Url::parse(&format!(
        "https://github.com/{}/{}/releases/download/{}/checksums.txt",
        MANAGED_RUNTIME_REPOSITORY_OWNER,
        MANAGED_RUNTIME_REPOSITORY_NAME,
        MANAGED_RUNTIME_RELEASE_TAG
    ))
    .map_err(|_| anyhow!("managed_runtime_url_invalid"))
}

fn allowed_release_host(host: &str) -> bool {
    matches!(
        host.to_ascii_lowercase().as_str(),
        "github.com"
            | "api.github.com"
            | "objects.githubusercontent.com"
            | "github-releases.githubusercontent.com"
            | "release-assets.githubusercontent.com"
    )
}

fn validate_release_url(url: &Url) -> anyhow::Result<()> {
    if url.scheme() != "https"
        || url.username() != ""
        || url.password().is_some()
        || !url.host_str().is_some_and(allowed_release_host)
    {
        bail!("managed_runtime_redirect_rejected");
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ManagedInstallProgressSpec {
    asset: MulticaRuntimeAsset,
    phase: &'static str,
}

async fn download_release_body_with_progress(
    client: &Client,
    initial: Url,
    limit: usize,
    cancel: Option<&AtomicBool>,
    progress: Option<ManagedInstallProgressSpec>,
) -> anyhow::Result<Vec<u8>> {
    retry_managed_download_operation(
        || download_release_body_attempt(client, initial.clone(), limit, cancel, progress),
        cancel,
        &MANAGED_RUNTIME_DOWNLOAD_RETRY_BACKOFFS,
    )
    .await
}

async fn retry_managed_download_operation<T, F, Fut>(
    mut operation: F,
    cancel: Option<&AtomicBool>,
    backoffs: &[Duration],
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    debug_assert_eq!(backoffs.len() + 1, MANAGED_RUNTIME_MAX_DOWNLOAD_ATTEMPTS);
    for attempt in 0..=backoffs.len() {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error)
                if managed_download_error_is_retryable(&error) && attempt < backoffs.len() =>
            {
                wait_for_managed_download_retry(backoffs[attempt], cancel).await?;
            }
            Err(error) => return Err(error),
        }
    }
    bail!("managed_runtime_download_network_error")
}

fn managed_download_error_is_retryable(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        matches!(
            cause.to_string().as_str(),
            "managed_runtime_download_timeout"
                | "managed_runtime_download_network_error"
                | "managed_runtime_http_retryable"
        )
    })
}

fn managed_http_status_is_retryable(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

async fn wait_for_managed_download_retry(
    delay: Duration,
    cancel: Option<&AtomicBool>,
) -> anyhow::Result<()> {
    tokio::select! {
        _ = tokio::time::sleep(delay) => Ok(()),
        _ = wait_for_managed_install_cancellation(cancel) => {
            Err(anyhow!("managed_runtime_install_cancelled"))
        }
    }
}

async fn download_release_body_attempt(
    client: &Client,
    initial: Url,
    limit: usize,
    cancel: Option<&AtomicBool>,
    progress: Option<ManagedInstallProgressSpec>,
) -> anyhow::Result<Vec<u8>> {
    check_managed_install_cancelled(cancel)?;
    validate_release_url(&initial)?;
    let mut url = initial;
    for _ in 0..=MANAGED_RUNTIME_MAX_REDIRECTS {
        check_managed_install_cancelled(cancel)?;
        let request = client
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "application/octet-stream");
        let response = await_managed_download(request.send(), cancel)
            .await
            .map_err(|error| match error {
                ManagedDownloadWaitError::Cancelled => anyhow!("managed_runtime_install_cancelled"),
                ManagedDownloadWaitError::Transport(error) if error.is_timeout() => {
                    anyhow!("managed_runtime_download_timeout")
                }
                ManagedDownloadWaitError::Transport(_) => {
                    anyhow!("managed_runtime_download_network_error")
                }
            })?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| anyhow!("managed_runtime_redirect_invalid"))?;
            let next = url
                .join(location)
                .map_err(|_| anyhow!("managed_runtime_redirect_invalid"))?;
            validate_release_url(&next)?;
            url = next;
            continue;
        }
        if !response.status().is_success() {
            if managed_http_status_is_retryable(response.status()) {
                bail!("managed_runtime_http_retryable");
            }
            bail!("managed_runtime_http_status");
        }
        return read_response_body_with_limit_with_cancel_and_progress(
            response, limit, cancel, progress,
        )
        .await
        .map_err(|error| anyhow!(error));
    }
    bail!("managed_runtime_too_many_redirects")
}

async fn read_response_body_with_limit_with_cancel_and_progress(
    mut response: reqwest::Response,
    limit: usize,
    cancel: Option<&AtomicBool>,
    progress: Option<ManagedInstallProgressSpec>,
) -> Result<Vec<u8>, &'static str> {
    if cancel.is_some_and(|token| token.load(Ordering::Acquire)) {
        return Err("managed_runtime_install_cancelled");
    }
    let total_bytes = response.content_length();
    if total_bytes.is_some_and(|length| length > limit as u64) {
        return Err("managed_runtime_response_too_large");
    }
    if let Some(progress) = progress {
        set_managed_install_progress(Some(progress.asset), progress.phase, 0, total_bytes, None);
    }
    let mut body = Vec::new();
    while let Some(chunk) = await_managed_download(response.chunk(), cancel)
        .await
        .map_err(|error| match error {
            ManagedDownloadWaitError::Cancelled => "managed_runtime_install_cancelled",
            ManagedDownloadWaitError::Transport(_) => "managed_runtime_download_network_error",
        })?
    {
        if cancel.is_some_and(|token| token.load(Ordering::Acquire)) {
            return Err("managed_runtime_install_cancelled");
        }
        let next_len = body
            .len()
            .checked_add(chunk.len())
            .ok_or("managed_runtime_response_too_large")?;
        if next_len > limit {
            return Err("managed_runtime_response_too_large");
        }
        body.extend_from_slice(&chunk);
        if let Some(progress) = progress {
            set_managed_install_progress(
                Some(progress.asset),
                progress.phase,
                body.len() as u64,
                total_bytes,
                None,
            );
        }
    }
    Ok(body)
}

fn parse_release_checksum(checksums: &[u8], asset_name: &str) -> anyhow::Result<String> {
    let text =
        std::str::from_utf8(checksums).map_err(|_| anyhow!("managed_runtime_checksum_invalid"))?;
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(digest) = fields.next() else {
            continue;
        };
        let Some(name) = fields.next() else { continue };
        let name = name.strip_prefix('*').unwrap_or(name);
        if name != asset_name {
            continue;
        }
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("managed_runtime_checksum_invalid");
        }
        return Ok(digest.to_ascii_lowercase());
    }
    bail!("managed_runtime_checksum_missing")
}

async fn download_managed_archive_with_cancel(
    asset: MulticaRuntimeAsset,
    cancel: Option<&AtomicBool>,
) -> anyhow::Result<Vec<u8>> {
    let client = build_client()?;
    let archive = download_release_body_with_progress(
        &client,
        managed_release_url(asset.asset_name)?,
        MANAGED_RUNTIME_MAX_ARCHIVE_BYTES,
        cancel,
        Some(ManagedInstallProgressSpec {
            asset,
            phase: "downloading_archive",
        }),
    )
    .await?;
    let checksums = download_release_body_with_progress(
        &client,
        managed_checksums_url()?,
        1024 * 1024,
        cancel,
        Some(ManagedInstallProgressSpec {
            asset,
            phase: "downloading_checksums",
        }),
    )
    .await?;
    check_managed_install_cancelled(cancel)?;
    let listed = parse_release_checksum(&checksums, asset.asset_name)?;
    if !listed.eq_ignore_ascii_case(asset.expected_sha256)
        || !sha256_hex(&archive).eq_ignore_ascii_case(asset.expected_sha256)
    {
        bail!("managed_runtime_checksum_mismatch");
    }
    Ok(archive)
}

fn managed_resource_candidates(asset: MulticaRuntimeAsset) -> Vec<PathBuf> {
    std::env::current_exe()
        .map(|executable| managed_resource_candidates_from_executable(asset, &executable))
        .unwrap_or_default()
}

fn managed_resource_candidates_from_executable(
    asset: MulticaRuntimeAsset,
    executable: &Path,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let Some(parent) = executable.parent() else {
        return candidates;
    };
    candidates.push(
        parent
            .join("resources")
            .join("multica")
            .join(asset.asset_name),
    );
    candidates.push(parent.join("multica").join(asset.asset_name));
    if let Some(grandparent) = parent.parent() {
        candidates.push(
            grandparent
                .join("resources")
                .join("multica")
                .join(asset.asset_name),
        );
        // A packaged macOS executable lives in Contents/MacOS while bundle
        // resources use the case-sensitive Contents/Resources directory.
        candidates.push(
            grandparent
                .join("Resources")
                .join("multica")
                .join(asset.asset_name),
        );
    }
    candidates
}

fn read_bounded_file(path: &Path, limit: usize) -> anyhow::Result<Vec<u8>> {
    let metadata = fs::metadata(path).with_context(|| "托管 Multica 资源不存在。")?;
    if !metadata.is_file() || metadata.len() > limit as u64 {
        bail!("managed_runtime_resource_invalid");
    }
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::with_capacity(metadata.len().min(limit as u64) as usize);
    file.read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        bail!("managed_runtime_resource_invalid");
    }
    Ok(bytes)
}

fn safe_archive_member(name: &str) -> anyhow::Result<String> {
    if name.is_empty() || name.contains('\\') || name.contains('\0') {
        bail!("managed_runtime_archive_path_invalid");
    }
    let path = Path::new(name);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        bail!("managed_runtime_archive_path_invalid");
    }
    let mut components = path.components();
    let Some(std::path::Component::Normal(first)) = components.next() else {
        bail!("managed_runtime_archive_path_invalid");
    };
    if components.next().is_some() {
        bail!("managed_runtime_archive_path_invalid");
    }
    Ok(first.to_string_lossy().to_string())
}

fn extract_managed_binary(archive: &[u8], asset: MulticaRuntimeAsset) -> anyhow::Result<Vec<u8>> {
    if asset.asset_name.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(Cursor::new(archive))
            .map_err(|_| anyhow!("managed_runtime_archive_invalid"))?;
        if zip.len() > MANAGED_RUNTIME_MAX_ARCHIVE_ENTRIES {
            bail!("managed_runtime_archive_too_many_entries");
        }
        let mut names = HashSet::new();
        let mut total_uncompressed = 0usize;
        let mut binary = None;
        for index in 0..zip.len() {
            let mut entry = zip
                .by_index(index)
                .map_err(|_| anyhow!("managed_runtime_archive_invalid"))?;
            let name = safe_archive_member(entry.name())?;
            if !names.insert(name.clone()) {
                bail!("managed_runtime_archive_duplicate_entry");
            }
            if entry.size() > MANAGED_RUNTIME_MAX_BINARY_BYTES as u64 {
                bail!("managed_runtime_archive_entry_too_large");
            }
            total_uncompressed = total_uncompressed
                .checked_add(entry.size() as usize)
                .ok_or_else(|| anyhow!("managed_runtime_archive_too_large"))?;
            if total_uncompressed > MANAGED_RUNTIME_MAX_BINARY_BYTES {
                bail!("managed_runtime_archive_too_large");
            }
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                bail!("managed_runtime_archive_symlink");
            }
            let allowed_metadata = matches!(
                name.as_str(),
                "LICENSE" | "NOTICE" | "README.md" | "README.zh.md"
            );
            if name != asset.binary_name && !allowed_metadata {
                bail!("managed_runtime_archive_member_unexpected");
            }
            if name == asset.binary_name {
                let mut bytes = Vec::with_capacity(entry.size() as usize);
                entry
                    .read_to_end(&mut bytes)
                    .map_err(|_| anyhow!("managed_runtime_archive_invalid"))?;
                if bytes.is_empty() {
                    bail!("managed_runtime_binary_empty");
                }
                binary = Some(bytes);
            }
        }
        return binary.ok_or_else(|| anyhow!("managed_runtime_binary_missing"));
    }

    if !asset.asset_name.ends_with(".tar.gz") {
        bail!("managed_runtime_archive_unsupported");
    }
    let decoder = GzDecoder::new(Cursor::new(archive));
    let mut tar = tar::Archive::new(decoder);
    let entries = tar
        .entries()
        .map_err(|_| anyhow!("managed_runtime_archive_invalid"))?;
    let mut names = HashSet::new();
    let mut binary = None;
    let mut count = 0usize;
    let mut total_uncompressed = 0usize;
    for entry in entries {
        count += 1;
        if count > MANAGED_RUNTIME_MAX_ARCHIVE_ENTRIES {
            bail!("managed_runtime_archive_too_many_entries");
        }
        let mut entry = entry.map_err(|_| anyhow!("managed_runtime_archive_invalid"))?;
        let path = entry
            .path()
            .map_err(|_| anyhow!("managed_runtime_archive_path_invalid"))?;
        let path = path
            .to_str()
            .ok_or_else(|| anyhow!("managed_runtime_archive_path_invalid"))?;
        let name = safe_archive_member(path)?;
        if !names.insert(name.clone()) {
            bail!("managed_runtime_archive_duplicate_entry");
        }
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() || !kind.is_file() {
            bail!("managed_runtime_archive_link_or_special");
        }
        let size = entry
            .header()
            .size()
            .map_err(|_| anyhow!("managed_runtime_archive_invalid"))?;
        if size > MANAGED_RUNTIME_MAX_BINARY_BYTES as u64 {
            bail!("managed_runtime_archive_entry_too_large");
        }
        let size =
            usize::try_from(size).map_err(|_| anyhow!("managed_runtime_archive_too_large"))?;
        total_uncompressed = total_uncompressed
            .checked_add(size)
            .ok_or_else(|| anyhow!("managed_runtime_archive_too_large"))?;
        if total_uncompressed > MANAGED_RUNTIME_MAX_BINARY_BYTES {
            bail!("managed_runtime_archive_too_large");
        }
        let allowed_metadata = matches!(
            name.as_str(),
            "LICENSE" | "NOTICE" | "README.md" | "README.zh.md"
        );
        if name != asset.binary_name && !allowed_metadata {
            bail!("managed_runtime_archive_member_unexpected");
        }
        if name == asset.binary_name {
            let mut bytes = Vec::with_capacity(size);
            entry
                .read_to_end(&mut bytes)
                .map_err(|_| anyhow!("managed_runtime_archive_invalid"))?;
            if bytes.len() != size {
                bail!("managed_runtime_archive_invalid");
            }
            if bytes.is_empty() {
                bail!("managed_runtime_binary_empty");
            }
            binary = Some(bytes);
        }
    }
    binary.ok_or_else(|| anyhow!("managed_runtime_binary_missing"))
}

fn write_synced_file_with_cancel(
    path: &Path,
    bytes: &[u8],
    cancel: Option<&AtomicBool>,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options.open(path)?;
    for chunk in bytes.chunks(64 * 1024) {
        check_managed_install_cancelled(cancel)?;
        file.write_all(chunk)?;
    }
    file.sync_all()?;
    Ok(())
}

fn mark_managed_binary_executable(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn managed_metadata_bytes(metadata: &ManagedRuntimeMetadata) -> anyhow::Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(metadata)?)
}

fn read_optional_managed_file(path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("读取托管 Multica 状态文件失败。"),
    }
}

fn restore_managed_pointer(path: &Path, original: Option<&[u8]>) -> anyhow::Result<()> {
    match original {
        Some(bytes) => match crate::settings::atomic_write(path, bytes) {
            Ok(()) => Ok(()),
            Err(_) if fs::read(path).is_ok_and(|restored| restored == bytes) => Ok(()),
            Err(error) => Err(error),
        },
        None => match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        },
    }
}

fn restore_managed_activation_pointers(
    current_path: &Path,
    old_current: Option<&[u8]>,
    previous_path: &Path,
    old_previous: Option<&[u8]>,
) -> anyhow::Result<()> {
    restore_managed_pointer(current_path, old_current)?;
    restore_managed_pointer(previous_path, old_previous)
}

fn managed_install_status_error(
    code: &str,
    asset: Option<MulticaRuntimeAsset>,
) -> MulticaRuntimeInstallStatus {
    MulticaRuntimeInstallStatus {
        install_state: if code == "unsupported_platform" {
            "unsupported_platform".to_string()
        } else if code.contains("checksum")
            || code.contains("verification")
            || code.contains("archive")
            || code.contains("resource")
            || code.contains("version")
        {
            "verification_failed".to_string()
        } else {
            "download_failed".to_string()
        },
        install_phase: Some(if code == "unsupported_platform" {
            "unsupported".to_string()
        } else {
            "failed".to_string()
        }),
        progress_percent: Some(0),
        target_triple: asset.map(|value| value.target_triple.to_string()),
        asset_name: asset.map(|value| value.asset_name.to_string()),
        executable_name: asset.map(|value| value.binary_name.to_string()),
        sha256_verified: false,
        last_install_error_code: Some(code.to_string()),
        updated_at_ms: Some(now_ms()),
        diagnostic: Some(code.to_string()),
        ..Default::default()
    }
}

fn managed_install_error_code_is_stable(code: &str) -> bool {
    (code == "unsupported_platform" || code.starts_with("managed_runtime_"))
        && code.len() <= 96
        && code
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn stable_managed_install_error_code(error: &anyhow::Error) -> String {
    error
        .chain()
        .map(|cause| cause.to_string())
        .find(|code| managed_install_error_code_is_stable(code))
        .unwrap_or_else(|| "managed_runtime_install_failed".to_string())
}

fn read_managed_install_failure(
    root: &Path,
) -> anyhow::Result<Option<ManagedInstallFailureRecord>> {
    let path = managed_install_failure_path(root);
    let Some(bytes) = read_optional_managed_file(&path)? else {
        return Ok(None);
    };
    if bytes.len() > MANAGED_RUNTIME_MAX_METADATA_BYTES {
        return Ok(None);
    }
    let record = match serde_json::from_slice::<ManagedInstallFailureRecord>(&bytes) {
        Ok(record)
            if record.updated_at_ms > 0 && managed_install_error_code_is_stable(&record.code) =>
        {
            record
        }
        _ => return Ok(None),
    };
    Ok(Some(record))
}

fn persist_managed_install_failure(
    root: &Path,
    code: &str,
) -> anyhow::Result<ManagedInstallFailureRecord> {
    let code = if managed_install_error_code_is_stable(code) {
        code.to_string()
    } else {
        "managed_runtime_install_failed".to_string()
    };
    let record = ManagedInstallFailureRecord {
        code,
        updated_at_ms: now_ms(),
    };
    let bytes = serde_json::to_vec_pretty(&record)
        .map_err(|_| anyhow!("managed_runtime_install_status_serialize_failed"))?;
    crate::settings::atomic_write(&managed_install_failure_path(root), &bytes)
        .map_err(|_| anyhow!("managed_runtime_install_status_write_failed"))?;
    Ok(record)
}

fn clear_managed_install_failure(root: &Path) -> anyhow::Result<()> {
    restore_managed_pointer(&managed_install_failure_path(root), None)
        .map_err(|_| anyhow!("managed_runtime_install_status_clear_failed"))
}

fn apply_managed_install_failure(
    mut status: MulticaRuntimeInstallStatus,
    record: &ManagedInstallFailureRecord,
) -> MulticaRuntimeInstallStatus {
    if record.code == "managed_runtime_install_cancelled" {
        status.install_state = "cancelled".to_string();
    } else if status.install_state == "not_installed" {
        status.install_state =
            managed_install_status_error(&record.code, managed_runtime_asset()).install_state;
    }
    status.install_phase = Some("failed".to_string());
    status.progress_percent = Some(if status.install_state == "ready" {
        100
    } else {
        0
    });
    status.last_install_error_code = Some(record.code.clone());
    status.updated_at_ms = Some(record.updated_at_ms);
    status.diagnostic = Some(record.code.clone());
    status
}

fn managed_install_failure_status_at(root: &Path, code: &str) -> MulticaRuntimeInstallStatus {
    let record = persist_managed_install_failure(root, code).unwrap_or_else(|_| {
        ManagedInstallFailureRecord {
            code: if managed_install_error_code_is_stable(code) {
                code.to_string()
            } else {
                "managed_runtime_install_failed".to_string()
            },
            updated_at_ms: now_ms(),
        }
    });
    let status = managed_runtime_status_at(root)
        .unwrap_or_else(|_| managed_install_status_error(&record.code, managed_runtime_asset()));
    apply_managed_install_failure(status, &record)
}

fn acquire_managed_install_lock(root: &Path) -> anyhow::Result<fs::File> {
    crate::settings::create_private_dir_all(root)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(managed_lock_path(root))
        .context("托管 Multica 安装锁无法打开。")?;
    lock.lock_exclusive()
        .context("托管 Multica 安装锁不可用。")?;
    Ok(lock)
}

fn run_async_without_nested_runtime<F, T>(future: F) -> anyhow::Result<T>
where
    F: Future<Output = anyhow::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    // Manager commands may call the synchronous API from an async Tauri
    // worker.  Running a second Tokio runtime on that thread panics, so use a
    // short-lived helper thread whenever a runtime is already active.
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::Builder::new()
            .name("ccp-multica-managed-download".to_string())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()
                    .map_err(|_| anyhow!("managed_runtime_runtime_init_failed"))?;
                runtime.block_on(future)
            })
            .map_err(|_| anyhow!("managed_runtime_runtime_init_failed"))?
            .join()
            .map_err(|_| anyhow!("managed_runtime_runtime_init_failed"))?
    } else {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .map_err(|_| anyhow!("managed_runtime_runtime_init_failed"))?;
        runtime.block_on(future)
    }
}

/// Validate and install one already-downloaded archive.  This is the common
/// path for bundled resources and the official Release fallback.  The caller
/// must hold the process-wide/cross-process install lock.
#[cfg(test)]
fn install_managed_archive_locked(
    root: &Path,
    asset: MulticaRuntimeAsset,
    archive: &[u8],
    source: &str,
    verify_binary: bool,
) -> anyhow::Result<MulticaRuntimeInstallStatus> {
    install_managed_archive_locked_with_context(
        root,
        asset,
        archive,
        source,
        verify_binary,
        None,
        None,
        None,
    )
}

#[cfg(test)]
fn install_managed_archive_locked_with_cancel(
    root: &Path,
    asset: MulticaRuntimeAsset,
    archive: &[u8],
    source: &str,
    verify_binary: bool,
    cancel: Option<&AtomicBool>,
) -> anyhow::Result<MulticaRuntimeInstallStatus> {
    install_managed_archive_locked_with_context(
        root,
        asset,
        archive,
        source,
        verify_binary,
        cancel,
        None,
        None,
    )
}

fn install_managed_archive_locked_with_context(
    root: &Path,
    asset: MulticaRuntimeAsset,
    archive: &[u8],
    source: &str,
    verify_binary: bool,
    cancel: Option<&AtomicBool>,
    connection_store: Option<&MulticaStore>,
    after_previous_write: Option<&dyn Fn()>,
) -> anyhow::Result<MulticaRuntimeInstallStatus> {
    check_managed_install_cancelled(cancel)?;
    if archive.len() > MANAGED_RUNTIME_MAX_ARCHIVE_BYTES {
        bail!("managed_runtime_response_too_large");
    }
    let archive_size = archive.len() as u64;
    set_managed_install_progress(Some(asset), "verifying", 0, Some(archive_size), Some(10));
    if !sha256_hex(archive).eq_ignore_ascii_case(asset.expected_sha256) {
        bail!("managed_runtime_checksum_mismatch");
    }
    set_managed_install_progress(
        Some(asset),
        "extracting",
        archive_size,
        Some(archive_size),
        Some(55),
    );
    let binary = extract_managed_binary(archive, asset)?;
    check_managed_install_cancelled(cancel)?;
    if binary.len() > MANAGED_RUNTIME_MAX_BINARY_BYTES {
        bail!("managed_runtime_binary_too_large");
    }

    let versions = managed_versions_dir(root);
    crate::settings::create_private_dir_all(&versions)?;
    let nonce = NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
    let staging = root.join(format!(".staging-{nonce}-{}", now_ms()));
    crate::settings::create_private_dir_all(&staging)?;
    let staged_binary = staging.join(asset.binary_name);
    let mut activated_dir_path = None;
    let result = (|| -> anyhow::Result<MulticaRuntimeInstallStatus> {
        set_managed_install_progress(
            Some(asset),
            "staging",
            archive_size,
            Some(archive_size),
            Some(70),
        );
        write_synced_file_with_cancel(&staged_binary, &binary, cancel)?;
        mark_managed_binary_executable(&staged_binary)?;
        check_managed_install_cancelled(cancel)?;
        if verify_binary {
            set_managed_install_progress(
                Some(asset),
                "probing",
                archive_size,
                Some(archive_size),
                Some(85),
            );
            verify_managed_binary_version(&staged_binary, asset.version)?;
        }

        check_managed_install_cancelled(cancel)?;

        set_managed_install_progress(
            Some(asset),
            "activating",
            archive_size,
            Some(archive_size),
            Some(95),
        );

        let directory_name = format!(
            "{}-{}-{}",
            asset.version,
            &asset.expected_sha256[..12],
            nonce
        );
        let activated_dir = versions.join(&directory_name);
        fs::rename(&staging, &activated_dir).context("托管 Multica 版本目录激活失败。")?;
        activated_dir_path = Some(activated_dir.clone());

        let current_path = managed_current_path(root);
        let previous_path = managed_previous_path(root);
        let old_current = read_optional_managed_file(&current_path)?;
        let old_previous = read_optional_managed_file(&previous_path)?;
        // A malformed current pointer is exactly one of the states a verified
        // reinstall must repair. Preserve its raw bytes for compensation, but
        // do not require it to deserialize merely to activate the replacement.
        let old_metadata = managed_metadata(&current_path)
            .ok()
            .flatten()
            .filter(|metadata| managed_metadata_is_verified(root, metadata));
        let metadata = ManagedRuntimeMetadata {
            version: asset.version.to_string(),
            target_triple: asset.target_triple.to_string(),
            asset_name: asset.asset_name.to_string(),
            binary_name: asset.binary_name.to_string(),
            binary_sha256: sha256_hex(&binary),
            sha256: sha256_hex(archive),
            asset_source: source.to_string(),
            directory_name,
            updated_at_ms: now_ms(),
        };
        let metadata_bytes = managed_metadata_bytes(&metadata)?;

        let activation_result = (|| -> anyhow::Result<()> {
            if old_metadata.is_some()
                && let Some(bytes) = old_current.as_ref()
            {
                crate::settings::atomic_write(&previous_path, bytes)
                    .context("托管 Multica 历史版本元数据写入失败。")?;
            }
            if let Some(hook) = after_previous_write {
                hook();
            }
            check_managed_install_cancelled(cancel)?;
            crate::settings::atomic_write(&current_path, &metadata_bytes)
                .context("托管 Multica 当前版本指针写入失败。")?;
            check_managed_install_cancelled(cancel)?;
            if let Some(store) = connection_store {
                store.rebind_managed_sidecar_contract_if_present(
                    managed_executable_for_metadata(root, &metadata),
                )?;
            }
            Ok(())
        })();
        if let Err(error) = activation_result {
            if let Err(restore_error) = restore_managed_activation_pointers(
                &current_path,
                old_current.as_deref(),
                &previous_path,
                old_previous.as_deref(),
            ) {
                // The new directory may still be referenced when pointer
                // restoration itself fails, so retain it for recovery.
                activated_dir_path = None;
                return Err(restore_error).context("托管 Multica 激活指针恢复失败。");
            }
            return Err(error);
        }
        let mut status = managed_status_from_metadata(root, Some(asset), Some(metadata));
        status.previous_version = managed_metadata(&previous_path)
            .ok()
            .flatten()
            .filter(|metadata| managed_metadata_is_verified(root, metadata))
            .map(|metadata| metadata.version);
        status.install_phase = Some("complete".to_string());
        status.downloaded_bytes = archive_size;
        status.total_bytes = Some(archive_size);
        status.progress_percent = Some(100);
        Ok(status)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
        if let Some(activated_dir) = activated_dir_path {
            let _ = fs::remove_dir_all(activated_dir);
        }
    }
    result
}

fn verify_managed_binary_version(path: &Path, expected_version: &str) -> anyhow::Result<()> {
    let mut command = Command::new(path);
    command
        .args(managed_version_probe_args())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_sidecar_environment(&mut command);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .context("托管 Multica CLI 无法启动版本探测。")?;
    let stdout_reader = child.stdout.take().map(|pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe
                .take((MAX_TEXT_LENGTH + 1) as u64)
                .read_to_end(&mut bytes);
            bytes
        })
    });
    let stderr_reader = child.stderr.take().map(|pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe
                .take((MAX_TEXT_LENGTH + 1) as u64)
                .read_to_end(&mut bytes);
            bytes
        })
    });
    let deadline = Instant::now() + Duration::from_secs(8);
    let status = loop {
        if check_managed_install_cancelled(None).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            bail!("managed_runtime_install_cancelled");
        }
        match child.try_wait()? {
            Some(status) => break status,
            None if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            None => {
                let _ = child.kill();
                let _ = child.wait();
                bail!("managed_runtime_version_probe_timeout");
            }
        }
    };
    if !status.success() {
        bail!("managed_runtime_version_probe_failed");
    }
    let stdout = stdout_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    // Stderr is intentionally not part of the version assertion.  The CLI's
    // machine-readable contract is stdout JSON; accepting mixed stderr text
    // would allow an unrelated diagnostic to masquerade as the version.
    let _ = stderr_reader.and_then(|reader| reader.join().ok());
    validate_managed_version_probe_output(&stdout, expected_version)
}

const MANAGED_AUTH_STATUS_TIMEOUT: Duration = Duration::from_secs(20);
const MANAGED_AUTH_MUTATION_TIMEOUT: Duration = Duration::from_secs(5 * 60 + 15);
const MANAGED_RUNTIME_APP_URL: &str = "https://multica.ai";

struct ManagedCliOutput {
    success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Validate the exact persisted URL immediately before executing a managed
/// child. The editor is intentionally free-form, so save preserves the user's
/// bytes and execution rejects unsafe or unusable values without rewriting
/// the stored connection.
fn validate_managed_cli_server_url(server_url: &str) -> anyhow::Result<()> {
    if server_url.is_empty() {
        bail!("managed_runtime_server_url_unconfigured");
    }
    if server_url != server_url.trim() {
        bail!("managed_runtime_server_url_invalid");
    }
    let url = Url::parse(server_url).map_err(|_| anyhow!("managed_runtime_server_url_invalid"))?;
    if url.username() != "" || url.password().is_some() {
        bail!("managed_runtime_server_url_credentials_forbidden");
    }
    if url.query().is_some() || url.fragment().is_some() {
        bail!("managed_runtime_server_url_query_forbidden");
    }
    if url.host_str().is_none_or(str::is_empty) {
        bail!("managed_runtime_server_url_host_missing");
    }
    if url.port_or_known_default().is_some_and(is_forbidden_port) {
        bail!("managed_runtime_server_url_reserved_port");
    }
    match url.scheme().to_ascii_lowercase().as_str() {
        "https" => {}
        "http" if is_loopback_host(&url) || is_private_lan_host(&url) => {}
        "http" => bail!("managed_runtime_server_url_insecure_host"),
        _ => bail!("managed_runtime_server_url_scheme_forbidden"),
    }
    Ok(())
}

fn managed_cli_context_from_server_url(
    server_url: &str,
    command: &[&str],
) -> anyhow::Result<(Vec<String>, String)> {
    validate_managed_cli_server_url(server_url)?;
    let mut args = vec!["--profile".to_string(), MANAGED_RUNTIME_PROFILE.to_string()];
    args.extend(command.iter().map(|argument| (*argument).to_string()));
    Ok((args, server_url.to_string()))
}

fn managed_cli_context(command: &[&str]) -> anyhow::Result<(Vec<String>, String)> {
    let server_url = MulticaStore::default()
        .load_connections()?
        .into_iter()
        .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
        .map(|connection| connection.server_url)
        .unwrap_or_else(|| MANAGED_RUNTIME_SERVER_URL.to_string());
    managed_cli_context_from_server_url(&server_url, command)
}

/// Execute one of the fixed managed CLI commands. All streams are bounded,
/// and no output is returned to the renderer. On Windows the child is hidden;
/// on Unix it gets its own process group so a timeout cannot leave a helper
/// process attached to the manager. The environment is the same tiny
/// allowlist used by sidecars, plus the pinned public Multica app URL needed
/// for the browser login flow.
fn run_managed_cli(command: &[&str], timeout: Duration) -> anyhow::Result<ManagedCliOutput> {
    let executable = managed_runtime_executable_path()?;
    let profile_dir = managed_profile_directory();
    crate::settings::create_private_dir_all(&profile_dir)
        .context("托管 Multica profile 目录无法创建。")?;
    let (args, server_url) = managed_cli_context(command)?;
    let mut process = Command::new(&executable);
    process
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&profile_dir);
    apply_sidecar_environment(&mut process);
    process
        .env("MULTICA_SERVER_URL", server_url)
        .env("MULTICA_APP_URL", MANAGED_RUNTIME_APP_URL);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        process.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        process.process_group(0);
    }
    let mut child = process.spawn().context("托管 Multica CLI 命令无法启动。")?;
    // Drain both pipes concurrently so a noisy/misbehaving CLI cannot block
    // on a full OS pipe before it reaches its exit status.
    let stdout_reader = child.stdout.take().map(|pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe
                .take((MAX_RESPONSE_BYTES + 1) as u64)
                .read_to_end(&mut bytes);
            bytes
        })
    });
    let stderr_reader = child.stderr.take().map(|pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe
                .take((MAX_RESPONSE_BYTES + 1) as u64)
                .read_to_end(&mut bytes);
            bytes
        })
    });
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = stdout_reader
                    .and_then(|reader| reader.join().ok())
                    .unwrap_or_default();
                let stderr = stderr_reader
                    .and_then(|reader| reader.join().ok())
                    .unwrap_or_default();
                if stdout.len() > MAX_RESPONSE_BYTES || stderr.len() > MAX_RESPONSE_BYTES {
                    bail!("managed_runtime_auth_output_too_large");
                }
                return Ok(ManagedCliOutput {
                    success: status.success(),
                    stdout,
                    stderr,
                });
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                bail!("managed_runtime_auth_timeout");
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                bail!("managed_runtime_auth_process_failed");
            }
        }
    }
}

fn classify_managed_auth_output(output: &ManagedCliOutput) -> MulticaManagedAuthStatus {
    // Multica's auth command writes human-readable status to stderr. Keep
    // parsing deliberately broad enough for the current CLI while reducing
    // every result to fixed status/diagnostic codes and never echoing text.
    let mut text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&output.stderr).to_ascii_lowercase());
    let authenticated = (text.contains("authenticated as") || text.contains("user:"))
        && !text.contains("not authenticated")
        && !text.contains("invalid or expired")
        && !text.contains("unauthorized");
    let needs_login = text.contains("not authenticated")
        || text.contains("invalid or expired")
        || text.contains("unauthorized")
        || text.contains("authentication required")
        || text.contains("token is invalid")
        || text.contains("token removed");
    let status = if authenticated {
        "authenticated"
    } else if needs_login {
        "needs_login"
    } else if text.contains("no server configured")
        || text.contains("run 'multica setup'")
        || text.contains("managed_runtime_not_installed")
    {
        "unconfigured"
    } else {
        "unknown"
    };
    let diagnostic = if status == "authenticated" || status == "needs_login" {
        None
    } else if status == "unconfigured" {
        Some("managed_auth_unconfigured".to_string())
    } else if output.success {
        Some("managed_auth_status_unrecognized".to_string())
    } else {
        Some("managed_auth_command_failed".to_string())
    };
    MulticaManagedAuthStatus {
        status: status.to_string(),
        checked_at_ms: Some(now_ms()),
        diagnostic,
    }
}

fn managed_auth_unconfigured(diagnostic_code: &'static str) -> MulticaManagedAuthStatus {
    MulticaManagedAuthStatus {
        status: "unconfigured".to_string(),
        checked_at_ms: Some(now_ms()),
        diagnostic: Some(diagnostic_code.to_string()),
    }
}

/// Query only the fixed managed profile. Missing or invalid installation is
/// represented as an unconfigured status so the Manager can remain usable.
pub fn managed_auth_status() -> anyhow::Result<MulticaManagedAuthStatus> {
    if managed_runtime_executable_path().is_err() {
        return Ok(managed_auth_unconfigured("managed_runtime_not_installed"));
    }
    let output = run_managed_cli(&["auth", "status"], MANAGED_AUTH_STATUS_TIMEOUT)?;
    Ok(classify_managed_auth_output(&output))
}

/// Start the official browser login flow for the fixed `ccp-managed` profile.
/// No token input is accepted by this API. After the CLI exits, verify the
/// resulting profile through the read-only auth status command.
pub fn login_managed() -> anyhow::Result<MulticaManagedAuthStatus> {
    if managed_runtime_executable_path().is_err() {
        return Ok(managed_auth_unconfigured("managed_runtime_not_installed"));
    }
    let output = run_managed_cli(&["login"], MANAGED_AUTH_MUTATION_TIMEOUT)?;
    if !output.success {
        return Ok(classify_managed_auth_output(&output));
    }
    managed_auth_status()
}

/// Remove only the credential stored in the fixed `ccp-managed` profile.
pub fn logout_managed() -> anyhow::Result<MulticaManagedAuthStatus> {
    if managed_runtime_executable_path().is_err() {
        return Ok(managed_auth_unconfigured("managed_runtime_not_installed"));
    }
    let output = run_managed_cli(&["auth", "logout"], MANAGED_AUTH_STATUS_TIMEOUT)?;
    if !output.success {
        return Ok(classify_managed_auth_output(&output));
    }
    managed_auth_status()
}

/// Alias names make the managed-runtime boundary explicit to callers while
/// keeping the short operations convenient for the Tauri command layer.
pub fn login_managed_runtime() -> anyhow::Result<MulticaManagedAuthStatus> {
    login_managed()
}

pub fn logout_managed_runtime() -> anyhow::Result<MulticaManagedAuthStatus> {
    logout_managed()
}

fn managed_version_probe_args() -> [&'static str; 3] {
    ["version", "--output", "json"]
}

fn validate_managed_version_probe_output(
    output: &[u8],
    expected_version: &str,
) -> anyhow::Result<()> {
    if output.is_empty() || output.len() > MAX_TEXT_LENGTH {
        bail!("managed_runtime_version_probe_failed");
    }
    let value = serde_json::from_slice::<Value>(output)
        .map_err(|_| anyhow!("managed_runtime_version_probe_failed"))?;
    let version = value
        .as_object()
        .and_then(|object| object.get("version"))
        .and_then(Value::as_str)
        .filter(|version| !version.is_empty())
        .ok_or_else(|| anyhow!("managed_runtime_version_probe_failed"))?;
    if version != expected_version {
        bail!("managed_runtime_version_mismatch");
    }
    Ok(())
}

fn bundled_managed_archive(asset: MulticaRuntimeAsset) -> anyhow::Result<Option<Vec<u8>>> {
    for candidate in managed_resource_candidates(asset) {
        if !candidate.exists() {
            continue;
        }
        let bytes = read_bounded_file(&candidate, MANAGED_RUNTIME_MAX_ARCHIVE_BYTES)?;
        if sha256_hex(&bytes).eq_ignore_ascii_case(asset.expected_sha256) {
            return Ok(Some(bytes));
        }
        // A present but damaged bundled resource is not trusted.  Continue
        // to the official Release fallback rather than activating it.
    }
    Ok(None)
}

fn install_managed_runtime_locked(
    root: &Path,
    connection_store: &MulticaStore,
) -> anyhow::Result<MulticaRuntimeInstallStatus> {
    let Some(asset) = managed_runtime_asset() else {
        return Ok(managed_install_status_error("unsupported_platform", None));
    };
    set_managed_install_progress(Some(asset), "preparing", 0, None, Some(0));
    let current = managed_runtime_status_at(root)?;
    let current_matches_build = managed_metadata(&managed_current_path(root))
        .ok()
        .flatten()
        .is_some_and(|metadata| managed_metadata_matches_asset(&metadata, asset));
    if current.install_state == "ready" && current_matches_build {
        return Ok(current);
    }

    set_managed_install_progress(Some(asset), "checking_bundle", 0, None, Some(2));
    if let Some(archive) = bundled_managed_archive(asset)? {
        return install_managed_archive_locked_with_context(
            root,
            asset,
            &archive,
            "bundled",
            true,
            None,
            Some(connection_store),
            None,
        );
    }

    let archive =
        run_async_without_nested_runtime(download_managed_archive_with_cancel(asset, None))?;
    install_managed_archive_locked_with_context(
        root,
        asset,
        &archive,
        "github_release",
        true,
        None,
        Some(connection_store),
        None,
    )
}

fn managed_runtime_status_is_usable(status: &MulticaRuntimeInstallStatus) -> bool {
    status.install_state == "ready" && status.sha256_verified
}

/// Stop an in-process managed child before changing the binary pointer. A
/// foreign or unverifiable PID blocks the transition; it is never killed or
/// replaced. The caller must already hold the cross-process install lock so a
/// second manager cannot race the stop with another pointer mutation.
fn prepare_managed_runtime_binary_transition() -> anyhow::Result<bool> {
    let state = {
        let mut processes = sidecars()
            .lock()
            .map_err(|_| anyhow!("managed_runtime_sidecar_state_unavailable"))?;
        let Some(process) = processes.get_mut(MANAGED_RUNTIME_CONNECTION_ID) else {
            invalidate_managed_supervisor(true, Some(managed_stopped_status()));
            return Ok(false);
        };
        let state = sidecar_process_state(process);
        if matches!(state, SidecarProcessState::Exited(_)) {
            if let Some(mut exited) = processes.remove(MANAGED_RUNTIME_CONNECTION_ID) {
                let _ = exited.child.wait();
            }
        }
        state
    };

    match state {
        SidecarProcessState::RunningOwned => {
            let stopped = stop_managed_runtime()?;
            if stopped.exited_at_ms.is_none() && stopped.status != "stopped" {
                bail!("managed_runtime_transition_stop_failed");
            }
            Ok(true)
        }
        SidecarProcessState::Exited(_) => {
            invalidate_managed_supervisor(true, Some(managed_stopped_status()));
            Ok(false)
        }
        SidecarProcessState::RunningForeign => {
            invalidate_managed_supervisor(true, None);
            bail!("managed_runtime_transition_pid_mismatch")
        }
        SidecarProcessState::RunningUnverified | SidecarProcessState::StatusUnavailable => {
            invalidate_managed_supervisor(true, None);
            bail!("managed_runtime_transition_pid_unverified")
        }
    }
}

fn restart_managed_runtime_after_binary_transition(
    root: &Path,
    was_running: bool,
    status: MulticaRuntimeInstallStatus,
) -> MulticaRuntimeInstallStatus {
    restart_managed_runtime_after_binary_transition_with(root, was_running, status, || {
        start_managed_runtime()
    })
}

fn restart_managed_runtime_after_binary_transition_with(
    root: &Path,
    was_running: bool,
    status: MulticaRuntimeInstallStatus,
    mut start: impl FnMut() -> anyhow::Result<MulticaDaemonStatus>,
) -> MulticaRuntimeInstallStatus {
    if !was_running || !managed_runtime_status_is_usable(&status) {
        return status;
    }
    match start() {
        Ok(daemon) if !managed_status_requires_rollback(&daemon) => status,
        Ok(_) | Err(_) => managed_install_failure_status_at(root, "managed_runtime_restart_failed"),
    }
}

/// Install the pinned Multica CLI, preferring a verified application resource
/// and falling back to the fixed official GitHub Release asset. A running
/// managed daemon is stopped only after the install lock is held, then
/// restarted from the resulting verified pointer before this call returns.
pub fn install_managed_runtime() -> anyhow::Result<MulticaRuntimeInstallStatus> {
    let root = managed_runtime_root();
    let connection_store = MulticaStore::default();
    let install_lock = acquire_managed_install_lock(&root)?;
    let was_running = prepare_managed_runtime_binary_transition()?;
    let cancel_guard = ManagedInstallGuard::begin();
    let result = match install_managed_runtime_locked(&root, &connection_store) {
        Ok(status) => clear_managed_install_failure(&root).map(|()| status),
        Err(error) if error.to_string() == "managed_runtime_install_cancelled" => Ok(
            managed_install_failure_status_at(&root, "managed_runtime_install_cancelled"),
        ),
        Err(error) => Ok(managed_install_failure_status_at(
            &root,
            &stable_managed_install_error_code(&error),
        )),
    };
    drop(cancel_guard);
    drop(install_lock);

    match result {
        Ok(status) => Ok(restart_managed_runtime_after_binary_transition(
            &root,
            was_running,
            status,
        )),
        Err(error) => {
            if was_running && managed_runtime_executable_path().is_ok() {
                let _ = start_managed_runtime();
            }
            Err(error)
        }
    }
}

/// Ensure the managed runtime exists and initialize its independent default
/// connection when the executable is available.
pub fn ensure_managed_runtime() -> anyhow::Result<MulticaRuntimeInstallStatus> {
    // Establish the isolated connection record before attempting network or
    // archive work.  A failed first download must still leave the Runtime page
    // editable and must never turn initialization into a supplier/profile
    // migration.  A second call below attaches only the fixed sidecar after a
    // successful install.
    let initial_connection = ensure_managed_connection();
    let mut status = install_managed_runtime()?;
    let connection_result = if status.install_state == "ready" {
        ensure_managed_connection()
    } else {
        initial_connection
    };
    status = status_after_managed_connection_init(status, connection_result);
    Ok(status)
}

fn status_after_managed_connection_init(
    mut status: MulticaRuntimeInstallStatus,
    result: anyhow::Result<MulticaConnectionConfig>,
) -> MulticaRuntimeInstallStatus {
    if result.is_err() {
        // Runtime installation is still usable when the independent
        // connection record cannot be initialized. Keep the ready state
        // visible, but expose a stable code instead of dropping the
        // failure or leaking filesystem/configuration details.
        if status.last_install_error_code.is_none() {
            status.last_install_error_code = Some(MANAGED_CONNECTION_INIT_ERROR_CODE.to_string());
            status.diagnostic = Some(MANAGED_CONNECTION_INIT_ERROR_CODE.to_string());
        }
        status.updated_at_ms = Some(now_ms());
    }
    status
}

/// Async spelling for callers already running a Tokio/Tauri task.  The work is
/// placed on a blocking thread because installation performs file replacement
/// and may invoke a bounded `--version` probe.
pub async fn ensure_managed_runtime_async() -> anyhow::Result<MulticaRuntimeInstallStatus> {
    tokio::task::spawn_blocking(ensure_managed_runtime)
        .await
        .map_err(|_| anyhow!("managed_runtime_worker_failed"))?
}

/// Cancel is intentionally idempotent.  Downloads run in a bounded worker and
/// do not expose a mutable process handle; a subsequent ensure call simply
/// retries after the current lock is released.
pub fn cancel_managed_runtime_install() -> anyhow::Result<MulticaRuntimeInstallStatus> {
    let active = managed_install_cancel_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .cloned();
    let mut status = managed_runtime_status()?;
    status.diagnostic = if let Some(token) = active {
        token.store(true, Ordering::Release);
        Some("managed_runtime_install_cancel_requested".to_string())
    } else {
        Some("managed_runtime_no_install_in_progress".to_string())
    };
    Ok(status)
}

pub fn managed_runtime_executable_path() -> anyhow::Result<PathBuf> {
    let root = managed_runtime_root();
    let Some(metadata) = managed_metadata(&managed_current_path(&root))? else {
        bail!("managed_runtime_not_installed");
    };
    let Some(asset) = managed_runtime_asset() else {
        bail!("unsupported_platform");
    };
    if managed_status_from_metadata(&root, Some(asset), Some(metadata.clone())).install_state
        != "ready"
    {
        bail!("managed_runtime_not_ready");
    }
    Ok(managed_executable_for_metadata(&root, &metadata))
}

fn managed_profile_directory() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| {
            dirs.home_dir()
                .join(".multica")
                .join("profiles")
                .join(MANAGED_RUNTIME_PROFILE)
        })
        .unwrap_or_else(|| {
            PathBuf::from(".multica")
                .join("profiles")
                .join(MANAGED_RUNTIME_PROFILE)
        })
}

/// Stable profile path used by the managed daemon.  The returned path is for
/// local process setup only; it is never serialized in a status DTO.
pub fn managed_profile_path() -> PathBuf {
    managed_profile_directory()
}

#[derive(Deserialize)]
struct ManagedWorkspaceProfileFile {
    server_url: Option<String>,
    app_url: Option<String>,
    workspace_id: Option<String>,
    token: Option<String>,
}

struct ManagedWorkspaceCredentials {
    server_origin: Url,
    app_origin: Url,
    workspace_id: String,
    authorization: HeaderValue,
}

fn validate_managed_workspace_origin(raw: Option<String>) -> anyhow::Result<Url> {
    let raw = raw.ok_or_else(|| anyhow!("managed_workspace_profile_invalid"))?;
    if raw.is_empty() || raw != raw.trim() {
        bail!("managed_workspace_profile_invalid");
    }
    let url = Url::parse(&raw).map_err(|_| anyhow!("managed_workspace_profile_invalid"))?;
    if url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
        || url.host_str().is_none_or(str::is_empty)
        || url.port_or_known_default().is_some_and(is_forbidden_port)
    {
        bail!("managed_workspace_profile_invalid");
    }
    match url.scheme().to_ascii_lowercase().as_str() {
        "https" => {}
        "http" if is_loopback_host(&url) || is_private_lan_host(&url) => {}
        _ => bail!("managed_workspace_profile_invalid"),
    }
    Ok(url)
}

fn validate_managed_workspace_id(raw: &str) -> anyhow::Result<String> {
    if raw.is_empty()
        || raw != raw.trim()
        || raw.len() > 160
        || !raw
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        bail!("managed_workspace_id_invalid");
    }
    Ok(raw.to_string())
}

fn load_managed_profile_credentials_from(
    profile_directory: &Path,
) -> anyhow::Result<ManagedWorkspaceCredentials> {
    let path = profile_directory.join(MANAGED_PROFILE_CONFIG_FILE);
    let file = fs::File::open(&path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => anyhow!("managed_workspace_profile_missing"),
        _ => anyhow!("managed_workspace_profile_read_failed"),
    })?;
    let metadata = file
        .metadata()
        .map_err(|_| anyhow!("managed_workspace_profile_read_failed"))?;
    if !metadata.is_file() || metadata.len() > MANAGED_PROFILE_MAX_CONFIG_BYTES {
        bail!("managed_workspace_profile_invalid");
    }
    let mut bytes = Vec::with_capacity(
        usize::try_from(metadata.len())
            .unwrap_or(MANAGED_PROFILE_MAX_CONFIG_BYTES as usize)
            .min(MANAGED_PROFILE_MAX_CONFIG_BYTES as usize),
    );
    file.take(MANAGED_PROFILE_MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow!("managed_workspace_profile_read_failed"))?;
    if bytes.len() > MANAGED_PROFILE_MAX_CONFIG_BYTES as usize {
        bail!("managed_workspace_profile_invalid");
    }
    let profile: ManagedWorkspaceProfileFile =
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("managed_workspace_profile_invalid"))?;
    let server_origin = validate_managed_workspace_origin(profile.server_url)?;
    let app_origin = validate_managed_workspace_origin(profile.app_url)?;
    let workspace_id = validate_managed_workspace_id(
        profile
            .workspace_id
            .as_deref()
            .ok_or_else(|| anyhow!("managed_workspace_profile_invalid"))?,
    )?;
    let token = profile
        .token
        .filter(|token| !token.is_empty() && token.len() <= MANAGED_WORKSPACE_MAX_TOKEN_LENGTH)
        .ok_or_else(|| anyhow!("managed_workspace_profile_invalid"))?;
    let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| anyhow!("managed_workspace_profile_invalid"))?;
    authorization.set_sensitive(true);
    Ok(ManagedWorkspaceCredentials {
        server_origin,
        app_origin,
        workspace_id,
        authorization,
    })
}

/// Load the one fixed CCP-managed Multica profile into a read-only client.
/// The profile path is not caller-selectable and the credential never leaves
/// this module.
pub fn managed_workspace_client() -> anyhow::Result<MulticaManagedWorkspaceClient> {
    managed_workspace_client_from(&managed_profile_directory())
}

fn managed_workspace_client_from(
    profile_directory: &Path,
) -> anyhow::Result<MulticaManagedWorkspaceClient> {
    let credentials = load_managed_profile_credentials_from(profile_directory)?;
    Ok(MulticaManagedWorkspaceClient {
        client: build_client().map_err(|_| anyhow!("managed_workspace_client_unavailable"))?,
        server_origin: credentials.server_origin,
        _app_origin: credentials.app_origin,
        workspace_id: credentials.workspace_id,
        authorization: credentials.authorization,
    })
}

fn managed_sidecar_for_executable(path: PathBuf) -> anyhow::Result<MulticaSidecarConfig> {
    let profile_dir = managed_profile_directory();
    crate::settings::create_private_dir_all(&profile_dir)?;
    Ok(MulticaSidecarConfig {
        executable: path.to_string_lossy().to_string(),
        working_dir: Some(profile_dir.to_string_lossy().to_string()),
        args: managed_runtime_args(),
        auto_start: true,
    })
}

fn managed_runtime_args() -> Vec<String> {
    vec![
        "--profile".to_string(),
        MANAGED_RUNTIME_PROFILE.to_string(),
        "daemon".to_string(),
        "start".to_string(),
        "--foreground".to_string(),
        "--no-auto-update".to_string(),
        "--no-auto-reload".to_string(),
    ]
}

/// Verify the immutable part of the managed daemon contract immediately
/// before every managed launch.  The editable connection name and server URL
/// are deliberately outside this check; neither value is a process argument.
fn validate_managed_runtime_sidecar(config: &MulticaConnectionConfig) -> anyhow::Result<()> {
    if !is_managed_connection_id(&config.connection_id) {
        bail!(MANAGED_CONNECTION_RESERVED_ERROR);
    }
    validate_managed_cli_server_url(&config.server_url)?;
    let sidecar = sidecar_config_for_start(config)?;
    if !sidecar.auto_start || sidecar.args != managed_runtime_args() {
        bail!("managed_runtime_sidecar_contract_invalid");
    }

    let expected_runtime_executable =
        managed_runtime_executable_path().context("managed_runtime_binary_unavailable")?;
    let expected_executable =
        verified_sidecar_executable_path(&expected_runtime_executable.to_string_lossy())
            .context("managed_runtime_binary_unavailable")?;
    let configured_executable = verified_sidecar_executable_path(&sidecar.executable)?;
    if !same_executable_path(&expected_executable, &configured_executable) {
        bail!("managed_runtime_sidecar_contract_invalid");
    }

    let expected_working_dir = managed_profile_directory();
    crate::settings::create_private_dir_all(&expected_working_dir)?;
    let expected_working_dir =
        fs::canonicalize(expected_working_dir).context("managed_runtime_profile_unavailable")?;
    let configured_working_dir = sidecar
        .working_dir
        .as_deref()
        .ok_or_else(|| anyhow!("managed_runtime_sidecar_contract_invalid"))
        .and_then(verified_sidecar_working_dir)?;
    if !same_executable_path(&expected_working_dir, &configured_working_dir) {
        bail!("managed_runtime_sidecar_contract_invalid");
    }
    Ok(())
}

fn managed_connection_is_supervision_eligible(connection: &MulticaConnectionConfig) -> bool {
    // `supervise` is a fixed property of the dedicated managed lifecycle.  It
    // is intentionally not inherited by, or exposed through, user-created
    // sidecars.  Disabling the managed connection is the user's stop switch.
    is_managed_connection_id(&connection.connection_id)
        && connection.enabled
        && connection
            .sidecar
            .as_ref()
            .is_some_and(|sidecar| sidecar.auto_start)
}

fn managed_supervision_eligible_now() -> bool {
    find_connection(MANAGED_RUNTIME_CONNECTION_ID)
        .map(|connection| managed_connection_is_supervision_eligible(&connection))
        .unwrap_or(false)
}

fn next_managed_supervisor_generation(state: &mut ManagedSupervisorState) -> u64 {
    state.generation = state.generation.wrapping_add(1);
    // Generation zero is valid in storage, but keeping a non-zero active
    // generation makes stale-worker diagnostics easier to reason about.
    if state.generation == 0 {
        state.generation = 1;
    }
    state.generation
}

/// Invalidate every existing worker.  The worker only mutates state when its
/// generation still matches, so a delayed pre-crash worker cannot overwrite a
/// later manual stop/start result.
fn invalidate_managed_supervisor(
    stop_requested: bool,
    terminal_status: Option<MulticaDaemonStatus>,
) -> u64 {
    let mut state = managed_supervisor()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let generation = next_managed_supervisor_generation(&mut state);
    state.worker_running = false;
    state.stop_requested = stop_requested;
    state.restart_attempts = 0;
    state.restart_exhausted = false;
    state.rollback_attempted = false;
    if let Some(status) = terminal_status {
        state.last_terminal_status = Some(status);
    } else if !stop_requested {
        state.last_terminal_status = None;
    }
    generation
}

fn begin_managed_supervision() -> u64 {
    invalidate_managed_supervisor(false, None)
}

fn managed_supervisor_should_continue(generation: u64) -> bool {
    managed_supervisor()
        .lock()
        .map(|state| {
            managed_supervisor_should_continue_state(&state, generation, shutdown_requested())
        })
        .unwrap_or(false)
}

/// Pure gate used by the worker and by state-machine tests.  Keeping the
/// shutdown decision as an argument makes generation/stop/restart semantics
/// testable without mutating the process-global shutdown flag.
fn managed_supervisor_should_continue_state(
    state: &ManagedSupervisorState,
    generation: u64,
    shutdown: bool,
) -> bool {
    !shutdown && state.generation == generation && !state.stop_requested && !state.restart_exhausted
}

fn set_managed_supervisor_terminal(generation: u64, status: MulticaDaemonStatus) -> bool {
    let Ok(mut state) = managed_supervisor().lock() else {
        return false;
    };
    if state.generation != generation || state.stop_requested || shutdown_requested() {
        return false;
    }
    state.last_terminal_status = Some(status);
    true
}

fn managed_supervisor_restart_delay(
    state: &mut ManagedSupervisorState,
    generation: u64,
    status: &MulticaDaemonStatus,
) -> Option<Duration> {
    if state.generation != generation || state.stop_requested || state.restart_exhausted {
        return None;
    }
    if state.restart_attempts >= MANAGED_SUPERVISOR_MAX_RESTARTS {
        state.restart_exhausted = true;
        state.last_terminal_status = Some(MulticaDaemonStatus {
            status: "restart_exhausted".to_string(),
            checked_at_ms: Some(now_ms()),
            diagnostic: Some(diagnostic("managed_runtime_restart_exhausted")),
            ..status.clone()
        });
        return None;
    }

    state.restart_attempts = state.restart_attempts.saturating_add(1);
    let mut crashed = status.clone();
    crashed.status = "crashed".to_string();
    crashed.checked_at_ms = Some(now_ms());
    crashed.diagnostic = Some(diagnostic("managed_runtime_crashed"));
    state.last_terminal_status = Some(crashed);
    MANAGED_SUPERVISOR_BACKOFFS
        .get(state.restart_attempts.saturating_sub(1) as usize)
        .copied()
}

fn managed_supervisor_reserve_rollback(
    state: &mut ManagedSupervisorState,
    generation: u64,
    shutdown: bool,
) -> bool {
    if state.generation != generation
        || state.stop_requested
        || state.rollback_attempted
        || shutdown
    {
        return false;
    }
    state.rollback_attempted = true;
    true
}

fn reserve_managed_supervisor_rollback(generation: u64) -> bool {
    let Ok(mut state) = managed_supervisor().lock() else {
        return false;
    };
    managed_supervisor_reserve_rollback(&mut state, generation, shutdown_requested())
}

fn managed_status_requires_rollback(status: &MulticaDaemonStatus) -> bool {
    matches!(
        status.status.as_str(),
        "stopped" | "degraded" | "unreachable" | "invalid_response" | "checking"
    )
}

fn managed_start_error_requires_rollback(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        matches!(
            cause.to_string().as_str(),
            "managed_runtime_binary_unavailable" | "managed_runtime_spawn_failed"
        )
    })
}

fn managed_supervisor_action(status: &MulticaDaemonStatus) -> ManagedSupervisorAction {
    if status.status == "degraded"
        && matches!(
            status.diagnostic.as_deref(),
            Some("sidecar_pid_mismatch")
                | Some("sidecar_pid_unverified")
                | Some("sidecar_status_failed")
                | Some("sidecar_state_unavailable")
                | Some("sidecar_replaced_during_probe")
                | Some("managed_runtime_health_probe_failed")
        )
    {
        return ManagedSupervisorAction::StopUnsafe;
    }

    match status.status.as_str() {
        "stopped" | "checking" | "degraded" | "unreachable" | "invalid_response" => {
            ManagedSupervisorAction::Recover
        }
        // Authentication is a user/profile state, not evidence that the
        // pinned daemon binary is broken. Keep probing without restarting.
        "unauthorized" | "needs_login" => ManagedSupervisorAction::Observe,
        _ => ManagedSupervisorAction::Observe,
    }
}

fn managed_supervisor_health_probe_due(
    status: &MulticaDaemonStatus,
    now: Instant,
    next_probe_at: Instant,
) -> bool {
    status.pid.is_some()
        && !matches!(
            managed_supervisor_action(status),
            ManagedSupervisorAction::StopUnsafe
        )
        && status.status != "stopped"
        && (status.status == "checking" || now >= next_probe_at)
}

fn managed_runtime_has_verified_previous_at(root: &Path) -> bool {
    let Some(asset) = managed_runtime_asset() else {
        return false;
    };
    managed_metadata(&managed_previous_path(root))
        .ok()
        .flatten()
        .is_some_and(|previous| {
            managed_metadata_matches_target(&previous, asset)
                && managed_metadata_is_verified(root, &previous)
        })
}

fn managed_runtime_has_verified_previous() -> bool {
    managed_runtime_has_verified_previous_at(&managed_runtime_root())
}

fn perform_managed_runtime_automatic_rollback_at(
    root: &Path,
    store: &MulticaStore,
    mut stop: impl FnMut() -> anyhow::Result<MulticaDaemonStatus>,
    mut start: impl FnMut() -> anyhow::Result<MulticaDaemonStatus>,
    on_rollback: impl FnOnce(),
) -> anyhow::Result<MulticaDaemonStatus> {
    stop()?;
    rollback_managed_runtime_at_with_store(root, store)
        .context("managed_runtime_auto_rollback_failed")?;
    on_rollback();

    let recovered = start()?;
    if managed_status_requires_rollback(&recovered) {
        let _ = stop();
        bail!("managed_runtime_auto_rollback_health_failed");
    }
    Ok(recovered)
}

fn try_managed_runtime_automatic_rollback(
    generation: u64,
) -> anyhow::Result<Option<MulticaDaemonStatus>> {
    if !managed_runtime_has_verified_previous() || !reserve_managed_supervisor_rollback(generation)
    {
        return Ok(None);
    }
    let root = managed_runtime_root();
    let store = MulticaStore::default();
    let recovered = perform_managed_runtime_automatic_rollback_at(
        &root,
        &store,
        || {
            // Only the process tracked under the fixed managed connection can
            // be stopped. Ownership checks still reject a foreign PID.
            stop_sidecar_with_scope(MANAGED_RUNTIME_CONNECTION_ID, true)
                .context("managed_runtime_auto_rollback_stop_failed")
        },
        || start_managed_runtime_sidecar().context("managed_runtime_auto_rollback_restart_failed"),
        || {
            log_sidecar_lifecycle(
                "automatic_rollback",
                MANAGED_RUNTIME_CONNECTION_ID,
                None,
                None,
                None,
                None,
                Some("recovering"),
                Some("managed_runtime_automatic_rollback"),
            );
        },
    )?;
    Ok(Some(recovered))
}

fn record_managed_supervisor_crash(
    generation: u64,
    status: &MulticaDaemonStatus,
) -> Option<Duration> {
    let Ok(mut state) = managed_supervisor().lock() else {
        return None;
    };
    managed_supervisor_restart_delay(&mut state, generation, status)
}

fn recover_managed_supervisor(generation: u64, status: &MulticaDaemonStatus) -> bool {
    match try_managed_runtime_automatic_rollback(generation) {
        Ok(Some(_)) => return true,
        Ok(None) => {}
        Err(_) => {
            // A failed fallback consumes the one rollback slot. Continue with
            // the bounded restart budget against the compensated active
            // pointer left by the rollback transaction.
        }
    }

    let Some(delay) = record_managed_supervisor_crash(generation, status) else {
        return false;
    };
    log_sidecar_lifecycle(
        "supervisor_restart_scheduled",
        MANAGED_RUNTIME_CONNECTION_ID,
        status.pid,
        status.started_at_ms,
        status.exited_at_ms,
        status.exit_code,
        Some("crashed"),
        Some("managed_runtime_crashed"),
    );

    if status.status != "stopped"
        && stop_sidecar_with_scope(MANAGED_RUNTIME_CONNECTION_ID, true).is_err()
    {
        let mut stop_failed = status.clone();
        stop_failed.status = "degraded".to_string();
        stop_failed.checked_at_ms = Some(now_ms());
        stop_failed.diagnostic = Some(diagnostic("managed_runtime_recovery_stop_failed"));
        let _ = set_managed_supervisor_terminal(generation, stop_failed);
        return false;
    }
    if !wait_for_managed_supervisor(generation, delay)
        || !managed_supervisor_should_continue(generation)
    {
        return false;
    }

    // A failed launch consumes the attempt reserved above. The next loop
    // either schedules the next bounded retry or records restart_exhausted.
    let _ = start_managed_runtime_sidecar();
    true
}

fn wait_for_managed_supervisor(generation: u64, duration: Duration) -> bool {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        if !managed_supervisor_should_continue(generation) || !managed_supervision_eligible_now() {
            return false;
        }
        std::thread::sleep(
            Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    managed_supervisor_should_continue(generation) && managed_supervision_eligible_now()
}

fn managed_supervisor_status_overlay(status: MulticaDaemonStatus) -> MulticaDaemonStatus {
    // A live process status is authoritative.  A stale terminal value must
    // never mask a manually started/recovered daemon.
    if !matches!(status.status.as_str(), "stopped" | "unconfigured") {
        return status;
    }
    let Ok(state) = managed_supervisor().lock() else {
        return status;
    };
    state
        .last_terminal_status
        .clone()
        .filter(|terminal| state.restart_exhausted || terminal.status == "crashed")
        .unwrap_or(status)
}

fn start_managed_supervisor_worker(generation: u64) -> anyhow::Result<()> {
    {
        let mut state = managed_supervisor()
            .lock()
            .map_err(|_| anyhow!("managed_runtime_supervisor_state_unavailable"))?;
        if state.generation != generation || state.stop_requested || shutdown_requested() {
            return Ok(());
        }
        if state.worker_running {
            return Ok(());
        }
        state.worker_running = true;
    }

    let spawn_result = std::thread::Builder::new()
        .name("ccp-managed-multica-supervisor".to_string())
        .spawn(move || run_managed_supervisor_worker(generation));
    if spawn_result.is_err() {
        let mut state = managed_supervisor()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.generation == generation {
            state.worker_running = false;
            state.last_terminal_status = Some(MulticaDaemonStatus {
                status: "degraded".to_string(),
                checked_at_ms: Some(now_ms()),
                diagnostic: Some(diagnostic("managed_runtime_supervisor_start_failed")),
                ..Default::default()
            });
        }
        bail!("managed_runtime_supervisor_start_failed");
    }
    Ok(())
}

fn run_managed_supervisor_worker(generation: u64) {
    let mut next_health_probe_at = Instant::now();
    loop {
        if !managed_supervisor_should_continue(generation) || !managed_supervision_eligible_now() {
            break;
        }

        let mut status = daemon_status_raw(MANAGED_RUNTIME_CONNECTION_ID, true);
        let now = Instant::now();
        if managed_supervisor_health_probe_due(&status, now, next_health_probe_at) {
            status = match find_connection(MANAGED_RUNTIME_CONNECTION_ID).and_then(|config| {
                probe_daemon_for_connection_blocking(
                    MANAGED_RUNTIME_CONNECTION_ID.to_string(),
                    config,
                    false,
                )
            }) {
                Ok(status) => status,
                Err(_) => MulticaDaemonStatus {
                    status: "degraded".to_string(),
                    pid: status.pid,
                    started_at_ms: status.started_at_ms,
                    checked_at_ms: Some(now_ms()),
                    diagnostic: Some(diagnostic("managed_runtime_health_probe_failed")),
                    ..Default::default()
                },
            };
            next_health_probe_at = Instant::now() + MANAGED_SUPERVISOR_HEALTH_PROBE_INTERVAL;
            if !managed_supervisor_should_continue(generation)
                || !managed_supervision_eligible_now()
            {
                break;
            }
        }

        match managed_supervisor_action(&status) {
            ManagedSupervisorAction::Recover => {
                if !recover_managed_supervisor(generation, &status) {
                    break;
                }
            }
            ManagedSupervisorAction::StopUnsafe => {
                // Never terminate or replace an unverified/foreign child.
                let _ = set_managed_supervisor_terminal(generation, status);
                break;
            }
            ManagedSupervisorAction::Observe => {
                if !wait_for_managed_supervisor(generation, MANAGED_SUPERVISOR_POLL_INTERVAL) {
                    break;
                }
            }
        }
    }

    if let Ok(mut state) = managed_supervisor().lock() {
        if state.generation == generation {
            state.worker_running = false;
        }
    }
}

/// Create the managed connection once, preserving every user-editable value
/// on subsequent calls.  A missing executable leaves the sidecar unset; the
/// next successful install fills it without touching the server URL/name.
pub fn ensure_managed_connection() -> anyhow::Result<MulticaConnectionConfig> {
    let store = MulticaStore::default();
    let sidecar = managed_runtime_executable_path()
        .ok()
        .map(managed_sidecar_for_executable)
        .transpose()?;
    store.ensure_managed_connection_record(sidecar)
}

/// Update only the managed connection's enabled bit. The operation is kept
/// separate from supplier/profile settings and preserves the saved URL,
/// display name, sidecar path, and all other user-editable fields verbatim.
/// Disabling first stops a child owned by this manager so the existing sidecar
/// configuration guard cannot reject the persisted state transition.
pub fn set_managed_enabled(enabled: bool) -> anyhow::Result<MulticaConnectionConfig> {
    let store = MulticaStore::default();
    if managed_connection().is_err() {
        ensure_managed_connection()?;
    }
    if !enabled {
        if sidecars()
            .lock()
            .map(|processes| processes.contains_key(MANAGED_RUNTIME_CONNECTION_ID))
            .unwrap_or(false)
        {
            let _ = stop_managed_runtime()?;
        } else {
            invalidate_managed_supervisor(true, Some(managed_stopped_status()));
        }
    }
    store.update_managed_enabled(enabled)
}

pub fn managed_connection() -> anyhow::Result<MulticaConnectionConfig> {
    find_connection(MANAGED_RUNTIME_CONNECTION_ID)
}

/// Return the full editable view for the fixed managed connection.  Generic
/// connection list responses remain redacted and cannot be used to edit this
/// record.
pub fn managed_connection_view() -> anyhow::Result<MulticaManagedConnectionView> {
    Ok(MulticaManagedConnectionView::from_connection(
        &managed_connection()?,
    ))
}

/// Persist only the explicitly editable managed fields.  Values are written
/// byte-for-byte, including empty name and URL fields; no URL validation,
/// defaulting, trimming, or proxy substitution is applied here.
pub fn update_managed_connection(
    update: MulticaManagedConnectionUpdate,
) -> anyhow::Result<MulticaManagedConnectionView> {
    let existing = match managed_connection() {
        Ok(connection) => connection,
        Err(_) => ensure_managed_connection()?,
    };

    // An explicit disable must first stop only a child proven to belong to the
    // managed connection.  The existing stop path verifies the image/PID and
    // refuses to act on an unrelated process.
    if !update.enabled
        && existing.enabled
        && sidecars()
            .lock()
            .map(|processes| processes.contains_key(MANAGED_RUNTIME_CONNECTION_ID))
            .unwrap_or(false)
    {
        let _ = stop_managed_runtime()?;
    } else if !update.enabled {
        invalidate_managed_supervisor(true, Some(managed_stopped_status()));
    }

    let saved = MulticaStore::default().update_managed_connection_values(
        update.display_name,
        update.server_url,
        update.enabled,
    )?;
    Ok(MulticaManagedConnectionView::from_connection(&saved))
}

/// Swap the current and previously verified managed version pointers.  The
/// operation is all-or-nothing at the metadata level and never touches CCP
/// settings or supplier files.
pub fn rollback_managed_runtime() -> anyhow::Result<MulticaRuntimeInstallStatus> {
    let root = managed_runtime_root();
    let store = MulticaStore::default();
    let install_lock = acquire_managed_install_lock(&root)?;
    let was_running = prepare_managed_runtime_binary_transition()?;
    let result = rollback_managed_runtime_at_inner_locked(&root, Some(&store));
    drop(install_lock);
    match result {
        Ok(status) => Ok(restart_managed_runtime_after_binary_transition(
            &root,
            was_running,
            status,
        )),
        Err(error) => {
            if was_running && managed_runtime_executable_path().is_ok() {
                let _ = start_managed_runtime();
            }
            Err(error)
        }
    }
}

#[cfg(test)]
fn rollback_managed_runtime_at(root: &Path) -> anyhow::Result<MulticaRuntimeInstallStatus> {
    rollback_managed_runtime_at_inner(root, None)
}

fn rollback_managed_runtime_at_with_store(
    root: &Path,
    connection_store: &MulticaStore,
) -> anyhow::Result<MulticaRuntimeInstallStatus> {
    rollback_managed_runtime_at_inner(root, Some(connection_store))
}

fn rollback_managed_runtime_at_inner(
    root: &Path,
    connection_store: Option<&MulticaStore>,
) -> anyhow::Result<MulticaRuntimeInstallStatus> {
    let _lock = acquire_managed_install_lock(root)?;
    rollback_managed_runtime_at_inner_locked(root, connection_store)
}

fn rollback_managed_runtime_at_inner_locked(
    root: &Path,
    connection_store: Option<&MulticaStore>,
) -> anyhow::Result<MulticaRuntimeInstallStatus> {
    let Some(previous) = managed_metadata(&managed_previous_path(root))? else {
        bail!("managed_runtime_no_previous_version");
    };
    let asset = managed_runtime_asset().ok_or_else(|| anyhow!("unsupported_platform"))?;
    if previous.binary_sha256.trim().is_empty() {
        bail!("managed_runtime_metadata_binary_digest_missing");
    }
    if !managed_metadata_matches_target(&previous, asset)
        || !managed_metadata_is_verified(root, &previous)
    {
        bail!("managed_runtime_previous_invalid");
    }
    let current_path = managed_current_path(root);
    let previous_path = managed_previous_path(root);
    let current_bytes = read_optional_managed_file(&current_path)?;
    let previous_bytes = fs::read(&previous_path)?;
    let swap_result = (|| -> anyhow::Result<()> {
        crate::settings::atomic_write(&current_path, &previous_bytes)?;
        restore_managed_pointer(&previous_path, current_bytes.as_deref())?;
        if let Some(store) = connection_store {
            store.rebind_managed_sidecar_contract_if_present(managed_executable_for_metadata(
                root, &previous,
            ))?;
        }
        Ok(())
    })();
    if let Err(error) = swap_result {
        restore_managed_activation_pointers(
            &current_path,
            current_bytes.as_deref(),
            &previous_path,
            Some(&previous_bytes),
        )
        .context("托管 Multica 回滚指针恢复失败。")?;
        return Err(error);
    }
    managed_runtime_status_at(root)
}

fn generated_connection_id() -> String {
    format!(
        "multica-{}-{}",
        now_ms(),
        NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed)
    )
}

fn truncate(value: impl AsRef<str>) -> String {
    let value = value.as_ref().trim();
    if value.chars().count() <= MAX_TEXT_LENGTH {
        return value.to_string();
    }
    value.chars().take(MAX_TEXT_LENGTH).collect::<String>() + "..."
}

/// Reduce text received from the external Multica service before it enters a
/// snapshot or crosses the Tauri boundary.  The renderer has a second guard,
/// but persisted snapshots must be safe even when they are read by a future
/// client or inspected outside React.
fn sanitize_public_text(value: impl AsRef<str>) -> String {
    // Do not reuse the broad memory redactor here: its `sk-` matcher is
    // intentionally permissive and would turn an innocuous id such as
    // `task-1` into `task-***`.  Snapshot ids/names still pass through this
    // helper, so every token redaction must honour identifier boundaries.
    let compact = value
        .as_ref()
        .replace('\r', " ")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    // Bound work on hostile or accidentally huge upstream error fields before
    // running the marker scanners. The final DTO remains capped at
    // `MAX_TEXT_LENGTH` below.
    let compact = compact
        .chars()
        .take(MAX_PUBLIC_TEXT_INPUT_LENGTH)
        .collect::<String>();
    let value = redact_urls_and_paths(&compact);
    let value = redact_header_values(&value);
    let value = redact_bearer_values(&value);
    let value = redact_prefixed_multica_tokens(&value);
    truncate(redact_named_multica_assignments(&value))
}

fn redact_header_values(input: &str) -> String {
    // Header names are handled separately from ordinary `name=value`
    // fields because Cookie/Authorization values may contain spaces.
    let mut output = input.to_string();
    for marker in ["set-cookie", "cookie", "x-api-key", "authorization"] {
        let mut cursor = 0;
        while cursor < output.len() {
            let lower = output.to_ascii_lowercase();
            let Some(relative) = lower[cursor..].find(marker) else {
                break;
            };
            let start = cursor + relative;
            let marker_end = start + marker.len();
            if !multica_boundary_ok(&output, start, marker_end) {
                cursor = marker_end;
                continue;
            }
            let Some(value_start) = assignment_value_start(&output, marker_end) else {
                cursor = marker_end;
                continue;
            };
            if value_start >= output.len() {
                break;
            }
            let (content_start, quote) = match output[value_start..].chars().next() {
                Some(quote @ ('\'' | '"')) => (value_start + quote.len_utf8(), Some(quote)),
                _ => (value_start, None),
            };
            let end = if marker.ends_with("cookie") && quote.is_none() {
                let scanned = scan_cookie_header_end(&output, content_start);
                if scanned > content_start {
                    scanned
                } else {
                    scan_header_token_end(&output, content_start)
                }
            } else {
                let mut end = content_start;
                for (offset, character) in output[content_start..].char_indices() {
                    if quote.is_some_and(|expected| character == expected)
                        || (quote.is_none() && matches!(character, ',' | ';' | '}' | ']' | '>'))
                    {
                        break;
                    }
                    end = content_start + offset + character.len_utf8();
                }
                end
            };
            if end <= content_start {
                cursor = content_start;
                continue;
            }
            output.replace_range(content_start..end, "[redacted]");
            cursor = content_start + "[redacted]".len();
        }
    }
    output
}

/// Return the start of a named field's value.  Upstream diagnostics commonly
/// render headers both as `Authorization: value` and as JSON-like
/// `"Authorization":"value"`; accepting an optional closing quote around the
/// field name keeps both forms on the same redaction path.
fn assignment_value_start(input: &str, marker_end: usize) -> Option<usize> {
    let mut delimiter_start = skip_ascii_space(input, marker_end);
    if let Some(quote) = input[delimiter_start..]
        .chars()
        .next()
        .filter(|character| matches!(character, '\'' | '"'))
    {
        let after_quote = delimiter_start + quote.len_utf8();
        delimiter_start = skip_ascii_space(input, after_quote);
    }
    let delimiter = input[delimiter_start..].chars().next()?;
    if !matches!(delimiter, ':' | '=') {
        return None;
    }
    Some(skip_ascii_space(
        input,
        delimiter_start + delimiter.len_utf8(),
    ))
}

fn scan_header_token_end(input: &str, start: usize) -> usize {
    let mut end = start;
    for (offset, character) in input[start..].char_indices() {
        if character.is_whitespace() || matches!(character, ',' | ';' | '}' | ']' | '>') {
            break;
        }
        end = start + offset + character.len_utf8();
    }
    end
}

fn scan_cookie_header_end(input: &str, start: usize) -> usize {
    // Cookie headers are a semicolon-separated list of name/value pairs. Walk
    // all complete pairs so a second cookie (for example `token=...`) cannot
    // survive the first replacement; stop before the next human-readable
    // sentence or header.
    let mut cursor = start;
    let mut end = start;
    loop {
        cursor = skip_ascii_space(input, cursor);
        if cursor >= input.len() {
            break;
        }
        let segment_end = input[cursor..]
            .find([';', ',', '}', ']', '>'])
            .map(|offset| cursor + offset)
            .unwrap_or(input.len());
        let segment = input[cursor..segment_end].trim_end();
        let Some(equal) = segment.find('=') else {
            break;
        };
        if equal == 0 || segment[..equal].chars().any(char::is_whitespace) {
            break;
        }
        let value = segment[equal + 1..].trim_start();
        if value.is_empty() {
            break;
        }
        // A cookie value is normally a compact token. If the text after it
        // contains whitespace, stop at that boundary so a following human
        // sentence/header remains available for the other redaction passes.
        let value_end_in_segment = if let Some(quote @ ('\'' | '"')) = value.chars().next() {
            let Some(close) = value[quote.len_utf8()..].find(quote) else {
                break;
            };
            quote.len_utf8() + close
        } else {
            value.find(char::is_whitespace).unwrap_or(value.len())
        };
        let value_leading = segment[equal + 1..].len() - segment[equal + 1..].trim_start().len();
        let value_start = cursor + equal + 1 + value_leading;
        let value_end = value_start + value_end_in_segment;
        if value_end <= value_start {
            break;
        }
        end = value_end;
        if segment_end >= input.len() || input.as_bytes()[segment_end] as char != ';' {
            break;
        }
        let next = skip_ascii_space(input, segment_end + 1);
        if next >= input.len() {
            break;
        }
        // Continue only when the next semicolon-delimited segment looks like
        // another cookie pair. Otherwise it is ordinary diagnostic text.
        let next_segment_end = input[next..]
            .find([';', ',', '}', ']', '>'])
            .map(|offset| next + offset)
            .unwrap_or(input.len());
        let next_segment = input[next..next_segment_end].trim();
        let Some(next_equal) = next_segment.find('=') else {
            break;
        };
        if next_equal == 0 || next_segment[..next_equal].chars().any(char::is_whitespace) {
            break;
        }
        cursor = next;
    }
    end
}

fn skip_ascii_space(input: &str, mut index: usize) -> usize {
    while let Some(character) = input[index..].chars().next() {
        if !character.is_whitespace() {
            break;
        }
        index += character.len_utf8();
    }
    index
}

fn multica_boundary_ok(input: &str, start: usize, end: usize) -> bool {
    let before_ok = input[..start].chars().next_back().is_none_or(|character| {
        !character.is_ascii_alphanumeric() && character != '_' && character != '-'
    });
    let after_ok = input[end..].chars().next().is_none_or(|character| {
        !character.is_ascii_alphanumeric() && character != '_' && character != '-'
    });
    before_ok && after_ok
}

fn redact_bearer_values(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while cursor < input.len() {
        let next = ["bearer", "basic"]
            .iter()
            .filter_map(|marker| {
                lower[cursor..]
                    .match_indices(marker)
                    .map(|(offset, _)| cursor + offset)
                    .find(|start| {
                        let end = *start + marker.len();
                        multica_boundary_ok(input, *start, end)
                            && input[end..].chars().next().is_some_and(char::is_whitespace)
                    })
                    .map(|start| (start, *marker))
            })
            .min_by_key(|(start, marker)| (*start, usize::MAX - marker.len()));
        let Some((start, marker)) = next else {
            output.push_str(&input[cursor..]);
            break;
        };
        let token_start = skip_ascii_space(input, start + marker.len());
        let (token_content_start, quote) = match input[token_start..].chars().next() {
            Some(quote @ ('\'' | '"')) => (token_start + quote.len_utf8(), Some(quote)),
            _ => (token_start, None),
        };
        let mut token_end = token_content_start;
        for (offset, character) in input[token_content_start..].char_indices() {
            if quote.is_some_and(|expected| character == expected)
                || (quote.is_none()
                    && (character.is_whitespace()
                        || matches!(character, ',' | ';' | '"' | '\'' | '}' | ']')))
            {
                break;
            }
            token_end = token_content_start + offset + character.len_utf8();
        }
        if token_end <= token_start {
            output.push_str(&input[cursor..start + marker.len()]);
            cursor = start + marker.len();
            continue;
        }
        output.push_str(&input[cursor..start]);
        output.push_str(if marker == "basic" {
            "Basic [redacted]"
        } else {
            "Bearer [redacted]"
        });
        cursor = token_end;
    }
    output
}

fn redact_prefixed_multica_tokens(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while cursor < input.len() {
        let Some(relative) = lower[cursor..].find("sk-") else {
            output.push_str(&input[cursor..]);
            break;
        };
        let start = cursor + relative;
        if start > 0
            && input[..start].chars().next_back().is_some_and(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
            })
        {
            output.push_str(&input[cursor..start + 3]);
            cursor = start + 3;
            continue;
        }
        let token_start = start + 3;
        let mut token_end = token_start;
        for (offset, character) in input[token_start..].char_indices() {
            if !(character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')) {
                break;
            }
            token_end = token_start + offset + character.len_utf8();
        }
        // A bare three-character `sk-` fragment is not a credential. Keep a
        // short but non-empty payload covered as well; assignment-style keys
        // are handled by the named-value pass below.
        if token_end <= token_start {
            output.push_str(&input[cursor..token_start]);
            cursor = token_start;
            continue;
        }
        output.push_str(&input[cursor..start]);
        output.push_str("[redacted]");
        cursor = token_end;
    }
    output
}

fn redact_named_multica_assignments(input: &str) -> String {
    const MARKERS: &[&str] = &[
        "access_token",
        "access-token",
        "auth_token",
        "auth-token",
        "api_key",
        "api-key",
        "api key",
        "apikey",
        "accesstoken",
        "authtoken",
        "refreshtoken",
        "client_secret",
        "client-secret",
        "clientsecret",
        "session_token",
        "session-token",
        "sessiontoken",
        "private_key",
        "private-key",
        "privatekey",
        "credential",
        "credentials",
        "password",
        "secret",
        "token",
        "key",
    ];
    let lower = input.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while cursor < input.len() {
        let next = MARKERS
            .iter()
            .filter_map(|marker| {
                lower[cursor..]
                    .find(marker)
                    .map(|offset| (cursor + offset, *marker))
            })
            .filter(|(start, marker)| {
                let marker_end = *start + marker.len();
                // Besides standalone field names, accept a sensitive marker at
                // the end of an environment-variable-like key (for example
                // `OPENAI_API_KEY=...`).  The delimiter check prevents ordinary
                // prose containing the word "key" or "token" from being treated
                // as an assignment.
                multica_boundary_ok(input, *start, marker_end)
                    || assignment_value_start(input, marker_end).is_some()
            })
            .min_by_key(|(start, marker)| (*start, usize::MAX - marker.len()));
        let Some((start, marker)) = next else {
            output.push_str(&input[cursor..]);
            break;
        };
        let marker_end = start + marker.len();
        let Some(value_start) = assignment_value_start(input, marker_end) else {
            output.push_str(&input[cursor..marker_end]);
            cursor = marker_end;
            continue;
        };
        if value_start >= input.len() {
            output.push_str(&input[cursor..]);
            break;
        }
        let (content_start, quote) = match input[value_start..].chars().next() {
            Some(quote @ ('\'' | '"')) => (value_start + quote.len_utf8(), Some(quote)),
            _ => (value_start, None),
        };
        let mut end = content_start;
        for (offset, character) in input[content_start..].char_indices() {
            if quote.is_some_and(|expected| character == expected)
                || (quote.is_none()
                    && (character.is_whitespace()
                        || matches!(character, ',' | ';' | '&' | '}' | ']')))
            {
                break;
            }
            end = content_start + offset + character.len_utf8();
        }
        if end <= content_start {
            output.push_str(&input[cursor..value_start]);
            cursor = value_start;
            continue;
        }
        output.push_str(&input[cursor..content_start]);
        output.push_str("[redacted]");
        cursor = end;
    }
    output
}

/// Replace URL paths/query strings and absolute filesystem paths.  URL origins
/// remain useful context, while credentials, request paths and query values do
/// not belong in a persisted snapshot or diagnostic.
fn redact_urls_and_paths(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        let rest = &input[index..];
        let lower = rest.to_ascii_lowercase();
        let url_prefix = [
            "https://", "http://", "wss://", "ws://", "ftp://", "file://",
        ]
        .iter()
        .find(|prefix| lower.starts_with(**prefix));
        if url_prefix.is_some() {
            let token_end = scan_redaction_token_end(rest, None, true);
            let raw = &rest[..token_end];
            if let Ok(url) = Url::parse(raw) {
                if let Some(host) = url.host_str() {
                    let host = if host.contains(':') {
                        format!("[{host}]")
                    } else {
                        host.to_string()
                    };
                    let port = url
                        .port()
                        .map(|port| format!(":{port}"))
                        .unwrap_or_default();
                    output.push_str(&format!(
                        "{}://{host}{port}",
                        url.scheme().to_ascii_lowercase()
                    ));
                } else {
                    output.push_str("[url]");
                }
            } else {
                output.push_str("[url]");
            }
            index += token_end;
            continue;
        }

        let bytes = rest.as_bytes();
        let windows_drive = bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/');
        let unc_path = rest.starts_with("\\\\");
        let unix_candidate = bytes.first() == Some(&b'/')
            && bytes.get(1) != Some(&b'/')
            && !matches!(bytes.get(1), Some(b' ' | b'\t' | b'\r' | b'\n'));
        let path_candidate = windows_drive || unc_path || unix_candidate;
        let token_end = if path_candidate {
            let quote = output
                .chars()
                .next_back()
                .filter(|character| matches!(character, '\'' | '"'));
            scan_redaction_token_end(rest, quote, false)
        } else {
            rest.chars().next().map(char::len_utf8).unwrap_or_default()
        };
        let path_token = &rest[..token_end];
        // Any non-empty slash-prefixed token is an absolute Unix path.  The
        // scanner already excludes `//` and slash followed by whitespace, so
        // this does not turn ordinary punctuation into a path while covering
        // less conventional roots such as `/secret` or `/app`.
        let unix_path = unix_candidate && path_token.len() > 1;
        if windows_drive || unc_path || unix_path {
            output.push_str("[path]");
            index += token_end;
            continue;
        }

        let character = rest
            .chars()
            .next()
            .expect("index remains on a char boundary");
        output.push(character);
        index += character.len_utf8();
    }
    output
}

fn scan_redaction_token_end(input: &str, quote: Option<char>, stop_whitespace: bool) -> usize {
    // `quoted` means the opening quote was already emitted before this token
    // (for example, `"C:\\Users\\Alice Smith\\file"`).
    let mut end = 0;
    for (offset, character) in input.char_indices() {
        if quote.is_some_and(|expected| character == expected) {
            break;
        }
        if quote.is_none()
            && ((stop_whitespace && character.is_whitespace())
                || matches!(
                    character,
                    '"' | '\'' | '<' | '>' | '`' | ',' | ';' | ')' | ']' | '}'
                ))
        {
            break;
        }
        end = offset + character.len_utf8();
    }
    if end == 0 && !input.is_empty() {
        input.chars().next().map(char::len_utf8).unwrap_or(0)
    } else {
        end
    }
}

fn diagnostic(code: &str) -> String {
    code.to_string()
}

/// Return the private log owned by the Multica adapter.  This is deliberately
/// separate from CCP's general diagnostic log: sidecar lifecycle records are
/// a different trust boundary and must never inherit arbitrary launcher or
/// proxy details.  The path contains no user-controlled connection material.
pub fn sidecar_lifecycle_log_path() -> PathBuf {
    #[cfg(test)]
    if let Some(lock) = SIDECAR_LIFECYCLE_LOG_PATH_OVERRIDE.get() {
        if let Ok(path) = lock.lock() {
            if let Some(path) = path.as_ref() {
                return path.clone();
            }
        }
    }
    crate::paths::default_multica_state_dir().join(SIDECAR_LIFECYCLE_LOG_FILE)
}

/// A deliberately narrow JSONL schema for sidecar lifecycle events.  Do not
/// add executable paths, working directories, argv, URLs, environment values,
/// stdout/stderr, or free-form error strings here.  The connection identifier
/// is represented only by a stable SHA-256 digest.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SidecarLifecycleRecord {
    timestamp_ms: u64,
    action: String,
    connection_id_hash: String,
    pid: Option<u32>,
    started_at_ms: Option<u64>,
    exited_at_ms: Option<u64>,
    exit_code: Option<i32>,
    status: Option<String>,
    diagnostic: Option<String>,
    /// Endpoint is a route category such as `health`, never a URL or host.
    endpoint: Option<String>,
    duration_ms: Option<u64>,
}

fn sidecar_lifecycle_log_lock() -> &'static Mutex<()> {
    SIDECAR_LIFECYCLE_LOG_LOCK.get_or_init(|| Mutex::new(()))
}

fn sidecar_connection_hash(connection_id: &str) -> String {
    let digest = Sha256::digest(connection_id.trim().as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Keep diagnostics in the lifecycle log to a small stable code vocabulary.
/// This guard is intentionally stricter than the public-text redactor: a
/// malformed/free-form value is replaced rather than partially persisted.
fn lifecycle_diagnostic_code(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty()
        || value.len() > 80
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || byte == b'_'
                || byte == b'-'
                || byte == b':'
        })
    {
        return Some("diagnostic_redacted".to_string());
    }
    Some(value.to_string())
}

fn lifecycle_status(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty()
        || value.len() > 32
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
    {
        return Some("unknown".to_string());
    }
    Some(value.to_string())
}

fn append_sidecar_lifecycle_record(record: &SidecarLifecycleRecord) -> std::io::Result<()> {
    append_sidecar_lifecycle_record_at(&sidecar_lifecycle_log_path(), record)
}

fn append_sidecar_lifecycle_record_at(
    path: &Path,
    record: &SidecarLifecycleRecord,
) -> std::io::Result<()> {
    let line = serde_json::to_string(record)
        .map_err(|error| std::io::Error::other(format!("lifecycle serialization: {error}")))?;
    let line_len = line.len().saturating_add(1) as u64;
    let _guard = sidecar_lifecycle_log_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(parent) = path.parent() {
        crate::settings::create_private_dir_all(parent)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
    }

    // Rotate before opening the active file.  Keep only one previous
    // generation and never let a failed rename prevent the current event from
    // being recorded.  Both files contain only the bounded schema above.
    if let Ok(metadata) = fs::metadata(&path)
        && metadata.len().saturating_add(line_len) > MAX_SIDECAR_LIFECYCLE_LOG_BYTES
    {
        let rotated = path
            .parent()
            .map(|parent| parent.join(SIDECAR_LIFECYCLE_LOG_ROTATED_FILE))
            .unwrap_or_else(|| PathBuf::from(SIDECAR_LIFECYCLE_LOG_ROTATED_FILE));
        // Windows rename does not replace an existing destination.  Removing
        // only our known rotated file keeps the operation bounded and scoped.
        let _ = fs::remove_file(&rotated);
        if fs::rename(&path, &rotated).is_err() {
            // If the active file cannot be renamed (for example, a transient
            // sharing violation), truncate only that exact path as a bounded
            // fallback.  A logging failure must never block sidecar control.
            let _ = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&path);
        } else {
            // A pre-existing file may have been created outside this process;
            // cap the retained generation as well so rotation cannot preserve
            // an unbounded blob indefinitely.
            let _ = cap_sidecar_lifecycle_file(&rotated);
        }
    }

    // `atomic_write` establishes the same private ACL/mode used by settings
    // files when the lifecycle log is first created.  Subsequent appends keep
    // that file identity and permissions intact.
    if !path.exists() {
        crate::settings::atomic_write(&path, b"")
            .map_err(|error| std::io::Error::other(error.to_string()))?;
    }
    let mut file = fs::OpenOptions::new().append(true).open(&path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

fn cap_sidecar_lifecycle_file(path: &Path) -> std::io::Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.len() <= MAX_SIDECAR_LIFECYCLE_LOG_BYTES {
        return Ok(());
    }
    // Keep complete JSONL records rather than cutting through a UTF-8/JSON
    // value. Read only one byte beyond the configured cap, so a malformed or
    // unexpectedly huge pre-existing file cannot force an unbounded buffer.
    let file = fs::File::open(path)?;
    let mut bounded = Vec::new();
    file.take(MAX_SIDECAR_LIFECYCLE_LOG_BYTES.saturating_add(1))
        .read_to_end(&mut bounded)?;
    bounded.truncate(MAX_SIDECAR_LIFECYCLE_LOG_BYTES as usize);
    if let Some(last_newline) = bounded.iter().rposition(|byte| *byte == b'\n') {
        bounded.truncate(last_newline + 1);
    } else {
        bounded.clear();
    }
    crate::settings::atomic_write(path, &bounded)
        .map_err(|error| std::io::Error::other(error.to_string()))
}

/// Best-effort lifecycle logging.  The sidecar must remain controllable even
/// when its private state directory is read-only or unavailable, so callers
/// intentionally ignore the result of this function.
fn log_sidecar_lifecycle(
    action: &str,
    connection_id: &str,
    pid: Option<u32>,
    started_at_ms: Option<u64>,
    exited_at_ms: Option<u64>,
    exit_code: Option<i32>,
    status: Option<&str>,
    diagnostic_code: Option<&str>,
) {
    log_sidecar_lifecycle_with_details(
        action,
        connection_id,
        pid,
        started_at_ms,
        exited_at_ms,
        exit_code,
        status,
        diagnostic_code,
        None,
        None,
    );
}

fn log_sidecar_lifecycle_with_details(
    action: &str,
    connection_id: &str,
    pid: Option<u32>,
    started_at_ms: Option<u64>,
    exited_at_ms: Option<u64>,
    exit_code: Option<i32>,
    status: Option<&str>,
    diagnostic_code: Option<&str>,
    endpoint: Option<&str>,
    duration_ms: Option<u64>,
) {
    let record = SidecarLifecycleRecord {
        timestamp_ms: now_ms(),
        action: action
            .trim()
            .chars()
            .take(48)
            .map(|character| {
                if character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '_' | '-' | '.')
                {
                    character
                } else {
                    '_'
                }
            })
            .collect(),
        connection_id_hash: sidecar_connection_hash(connection_id),
        pid,
        started_at_ms,
        exited_at_ms,
        exit_code,
        status: lifecycle_status(status),
        diagnostic: lifecycle_diagnostic_code(diagnostic_code),
        endpoint: endpoint
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                value
                    .chars()
                    .take(32)
                    .map(|character| {
                        if character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || matches!(character, '_' | '-' | '/')
                        {
                            character
                        } else {
                            '_'
                        }
                    })
                    .collect()
            }),
        duration_ms,
    };
    let _ = append_sidecar_lifecycle_record(&record);
}

fn log_sidecar_health(
    action: &str,
    connection_id: &str,
    pid: Option<u32>,
    started_at_ms: Option<u64>,
    status: &MulticaDaemonStatus,
) {
    log_sidecar_lifecycle_with_details(
        action,
        connection_id,
        pid.or(status.pid),
        started_at_ms.or(status.started_at_ms),
        status.exited_at_ms,
        status.exit_code,
        Some(&status.status),
        status.diagnostic.as_deref(),
        status.endpoint.as_deref(),
        status.duration_ms,
    );
}

#[cfg(test)]
#[doc(hidden)]
pub fn set_sidecar_lifecycle_log_path_for_tests(path: Option<PathBuf>) {
    let lock = SIDECAR_LIFECYCLE_LOG_PATH_OVERRIDE.get_or_init(|| Mutex::new(None));
    *lock
        .lock()
        .expect("sidecar lifecycle log path lock poisoned") = path;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaSidecarConfig {
    pub executable: String,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaConnectionConfig {
    #[serde(default)]
    pub connection_id: String,
    #[serde(default)]
    pub display_name: String,
    pub server_url: String,
    #[serde(default)]
    pub api_prefix: Option<String>,
    /// Optional workspace context sent only as the fixed Multica headers.
    /// User identity is intentionally not accepted from this configuration.
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub workspace_slug: Option<String>,
    /// Name of a protected environment variable.  The token value never
    /// enters this structure, the command line, logs, or the JSON file.
    #[serde(default)]
    pub token_env_var: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// HTTP outside loopback is opt-in and limited to private LAN addresses.
    /// This acknowledgement is persisted separately from the URL so a later
    /// edit cannot silently broaden the connection's transport policy.
    #[serde(default)]
    pub allow_insecure_lan_http: bool,
    #[serde(default)]
    pub sidecar: Option<MulticaSidecarConfig>,
    #[serde(default)]
    pub created_at_ms: u64,
    #[serde(default)]
    pub updated_at_ms: u64,
}

/// IPC input for creating or editing a connection.
///
/// `Option<Option<String>>` is intentional: serde leaves the outer option as
/// `None` when the property is omitted, while an explicit JSON `null` becomes
/// `Some(None)`.  This lets an edit preserve an existing token reference unless
/// the caller explicitly asks to clear it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaConnectionInput {
    #[serde(default)]
    pub connection_id: Option<String>,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub server_url: String,
    #[serde(default)]
    pub api_prefix: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub workspace_slug: Option<String>,
    #[serde(default)]
    pub token_env_var: Option<Option<String>>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub allow_insecure_lan_http: bool,
    #[serde(default)]
    pub sidecar: Option<Option<MulticaSidecarConfig>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticaConnectionView {
    pub connection_id: String,
    pub display_name: String,
    /// A display-safe origin only. The persisted URL's path, query, fragment,
    /// and any malformed input are intentionally never exposed over IPC.
    pub server_url_display: String,
    pub api_prefix: Option<String>,
    pub workspace_id: Option<String>,
    pub workspace_slug: Option<String>,
    pub enabled: bool,
    /// Safe to expose because it is a boolean acknowledgement only; the
    /// actual saved address stays redacted behind `server_url_display`.
    pub allow_insecure_lan_http: bool,
    pub token_configured: bool,
    pub sidecar_configured: bool,
    pub sidecar_executable_name: Option<String>,
    pub sidecar_auto_start: bool,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl MulticaConnectionConfig {
    pub fn view(&self) -> MulticaConnectionView {
        let (sidecar_configured, sidecar_executable_name, sidecar_auto_start) = self
            .sidecar
            .as_ref()
            .map(|sidecar| {
                (
                    true,
                    Path::new(&sidecar.executable)
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string()),
                    sidecar.auto_start,
                )
            })
            .unwrap_or((false, None, false));
        MulticaConnectionView {
            connection_id: self.connection_id.clone(),
            display_name: self.display_name.clone(),
            server_url_display: server_url_display(&self.server_url),
            api_prefix: self.api_prefix.clone(),
            workspace_id: self.workspace_id.clone(),
            workspace_slug: self.workspace_slug.clone(),
            enabled: self.enabled,
            allow_insecure_lan_http: self.allow_insecure_lan_http,
            token_configured: token_is_configured(self),
            sidecar_configured,
            sidecar_executable_name,
            sidecar_auto_start,
            created_at_ms: self.created_at_ms,
            updated_at_ms: self.updated_at_ms,
        }
    }
}

fn server_url_display(raw: &str) -> String {
    let Ok(url) = Url::parse(raw.trim()) else {
        return "已隐藏".to_string();
    };
    let Some(host) = url.host_str() else {
        return "已隐藏".to_string();
    };
    let host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    let port = url
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    format!("{}://{host}{port}/", url.scheme().to_ascii_lowercase())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MulticaHealthStatus {
    pub status: String,
    pub endpoint: Option<String>,
    pub http_status: Option<u16>,
    pub version: Option<String>,
    pub checked_at_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MulticaDaemonStatus {
    pub status: String,
    pub pid: Option<u32>,
    pub started_at_ms: Option<u64>,
    pub exited_at_ms: Option<u64>,
    pub exit_code: Option<i32>,
    /// The probe category and HTTP metadata are kept separate from the
    /// process lifecycle fields.  This prevents a live child from being
    /// mistaken for a ready daemon and lets the UI explain the last probe.
    pub endpoint: Option<String>,
    pub http_status: Option<u16>,
    pub version: Option<String>,
    pub checked_at_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MulticaConnectionStatus {
    pub connection_id: String,
    pub server: MulticaHealthStatus,
    pub daemon: MulticaDaemonStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MulticaRuntimeItem {
    pub id: String,
    pub name: Option<String>,
    pub title: Option<String>,
    pub status: String,
    /// A bounded, human-readable explanation when the upstream status is not
    /// one of the statuses understood by this adapter.
    pub diagnostic: Option<String>,
    pub updated_at_ms: Option<u64>,
    pub runtime_type: Option<String>,
    /// The provider reported by Multica for this runtime.  This is exposed as
    /// read-only runtime metadata and is intentionally never mapped to a CCP
    /// supplier, profile, or upstream URL.
    pub provider: Option<String>,
    pub error_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MulticaRuntimeSnapshot {
    pub source_connection_id: String,
    pub fetched_at_ms: u64,
    pub stale: bool,
    pub runtimes: Vec<MulticaRuntimeItem>,
    pub agents: Vec<MulticaRuntimeItem>,
    pub tasks: Vec<MulticaRuntimeItem>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedConnections {
    #[serde(default)]
    connections: Vec<MulticaConnectionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedSnapshots {
    #[serde(default)]
    snapshots: Vec<MulticaRuntimeSnapshot>,
}

#[derive(Clone)]
pub struct MulticaStore {
    connections_path: PathBuf,
    snapshots_path: PathBuf,
}

struct MulticaConnectionFileLock {
    file: fs::File,
}

impl MulticaConnectionFileLock {
    fn acquire(connections_path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = connections_path.parent() {
            crate::settings::create_private_dir_all(parent)?;
        }
        let file_name = connections_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("Multica 连接锁路径无效。"))?;
        let lock_path = connections_path.with_file_name(format!("{file_name}.lock"));
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(lock_path)
            .context("打开 Multica 连接锁失败。")?;
        file.lock_exclusive().context("获取 Multica 连接锁失败。")?;
        Ok(Self { file })
    }
}

impl Drop for MulticaConnectionFileLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

impl Default for MulticaStore {
    fn default() -> Self {
        let root = crate::paths::default_multica_state_dir();
        Self::new(root.join(CONNECTIONS_FILE), root.join(SNAPSHOTS_FILE))
    }
}

impl MulticaStore {
    pub fn new(connections_path: PathBuf, snapshots_path: PathBuf) -> Self {
        Self {
            connections_path,
            snapshots_path,
        }
    }

    pub fn load_connections(&self) -> anyhow::Result<Vec<MulticaConnectionConfig>> {
        let _guard = store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _file_lock = MulticaConnectionFileLock::acquire(&self.connections_path)?;
        read_connections(&self.connections_path)
    }

    /// Create the fixed managed record or refresh only its immutable sidecar
    /// contract while one in-process and cross-process transaction is held.
    /// User-editable fields are never copied from a stale pre-lock snapshot.
    fn ensure_managed_connection_record(
        &self,
        expected_sidecar: Option<MulticaSidecarConfig>,
    ) -> anyhow::Result<MulticaConnectionConfig> {
        let _processes = sidecars()
            .lock()
            .map_err(|_| anyhow!("sidecar 状态锁不可用。"))?;
        let _guard = store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _file_lock = MulticaConnectionFileLock::acquire(&self.connections_path)?;
        let mut connections = read_connections(&self.connections_path)?;
        let now = now_ms();

        if let Some(index) = connections
            .iter()
            .position(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
        {
            let changed = expected_sidecar
                .as_ref()
                .is_some_and(|expected| connections[index].sidecar.as_ref() != Some(expected));
            if changed {
                connections[index].sidecar = expected_sidecar;
                connections[index].updated_at_ms = now;
                write_connections(&self.connections_path, &connections)?;
            }
            return Ok(connections[index].clone());
        }

        let defaults = MulticaManagedConnection::default();
        let connection = MulticaConnectionConfig {
            connection_id: defaults.connection_id,
            display_name: defaults.display_name,
            server_url: defaults.server_url,
            api_prefix: None,
            workspace_id: None,
            workspace_slug: None,
            token_env_var: None,
            enabled: defaults.enabled,
            allow_insecure_lan_http: false,
            sidecar: expected_sidecar,
            created_at_ms: now,
            updated_at_ms: now,
        };
        connections.push(connection.clone());
        write_connections(&self.connections_path, &connections)?;
        Ok(connection)
    }

    pub fn save_connection(
        &self,
        connection: MulticaConnectionConfig,
    ) -> anyhow::Result<MulticaConnectionConfig> {
        self.save_connection_with_scope(connection, false)
    }

    /// The managed record is written only by the dedicated runtime workflow.
    /// Keeping this narrow escape hatch private prevents ordinary connection
    /// CRUD and IPC calls from changing the reserved connection ID.
    #[cfg(test)]
    fn save_managed_connection(
        &self,
        connection: MulticaConnectionConfig,
    ) -> anyhow::Result<MulticaConnectionConfig> {
        self.save_connection_with_scope(connection, true)
    }

    /// Add the fixed managed sidecar only when an existing managed record has
    /// none.  This intentionally bypasses generic connection validation: a
    /// user may have saved an empty or otherwise not-yet-checkable managed URL
    /// and runtime installation must not rewrite or reject that value.
    #[cfg(test)]
    fn attach_managed_sidecar_if_missing(
        &self,
        sidecar: MulticaSidecarConfig,
    ) -> anyhow::Result<MulticaConnectionConfig> {
        let _processes = sidecars()
            .lock()
            .map_err(|_| anyhow!("sidecar 状态锁不可用。"))?;
        let _guard = store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _file_lock = MulticaConnectionFileLock::acquire(&self.connections_path)?;
        let mut connections = read_connections(&self.connections_path)?;
        let (saved, changed) = {
            let connection = connections
                .iter_mut()
                .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
                .ok_or_else(|| anyhow!("managed_runtime_connection_missing"))?;
            if connection.sidecar.is_none() {
                connection.sidecar = Some(sidecar);
                connection.updated_at_ms = now_ms();
                (connection.clone(), true)
            } else {
                (connection.clone(), false)
            }
        };
        if changed {
            write_connections(&self.connections_path, &connections)?;
        }
        Ok(saved)
    }

    /// Rebind the complete immutable managed sidecar contract after the active
    /// runtime pointer changes. Connection-level user values are retained
    /// verbatim; ordinary connections are never visited or rewritten.
    fn rebind_managed_sidecar_contract_if_present(
        &self,
        executable: PathBuf,
    ) -> anyhow::Result<Option<MulticaConnectionConfig>> {
        let _processes = sidecars()
            .lock()
            .map_err(|_| anyhow!("sidecar 状态锁不可用。"))?;
        let _guard = store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _file_lock = MulticaConnectionFileLock::acquire(&self.connections_path)?;
        let original_connections = read_optional_managed_file(&self.connections_path)?;
        let mut connections = read_connections(&self.connections_path)?;
        let Some(index) = connections
            .iter()
            .position(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
        else {
            return Ok(None);
        };

        let expected_sidecar = managed_sidecar_for_executable(executable)?;
        let connection = &mut connections[index];
        let changed = connection.sidecar.as_ref() != Some(&expected_sidecar);
        if changed {
            connection.sidecar = Some(expected_sidecar);
            connection.updated_at_ms = now_ms();
            if let Err(error) = write_connections(&self.connections_path, &connections) {
                restore_managed_pointer(&self.connections_path, original_connections.as_deref())
                    .context("托管 Multica 连接重绑定恢复失败。")?;
                return Err(error);
            }
        }
        Ok(Some(connections[index].clone()))
    }

    /// Persist only the three managed Runtime fields that the user can edit.
    /// This is deliberately not routed through `save_connection_with_scope`:
    /// generic connection saves preserve a blank URL from the old record and
    /// validate/normalize fields that must remain raw for the managed form.
    fn update_managed_connection_values(
        &self,
        display_name: String,
        server_url: String,
        enabled: bool,
    ) -> anyhow::Result<MulticaConnectionConfig> {
        let _processes = sidecars()
            .lock()
            .map_err(|_| anyhow!("sidecar 状态锁不可用。"))?;
        let _guard = store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _file_lock = MulticaConnectionFileLock::acquire(&self.connections_path)?;
        let mut connections = read_connections(&self.connections_path)?;
        let connection = connections
            .iter_mut()
            .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
            .ok_or_else(|| anyhow!("managed_runtime_connection_missing"))?;
        connection.display_name = display_name;
        connection.server_url = server_url;
        connection.enabled = enabled;
        connection.updated_at_ms = now_ms();
        let saved = connection.clone();
        write_connections(&self.connections_path, &connections)?;
        Ok(saved)
    }

    /// Persist only the managed enable switch under the same cross-process
    /// transaction used by the full managed editor. This prevents a stale
    /// pre-lock name or URL snapshot from overwriting a concurrent user save.
    fn update_managed_enabled(&self, enabled: bool) -> anyhow::Result<MulticaConnectionConfig> {
        let _processes = sidecars()
            .lock()
            .map_err(|_| anyhow!("sidecar 状态锁不可用。"))?;
        let _guard = store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _file_lock = MulticaConnectionFileLock::acquire(&self.connections_path)?;
        let mut connections = read_connections(&self.connections_path)?;
        let connection = connections
            .iter_mut()
            .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
            .ok_or_else(|| anyhow!("managed_runtime_connection_missing"))?;
        connection.enabled = enabled;
        connection.updated_at_ms = now_ms();
        let saved = connection.clone();
        write_connections(&self.connections_path, &connections)?;
        Ok(saved)
    }

    fn save_connection_with_scope(
        &self,
        mut connection: MulticaConnectionConfig,
        allow_managed_connection: bool,
    ) -> anyhow::Result<MulticaConnectionConfig> {
        // Serialize configuration writes with sidecar lifecycle mutations.
        // The registry lock is intentionally acquired first, matching
        // `delete_connection` and the final lock in `start_sidecar`; this
        // prevents an edit from disabling/replacing a live child between its
        // ownership check and the atomic settings write.
        let mut processes = sidecars()
            .lock()
            .map_err(|_| anyhow!("sidecar 状态锁不可用。"))?;
        let _guard = store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _file_lock = MulticaConnectionFileLock::acquire(&self.connections_path)?;
        let mut connections = read_connections(&self.connections_path)?;
        if connection.connection_id.trim().is_empty() {
            connection.connection_id = generated_connection_id();
        } else {
            connection.connection_id = connection.connection_id.trim().to_string();
        }
        if !allow_managed_connection && is_managed_connection_id(&connection.connection_id) {
            bail!(MANAGED_CONNECTION_RESERVED_ERROR);
        }
        if connection.server_url.trim().is_empty() {
            if let Some(existing) = connections
                .iter()
                .find(|item| item.connection_id == connection.connection_id)
            {
                connection.server_url = existing.server_url.clone();
            }
        }
        // A blank URL only has meaning for an existing edit above. New
        // connections still reach this validation with an empty URL and fail.
        validate_connection(&connection)?;
        connection.token_env_var = normalize_token_env_var(connection.token_env_var)?;
        connection.workspace_id =
            normalize_workspace_context(connection.workspace_id, "workspace_id")?;
        connection.workspace_slug =
            normalize_workspace_context(connection.workspace_slug, "workspace_slug")?;
        let canonical_key = canonical_connection_key(&connection);
        if connections.iter().any(|existing| {
            existing.connection_id != connection.connection_id
                && canonical_connection_key(existing) == canonical_key
        }) {
            bail!("Multica 连接已存在，请编辑已有连接记录。")
        }
        let now = now_ms();
        if let Some(existing) = connections
            .iter_mut()
            .find(|item| item.connection_id == connection.connection_id)
        {
            ensure_sidecar_runtime_update_allowed(&mut processes, existing, &connection)?;
            connection.created_at_ms = if existing.created_at_ms == 0 {
                now
            } else {
                existing.created_at_ms
            };
            connection.updated_at_ms = now;
            *existing = connection.clone();
        } else {
            // A stale or externally edited connections file must not let a
            // new record reuse an ID that still owns a live child in this
            // process.  Treat the in-memory registration as authoritative for
            // lifecycle safety even when persistence no longer has a matching
            // row; exited children are reaped before the ID can be reused.
            ensure_sidecar_registration_available(&mut processes, &connection.connection_id)?;
            if connection.created_at_ms == 0 {
                connection.created_at_ms = now;
            }
            connection.updated_at_ms = now;
            connections.push(connection.clone());
        }
        write_connections(&self.connections_path, &connections)?;
        drop(processes);
        Ok(connection)
    }

    /// Save an IPC connection input while preserving old protected fields when
    /// their properties were omitted. Explicit `null` clears a field and an
    /// object replaces it. This is important because the renderer only gets a
    /// redacted sidecar summary and must not be forced to echo local paths.
    pub fn save_connection_input(
        &self,
        input: MulticaConnectionInput,
    ) -> anyhow::Result<MulticaConnectionConfig> {
        let connection_id = input
            .connection_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string();
        let existing = if connection_id.is_empty() {
            None
        } else {
            self.load_connections()?
                .into_iter()
                .find(|connection| connection.connection_id == connection_id)
        };
        let token_env_var = match input.token_env_var {
            None => existing
                .as_ref()
                .and_then(|connection| connection.token_env_var.clone()),
            Some(value) => normalize_token_env_var(value)?,
        };
        let sidecar = match input.sidecar {
            None => existing
                .as_ref()
                .and_then(|connection| connection.sidecar.clone()),
            Some(value) => value,
        };
        let server_url = match existing.as_ref() {
            Some(connection) if input.server_url.trim().is_empty() => connection.server_url.clone(),
            _ => input.server_url,
        };
        self.save_connection(MulticaConnectionConfig {
            connection_id,
            display_name: input.display_name,
            server_url,
            api_prefix: input.api_prefix,
            workspace_id: normalize_workspace_context(input.workspace_id, "workspace_id")?,
            workspace_slug: normalize_workspace_context(input.workspace_slug, "workspace_slug")?,
            token_env_var,
            enabled: input.enabled,
            allow_insecure_lan_http: input.allow_insecure_lan_http,
            sidecar,
            created_at_ms: 0,
            updated_at_ms: 0,
        })
    }

    pub fn delete_connection(&self, connection_id: &str) -> anyhow::Result<bool> {
        if is_managed_connection_id(connection_id) {
            bail!(MANAGED_CONNECTION_RESERVED_ERROR);
        }
        // Keep the registry lock through the persisted delete.  A start first
        // validates the saved record and then takes this same lock before it
        // can spawn/insert; releasing it after the liveness check would leave
        // a window where a concurrent start could create an orphan sidecar
        // just as the connection is removed from disk.
        let mut processes = sidecars()
            .lock()
            .map_err(|_| anyhow!("sidecar 状态锁不可用。"))?;
        if let Some(process) = processes.get_mut(connection_id) {
            match sidecar_process_state(process) {
                SidecarProcessState::RunningOwned => {
                    bail!("请先停止该 Multica sidecar，再删除连接。")
                }
                SidecarProcessState::RunningForeign
                | SidecarProcessState::RunningUnverified
                | SidecarProcessState::StatusUnavailable => {
                    bail!("无法验证 Multica sidecar 进程归属，已拒绝删除。")
                }
                SidecarProcessState::Exited(_) => {
                    // The child has already exited; reap it before removing
                    // the in-memory record. No PID is ever reconstructed or
                    // terminated from persisted data.
                    let mut process = processes
                        .remove(connection_id)
                        .expect("sidecar record exists while locked");
                    let _ = process.child.wait();
                }
            }
        }
        let _guard = store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _file_lock = MulticaConnectionFileLock::acquire(&self.connections_path)?;
        let mut connections = read_connections(&self.connections_path)?;
        let before = connections.len();
        connections.retain(|item| item.connection_id != connection_id);
        if before != connections.len() {
            write_connections(&self.connections_path, &connections)?;
            if let Ok(mut cache) = snapshot_cache().lock() {
                cache.remove(connection_id);
            }
        }
        // Explicitly release before returning; this makes the lock boundary
        // obvious and keeps future changes from accidentally holding it while
        // doing unrelated work.
        drop(processes);
        Ok(before != connections.len())
    }

    pub fn save_snapshot(&self, snapshot: &MulticaRuntimeSnapshot) -> anyhow::Result<()> {
        let _guard = store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut snapshots = read_snapshots(&self.snapshots_path)?;
        snapshots.retain(|item| item.source_connection_id != snapshot.source_connection_id);
        snapshots.push(snapshot.clone());
        if snapshots.len() > 32 {
            snapshots.sort_by_key(|item| item.fetched_at_ms);
            snapshots.drain(..snapshots.len() - 32);
        }
        write_snapshots(&self.snapshots_path, &snapshots)?;
        if let Ok(mut cache) = snapshot_cache().lock() {
            cache.insert(snapshot.source_connection_id.clone(), snapshot.clone());
        }
        Ok(())
    }

    /// Commit a snapshot only while the request lease is still the newest
    /// lease for its connection.  The active-request lock is held through the
    /// atomic file write, so a newer request cannot begin between the check and
    /// the cache replacement.
    fn save_snapshot_if_current(
        &self,
        snapshot: &MulticaRuntimeSnapshot,
        request: &RequestGuard,
    ) -> anyhow::Result<bool> {
        let requests = active_requests()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !requests
            .get(&request.key)
            .is_some_and(|active| active.sequence == request.sequence)
        {
            return Ok(false);
        }
        self.save_snapshot(snapshot)?;
        Ok(true)
    }

    pub fn load_snapshot(
        &self,
        connection_id: &str,
    ) -> anyhow::Result<Option<MulticaRuntimeSnapshot>> {
        if let Ok(cache) = snapshot_cache().lock() {
            if let Some(snapshot) = cache.get(connection_id) {
                return Ok(Some(snapshot.clone()));
            }
        }
        let _guard = store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let snapshot = read_snapshots(&self.snapshots_path)?
            .into_iter()
            .find(|item| item.source_connection_id == connection_id);
        if let Some(snapshot) = snapshot.as_ref() {
            if let Ok(mut cache) = snapshot_cache().lock() {
                cache.insert(connection_id.to_string(), snapshot.clone());
            }
        }
        Ok(snapshot)
    }
}

fn read_connections(path: &Path) -> anyhow::Result<Vec<MulticaConnectionConfig>> {
    match fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice::<PersistedConnections>(&bytes)
            .context("Multica 连接文件格式无效")?
            .connections),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error).with_context(|| "读取 Multica 连接失败。"),
    }
}

fn write_connections(path: &Path, connections: &[MulticaConnectionConfig]) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec_pretty(&PersistedConnections {
        connections: connections.to_vec(),
    })?;
    crate::settings::atomic_write(path, &bytes).map_err(|_| anyhow!("写入 Multica 连接失败。"))
}

fn read_snapshots(path: &Path) -> anyhow::Result<Vec<MulticaRuntimeSnapshot>> {
    match fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice::<PersistedSnapshots>(&bytes)
            .context("Multica 快照文件格式无效")?
            .snapshots),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error).with_context(|| "读取 Multica 快照失败。"),
    }
}

fn write_snapshots(path: &Path, snapshots: &[MulticaRuntimeSnapshot]) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec_pretty(&PersistedSnapshots {
        snapshots: snapshots.to_vec(),
    })?;
    crate::settings::atomic_write(path, &bytes).map_err(|_| anyhow!("写入 Multica 快照失败。"))
}

fn normalize_token_env_var(value: Option<String>) -> anyhow::Result<Option<String>> {
    let Some(value) = value else { return Ok(None) };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let valid = value.bytes().enumerate().all(|(index, byte)| {
        if index == 0 {
            byte == b'_' || byte.is_ascii_alphabetic()
        } else {
            byte == b'_' || byte.is_ascii_alphanumeric()
        }
    });
    if !valid {
        bail!("令牌环境变量名无效，只允许字母、数字和下划线。")
    }
    Ok(Some(value.to_string()))
}

fn token_is_configured(connection: &MulticaConnectionConfig) -> bool {
    connection
        .token_env_var
        .as_deref()
        .and_then(|name| std::env::var_os(name))
        .is_some_and(|value| !value.is_empty())
}

fn validate_connection(connection: &MulticaConnectionConfig) -> anyhow::Result<()> {
    let raw = connection.server_url.trim();
    if raw.is_empty() {
        bail!("Multica 服务地址不能为空。")
    }
    let url = Url::parse(raw).context("Multica 服务地址格式无效。")?;
    if url.username() != "" || url.password().is_some() {
        bail!("Multica 服务地址不能包含用户名或密码。")
    }
    if url.query().is_some() || url.fragment().is_some() {
        bail!("Multica 服务地址不能包含查询参数或片段。")
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("Multica 服务地址缺少主机名。"))?;
    if url.port_or_known_default().is_some_and(is_forbidden_port) {
        bail!("Multica 服务地址不得使用 CCP 固定端口。")
    }
    match url.scheme().to_ascii_lowercase().as_str() {
        "https" => {}
        "http" if is_loopback_host(&url) => {}
        "http" if connection.allow_insecure_lan_http && is_private_lan_host(&url) => {}
        "http" if connection.allow_insecure_lan_http => {
            bail!("非加密 Multica 地址必须是本机或已确认的局域网地址。")
        }
        "http" => bail!("非加密 Multica 地址仅允许 loopback；局域网 HTTP 需明确确认。"),
        _ => bail!("Multica 服务地址只允许 https，或已确认的本机/局域网 http。"),
    }
    if host.is_empty() {
        bail!("Multica 服务地址缺少主机名。")
    }
    if let Some(prefix) = connection.api_prefix.as_deref() {
        validate_api_prefix(prefix)?;
    }
    normalize_workspace_context(connection.workspace_id.clone(), "workspace_id")?;
    normalize_workspace_context(connection.workspace_slug.clone(), "workspace_slug")?;
    // Validate the reference without reading or persisting its value.  The
    // token itself remains in the protected environment only.
    normalize_token_env_var(connection.token_env_var.clone())?;
    if let Some(sidecar) = connection.sidecar.as_ref() {
        validate_sidecar_config(sidecar)?;
    }
    Ok(())
}

/// Validate the optional path prefix as a relative path fragment.  It is
/// appended to the saved server URL for read-only requests, so query/fragment
/// syntax, control characters, traversal segments, and URL/user-info forms
/// must be rejected rather than percent-encoded into a surprising endpoint.
fn validate_api_prefix(prefix: &str) -> anyhow::Result<()> {
    let prefix = prefix.trim();
    if prefix.is_empty() {
        return Ok(());
    }
    if prefix.chars().count() > 160
        || prefix.chars().any(char::is_control)
        || prefix.contains(['?', '#', '\\', '@'])
        || prefix.contains("//")
        || prefix
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        || prefix
            .split('/')
            .next()
            .is_some_and(|segment| segment.contains(':'))
    {
        bail!("Multica API 前缀格式无效。")
    }
    Ok(())
}

/// Accept only address forms that are intrinsically local.  This deliberately
/// avoids resolving arbitrary DNS names while saving configuration, which
/// would both block the GUI and make a DNS-rebinding policy hard to audit.
fn is_private_lan_host(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    if let Ok(address) = host.parse::<IpAddr>() {
        return match address {
            IpAddr::V4(address) => address.is_private() || address.is_link_local(),
            IpAddr::V6(address) => {
                let octets = address.octets();
                // RFC 4193 unique-local (fc00::/7) and RFC 4291 link-local
                // (fe80::/10) are the two IPv6 equivalents of a LAN host.
                (octets[0] & 0xfe) == 0xfc || (octets[0] == 0xfe && (octets[1] & 0xc0) == 0x80)
            }
        };
    }
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host.ends_with(".local") || host.ends_with(".lan")
}

fn is_forbidden_port(port: u16) -> bool {
    FORBIDDEN_PORTS.iter().any(|value| {
        value
            .parse::<u16>()
            .map(|forbidden| forbidden == port)
            .unwrap_or(false)
    })
}

fn normalize_workspace_context(
    value: Option<String>,
    field_name: &str,
) -> anyhow::Result<Option<String>> {
    let Some(value) = value else { return Ok(None) };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 160 || value.chars().any(char::is_control) {
        bail!("Multica {field_name} 格式无效。")
    }
    Ok(Some(value.to_string()))
}

fn validate_sidecar_config(sidecar: &MulticaSidecarConfig) -> anyhow::Result<()> {
    // Validate files when the configuration is saved, not only right before
    // spawning. This prevents a stale free-text path from becoming a
    // persisted executable instruction that is discovered much later.
    let _ = verified_sidecar_executable_path(&sidecar.executable)?;
    if let Some(working_dir) = sidecar
        .working_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let _ = verified_sidecar_working_dir(working_dir)?;
    }
    for (index, arg) in sidecar.args.iter().enumerate() {
        let argument = arg.trim();
        if argument.chars().any(char::is_control) {
            bail!("sidecar 参数格式无效。")
        }
        let lower = argument.to_ascii_lowercase();
        let flag = lower
            .split_once('=')
            .map_or(lower.as_str(), |(flag, _)| flag);
        if is_forbidden_sidecar_credential_flag(flag)
            || argument == "-H"
            || argument.starts_with("-H") && argument.len() > 2
            || is_bare_environment_assignment(argument)
            || contains_obvious_credential_literal(argument)
        {
            bail!("sidecar 参数包含受保护凭据或不允许的环境赋值。")
        }
        // Listener values are not always attached to a conventional
        // `--port`/`--listen` flag. Inspect every argument for an explicit
        // host:port or key=port composite so a renamed/custom flag cannot
        // smuggle one of CCP's reserved ports through validation.
        if contains_forbidden_port_in_composite(argument) {
            bail!("sidecar 参数包含受保护凭据或 CCP 固定端口。")
        }
        if is_sidecar_port_flag(flag) {
            let inline_value = lower
                .split_once('=')
                .and_then(|(_, value)| Some(value.trim()));
            let next_value = sidecar.args.get(index + 1).map(|value| value.trim());
            if inline_value.is_some_and(is_forbidden_port_text)
                || (inline_value.is_none() && next_value.is_some_and(is_forbidden_port_text))
            {
                bail!("sidecar 参数包含受保护凭据或 CCP 固定端口。")
            }
        }
    }
    validated_sidecar_profile(&sidecar.args).map(|_| ())
}

fn verified_sidecar_executable_path(raw: &str) -> anyhow::Result<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() {
        bail!("Multica sidecar 可执行文件不能为空。")
    }
    let path = Path::new(raw);
    if !path.is_absolute() {
        bail!("Multica sidecar 可执行文件必须是已验证的绝对本地路径。")
    }
    #[cfg(windows)]
    if !is_local_windows_path(path) {
        bail!("Multica sidecar 可执行文件必须位于本机磁盘。")
    }
    let executable =
        fs::canonicalize(path).with_context(|| "Multica sidecar 可执行文件不存在或无法读取。")?;
    #[cfg(windows)]
    if !is_local_windows_path(&executable) {
        bail!("Multica sidecar 可执行文件必须位于本机磁盘。")
    }
    let metadata = fs::metadata(&executable)?;
    if !metadata.is_file() {
        bail!("Multica sidecar 路径不是文件。")
    }
    // `Command::new` can execute an executable shebang file directly on Unix,
    // which would turn a user-controlled wrapper into an arbitrary shell
    // command. The sidecar boundary accepts a native binary only; reject
    // scripts before persisting or spawning them. Reading a fixed two-byte
    // prefix keeps validation bounded even for a large binary.
    if is_shebang_script(&executable)? {
        bail!("Multica sidecar 必须是本地原生可执行文件，不得执行脚本。")
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            bail!("Multica sidecar 文件没有可执行权限。")
        }
    }
    #[cfg(windows)]
    if !matches!(
        executable
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("exe" | "com")
    ) {
        bail!("Windows sidecar 必须是已验证的 exe 或 com 文件。")
    }
    Ok(executable)
}

fn is_shebang_script(path: &Path) -> anyhow::Result<bool> {
    let mut file = fs::File::open(path)?;
    let mut prefix = [0_u8; 2];
    let read = file.read(&mut prefix)?;
    Ok(read == prefix.len() && prefix == [b'#', b'!'])
}

fn verified_sidecar_working_dir(raw: &str) -> anyhow::Result<PathBuf> {
    let raw = raw.trim();
    let path = Path::new(raw);
    #[cfg(windows)]
    if path.is_absolute() && !is_local_windows_path(path) {
        bail!("Multica sidecar 工作目录必须位于本机磁盘。")
    }
    let working_dir =
        fs::canonicalize(path).with_context(|| "Multica sidecar 工作目录不存在或无法读取。")?;
    #[cfg(windows)]
    if !is_local_windows_path(&working_dir) {
        bail!("Multica sidecar 工作目录必须位于本机磁盘。")
    }
    if !working_dir.is_dir() {
        bail!("Multica sidecar 工作目录不是目录。")
    }
    Ok(working_dir)
}

#[cfg(windows)]
fn is_local_windows_path(path: &Path) -> bool {
    use std::path::{Component, Prefix};

    matches!(
        path.components().next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    )
}

/// Multica's daemon health listener uses a deterministic byte-sum offset for
/// each explicitly configured profile. Keep this derivation local to the
/// adapter: the configured server URL is never used as a proxy or rewritten to
/// this address.
fn daemon_health_port_for_profile(profile: &str) -> u16 {
    let offset = profile
        .as_bytes()
        .iter()
        .fold(0u32, |sum, byte| sum.saturating_add(u32::from(*byte)))
        % 1000;
    DEFAULT_DAEMON_HEALTH_PORT.saturating_add(1 + offset as u16)
}

/// Read and validate exactly one explicit, non-default profile from the argv
/// forms supported by the Multica CLI. The daemon's health listener is profile
/// scoped, so accepting an unnamed/default profile could inspect another
/// process and incorrectly report this sidecar as healthy.
fn validated_sidecar_profile(args: &[String]) -> anyhow::Result<String> {
    let mut profile = None;
    let mut index = 0;
    while index < args.len() {
        let argument = args[index].trim();
        let value = if let Some(value) = argument.strip_prefix("--profile=") {
            Some(value)
        } else if argument == "--profile" {
            index += 1;
            Some(
                args.get(index)
                    .ok_or_else(|| anyhow!("sidecar 必须提供具名 profile。"))?
                    .trim(),
            )
        } else {
            None
        };
        if let Some(value) = value {
            let value = value.trim();
            if profile.is_some() {
                bail!("sidecar 只能提供一个 profile。")
            }
            if value.is_empty()
                || value.eq_ignore_ascii_case("default")
                || value.starts_with('-')
                || value.contains(['/', '\\', ':'])
                || value
                    .chars()
                    .any(|character| character.is_control() || character.is_whitespace())
                || value.chars().count() > 160
            {
                bail!("sidecar 必须提供有效的具名 profile。")
            }
            profile = Some(value.to_string());
        }
        index += 1;
    }
    profile.ok_or_else(|| anyhow!("sidecar 必须显式提供具名 profile。"))
}

fn daemon_health_endpoint(profile: &str) -> anyhow::Result<Url> {
    Url::parse(&format!(
        "http://127.0.0.1:{}/health",
        daemon_health_port_for_profile(profile)
    ))
    .context("Multica daemon health endpoint 无效。")
}

fn is_forbidden_sidecar_credential_flag(flag: &str) -> bool {
    matches!(
        flag,
        "--token"
            | "--api-key"
            | "--api_key"
            | "--authorization"
            | "--bearer"
            | "--access-token"
            | "--access_token"
            | "--secret"
            | "--password"
            | "--passwd"
            | "--cookie"
            | "--header"
            | "--headers"
            | "--env"
            | "--env-file"
            | "--env_file"
            | "--credentials"
            | "--credential"
    )
}

fn is_bare_environment_assignment(argument: &str) -> bool {
    let Some((name, _)) = argument.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte == b'_' || byte.is_ascii_alphabetic()
            } else {
                byte == b'_' || byte.is_ascii_alphanumeric()
            }
        })
}

fn contains_obvious_credential_literal(argument: &str) -> bool {
    let lower = argument.trim().to_ascii_lowercase();
    [
        "authorization:",
        "authorization=",
        "bearer ",
        "bearer%20",
        "api_key=",
        "api-key=",
        "access_token=",
        "access-token=",
        "token=",
        "secret=",
        "password=",
        "passwd=",
        "cookie:",
        "cookie=",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
        || Url::parse(argument)
            .ok()
            .is_some_and(|url| !url.username().is_empty() || url.password().is_some())
}

fn is_sidecar_port_flag(flag: &str) -> bool {
    let normalized = flag
        .trim_start_matches('-')
        .replace('_', "-")
        .to_ascii_lowercase();
    normalized == "port"
        || normalized == "listen"
        || normalized == "bind"
        || normalized == "address"
        || normalized == "addr"
        || normalized.ends_with("-port")
        || normalized.ends_with("-listen")
        || normalized.ends_with("-bind")
        || normalized.ends_with("-address")
        || normalized.ends_with("-addr")
}

fn is_forbidden_port_text(value: &str) -> bool {
    let value = value.trim();
    if is_forbidden_port_number(value) {
        return true;
    }
    contains_forbidden_port_in_composite(value)
}

/// Parse a decimal port token without allowing a leading-zero spelling to
/// bypass the reserved-port guard.  Multica's CLI accepts decimal values, and
/// treating all equivalent spellings as the same port keeps the guard aligned
/// with the daemon rather than with the textual representation.
fn is_forbidden_port_number(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let Ok(port) = value.parse::<u32>() else {
        return false;
    };
    FORBIDDEN_PORTS.iter().any(|forbidden| {
        forbidden
            .parse::<u32>()
            .map(|candidate| candidate == port)
            .unwrap_or(false)
    })
}

/// A numeric listener token ends at punctuation/whitespace, but not in the
/// middle of an identifier.  This catches `57321/tcp` and `057321` while
/// leaving profile names such as `worker-57321` untouched.
fn is_port_token_boundary(byte: u8) -> bool {
    !byte.is_ascii_alphanumeric() && byte != b'_' && byte != b'-'
}

fn contains_forbidden_port_in_composite(value: &str) -> bool {
    let value = value.trim();
    // Catch composite listener values such as `127.0.0.1:57321`,
    // `[::1]:57331`, `:57320/path`, and `host=57321` without treating a
    // profile name like `worker-57321` as a port.  Parse the complete decimal
    // token so equivalent leading-zero spellings cannot slip through.
    let bytes = value.as_bytes();
    for (index, separator) in bytes.iter().enumerate() {
        if *separator != b':' && *separator != b'=' {
            continue;
        }
        let start = index + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == start {
            continue;
        }
        if end == bytes.len() || is_port_token_boundary(bytes[end]) {
            // SAFETY: `start` and `end` are discovered on ASCII bytes.  The
            // boundaries may still be adjacent to UTF-8, but slicing at the
            // ASCII digit run itself is valid because every digit is one byte
            // and the run starts immediately after an ASCII separator.
            if is_forbidden_port_number(&value[start..end]) {
                return true;
            }
        }
    }
    false
}

fn is_sensitive_environment_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    [
        "TOKEN",
        "API_KEY",
        "APIKEY",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "AUTH",
        "COOKIE",
        "CREDENTIAL",
        "PRIVATE_KEY",
        "ACCESS_KEY",
        "BEARER",
    ]
    .iter()
    .any(|needle| upper.contains(needle))
}

/// Launch sidecars with a tiny runtime-only environment allowlist. In
/// particular, no configured credential variable is read, renamed, or copied
/// into the child process.
fn is_sidecar_environment_name_allowed(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "PATH"
            | "PATHEXT"
            | "SYSTEMROOT"
            | "WINDIR"
            | "COMSPEC"
            | "TEMP"
            | "TMP"
            | "HOME"
            | "USERPROFILE"
            | "APPDATA"
            | "LOCALAPPDATA"
            | "LANG"
            | "TERM"
            | "NO_COLOR"
    ) || upper.starts_with("LC_")
}

fn scrub_sidecar_environment<I>(vars: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (String, String)>,
{
    vars.into_iter()
        .filter(|(name, _)| {
            is_sidecar_environment_name_allowed(name) && !is_sensitive_environment_name(name)
        })
        .collect()
}

fn sidecar_environment_entries() -> Vec<(String, String)> {
    scrub_sidecar_environment(std::env::vars())
}

fn apply_sidecar_environment(command: &mut Command) {
    command.env_clear();
    for (name, value) in sidecar_environment_entries() {
        command.env(name, value);
    }
}

fn managed_codex_runtime_environment_from(
    executable: &Path,
    inherited_path: Option<&OsStr>,
) -> anyhow::Result<(PathBuf, OsString)> {
    let executable = fs::canonicalize(executable).context("managed_codex_runtime_unavailable")?;
    if !executable.is_file() {
        bail!("managed_codex_runtime_unavailable");
    }
    let file_name = executable
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    let expected_name = if cfg!(windows) { "codex.exe" } else { "codex" };
    if !file_name.eq_ignore_ascii_case(expected_name) {
        bail!("managed_codex_runtime_untrusted");
    }
    let directory = executable
        .parent()
        .ok_or_else(|| anyhow!("managed_codex_runtime_unavailable"))?
        .to_path_buf();
    let mut entries = vec![directory.clone()];
    if let Some(inherited_path) = inherited_path {
        entries.extend(std::env::split_paths(inherited_path));
    }
    let path = std::env::join_paths(entries).context("managed_codex_runtime_path_invalid")?;
    Ok((executable, path))
}

/// Bind Multica's managed daemon to Codex Desktop's own app-server. These
/// overrides are process-local: parent environment, suppliers, proxy routing,
/// model selection and the user's Codex configuration remain untouched.
fn apply_managed_codex_runtime_environment(command: &mut Command) -> anyhow::Result<PathBuf> {
    let executable = crate::app_paths::find_codex_desktop_cli()
        .ok_or_else(|| anyhow!("managed_codex_runtime_unavailable"))?;
    let (executable, path) =
        managed_codex_runtime_environment_from(&executable, std::env::var_os("PATH").as_deref())?;
    command
        .env("MULTICA_CODEX_PATH", &executable)
        .env("MULTICA_CODEX_MULTI_AGENT", "1")
        .env("PATH", path);
    Ok(executable)
}

fn canonical_connection_key(connection: &MulticaConnectionConfig) -> String {
    let parsed = Url::parse(connection.server_url.trim());
    let url_key = parsed
        .ok()
        .map(|mut url| {
            // URL schemes and host names are case-insensitive, but path
            // segments are not.  Do not lower-case the complete URL: doing so
            // would incorrectly collapse distinct `/Base` and `/base`
            // deployments into one connection.
            let scheme = url.scheme().to_ascii_lowercase();
            let _ = url.set_scheme(&scheme);
            if let Some(host) = url.host_str().map(str::to_ascii_lowercase) {
                let _ = url.set_host(Some(&host));
            }
            if url
                .port()
                .is_some_and(|port| match url.scheme().to_ascii_lowercase().as_str() {
                    "http" => port == 80,
                    "https" => port == 443,
                    _ => false,
                })
            {
                let _ = url.set_port(None);
            }
            let path = url.path().trim_end_matches('/').to_string();
            url.set_path(if path.is_empty() { "/" } else { &path });
            url.set_query(None);
            url.set_fragment(None);
            url.to_string()
        })
        .unwrap_or_else(|| {
            connection
                .server_url
                .trim()
                .trim_end_matches('/')
                .to_string()
        });
    format!(
        "{}|{}",
        url_key,
        connection
            .api_prefix
            .as_deref()
            .unwrap_or_default()
            .trim()
            .trim_matches('/')
            .to_ascii_lowercase()
    )
}

struct SidecarProcess {
    /// Kept in memory solely to bind lifecycle diagnostics to the tracked
    /// child.  It is hashed before any log write and is never serialized.
    connection_id: String,
    child: Child,
    executable: PathBuf,
    #[cfg(windows)]
    managed_job: Option<ManagedProcessJob>,
    _managed_lifecycle: Option<ManagedLifecycleLease>,
    started_at_ms: u64,
    exited_at_ms: Option<u64>,
    exit_code: Option<i32>,
    last_health: Option<MulticaDaemonStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidecarProcessState {
    RunningOwned,
    RunningUnverified,
    RunningForeign,
    Exited(Option<i32>),
    StatusUnavailable,
}

fn sidecar_state_is_owned(state: SidecarProcessState) -> bool {
    matches!(state, SidecarProcessState::RunningOwned)
}

/// Reject edits that would invalidate the ownership/configuration contract of
/// a live sidecar. Ordinary connection metadata (display name, URL, API
/// prefix, workspace context, and token reference) remains editable while the
/// process runs because it is independent from the child command line.
fn ensure_sidecar_runtime_update_allowed(
    processes: &mut HashMap<String, SidecarProcess>,
    existing: &MulticaConnectionConfig,
    incoming: &MulticaConnectionConfig,
) -> anyhow::Result<()> {
    let sidecar_changed = existing.sidecar != incoming.sidecar;
    let enabled_changed = existing.enabled != incoming.enabled;
    if !sidecar_changed && !enabled_changed {
        return Ok(());
    }

    let Some(state) = processes
        .get_mut(&existing.connection_id)
        .map(sidecar_process_state)
    else {
        // A child from a previous manager process is deliberately not tracked
        // or inferred from persisted PID data; there is no safe local process
        // to gate here.
        return Ok(());
    };
    match state {
        SidecarProcessState::RunningOwned => {
            bail!("请先停止该 Multica sidecar，再修改运行时配置。")
        }
        SidecarProcessState::RunningForeign
        | SidecarProcessState::RunningUnverified
        | SidecarProcessState::StatusUnavailable => {
            bail!("无法验证 Multica sidecar 进程归属，已拒绝修改运行时配置。")
        }
        SidecarProcessState::Exited(_) => {
            // Reap an exited child and clear its stale record before allowing
            // a replacement command or disabled state to be persisted.
            if let Some(mut process) = processes.remove(&existing.connection_id) {
                let _ = process.child.wait();
            }
            Ok(())
        }
    }
}

fn ensure_sidecar_registration_available(
    processes: &mut HashMap<String, SidecarProcess>,
    connection_id: &str,
) -> anyhow::Result<()> {
    let Some(state) = processes.get_mut(connection_id).map(sidecar_process_state) else {
        return Ok(());
    };
    match state {
        SidecarProcessState::RunningOwned => {
            bail!("请先停止该 Multica sidecar，再复用连接 ID。")
        }
        SidecarProcessState::RunningForeign
        | SidecarProcessState::RunningUnverified
        | SidecarProcessState::StatusUnavailable => {
            bail!("无法验证 Multica sidecar 进程归属，已拒绝复用连接 ID。")
        }
        SidecarProcessState::Exited(_) => {
            if let Some(mut process) = processes.remove(connection_id) {
                let _ = process.child.wait();
            }
            Ok(())
        }
    }
}

/// Check both the child handle and the image path.  A PID alone is not a
/// sufficient ownership proof because Windows can reuse it after a crash.
fn sidecar_process_state(process: &mut SidecarProcess) -> SidecarProcessState {
    let pid = process.child.id();
    match process.child.try_wait() {
        Ok(Some(exit)) => {
            let first_observed_exit = process.exited_at_ms.is_none();
            if first_observed_exit {
                process.exited_at_ms = Some(now_ms());
            }
            process.exit_code = exit.code();
            if first_observed_exit {
                log_sidecar_lifecycle(
                    "exited",
                    &process.connection_id,
                    Some(pid),
                    Some(process.started_at_ms),
                    process.exited_at_ms,
                    process.exit_code,
                    Some("stopped"),
                    Some("sidecar_exited"),
                );
            }
            SidecarProcessState::Exited(process.exit_code)
        }
        Err(_) => SidecarProcessState::StatusUnavailable,
        Ok(None) => match query_process_executable_path(pid) {
            Some(actual) if same_executable_path(&process.executable, &actual) => {
                SidecarProcessState::RunningOwned
            }
            Some(_) => SidecarProcessState::RunningForeign,
            None => {
                // On platforms where the OS does not expose an image path,
                // the Child handle is the best available ownership proof.
                #[cfg(any(windows, unix))]
                {
                    SidecarProcessState::RunningUnverified
                }
                #[cfg(not(any(windows, unix)))]
                {
                    SidecarProcessState::RunningOwned
                }
            }
        },
    }
}

#[cfg(test)]
fn sidecar_is_running(connection_id: &str) -> bool {
    let Ok(mut processes) = sidecars().lock() else {
        return false;
    };
    let Some(process) = processes.get_mut(connection_id) else {
        return false;
    };
    matches!(
        sidecar_process_state(process),
        SidecarProcessState::RunningOwned
            | SidecarProcessState::RunningUnverified
            | SidecarProcessState::RunningForeign
            | SidecarProcessState::StatusUnavailable
    )
}

fn should_auto_start_sidecar(connection: &MulticaConnectionConfig) -> bool {
    !is_managed_connection_id(&connection.connection_id)
        && connection.enabled
        && connection
            .sidecar
            .as_ref()
            .is_some_and(|sidecar| sidecar.auto_start)
}

fn same_executable_path(expected: &Path, actual: &Path) -> bool {
    #[cfg(windows)]
    {
        expected
            .to_string_lossy()
            .eq_ignore_ascii_case(&actual.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        expected == actual
    }
}

#[cfg(windows)]
fn query_process_executable_path(process_id: u32) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    use windows::core::PWSTR;

    // Keep the call deliberately limited to process-image inspection.  Do
    // not request terminate or VM access merely to validate ownership.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };
    if handle.is_invalid() {
        return None;
    }
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            Default::default(),
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    let _ = unsafe { CloseHandle(handle) };
    result.ok()?;
    if length == 0 {
        return None;
    }
    let raw = PathBuf::from(OsString::from_wide(&buffer[..length as usize]));
    fs::canonicalize(raw).ok()
}

#[cfg(unix)]
fn query_process_executable_path(process_id: u32) -> Option<PathBuf> {
    fs::read_link(format!("/proc/{process_id}/exe"))
        .ok()
        .and_then(|path| fs::canonicalize(path).ok())
}

#[cfg(not(any(windows, unix)))]
fn query_process_executable_path(_process_id: u32) -> Option<PathBuf> {
    None
}

fn daemon_status(connection_id: &str, configured: bool) -> MulticaDaemonStatus {
    let status = daemon_status_raw(connection_id, configured);
    if is_managed_connection_id(connection_id) {
        managed_supervisor_status_overlay(status)
    } else {
        status
    }
}

/// Read only the in-memory Child registry.  The managed supervisor uses this
/// raw view to distinguish a process exit from its own terminal overlay.
fn daemon_status_raw(connection_id: &str, configured: bool) -> MulticaDaemonStatus {
    let checked_at_ms = Some(now_ms());
    let Ok(mut processes) = sidecars().lock() else {
        return MulticaDaemonStatus {
            status: "degraded".to_string(),
            checked_at_ms,
            diagnostic: Some(diagnostic("sidecar_state_unavailable")),
            ..Default::default()
        };
    };
    let Some(process) = processes.get_mut(connection_id) else {
        return MulticaDaemonStatus {
            status: if configured {
                "stopped"
            } else {
                "unconfigured"
            }
            .to_string(),
            checked_at_ms,
            ..Default::default()
        };
    };
    let pid = process.child.id();
    match sidecar_process_state(process) {
        SidecarProcessState::RunningOwned => {
            // A live process only proves that the executable has not exited.
            // Reuse the most recent independent daemon probe when available;
            // otherwise leave the state as checking until that probe runs.
            let mut status = process
                .last_health
                .clone()
                .unwrap_or_else(|| MulticaDaemonStatus {
                    status: "checking".to_string(),
                    ..Default::default()
                });
            status.pid = Some(pid);
            status.started_at_ms = Some(process.started_at_ms);
            if status.checked_at_ms.is_none() {
                status.checked_at_ms = checked_at_ms;
            }
            status
        }
        SidecarProcessState::RunningUnverified => MulticaDaemonStatus {
            status: "degraded".to_string(),
            pid: Some(pid),
            started_at_ms: Some(process.started_at_ms),
            checked_at_ms,
            diagnostic: Some(diagnostic("sidecar_pid_unverified")),
            ..Default::default()
        },
        SidecarProcessState::RunningForeign => MulticaDaemonStatus {
            status: "degraded".to_string(),
            pid: Some(pid),
            started_at_ms: Some(process.started_at_ms),
            checked_at_ms,
            diagnostic: Some(diagnostic("sidecar_pid_mismatch")),
            ..Default::default()
        },
        SidecarProcessState::Exited(exit_code) => MulticaDaemonStatus {
            status: "stopped".to_string(),
            pid: Some(pid),
            started_at_ms: Some(process.started_at_ms),
            exited_at_ms: process.exited_at_ms,
            exit_code,
            checked_at_ms,
            diagnostic: Some(diagnostic("sidecar_exited")),
            ..Default::default()
        },
        SidecarProcessState::StatusUnavailable => MulticaDaemonStatus {
            status: "degraded".to_string(),
            pid: Some(pid),
            started_at_ms: Some(process.started_at_ms),
            exited_at_ms: None,
            exit_code: None,
            checked_at_ms,
            diagnostic: Some(diagnostic("sidecar_status_failed")),
            ..Default::default()
        },
    }
}

pub fn list_connections() -> anyhow::Result<Vec<MulticaConnectionView>> {
    let connections = MulticaStore::default().load_connections()?;
    Ok(manual_connection_views(&connections))
}

fn manual_connection_views(connections: &[MulticaConnectionConfig]) -> Vec<MulticaConnectionView> {
    connections
        .iter()
        .filter(|connection| !is_managed_connection_id(&connection.connection_id))
        .map(MulticaConnectionConfig::view)
        .collect()
}

fn is_managed_connection_id(connection_id: &str) -> bool {
    connection_id.trim() == MANAGED_RUNTIME_CONNECTION_ID
}

/// Restore only sidecars explicitly opted into by the user.  Each outcome is
/// paired with its connection ID so the manager can record a safe summary;
/// failure details intentionally stay inside this process and are reduced to a
/// stable diagnostic code.
pub fn start_auto_start_sidecars() -> anyhow::Result<Vec<(String, MulticaDaemonStatus)>> {
    // The manager may request shutdown while this background operation is
    // waiting on disk I/O.  Check the admission gate before loading and again
    // before each spawn so an exit cannot start a fresh sidecar.
    if shutdown_requested() {
        return Ok(Vec::new());
    }
    let connections = MulticaStore::default().load_connections()?;
    let mut outcomes = Vec::new();
    for connection in connections.into_iter().filter(should_auto_start_sidecar) {
        if shutdown_requested() {
            break;
        }
        let connection_id = connection.connection_id;
        let status = match start_sidecar(&connection_id) {
            Ok(status) => status,
            Err(_) => MulticaDaemonStatus {
                status: "degraded".to_string(),
                checked_at_ms: Some(now_ms()),
                diagnostic: Some(diagnostic("sidecar_auto_start_failed")),
                ..Default::default()
            },
        };
        outcomes.push((connection_id, status));
    }
    Ok(outcomes)
}

pub fn save_connection(
    connection: MulticaConnectionConfig,
) -> anyhow::Result<MulticaConnectionView> {
    Ok(MulticaStore::default().save_connection(connection)?.view())
}

pub fn save_connection_input(
    input: MulticaConnectionInput,
) -> anyhow::Result<MulticaConnectionView> {
    Ok(MulticaStore::default().save_connection_input(input)?.view())
}

pub fn delete_connection(connection_id: &str) -> anyhow::Result<bool> {
    MulticaStore::default().delete_connection(connection_id)
}

pub async fn check_connection(connection_id: &str) -> anyhow::Result<MulticaConnectionStatus> {
    if is_managed_connection_id(connection_id) {
        bail!(MANAGED_CONNECTION_RESERVED_ERROR);
    }
    check_connection_with_scope(connection_id).await
}

/// Read-only health check for the fixed managed connection.  This stays
/// separate from the generic API so a caller cannot select the managed record
/// through an arbitrary connection-ID request.
pub async fn check_managed_runtime_connection() -> anyhow::Result<MulticaConnectionStatus> {
    check_connection_with_scope(MANAGED_RUNTIME_CONNECTION_ID).await
}

async fn check_connection_with_scope(
    connection_id: &str,
) -> anyhow::Result<MulticaConnectionStatus> {
    let config = find_connection(connection_id)?;
    let guard = RequestGuard::begin("health", connection_id);
    let operation = check_connection_inner(connection_id.to_string(), config.clone());
    let timed = tokio::time::timeout(
        HEALTH_TOTAL_TIMEOUT,
        guard.run(operation, "health_request_superseded"),
    )
    .await;
    let current = guard.is_current();
    let result = match timed {
        Ok(result) => result,
        Err(_) if current => Ok(MulticaConnectionStatus {
            connection_id: connection_id.to_string(),
            server: MulticaHealthStatus {
                status: "unreachable".to_string(),
                checked_at_ms: Some(now_ms()),
                diagnostic: Some(diagnostic("health_total_timeout")),
                ..Default::default()
            },
            daemon: daemon_status(connection_id, config.sidecar.is_some()),
        }),
        Err(_) => Err(anyhow!("health_request_superseded")),
    };
    guard.finish();
    if !current {
        return Err(anyhow!("health_request_superseded"));
    }
    result
}

async fn check_connection_inner(
    connection_id: String,
    config: MulticaConnectionConfig,
) -> MulticaConnectionStatus {
    if !config.enabled {
        let daemon = daemon_status(&connection_id, config.sidecar.is_some());
        return MulticaConnectionStatus {
            connection_id: connection_id.clone(),
            server: MulticaHealthStatus {
                status: "unconfigured".to_string(),
                checked_at_ms: Some(now_ms()),
                diagnostic: Some(diagnostic("connection_disabled")),
                ..Default::default()
            },
            daemon,
        };
    }
    let (server, daemon) = tokio::join!(
        probe_server(&config),
        probe_daemon_for_connection(&connection_id, &config)
    );
    MulticaConnectionStatus {
        connection_id,
        server,
        daemon,
    }
}

pub async fn get_snapshot(connection_id: &str) -> anyhow::Result<MulticaRuntimeSnapshot> {
    let config = find_connection(connection_id)?;
    let guard = RequestGuard::begin("snapshot", connection_id);
    let store = MulticaStore::default();
    let operation = get_snapshot_inner(
        connection_id.to_string(),
        config.clone(),
        store.clone(),
        guard.clone(),
    );
    let timed = tokio::time::timeout(
        SNAPSHOT_TOTAL_TIMEOUT,
        guard.run(operation, "snapshot_request_superseded"),
    )
    .await;
    let current = guard.is_current();
    let result = match timed {
        Ok(result) => result.and_then(|snapshot| snapshot),
        Err(_) if current => {
            let previous = store.load_snapshot(connection_id)?;
            if let Some(previous) = previous {
                let snapshot = stale_snapshot(connection_id, "snapshot_timeout", Some(previous));
                if guard.is_current() {
                    // Best effort: the timeout result remains useful even if
                    // a local cache write is unavailable.
                    let _ = store.save_snapshot_if_current(&snapshot, &guard);
                }
                Ok(snapshot)
            } else {
                Err(anyhow!("snapshot_timeout"))
            }
        }
        Err(_) => Err(anyhow!("snapshot_request_superseded")),
    };
    guard.finish();
    if !current {
        return Err(anyhow!("snapshot_request_superseded"));
    }
    result
}

async fn get_snapshot_inner(
    connection_id: String,
    config: MulticaConnectionConfig,
    store: MulticaStore,
    guard: RequestGuard,
) -> anyhow::Result<MulticaRuntimeSnapshot> {
    let _permit: OwnedSemaphorePermit = guard
        .run(
            snapshot_semaphore().acquire_owned(),
            "snapshot_request_superseded",
        )
        .await??;
    let previous = store.load_snapshot(&connection_id)?;
    if !config.enabled {
        let snapshot = stale_snapshot(&connection_id, "connection_disabled", previous);
        if guard.is_current() {
            // Preserve the stale marker across an application restart.  A
            // write failure must not turn a successful read of the old cache
            // into a hard refresh error.
            let _ = store.save_snapshot_if_current(&snapshot, &guard);
        }
        return Ok(snapshot);
    }

    let client = build_client()?;
    let mut diagnostic_codes = Vec::new();
    let runtimes =
        fetch_collection(&client, &config, &["api/runtimes"], &mut diagnostic_codes).await;
    let agents = fetch_collection(&client, &config, &["api/agents"], &mut diagnostic_codes).await;
    let tasks = fetch_collection(
        &client,
        &config,
        &["api/agent-task-snapshot"],
        &mut diagnostic_codes,
    )
    .await;
    let stale = runtimes.is_err() || agents.is_err() || tasks.is_err();
    if diagnostic_codes.is_empty() && stale {
        diagnostic_codes.push("snapshot_unreachable".to_string());
    }
    // A failed first refresh has no trustworthy data to expose.  Return the
    // bounded diagnostic to the caller and leave the cache untouched so the
    // UI can render an error/empty state instead of a synthetic stale list.
    if stale && previous.is_none() {
        return Err(anyhow!(diagnostic_codes.join(",")));
    }
    let fetched_at_ms = if stale {
        previous
            .as_ref()
            .map(|snapshot| snapshot.fetched_at_ms)
            .filter(|timestamp| *timestamp > 0)
            .unwrap_or_else(now_ms)
    } else {
        now_ms()
    };
    let snapshot = MulticaRuntimeSnapshot {
        source_connection_id: connection_id.clone(),
        fetched_at_ms,
        stale,
        // Keep the last known collection when one endpoint fails.  A partial
        // outage must never present an empty list as a fresh authoritative
        // result.
        runtimes: runtimes.unwrap_or_else(|_| {
            previous
                .as_ref()
                .map(|snapshot| snapshot.runtimes.clone())
                .unwrap_or_default()
        }),
        agents: agents.unwrap_or_else(|_| {
            previous
                .as_ref()
                .map(|snapshot| snapshot.agents.clone())
                .unwrap_or_default()
        }),
        tasks: tasks.unwrap_or_else(|_| {
            previous
                .as_ref()
                .map(|snapshot| snapshot.tasks.clone())
                .unwrap_or_default()
        }),
        diagnostic: (!diagnostic_codes.is_empty()).then(|| diagnostic_codes.join(",")),
    };
    if !guard.is_current() {
        return Err(anyhow!("snapshot_request_superseded"));
    }
    if !store.save_snapshot_if_current(&snapshot, &guard)? {
        return Err(anyhow!("snapshot_request_superseded"));
    }
    Ok(snapshot)
}

fn stale_snapshot(
    connection_id: &str,
    reason: &str,
    previous: Option<MulticaRuntimeSnapshot>,
) -> MulticaRuntimeSnapshot {
    let mut snapshot = previous.unwrap_or_else(|| MulticaRuntimeSnapshot {
        source_connection_id: connection_id.to_string(),
        ..Default::default()
    });
    snapshot.source_connection_id = connection_id.to_string();
    snapshot.stale = true;
    snapshot.diagnostic = Some(diagnostic(reason));
    snapshot
}

fn managed_stopped_status() -> MulticaDaemonStatus {
    MulticaDaemonStatus {
        status: "stopped".to_string(),
        checked_at_ms: Some(now_ms()),
        ..Default::default()
    }
}

fn managed_start_failure_status() -> MulticaDaemonStatus {
    MulticaDaemonStatus {
        status: "degraded".to_string(),
        checked_at_ms: Some(now_ms()),
        diagnostic: Some(diagnostic("managed_runtime_start_failed")),
        ..Default::default()
    }
}

/// Start exactly the one pinned managed Runtime command.  This is private so
/// the supervisor cannot accidentally inherit the generic sidecar API.
fn start_managed_runtime_sidecar() -> anyhow::Result<MulticaDaemonStatus> {
    let config = find_connection(MANAGED_RUNTIME_CONNECTION_ID)?;
    validate_managed_runtime_sidecar(&config)?;
    start_sidecar_with_scope(MANAGED_RUNTIME_CONNECTION_ID, true)
}

#[cfg(windows)]
fn cleanup_failed_managed_start(child: &mut Child, job: Option<&ManagedProcessJob>) {
    if job.is_none_or(|job| job.terminate().is_err()) {
        let _ = child.kill();
    }
    let _ = wait_for_sidecar_exit(child, SIDECAR_STOP_WAIT_TIMEOUT);
}

#[cfg(not(windows))]
fn cleanup_failed_managed_start(child: &mut Child) {
    if terminate_sidecar_process_tree(child.id()).is_err() {
        let _ = child.kill();
    }
    let _ = wait_for_sidecar_exit(child, SIDECAR_STOP_WAIT_TIMEOUT);
}

/// Explicit managed start resets a previous crash budget and creates a fresh
/// supervision generation.  It intentionally accepts no executable, profile,
/// connection ID, arguments, URL, or environment from its caller.
pub fn start_managed_runtime() -> anyhow::Result<MulticaDaemonStatus> {
    let generation = begin_managed_supervision();
    match start_managed_runtime_sidecar() {
        Ok(mut status) => {
            if managed_status_requires_rollback(&status) {
                match try_managed_runtime_automatic_rollback(generation) {
                    Ok(Some(recovered)) => status = recovered,
                    // With no verified previous version, keep the observed
                    // child under supervision. The worker applies the same
                    // bounded recovery budget used for later health loss.
                    Ok(None) => {}
                    Err(error) => {
                        let _ = set_managed_supervisor_terminal(
                            generation,
                            managed_start_failure_status(),
                        );
                        return Err(error);
                    }
                }
            }
            if let Err(error) = start_managed_supervisor_worker(generation) {
                let _ = set_managed_supervisor_terminal(generation, managed_start_failure_status());
                return Err(error);
            }
            Ok(status)
        }
        Err(error) => {
            if managed_start_error_requires_rollback(&error)
                && let Ok(Some(recovered)) = try_managed_runtime_automatic_rollback(generation)
            {
                if let Err(supervisor_error) = start_managed_supervisor_worker(generation) {
                    let _ =
                        set_managed_supervisor_terminal(generation, managed_start_failure_status());
                    return Err(supervisor_error);
                }
                return Ok(recovered);
            }
            let _ = set_managed_supervisor_terminal(generation, managed_start_failure_status());
            Err(error)
        }
    }
}

/// Start the managed Runtime only when all persisted managed lifecycle gates
/// are enabled.  This is intentionally distinct from `start_auto_start_sidecars`,
/// which handles user-configured sidecars and always excludes the reserved ID.
pub fn start_managed_runtime_supervision_if_enabled() -> anyhow::Result<MulticaDaemonStatus> {
    let connection = match managed_connection() {
        Ok(connection) => connection,
        Err(_) => ensure_managed_connection()?,
    };
    if !managed_connection_is_supervision_eligible(&connection) {
        let status = managed_stopped_status();
        invalidate_managed_supervisor(true, Some(status.clone()));
        return Ok(status);
    }
    start_managed_runtime()
}

/// Explicit managed stop invalidates the current worker before looking at the
/// process table.  Consequently a worker sleeping in backoff cannot resurrect
/// a daemon after the user has pressed Stop.
pub fn stop_managed_runtime() -> anyhow::Result<MulticaDaemonStatus> {
    let stopped = managed_stopped_status();
    invalidate_managed_supervisor(true, Some(stopped));
    stop_sidecar_with_scope(MANAGED_RUNTIME_CONNECTION_ID, true)
}

/// Manual restart is a new user action, not a retry within an old crash loop.
/// It therefore cancels the old worker/attempt counter before stop/start.
pub fn restart_managed_runtime() -> anyhow::Result<MulticaDaemonStatus> {
    let _ = stop_managed_runtime()?;
    start_managed_runtime()
}

/// Managed-only status path used by the Tauri Runtime panel.  Generic status
/// calls never need to know about supervisor generations or retry budgets.
pub fn managed_daemon_status() -> anyhow::Result<MulticaDaemonStatus> {
    let config = managed_connection()?;
    Ok(daemon_status(
        MANAGED_RUNTIME_CONNECTION_ID,
        config.sidecar.is_some(),
    ))
}

/// Start a user-configured sidecar.  The reserved managed connection has a
/// separate API so callers cannot supply a mutable command configuration for
/// the pinned Runtime.
pub fn start_sidecar(connection_id: &str) -> anyhow::Result<MulticaDaemonStatus> {
    if is_managed_connection_id(connection_id) {
        bail!(MANAGED_CONNECTION_RESERVED_ERROR);
    }
    start_sidecar_with_scope(connection_id, false)
}

fn start_sidecar_with_scope(
    connection_id: &str,
    allow_managed_connection: bool,
) -> anyhow::Result<MulticaDaemonStatus> {
    if is_managed_connection_id(connection_id) && !allow_managed_connection {
        bail!(MANAGED_CONNECTION_RESERVED_ERROR);
    }
    if shutdown_requested() {
        bail!("应用正在退出，已拒绝启动 Multica sidecar。")
    }
    // Fail fast for an unknown ID, then re-read the complete configuration
    // under the registry lock below so deletion cannot race this start.
    let _ = find_connection(connection_id)?;
    // Keep the registry lock through the short spawn/insert window.  Besides
    // coordinating shutdown, this serializes ordinary start/stop requests so
    // a user clicking Stop cannot observe an empty map just before a racing
    // Start inserts a child.
    let mut processes = sidecars()
        .lock()
        .map_err(|_| anyhow!("sidecar 状态锁不可用。"))?;
    if shutdown_requested() {
        bail!("应用正在退出，已拒绝启动 Multica sidecar。")
    }
    // The initial lookup above can race a concurrent connection deletion while
    // this request waits for the registry lock. Re-read the persisted record
    // after acquiring the lock; otherwise a stale config could spawn a child
    // after its connection had already been deleted.
    let config = find_connection(connection_id)?;
    if is_managed_connection_id(connection_id) {
        validate_managed_runtime_sidecar(&config)?;
    }
    let sidecar = sidecar_config_for_start(&config)?;
    // The health listener is profile-scoped. Validate this at the final start
    // boundary so a persisted sidecar is never launched without an explicit
    // identity that can later be verified by the health response.
    validate_sidecar_config(sidecar)?;
    validated_sidecar_profile(&sidecar.args)?;
    let executable = verified_sidecar_executable_path(&sidecar.executable)?;
    let already_running = if let Some(process) = processes.get_mut(connection_id) {
        match sidecar_process_state(process) {
            SidecarProcessState::RunningOwned => true,
            SidecarProcessState::RunningForeign
            | SidecarProcessState::RunningUnverified
            | SidecarProcessState::StatusUnavailable => {
                bail!("无法验证 Multica sidecar 进程归属，已拒绝重复启动。")
            }
            SidecarProcessState::Exited(_) => {
                // Reap a naturally exited child before replacing the record.
                // This avoids a zombie on Unix and keeps the recorded exit
                // code available until this next start.
                let _ = process.child.wait();
                processes.remove(connection_id);
                false
            }
        }
    } else {
        false
    };
    if already_running {
        let (pid, started_at_ms) = processes
            .get(connection_id)
            .map(|process| (Some(process.child.id()), Some(process.started_at_ms)))
            .unwrap_or((None, None));
        drop(processes);
        let status =
            probe_daemon_for_connection_blocking(connection_id.to_string(), config, false)?;
        log_sidecar_health("start_reused", connection_id, pid, started_at_ms, &status);
        return Ok(status);
    }

    let managed_lifecycle_lock = if is_managed_connection_id(connection_id) {
        Some(acquire_managed_lifecycle_lock()?)
    } else {
        None
    };

    let mut command = Command::new(&executable);
    command
        .args(&sidecar.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    apply_sidecar_environment(&mut command);
    if is_managed_connection_id(connection_id) {
        // `apply_sidecar_environment` clears inherited variables first. Only
        // this managed child receives the exact validated connection URL and
        // the verified Codex Desktop app-server binding.
        command.env("MULTICA_SERVER_URL", &config.server_url);
        apply_managed_codex_runtime_environment(&mut command)?;
    }
    if let Some(working_dir) = sidecar
        .working_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let working_dir = verified_sidecar_working_dir(working_dir)?;
        command.current_dir(working_dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Put the sidecar in its own process group so stop can terminate its
        // descendants without touching unrelated CCP or GUI processes.
        command.process_group(0);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            log_sidecar_lifecycle(
                "start_failed",
                connection_id,
                None,
                None,
                None,
                None,
                Some("degraded"),
                Some("sidecar_spawn_failed"),
            );
            let error = anyhow::Error::new(error);
            let error = if is_managed_connection_id(connection_id) {
                error.context("managed_runtime_spawn_failed")
            } else {
                error
            };
            return Err(error).context("Multica sidecar 启动失败。");
        }
    };
    #[cfg(windows)]
    let managed_job = if is_managed_connection_id(connection_id) {
        match ManagedProcessJob::assign(&child) {
            Ok(job) => Some(job),
            Err(error) => {
                cleanup_failed_managed_start(&mut child, None);
                log_sidecar_lifecycle(
                    "start_failed",
                    connection_id,
                    Some(child.id()),
                    None,
                    Some(now_ms()),
                    None,
                    Some("degraded"),
                    Some("managed_runtime_job_assign_failed"),
                );
                return Err(error);
            }
        }
    } else {
        None
    };
    let pid = child.id();
    let started_at_ms = now_ms();
    let managed_lifecycle = if let Some(lock) = managed_lifecycle_lock {
        let activation = managed_owner_for_process(&executable, pid, started_at_ms)
            .and_then(|owner| lock.activate(owner));
        match activation {
            Ok(lease) => Some(lease),
            Err(error) => {
                #[cfg(windows)]
                cleanup_failed_managed_start(&mut child, managed_job.as_ref());
                #[cfg(not(windows))]
                cleanup_failed_managed_start(&mut child);
                log_sidecar_lifecycle(
                    "start_failed",
                    connection_id,
                    Some(pid),
                    Some(started_at_ms),
                    Some(now_ms()),
                    None,
                    Some("degraded"),
                    Some("managed_runtime_owner_write_failed"),
                );
                return Err(error);
            }
        }
    } else {
        None
    };
    processes.insert(
        connection_id.to_string(),
        SidecarProcess {
            connection_id: connection_id.to_string(),
            child,
            executable,
            #[cfg(windows)]
            managed_job,
            _managed_lifecycle: managed_lifecycle,
            started_at_ms,
            exited_at_ms: None,
            exit_code: None,
            last_health: None,
        },
    );
    drop(processes);

    log_sidecar_lifecycle(
        "started",
        connection_id,
        Some(pid),
        Some(started_at_ms),
        None,
        None,
        Some("checking"),
        None,
    );

    let status = probe_daemon_for_connection_blocking(connection_id.to_string(), config, true)?;
    log_sidecar_health(
        "health_checked",
        connection_id,
        Some(pid),
        Some(started_at_ms),
        &status,
    );
    Ok(status)
}

/// `start_sidecar` is synchronous for the IPC boundary, but daemon health is
/// asynchronous. Run the bounded probe on a short-lived runtime after the
/// child has been recorded and the sidecar lock is released. This returns an
/// observed health state instead of reporting a successful spawn as healthy.
fn probe_daemon_for_connection_blocking(
    connection_id: String,
    config: MulticaConnectionConfig,
    retry_after_start: bool,
) -> anyhow::Result<MulticaDaemonStatus> {
    std::thread::Builder::new()
        .name("ccp-multica-health-probe".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .map_err(|_| anyhow!("Multica daemon health runtime 初始化失败。"))?;
            Ok::<_, anyhow::Error>(if retry_after_start {
                runtime.block_on(probe_daemon_after_start(&connection_id, &config))
            } else {
                runtime.block_on(probe_daemon_for_connection(&connection_id, &config))
            })
        })
        .map_err(|_| anyhow!("Multica daemon health probe 启动失败。"))?
        .join()
        .map_err(|_| anyhow!("Multica daemon health probe 异常退出。"))?
}

fn sidecar_config_for_start(
    config: &MulticaConnectionConfig,
) -> anyhow::Result<&MulticaSidecarConfig> {
    if !config.enabled {
        bail!("Multica 连接已停用，不能启动 sidecar。")
    }
    config
        .sidecar
        .as_ref()
        .ok_or_else(|| anyhow!("该连接未配置 Multica sidecar。"))
}

#[cfg(windows)]
fn terminate_sidecar_process_tree(process_id: u32) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;

    let mut command = Command::new("taskkill.exe");
    command
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);
    let status = command
        .status()
        .context("Multica sidecar 进程树停止失败。")?;
    if status.success() {
        Ok(())
    } else {
        bail!("Multica sidecar 进程树停止失败。")
    }
}

#[cfg(unix)]
fn terminate_sidecar_process_tree(process_id: u32) -> anyhow::Result<()> {
    // `start_sidecar` places the child in a new process group whose id is the
    // child's PID.  Sending SIGKILL to the negative group id therefore reaps
    // descendants as well as the direct child, without matching unrelated
    // processes by executable name.  ESRCH means the group already exited and
    // is treated as a successful stop.
    if process_id < 2 {
        bail!("Multica sidecar PID 无效。")
    }
    let process_id =
        i32::try_from(process_id).map_err(|_| anyhow!("Multica sidecar PID 无效。"))?;
    unsafe extern "C" {
        fn kill(process: i32, signal: i32) -> i32;
    }
    const SIGKILL: i32 = 9;
    const ESRCH: i32 = 3;
    let result = unsafe { kill(-process_id, SIGKILL) };
    if result == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(ESRCH) {
        return Ok(());
    }
    Err(anyhow!("Multica sidecar 进程树停止失败。"))
}

#[cfg(not(any(windows, unix)))]
fn terminate_sidecar_process_tree(_process_id: u32) -> anyhow::Result<()> {
    // There is no portable process-group primitive on the remaining targets;
    // the caller will fall back to the verified Child handle.
    bail!("Multica sidecar 进程树停止失败。")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidecarWaitResult {
    Exited(Option<i32>),
    TimedOut,
    Unavailable,
}

/// Wait for a child without ever calling the unbounded `Child::wait`. A
/// failed tree kill must not leave the IPC command hung forever, and a timeout
/// deliberately leaves the process record in the registry so a later status
/// check or retry can still prove ownership.
fn wait_for_sidecar_exit(child: &mut Child, timeout: Duration) -> SidecarWaitResult {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return SidecarWaitResult::Exited(status.code()),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(SIDECAR_STOP_POLL_INTERVAL);
            }
            Ok(None) => return SidecarWaitResult::TimedOut,
            Err(_) => return SidecarWaitResult::Unavailable,
        }
    }
}

/// Stop a user-configured sidecar.  Managed shutdown is deliberately routed
/// through `stop_managed_runtime` so it first invalidates its supervisor.
pub fn stop_sidecar(connection_id: &str) -> anyhow::Result<MulticaDaemonStatus> {
    if is_managed_connection_id(connection_id) {
        bail!(MANAGED_CONNECTION_RESERVED_ERROR);
    }
    stop_sidecar_with_scope(connection_id, false)
}

fn stop_sidecar_with_scope(
    connection_id: &str,
    allow_managed_connection: bool,
) -> anyhow::Result<MulticaDaemonStatus> {
    if is_managed_connection_id(connection_id) && !allow_managed_connection {
        bail!(MANAGED_CONNECTION_RESERVED_ERROR);
    }
    let mut processes = sidecars()
        .lock()
        .map_err(|_| anyhow!("sidecar 状态锁不可用。"))?;
    let Some(process) = processes.get_mut(connection_id) else {
        let status = MulticaDaemonStatus {
            status: "stopped".to_string(),
            checked_at_ms: Some(now_ms()),
            ..Default::default()
        };
        let diagnostic_code = Some("sidecar_not_tracked");
        log_sidecar_lifecycle(
            "stop_noop",
            connection_id,
            None,
            None,
            None,
            None,
            Some(&status.status),
            diagnostic_code,
        );
        return Ok(status);
    };
    let pid = process.child.id();
    let started_at_ms = process.started_at_ms;
    let state = sidecar_process_state(process);
    if matches!(
        state,
        SidecarProcessState::RunningForeign
            | SidecarProcessState::RunningUnverified
            | SidecarProcessState::StatusUnavailable
    ) {
        // Put the handle back so a later status check can diagnose the
        // condition.  Most importantly, never kill a PID whose image path
        // does not match the executable that CCP started.
        let refusal_code = match state {
            SidecarProcessState::RunningForeign => "sidecar_pid_mismatch",
            SidecarProcessState::RunningUnverified => "sidecar_pid_unverified",
            SidecarProcessState::StatusUnavailable => "sidecar_status_failed",
            _ => "sidecar_stop_refused",
        };
        log_sidecar_lifecycle(
            "stop_rejected",
            connection_id,
            Some(pid),
            Some(started_at_ms),
            process.exited_at_ms,
            process.exit_code,
            Some("degraded"),
            Some(refusal_code),
        );
        bail!("无法验证 Multica sidecar 进程归属，已拒绝停止。")
    }
    if let SidecarProcessState::Exited(exit_code) = state {
        let exited_at_ms = process.exited_at_ms.or_else(|| Some(now_ms()));
        let status = MulticaDaemonStatus {
            status: "stopped".to_string(),
            pid: Some(pid),
            started_at_ms: Some(started_at_ms),
            exited_at_ms,
            exit_code,
            checked_at_ms: Some(now_ms()),
            diagnostic: Some(diagnostic("sidecar_exited")),
            ..Default::default()
        };
        // `try_wait` above already reaped the child. Remove the record only
        // after the terminal state has been observed and logged.
        processes.remove(connection_id);
        log_sidecar_lifecycle(
            "stop_observed_exit",
            connection_id,
            Some(pid),
            Some(started_at_ms),
            status.exited_at_ms,
            status.exit_code,
            Some(&status.status),
            status.diagnostic.as_deref(),
        );
        return Ok(status);
    }

    // Re-check the child image immediately before issuing a tree/direct kill.
    // A process can exit and its PID can be reused between the first status
    // read above and this point; acting on a stale PID would violate the
    // no-mis-kill contract even though the original ownership check passed.
    let revalidated_state = sidecar_process_state(process);
    if !sidecar_state_is_owned(revalidated_state) {
        if let SidecarProcessState::Exited(exit_code) = revalidated_state {
            let exited_at_ms = process.exited_at_ms.or_else(|| Some(now_ms()));
            let status = MulticaDaemonStatus {
                status: "stopped".to_string(),
                pid: Some(pid),
                started_at_ms: Some(started_at_ms),
                exited_at_ms,
                exit_code,
                checked_at_ms: Some(now_ms()),
                diagnostic: Some(diagnostic("sidecar_exited")),
                ..Default::default()
            };
            processes.remove(connection_id);
            log_sidecar_lifecycle(
                "stop_observed_exit",
                connection_id,
                Some(pid),
                Some(started_at_ms),
                status.exited_at_ms,
                status.exit_code,
                Some(&status.status),
                status.diagnostic.as_deref(),
            );
            return Ok(status);
        }
        let refusal_code = match revalidated_state {
            SidecarProcessState::RunningForeign => "sidecar_pid_mismatch",
            SidecarProcessState::RunningUnverified => "sidecar_pid_unverified",
            SidecarProcessState::StatusUnavailable => "sidecar_status_failed",
            SidecarProcessState::RunningOwned | SidecarProcessState::Exited(_) => {
                "sidecar_stop_refused"
            }
        };
        log_sidecar_lifecycle(
            "stop_rejected",
            connection_id,
            Some(pid),
            Some(started_at_ms),
            process.exited_at_ms,
            process.exit_code,
            Some("degraded"),
            Some(refusal_code),
        );
        bail!("无法在停止前验证 Multica sidecar 进程归属。")
    }

    let mut stop_diagnostic = None;
    #[cfg(windows)]
    let tree_stop_failed = if is_managed_connection_id(connection_id) {
        process
            .managed_job
            .as_ref()
            .ok_or_else(|| anyhow!("managed_runtime_job_missing"))
            .and_then(ManagedProcessJob::terminate)
            .is_err()
    } else {
        terminate_sidecar_process_tree(pid).is_err()
    };
    #[cfg(not(windows))]
    let tree_stop_failed = terminate_sidecar_process_tree(pid).is_err();

    // The image path was verified above, so a direct handle kill is a safe
    // fallback when the platform tree primitive is unavailable. We also use
    // it after a tree failure, while retaining the record if the child does
    // not exit within the bounded wait below.
    if tree_stop_failed && process.child.kill().is_err() {
        stop_diagnostic = Some(diagnostic("sidecar_tree_stop_failed"));
    }
    let wait_result = wait_for_sidecar_exit(&mut process.child, SIDECAR_STOP_WAIT_TIMEOUT);
    let (exit_code, exited_at_ms) = match wait_result {
        SidecarWaitResult::Exited(exit_code) => (exit_code, Some(now_ms())),
        SidecarWaitResult::TimedOut => {
            // A final verified direct-handle attempt covers platforms where a
            // process-group command returned before the direct child exited.
            let _ = process.child.kill();
            match wait_for_sidecar_exit(&mut process.child, Duration::from_millis(250)) {
                SidecarWaitResult::Exited(exit_code) => (exit_code, Some(now_ms())),
                SidecarWaitResult::TimedOut => {
                    stop_diagnostic = Some(diagnostic("sidecar_stop_timeout"));
                    (None, None)
                }
                SidecarWaitResult::Unavailable => {
                    stop_diagnostic = Some(diagnostic("sidecar_wait_failed"));
                    (None, None)
                }
            }
        }
        SidecarWaitResult::Unavailable => {
            stop_diagnostic = Some(diagnostic("sidecar_wait_failed"));
            (None, None)
        }
    };

    if exited_at_ms.is_none() {
        let status = MulticaDaemonStatus {
            status: "degraded".to_string(),
            pid: Some(pid),
            started_at_ms: Some(started_at_ms),
            exited_at_ms: None,
            exit_code,
            checked_at_ms: Some(now_ms()),
            diagnostic: stop_diagnostic
                .clone()
                .or_else(|| Some(diagnostic("sidecar_stop_failed"))),
            ..Default::default()
        };
        log_sidecar_lifecycle(
            "stop_failed",
            connection_id,
            Some(pid),
            Some(started_at_ms),
            None,
            exit_code,
            Some(&status.status),
            status.diagnostic.as_deref(),
        );
        // Keep `process` in `processes`: it is still owned and can be retried
        // or diagnosed. Dropping it here would orphan a live child.
        return Ok(status);
    }

    let status = MulticaDaemonStatus {
        status: if stop_diagnostic.is_some() {
            "degraded".to_string()
        } else {
            "stopped".to_string()
        },
        pid: Some(pid),
        started_at_ms: Some(started_at_ms),
        exited_at_ms,
        exit_code,
        checked_at_ms: Some(now_ms()),
        diagnostic: stop_diagnostic,
        ..Default::default()
    };
    processes.remove(connection_id);
    log_sidecar_lifecycle(
        "stopped",
        connection_id,
        Some(pid),
        Some(started_at_ms),
        status.exited_at_ms,
        status.exit_code,
        Some(&status.status),
        status.diagnostic.as_deref(),
    );
    Ok(status)
}

/// Stop every sidecar that this process is currently tracking.
///
/// Shutdown must not reconstruct a target from persisted PIDs or search by a
/// generic process name. We first snapshot the in-memory connection IDs and
/// then reuse `stop_sidecar`, which verifies the child image path immediately
/// before terminating it. A foreign or unverified child is left in the map and
/// represented by a bounded degraded status so one bad record cannot prevent
/// the remaining owned sidecars from being cleaned up.
pub fn stop_all_sidecars() -> anyhow::Result<Vec<(String, MulticaDaemonStatus)>> {
    let connection_ids = sidecars()
        .lock()
        .map_err(|_| anyhow!("sidecar 状态锁不可用。"))?
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    let mut outcomes = Vec::with_capacity(connection_ids.len());
    for connection_id in connection_ids {
        let status = match stop_sidecar_with_scope(&connection_id, true) {
            Ok(status) => status,
            Err(_) => sidecar_stop_refused_status(&connection_id),
        };
        outcomes.push((connection_id, status));
    }
    Ok(outcomes)
}

/// Convert a refused stop into a stable, non-sensitive status for shutdown
/// diagnostics. The tracked process is deliberately retained by
/// `stop_sidecar` so a later status check can explain why ownership could not
/// be proven.
fn sidecar_stop_refused_status(connection_id: &str) -> MulticaDaemonStatus {
    let mut status = MulticaDaemonStatus {
        status: "degraded".to_string(),
        checked_at_ms: Some(now_ms()),
        diagnostic: Some(diagnostic("sidecar_stop_refused")),
        ..Default::default()
    };
    let Ok(mut processes) = sidecars().lock() else {
        return status;
    };
    let Some(process) = processes.get_mut(connection_id) else {
        return status;
    };
    status.pid = Some(process.child.id());
    status.started_at_ms = Some(process.started_at_ms);
    status.diagnostic = Some(match sidecar_process_state(process) {
        SidecarProcessState::RunningForeign => diagnostic("sidecar_pid_mismatch"),
        SidecarProcessState::RunningUnverified => diagnostic("sidecar_pid_unverified"),
        SidecarProcessState::StatusUnavailable => diagnostic("sidecar_status_failed"),
        SidecarProcessState::Exited(exit_code) => {
            status.status = "stopped".to_string();
            status.exit_code = exit_code;
            diagnostic("sidecar_exited")
        }
        SidecarProcessState::RunningOwned => diagnostic("sidecar_stop_failed"),
    });
    status
}

pub fn restart_sidecar(connection_id: &str) -> anyhow::Result<MulticaDaemonStatus> {
    if is_managed_connection_id(connection_id) {
        bail!(MANAGED_CONNECTION_RESERVED_ERROR);
    }
    let _ = stop_sidecar(connection_id)?;
    start_sidecar(connection_id)
}

pub fn daemon_status_for_connection(connection_id: &str) -> anyhow::Result<MulticaDaemonStatus> {
    let config = find_connection(connection_id)?;
    Ok(daemon_status(connection_id, config.sidecar.is_some()))
}

fn find_connection(connection_id: &str) -> anyhow::Result<MulticaConnectionConfig> {
    MulticaStore::default()
        .load_connections()?
        .into_iter()
        .find(|connection| connection.connection_id == connection_id)
        .ok_or_else(|| anyhow!("未找到 Multica 连接。"))
}

fn build_client() -> anyhow::Result<Client> {
    Ok(Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}

fn endpoint_url(config: &MulticaConnectionConfig, endpoint: &str) -> anyhow::Result<Url> {
    let mut url = Url::parse(config.server_url.trim())?;
    let endpoint_without_suffix = endpoint
        .split(|character| character == '?' || character == '#')
        .next()
        .unwrap_or_default();
    let endpoint_has_trailing_slash = endpoint_without_suffix.ends_with('/');
    let mut segments = Vec::new();
    append_path_component(&mut segments, url.path());
    append_path_component(
        &mut segments,
        config.api_prefix.as_deref().unwrap_or_default(),
    );
    append_path_component(&mut segments, endpoint_without_suffix);
    let mut path = if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    };
    if endpoint_has_trailing_slash && !path.ends_with('/') {
        path.push('/');
    }
    // `Url::set_path` accepts an empty path, but all Multica endpoints are
    // rooted. Keep the root stable when the caller supplies an empty endpoint.
    if path.is_empty() {
        path.push('/');
    }
    url.set_path(&path);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

/// Construct a bounded read-only collection request URL.  Multica currently
/// serves these list routes as non-paginated arrays, so the adapter enforces a
/// local ceiling as well as sending a fixed `limit` hint for deployments that
/// implement the common query contract.  The base URL's query and fragment
/// are discarded by `endpoint_url`; this function never writes them back.
fn collection_endpoint_url(
    config: &MulticaConnectionConfig,
    endpoint: &str,
) -> anyhow::Result<Url> {
    let mut url = endpoint_url(config, endpoint)?;
    url.query_pairs_mut()
        .append_pair("limit", &MAX_COLLECTION_ITEMS.to_string());
    Ok(url)
}

/// Append one path component while removing only the overlap at the boundary
/// with the path already accumulated.  Internal repeated segments are kept;
/// this prevents a configured `/v1` base plus `v1` prefix from becoming
/// `/v1/v1` without rewriting the user's saved URL.
fn append_path_component(segments: &mut Vec<String>, component: &str) {
    let incoming = component
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if incoming.is_empty() {
        return;
    }
    let max_overlap = segments.len().min(incoming.len());
    let overlap = (1..=max_overlap)
        .rev()
        .find(|length| {
            segments[segments.len() - length..]
                .iter()
                .map(String::as_str)
                .eq(incoming[..*length].iter().copied())
        })
        .unwrap_or(0);
    segments.extend(
        incoming[overlap..]
            .iter()
            .map(|segment| (*segment).to_string()),
    );
}

fn is_loopback_host(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<std::net::IpAddr>()
        .map(|address| address.is_loopback())
        .unwrap_or(false)
}

fn apply_token(
    request: reqwest::RequestBuilder,
    config: &MulticaConnectionConfig,
) -> reqwest::RequestBuilder {
    let token = config
        .token_env_var
        .as_deref()
        .and_then(|name| std::env::var(name).ok())
        .filter(|token| !token.trim().is_empty());
    match token {
        Some(token) => request.bearer_auth(token),
        None => request,
    }
}

fn apply_workspace_headers(
    mut request: reqwest::RequestBuilder,
    config: &MulticaConnectionConfig,
) -> reqwest::RequestBuilder {
    if let Some(value) = config
        .workspace_id
        .as_deref()
        .and_then(|value| HeaderValue::from_str(value).ok())
    {
        request = request.header("X-Workspace-ID", value);
    }
    if let Some(value) = config
        .workspace_slug
        .as_deref()
        .and_then(|value| HeaderValue::from_str(value).ok())
    {
        request = request.header("X-Workspace-Slug", value);
    }
    request
}

fn prepare_get(
    client: &Client,
    url: Url,
    config: &MulticaConnectionConfig,
) -> reqwest::RequestBuilder {
    apply_workspace_headers(apply_token(client.get(url), config), config)
}

#[derive(Deserialize)]
struct ManagedWorkspaceIssuesPayload {
    issues: Vec<MulticaWorkspaceIssue>,
    total: u64,
    #[serde(default)]
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct ManagedWorkspaceProjectsPayload {
    projects: Vec<MulticaWorkspaceProject>,
    total: u64,
    #[serde(default)]
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct ManagedWorkspaceAutopilotsPayload {
    autopilots: Vec<MulticaWorkspaceAutopilot>,
    total: u64,
    #[serde(default)]
    next_cursor: Option<String>,
}

fn validate_managed_workspace_cursor(cursor: Option<&str>) -> anyhow::Result<Option<String>> {
    let Some(cursor) = cursor else {
        return Ok(None);
    };
    if cursor.is_empty()
        || cursor.len() > MANAGED_WORKSPACE_MAX_CURSOR_LENGTH
        || cursor.chars().any(char::is_control)
    {
        bail!("managed_workspace_cursor_invalid");
    }
    Ok(Some(cursor.to_string()))
}

fn validate_runtime_local_skill(skill: &mut MulticaRuntimeLocalSkillSummary) -> anyhow::Result<()> {
    let stable_ref = |value: &str| {
        !value.is_empty()
            && value.len() <= 240
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'@')
            })
    };
    if !stable_ref(&skill.key) || skill.name.trim().is_empty() {
        bail!("managed_workspace_skill_inventory_invalid");
    }
    skill.name = skill.name.trim().chars().take(MAX_TEXT_LENGTH).collect();
    skill.description = skill.description.take().map(|value| {
        value
            .trim()
            .chars()
            .take(MAX_PUBLIC_TEXT_INPUT_LENGTH)
            .collect()
    });
    for value in [&mut skill.provider, &mut skill.root, &mut skill.plugin] {
        if let Some(text) = value.take() {
            let text = text.trim();
            if !text.is_empty() && !stable_ref(text) {
                bail!("managed_workspace_skill_inventory_invalid");
            }
            *value = (!text.is_empty()).then(|| text.to_string());
        }
    }
    skill.file_count = skill.file_count.min(10_000);
    Ok(())
}

fn managed_workspace_list_result<T>(
    mut items: Vec<T>,
    total: Option<u64>,
    next_cursor: Option<String>,
    request: &MulticaWorkspaceListRequest,
    workspace_id: impl Fn(&T) -> &str,
) -> anyhow::Result<(Vec<T>, u64, Option<String>)> {
    if items
        .iter()
        .any(|item| workspace_id(item) != request.workspace_id)
    {
        bail!("managed_workspace_tenant_mismatch");
    }
    let observed_total = u64::try_from(items.len()).unwrap_or(u64::MAX);
    items.truncate(request.limit);
    let next_cursor = validate_managed_workspace_cursor(next_cursor.as_deref())?;
    Ok((items, total.unwrap_or(observed_total), next_cursor))
}

impl MulticaManagedWorkspaceClient {
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    fn validate_workspace(&self, workspace_id: &str) -> anyhow::Result<()> {
        let workspace_id = validate_managed_workspace_id(workspace_id)?;
        if workspace_id != self.workspace_id {
            bail!("managed_workspace_scope_mismatch");
        }
        Ok(())
    }

    fn endpoint(&self, resource: MulticaWorkspaceReadResource) -> anyhow::Result<Url> {
        let endpoint = self
            .server_origin
            .join(resource.path())
            .map_err(|_| anyhow!("managed_workspace_endpoint_invalid"))?;
        if endpoint.scheme() != self.server_origin.scheme()
            || endpoint.host_str() != self.server_origin.host_str()
            || endpoint.port_or_known_default() != self.server_origin.port_or_known_default()
        {
            bail!("managed_workspace_endpoint_invalid");
        }
        Ok(endpoint)
    }

    fn list_endpoint(
        &self,
        resource: MulticaWorkspaceReadResource,
        request: &MulticaWorkspaceListRequest,
    ) -> anyhow::Result<Url> {
        self.validate_workspace(&request.workspace_id)?;
        if request.limit == 0 || request.limit > MAX_COLLECTION_ITEMS {
            bail!("managed_workspace_limit_invalid");
        }
        let cursor = validate_managed_workspace_cursor(request.cursor.as_deref())?;
        let mut endpoint = self.endpoint(resource)?;
        {
            let mut query = endpoint.query_pairs_mut();
            query.append_pair("workspace_id", &self.workspace_id);
            query.append_pair("limit", &request.limit.to_string());
            if let Some(cursor) = cursor.as_deref() {
                query.append_pair("cursor", cursor);
            }
        }
        Ok(endpoint)
    }

    fn runtime_local_skills_endpoint(
        &self,
        runtime_id: &str,
        request_id: Option<&str>,
    ) -> anyhow::Result<Url> {
        let runtime_id = validate_managed_workspace_id(runtime_id)?;
        let request_id = request_id.map(validate_managed_workspace_id).transpose()?;
        let path = match request_id {
            Some(request_id) => {
                format!("/api/runtimes/{runtime_id}/local-skills/{request_id}")
            }
            None => format!("/api/runtimes/{runtime_id}/local-skills"),
        };
        let endpoint = self
            .server_origin
            .join(&path)
            .map_err(|_| anyhow!("managed_workspace_endpoint_invalid"))?;
        if endpoint.scheme() != self.server_origin.scheme()
            || endpoint.host_str() != self.server_origin.host_str()
            || endpoint.port_or_known_default() != self.server_origin.port_or_known_default()
        {
            bail!("managed_workspace_endpoint_invalid");
        }
        Ok(endpoint)
    }

    async fn request_json<T: DeserializeOwned>(
        &self,
        method: Method,
        url: Url,
    ) -> anyhow::Result<T> {
        let response = self
            .client
            .request(method, url)
            .header(ACCEPT, "application/json")
            .header(AUTHORIZATION, self.authorization.clone())
            .header("X-Workspace-ID", &self.workspace_id)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    anyhow!("managed_workspace_timeout")
                } else {
                    anyhow!("managed_workspace_network_error")
                }
            })?;
        match response.status() {
            StatusCode::UNAUTHORIZED => bail!("managed_workspace_unauthorized"),
            StatusCode::FORBIDDEN => bail!("managed_workspace_forbidden"),
            StatusCode::NOT_FOUND => bail!("managed_workspace_not_found"),
            status if !status.is_success() => bail!("managed_workspace_http_status"),
            _ => {}
        }
        let bytes = read_response_body_limited(response)
            .await
            .map_err(|error| match error {
                ResponseBodyError::TooLarge => anyhow!("managed_workspace_response_too_large"),
                ResponseBodyError::ReadFailed => anyhow!("managed_workspace_network_error"),
            })?;
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("managed_workspace_invalid_json"))
    }

    async fn get_json<T: DeserializeOwned>(&self, url: Url) -> anyhow::Result<T> {
        self.request_json(Method::GET, url).await
    }

    async fn post_empty_json<T: DeserializeOwned>(&self, url: Url) -> anyhow::Result<T> {
        self.request_json(Method::POST, url).await
    }

    pub async fn get_me(&self, workspace_id: &str) -> anyhow::Result<MulticaWorkspaceMeResponse> {
        self.validate_workspace(workspace_id)?;
        let user = self
            .get_json(self.endpoint(MulticaWorkspaceReadResource::Me)?)
            .await?;
        Ok(MulticaWorkspaceMeResponse {
            workspace_id: self.workspace_id.clone(),
            user,
        })
    }

    pub async fn list_issues(
        &self,
        request: &MulticaWorkspaceListRequest,
    ) -> anyhow::Result<MulticaWorkspaceIssuesResponse> {
        let payload: ManagedWorkspaceIssuesPayload = self
            .get_json(self.list_endpoint(MulticaWorkspaceReadResource::Issues, request)?)
            .await?;
        let (issues, total, next_cursor) = managed_workspace_list_result(
            payload.issues,
            Some(payload.total),
            payload.next_cursor,
            request,
            |item| &item.workspace_id,
        )?;
        Ok(MulticaWorkspaceIssuesResponse {
            workspace_id: self.workspace_id.clone(),
            issues,
            total,
            next_cursor,
        })
    }

    pub async fn list_projects(
        &self,
        request: &MulticaWorkspaceListRequest,
    ) -> anyhow::Result<MulticaWorkspaceProjectsResponse> {
        let payload: ManagedWorkspaceProjectsPayload = self
            .get_json(self.list_endpoint(MulticaWorkspaceReadResource::Projects, request)?)
            .await?;
        let (projects, total, next_cursor) = managed_workspace_list_result(
            payload.projects,
            Some(payload.total),
            payload.next_cursor,
            request,
            |item| &item.workspace_id,
        )?;
        Ok(MulticaWorkspaceProjectsResponse {
            workspace_id: self.workspace_id.clone(),
            projects,
            total,
            next_cursor,
        })
    }

    pub async fn list_agents(
        &self,
        request: &MulticaWorkspaceListRequest,
    ) -> anyhow::Result<MulticaWorkspaceAgentsResponse> {
        let items: Vec<MulticaWorkspaceAgent> = self
            .get_json(self.list_endpoint(MulticaWorkspaceReadResource::Agents, request)?)
            .await?;
        let (agents, total, next_cursor) =
            managed_workspace_list_result(items, None, None, request, |item| &item.workspace_id)?;
        Ok(MulticaWorkspaceAgentsResponse {
            workspace_id: self.workspace_id.clone(),
            agents,
            total,
            next_cursor,
        })
    }

    pub async fn list_runtimes(
        &self,
        request: &MulticaWorkspaceListRequest,
    ) -> anyhow::Result<MulticaWorkspaceRuntimesResponse> {
        let items: Vec<MulticaWorkspaceRuntime> = self
            .get_json(self.list_endpoint(MulticaWorkspaceReadResource::Runtimes, request)?)
            .await?;
        let (runtimes, total, next_cursor) =
            managed_workspace_list_result(items, None, None, request, |item| &item.workspace_id)?;
        Ok(MulticaWorkspaceRuntimesResponse {
            workspace_id: self.workspace_id.clone(),
            runtimes,
            total,
            next_cursor,
        })
    }

    pub async fn list_skills(
        &self,
        request: &MulticaWorkspaceListRequest,
    ) -> anyhow::Result<MulticaWorkspaceSkillsResponse> {
        let items: Vec<MulticaWorkspaceSkill> = self
            .get_json(self.list_endpoint(MulticaWorkspaceReadResource::Skills, request)?)
            .await?;
        let (skills, total, next_cursor) =
            managed_workspace_list_result(items, None, None, request, |item| &item.workspace_id)?;
        Ok(MulticaWorkspaceSkillsResponse {
            workspace_id: self.workspace_id.clone(),
            skills,
            total,
            next_cursor,
        })
    }

    pub async fn discover_runtime_local_skills(
        &self,
        runtime: &MulticaWorkspaceRuntime,
    ) -> anyhow::Result<MulticaRuntimeLocalSkillInventory> {
        self.validate_workspace(&runtime.workspace_id)?;
        if runtime.provider.as_deref() != Some("codex") {
            bail!("managed_workspace_runtime_provider_mismatch");
        }
        if !matches!(
            runtime.status.as_deref(),
            Some("online" | "ready" | "idle" | "working")
        ) {
            bail!("managed_workspace_runtime_unavailable");
        }

        let capabilities = runtime.capabilities();
        if !capabilities.iter().any(|value| value == "skill-bundles-v1")
            && !capabilities.iter().any(|value| value == "agent-skill-v1")
        {
            bail!("managed_workspace_runtime_skills_unsupported");
        }

        let mut request: ManagedRuntimeLocalSkillRequest = self
            .post_empty_json(self.runtime_local_skills_endpoint(&runtime.id, None)?)
            .await?;
        if request.runtime_id != runtime.id {
            bail!("managed_workspace_runtime_mismatch");
        }
        let request_id = validate_managed_workspace_id(&request.id)?;
        for attempt in 0..=MANAGED_WORKSPACE_SKILL_POLL_ATTEMPTS {
            match request.status.as_str() {
                "completed" => break,
                "failed" | "timeout" | "conflict" => {
                    bail!("managed_workspace_skill_inventory_failed")
                }
                "pending" | "running" if attempt < MANAGED_WORKSPACE_SKILL_POLL_ATTEMPTS => {
                    tokio::time::sleep(MANAGED_WORKSPACE_SKILL_POLL_DELAY).await;
                    request = self
                        .get_json(
                            self.runtime_local_skills_endpoint(&runtime.id, Some(&request_id))?,
                        )
                        .await?;
                    if request.runtime_id != runtime.id || request.id != request_id {
                        bail!("managed_workspace_runtime_mismatch");
                    }
                }
                _ => bail!("managed_workspace_skill_inventory_timeout"),
            }
        }

        request.skills.truncate(MAX_COLLECTION_ITEMS);
        for skill in &mut request.skills {
            validate_runtime_local_skill(skill)?;
        }
        Ok(MulticaRuntimeLocalSkillInventory {
            workspace_id: self.workspace_id.clone(),
            runtime_id: runtime.id.clone(),
            supported: request.supported,
            skills: request.skills,
        })
    }

    pub async fn list_squads(
        &self,
        request: &MulticaWorkspaceListRequest,
    ) -> anyhow::Result<MulticaWorkspaceSquadsResponse> {
        let items: Vec<MulticaWorkspaceSquad> = self
            .get_json(self.list_endpoint(MulticaWorkspaceReadResource::Squads, request)?)
            .await?;
        let (squads, total, next_cursor) =
            managed_workspace_list_result(items, None, None, request, |item| &item.workspace_id)?;
        Ok(MulticaWorkspaceSquadsResponse {
            workspace_id: self.workspace_id.clone(),
            squads,
            total,
            next_cursor,
        })
    }

    pub async fn list_autopilots(
        &self,
        request: &MulticaWorkspaceListRequest,
    ) -> anyhow::Result<MulticaWorkspaceAutopilotsResponse> {
        let payload: ManagedWorkspaceAutopilotsPayload = self
            .get_json(self.list_endpoint(MulticaWorkspaceReadResource::Autopilots, request)?)
            .await?;
        let (autopilots, total, next_cursor) = managed_workspace_list_result(
            payload.autopilots,
            Some(payload.total),
            payload.next_cursor,
            request,
            |item| &item.workspace_id,
        )?;
        Ok(MulticaWorkspaceAutopilotsResponse {
            workspace_id: self.workspace_id.clone(),
            autopilots,
            total,
            next_cursor,
        })
    }
}

#[derive(Debug)]
enum HealthProbeResult {
    Healthy {
        endpoint: &'static str,
        http_status: u16,
        version: Option<String>,
        service_status: Option<String>,
    },
    NotFound,
    Unauthorized(u16),
    WorkspaceContextRequired(u16),
    HttpStatus(u16),
    Timeout,
    NetworkError,
    InvalidJson(u16),
    ResponseTooLarge(u16),
    InvalidEndpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponseBodyError {
    TooLarge,
    ReadFailed,
}

/// Read an HTTP body with an explicit upper bound.  The response is consumed
/// in chunks so a missing/incorrect Content-Length cannot bypass the limit.
/// No body is logged or included in an error string.
async fn read_response_body_limited(
    mut response: reqwest::Response,
) -> Result<Vec<u8>, ResponseBodyError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(ResponseBodyError::TooLarge);
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ResponseBodyError::ReadFailed)?
    {
        let Some(next_len) = body.len().checked_add(chunk.len()) else {
            return Err(ResponseBodyError::TooLarge);
        };
        if next_len > MAX_RESPONSE_BYTES {
            return Err(ResponseBodyError::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

async fn probe_health_endpoint(
    client: &Client,
    config: &MulticaConnectionConfig,
    endpoint: &'static str,
) -> HealthProbeResult {
    let url = match endpoint_url(config, endpoint) {
        Ok(url) => url,
        Err(_) => return HealthProbeResult::InvalidEndpoint,
    };
    let response = match prepare_get(client, url, config).send().await {
        Ok(response) => response,
        Err(error) if error.is_timeout() => return HealthProbeResult::Timeout,
        Err(_) => return HealthProbeResult::NetworkError,
    };
    let http_status = response.status().as_u16();
    if response.status() == StatusCode::NOT_FOUND {
        return HealthProbeResult::NotFound;
    }
    if response.status() == StatusCode::UNAUTHORIZED {
        return HealthProbeResult::Unauthorized(http_status);
    }
    if response.status() == StatusCode::FORBIDDEN {
        return HealthProbeResult::WorkspaceContextRequired(http_status);
    }
    if !response.status().is_success() {
        return HealthProbeResult::HttpStatus(http_status);
    }
    let bytes = match read_response_body_limited(response).await {
        Ok(bytes) => bytes,
        Err(ResponseBodyError::TooLarge) => {
            return HealthProbeResult::ResponseTooLarge(http_status);
        }
        Err(ResponseBodyError::ReadFailed) => return HealthProbeResult::NetworkError,
    };
    let value = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) if value.is_object() => value,
        Ok(_) | Err(_) => return HealthProbeResult::InvalidJson(http_status),
    };
    // Both Multica health endpoints expose a schema-level `status` field.
    // Treat a successful HTTP response without a usable status as an
    // incompatible response; a 2xx alone is not proof that the service is
    // ready (and must not be surfaced as healthy by the manager).
    let service_status = value
        .get("status")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    if service_status.is_none() {
        return HealthProbeResult::InvalidJson(http_status);
    }
    HealthProbeResult::Healthy {
        endpoint,
        http_status,
        version: extract_version(&value),
        service_status,
    }
}

async fn probe_server(config: &MulticaConnectionConfig) -> MulticaHealthStatus {
    let started = Instant::now();
    let checked_at_ms = Some(now_ms());
    let client = match build_client() {
        Ok(client) => client,
        Err(_) => {
            return MulticaHealthStatus {
                status: "degraded".to_string(),
                checked_at_ms,
                duration_ms: Some(started.elapsed().as_millis() as u64),
                diagnostic: Some(diagnostic("http_client_init_failed")),
                ..Default::default()
            };
        }
    };
    let finish = |status: &str,
                  endpoint: Option<&str>,
                  http_status: Option<u16>,
                  version: Option<String>,
                  diagnostic_code: Option<&str>| MulticaHealthStatus {
        status: status.to_string(),
        endpoint: endpoint.map(str::to_string),
        http_status,
        version,
        checked_at_ms,
        duration_ms: Some(started.elapsed().as_millis() as u64),
        diagnostic: diagnostic_code.map(diagnostic),
    };

    let live = probe_health_endpoint(&client, config, "health").await;
    let live_version = match live {
        HealthProbeResult::Healthy { version, .. } => version,
        HealthProbeResult::NotFound => {
            return finish(
                "invalid_response",
                Some("health"),
                Some(StatusCode::NOT_FOUND.as_u16()),
                None,
                Some("health_endpoint_not_found"),
            );
        }
        HealthProbeResult::Unauthorized(status) => {
            return finish(
                "unauthorized",
                Some("health"),
                Some(status),
                None,
                Some("authentication_required"),
            );
        }
        HealthProbeResult::WorkspaceContextRequired(status) => {
            return finish(
                "degraded",
                Some("health"),
                Some(status),
                None,
                Some("workspace_context_required"),
            );
        }
        HealthProbeResult::HttpStatus(status) => {
            return finish(
                if status >= 500 {
                    "degraded"
                } else {
                    "invalid_response"
                },
                Some("health"),
                Some(status),
                None,
                Some("http_status_not_2xx"),
            );
        }
        HealthProbeResult::Timeout => {
            return finish("unreachable", Some("health"), None, None, Some("timeout"));
        }
        HealthProbeResult::NetworkError => {
            return finish(
                "unreachable",
                Some("health"),
                None,
                None,
                Some("network_error"),
            );
        }
        HealthProbeResult::InvalidJson(status) => {
            return finish(
                "invalid_response",
                Some("health"),
                Some(status),
                None,
                Some("invalid_json"),
            );
        }
        HealthProbeResult::ResponseTooLarge(status) => {
            return finish(
                "invalid_response",
                Some("health"),
                Some(status),
                None,
                Some("response_too_large"),
            );
        }
        HealthProbeResult::InvalidEndpoint => {
            return finish(
                "invalid_response",
                None,
                None,
                None,
                Some("endpoint_url_invalid"),
            );
        }
    };

    // `/readyz` is the canonical readiness endpoint. `/healthz` is retained
    // only as the documented alias used by older Multica deployments.
    let (ready_endpoint, ready) = match probe_health_endpoint(&client, config, "readyz").await {
        HealthProbeResult::NotFound => (
            "healthz",
            probe_health_endpoint(&client, config, "healthz").await,
        ),
        result => ("readyz", result),
    };
    match ready {
        HealthProbeResult::Healthy {
            endpoint,
            http_status,
            version,
            service_status,
        } => {
            if service_status
                .as_deref()
                .is_some_and(|status| status != "ok" && status != "ready" && status != "healthy")
            {
                return finish(
                    "degraded",
                    Some(endpoint),
                    Some(http_status),
                    version.or(live_version),
                    Some("not_ready"),
                );
            }
            finish(
                "healthy",
                Some(endpoint),
                Some(http_status),
                version.or(live_version),
                None,
            )
        }
        HealthProbeResult::NotFound => finish(
            "invalid_response",
            Some(ready_endpoint),
            Some(StatusCode::NOT_FOUND.as_u16()),
            live_version,
            Some("readiness_endpoint_not_found"),
        ),
        HealthProbeResult::Unauthorized(status) => finish(
            "unauthorized",
            Some(ready_endpoint),
            Some(status),
            live_version,
            Some("authentication_required"),
        ),
        HealthProbeResult::WorkspaceContextRequired(status) => finish(
            "degraded",
            Some(ready_endpoint),
            Some(status),
            live_version,
            Some("workspace_context_required"),
        ),
        HealthProbeResult::HttpStatus(status) => finish(
            if status == StatusCode::SERVICE_UNAVAILABLE.as_u16() || status >= 500 {
                "degraded"
            } else {
                "invalid_response"
            },
            Some(ready_endpoint),
            Some(status),
            live_version,
            Some(if status == StatusCode::SERVICE_UNAVAILABLE.as_u16() {
                "not_ready"
            } else {
                "http_status_not_2xx"
            }),
        ),
        HealthProbeResult::Timeout => finish(
            "unreachable",
            Some(ready_endpoint),
            None,
            live_version,
            Some("timeout"),
        ),
        HealthProbeResult::NetworkError => finish(
            "unreachable",
            Some(ready_endpoint),
            None,
            live_version,
            Some("network_error"),
        ),
        HealthProbeResult::InvalidJson(status) => finish(
            "invalid_response",
            Some(ready_endpoint),
            Some(status),
            live_version,
            Some("invalid_json"),
        ),
        HealthProbeResult::ResponseTooLarge(status) => finish(
            "invalid_response",
            Some(ready_endpoint),
            Some(status),
            live_version,
            Some("response_too_large"),
        ),
        HealthProbeResult::InvalidEndpoint => finish(
            "invalid_response",
            None,
            None,
            None,
            Some("endpoint_url_invalid"),
        ),
    }
}

#[derive(Debug)]
enum DaemonHealthProbeResult {
    Response {
        status: String,
        pid: Option<u32>,
        profile: Option<String>,
        version: Option<String>,
        http_status: u16,
    },
    Unauthorized(u16),
    HttpStatus(u16),
    Timeout,
    NetworkError,
    InvalidJson(u16),
    ResponseTooLarge(u16),
    InvalidEndpoint,
}

#[derive(Debug)]
struct DaemonHealthProbe {
    result: DaemonHealthProbeResult,
    duration_ms: u64,
}

/// Probe the Multica daemon's local read-only `/health` endpoint.  This is
/// deliberately independent of the configured Multica server URL and uses a
/// short request timeout so a dead local daemon cannot hold up the manager.
async fn probe_daemon_health(profile: &str) -> DaemonHealthProbe {
    let started = Instant::now();
    let finish = |result| DaemonHealthProbe {
        result,
        duration_ms: started.elapsed().as_millis() as u64,
    };
    let endpoint = match daemon_health_endpoint(profile) {
        Ok(endpoint) => endpoint,
        Err(_) => return finish(DaemonHealthProbeResult::InvalidEndpoint),
    };
    let client = match Client::builder()
        .timeout(DAEMON_HEALTH_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT.min(DAEMON_HEALTH_TIMEOUT))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(_) => return finish(DaemonHealthProbeResult::NetworkError),
    };
    let response = match client.get(endpoint).send().await {
        Ok(response) => response,
        Err(error) if error.is_timeout() => {
            return finish(DaemonHealthProbeResult::Timeout);
        }
        Err(_) => return finish(DaemonHealthProbeResult::NetworkError),
    };
    let http_status = response.status().as_u16();
    if response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::FORBIDDEN {
        return finish(DaemonHealthProbeResult::Unauthorized(http_status));
    }
    if !response.status().is_success() {
        return finish(DaemonHealthProbeResult::HttpStatus(http_status));
    }
    let bytes = match read_response_body_limited(response).await {
        Ok(bytes) => bytes,
        Err(ResponseBodyError::TooLarge) => {
            return finish(DaemonHealthProbeResult::ResponseTooLarge(http_status));
        }
        Err(ResponseBodyError::ReadFailed) => {
            return finish(DaemonHealthProbeResult::NetworkError);
        }
    };
    let value = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) if value.is_object() => value,
        Ok(_) | Err(_) => return finish(DaemonHealthProbeResult::InvalidJson(http_status)),
    };
    let Some(status) = value
        .get("status")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
    else {
        return finish(DaemonHealthProbeResult::InvalidJson(http_status));
    };
    let pid = value
        .get("pid")
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
        .and_then(|value| u32::try_from(value).ok());
    let profile = value
        .get("profile")
        .and_then(Value::as_str)
        .map(|value| value.to_string());
    let version = extract_version(&value).or_else(|| {
        value
            .get("cli_version")
            .and_then(Value::as_str)
            .map(truncate)
    });
    finish(DaemonHealthProbeResult::Response {
        status,
        pid,
        profile,
        version,
        http_status,
    })
}

fn daemon_status_from_probe(
    probe: DaemonHealthProbe,
    expected_pid: u32,
    expected_profile: &str,
    started_at_ms: u64,
) -> MulticaDaemonStatus {
    let checked_at_ms = Some(now_ms());
    let common = |status: &str,
                  endpoint: Option<&str>,
                  http_status: Option<u16>,
                  version: Option<String>,
                  diagnostic_code: Option<&str>| MulticaDaemonStatus {
        status: status.to_string(),
        pid: Some(expected_pid),
        started_at_ms: Some(started_at_ms),
        endpoint: endpoint.map(str::to_string),
        http_status,
        version,
        checked_at_ms,
        duration_ms: Some(probe.duration_ms),
        diagnostic: diagnostic_code.map(diagnostic),
        ..Default::default()
    };

    match probe.result {
        DaemonHealthProbeResult::Response {
            status,
            pid,
            profile,
            version,
            http_status,
        } => {
            let Some(reported_pid) = pid else {
                return common(
                    "degraded",
                    Some("health"),
                    Some(http_status),
                    version,
                    Some("daemon_pid_missing"),
                );
            };
            if reported_pid != expected_pid {
                return common(
                    "degraded",
                    Some("health"),
                    Some(http_status),
                    version,
                    Some("daemon_pid_mismatch"),
                );
            }
            let Some(reported_profile) = profile.as_deref() else {
                return common(
                    "degraded",
                    Some("health"),
                    Some(http_status),
                    version,
                    Some("daemon_profile_missing"),
                );
            };
            if reported_profile != expected_profile {
                return common(
                    "degraded",
                    Some("health"),
                    Some(http_status),
                    version,
                    Some("daemon_profile_mismatch"),
                );
            }
            match status.as_str() {
                // Multica's daemon contract treats `running` as readiness.
                // `starting` may already have a bound health port, but it has
                // not completed preflight and must never be surfaced as a
                // healthy sidecar.
                "running" => common("healthy", Some("health"), Some(http_status), version, None),
                "starting" => common(
                    "checking",
                    Some("health"),
                    Some(http_status),
                    version,
                    Some("daemon_starting"),
                ),
                "degraded" | "unhealthy" => common(
                    "degraded",
                    Some("health"),
                    Some(http_status),
                    version,
                    Some("daemon_degraded"),
                ),
                "stopped" | "failed" | "error" => common(
                    "stopped",
                    Some("health"),
                    Some(http_status),
                    version,
                    Some("daemon_stopped"),
                ),
                _ => common(
                    "invalid_response",
                    Some("health"),
                    Some(http_status),
                    version,
                    Some("unknown_daemon_status"),
                ),
            }
        }
        DaemonHealthProbeResult::Unauthorized(http_status) => common(
            "unauthorized",
            Some("health"),
            Some(http_status),
            None,
            Some("authentication_required"),
        ),
        DaemonHealthProbeResult::HttpStatus(http_status) => common(
            if http_status >= 500 {
                "degraded"
            } else {
                "invalid_response"
            },
            Some("health"),
            Some(http_status),
            None,
            Some("http_status_not_2xx"),
        ),
        DaemonHealthProbeResult::Timeout => {
            common("unreachable", Some("health"), None, None, Some("timeout"))
        }
        DaemonHealthProbeResult::NetworkError => common(
            "unreachable",
            Some("health"),
            None,
            None,
            Some("network_error"),
        ),
        DaemonHealthProbeResult::InvalidJson(http_status) => common(
            "invalid_response",
            Some("health"),
            Some(http_status),
            None,
            Some("invalid_json"),
        ),
        DaemonHealthProbeResult::ResponseTooLarge(http_status) => common(
            "invalid_response",
            Some("health"),
            Some(http_status),
            None,
            Some("response_too_large"),
        ),
        DaemonHealthProbeResult::InvalidEndpoint => common(
            "invalid_response",
            None,
            None,
            None,
            Some("endpoint_url_invalid"),
        ),
    }
}

/// A freshly spawned daemon can bind its profile-specific health listener
/// before its preflight checks finish. Retry only transient startup outcomes
/// within the existing health budget; all terminal failures remain visible to
/// the caller immediately.
fn should_retry_daemon_after_start(status: &MulticaDaemonStatus) -> bool {
    matches!(status.status.as_str(), "checking" | "unreachable")
}

async fn probe_daemon_after_start(
    connection_id: &str,
    config: &MulticaConnectionConfig,
) -> MulticaDaemonStatus {
    let deadline = Instant::now() + HEALTH_TOTAL_TIMEOUT;
    let mut status = probe_daemon_for_connection(connection_id, config).await;

    for _ in 1..DAEMON_STARTUP_PROBE_ATTEMPTS {
        if !should_retry_daemon_after_start(&status) {
            return status;
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        tokio::time::sleep(DAEMON_STARTUP_PROBE_RETRY_DELAY.min(remaining)).await;
        status = probe_daemon_for_connection(connection_id, config).await;
    }

    status
}

/// Probe a running sidecar and cache only the result belonging to the same
/// child PID.  If the process was replaced while the HTTP request was in
/// flight, the result is returned as degraded but never written over the new
/// process's cache.
async fn probe_daemon_for_connection(
    connection_id: &str,
    config: &MulticaConnectionConfig,
) -> MulticaDaemonStatus {
    let Some(sidecar) = config.sidecar.as_ref() else {
        return daemon_status(connection_id, false);
    };
    let profile = match validated_sidecar_profile(&sidecar.args) {
        Ok(profile) => profile,
        Err(_) => {
            return MulticaDaemonStatus {
                status: "degraded".to_string(),
                checked_at_ms: Some(now_ms()),
                diagnostic: Some(diagnostic("sidecar_profile_invalid")),
                ..Default::default()
            };
        }
    };
    let target = {
        let mut processes = match sidecars().lock() {
            Ok(processes) => processes,
            Err(_) => {
                return MulticaDaemonStatus {
                    status: "degraded".to_string(),
                    checked_at_ms: Some(now_ms()),
                    diagnostic: Some(diagnostic("sidecar_state_unavailable")),
                    ..Default::default()
                };
            }
        };
        let Some(process) = processes.get_mut(connection_id) else {
            return daemon_status(connection_id, true);
        };
        match sidecar_process_state(process) {
            SidecarProcessState::RunningOwned => Some((process.child.id(), process.started_at_ms)),
            SidecarProcessState::RunningUnverified => {
                return MulticaDaemonStatus {
                    status: "degraded".to_string(),
                    pid: Some(process.child.id()),
                    started_at_ms: Some(process.started_at_ms),
                    checked_at_ms: Some(now_ms()),
                    diagnostic: Some(diagnostic("sidecar_pid_unverified")),
                    ..Default::default()
                };
            }
            SidecarProcessState::RunningForeign => {
                return MulticaDaemonStatus {
                    status: "degraded".to_string(),
                    pid: Some(process.child.id()),
                    started_at_ms: Some(process.started_at_ms),
                    checked_at_ms: Some(now_ms()),
                    diagnostic: Some(diagnostic("sidecar_pid_mismatch")),
                    ..Default::default()
                };
            }
            SidecarProcessState::Exited(exit_code) => {
                return MulticaDaemonStatus {
                    status: "stopped".to_string(),
                    pid: Some(process.child.id()),
                    started_at_ms: Some(process.started_at_ms),
                    exited_at_ms: process.exited_at_ms,
                    exit_code,
                    checked_at_ms: Some(now_ms()),
                    diagnostic: Some(diagnostic("sidecar_exited")),
                    ..Default::default()
                };
            }
            SidecarProcessState::StatusUnavailable => {
                return MulticaDaemonStatus {
                    status: "degraded".to_string(),
                    pid: Some(process.child.id()),
                    started_at_ms: Some(process.started_at_ms),
                    checked_at_ms: Some(now_ms()),
                    diagnostic: Some(diagnostic("sidecar_status_failed")),
                    ..Default::default()
                };
            }
        }
    };
    let Some((pid, started_at_ms)) = target else {
        return daemon_status(connection_id, true);
    };
    let status = daemon_status_from_probe(
        probe_daemon_health(&profile).await,
        pid,
        &profile,
        started_at_ms,
    );
    let mut processes = match sidecars().lock() {
        Ok(processes) => processes,
        Err(_) => return status,
    };
    let Some(process) = processes.get_mut(connection_id) else {
        return status;
    };
    if process.child.id() != pid {
        return MulticaDaemonStatus {
            status: "degraded".to_string(),
            pid: Some(pid),
            started_at_ms: Some(started_at_ms),
            checked_at_ms: Some(now_ms()),
            duration_ms: status.duration_ms,
            diagnostic: Some(diagnostic("sidecar_replaced_during_probe")),
            ..Default::default()
        };
    }
    process.last_health = Some(status.clone());
    status
}

async fn fetch_collection(
    client: &Client,
    config: &MulticaConnectionConfig,
    endpoints: &[&str],
    diagnostics: &mut Vec<String>,
) -> anyhow::Result<Vec<MulticaRuntimeItem>> {
    for endpoint in endpoints {
        let url = collection_endpoint_url(config, endpoint)?;
        let response = match prepare_get(client, url, config).send().await {
            Ok(response) => response,
            Err(error) => {
                diagnostics.push(if error.is_timeout() {
                    "snapshot_timeout".to_string()
                } else {
                    "snapshot_network_error".to_string()
                });
                continue;
            }
        };
        if response.status() == StatusCode::NOT_FOUND {
            continue;
        }
        if response.status() == StatusCode::UNAUTHORIZED {
            diagnostics.push("snapshot_unauthorized".to_string());
            return Err(anyhow!("snapshot_unauthorized"));
        }
        if response.status() == StatusCode::FORBIDDEN {
            // Multica's collection handlers require workspace and user
            // context.  A forbidden response is therefore actionable context
            // feedback, not evidence that the server is unreachable.
            diagnostics.push("workspace_context_required".to_string());
            return Err(anyhow!("workspace_context_required"));
        }
        if !response.status().is_success() {
            diagnostics.push("snapshot_http_status".to_string());
            continue;
        }
        let bytes = match read_response_body_limited(response).await {
            Ok(bytes) => bytes,
            Err(ResponseBodyError::TooLarge) => {
                diagnostics.push("snapshot_response_too_large".to_string());
                return Err(anyhow!("snapshot_response_too_large"));
            }
            Err(ResponseBodyError::ReadFailed) => {
                diagnostics.push("snapshot_network_error".to_string());
                return Err(anyhow!("snapshot_network_error"));
            }
        };
        let value = match serde_json::from_slice::<Value>(&bytes) {
            Ok(value) => value,
            Err(_) => {
                diagnostics.push("snapshot_invalid_json".to_string());
                return Err(anyhow!("snapshot_invalid_json"));
            }
        };
        return Ok(parse_collection(&value));
    }
    diagnostics.push("snapshot_not_found".to_string());
    Err(anyhow!("snapshot_not_found"))
}

fn extract_version(value: &Value) -> Option<String> {
    [
        value.get("version"),
        value.get("data").and_then(|data| data.get("version")),
        value
            .get("service")
            .and_then(|service| service.get("version")),
    ]
    .into_iter()
    .flatten()
    .find_map(Value::as_str)
    .map(sanitize_public_text)
}

fn parse_collection(value: &Value) -> Vec<MulticaRuntimeItem> {
    let array: &[Value] = match value {
        Value::Array(items) => items,
        Value::Object(object) => ["items", "data", "results", "runtimes", "agents", "tasks"]
            .into_iter()
            .find_map(|key| object.get(key).and_then(Value::as_array))
            .map_or(&[][..], |items| items.as_slice()),
        _ => &[][..],
    };
    array
        .iter()
        .take(MAX_COLLECTION_ITEMS)
        .filter_map(parse_item)
        .collect()
}

fn parse_item(value: &Value) -> Option<MulticaRuntimeItem> {
    let object = value.as_object()?;
    let id = ["id", "uuid", "key"]
        .into_iter()
        .find_map(|key| object.get(key).and_then(Value::as_str))
        .map(sanitize_public_text)?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(sanitize_public_text);
    let title = object
        .get("title")
        .and_then(Value::as_str)
        .map(sanitize_public_text);
    let raw_status = object
        .get("status")
        .and_then(Value::as_str)
        .map(|value| sanitize_public_text(value).to_ascii_lowercase());
    let (status, status_diagnostic) = match raw_status {
        None => ("unknown".to_string(), None),
        Some(value) if KNOWN_ITEM_STATUSES.contains(&value.as_str()) => (value, None),
        Some(value) => (
            "unknown".to_string(),
            Some(format!("unknown_status:{value}")),
        ),
    };
    // Runtime heartbeats update `last_seen_at`; prefer that value over the
    // less-frequently-changing resource `updated_at` so the UI does not show
    // an online runtime as stale.  Keep camelCase and millisecond aliases for
    // older/alternate Multica deployments.
    let updated_at_ms = [
        "lastSeenAtMs",
        "last_seen_at_ms",
        "lastSeenAt",
        "last_seen_at",
        "updatedAtMs",
        "updated_at_ms",
        "updatedAt",
        "updated_at",
    ]
    .into_iter()
    .find_map(|key| object.get(key).and_then(parse_timestamp));
    let runtime_type = object
        .get("runtimeMode")
        .or_else(|| object.get("runtime_mode"))
        .or_else(|| object.get("runtimeType"))
        .or_else(|| object.get("runtime_type"))
        .or_else(|| object.get("type"))
        .and_then(Value::as_str)
        .map(sanitize_public_text);
    let provider = object
        .get("provider")
        .and_then(Value::as_str)
        .map(sanitize_public_text);
    let error_summary = object
        .get("error")
        .or_else(|| object.get("errorSummary"))
        .or_else(|| object.get("error_summary"))
        .and_then(Value::as_str)
        .map(sanitize_public_text);
    Some(MulticaRuntimeItem {
        id,
        name,
        title,
        status,
        diagnostic: status_diagnostic,
        updated_at_ms,
        runtime_type,
        provider,
        error_summary,
    })
}

fn parse_timestamp(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number
            .as_u64()
            .map(normalize_timestamp_integer)
            .or_else(|| number.as_f64().and_then(normalize_timestamp_float)),
        Value::String(text) => {
            let text = text.trim();
            text.parse::<u64>()
                .ok()
                .map(normalize_timestamp_integer)
                .or_else(|| text.parse::<f64>().ok().and_then(normalize_timestamp_float))
                .or_else(|| parse_rfc3339_ms(text).and_then(|value| u64::try_from(value).ok()))
        }
        _ => None,
    }
}

fn normalize_timestamp_integer(value: u64) -> u64 {
    if value < 10_000_000_000 {
        value.saturating_mul(1000)
    } else {
        value
    }
}

fn normalize_timestamp_float(value: f64) -> Option<u64> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let millis = if value < 10_000_000_000.0 {
        value * 1000.0
    } else {
        value
    };
    if millis > u64::MAX as f64 {
        return None;
    }
    Some(millis.round() as u64)
}

/// Parse an RFC3339/ISO-8601 timestamp without adding a date-time dependency.
/// Fractional seconds are truncated to milliseconds and both `+HH:MM` and
/// `+HHMM` offsets are accepted.
fn parse_rfc3339_ms(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || !matches!(bytes.get(10), Some(b'T' | b't' | b' '))
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return None;
    }
    let year = parse_digits(bytes, 0, 4)? as i64;
    let month = parse_digits(bytes, 5, 2)?;
    let day = parse_digits(bytes, 8, 2)?;
    let hour = parse_digits(bytes, 11, 2)?;
    let minute = parse_digits(bytes, 14, 2)?;
    let second = parse_digits(bytes, 17, 2)?;
    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }

    let mut index = 19usize;
    let mut fraction_ms = 0i64;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fraction_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == fraction_start {
            return None;
        }
        let digits = &bytes[fraction_start..index];
        for offset in 0..3 {
            fraction_ms *= 10;
            if let Some(digit) = digits.get(offset) {
                fraction_ms += i64::from(*digit - b'0');
            }
        }
    }

    let offset_seconds = match bytes.get(index).copied()? {
        b'Z' | b'z' if index + 1 == bytes.len() => 0i64,
        sign @ (b'+' | b'-') => {
            let remaining = &bytes[index + 1..];
            let (offset_hour, offset_minute) = match remaining.len() {
                5 if remaining.get(2) == Some(&b':') => (
                    parse_digits(remaining, 0, 2)?,
                    parse_digits(remaining, 3, 2)?,
                ),
                4 => (
                    parse_digits(remaining, 0, 2)?,
                    parse_digits(remaining, 2, 2)?,
                ),
                _ => return None,
            };
            if offset_hour > 23 || offset_minute > 59 {
                return None;
            }
            let offset = i64::from(offset_hour * 3_600 + offset_minute * 60);
            if sign == b'-' { -offset } else { offset }
        }
        _ => return None,
    };

    let days = days_from_civil(year, month, day);
    let seconds = days
        .checked_mul(86_400)?
        .checked_add(i64::from(hour) * 3_600)?
        .checked_add(i64::from(minute) * 60)?
        .checked_add(i64::from(second))?
        .checked_sub(offset_seconds)?;
    seconds.checked_mul(1_000)?.checked_add(fraction_ms)
}

fn parse_digits(bytes: &[u8], start: usize, len: usize) -> Option<u32> {
    let digits = bytes.get(start..start.checked_add(len)?)?;
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    digits.iter().try_fold(0u32, |value, digit| {
        value.checked_mul(10)?.checked_add(u32::from(*digit - b'0'))
    })
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::JoinHandle;

    #[derive(Clone)]
    struct FakeHttpResponse {
        status: u16,
        body: &'static str,
    }

    struct FakeHttpServer {
        base_url: String,
        requests: Arc<Mutex<Vec<String>>>,
        handle: Option<JoinHandle<()>>,
    }

    impl FakeHttpServer {
        fn start(responses: Vec<FakeHttpResponse>) -> Self {
            let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind fake server");
            listener
                .set_nonblocking(true)
                .expect("configure fake server listener");
            let address = listener.local_addr().expect("fake server address");
            let requests = Arc::new(Mutex::new(Vec::new()));
            let recorded_requests = Arc::clone(&requests);
            let handle = std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(5);
                let mut response_index = 0;
                while response_index < responses.len() && Instant::now() < deadline {
                    let (mut stream, _) = match listener.accept() {
                        Ok(connection) => connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                            continue;
                        }
                        Err(_) => break,
                    };
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
                    let mut request = Vec::new();
                    let mut buffer = [0u8; 4 * 1024];
                    let request_deadline = Instant::now() + Duration::from_secs(5);
                    while request.len() < 16 * 1024 {
                        match stream.read(&mut buffer) {
                            Ok(0) => break,
                            Ok(size) => {
                                request.extend_from_slice(&buffer[..size]);
                                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                                    break;
                                }
                            }
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::WouldBlock
                                        | std::io::ErrorKind::TimedOut
                                        | std::io::ErrorKind::Interrupted
                                ) && Instant::now() < request_deadline => {}
                            Err(_) => break,
                        }
                    }
                    if let Ok(mut requests) = recorded_requests.lock() {
                        requests.push(String::from_utf8_lossy(&request).into_owned());
                    }
                    let response = &responses[response_index];
                    let reason = match response.status {
                        200 => "OK",
                        401 => "Unauthorized",
                        403 => "Forbidden",
                        404 => "Not Found",
                        500 => "Internal Server Error",
                        _ => "Fixture",
                    };
                    let header = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        response.status,
                        reason,
                        response.body.len()
                    );
                    let _ = stream.write_all(header.as_bytes());
                    let _ = stream.write_all(response.body.as_bytes());
                    response_index += 1;
                }
            });
            Self {
                base_url: format!("http://{address}"),
                requests,
                handle: Some(handle),
            }
        }

        fn join(mut self) -> Vec<String> {
            self.handle
                .take()
                .expect("fake server thread")
                .join()
                .expect("fake server should not panic");
            self.requests
                .lock()
                .map(|requests| requests.clone())
                .unwrap_or_default()
        }
    }

    fn config(url: &str) -> MulticaConnectionConfig {
        MulticaConnectionConfig {
            connection_id: "test".to_string(),
            display_name: "Test".to_string(),
            server_url: url.to_string(),
            api_prefix: None,
            workspace_id: None,
            workspace_slug: None,
            token_env_var: None,
            enabled: true,
            allow_insecure_lan_http: false,
            sidecar: None,
            created_at_ms: 0,
            updated_at_ms: 0,
        }
    }

    fn write_managed_workspace_profile(
        directory: &Path,
        server_url: &str,
        app_url: &str,
        workspace_id: &str,
        token: &str,
    ) {
        fs::create_dir_all(directory).expect("create managed workspace profile fixture");
        let bytes = serde_json::to_vec(&serde_json::json!({
            "server_url": server_url,
            "app_url": app_url,
            "workspace_id": workspace_id,
            "token": token,
            "device_name": "ignored-forward-compatible-field"
        }))
        .expect("encode managed workspace profile fixture");
        fs::write(directory.join(MANAGED_PROFILE_CONFIG_FILE), bytes)
            .expect("write managed workspace profile fixture");
    }

    fn workspace_request(workspace_id: &str, limit: usize) -> MulticaWorkspaceListRequest {
        MulticaWorkspaceListRequest {
            workspace_id: workspace_id.to_string(),
            cursor: Some("cursor-one".to_string()),
            limit,
        }
    }

    #[test]
    fn multica_workspace_profile_credentials_are_bounded_structured_and_private() {
        let temp = tempfile::tempdir().unwrap();
        write_managed_workspace_profile(
            temp.path(),
            "https://api.multica.example",
            "https://multica.example",
            "workspace-a",
            "fixture-secret-token",
        );

        let credentials = load_managed_profile_credentials_from(temp.path()).unwrap();
        assert_eq!(
            credentials.server_origin.as_str(),
            "https://api.multica.example/"
        );
        assert_eq!(credentials.app_origin.as_str(), "https://multica.example/");
        assert_eq!(credentials.workspace_id, "workspace-a");
        assert!(credentials.authorization.is_sensitive());

        let missing = tempfile::tempdir().unwrap();
        let error = load_managed_profile_credentials_from(missing.path())
            .err()
            .expect("missing profile must fail");
        assert_eq!(error.to_string(), "managed_workspace_profile_missing");

        fs::write(
            temp.path().join(MANAGED_PROFILE_CONFIG_FILE),
            vec![b'x'; MANAGED_PROFILE_MAX_CONFIG_BYTES as usize + 1],
        )
        .unwrap();
        let error = load_managed_profile_credentials_from(temp.path())
            .err()
            .expect("oversized profile must fail");
        assert_eq!(error.to_string(), "managed_workspace_profile_invalid");

        fs::write(
            temp.path().join(MANAGED_PROFILE_CONFIG_FILE),
            br#"{"server_url":"https://api.example","token":"fixture-secret-token""#,
        )
        .unwrap();
        let error = load_managed_profile_credentials_from(temp.path())
            .err()
            .expect("invalid JSON must fail");
        assert_eq!(error.to_string(), "managed_workspace_profile_invalid");
        assert!(!error.to_string().contains("fixture-secret-token"));
    }

    #[test]
    fn multica_workspace_profile_rejects_unsafe_origins_and_workspace_ids() {
        let temp = tempfile::tempdir().unwrap();
        let invalid_profiles = [
            (
                "https://user:pass@api.example",
                "https://app.example",
                "workspace-a",
            ),
            (
                "https://api.example?token=secret",
                "https://app.example",
                "workspace-a",
            ),
            (
                "https://api.example/api",
                "https://app.example",
                "workspace-a",
            ),
            (
                "https://api.example",
                "https://app.example/#fragment",
                "workspace-a",
            ),
            (
                "http://public.example",
                "https://app.example",
                "workspace-a",
            ),
            (
                "https://api.example",
                "https://app.example",
                "../workspace-b",
            ),
        ];
        for (server_url, app_url, workspace_id) in invalid_profiles {
            write_managed_workspace_profile(
                temp.path(),
                server_url,
                app_url,
                workspace_id,
                "fixture-secret-token",
            );
            let error = load_managed_profile_credentials_from(temp.path())
                .err()
                .expect("unsafe managed profile must fail");
            assert!(matches!(
                error.to_string().as_str(),
                "managed_workspace_profile_invalid" | "managed_workspace_id_invalid"
            ));
            assert!(!error.to_string().contains("secret"));
            assert!(!error.to_string().contains("https://"));
        }
    }

    #[tokio::test]
    async fn multica_workspace_client_uses_only_fixed_get_routes_and_typed_dtos() {
        let server = FakeHttpServer::start(vec![
            FakeHttpResponse {
                status: 200,
                body: r#"{"id":"user-a","name":"User","email":"user@example.test","token":"must-be-ignored"}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"{"issues":[{"id":"issue-a","workspace_id":"workspace-a","title":"Issue","status":"open","description":"private full body"}],"total":1}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"{"projects":[{"id":"project-a","workspace_id":"workspace-a","title":"Project","status":"planned"}],"total":1}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"[{"id":"agent-a","workspace_id":"workspace-a","name":"Agent","status":"idle","instructions":"private prompt","mcp_config":{"token":"private"}}]"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"[{"id":"runtime-a","workspace_id":"workspace-a","name":"Runtime","status":"online","metadata":{"api_key":"private"}}]"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"[{"id":"skill-a","workspace_id":"workspace-a","name":"Skill","description":"Summary","content":"private full skill"}]"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"[{"id":"squad-a","workspace_id":"workspace-a","name":"Squad","leader_id":"agent-a","instructions":"private"}]"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"{"autopilots":[{"id":"autopilot-a","workspace_id":"workspace-a","title":"Daily","status":"active","assignee_id":"agent-a"}],"total":1}"#,
            },
        ]);
        let temp = tempfile::tempdir().unwrap();
        write_managed_workspace_profile(
            temp.path(),
            &server.base_url,
            &server.base_url,
            "workspace-a",
            "fixture-secret-token",
        );
        let client = managed_workspace_client_from(temp.path()).unwrap();
        let request = workspace_request("workspace-a", 25);

        let me = client.get_me("workspace-a").await.unwrap();
        let issues = client.list_issues(&request).await.unwrap();
        let projects = client.list_projects(&request).await.unwrap();
        let agents = client.list_agents(&request).await.unwrap();
        let runtimes = client.list_runtimes(&request).await.unwrap();
        let skills = client.list_skills(&request).await.unwrap();
        let squads = client.list_squads(&request).await.unwrap();
        let autopilots = client.list_autopilots(&request).await.unwrap();

        assert_eq!(me.user.id, "user-a");
        assert_eq!(issues.issues[0].id, "issue-a");
        assert_eq!(projects.projects[0].id, "project-a");
        assert_eq!(agents.agents[0].id, "agent-a");
        assert_eq!(runtimes.runtimes[0].id, "runtime-a");
        assert_eq!(skills.skills[0].id, "skill-a");
        assert_eq!(squads.squads[0].id, "squad-a");
        assert_eq!(autopilots.autopilots[0].id, "autopilot-a");

        let renderer_json = serde_json::to_string(&serde_json::json!({
            "me": me,
            "issues": issues,
            "projects": projects,
            "agents": agents,
            "runtimes": runtimes,
            "skills": skills,
            "squads": squads,
            "autopilots": autopilots
        }))
        .unwrap();
        for forbidden in [
            "fixture-secret-token",
            "private full body",
            "private prompt",
            "private full skill",
            "api_key",
            "mcp_config",
        ] {
            assert!(!renderer_json.contains(forbidden));
        }

        let requests = server.join();
        assert_eq!(requests.len(), 8);
        let expected_paths = [
            "/api/me",
            "/api/issues",
            "/api/projects",
            "/api/agents",
            "/api/runtimes",
            "/api/skills",
            "/api/squads",
            "/api/autopilots",
        ];
        for (request_text, expected_path) in requests.iter().zip(expected_paths) {
            let lower = request_text.to_ascii_lowercase();
            let request_line = request_text.lines().next().unwrap_or_default();
            assert!(request_line.starts_with(&format!("GET {expected_path}")));
            assert!(!request_line.contains("fixture-secret-token"));
            assert!(lower.contains("authorization: bearer fixture-secret-token"));
            assert!(lower.contains("x-workspace-id: workspace-a"));
            assert!(!lower.contains("cookie:"));
        }
    }

    #[tokio::test]
    async fn multica_workspace_runtime_skill_inventory_is_fixed_typed_and_path_private() {
        let server = FakeHttpServer::start(vec![
            FakeHttpResponse {
                status: 200,
                body: r#"{"id":"request-a","runtime_id":"runtime-a","status":"pending","supported":true}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"{"id":"request-a","runtime_id":"runtime-a","status":"completed","supported":true,"skills":[{"key":"codex:review-helper","name":"Review Helper","description":"Review changes","source_path":"C:\\Users\\fixture\\.codex\\skills\\review-helper","provider":"codex","root":"provider","file_count":3}]}"#,
            },
        ]);
        let temp = tempfile::tempdir().unwrap();
        write_managed_workspace_profile(
            temp.path(),
            &server.base_url,
            &server.base_url,
            "workspace-a",
            "fixture-secret-token",
        );
        let client = managed_workspace_client_from(temp.path()).unwrap();
        let runtime: MulticaWorkspaceRuntime = serde_json::from_value(serde_json::json!({
            "id": "runtime-a",
            "workspace_id": "workspace-a",
            "provider": "codex",
            "status": "online",
            "metadata": {
                "capabilities": ["skill-bundles-v1", "agent-skill-v1"],
                "api_key": "must-not-serialize"
            }
        }))
        .unwrap();

        let inventory = client
            .discover_runtime_local_skills(&runtime)
            .await
            .unwrap();

        assert!(inventory.supported);
        assert_eq!(inventory.runtime_id, "runtime-a");
        assert_eq!(inventory.skills[0].key, "codex:review-helper");
        let renderer_json = serde_json::to_string(&inventory).unwrap();
        assert!(!renderer_json.contains("source_path"));
        assert!(!renderer_json.contains("C:\\\\Users"));
        assert!(!renderer_json.contains("fixture-secret-token"));
        assert!(!renderer_json.contains("must-not-serialize"));

        let requests = server.join();
        assert_eq!(requests.len(), 2);
        assert!(
            requests[0]
                .lines()
                .next()
                .unwrap_or_default()
                .starts_with("POST /api/runtimes/runtime-a/local-skills ")
        );
        assert!(
            requests[1]
                .lines()
                .next()
                .unwrap_or_default()
                .starts_with("GET /api/runtimes/runtime-a/local-skills/request-a ")
        );
    }

    #[tokio::test]
    async fn multica_workspace_client_enforces_scope_limit_and_response_tenant() {
        let server = FakeHttpServer::start(vec![FakeHttpResponse {
            status: 200,
            body: r#"[{"id":"agent-b","workspace_id":"workspace-b","name":"Other"}]"#,
        }]);
        let temp = tempfile::tempdir().unwrap();
        write_managed_workspace_profile(
            temp.path(),
            &server.base_url,
            &server.base_url,
            "workspace-a",
            "fixture-secret-token",
        );
        let client = managed_workspace_client_from(temp.path()).unwrap();

        let error = client
            .list_agents(&workspace_request("workspace-b", 25))
            .await
            .err()
            .expect("cross-workspace request must fail before I/O");
        assert_eq!(error.to_string(), "managed_workspace_scope_mismatch");
        let error = client
            .list_agents(&workspace_request("workspace-a", 101))
            .await
            .err()
            .expect("limit above 100 must fail before I/O");
        assert_eq!(error.to_string(), "managed_workspace_limit_invalid");
        let error = client
            .list_agents(&workspace_request("workspace-a", 25))
            .await
            .err()
            .expect("foreign response item must fail");
        assert_eq!(error.to_string(), "managed_workspace_tenant_mismatch");
        let requests = server.join();
        assert_eq!(requests.len(), 1);
    }

    #[tokio::test]
    async fn multica_workspace_client_bounds_and_redacts_http_failures() {
        let oversized = Box::leak(
            format!(r#"{{"id":"{}"}}"#, "x".repeat(MAX_RESPONSE_BYTES + 1)).into_boxed_str(),
        );
        let server = FakeHttpServer::start(vec![
            FakeHttpResponse {
                status: 401,
                body: r#"{"error":"fixture-secret-token unauthorized"}"#,
            },
            FakeHttpResponse {
                status: 403,
                body: r#"{"error":"private workspace details"}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: "not-json-fixture-secret-token",
            },
            FakeHttpResponse {
                status: 200,
                body: oversized,
            },
        ]);
        let temp = tempfile::tempdir().unwrap();
        write_managed_workspace_profile(
            temp.path(),
            &server.base_url,
            &server.base_url,
            "workspace-a",
            "fixture-secret-token",
        );
        let client = managed_workspace_client_from(temp.path()).unwrap();

        let expected = [
            "managed_workspace_unauthorized",
            "managed_workspace_forbidden",
            "managed_workspace_invalid_json",
            "managed_workspace_response_too_large",
        ];
        for expected_error in expected {
            let error = client
                .get_me("workspace-a")
                .await
                .err()
                .expect("fixture request must fail");
            assert_eq!(error.to_string(), expected_error);
            assert!(!error.to_string().contains("fixture-secret-token"));
            assert!(!error.to_string().contains("private workspace"));
        }
        let requests = server.join();
        assert_eq!(requests.len(), 4);
    }

    #[test]
    fn managed_asset_allowlist_and_defaults_are_pinned() {
        let defaults = managed_connection_defaults();
        assert_eq!(defaults.connection_id, "managed-multica");
        assert_eq!(defaults.display_name, "内置 Multica Runtime");
        assert_eq!(defaults.server_url, "https://api.multica.ai");
        assert_eq!(defaults.profile, "ccp-managed");
        assert!(defaults.enabled && defaults.auto_start && defaults.supervise);

        let windows = managed_asset_for_target("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(windows.asset_name, "multica-cli-0.4.36-windows-amd64.zip");
        assert_eq!(windows.binary_name, "multica.exe");
        assert_eq!(windows.expected_sha256.len(), 64);
        assert!(managed_asset_for_target("x86_64-pc-windows-gnu").is_none());
        assert!(managed_asset_for_target("riscv64gc-unknown-linux-gnu").is_none());
    }

    #[test]
    fn managed_supervisor_generation_and_stop_gates_reject_stale_workers() {
        let mut state = ManagedSupervisorState {
            generation: 11,
            ..Default::default()
        };
        assert!(managed_supervisor_should_continue_state(&state, 11, false));
        assert!(!managed_supervisor_should_continue_state(&state, 10, false));

        state.stop_requested = true;
        assert!(!managed_supervisor_should_continue_state(&state, 11, false));
        state.stop_requested = false;
        assert!(!managed_supervisor_should_continue_state(&state, 11, true));

        state.restart_exhausted = true;
        assert!(!managed_supervisor_should_continue_state(&state, 11, false));
    }

    #[test]
    fn disabled_managed_connection_is_not_supervision_eligible() {
        let mut connection = config("https://managed.multica.example");
        connection.connection_id = MANAGED_RUNTIME_CONNECTION_ID.to_string();
        connection.sidecar = Some(MulticaSidecarConfig {
            executable: "verified-managed-multica.exe".to_string(),
            working_dir: None,
            args: Vec::new(),
            auto_start: true,
        });
        assert!(managed_connection_is_supervision_eligible(&connection));
        connection.enabled = false;
        assert!(!managed_connection_is_supervision_eligible(&connection));
        connection.enabled = true;
        connection.sidecar.as_mut().unwrap().auto_start = false;
        assert!(!managed_connection_is_supervision_eligible(&connection));
        connection.connection_id = "ordinary".to_string();
        connection.sidecar.as_mut().unwrap().auto_start = true;
        assert!(!managed_connection_is_supervision_eligible(&connection));
    }

    #[test]
    fn managed_binary_transition_restarts_only_a_previously_running_usable_runtime() {
        let temp = tempfile::tempdir().expect("temporary transition fixture");
        let ready = MulticaRuntimeInstallStatus {
            install_state: "ready".to_string(),
            sha256_verified: true,
            ..Default::default()
        };
        let starts = AtomicU64::new(0);
        let unchanged = restart_managed_runtime_after_binary_transition_with(
            temp.path(),
            false,
            ready.clone(),
            || {
                starts.fetch_add(1, Ordering::SeqCst);
                Ok(MulticaDaemonStatus {
                    status: "healthy".to_string(),
                    ..Default::default()
                })
            },
        );
        assert_eq!(unchanged, ready);
        assert_eq!(starts.load(Ordering::SeqCst), 0);

        let restarted = restart_managed_runtime_after_binary_transition_with(
            temp.path(),
            true,
            ready.clone(),
            || {
                starts.fetch_add(1, Ordering::SeqCst);
                Ok(MulticaDaemonStatus {
                    status: "healthy".to_string(),
                    ..Default::default()
                })
            },
        );
        assert_eq!(restarted, ready);
        assert_eq!(starts.load(Ordering::SeqCst), 1);

        let failed_restart =
            restart_managed_runtime_after_binary_transition_with(temp.path(), true, ready, || {
                Err(anyhow!("managed_runtime_spawn_failed"))
            });
        assert_eq!(
            failed_restart.last_install_error_code.as_deref(),
            Some("managed_runtime_restart_failed")
        );
        assert_eq!(starts.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn foreign_and_unverified_sidecar_states_are_never_owned() {
        assert!(sidecar_state_is_owned(SidecarProcessState::RunningOwned));
        for state in [
            SidecarProcessState::RunningForeign,
            SidecarProcessState::RunningUnverified,
            SidecarProcessState::StatusUnavailable,
            SidecarProcessState::Exited(None),
        ] {
            assert!(!sidecar_state_is_owned(state));
        }
    }

    #[test]
    fn managed_supervisor_stops_after_three_bounded_restart_attempts() {
        let mut state = ManagedSupervisorState {
            generation: 23,
            ..Default::default()
        };
        let status = MulticaDaemonStatus {
            status: "stopped".to_string(),
            ..Default::default()
        };
        assert!(managed_supervisor_restart_delay(&mut state, 23, &status).is_some());
        assert_eq!(state.restart_attempts, 1);
        assert!(managed_supervisor_restart_delay(&mut state, 23, &status).is_some());
        assert_eq!(state.restart_attempts, 2);
        assert!(managed_supervisor_restart_delay(&mut state, 23, &status).is_some());
        assert_eq!(state.restart_attempts, 3);
        assert!(managed_supervisor_restart_delay(&mut state, 23, &status).is_none());
        assert!(state.restart_exhausted);
        assert_eq!(
            state
                .last_terminal_status
                .as_ref()
                .map(|value| value.status.as_str()),
            Some("restart_exhausted")
        );
        assert!(managed_supervisor_restart_delay(&mut state, 23, &status).is_none());
    }

    #[test]
    fn managed_supervisor_reserves_automatic_rollback_only_once() {
        let mut state = ManagedSupervisorState {
            generation: 31,
            ..Default::default()
        };

        assert!(managed_supervisor_reserve_rollback(&mut state, 31, false));
        assert!(state.rollback_attempted);
        assert!(!managed_supervisor_reserve_rollback(&mut state, 31, false));
        assert!(!managed_supervisor_reserve_rollback(&mut state, 30, false));

        state.rollback_attempted = false;
        state.stop_requested = true;
        assert!(!managed_supervisor_reserve_rollback(&mut state, 31, false));
        state.stop_requested = false;
        assert!(!managed_supervisor_reserve_rollback(&mut state, 31, true));
    }

    #[test]
    fn managed_automatic_rollback_only_handles_runtime_failures() {
        for status in [
            "stopped",
            "degraded",
            "unreachable",
            "invalid_response",
            "checking",
        ] {
            assert!(
                managed_status_requires_rollback(&MulticaDaemonStatus {
                    status: status.to_string(),
                    ..Default::default()
                }),
                "{status} must trigger one automatic rollback attempt"
            );
        }

        for status in ["healthy", "unauthorized", "needs_login"] {
            assert!(
                !managed_status_requires_rollback(&MulticaDaemonStatus {
                    status: status.to_string(),
                    ..Default::default()
                }),
                "{status} must not roll back a verified runtime"
            );
        }
    }

    #[test]
    fn managed_supervisor_classifies_live_health_without_restarting_auth_failures() {
        for status in [
            "stopped",
            "checking",
            "degraded",
            "unreachable",
            "invalid_response",
        ] {
            assert_eq!(
                managed_supervisor_action(&MulticaDaemonStatus {
                    status: status.to_string(),
                    ..Default::default()
                }),
                ManagedSupervisorAction::Recover,
                "{status} must enter bounded recovery"
            );
        }

        for status in ["healthy", "unauthorized", "needs_login"] {
            assert_eq!(
                managed_supervisor_action(&MulticaDaemonStatus {
                    status: status.to_string(),
                    ..Default::default()
                }),
                ManagedSupervisorAction::Observe,
                "{status} must not restart the daemon"
            );
        }

        for diagnostic_code in [
            "sidecar_pid_mismatch",
            "sidecar_pid_unverified",
            "sidecar_status_failed",
            "sidecar_state_unavailable",
            "sidecar_replaced_during_probe",
            "managed_runtime_health_probe_failed",
        ] {
            assert_eq!(
                managed_supervisor_action(&MulticaDaemonStatus {
                    status: "degraded".to_string(),
                    diagnostic: Some(diagnostic(diagnostic_code)),
                    ..Default::default()
                }),
                ManagedSupervisorAction::StopUnsafe,
                "{diagnostic_code} must stop supervision without killing the PID"
            );
        }
    }

    #[test]
    fn managed_supervisor_periodically_reprobes_running_daemon_health() {
        let now = Instant::now();
        let healthy = MulticaDaemonStatus {
            status: "healthy".to_string(),
            pid: Some(42),
            ..Default::default()
        };
        assert!(!managed_supervisor_health_probe_due(
            &healthy,
            now,
            now + MANAGED_SUPERVISOR_HEALTH_PROBE_INTERVAL
        ));
        assert!(managed_supervisor_health_probe_due(&healthy, now, now));

        let checking = MulticaDaemonStatus {
            status: "checking".to_string(),
            pid: Some(42),
            ..Default::default()
        };
        assert!(managed_supervisor_health_probe_due(
            &checking,
            now,
            now + MANAGED_SUPERVISOR_HEALTH_PROBE_INTERVAL
        ));

        let stopped = MulticaDaemonStatus {
            status: "stopped".to_string(),
            pid: Some(42),
            ..Default::default()
        };
        assert!(!managed_supervisor_health_probe_due(&stopped, now, now));
    }

    #[test]
    fn managed_start_failure_rolls_back_to_verified_previous_and_recovers() {
        let temp = tempfile::tempdir().expect("temporary rollback fixture");
        let runtime_root = temp.path().join("runtime");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );
        install_managed_rollback_fixture(&runtime_root, &store);

        let start_error = anyhow!("operating system failure")
            .context("managed_runtime_spawn_failed")
            .context("managed runtime start failed");
        assert!(managed_start_error_requires_rollback(&start_error));

        let stop_calls = std::cell::Cell::new(0usize);
        let restart_calls = std::cell::Cell::new(0usize);
        let rollback_events = std::cell::Cell::new(0usize);
        let recovered = perform_managed_runtime_automatic_rollback_at(
            &runtime_root,
            &store,
            || {
                stop_calls.set(stop_calls.get() + 1);
                Ok(MulticaDaemonStatus {
                    status: "stopped".to_string(),
                    ..Default::default()
                })
            },
            || {
                restart_calls.set(restart_calls.get() + 1);
                assert_managed_runtime_pointer_versions(&runtime_root, "test-1", "test-2");
                Ok(MulticaDaemonStatus {
                    status: "healthy".to_string(),
                    ..Default::default()
                })
            },
            || rollback_events.set(rollback_events.get() + 1),
        )
        .expect("verified previous runtime should recover a failed start");

        assert_eq!(recovered.status, "healthy");
        assert_eq!(stop_calls.get(), 1);
        assert_eq!(restart_calls.get(), 1);
        assert_eq!(rollback_events.get(), 1);
        assert_managed_runtime_pointer_versions(&runtime_root, "test-1", "test-2");
    }

    #[test]
    fn managed_unreachable_health_rolls_back_to_verified_previous_and_recovers() {
        let temp = tempfile::tempdir().expect("temporary rollback fixture");
        let runtime_root = temp.path().join("runtime");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );
        install_managed_rollback_fixture(&runtime_root, &store);

        let failed_health = MulticaDaemonStatus {
            status: "unreachable".to_string(),
            ..Default::default()
        };
        assert!(managed_status_requires_rollback(&failed_health));

        let stop_calls = std::cell::Cell::new(0usize);
        let restart_calls = std::cell::Cell::new(0usize);
        let recovered = perform_managed_runtime_automatic_rollback_at(
            &runtime_root,
            &store,
            || {
                stop_calls.set(stop_calls.get() + 1);
                Ok(MulticaDaemonStatus {
                    status: "stopped".to_string(),
                    ..Default::default()
                })
            },
            || {
                restart_calls.set(restart_calls.get() + 1);
                assert_managed_runtime_pointer_versions(&runtime_root, "test-1", "test-2");
                Ok(MulticaDaemonStatus {
                    status: "healthy".to_string(),
                    ..Default::default()
                })
            },
            || {},
        )
        .expect("verified previous runtime should recover failed health");

        assert_eq!(recovered.status, "healthy");
        assert_eq!(stop_calls.get(), 1);
        assert_eq!(restart_calls.get(), 1);
        assert_managed_runtime_pointer_versions(&runtime_root, "test-1", "test-2");
    }

    #[test]
    fn managed_start_error_rollback_gate_rejects_configuration_and_state_errors() {
        for message in [
            "managed_runtime_server_url_unconfigured",
            "managed_runtime_server_url_invalid",
            "managed_runtime_server_url_reserved_port",
            "managed_runtime_sidecar_contract_invalid",
            "Multica 连接已停用，不能启动 sidecar。",
            "应用正在退出，已拒绝启动 Multica sidecar。",
        ] {
            assert!(
                !managed_start_error_requires_rollback(&anyhow!(message)),
                "{message} must not change the active runtime pointer"
            );
        }

        for marker in [
            "managed_runtime_binary_unavailable",
            "managed_runtime_spawn_failed",
        ] {
            let error = anyhow!("operating system failure")
                .context(marker)
                .context("managed runtime start failed");
            assert!(
                managed_start_error_requires_rollback(&error),
                "{marker} must allow one automatic rollback attempt"
            );
        }
    }

    #[test]
    fn stale_generation_cannot_consume_restart_budget() {
        let mut state = ManagedSupervisorState {
            generation: 24,
            ..Default::default()
        };
        let status = MulticaDaemonStatus {
            status: "stopped".to_string(),
            ..Default::default()
        };
        assert!(managed_supervisor_restart_delay(&mut state, 23, &status).is_none());
        assert_eq!(state.restart_attempts, 0);
        assert!(!state.restart_exhausted);
    }

    #[test]
    fn managed_connection_init_success_keeps_install_status_clean() {
        let status = MulticaRuntimeInstallStatus {
            install_state: "ready".to_string(),
            updated_at_ms: Some(7),
            ..Default::default()
        };
        let original = status.clone();
        let result =
            status_after_managed_connection_init(status, Ok(config("https://example.com")));
        assert_eq!(result, original);
    }

    #[test]
    fn managed_connection_init_failure_is_visible_without_leaking_error_details() {
        let status = MulticaRuntimeInstallStatus {
            install_state: "ready".to_string(),
            updated_at_ms: Some(7),
            ..Default::default()
        };
        let result = status_after_managed_connection_init(
            status,
            Err(anyhow!(
                "secret local path: C:\\Users\\test\\connections.json"
            )),
        );
        assert_eq!(result.install_state, "ready");
        assert_eq!(
            result.last_install_error_code.as_deref(),
            Some(MANAGED_CONNECTION_INIT_ERROR_CODE)
        );
        assert_eq!(
            result.diagnostic.as_deref(),
            Some(MANAGED_CONNECTION_INIT_ERROR_CODE)
        );
        assert_ne!(result.updated_at_ms, Some(7));
        let serialized = serde_json::to_string(&result).expect("serialize status");
        assert!(!serialized.contains("secret local path"));
        assert!(!serialized.contains("connections.json"));
    }

    #[test]
    fn managed_release_urls_reject_unallowlisted_assets_and_hosts() {
        let asset = managed_runtime_asset().expect("test target should be supported");
        let url = managed_release_url(asset.asset_name).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("github.com"));
        assert!(managed_release_url("latest.zip").is_err());
        assert!(validate_release_url(&Url::parse("http://github.com/a").unwrap()).is_err());
        assert!(validate_release_url(&Url::parse("https://evil.example/a").unwrap()).is_err());
        assert!(validate_release_url(&Url::parse("https://github.com/a").unwrap()).is_ok());
        assert!(
            validate_release_url(&Url::parse("https://objects.githubusercontent.com/a").unwrap())
                .is_ok()
        );
    }

    #[test]
    fn managed_checksum_parser_requires_exact_filename_and_digest() {
        let checksums = b"deadbeef  other.zip\n            b96bc1df13824ed1bcb733351eb29ae570cdf3bae1f004dba45215cd011c744c  *target.zip\n";
        assert_eq!(
            parse_release_checksum(checksums, "target.zip").unwrap(),
            "b96bc1df13824ed1bcb733351eb29ae570cdf3bae1f004dba45215cd011c744c"
        );
        assert!(parse_release_checksum(checksums, "other.zip").is_err());
        assert!(parse_release_checksum(b"00 target.zip\n", "target.zip").is_err());
    }

    #[test]
    fn managed_version_probe_requires_exact_json_output() {
        assert_eq!(
            managed_version_probe_args(),
            ["version", "--output", "json"]
        );
        assert!(
            validate_managed_version_probe_output(
                br#"{"version":"0.4.36","commit":"abc"}"#,
                "0.4.36"
            )
            .is_ok()
        );
        assert!(
            validate_managed_version_probe_output(br#"{"version":"0.4.35"}"#, "0.4.36").is_err()
        );
        assert!(
            validate_managed_version_probe_output(br#"{"version":" 0.4.36 "}"#, "0.4.36").is_err()
        );
        assert!(
            validate_managed_version_probe_output(b"multica 0.4.36 (commit: abc)", "0.4.36")
                .is_err()
        );
        assert!(validate_managed_version_probe_output(br#"{"commit":"abc"}"#, "0.4.36").is_err());
        assert!(
            validate_managed_version_probe_output(br#"{"version":"0.4.36"} trailing"#, "0.4.36")
                .is_err()
        );
        let oversized = vec![b' '; MAX_TEXT_LENGTH + 1];
        assert!(validate_managed_version_probe_output(&oversized, "0.4.36").is_err());
    }

    fn test_asset(
        binary_name: &'static str,
        archive_name: &'static str,
        digest: String,
    ) -> MulticaRuntimeAsset {
        test_asset_version("test-1", binary_name, archive_name, digest)
    }

    fn test_asset_version(
        version: &'static str,
        binary_name: &'static str,
        archive_name: &'static str,
        digest: String,
    ) -> MulticaRuntimeAsset {
        let digest = Box::leak(digest.into_boxed_str());
        MulticaRuntimeAsset {
            version,
            target_triple: "x86_64-pc-windows-msvc",
            asset_name: archive_name,
            binary_name,
            expected_sha256: digest,
        }
    }

    fn current_target_test_asset_version(
        version: &'static str,
        archive_name: &'static str,
        digest: String,
    ) -> MulticaRuntimeAsset {
        let pinned = managed_runtime_asset().expect("test target should be supported");
        let digest = Box::leak(digest.into_boxed_str());
        MulticaRuntimeAsset {
            version,
            target_triple: pinned.target_triple,
            asset_name: archive_name,
            binary_name: pinned.binary_name,
            expected_sha256: digest,
        }
    }

    fn test_zip_asset(binary_name: &str, content: &[u8]) -> Vec<u8> {
        let mut archive = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut archive);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("LICENSE", options).unwrap();
            writer.write_all(b"license").unwrap();
            writer.start_file(binary_name, options).unwrap();
            writer.write_all(content).unwrap();
            writer.finish().unwrap();
        }
        archive.into_inner()
    }

    fn install_managed_rollback_fixture(root: &Path, store: &MulticaStore) {
        let binary_name = managed_runtime_asset()
            .expect("test target should be supported")
            .binary_name;
        store
            .ensure_managed_connection_record(None)
            .expect("seed managed connection");

        let v1_archive = test_zip_asset(binary_name, b"managed binary v1");
        let v1_asset =
            current_target_test_asset_version("test-1", "fixture-v1.zip", sha256_hex(&v1_archive));
        let lock = acquire_managed_install_lock(root).expect("lock fixture v1 install");
        install_managed_archive_locked_with_context(
            root,
            v1_asset,
            &v1_archive,
            "bundled",
            false,
            None,
            Some(store),
            None,
        )
        .expect("install fixture v1");
        drop(lock);

        let v2_archive = test_zip_asset(binary_name, b"managed binary v2");
        let v2_asset =
            current_target_test_asset_version("test-2", "fixture-v2.zip", sha256_hex(&v2_archive));
        let lock = acquire_managed_install_lock(root).expect("lock fixture v2 install");
        install_managed_archive_locked_with_context(
            root,
            v2_asset,
            &v2_archive,
            "github_release",
            false,
            None,
            Some(store),
            None,
        )
        .expect("upgrade fixture to v2");
        drop(lock);

        assert_managed_runtime_pointer_versions(root, "test-2", "test-1");
        assert!(managed_runtime_has_verified_previous_at(root));
    }

    fn assert_managed_runtime_pointer_versions(root: &Path, current: &str, previous: &str) {
        assert_eq!(
            managed_metadata(&managed_current_path(root))
                .expect("read current metadata")
                .expect("current metadata")
                .version,
            current
        );
        assert_eq!(
            managed_metadata(&managed_previous_path(root))
                .expect("read previous metadata")
                .expect("previous metadata")
                .version,
            previous
        );
    }

    fn test_tar_gz_asset<F>(build: F) -> Vec<u8>
    where
        F: FnOnce(&mut tar::Builder<&mut GzEncoder<Vec<u8>>>) -> std::io::Result<()>,
    {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut builder = tar::Builder::new(&mut encoder);
            build(&mut builder).unwrap();
            builder.finish().unwrap();
        }
        encoder.finish().unwrap()
    }

    fn append_tar_file(
        builder: &mut tar::Builder<&mut GzEncoder<Vec<u8>>>,
        name: &str,
        content: &[u8],
    ) -> std::io::Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, name, content)
    }

    #[test]
    fn managed_archive_install_is_atomic_and_rollback_keeps_old_content() {
        let temp = tempfile::tempdir().unwrap();
        let binary = b"new managed binary";
        let archive_name = "multica-cli-test-windows-amd64.zip";
        let archive = test_zip_asset("multica.exe", binary);
        let asset = test_asset("multica.exe", archive_name, sha256_hex(&archive));
        let lock = acquire_managed_install_lock(temp.path()).unwrap();
        let status =
            install_managed_archive_locked(temp.path(), asset, &archive, "bundled", false).unwrap();
        drop(lock);
        assert_eq!(status.install_state, "ready");
        assert!(status.sha256_verified);
        let current = managed_metadata(&managed_current_path(temp.path()))
            .unwrap()
            .unwrap();
        assert_eq!(
            fs::read(managed_executable_for_metadata(temp.path(), &current)).unwrap(),
            binary
        );
        assert!(
            managed_metadata(&managed_previous_path(temp.path()))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn managed_install_failure_persists_only_a_redacted_code_and_keeps_ready_runtime() {
        let temp = tempfile::tempdir().expect("temporary managed runtime fixture");
        let binary_name = managed_runtime_asset()
            .expect("test target should be supported")
            .binary_name;
        let archive = test_zip_asset(binary_name, b"verified managed binary");
        let asset = current_target_test_asset_version(
            "test-ready",
            "fixture-ready.zip",
            sha256_hex(&archive),
        );
        let lock = acquire_managed_install_lock(temp.path()).unwrap();
        install_managed_archive_locked(temp.path(), asset, &archive, "bundled", false)
            .expect("install ready runtime fixture");
        drop(lock);

        let raw_error = anyhow!("managed_runtime_download_timeout")
            .context("https://release.invalid/file?token=must-not-persist");
        let code = stable_managed_install_error_code(&raw_error);
        assert_eq!(code, "managed_runtime_download_timeout");
        let status = managed_install_failure_status_at(temp.path(), &code);
        assert_eq!(status.install_state, "ready");
        assert_eq!(status.install_phase.as_deref(), Some("failed"));
        assert_eq!(
            status.last_install_error_code.as_deref(),
            Some("managed_runtime_download_timeout")
        );

        let persisted = fs::read_to_string(managed_install_failure_path(temp.path()))
            .expect("read persisted install failure");
        assert!(persisted.contains("managed_runtime_download_timeout"));
        assert!(!persisted.contains("must-not-persist"));
        assert!(!persisted.contains("https://"));

        let after_restart = managed_runtime_status_at(temp.path())
            .expect("persisted failure should survive a status reload");
        assert_eq!(after_restart.install_state, "ready");
        assert_eq!(
            after_restart.last_install_error_code.as_deref(),
            Some("managed_runtime_download_timeout")
        );

        clear_managed_install_failure(temp.path()).expect("clear failure after success");
        let cleared = managed_runtime_status_at(temp.path()).expect("load cleared status");
        assert_eq!(cleared.install_state, "ready");
        assert!(cleared.last_install_error_code.is_none());
    }

    #[test]
    fn managed_upgrade_validates_previous_rebinds_sidecar_and_rolls_back() {
        let temp = tempfile::tempdir().expect("temporary managed runtime fixture");
        let runtime_root = temp.path().join("runtime");
        let binary_name = managed_runtime_asset()
            .expect("test target should be supported")
            .binary_name;
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );

        let original_working_dir = temp.path().join("custom-profile");
        fs::create_dir_all(&original_working_dir).expect("create custom profile fixture");
        let original_args = vec![
            "--profile".to_string(),
            "user-preserved-profile".to_string(),
            "daemon".to_string(),
            "start".to_string(),
        ];
        let mut managed = config("https://managed.multica.example/custom");
        managed.connection_id = MANAGED_RUNTIME_CONNECTION_ID.to_string();
        managed.display_name = "User managed runtime".to_string();
        managed.api_prefix = Some("preserve-api-prefix".to_string());
        managed.workspace_id = Some("preserve-workspace".to_string());
        managed.workspace_slug = Some("preserve-slug".to_string());
        managed.token_env_var = Some("MULTICA_TEST_TOKEN".to_string());
        managed.enabled = false;
        managed.allow_insecure_lan_http = true;
        managed.sidecar = Some(MulticaSidecarConfig {
            executable: std::env::current_exe()
                .expect("test executable")
                .to_string_lossy()
                .to_string(),
            working_dir: Some(original_working_dir.to_string_lossy().to_string()),
            args: original_args.clone(),
            auto_start: false,
        });
        store
            .save_managed_connection(managed)
            .expect("seed managed connection");

        let mut manual = config("https://manual.multica.example");
        manual.connection_id = "manual-preserved".to_string();
        manual.display_name = "Manual preserved".to_string();
        manual.sidecar = Some(MulticaSidecarConfig {
            executable: std::env::current_exe()
                .expect("test executable")
                .to_string_lossy()
                .to_string(),
            working_dir: Some(temp.path().to_string_lossy().to_string()),
            args: vec!["--profile".to_string(), "manual-profile".to_string()],
            auto_start: false,
        });
        let manual_before = serde_json::to_value(
            store
                .save_connection(manual)
                .expect("seed ordinary connection"),
        )
        .expect("serialize ordinary connection");

        let v1_archive = test_zip_asset(binary_name, b"managed binary v1");
        let v1_asset =
            current_target_test_asset_version("test-1", "fixture-v1.zip", sha256_hex(&v1_archive));
        let lock = acquire_managed_install_lock(&runtime_root).unwrap();
        install_managed_archive_locked_with_context(
            &runtime_root,
            v1_asset,
            &v1_archive,
            "bundled",
            false,
            None,
            Some(&store),
            None,
        )
        .expect("install fixture v1");
        drop(lock);

        let v2_archive = test_zip_asset(binary_name, b"managed binary v2");
        let v2_asset =
            current_target_test_asset_version("test-2", "fixture-v2.zip", sha256_hex(&v2_archive));
        let lock = acquire_managed_install_lock(&runtime_root).unwrap();
        let upgraded = install_managed_archive_locked_with_context(
            &runtime_root,
            v2_asset,
            &v2_archive,
            "github_release",
            false,
            None,
            Some(&store),
            None,
        )
        .expect("upgrade fixture to v2");
        drop(lock);
        assert_eq!(upgraded.installed_version.as_deref(), Some("test-2"));
        assert_eq!(upgraded.previous_version.as_deref(), Some("test-1"));

        let previous = managed_metadata(&managed_previous_path(&runtime_root))
            .unwrap()
            .expect("v1 previous metadata");
        assert_eq!(previous.version, "test-1");
        assert!(managed_metadata_is_verified(&runtime_root, &previous));
        assert!(managed_metadata_matches_asset(&previous, v1_asset));
        assert!(
            !managed_metadata_matches_asset(&previous, v2_asset),
            "historical metadata may verify itself but must not satisfy the current install allowlist"
        );

        let managed_after_upgrade = store
            .load_connections()
            .unwrap()
            .into_iter()
            .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
            .expect("managed connection after upgrade");
        let v2_current = managed_metadata(&managed_current_path(&runtime_root))
            .unwrap()
            .expect("v2 current metadata");
        let v2_executable = managed_executable_for_metadata(&runtime_root, &v2_current);
        let expected_managed_working_dir =
            managed_profile_directory().to_string_lossy().to_string();
        let rebound = managed_after_upgrade.sidecar.as_ref().unwrap();
        assert_eq!(Path::new(&rebound.executable), v2_executable);
        assert_eq!(
            rebound.working_dir.as_deref(),
            Some(expected_managed_working_dir.as_str())
        );
        assert_eq!(rebound.args, managed_runtime_args());
        assert!(rebound.auto_start);
        assert_eq!(managed_after_upgrade.display_name, "User managed runtime");
        assert_eq!(
            managed_after_upgrade.server_url,
            "https://managed.multica.example/custom"
        );
        assert!(!managed_after_upgrade.enabled);
        assert_eq!(
            managed_after_upgrade.api_prefix.as_deref(),
            Some("preserve-api-prefix")
        );
        assert_eq!(
            managed_after_upgrade.workspace_id.as_deref(),
            Some("preserve-workspace")
        );
        assert_eq!(
            managed_after_upgrade.workspace_slug.as_deref(),
            Some("preserve-slug")
        );
        assert_eq!(
            managed_after_upgrade.token_env_var.as_deref(),
            Some("MULTICA_TEST_TOKEN")
        );
        assert!(managed_after_upgrade.allow_insecure_lan_http);

        let rolled_back = rollback_managed_runtime_at_with_store(&runtime_root, &store)
            .expect("explicit rollback to v1");
        assert_eq!(rolled_back.install_state, "ready");
        assert_eq!(rolled_back.installed_version.as_deref(), Some("test-1"));
        assert_eq!(rolled_back.previous_version.as_deref(), Some("test-2"));
        let rolled_back_current = managed_metadata(&managed_current_path(&runtime_root))
            .unwrap()
            .expect("rolled back current metadata");
        assert!(managed_metadata_is_verified(
            &runtime_root,
            &rolled_back_current
        ));
        let managed_after_rollback = store
            .load_connections()
            .unwrap()
            .into_iter()
            .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
            .expect("managed connection after rollback");
        let rolled_back_sidecar = managed_after_rollback.sidecar.as_ref().unwrap();
        assert_eq!(
            Path::new(&rolled_back_sidecar.executable),
            managed_executable_for_metadata(&runtime_root, &rolled_back_current)
        );
        assert_eq!(
            rolled_back_sidecar.working_dir.as_deref(),
            Some(expected_managed_working_dir.as_str())
        );
        assert_eq!(rolled_back_sidecar.args, managed_runtime_args());
        assert!(rolled_back_sidecar.auto_start);

        fs::write(
            managed_current_path(&runtime_root),
            b"{ damaged current metadata",
        )
        .expect("damage current metadata fixture");
        let lock = acquire_managed_install_lock(&runtime_root).unwrap();
        let reinstalled = install_managed_archive_locked_with_context(
            &runtime_root,
            v2_asset,
            &v2_archive,
            "github_release",
            false,
            None,
            Some(&store),
            None,
        )
        .expect("verified reinstall replaces damaged current metadata");
        drop(lock);
        assert_eq!(reinstalled.install_state, "ready");
        assert_eq!(reinstalled.installed_version.as_deref(), Some("test-2"));
        assert_eq!(reinstalled.previous_version.as_deref(), Some("test-2"));
        let reinstalled_current = managed_metadata(&managed_current_path(&runtime_root))
            .unwrap()
            .expect("reinstalled current metadata");
        let retained_previous = managed_metadata(&managed_previous_path(&runtime_root))
            .unwrap()
            .expect("damaged current must not replace the verified previous metadata");
        assert_eq!(retained_previous.version, "test-2");
        assert!(managed_metadata_is_verified(
            &runtime_root,
            &retained_previous
        ));
        let managed_after_reinstall = store
            .load_connections()
            .unwrap()
            .into_iter()
            .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
            .expect("managed connection after reinstall");
        let reinstalled_sidecar = managed_after_reinstall.sidecar.as_ref().unwrap();
        assert_eq!(
            Path::new(&reinstalled_sidecar.executable),
            managed_executable_for_metadata(&runtime_root, &reinstalled_current)
        );
        assert_eq!(
            reinstalled_sidecar.working_dir.as_deref(),
            Some(expected_managed_working_dir.as_str())
        );
        assert_eq!(reinstalled_sidecar.args, managed_runtime_args());
        assert!(reinstalled_sidecar.auto_start);
        assert_eq!(managed_after_reinstall.display_name, "User managed runtime");
        assert_eq!(
            managed_after_reinstall.server_url,
            "https://managed.multica.example/custom"
        );
        assert!(!managed_after_reinstall.enabled);

        let manual_after = store
            .load_connections()
            .unwrap()
            .into_iter()
            .find(|connection| connection.connection_id == "manual-preserved")
            .expect("ordinary connection after managed operations");
        assert_eq!(
            serde_json::to_value(manual_after).expect("serialize ordinary connection"),
            manual_before
        );
    }

    #[test]
    fn managed_archive_cancel_preserves_current_and_cleans_staging() {
        let temp = tempfile::tempdir().unwrap();
        let old_archive = test_zip_asset("multica.exe", b"old managed binary");
        let old_asset = test_asset("multica.exe", "fixture-old.zip", sha256_hex(&old_archive));
        let lock = acquire_managed_install_lock(temp.path()).unwrap();
        install_managed_archive_locked(temp.path(), old_asset, &old_archive, "bundled", false)
            .unwrap();
        drop(lock);

        let new_archive = test_zip_asset("multica.exe", b"new managed binary");
        let new_asset = test_asset("multica.exe", "fixture-new.zip", sha256_hex(&new_archive));
        let cancelled = AtomicBool::new(true);
        let lock = acquire_managed_install_lock(temp.path()).unwrap();
        let error = install_managed_archive_locked_with_cancel(
            temp.path(),
            new_asset,
            &new_archive,
            "bundled",
            false,
            Some(&cancelled),
        )
        .unwrap_err();
        drop(lock);
        assert_eq!(error.to_string(), "managed_runtime_install_cancelled");

        let current = managed_metadata(&managed_current_path(temp.path()))
            .unwrap()
            .unwrap();
        assert_eq!(current.asset_name, old_asset.asset_name);
        assert_eq!(
            fs::read(managed_executable_for_metadata(temp.path(), &current)).unwrap(),
            b"old managed binary"
        );
        let entries = fs::read_dir(temp.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(entries.iter().all(|name| !name.starts_with(".staging-")));
    }

    #[test]
    fn managed_archive_late_cancel_restores_both_activation_pointers() {
        let temp = tempfile::tempdir().unwrap();
        let binary_name = managed_runtime_asset()
            .expect("test target should be supported")
            .binary_name;
        let v1_archive = test_zip_asset(binary_name, b"managed binary v1");
        let v1_asset =
            current_target_test_asset_version("test-1", "fixture-v1.zip", sha256_hex(&v1_archive));
        let lock = acquire_managed_install_lock(temp.path()).unwrap();
        install_managed_archive_locked(temp.path(), v1_asset, &v1_archive, "bundled", false)
            .unwrap();
        drop(lock);

        let v2_archive = test_zip_asset(binary_name, b"managed binary v2");
        let v2_asset =
            current_target_test_asset_version("test-2", "fixture-v2.zip", sha256_hex(&v2_archive));
        let lock = acquire_managed_install_lock(temp.path()).unwrap();
        install_managed_archive_locked(temp.path(), v2_asset, &v2_archive, "bundled", false)
            .unwrap();
        drop(lock);

        let current_path = managed_current_path(temp.path());
        let previous_path = managed_previous_path(temp.path());
        let current_before = fs::read(&current_path).unwrap();
        let previous_before = fs::read(&previous_path).unwrap();
        let cancelled = AtomicBool::new(false);
        let hook_ran = AtomicBool::new(false);
        let cancel_after_previous_write = || {
            assert_eq!(
                fs::read(&previous_path).unwrap(),
                current_before,
                "hook must run after current has been copied to previous"
            );
            hook_ran.store(true, Ordering::Release);
            cancelled.store(true, Ordering::Release);
        };

        let v3_archive = test_zip_asset(binary_name, b"managed binary v3");
        let v3_asset =
            current_target_test_asset_version("test-3", "fixture-v3.zip", sha256_hex(&v3_archive));
        let lock = acquire_managed_install_lock(temp.path()).unwrap();
        let error = install_managed_archive_locked_with_context(
            temp.path(),
            v3_asset,
            &v3_archive,
            "bundled",
            false,
            Some(&cancelled),
            None,
            Some(&cancel_after_previous_write),
        )
        .unwrap_err();
        drop(lock);

        assert!(hook_ran.load(Ordering::Acquire));
        assert_eq!(error.to_string(), "managed_runtime_install_cancelled");
        assert_eq!(fs::read(&current_path).unwrap(), current_before);
        assert_eq!(fs::read(&previous_path).unwrap(), previous_before);
        let entries = fs::read_dir(managed_versions_dir(temp.path()))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(entries.iter().all(|name| !name.starts_with("test-3-")));
        assert!(
            fs::read_dir(temp.path())
                .unwrap()
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().starts_with(".staging-"))
        );
    }

    #[tokio::test]
    async fn managed_download_wait_cancels_pending_send_or_chunk_promptly() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancelled);
        let setter = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1)).await;
            signal.store(true, Ordering::Release);
        });
        let started = Instant::now();
        let result = await_managed_download(
            std::future::pending::<Result<(), reqwest::Error>>(),
            Some(cancelled.as_ref()),
        )
        .await;
        setter.await.unwrap();
        assert!(matches!(result, Err(ManagedDownloadWaitError::Cancelled)));
        assert!(
            started.elapsed() < Duration::from_millis(250),
            "cancellation should not wait for the request timeout"
        );
    }

    #[tokio::test]
    async fn managed_downloader_retries_only_transient_failures() {
        let transient_attempts = Arc::new(AtomicU64::new(0));
        let transient_counter = Arc::clone(&transient_attempts);
        let recovered = retry_managed_download_operation(
            move || {
                let attempt = transient_counter.fetch_add(1, Ordering::SeqCst) + 1;
                std::future::ready(if attempt < 3 {
                    Err(anyhow!("managed_runtime_http_retryable"))
                } else {
                    Ok("downloaded")
                })
            },
            None,
            &[Duration::ZERO, Duration::ZERO],
        )
        .await
        .expect("third transient attempt should succeed");
        assert_eq!(recovered, "downloaded");
        assert_eq!(transient_attempts.load(Ordering::SeqCst), 3);

        let permanent_attempts = Arc::new(AtomicU64::new(0));
        let permanent_counter = Arc::clone(&permanent_attempts);
        let error = retry_managed_download_operation::<(), _, _>(
            move || {
                permanent_counter.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Err(anyhow!("managed_runtime_http_status")))
            },
            None,
            &[Duration::ZERO, Duration::ZERO],
        )
        .await
        .expect_err("permanent HTTP failures must not retry");
        assert_eq!(error.to_string(), "managed_runtime_http_status");
        assert_eq!(permanent_attempts.load(Ordering::SeqCst), 1);

        for status in [408, 425, 429, 500, 502, 503, 504] {
            assert!(managed_http_status_is_retryable(
                StatusCode::from_u16(status).unwrap()
            ));
        }
        for status in [400, 401, 403, 404, 409, 422, 501] {
            assert!(!managed_http_status_is_retryable(
                StatusCode::from_u16(status).unwrap()
            ));
        }
    }

    #[test]
    fn managed_archive_rejects_escape_duplicate_and_unexpected_members() {
        let asset = test_asset("multica.exe", "fixture.zip", "0".repeat(64));
        let mut archive = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut archive);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("../evil", options).unwrap();
            writer.write_all(b"bad").unwrap();
            writer.finish().unwrap();
        }
        assert!(extract_managed_binary(&archive.into_inner(), asset).is_err());

        let mut archive = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut archive);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("unexpected.bin", options).unwrap();
            writer.write_all(b"bad").unwrap();
            writer.finish().unwrap();
        }
        assert!(extract_managed_binary(&archive.into_inner(), asset).is_err());
    }

    #[test]
    fn managed_tar_archive_rejects_non_utf8_paths() {
        let archive = test_tar_gz_asset(|builder| {
            let mut header = tar::Header::new_gnu();
            header.as_mut_bytes()[..100].fill(0);
            header.as_mut_bytes()[..4].copy_from_slice(b"bad\xff");
            header.set_size(1);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&mut header, &[0u8][..])
        });
        let asset = test_asset("multica", "fixture.tar.gz", "0".repeat(64));
        let error = extract_managed_binary(&archive, asset).unwrap_err();
        assert_eq!(error.to_string(), "managed_runtime_archive_path_invalid");
    }

    #[test]
    fn managed_tar_archive_rejects_cumulative_uncompressed_size_over_limit() {
        let archive = test_tar_gz_asset(|builder| {
            let first_size = MANAGED_RUNTIME_MAX_BINARY_BYTES;
            let first = vec![0u8; first_size];
            append_tar_file(builder, "LICENSE", &first)?;
            append_tar_file(builder, "multica", &[0u8])
        });
        let asset = test_asset("multica", "fixture.tar.gz", "0".repeat(64));
        let error = extract_managed_binary(&archive, asset).unwrap_err();
        assert_eq!(error.to_string(), "managed_runtime_archive_too_large");
    }

    #[test]
    fn validation_preserves_server_url_and_rejects_credentials() {
        let original = "https://Example.com/base/";
        let value = config(original);
        validate_connection(&value).unwrap();
        assert_eq!(value.server_url, original);
        assert!(validate_connection(&config("https://user:pass@example.com")).is_err());
        assert!(validate_connection(&config("https://example.com/api?token=secret")).is_err());
        assert!(validate_connection(&config("https://example.com/api#fragment")).is_err());
        let mut invalid_prefix = config("https://example.com");
        invalid_prefix.api_prefix = Some("v1?token=secret".to_string());
        assert!(validate_connection(&invalid_prefix).is_err());
        invalid_prefix.api_prefix = Some("v1/../admin".to_string());
        assert!(validate_connection(&invalid_prefix).is_err());
        invalid_prefix.api_prefix = Some("https:%2f%2fevil".to_string());
        assert!(validate_connection(&invalid_prefix).is_err());
        assert!(validate_connection(&config("file:///tmp/multica")).is_err());
        assert!(validate_connection(&config("http://example.com")).is_err());
        assert!(validate_connection(&config("http://127.0.0.1:43123")).is_ok());

        #[cfg(windows)]
        {
            assert!(!is_local_windows_path(Path::new(
                r"\\server\share\multica.exe"
            )));
            assert!(!is_local_windows_path(Path::new(
                r"\\?\UNC\server\share\multica.exe"
            )));
            assert!(is_local_windows_path(Path::new(r"C:\\tools\\multica.exe")));
        }

        let mut confirmed_lan = config("http://192.168.8.10:43123/api");
        assert!(validate_connection(&confirmed_lan).is_err());
        confirmed_lan.allow_insecure_lan_http = true;
        assert!(validate_connection(&confirmed_lan).is_ok());

        let mut confirmed_public_http = config("http://example.com:43123/api");
        confirmed_public_http.allow_insecure_lan_http = true;
        assert!(validate_connection(&confirmed_public_http).is_err());
    }

    #[test]
    fn saving_sidecar_requires_verified_local_file_and_directory() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );

        let mut missing_executable = config("https://multica.example");
        missing_executable.sidecar = Some(MulticaSidecarConfig {
            executable: temp
                .path()
                .join("missing.exe")
                .to_string_lossy()
                .to_string(),
            working_dir: Some(temp.path().to_string_lossy().to_string()),
            args: vec!["--profile".to_string(), "sandbox".to_string()],
            auto_start: false,
        });
        assert!(store.save_connection(missing_executable).is_err());

        let mut verified = config("https://verified.multica.example");
        verified.sidecar = Some(MulticaSidecarConfig {
            executable: std::env::current_exe()
                .expect("test executable")
                .to_string_lossy()
                .to_string(),
            working_dir: Some(temp.path().to_string_lossy().to_string()),
            args: vec!["--profile=sandbox".to_string()],
            auto_start: false,
        });
        assert!(store.save_connection(verified).is_ok());
    }

    #[test]
    fn sidecar_validation_rejects_shebang_scripts() {
        let temp = tempfile::tempdir().expect("temporary Multica directory");
        let script = temp.path().join("sidecar.bin");
        fs::write(&script, b"#!/bin/sh\nexit 0\n").expect("write script fixture");
        assert!(is_shebang_script(&script).expect("read script prefix"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&script)
                .expect("script metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script, permissions).expect("make script executable");
        }

        let mut value = config("https://script.multica.example");
        value.sidecar = Some(MulticaSidecarConfig {
            executable: script.to_string_lossy().to_string(),
            working_dir: Some(temp.path().to_string_lossy().to_string()),
            args: vec!["--profile=sandbox".to_string()],
            auto_start: false,
        });
        assert!(
            validate_connection(&value).is_err(),
            "shebang wrappers must not be accepted as native sidecars"
        );
    }

    #[test]
    fn saving_connection_rejects_live_sidecar_runtime_changes() {
        let temp = tempfile::tempdir().expect("temporary Multica directory");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );
        let connection_id = format!("live-edit-{}", now_ms());

        // Use a short-lived native process as the ownership fixture. The
        // executable path is read back from the OS so the test exercises the
        // same image-identity check used by stop/delete in production.
        #[cfg(windows)]
        let child = Command::new("cmd.exe")
            .args(["/C", "ping", "127.0.0.1", "-n", "6"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn Windows sidecar fixture");
        #[cfg(unix)]
        let mut child = Command::new("sleep")
            .arg("5")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn Unix sidecar fixture");

        let executable = (0..20)
            .find_map(|_| {
                let path = query_process_executable_path(child.id());
                if path.is_none() {
                    std::thread::sleep(Duration::from_millis(10));
                }
                path
            })
            .expect("sidecar fixture executable path");
        let mut initial = config("https://live-edit.multica.example");
        initial.connection_id = connection_id.clone();
        initial.sidecar = Some(MulticaSidecarConfig {
            executable: executable.to_string_lossy().to_string(),
            working_dir: Some(temp.path().to_string_lossy().to_string()),
            args: vec!["--profile=sandbox".to_string()],
            auto_start: false,
        });
        store
            .save_connection(initial.clone())
            .expect("save initial connection");

        sidecars().lock().expect("sidecar registry").insert(
            connection_id.clone(),
            SidecarProcess {
                connection_id: connection_id.clone(),
                child,
                executable,
                #[cfg(windows)]
                managed_job: None,
                _managed_lifecycle: None,
                started_at_ms: now_ms(),
                exited_at_ms: None,
                exit_code: None,
                last_health: None,
            },
        );

        let mut replacement = initial.clone();
        replacement.sidecar.as_mut().unwrap().args = vec!["--profile=replacement".to_string()];
        let replacement_error = store
            .save_connection(replacement)
            .expect_err("replacing a live sidecar must be rejected");

        let mut disabled = initial.clone();
        disabled.enabled = false;
        let disabled_error = store
            .save_connection(disabled)
            .expect_err("disabling a live sidecar must be rejected");

        let input_error = store
            .save_connection_input(MulticaConnectionInput {
                connection_id: Some(connection_id.clone()),
                display_name: "clear-sidecar".to_string(),
                server_url: String::new(),
                api_prefix: None,
                workspace_id: None,
                workspace_slug: None,
                token_env_var: None,
                enabled: true,
                allow_insecure_lan_http: false,
                sidecar: Some(None),
            })
            .expect_err("clearing a live sidecar must be rejected");

        let mut metadata_only = initial;
        metadata_only.display_name = "metadata-only-edit".to_string();
        store
            .save_connection(metadata_only)
            .expect("metadata edits remain allowed while sidecar runs");

        let mut tracked = sidecars()
            .lock()
            .expect("sidecar registry")
            .remove(&connection_id)
            .expect("tracked sidecar fixture");
        let _ = tracked.child.kill();
        let _ = tracked.child.wait();

        assert!(replacement_error.to_string().contains("请先停止"));
        assert!(disabled_error.to_string().contains("请先停止"));
        assert!(input_error.to_string().contains("请先停止"));
    }

    #[test]
    fn validation_rejects_ccp_fixed_ports() {
        for url in [
            "http://127.0.0.1:57321/v1",
            "https://example.com:57331/v1",
            "http://localhost:57320",
            "https://example.com:9230/api",
        ] {
            assert!(
                validate_connection(&config(url)).is_err(),
                "CCP fixed port must be rejected: {url}"
            );
        }
        // The normal HTTP/HTTPS defaults are unrelated to CCP's loopback
        // ports and remain valid when the host/protocol rules permit them.
        assert!(validate_connection(&config("https://example.com:443")).is_ok());
        assert!(validate_connection(&config("http://127.0.0.1")).is_ok());
    }

    #[test]
    fn sidecar_port_guard_rejects_composite_listener_forms() {
        for value in [
            "127.0.0.1:57321",
            "--listen=127.0.0.1:57321",
            "--bind-address=127.0.0.1:57321",
            "[::1]:57331",
            "host=57321",
            "http://localhost:57320/health",
            "--port=057321",
            "--listen=127.0.0.1:057331/path",
            "custom=009230",
        ] {
            assert!(
                contains_forbidden_port_in_composite(value),
                "reserved port should be detected in {value}"
            );
        }
        assert!(!contains_forbidden_port_in_composite("worker-57321"));
        assert!(!contains_forbidden_port_in_composite(
            "profile=worker-57321"
        ));
        assert!(!contains_forbidden_port_in_composite(
            "profile=057321-worker"
        ));

        for flag in ["--listen", "--bind", "--bind-address", "--health-port"] {
            assert!(is_sidecar_port_flag(flag), "{flag} should be port-like");
        }
    }

    #[test]
    fn managed_cli_args_validate_at_execution_boundary_without_rewriting_url() {
        let credentials = managed_cli_context_from_server_url(
            "https://user:pass@example.com",
            &["auth", "status"],
        )
        .unwrap_err()
        .to_string();
        assert_eq!(
            credentials,
            "managed_runtime_server_url_credentials_forbidden"
        );
        assert!(!credentials.contains("pass"));

        let query = managed_cli_context_from_server_url(
            "https://example.com/api?token=secret",
            &["auth", "status"],
        )
        .unwrap_err()
        .to_string();
        assert_eq!(query, "managed_runtime_server_url_query_forbidden");
        assert!(!query.contains("secret"));

        let whitespace =
            managed_cli_context_from_server_url(" https://Example.com/base/ ", &["auth", "status"])
                .unwrap_err()
                .to_string();
        assert_eq!(whitespace, "managed_runtime_server_url_invalid");

        for value in [
            "ftp://example.com",
            "http://example.com",
            "https://example.com:57321",
            "http://127.0.0.1:57331",
        ] {
            assert!(
                managed_cli_context_from_server_url(value, &["auth", "status"]).is_err(),
                "unsafe managed URL should be rejected: {value}"
            );
        }

        for value in [
            "https://Example.com/base/",
            "http://127.0.0.1:19587",
            "http://192.168.1.20:8080",
            "http://runtime.local:8080",
        ] {
            let (args, child_server_url) =
                managed_cli_context_from_server_url(value, &["auth", "status"])
                    .expect("valid URL should produce a managed child context");
            assert_eq!(child_server_url, value);
            assert_eq!(
                args,
                vec!["--profile", MANAGED_RUNTIME_PROFILE, "auth", "status"]
            );
            assert!(!args.iter().any(|argument| argument.contains(value)));
        }
    }

    #[test]
    fn managed_daemon_contract_is_pinned_and_macos_bundle_resource_is_discoverable() {
        assert_eq!(
            managed_runtime_args(),
            vec![
                "--profile",
                MANAGED_RUNTIME_PROFILE,
                "daemon",
                "start",
                "--foreground",
                "--no-auto-update",
                "--no-auto-reload",
            ]
        );

        let asset = managed_asset_for_target("aarch64-apple-darwin").unwrap();
        let executable = PathBuf::from("bundle")
            .join("Claude Codex Pro.app")
            .join("Contents")
            .join("MacOS")
            .join("claude-codex-pro");
        let expected = PathBuf::from("bundle")
            .join("Claude Codex Pro.app")
            .join("Contents")
            .join("Resources")
            .join("multica")
            .join(asset.asset_name);
        assert!(
            managed_resource_candidates_from_executable(asset, &executable).contains(&expected)
        );
    }

    #[test]
    fn sidecar_validation_rejects_reserved_ports_even_with_custom_flags() {
        let temp = tempfile::tempdir().expect("temporary Multica directory");
        let executable = std::env::current_exe().expect("test executable");
        let sidecar = MulticaSidecarConfig {
            executable: executable.to_string_lossy().to_string(),
            working_dir: Some(temp.path().to_string_lossy().to_string()),
            args: vec![
                "--profile=sandbox".to_string(),
                "--custom-listener=127.0.0.1:57321".to_string(),
            ],
            auto_start: false,
        };
        assert!(validate_sidecar_config(&sidecar).is_err());
    }

    #[test]
    fn canonical_key_normalizes_default_port_and_api_prefix() {
        let mut explicit_default = config("https://Example.com:443/base/");
        explicit_default.api_prefix = Some("/V1/".to_string());
        let mut implicit_default = config("https://example.com/base");
        implicit_default.api_prefix = Some("v1".to_string());
        assert_eq!(
            canonical_connection_key(&explicit_default),
            canonical_connection_key(&implicit_default)
        );

        let mut different_api_prefix = config("https://example.com/base");
        different_api_prefix.api_prefix = Some("v2".to_string());
        assert_ne!(
            canonical_connection_key(&implicit_default),
            canonical_connection_key(&different_api_prefix)
        );
    }

    #[test]
    fn canonical_key_preserves_case_sensitive_url_path() {
        let mut upper_path = config("https://Example.com/Base/");
        upper_path.api_prefix = Some("/V1/".to_string());
        let mut lower_path = config("https://example.com/base");
        lower_path.api_prefix = Some("v1".to_string());
        assert_ne!(
            canonical_connection_key(&upper_path),
            canonical_connection_key(&lower_path),
            "URL path case must remain significant while host/prefix case is normalized"
        );
    }

    #[test]
    fn sidecar_auto_start_defaults_off_without_implicit_launch() {
        let sidecar = serde_json::from_value::<MulticaSidecarConfig>(serde_json::json!({
            "executable": "multica-daemon"
        }))
        .unwrap();
        assert!(!sidecar.auto_start);

        let mut value = config("https://example.com");
        value.sidecar = Some(sidecar);
        let view = value.view();
        assert!(view.sidecar_configured);
        assert!(!view.sidecar_auto_start);
        // Startup is intentionally explicit through start_sidecar; merely
        // loading or viewing a connection never spawns a child process.
    }

    #[test]
    fn sidecar_auto_start_requires_enabled_connection_and_explicit_flag() {
        let mut value = config("https://example.com");
        value.sidecar = Some(MulticaSidecarConfig {
            executable: "multica-daemon".to_string(),
            working_dir: None,
            args: Vec::new(),
            auto_start: true,
        });
        assert!(should_auto_start_sidecar(&value));
        value.enabled = false;
        assert!(!should_auto_start_sidecar(&value));
        value.enabled = true;
        value.sidecar.as_mut().unwrap().auto_start = false;
        assert!(!should_auto_start_sidecar(&value));
        value.sidecar = None;
        assert!(!should_auto_start_sidecar(&value));
    }

    #[test]
    fn disabled_connection_cannot_start_sidecar() {
        let mut value = config("https://example.com");
        value.enabled = false;
        value.sidecar = Some(MulticaSidecarConfig {
            executable: "multica-daemon".to_string(),
            working_dir: None,
            args: Vec::new(),
            auto_start: false,
        });
        let error = sidecar_config_for_start(&value).unwrap_err();
        assert!(error.to_string().contains("已停用"));
    }

    #[test]
    fn save_connection_rejects_duplicate_when_editing_existing_id() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );

        let mut first = config("https://Example.com:443/base/");
        first.connection_id = "first".to_string();
        first.display_name = "Original name".to_string();
        first.api_prefix = Some("/V1/".to_string());
        let saved_first = store.save_connection(first).unwrap();
        assert_eq!(saved_first.server_url, "https://Example.com:443/base/");

        let mut second = config("https://other.example/api");
        second.connection_id = "second".to_string();
        second.api_prefix = Some("v2".to_string());
        store.save_connection(second).unwrap();

        // Editing an existing record must still reject a canonical collision
        // with another record, even though the connection ID already exists.
        let mut conflicting_edit = config("https://example.com/base");
        conflicting_edit.connection_id = "second".to_string();
        conflicting_edit.api_prefix = Some("v1".to_string());
        let error = store.save_connection(conflicting_edit).unwrap_err();
        assert!(error.to_string().contains("Multica 连接已存在"));

        let connections = store.load_connections().unwrap();
        assert_eq!(connections.len(), 2);
        let unchanged_second = connections
            .iter()
            .find(|connection| connection.connection_id == "second")
            .expect("second connection remains");
        assert_eq!(unchanged_second.server_url, "https://other.example/api");
        assert_eq!(unchanged_second.api_prefix.as_deref(), Some("v2"));

        // The current record is excluded from the duplicate check, so a
        // case/trailing-slash-only edit of that record remains an update and
        // preserves the exact URL entered by the user.
        let mut equivalent_edit = config("https://EXAMPLE.com/base");
        equivalent_edit.connection_id = "first".to_string();
        equivalent_edit.display_name = "Renamed".to_string();
        equivalent_edit.api_prefix = Some("v1".to_string());
        let updated_first = store.save_connection(equivalent_edit).unwrap();
        assert_eq!(updated_first.display_name, "Renamed");
        assert_eq!(updated_first.server_url, "https://EXAMPLE.com/base");
        assert_eq!(store.load_connections().unwrap().len(), 2);
    }

    #[test]
    fn managed_connection_is_hidden_and_reserved_from_manual_crud() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );

        let mut managed = config("https://managed.multica.example");
        managed.connection_id = MANAGED_RUNTIME_CONNECTION_ID.to_string();
        managed.display_name = "Keep this managed name".to_string();
        let saved_managed = store
            .save_managed_connection(managed.clone())
            .expect("dedicated managed save");
        assert_eq!(saved_managed.display_name, "Keep this managed name");
        assert_eq!(saved_managed.server_url, "https://managed.multica.example");

        let mut manual = config("https://manual.multica.example");
        manual.connection_id = "manual".to_string();
        store
            .save_connection(manual)
            .expect("save manual connection");

        let listed = manual_connection_views(&store.load_connections().unwrap());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].connection_id, "manual");

        let mut manual_edit = managed;
        manual_edit.display_name = "must not overwrite".to_string();
        let save_error = store.save_connection(manual_edit).unwrap_err();
        let delete_error = store
            .delete_connection(MANAGED_RUNTIME_CONNECTION_ID)
            .unwrap_err();
        assert_eq!(save_error.to_string(), MANAGED_CONNECTION_RESERVED_ERROR);
        assert_eq!(delete_error.to_string(), MANAGED_CONNECTION_RESERVED_ERROR);

        let persisted = store.load_connections().unwrap();
        let persisted_managed = persisted
            .iter()
            .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
            .expect("managed connection remains");
        assert_eq!(persisted_managed.display_name, "Keep this managed name");
        assert_eq!(
            persisted_managed.server_url,
            "https://managed.multica.example"
        );
    }

    #[test]
    fn managed_connection_update_keeps_empty_values_and_noneditable_fields_verbatim() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );
        let mut managed = config("https://managed.multica.example/original");
        managed.connection_id = MANAGED_RUNTIME_CONNECTION_ID.to_string();
        managed.display_name = "Original managed name".to_string();
        managed.api_prefix = Some("runtime-v1".to_string());
        managed.workspace_id = Some("workspace-unchanged".to_string());
        managed.workspace_slug = Some("slug-unchanged".to_string());
        managed.sidecar = Some(MulticaSidecarConfig {
            executable: std::env::current_exe()
                .expect("test executable")
                .to_string_lossy()
                .to_string(),
            working_dir: Some(temp.path().to_string_lossy().to_string()),
            args: vec!["--profile".to_string(), MANAGED_RUNTIME_PROFILE.to_string()],
            auto_start: true,
        });
        let original_sidecar = managed.sidecar.clone();
        store
            .save_managed_connection(managed)
            .expect("save managed connection");

        let saved = store
            .update_managed_connection_values(String::new(), String::new(), false)
            .expect("managed update accepts blank values");
        assert_eq!(saved.display_name, "");
        assert_eq!(saved.server_url, "");
        assert!(!saved.enabled);
        assert_eq!(saved.api_prefix.as_deref(), Some("runtime-v1"));
        assert_eq!(saved.workspace_id.as_deref(), Some("workspace-unchanged"));
        assert_eq!(saved.workspace_slug.as_deref(), Some("slug-unchanged"));
        assert_eq!(saved.sidecar, original_sidecar);

        let persisted = store
            .load_connections()
            .expect("load managed connection")
            .into_iter()
            .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
            .expect("managed connection exists");
        assert_eq!(persisted.display_name, "");
        assert_eq!(persisted.server_url, "");
        assert_eq!(persisted.sidecar, original_sidecar);
    }

    #[test]
    fn managed_enabled_update_never_replays_stale_name_or_url_values() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = Arc::new(MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        ));
        store
            .ensure_managed_connection_record(None)
            .expect("seed managed connection");

        let barrier = Arc::new(std::sync::Barrier::new(2));
        let editor_store = Arc::clone(&store);
        let editor_barrier = Arc::clone(&barrier);
        let editor = std::thread::spawn(move || {
            editor_barrier.wait();
            for _ in 0..32 {
                editor_store
                    .update_managed_connection_values(
                        "User edited name".to_string(),
                        "https://user-edited.multica.example/custom".to_string(),
                        true,
                    )
                    .expect("save managed user values");
            }
        });

        let toggle_store = Arc::clone(&store);
        let toggle_barrier = Arc::clone(&barrier);
        let toggler = std::thread::spawn(move || {
            toggle_barrier.wait();
            for enabled in [false, true].into_iter().cycle().take(32) {
                toggle_store
                    .update_managed_enabled(enabled)
                    .expect("toggle only managed enabled bit");
            }
        });

        editor.join().expect("managed editor worker");
        toggler.join().expect("managed toggle worker");
        let saved = store
            .load_connections()
            .expect("load managed connection")
            .into_iter()
            .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
            .expect("managed connection exists");
        assert_eq!(saved.display_name, "User edited name");
        assert_eq!(
            saved.server_url,
            "https://user-edited.multica.example/custom"
        );
    }

    #[test]
    fn managed_connection_initialization_is_atomic_and_idempotent() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = Arc::new(MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        ));
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                store
                    .ensure_managed_connection_record(None)
                    .expect("atomic managed initialization")
            }));
        }
        for worker in workers {
            assert_eq!(
                worker.join().expect("managed initializer").connection_id,
                MANAGED_RUNTIME_CONNECTION_ID
            );
        }

        let connections = store.load_connections().expect("load connections");
        assert_eq!(
            connections
                .iter()
                .filter(|connection| { connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID })
                .count(),
            1
        );
    }

    #[test]
    fn managed_connection_atomic_rebind_preserves_concurrent_user_values() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = Arc::new(MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        ));
        store
            .ensure_managed_connection_record(None)
            .expect("seed managed connection");
        let expected_sidecar = MulticaSidecarConfig {
            executable: "managed-runtime-v2.exe".to_string(),
            working_dir: Some("ccp-managed-profile".to_string()),
            args: managed_runtime_args(),
            auto_start: true,
        };
        let barrier = Arc::new(std::sync::Barrier::new(2));

        let rebinding_store = Arc::clone(&store);
        let rebinding_barrier = Arc::clone(&barrier);
        let rebinding_sidecar = expected_sidecar.clone();
        let rebinder = std::thread::spawn(move || {
            rebinding_barrier.wait();
            for _ in 0..16 {
                rebinding_store
                    .ensure_managed_connection_record(Some(rebinding_sidecar.clone()))
                    .expect("atomic managed rebind");
            }
        });

        let editing_store = Arc::clone(&store);
        let editing_barrier = Arc::clone(&barrier);
        let editor = std::thread::spawn(move || {
            editing_barrier.wait();
            for _ in 0..16 {
                editing_store
                    .update_managed_connection_values(
                        "User value".to_string(),
                        "".to_string(),
                        false,
                    )
                    .expect("concurrent user save");
            }
        });
        rebinder.join().expect("managed rebind worker");
        editor.join().expect("managed editor worker");

        let saved = store
            .load_connections()
            .expect("load managed connection")
            .into_iter()
            .find(|connection| connection.connection_id == MANAGED_RUNTIME_CONNECTION_ID)
            .expect("managed connection remains");
        assert_eq!(saved.display_name, "User value");
        assert_eq!(saved.server_url, "");
        assert!(!saved.enabled);
        assert_eq!(saved.sidecar, Some(expected_sidecar));
    }

    #[test]
    fn managed_connection_file_lock_serializes_independent_handles() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let path = temp.path().join("connections.json");
        let first = MulticaConnectionFileLock::acquire(&path).expect("first connection lock");
        let (attempted_tx, attempted_rx) = std::sync::mpsc::channel();
        let (acquired_tx, acquired_rx) = std::sync::mpsc::channel();
        let worker_path = path.clone();
        let worker = std::thread::spawn(move || {
            attempted_tx.send(()).unwrap();
            let second = MulticaConnectionFileLock::acquire(&worker_path)
                .expect("second connection lock after release");
            acquired_tx.send(()).unwrap();
            drop(second);
        });

        attempted_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second lock attempted");
        assert!(
            acquired_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "the second handle must not enter while the first owns the file lock"
        );
        drop(first);
        acquired_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("second lock acquired after release");
        worker.join().expect("connection lock worker");
    }

    fn test_managed_owner(pid: u32) -> ManagedRuntimeOwner {
        ManagedRuntimeOwner {
            pid,
            version: "test-1".to_string(),
            executable: fs::canonicalize(std::env::current_exe().expect("test executable"))
                .expect("canonical test executable")
                .to_string_lossy()
                .to_string(),
            profile: MANAGED_RUNTIME_PROFILE.to_string(),
            connection_id: MANAGED_RUNTIME_CONNECTION_ID.to_string(),
            started_at_ms: 1,
        }
    }

    #[test]
    fn managed_lifecycle_lock_has_one_owner_and_removes_matching_record_on_drop() {
        let temp = tempfile::tempdir().expect("temporary managed runtime root");
        let owner = test_managed_owner(std::process::id());
        let lease = acquire_managed_lifecycle_lock_at(temp.path())
            .expect("first lifecycle lock")
            .activate(owner.clone())
            .expect("persist managed owner");

        assert_eq!(
            read_managed_owner(&managed_owner_path(temp.path()))
                .expect("read managed owner")
                .as_ref(),
            Some(&owner)
        );
        let error = acquire_managed_lifecycle_lock_at(temp.path())
            .expect_err("second manager must not own the same daemon lifecycle");
        assert_eq!(error.to_string(), "managed_runtime_owned_by_other_manager");

        drop(lease);
        assert!(!managed_owner_path(temp.path()).exists());
        drop(
            acquire_managed_lifecycle_lock_at(temp.path())
                .expect("lifecycle lock is reusable after owner cleanup"),
        );
    }

    #[test]
    fn managed_lifecycle_lock_cleans_a_dead_stale_owner() {
        let temp = tempfile::tempdir().expect("temporary managed runtime root");
        let owner = test_managed_owner(u32::MAX);
        crate::settings::atomic_write(
            &managed_owner_path(temp.path()),
            &serde_json::to_vec_pretty(&owner).expect("serialize stale owner"),
        )
        .expect("write stale owner");

        let lock = acquire_managed_lifecycle_lock_at(temp.path())
            .expect("dead stale owner must not block the next manager");
        assert!(
            !managed_owner_path(temp.path()).exists(),
            "the stale private owner record must be removed while the lifecycle lock is held"
        );
        drop(lock);
    }

    #[test]
    fn stale_managed_owner_with_matching_process_is_terminated_and_removed() {
        let temp = tempfile::tempdir().expect("temporary managed runtime root");
        let owner = test_managed_owner(4242);
        let expected_executable = PathBuf::from(&owner.executable);
        crate::settings::atomic_write(
            &managed_owner_path(temp.path()),
            &serde_json::to_vec_pretty(&owner).expect("serialize stale owner"),
        )
        .expect("write stale owner");

        let running = std::cell::Cell::new(true);
        let terminated_pid = std::cell::Cell::new(None);
        cleanup_stale_managed_owner_with(
            temp.path(),
            |pid| {
                assert_eq!(pid, owner.pid);
                running.get()
            },
            |pid| {
                assert_eq!(pid, owner.pid);
                Some(expected_executable.clone())
            },
            |pid| {
                terminated_pid.set(Some(pid));
                running.set(false);
                Ok(())
            },
        )
        .expect("matching stale process should be reclaimed");

        assert_eq!(terminated_pid.get(), Some(owner.pid));
        assert!(
            !managed_owner_path(temp.path()).exists(),
            "the owner record must be removed after process exit is confirmed"
        );
    }

    #[test]
    fn stale_managed_owner_with_foreign_process_only_removes_record() {
        let temp = tempfile::tempdir().expect("temporary managed runtime root");
        let owner = test_managed_owner(4243);
        crate::settings::atomic_write(
            &managed_owner_path(temp.path()),
            &serde_json::to_vec_pretty(&owner).expect("serialize stale owner"),
        )
        .expect("write stale owner");

        let terminate_calls = std::cell::Cell::new(0usize);
        cleanup_stale_managed_owner_with(
            temp.path(),
            |pid| {
                assert_eq!(pid, owner.pid);
                true
            },
            |pid| {
                assert_eq!(pid, owner.pid);
                Some(temp.path().join("foreign-multica.exe"))
            },
            |_| {
                terminate_calls.set(terminate_calls.get() + 1);
                Ok(())
            },
        )
        .expect("a reused PID must not block stale record cleanup");

        assert_eq!(
            terminate_calls.get(),
            0,
            "a foreign process must not be killed"
        );
        assert!(
            !managed_owner_path(temp.path()).exists(),
            "the stale record for a reused PID must be removed"
        );
    }

    #[test]
    fn stale_managed_owner_with_unverifiable_process_refuses_termination() {
        let temp = tempfile::tempdir().expect("temporary managed runtime root");
        let owner = test_managed_owner(4244);
        crate::settings::atomic_write(
            &managed_owner_path(temp.path()),
            &serde_json::to_vec_pretty(&owner).expect("serialize stale owner"),
        )
        .expect("write stale owner");

        let terminate_calls = std::cell::Cell::new(0usize);
        let error = cleanup_stale_managed_owner_with(
            temp.path(),
            |pid| {
                assert_eq!(pid, owner.pid);
                true
            },
            |pid| {
                assert_eq!(pid, owner.pid);
                None
            },
            |_| {
                terminate_calls.set(terminate_calls.get() + 1);
                Ok(())
            },
        )
        .expect_err("an unverifiable live PID must block reclamation");

        assert_eq!(
            error.to_string(),
            "managed_runtime_owner_process_unverified"
        );
        assert_eq!(
            terminate_calls.get(),
            0,
            "an unverified process must not be killed"
        );
        assert_eq!(
            read_managed_owner(&managed_owner_path(temp.path()))
                .expect("read retained owner record")
                .as_ref(),
            Some(&owner),
            "the owner record must remain so the unsafe transition stays blocked"
        );
    }

    #[test]
    fn managed_sidecar_completion_preserves_saved_user_values() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );
        let mut managed = config("https://managed.multica.example/custom/path");
        managed.connection_id = MANAGED_RUNTIME_CONNECTION_ID.to_string();
        managed.display_name = "User selected name".to_string();
        managed.enabled = false;
        managed.api_prefix = Some("preserve-prefix".to_string());
        managed.sidecar = None;
        store
            .save_managed_connection(managed)
            .expect("save managed connection without sidecar");

        let expected_sidecar = MulticaSidecarConfig {
            executable: "verified-managed-multica.exe".to_string(),
            working_dir: Some("ccp-managed-profile".to_string()),
            args: vec!["--profile".to_string(), MANAGED_RUNTIME_PROFILE.to_string()],
            auto_start: true,
        };
        let completed = store
            .attach_managed_sidecar_if_missing(expected_sidecar.clone())
            .expect("complete missing managed sidecar");
        assert_eq!(completed.display_name, "User selected name");
        assert_eq!(
            completed.server_url,
            "https://managed.multica.example/custom/path"
        );
        assert!(!completed.enabled);
        assert_eq!(completed.api_prefix.as_deref(), Some("preserve-prefix"));
        assert_eq!(completed.sidecar, Some(expected_sidecar.clone()));

        let ignored_replacement = MulticaSidecarConfig {
            executable: "must-not-replace.exe".to_string(),
            working_dir: None,
            args: Vec::new(),
            auto_start: false,
        };
        let repeated = store
            .attach_managed_sidecar_if_missing(ignored_replacement)
            .expect("existing sidecar remains untouched");
        assert_eq!(repeated.sidecar, Some(expected_sidecar));
    }

    #[test]
    fn managed_sidecar_rebind_repairs_full_contract_when_executable_is_unchanged() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );
        let executable = std::env::current_exe().expect("test executable");
        let mut managed = config("https://managed.multica.example/custom");
        managed.connection_id = MANAGED_RUNTIME_CONNECTION_ID.to_string();
        managed.display_name = "Preserved managed name".to_string();
        managed.enabled = false;
        managed.sidecar = Some(MulticaSidecarConfig {
            executable: executable.to_string_lossy().to_string(),
            working_dir: Some(temp.path().to_string_lossy().to_string()),
            args: vec!["--profile".to_string(), MANAGED_RUNTIME_PROFILE.to_string()],
            auto_start: false,
        });
        store
            .save_managed_connection(managed)
            .expect("seed stale managed sidecar contract");

        let expected = managed_sidecar_for_executable(executable.clone())
            .expect("build fixed managed sidecar contract");
        let rebound = store
            .rebind_managed_sidecar_contract_if_present(executable)
            .expect("repair stale managed sidecar contract")
            .expect("managed connection remains");

        assert_eq!(rebound.sidecar, Some(expected));
        assert_eq!(rebound.display_name, "Preserved managed name");
        assert_eq!(rebound.server_url, "https://managed.multica.example/custom");
        assert!(!rebound.enabled);
    }

    #[test]
    fn missing_managed_binary_digest_has_stable_current_and_previous_diagnostics() {
        let Some(asset) = managed_runtime_asset() else {
            return;
        };
        let temp = tempfile::tempdir().expect("temporary managed runtime root");
        fs::create_dir_all(temp.path()).expect("create runtime root");
        let legacy_metadata = serde_json::json!({
            "version": asset.version,
            "targetTriple": asset.target_triple,
            "assetName": asset.asset_name,
            "binaryName": asset.binary_name,
            "sha256": asset.expected_sha256,
            "assetSource": "bundled",
            "directoryName": format!("{}-legacy", asset.version),
            "updatedAtMs": 1,
        });
        fs::write(
            managed_current_path(temp.path()),
            serde_json::to_vec(&legacy_metadata).expect("serialize legacy current metadata"),
        )
        .expect("write legacy current metadata");
        let current = managed_runtime_status_at(temp.path()).expect("read current status");
        assert_eq!(current.install_state, "verification_failed");
        assert_eq!(
            current.last_install_error_code.as_deref(),
            Some("managed_runtime_metadata_binary_digest_missing")
        );
        assert_eq!(
            current.diagnostic.as_deref(),
            Some("managed_runtime_metadata_binary_digest_missing")
        );

        fs::write(
            managed_previous_path(temp.path()),
            serde_json::to_vec(&legacy_metadata).expect("serialize legacy previous metadata"),
        )
        .expect("write legacy previous metadata");
        let rollback_error = rollback_managed_runtime_at(temp.path())
            .expect_err("legacy previous metadata must not be trusted for rollback");
        assert_eq!(
            rollback_error.to_string(),
            "managed_runtime_metadata_binary_digest_missing"
        );
    }

    #[test]
    fn blank_server_url_only_preserves_an_existing_connection() {
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );
        let mut existing = config("https://multica.example/original/path");
        existing.connection_id = "existing".to_string();
        existing.display_name = "Existing".to_string();
        store.save_connection(existing).unwrap();

        let mut direct_edit = config("");
        direct_edit.connection_id = "existing".to_string();
        direct_edit.display_name = "Direct edit".to_string();
        let direct_saved = store.save_connection(direct_edit).unwrap();
        assert_eq!(
            direct_saved.server_url,
            "https://multica.example/original/path"
        );

        let direct_new_error = store.save_connection(config("")).unwrap_err();
        assert!(direct_new_error.to_string().contains("服务地址不能为空"));

        let input_saved = store
            .save_connection_input(MulticaConnectionInput {
                connection_id: Some("existing".to_string()),
                display_name: "Input edit".to_string(),
                server_url: String::new(),
                api_prefix: None,
                workspace_id: None,
                workspace_slug: None,
                token_env_var: None,
                enabled: true,
                allow_insecure_lan_http: false,
                sidecar: None,
            })
            .unwrap();
        assert_eq!(
            input_saved.server_url,
            "https://multica.example/original/path"
        );

        let input_new_error = store
            .save_connection_input(MulticaConnectionInput {
                connection_id: None,
                display_name: "New".to_string(),
                server_url: String::new(),
                api_prefix: None,
                workspace_id: None,
                workspace_slug: None,
                token_env_var: None,
                enabled: true,
                allow_insecure_lan_http: false,
                sidecar: None,
            })
            .unwrap_err();
        assert!(input_new_error.to_string().contains("服务地址不能为空"));
    }

    #[test]
    fn token_reference_is_validated_and_view_is_redacted() {
        let mut value = config("https://example.com");
        value.token_env_var = Some("CUSTOM_REFERENCE".to_string());
        validate_connection(&value).unwrap();
        let json = serde_json::to_string(&value.view()).unwrap();
        assert!(!json.contains("secret"));
        assert!(!json.contains("CUSTOM_REFERENCE"));
        value.server_url =
            "https://multica.example/private/path?access_token=secret#fragment".to_string();
        let json = serde_json::to_string(&value.view()).unwrap();
        assert!(json.contains("https://multica.example/"));
        assert!(!json.contains("private"));
        assert!(!json.contains("access_token"));
        assert!(!json.contains("secret"));
        assert!(!json.contains("fragment"));
        value.sidecar = Some(MulticaSidecarConfig {
            executable: "C:\\secrets\\multica.exe".to_string(),
            working_dir: Some("C:\\private".to_string()),
            args: vec!["--mode".to_string(), "daemon".to_string()],
            auto_start: true,
        });
        let json = serde_json::to_string(&value.view()).unwrap();
        assert!(!json.contains("C:\\\\secrets"));
        assert!(!json.contains("C:\\\\private"));
        assert!(json.contains("multica.exe"));
        assert!(json.contains("sidecarConfigured"));
        value.token_env_var = Some("token-value".to_string());
        assert!(validate_connection(&value).is_err());
    }

    #[test]
    fn parser_keeps_unknown_status_as_unknown_and_caps_fields() {
        let value = serde_json::json!({
            "items": [{
                "id": "task-1",
                "status": "brand_new",
                "title": "hello",
                "message": "must not be returned",
                "updatedAtMs": 1234,
                "unexpected": true
            }]
        });
        let items = parse_collection(&value);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, "unknown");
        assert_eq!(
            items[0].diagnostic.as_deref(),
            Some("unknown_status:brand_new")
        );
        assert!(serde_json::to_string(&items[0]).unwrap().contains("task-1"));
        assert!(
            !serde_json::to_string(&items[0])
                .unwrap()
                .contains("must not be returned")
        );
    }

    #[test]
    fn parser_maps_multica_runtime_fields_and_prefers_last_seen() {
        let value = serde_json::json!({
            "id": "runtime-1",
            "name": "Desktop runtime",
            "runtime_mode": "codex",
            "provider": "openai",
            "status": "running",
            "last_seen_at": "2026-08-29T12:34:56Z",
            "updated_at": "2020-01-01T00:00:00Z"
        });
        let item = parse_item(&value).expect("runtime record should parse");
        assert_eq!(item.id, "runtime-1");
        assert_eq!(item.runtime_type.as_deref(), Some("codex"));
        assert_eq!(item.provider.as_deref(), Some("openai"));
        assert_eq!(item.status, "running");
        assert_eq!(item.updated_at_ms, Some(1_788_006_896_000));
    }

    #[test]
    fn parser_redacts_upstream_free_text_before_building_the_dto() {
        let value = serde_json::json!({
            "id": "task-1",
            "name": "worker",
            "title": "request https://multica.example/private/task?token=url-secret",
            "status": "future-state",
            "error": "Authorization: Bearer bearer-secret Cookie: session=cookie-secret; role=user api_key=api-secret at C:\\Users\\Alice\\private.txt and /home/alice/work/task.log"
        });
        let item = parse_item(&value).expect("record should parse");
        let encoded = serde_json::to_string(&item).expect("dto should serialize");

        assert_eq!(item.id, "task-1");
        assert_eq!(item.status, "unknown");
        for secret in [
            "url-secret",
            "bearer-secret",
            "cookie-secret",
            "api-secret",
            "C:\\Users\\Alice",
            "/home/alice/work",
        ] {
            assert!(!encoded.contains(secret), "secret leaked: {secret}");
        }
        assert!(encoded.contains("https://multica.example"));
        assert!(encoded.contains("[path]"));
        assert!(
            item.diagnostic
                .as_deref()
                .is_some_and(|diagnostic| diagnostic.contains("future-state"))
        );
    }

    #[test]
    fn public_text_redaction_covers_headers_urls_and_absolute_paths() {
        let input = "https://multica.example/private/path?access_token=url-secret \
            Authorization: Bearer bearer-secret \
            Cookie: session=cookie-secret; role=role-secret; theme=dark \
            API Key: api-secret \
            C:\\Users\\Alice Smith\\secret.txt /srv/multica/private.log";
        let output = sanitize_public_text(input);
        for secret in [
            "url-secret",
            "bearer-secret",
            "cookie-secret",
            "role-secret",
            "api-secret",
            "C:\\Users\\Alice Smith",
            "Smith\\secret.txt",
            "/srv/multica/private.log",
        ] {
            assert!(
                !output.contains(secret),
                "secret leaked: {secret}; output={output}"
            );
        }
        assert!(output.contains("https://multica.example"));
        assert!(!output.contains("/private/path"));
        assert!(!output.contains("access_token"));
        assert!(output.contains("[redacted]"));
        assert!(output.contains("[path]"));
    }

    #[test]
    fn public_text_redaction_handles_case_and_malformed_header_values() {
        let inputs = [
            "COOKIE: bare-cookie-secret",
            "authorization=Bearer bare-auth-secret",
            "X-API-Key: x-api-secret",
            "apiKey: camel-api-secret",
            "Bearer standalone-secret",
            "sk-standalone-secret",
            "\\\\server\\share\\private.txt",
            "'C:\\Users\\Alice Smith\\private.txt'",
            "/absolute/private/path",
        ];
        for input in inputs {
            let output = sanitize_public_text(input);
            for secret in [
                "bare-cookie-secret",
                "bare-auth-secret",
                "x-api-secret",
                "camel-api-secret",
                "standalone-secret",
                "sk-standalone-secret",
                "server\\share\\private.txt",
                "Alice Smith\\private.txt",
                "/absolute/private/path",
            ] {
                assert!(
                    !output.contains(secret),
                    "secret leaked for {input}: {secret}"
                );
            }
        }
    }

    #[test]
    fn public_text_redaction_handles_json_headers_and_env_assignments() {
        let input = r#"headers={"Authorization":"Bearer json-auth-secret","Cookie":"session=json-cookie-secret; role=user","X-API-Key":"json-api-secret"} OPENAI_API_KEY=env-api-secret MY_ACCESS_TOKEN=env-token-secret /secret"#;
        let output = sanitize_public_text(input);
        for secret in [
            "json-auth-secret",
            "json-cookie-secret",
            "json-api-secret",
            "env-api-secret",
            "env-token-secret",
            "/secret",
        ] {
            assert!(
                !output.contains(secret),
                "secret leaked: {secret}; output={output}"
            );
        }
        assert!(output.contains("[redacted]"));
        assert!(output.contains("[path]"));
    }

    #[test]
    fn sidecar_is_running_requires_a_record() {
        let id = format!("missing-{}", now_ms());
        assert!(!sidecar_is_running(&id));
    }

    #[test]
    fn sidecar_environment_filters_sensitive_names() {
        for name in [
            "MULTICA_TOKEN",
            "OPENAI_API_KEY",
            "SERVICE_SECRET",
            "AUTHORIZATION",
            "SESSION_COOKIE",
            "DATABASE_PASSWORD",
            "AWS_ACCESS_KEY_ID",
        ] {
            assert!(is_sensitive_environment_name(name), "{name}");
        }
        for name in ["PATH", "HOME", "SystemRoot", "TEMP", "MULTICA_MODE"] {
            assert!(!is_sensitive_environment_name(name), "{name}");
        }
    }

    #[test]
    fn sidecar_environment_never_copies_protected_token_reference() {
        let entries = scrub_sidecar_environment(vec![
            ("PATH".to_string(), "path".to_string()),
            ("CUSTOM_REFERENCE".to_string(), "secret-value".to_string()),
            ("CUSTOM_TOKEN_REF".to_string(), "secret-value".to_string()),
            ("OPENAI_API_KEY".to_string(), "other-secret".to_string()),
            ("MULTICA_TOKEN".to_string(), "stale-value".to_string()),
        ]);
        assert!(entries.contains(&("PATH".to_string(), "path".to_string())));
        assert!(!entries.iter().any(|(name, _)| name == "CUSTOM_REFERENCE"));
        assert!(!entries.iter().any(|(name, _)| name == "CUSTOM_TOKEN_REF"));
        assert!(!entries.iter().any(|(name, _)| name == "OPENAI_API_KEY"));
        assert!(!entries.iter().any(|(name, _)| name == "MULTICA_TOKEN"));
        assert!(!entries.iter().any(|(_, value)| value.contains("secret")));
    }

    #[test]
    fn managed_codex_runtime_environment_pins_desktop_binary_and_path_order() {
        let temp = tempfile::tempdir().unwrap();
        let executable_name = if cfg!(windows) { "codex.exe" } else { "codex" };
        let executable = temp.path().join("desktop").join(executable_name);
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"codex fixture").unwrap();
        let inherited_dir = temp.path().join("inherited");
        fs::create_dir_all(&inherited_dir).unwrap();
        let inherited = std::env::join_paths([inherited_dir.clone()]).unwrap();

        let (resolved, path) =
            managed_codex_runtime_environment_from(&executable, Some(&inherited)).unwrap();
        let entries = std::env::split_paths(&path).collect::<Vec<_>>();

        assert_eq!(resolved, fs::canonicalize(&executable).unwrap());
        assert_eq!(
            entries.first().map(|entry| entry.as_path()),
            resolved.parent()
        );
        assert!(entries.iter().any(|entry| entry == &inherited_dir));
    }

    #[test]
    fn stale_snapshot_preserves_previous_collections() {
        let previous = MulticaRuntimeSnapshot {
            source_connection_id: "test".to_string(),
            fetched_at_ms: 123,
            stale: false,
            runtimes: vec![MulticaRuntimeItem {
                id: "runtime-1".to_string(),
                status: "running".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let stale = stale_snapshot("test", "snapshot_timeout", Some(previous.clone()));
        assert!(stale.stale);
        assert_eq!(stale.fetched_at_ms, previous.fetched_at_ms);
        assert_eq!(stale.runtimes[0].id, "runtime-1");
        assert_eq!(stale.diagnostic.as_deref(), Some("snapshot_timeout"));
    }

    #[test]
    fn endpoint_url_does_not_use_ccp_proxy_ports() {
        let value = config("https://multica.example/base/");
        let url = endpoint_url(&value, "health").unwrap();
        let text = url.to_string();
        assert!(text.ends_with("/base/health"));
        for port in FORBIDDEN_PORTS {
            assert!(!text.contains(port));
        }
    }

    #[test]
    fn endpoint_url_discards_saved_and_endpoint_query_fragments() {
        let value = config("https://multica.example/base/?saved=must-not-leak#saved-fragment");
        let url = endpoint_url(&value, "api/runtimes?page=7#endpoint-fragment").unwrap();
        assert_eq!(url.path(), "/base/api/runtimes");
        assert!(url.query().is_none());
        assert!(url.fragment().is_none());

        let bounded =
            collection_endpoint_url(&value, "api/runtimes?page=7#endpoint-fragment").unwrap();
        assert_eq!(bounded.path(), "/base/api/runtimes");
        assert_eq!(bounded.fragment(), None);
        assert_eq!(
            bounded
                .query_pairs()
                .find(|(key, _)| key == "limit")
                .map(|(_, value)| value.into_owned()),
            Some(MAX_COLLECTION_ITEMS.to_string())
        );
    }

    #[test]
    fn daemon_profile_args_and_health_ports_are_deterministic() {
        let equals = vec!["--profile=staging".to_string()];
        assert_eq!(validated_sidecar_profile(&equals).unwrap(), "staging");

        let equals_with_padding = vec!["--profile=  staging  ".to_string()];
        assert_eq!(
            validated_sidecar_profile(&equals_with_padding).unwrap(),
            "staging"
        );

        let separated = vec![
            "--mode".to_string(),
            "daemon".to_string(),
            "--profile".to_string(),
            "staging".to_string(),
        ];
        assert_eq!(validated_sidecar_profile(&separated).unwrap(), "staging");
        assert!(validated_sidecar_profile(&[]).is_err());
        assert!(validated_sidecar_profile(&["--profile=default".to_string()]).is_err());
        assert!(validated_sidecar_profile(&["--profile=stage name".to_string()]).is_err());
        for value in ["../default", r"profiles\default", "C:default"] {
            assert!(
                validated_sidecar_profile(&[format!("--profile={value}")]).is_err(),
                "profile path escape must be rejected: {value}"
            );
        }

        assert_eq!(
            daemon_health_port_for_profile("staging"),
            19514
                + 1
                + (b's' as u16
                    + b't' as u16
                    + b'a' as u16
                    + b'g' as u16
                    + b'i' as u16
                    + b'n' as u16
                    + b'g' as u16)
                    % 1000
        );
        assert_ne!(
            daemon_health_port_for_profile("staging"),
            DEFAULT_DAEMON_HEALTH_PORT
        );
    }

    #[test]
    fn daemon_health_status_maps_readiness_and_identity() {
        let healthy = daemon_status_from_probe(
            DaemonHealthProbe {
                result: DaemonHealthProbeResult::Response {
                    status: "running".to_string(),
                    pid: Some(42),
                    profile: Some("staging".to_string()),
                    version: Some("1.2.3".to_string()),
                    http_status: 200,
                },
                duration_ms: 7,
            },
            42,
            "staging",
            10,
        );
        assert_eq!(healthy.status, "healthy");
        assert_eq!(healthy.endpoint.as_deref(), Some("health"));
        assert_eq!(healthy.http_status, Some(200));
        assert_eq!(healthy.version.as_deref(), Some("1.2.3"));

        let missing_pid = daemon_status_from_probe(
            DaemonHealthProbe {
                result: DaemonHealthProbeResult::Response {
                    status: "starting".to_string(),
                    pid: None,
                    profile: None,
                    version: None,
                    http_status: 200,
                },
                duration_ms: 2,
            },
            42,
            "staging",
            10,
        );
        assert_eq!(missing_pid.status, "degraded");
        assert_eq!(
            missing_pid.diagnostic.as_deref(),
            Some("daemon_pid_missing")
        );

        let missing_profile = daemon_status_from_probe(
            DaemonHealthProbe {
                result: DaemonHealthProbeResult::Response {
                    status: "running".to_string(),
                    pid: Some(42),
                    profile: None,
                    version: None,
                    http_status: 200,
                },
                duration_ms: 2,
            },
            42,
            "staging",
            10,
        );
        assert_eq!(missing_profile.status, "degraded");
        assert_eq!(
            missing_profile.diagnostic.as_deref(),
            Some("daemon_profile_missing")
        );

        let starting = daemon_status_from_probe(
            DaemonHealthProbe {
                result: DaemonHealthProbeResult::Response {
                    status: "starting".to_string(),
                    pid: Some(42),
                    profile: Some("staging".to_string()),
                    version: None,
                    http_status: 200,
                },
                duration_ms: 2,
            },
            42,
            "staging",
            10,
        );
        assert_eq!(starting.status, "checking");
        assert_eq!(starting.diagnostic.as_deref(), Some("daemon_starting"));

        let unknown = daemon_status_from_probe(
            DaemonHealthProbe {
                result: DaemonHealthProbeResult::Response {
                    status: "future_state".to_string(),
                    pid: Some(42),
                    profile: Some("staging".to_string()),
                    version: None,
                    http_status: 200,
                },
                duration_ms: 1,
            },
            42,
            "staging",
            10,
        );
        assert_eq!(unknown.status, "invalid_response");
        assert_eq!(unknown.diagnostic.as_deref(), Some("unknown_daemon_status"));

        let pid_mismatch = daemon_status_from_probe(
            DaemonHealthProbe {
                result: DaemonHealthProbeResult::Response {
                    status: "healthy".to_string(),
                    pid: Some(99),
                    profile: Some("staging".to_string()),
                    version: None,
                    http_status: 200,
                },
                duration_ms: 1,
            },
            42,
            "staging",
            10,
        );
        assert_eq!(pid_mismatch.status, "degraded");
        assert_eq!(
            pid_mismatch.diagnostic.as_deref(),
            Some("daemon_pid_mismatch")
        );

        let profile_mismatch = daemon_status_from_probe(
            DaemonHealthProbe {
                result: DaemonHealthProbeResult::Response {
                    status: "running".to_string(),
                    pid: Some(42),
                    profile: Some("other".to_string()),
                    version: None,
                    http_status: 200,
                },
                duration_ms: 1,
            },
            42,
            "staging",
            10,
        );
        assert_eq!(profile_mismatch.status, "degraded");
        assert_eq!(
            profile_mismatch.diagnostic.as_deref(),
            Some("daemon_profile_mismatch")
        );
    }

    #[test]
    fn freshly_started_daemon_retries_only_transient_health_states() {
        for (state, expected) in [
            ("checking", true),
            ("unreachable", true),
            ("healthy", false),
            ("degraded", false),
            ("unauthorized", false),
            ("invalid_response", false),
            ("stopped", false),
        ] {
            assert_eq!(
                should_retry_daemon_after_start(&MulticaDaemonStatus {
                    status: state.to_string(),
                    ..Default::default()
                }),
                expected,
                "unexpected retry decision for {state}"
            );
        }

        let worst_case_ms = DAEMON_HEALTH_TIMEOUT.as_millis()
            * DAEMON_STARTUP_PROBE_ATTEMPTS as u128
            + DAEMON_STARTUP_PROBE_RETRY_DELAY.as_millis()
                * (DAEMON_STARTUP_PROBE_ATTEMPTS - 1) as u128;
        assert!(
            worst_case_ms <= HEALTH_TOTAL_TIMEOUT.as_millis(),
            "startup health retries must remain within the total health budget"
        );
    }

    #[tokio::test]
    async fn request_guard_cancels_superseded_operation() {
        let connection_id = format!("guard-{}", now_ms());
        let first = RequestGuard::begin("test", &connection_id);
        let first_task = tokio::spawn({
            let first = first.clone();
            async move {
                first
                    .run(
                        async {
                            tokio::time::sleep(Duration::from_secs(30)).await;
                        },
                        "test_request_superseded",
                    )
                    .await
            }
        });
        tokio::time::sleep(Duration::from_millis(1)).await;
        let second = RequestGuard::begin("test", &connection_id);
        let result = tokio::time::timeout(Duration::from_millis(100), first_task)
            .await
            .expect("superseded request should be cancelled promptly")
            .expect("request task should not panic");
        assert_eq!(result.unwrap_err().to_string(), "test_request_superseded");
        first.finish();
        second.finish();
    }

    #[tokio::test]
    async fn snapshot_concurrency_is_bounded() {
        let semaphore = snapshot_semaphore();
        let permits = semaphore
            .clone()
            .acquire_many_owned(SNAPSHOT_CONCURRENCY_LIMIT as u32)
            .await
            .expect("semaphore should provide the configured capacity");
        let blocked =
            tokio::time::timeout(Duration::from_millis(20), semaphore.clone().acquire_owned())
                .await;
        assert!(blocked.is_err(), "a fifth snapshot must wait for a permit");
        drop(permits);
        let _next_permit =
            tokio::time::timeout(Duration::from_millis(100), semaphore.acquire_owned())
                .await
                .expect("a released permit should unblock the next snapshot")
                .expect("semaphore should remain open");
    }

    #[test]
    fn stale_request_cannot_overwrite_new_snapshot() {
        let connection_id = format!("snapshot-generation-{}", now_ms());
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );
        let old = RequestGuard::begin("snapshot-generation", &connection_id);
        let new = RequestGuard::begin("snapshot-generation", &connection_id);
        let old_snapshot = MulticaRuntimeSnapshot {
            source_connection_id: connection_id.clone(),
            fetched_at_ms: 1,
            ..Default::default()
        };
        let new_snapshot = MulticaRuntimeSnapshot {
            source_connection_id: connection_id.clone(),
            fetched_at_ms: 2,
            ..Default::default()
        };
        assert!(
            !store
                .save_snapshot_if_current(&old_snapshot, &old)
                .expect("old snapshot check should succeed")
        );
        assert!(
            store
                .save_snapshot_if_current(&new_snapshot, &new)
                .expect("new snapshot should be committed")
        );
        assert_eq!(
            store
                .load_snapshot(&connection_id)
                .expect("snapshot should load")
                .expect("new snapshot should exist")
                .fetched_at_ms,
            2
        );
        old.finish();
        new.finish();
    }

    #[test]
    fn rfc3339_timestamps_normalize_to_milliseconds() {
        assert_eq!(
            parse_timestamp(&serde_json::json!("2026-08-29T12:34:56Z")),
            Some(1_788_006_896_000)
        );
    }

    #[tokio::test]
    async fn fake_server_health_probe_uses_read_only_endpoints_and_context_headers() {
        let server = FakeHttpServer::start(vec![
            FakeHttpResponse {
                status: 200,
                body: r#"{"status":"ok","version":"1.0.0"}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"{"status":"ready","version":"1.1.0"}"#,
            },
        ]);
        let mut value = config(&format!(
            "{}/base/?saved=query#saved-fragment",
            server.base_url
        ));
        value.workspace_id = Some("workspace-test".to_string());
        value.workspace_slug = Some("sandbox".to_string());

        let status = probe_server(&value).await;
        let requests = server.join();

        assert_eq!(status.status, "healthy");
        assert_eq!(status.endpoint.as_deref(), Some("readyz"));
        assert_eq!(status.http_status, Some(200));
        assert_eq!(status.version.as_deref(), Some("1.1.0"));
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /base/health HTTP/1.1"));
        assert!(requests[1].starts_with("GET /base/readyz HTTP/1.1"));
        for request in requests {
            let request_lower = request.to_ascii_lowercase();
            assert!(request_lower.contains("x-workspace-id: workspace-test"));
            assert!(request_lower.contains("x-workspace-slug: sandbox"));
            assert!(!request.contains("saved=query"));
            assert!(!request.contains("saved-fragment"));
        }
    }

    #[tokio::test]
    async fn fake_server_health_probe_classifies_auth_and_invalid_json() {
        let unauthorized = FakeHttpServer::start(vec![FakeHttpResponse {
            status: 401,
            body: r#"{"error":"token required"}"#,
        }]);
        let unauthorized_config = config(&unauthorized.base_url);
        let unauthorized_status = probe_server(&unauthorized_config).await;
        let unauthorized_requests = unauthorized.join();
        assert_eq!(unauthorized_status.status, "unauthorized");
        assert_eq!(
            unauthorized_status.diagnostic.as_deref(),
            Some("authentication_required")
        );
        assert_eq!(unauthorized_requests.len(), 1);

        let invalid = FakeHttpServer::start(vec![
            FakeHttpResponse {
                status: 200,
                body: r#"{"status":"ok","version":"1.0.0"}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: "not-json",
            },
        ]);
        let invalid_config = config(&invalid.base_url);
        let invalid_status = probe_server(&invalid_config).await;
        let invalid_requests = invalid.join();
        assert_eq!(invalid_status.status, "invalid_response");
        assert_eq!(invalid_status.diagnostic.as_deref(), Some("invalid_json"));
        assert_eq!(invalid_requests.len(), 2);
    }

    #[tokio::test]
    async fn fake_server_health_probe_rejects_success_without_status() {
        let missing_live_status = FakeHttpServer::start(vec![FakeHttpResponse {
            status: 200,
            body: r#"{"version":"1.0.0"}"#,
        }]);
        let missing_live = probe_server(&config(&missing_live_status.base_url)).await;
        let live_requests = missing_live_status.join();
        assert_eq!(missing_live.status, "invalid_response");
        assert_eq!(missing_live.endpoint.as_deref(), Some("health"));
        assert_eq!(missing_live.http_status, Some(200));
        assert_eq!(missing_live.diagnostic.as_deref(), Some("invalid_json"));
        assert_eq!(live_requests.len(), 1);

        let missing_readiness_status = FakeHttpServer::start(vec![
            FakeHttpResponse {
                status: 200,
                body: r#"{"status":"ok","version":"1.0.0"}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"{}"#,
            },
        ]);
        let missing_readiness = probe_server(&config(&missing_readiness_status.base_url)).await;
        let readiness_requests = missing_readiness_status.join();
        assert_eq!(missing_readiness.status, "invalid_response");
        assert_eq!(missing_readiness.endpoint.as_deref(), Some("readyz"));
        assert_eq!(missing_readiness.http_status, Some(200));
        assert_eq!(
            missing_readiness.diagnostic.as_deref(),
            Some("invalid_json")
        );
        assert_eq!(readiness_requests.len(), 2);

        let missing_healthz_status = FakeHttpServer::start(vec![
            FakeHttpResponse {
                status: 200,
                body: r#"{"status":"ok","version":"1.0.0"}"#,
            },
            FakeHttpResponse {
                status: 404,
                body: r#"{"status":"missing"}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"{"version":"1.0.0"}"#,
            },
        ]);
        let missing_healthz = probe_server(&config(&missing_healthz_status.base_url)).await;
        let healthz_requests = missing_healthz_status.join();
        assert_eq!(missing_healthz.status, "invalid_response");
        assert_eq!(missing_healthz.endpoint.as_deref(), Some("healthz"));
        assert_eq!(missing_healthz.http_status, Some(200));
        assert_eq!(missing_healthz.diagnostic.as_deref(), Some("invalid_json"));
        assert_eq!(healthz_requests.len(), 3);
    }

    #[tokio::test]
    async fn fake_server_snapshot_is_bounded_and_keeps_read_only_runtime_metadata() {
        let server = FakeHttpServer::start(vec![
            FakeHttpResponse {
                status: 200,
                body: r#"{"items":[{"id":"runtime-1","runtime_mode":"codex","provider":"openai","status":"running","last_seen_at":"2026-08-29T12:34:56Z"}]}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"{"results":[{"uuid":"agent-1","name":"Agent","status":"ready","updated_at_ms":1724934896000}]}"#,
            },
            FakeHttpResponse {
                status: 200,
                body: r#"[{"key":"task-1","title":"Task","status":"queued","updated_at":"2026-08-29T12:34:56Z"}]"#,
            },
        ]);
        let connection_id = format!("fake-snapshot-{}", now_ms());
        let mut value = config(&format!(
            "{}/base/?saved=query#saved-fragment",
            server.base_url
        ));
        value.connection_id = connection_id.clone();
        let temp = tempfile::tempdir().expect("temporary Multica store");
        let store = MulticaStore::new(
            temp.path().join("connections.json"),
            temp.path().join("snapshots.json"),
        );
        let guard = RequestGuard::begin("snapshot", &connection_id);
        let snapshot = get_snapshot_inner(connection_id.clone(), value, store, guard.clone()).await;
        guard.finish();
        let requests = server.join();
        let snapshot = snapshot.unwrap_or_else(|error| {
            let request_lines = requests
                .iter()
                .filter_map(|request| request.lines().next())
                .collect::<Vec<_>>();
            panic!("fake snapshot should load: {error:#}; observed {request_lines:?}")
        });

        assert!(!snapshot.stale);
        assert_eq!(snapshot.source_connection_id, connection_id);
        assert_eq!(snapshot.runtimes.len(), 1);
        assert_eq!(snapshot.runtimes[0].runtime_type.as_deref(), Some("codex"));
        assert_eq!(snapshot.runtimes[0].provider.as_deref(), Some("openai"));
        assert_eq!(snapshot.agents[0].id, "agent-1");
        assert_eq!(snapshot.tasks[0].id, "task-1");
        assert_eq!(requests.len(), 3);
        let request_lines = requests
            .iter()
            .filter_map(|request| request.lines().next())
            .collect::<Vec<_>>();
        for expected in [
            "GET /base/api/runtimes?limit=100 HTTP/1.1",
            "GET /base/api/agents?limit=100 HTTP/1.1",
            "GET /base/api/agent-task-snapshot?limit=100 HTTP/1.1",
        ] {
            assert!(
                request_lines.contains(&expected),
                "missing {expected}; observed {request_lines:?}"
            );
        }
        for request in requests {
            assert!(!request.contains("saved=query"));
            assert!(!request.contains("saved-fragment"));
        }
    }

    #[test]
    fn sidecar_lifecycle_log_is_hashed_bounded_and_schema_limited() {
        let temp = tempfile::tempdir().expect("temporary lifecycle log directory");
        let path = temp.path().join("sidecar-lifecycle.jsonl");

        let connection_id = "connection/with-secret-token";
        let first = SidecarLifecycleRecord {
            timestamp_ms: now_ms(),
            action: "started".to_string(),
            connection_id_hash: sidecar_connection_hash(connection_id),
            pid: Some(4242),
            started_at_ms: Some(100),
            exited_at_ms: None,
            exit_code: None,
            status: Some("checking".to_string()),
            diagnostic: Some("sidecar_spawn_failed".to_string()),
            endpoint: Some("health".to_string()),
            duration_ms: Some(4),
        };
        append_sidecar_lifecycle_record_at(&path, &first).expect("lifecycle record should write");
        let raw = fs::read_to_string(&path).expect("lifecycle record should be written");
        assert!(!raw.contains(connection_id));
        assert!(raw.contains(&sidecar_connection_hash(connection_id)));
        assert!(!raw.contains("Authorization"));
        assert!(!raw.contains("https://"));
        assert!(!raw.contains("stdout"));
        assert!(!raw.contains("stderr"));

        let record: Value = serde_json::from_str(raw.lines().next().unwrap()).unwrap();
        let object = record.as_object().unwrap();
        let allowed = [
            "timestampMs",
            "action",
            "connectionIdHash",
            "pid",
            "startedAtMs",
            "exitedAtMs",
            "exitCode",
            "status",
            "diagnostic",
            "endpoint",
            "durationMs",
        ];
        assert!(object.keys().all(|key| allowed.contains(&key.as_str())));
        assert_eq!(object["action"], "started");
        assert_eq!(object["status"], "checking");
        assert_eq!(object["endpoint"], "health");
        assert_eq!(object["durationMs"], 4);

        // Force a rotation with an oversized previous generation. Both the
        // active and retained files stay within the configured bound.
        fs::write(
            &path,
            vec![b'x'; MAX_SIDECAR_LIFECYCLE_LOG_BYTES as usize + 1],
        )
        .expect("seed oversized lifecycle log");
        let second = SidecarLifecycleRecord {
            timestamp_ms: now_ms(),
            action: "stopped".to_string(),
            connection_id_hash: sidecar_connection_hash(connection_id),
            pid: Some(4242),
            started_at_ms: Some(100),
            exited_at_ms: Some(200),
            exit_code: Some(0),
            status: Some("stopped".to_string()),
            diagnostic: Some("sidecar_exited".to_string()),
            endpoint: None,
            duration_ms: None,
        };
        append_sidecar_lifecycle_record_at(&path, &second)
            .expect("rotated lifecycle record should write");
        assert!(fs::metadata(&path).unwrap().len() <= MAX_SIDECAR_LIFECYCLE_LOG_BYTES);
        let rotated = temp.path().join(SIDECAR_LIFECYCLE_LOG_ROTATED_FILE);
        assert!(rotated.exists());
        assert!(fs::metadata(rotated).unwrap().len() <= MAX_SIDECAR_LIFECYCLE_LOG_BYTES);
    }

    #[test]
    fn sidecar_lifecycle_log_rejects_free_form_diagnostics() {
        assert_eq!(
            lifecycle_diagnostic_code(Some("safe_code-1:retry")),
            Some("safe_code-1:retry".to_string())
        );
        assert_eq!(
            lifecycle_diagnostic_code(Some("Bearer secret-token")),
            Some("diagnostic_redacted".to_string())
        );
        assert_eq!(
            lifecycle_status(Some("HEALTHY")),
            Some("unknown".to_string())
        );
    }

    #[test]
    fn sidecar_wait_is_bounded() {
        #[cfg(windows)]
        let mut child = Command::new("ping")
            .args(["127.0.0.1", "-n", "8"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn bounded wait fixture");
        #[cfg(unix)]
        let mut child = Command::new("sleep")
            .arg("8")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn bounded wait fixture");

        let started = Instant::now();
        assert_eq!(
            wait_for_sidecar_exit(&mut child, Duration::from_millis(40)),
            SidecarWaitResult::TimedOut
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn stop_sidecar_reaps_only_the_tracked_child() {
        let connection_id = format!("stop-fixture-{}", now_ms());
        #[cfg(windows)]
        let child = Command::new("ping")
            .args(["127.0.0.1", "-n", "30"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn stop fixture");
        #[cfg(unix)]
        let child = Command::new("sleep")
            .arg("30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn stop fixture");
        let pid = child.id();
        let executable = (0..20)
            .find_map(|_| {
                let path = query_process_executable_path(pid);
                if path.is_none() {
                    std::thread::sleep(Duration::from_millis(10));
                }
                path
            })
            .expect("stop fixture executable path");
        sidecars().lock().expect("sidecar registry").insert(
            connection_id.clone(),
            SidecarProcess {
                connection_id: connection_id.clone(),
                child,
                executable,
                #[cfg(windows)]
                managed_job: None,
                _managed_lifecycle: None,
                started_at_ms: now_ms(),
                exited_at_ms: None,
                exit_code: None,
                last_health: None,
            },
        );

        let started = Instant::now();
        let status = stop_sidecar(&connection_id).expect("tracked sidecar should stop");
        assert!(started.elapsed() < Duration::from_secs(4));
        assert!(matches!(status.status.as_str(), "stopped" | "degraded"));
        assert!(
            !sidecars()
                .lock()
                .expect("sidecar registry")
                .contains_key(&connection_id),
            "a successfully reaped child must leave the registry"
        );
    }
}
