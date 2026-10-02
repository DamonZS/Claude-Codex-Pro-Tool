//! Settings-page commands: device preferences, local routing, data backups
//! and config-file agent suppliers. Split out of `commands.rs`; shares its
//! helpers (`ok`, `failed`, settings locks) through `use super::*`.

use super::*;

// ============================================================================
// 设备级偏好设置（load_app_preferences / save_app_preferences）
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferencesPayload {
    pub preferences: claude_codex_pro_core::app_preferences::AppPreferences,
    pub resolved_dirs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAppPreferencesRequest {
    pub preferences: claude_codex_pro_core::app_preferences::AppPreferences,
}

/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` value for this app.
#[cfg(windows)]
const STARTUP_RUN_KEY: &str = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
#[cfg(windows)]
const STARTUP_RUN_VALUE: &str = "ClaudeCodexPro";

fn app_preferences_store() -> claude_codex_pro_core::app_preferences::PreferencesStore {
    claude_codex_pro_core::app_preferences::PreferencesStore::new(
        claude_codex_pro_core::app_preferences::PreferencesStore::default_path(),
    )
}

fn app_preferences_payload(
    preferences: claude_codex_pro_core::app_preferences::AppPreferences,
) -> AppPreferencesPayload {
    let resolved_dirs = claude_codex_pro_core::app_preferences::resolved_dirs(&preferences);
    AppPreferencesPayload {
        preferences,
        resolved_dirs,
    }
}

/// Currently persisted preferences, used as the fallback payload on errors.
fn saved_app_preferences_payload() -> AppPreferencesPayload {
    app_preferences_payload(app_preferences_store().load())
}

#[tauri::command]
pub async fn load_app_preferences() -> CommandResult<AppPreferencesPayload> {
    // Reading and normalizing the preferences file is blocking file IO.
    tauri::async_runtime::spawn_blocking(|| ok("偏好设置已加载。", saved_app_preferences_payload()))
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("加载偏好设置任务失败：{join_error}"),
                saved_app_preferences_payload(),
            )
        })
}

#[tauri::command]
pub async fn save_app_preferences(
    request: SaveAppPreferencesRequest,
) -> CommandResult<AppPreferencesPayload> {
    let requested = claude_codex_pro_core::app_preferences::normalize(request.preferences);
    // Both the preferences file and the Run key are written off the UI thread.
    tauri::async_runtime::spawn_blocking(move || {
        let store = app_preferences_store();
        let previous = store.load();

        let startup_changed = previous.launch_on_startup != requested.launch_on_startup
            || (requested.launch_on_startup && previous.silent_startup != requested.silent_startup);
        let startup_result = if startup_changed {
            sync_launch_on_startup(requested.launch_on_startup, requested.silent_startup)
        } else {
            Ok(())
        };

        match store.save(requested) {
            Ok(saved) => {
                let payload = app_preferences_payload(saved);
                match startup_result {
                    Ok(()) => ok("设置已保存。", payload),
                    Err(error) => failed(&format!("开机自启设置失败：{error}"), payload),
                }
            }
            Err(error) => failed(
                &format!("保存偏好设置失败：{error}"),
                saved_app_preferences_payload(),
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("保存偏好设置任务失败：{join_error}"),
            saved_app_preferences_payload(),
        )
    })
}

/// Register or unregister the current executable under the per-user Run key.
///
/// Non-Windows platforms have no equivalent here yet, so they report success.
#[cfg(windows)]
fn sync_launch_on_startup(enabled: bool, silent_startup: bool) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;

    let mut reg = std::process::Command::new("reg.exe");
    if enabled {
        let executable = std::env::current_exe().context("无法定位 Claude Codex Pro 运行文件")?;
        let mut command = format!("\"{}\"", executable.to_string_lossy());
        if silent_startup {
            command.push_str(" --silent");
        }
        reg.args([
            "add",
            STARTUP_RUN_KEY,
            "/v",
            STARTUP_RUN_VALUE,
            "/t",
            "REG_SZ",
            "/d",
        ])
        .arg(command)
        .arg("/f");
    } else {
        reg.args(["delete", STARTUP_RUN_KEY, "/v", STARTUP_RUN_VALUE, "/f"]);
    }
    reg.creation_flags(claude_codex_pro_core::windows_create_no_window())
        .stdin(std::process::Stdio::null());

    if !enabled {
        // Exit code alone tells us whether the value exists; reg.exe stderr is
        // localized (GBK on Chinese Windows), so never match on its text.
        let exists = std::process::Command::new("reg.exe")
            .args(["query", STARTUP_RUN_KEY, "/v", STARTUP_RUN_VALUE])
            .creation_flags(claude_codex_pro_core::windows_create_no_window())
            .stdin(std::process::Stdio::null())
            .output()
            .map(|output| output.status.success())
            .unwrap_or(true);
        if !exists {
            return Ok(());
        }
    }

    let output = reg.output().context("执行 reg.exe 失败")?;
    if !output.status.success() {
        anyhow::bail!(
            "{}注册表启动项失败（reg.exe 退出码 {:?}）",
            if enabled { "写入" } else { "删除" },
            output.status.code()
        );
    }
    Ok(())
}

#[cfg(not(windows))]
fn sync_launch_on_startup(_enabled: bool, _silent_startup: bool) -> anyhow::Result<()> {
    Ok(())
}

// ============================================================================
// 本地路由（load_routing_config / save_routing_config / set_routing_enabled ...）
// ============================================================================

/// Per-port TCP probe budget for `scan_local_proxies`.
const LOCAL_PROXY_SCAN_TIMEOUT: Duration = Duration::from_millis(300);
/// Requests inspected when computing `totalRequests` / `successRate`.
const ROUTING_TELEMETRY_WINDOW: usize = 500;
/// Outbound HTTP API contract: 8s timeout, one retry (CC Switch 行为).
const GLOBAL_PROXY_CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const GLOBAL_PROXY_TEST_URL: &str = "https://www.gstatic.com/generate_204";

/// When the local routing proxy was last switched on by this process.
static ROUTING_STARTED_AT: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

fn routing_started_at() -> &'static Mutex<Option<Instant>> {
    ROUTING_STARTED_AT.get_or_init(|| Mutex::new(None))
}

fn mark_routing_started() {
    let mut started = match routing_started_at().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *started = Some(Instant::now());
}

fn mark_routing_started_if_unset() {
    let mut started = match routing_started_at().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    started.get_or_insert_with(Instant::now);
}

fn clear_routing_started() {
    let mut started = match routing_started_at().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *started = None;
}

fn routing_uptime_seconds() -> u64 {
    let started = match routing_started_at().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    started
        .map(|instant| instant.elapsed().as_secs())
        .unwrap_or_default()
}

/// Live view of the local routing proxy (contract `RoutingStatus`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingRuntime {
    pub running: bool,
    pub address: String,
    pub port: u16,
    /// The proxy does not track per-connection sessions yet.
    pub active_connections: u64,
    pub total_requests: u64,
    pub success_rate: f64,
    pub uptime_seconds: u64,
    pub current_providers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailoverQueueEntry {
    pub provider_id: String,
    pub name: String,
    pub priority: i64,
    pub circuit_state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingPayload {
    pub config: claude_codex_pro_core::routing_config::RoutingConfig,
    /// 命名为 runtime，避免与 `CommandResult.status` 冲突。
    pub runtime: RoutingRuntime,
    pub queues: BTreeMap<String, Vec<FailoverQueueEntry>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalProxyTestPayload {
    pub ok: bool,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProxyScanPayload {
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRoutingConfigRequest {
    pub config: claude_codex_pro_core::routing_config::RoutingConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRoutingEnabledRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRoutingAppTakeoverRequest {
    pub app_id: String,
    pub takeover: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailoverQueueRequest {
    pub app_id: String,
    pub provider_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestGlobalProxyRequest {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

fn routing_store() -> anyhow::Result<claude_codex_pro_core::routing_config::RoutingStore> {
    claude_codex_pro_core::routing_config::RoutingStore::open_default()
}

/// `status` values actually written by `request_telemetry` are `success`,
/// `failed`, `interrupted` and `observed`; only `success` counts here.
const TELEMETRY_SUCCESS_STATUS: &str = "success";

/// `(totalRequests, successRate)` over the recent request log.
fn routing_request_stats() -> (u64, f64) {
    let records =
        claude_codex_pro_core::request_telemetry::read_recent_requests(ROUTING_TELEMETRY_WINDOW)
            .unwrap_or_default();
    let total = records.len() as u64;
    if total == 0 {
        return (0, 0.0);
    }
    let succeeded = records
        .iter()
        .filter(|record| record.status == TELEMETRY_SUCCESS_STATUS)
        .count();
    (total, succeeded as f64 / total as f64)
}

/// Display name of the profile id currently used by `app_id`.
fn routing_current_provider_name(app_id: &str) -> String {
    match SettingsStore::default().load() {
        Ok(settings) => {
            let profile = settings.active_relay_profile_for_target(app_id);
            let name = profile.name.trim();
            if name.is_empty() {
                profile.id
            } else {
                name.to_string()
            }
        }
        Err(_) => String::new(),
    }
}

/// Provider id → display name, used to label failover queue entries.
fn relay_profile_names(settings: &BackendSettings) -> BTreeMap<String, String> {
    settings
        .relay_profiles
        .iter()
        .filter(|profile| !profile.id.trim().is_empty())
        .map(|profile| {
            let name = profile.name.trim();
            let name = if name.is_empty() {
                profile.id.clone()
            } else {
                name.to_string()
            };
            (profile.id.clone(), name)
        })
        .collect()
}

fn routing_runtime(
    config: &claude_codex_pro_core::routing_config::RoutingConfig,
) -> RoutingRuntime {
    // `protocol_proxy_backend_online` probes the helper's status socket, so it
    // runs off the UI thread (every caller already sits in `spawn_blocking`).
    let running = config.enabled
        && claude_codex_pro_core::launcher::protocol_proxy_backend_online(config.listen_port);
    let (total_requests, success_rate) = routing_request_stats();
    let mut current_providers = BTreeMap::new();
    for app_id in ["claude", "claude-desktop", "codex"] {
        let name = routing_current_provider_name(app_id);
        if !name.is_empty() {
            current_providers.insert(app_id.to_string(), name);
        }
    }
    RoutingRuntime {
        running,
        address: config.listen_address.clone(),
        port: config.listen_port,
        active_connections: 0,
        total_requests,
        success_rate,
        uptime_seconds: if running {
            // Routing enabled in an earlier session: start counting from the
            // first time this process observes it running.
            if routing_uptime_seconds() == 0 {
                mark_routing_started_if_unset();
            }
            routing_uptime_seconds()
        } else {
            0
        },
        current_providers,
    }
}

/// The full routing payload: stored config, live status and per-app queues.
fn routing_payload_from_store(
    store: &claude_codex_pro_core::routing_config::RoutingStore,
) -> anyhow::Result<RoutingPayload> {
    let config = store.load()?;
    let settings = SettingsStore::default().load().unwrap_or_default();
    let names = relay_profile_names(&settings);

    let mut queues = BTreeMap::new();
    for app_id in claude_codex_pro_core::routing_config::PROXY_APP_IDS {
        let rows = store.queue(app_id)?;
        let entries = rows
            .into_iter()
            .map(|row| {
                let name = names
                    .get(&row.provider_id)
                    .cloned()
                    .unwrap_or_else(|| row.provider_id.clone());
                FailoverQueueEntry {
                    provider_id: row.provider_id,
                    name,
                    priority: row.priority,
                    circuit_state: row.circuit_state,
                }
            })
            .collect();
        queues.insert(app_id.to_string(), entries);
    }

    let runtime = routing_runtime(&config);
    Ok(RoutingPayload {
        config,
        runtime,
        queues,
    })
}

/// Fallback payload: the stored configuration plus an offline runtime. Never
/// fails, so a command error can still return a usable snapshot.
fn routing_fallback_payload() -> RoutingPayload {
    let config = routing_store()
        .and_then(|store| store.load())
        .unwrap_or_default();
    let mut queues = BTreeMap::new();
    for app_id in claude_codex_pro_core::routing_config::PROXY_APP_IDS {
        queues.insert(app_id.to_string(), Vec::new());
    }
    let runtime = RoutingRuntime {
        running: false,
        address: config.listen_address.clone(),
        port: config.listen_port,
        active_connections: 0,
        total_requests: 0,
        success_rate: 0.0,
        uptime_seconds: 0,
        current_providers: BTreeMap::new(),
    };
    RoutingPayload {
        config,
        runtime,
        queues,
    }
}

/// Validate one of the routing commands' app ids before touching the store.
fn routing_known_app(app_id: &str) -> bool {
    claude_codex_pro_core::routing_config::PROXY_APP_IDS.contains(&app_id)
}

/// Whether `provider_id` names a profile the user actually configured.
fn relay_profile_exists(provider_id: &str) -> bool {
    match SettingsStore::default().load() {
        Ok(settings) => settings
            .relay_profiles
            .iter()
            .any(|profile| profile.id == provider_id),
        Err(_) => false,
    }
}

#[tauri::command]
pub async fn load_routing_config() -> CommandResult<RoutingPayload> {
    // Configuration read plus helper liveness probe and telemetry scan are all blocking.
    tauri::async_runtime::spawn_blocking(|| match routing_store() {
        Ok(store) => match routing_payload_from_store(&store) {
            Ok(payload) => ok("路由配置已加载。", payload),
            Err(error) => failed(
                &format!("加载路由配置失败：{error}"),
                routing_fallback_payload(),
            ),
        },
        Err(error) => failed(
            &format!("加载路由配置失败：{error}"),
            routing_fallback_payload(),
        ),
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("加载路由配置任务失败：{join_error}"),
            routing_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn save_routing_config(
    request: SaveRoutingConfigRequest,
) -> CommandResult<RoutingPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = match routing_store() {
            Ok(store) => store,
            Err(error) => {
                return failed(
                    &format!("保存路由配置失败：{error}"),
                    routing_fallback_payload(),
                );
            }
        };
        if let Err(error) = store.save(request.config) {
            return failed(
                &format!("保存路由配置失败：{error}"),
                routing_fallback_payload(),
            );
        }
        match routing_payload_from_store(&store) {
            Ok(payload) => ok("路由配置已保存。", payload),
            Err(error) => failed(
                &format!("保存路由配置失败：{error}"),
                routing_fallback_payload(),
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("保存路由配置任务失败：{join_error}"),
            routing_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn set_routing_enabled(
    request: SetRoutingEnabledRequest,
) -> CommandResult<RoutingPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = match routing_store() {
            Ok(store) => store,
            Err(error) => {
                return failed(
                    &format!("切换本地路由失败：{error}"),
                    routing_fallback_payload(),
                );
            }
        };
        if request.enabled {
            // The shared helper listener serves the routing proxy; bring it up
            // before persisting `enabled` so the flag never claims a dead proxy.
            let port = store
                .load()
                .map(|config| config.listen_port)
                .unwrap_or(claude_codex_pro_core::protocol_proxy::DEFAULT_PROTOCOL_PROXY_PORT);
            if let Err(error) = tauri::async_runtime::block_on(
                claude_codex_pro_core::launcher::ensure_detached_helper(port),
            ) {
                return failed(
                    &format!("本地路由代理 {port} 启动失败：{error}"),
                    routing_fallback_payload(),
                );
            }
        }
        if let Err(error) = store.set_enabled(request.enabled) {
            return failed(
                &format!("切换本地路由失败：{error}"),
                routing_fallback_payload(),
            );
        }
        if request.enabled {
            mark_routing_started();
        } else {
            clear_routing_started();
        }
        let message = if request.enabled {
            "本地路由已启用。"
        } else {
            "本地路由已停用。"
        };
        match routing_payload_from_store(&store) {
            Ok(payload) => ok(message, payload),
            Err(error) => failed(
                &format!("切换本地路由失败：{error}"),
                routing_fallback_payload(),
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("切换本地路由任务失败：{join_error}"),
            routing_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn set_routing_app_takeover(
    request: SetRoutingAppTakeoverRequest,
) -> CommandResult<RoutingPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = match routing_store() {
            Ok(store) => store,
            Err(error) => {
                return failed(
                    &format!("切换应用接管失败：{error}"),
                    routing_fallback_payload(),
                );
            }
        };
        if let Err(error) = store.set_app_takeover(&request.app_id, request.takeover) {
            return failed(
                &format!("切换应用接管失败：{error}"),
                routing_fallback_payload(),
            );
        }
        let message = format!(
            "{} 接管已{}。",
            request.app_id,
            if request.takeover { "开启" } else { "关闭" }
        );
        match routing_payload_from_store(&store) {
            Ok(payload) => ok(&message, payload),
            Err(error) => failed(
                &format!("切换应用接管失败：{error}"),
                routing_fallback_payload(),
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("切换应用接管任务失败：{join_error}"),
            routing_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn add_failover_queue_provider(
    request: FailoverQueueRequest,
) -> CommandResult<RoutingPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        if !routing_known_app(&request.app_id) {
            return failed(
                &format!("未知的路由应用：{}", request.app_id),
                routing_fallback_payload(),
            );
        }
        if !relay_profile_exists(request.provider_id.trim()) {
            return failed(
                &format!("供应商不存在：{}", request.provider_id.trim()),
                routing_fallback_payload(),
            );
        }
        routing_queue_mutation("已加入故障转移队列。", move |store| {
            store.add_to_queue(&request.app_id, &request.provider_id)
        })
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("加入故障转移队列任务失败：{join_error}"),
            routing_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn remove_failover_queue_provider(
    request: FailoverQueueRequest,
) -> CommandResult<RoutingPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        routing_queue_mutation("已移出故障转移队列。", move |store| {
            store.remove_from_queue(&request.app_id, &request.provider_id)
        })
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("移出故障转移队列任务失败：{join_error}"),
            routing_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn reset_circuit_breaker(request: FailoverQueueRequest) -> CommandResult<RoutingPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        routing_queue_mutation("熔断器已重置。", move |store| {
            store.reset_circuit(&request.app_id, &request.provider_id)
        })
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("重置熔断器任务失败：{join_error}"),
            routing_fallback_payload(),
        )
    })
}

/// Shared body of the three queue commands: open the store, apply `mutate`,
/// then answer with a freshly reloaded payload.
fn routing_queue_mutation<F>(message: &str, mutate: F) -> CommandResult<RoutingPayload>
where
    F: FnOnce(&mut claude_codex_pro_core::routing_config::RoutingStore) -> anyhow::Result<()>,
{
    let mut store = match routing_store() {
        Ok(store) => store,
        Err(error) => {
            return failed(
                &format!("更新故障转移队列失败：{error}"),
                routing_fallback_payload(),
            );
        }
    };
    if let Err(error) = mutate(&mut store) {
        return failed(
            &format!("更新故障转移队列失败：{error}"),
            routing_fallback_payload(),
        );
    }
    match routing_payload_from_store(&store) {
        Ok(payload) => ok(message, payload),
        Err(error) => failed(
            &format!("更新故障转移队列失败：{error}"),
            routing_fallback_payload(),
        ),
    }
}

#[tauri::command]
pub async fn test_global_proxy(
    request: TestGlobalProxyRequest,
) -> CommandResult<GlobalProxyTestPayload> {
    let url = request.url.trim().to_string();
    if url.is_empty() {
        return failed(
            "请先填写代理地址",
            GlobalProxyTestPayload {
                ok: false,
                latency_ms: None,
            },
        );
    }
    // 密码留空时回退到已保存的密码；任何错误信息都不得包含密码。
    let password = match request.password.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => Some(value.to_string()),
        _ => tauri::async_runtime::spawn_blocking(|| match routing_store() {
            Ok(store) => store.global_proxy_password().unwrap_or_default(),
            Err(_) => None,
        })
        .await
        .unwrap_or_default(),
    };
    let username = request
        .username
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    match test_global_proxy_once(&url, username.as_deref(), password.as_deref()).await {
        Ok(latency_ms) => ok(
            &format!("代理连通（{latency_ms} ms）"),
            GlobalProxyTestPayload {
                ok: true,
                latency_ms: Some(latency_ms),
            },
        ),
        Err(first_error) => {
            // CC Switch 行为：超时重试 1 次。
            match test_global_proxy_once(&url, username.as_deref(), password.as_deref()).await {
                Ok(latency_ms) => ok(
                    &format!("代理连通（{latency_ms} ms）"),
                    GlobalProxyTestPayload {
                        ok: true,
                        latency_ms: Some(latency_ms),
                    },
                ),
                Err(_) => failed(
                    &format!("代理测试失败：{first_error}"),
                    GlobalProxyTestPayload {
                        ok: false,
                        latency_ms: None,
                    },
                ),
            }
        }
    }
}

/// One proxy connectivity attempt. Any HTTP status counts as reachable.
async fn test_global_proxy_once(
    url: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> anyhow::Result<u64> {
    let mut proxy = reqwest::Proxy::all(url).context("代理地址无效")?;
    if let Some(username) = username {
        proxy = proxy.basic_auth(username, password.unwrap_or_default());
    }
    let client = reqwest::Client::builder()
        .proxy(proxy)
        // Some proxies/gateways drop requests that carry no User-Agent.
        .user_agent(format!("ClaudeCodexPro/{}", env!("CARGO_PKG_VERSION")))
        .timeout(GLOBAL_PROXY_CONNECT_TIMEOUT)
        .build()
        .context("创建代理客户端失败")?;
    let started = Instant::now();
    let response = client
        .get(GLOBAL_PROXY_TEST_URL)
        .send()
        .await
        .context("无法通过代理建立连接")?;
    let latency_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
    // 任何 HTTP 状态码都说明代理连通。
    let _ = response.status();
    Ok(latency_ms)
}

#[tauri::command]
pub async fn scan_local_proxies() -> CommandResult<LocalProxyScanPayload> {
    // Port probing blocks on TCP connect timeouts, so keep it off the UI thread.
    tauri::async_runtime::spawn_blocking(|| {
        let mut candidates = Vec::new();
        // Only HTTP proxies: the global proxy setting accepts http(s) only, so a
        // socks5 candidate would be auto-filled and then rejected on save.
        for port in [7890_u16, 7891, 10808, 10809] {
            if local_port_is_open(port) {
                candidates.push(format!("http://127.0.0.1:{port}"));
            }
        }
        let message = if candidates.is_empty() {
            "未找到本机代理。".to_string()
        } else {
            format!("找到 {} 个本机代理。", candidates.len())
        };
        ok(&message, LocalProxyScanPayload { candidates })
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("扫描本机代理任务失败：{join_error}"),
            LocalProxyScanPayload {
                candidates: Vec::new(),
            },
        )
    })
}

fn local_port_is_open(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&address, LOCAL_PROXY_SCAN_TIMEOUT).is_ok()
}

// ============================================================================
// 数据备份与迁移（ccp.db / settings.json / app-preferences.json）
// ============================================================================

/// 列表型命令的返回体，对应 `contract.ts` 的 `BackupListResult`。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupListPayload {
    pub backups: Vec<claude_codex_pro_core::ccp_backup::BackupEntry>,
    pub dir: String,
}

/// 导入导出命令的返回体，对应 `contract.ts` 的 `DataTransferResult`。
///
/// 两个字段互斥：导出只需要 `path`，导入只需要 `safetyBackupId`；另一个
/// 序列化为 `null`，前端按可选字段处理。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataTransferPayload {
    pub path: Option<String>,
    pub safety_backup_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupIdRequest {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameBackupRequest {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDataRequest {
    pub dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDataRequest {
    pub path: String,
}

/// 备份用的三处数据位置与备份目录。
fn backup_paths() -> claude_codex_pro_core::ccp_backup::BackupPaths {
    claude_codex_pro_core::ccp_backup::BackupPaths::default_paths()
}

/// 当前备份列表，作为失败时的兜底负载（读取失败时返回空列表）。
fn backup_list_fallback_payload() -> BackupListPayload {
    let paths = backup_paths();
    let dir = paths.backup_dir.to_string_lossy().to_string();
    let backups = claude_codex_pro_core::ccp_backup::list_backups(&paths).unwrap_or_default();
    BackupListPayload { backups, dir }
}

/// 数据导入导出命令的兜底负载。
fn data_transfer_fallback_payload() -> DataTransferPayload {
    DataTransferPayload {
        path: None,
        safety_backup_id: None,
    }
}

#[tauri::command]
pub async fn list_database_backups() -> CommandResult<BackupListPayload> {
    tauri::async_runtime::spawn_blocking(|| {
        let paths = backup_paths();
        let dir = paths.backup_dir.to_string_lossy().to_string();
        match claude_codex_pro_core::ccp_backup::list_backups(&paths) {
            Ok(backups) => {
                let message = format!("已加载 {} 个备份。", backups.len());
                ok(&message, BackupListPayload { backups, dir })
            }
            Err(error) => failed(
                &format!("加载备份列表失败：{error}"),
                BackupListPayload {
                    backups: Vec::new(),
                    dir,
                },
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("加载备份列表失败：{join_error}"),
            backup_list_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn create_database_backup() -> CommandResult<BackupListPayload> {
    tauri::async_runtime::spawn_blocking(|| {
        let paths = backup_paths();
        let entry = match claude_codex_pro_core::ccp_backup::create_backup(&paths, None) {
            Ok(entry) => entry,
            Err(error) => {
                return failed(
                    &format!("创建备份失败：{error}"),
                    backup_list_fallback_payload(),
                );
            }
        };
        // 保留策略与备份间隔都来自设备偏好设置；清理失败不影响创建结果。
        let retain = app_preferences_store().load().backup_retain_count as usize;
        let _ = claude_codex_pro_core::ccp_backup::prune_backups(&paths, retain);
        let dir = paths.backup_dir.to_string_lossy().to_string();
        match claude_codex_pro_core::ccp_backup::list_backups(&paths) {
            Ok(backups) => {
                let message = format!("已创建备份 {}。", entry.id);
                ok(&message, BackupListPayload { backups, dir })
            }
            Err(error) => failed(
                &format!("创建备份失败：{error}"),
                BackupListPayload {
                    backups: Vec::new(),
                    dir,
                },
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("创建备份失败：{join_error}"),
            backup_list_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn restore_database_backup(request: BackupIdRequest) -> CommandResult<BackupListPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = backup_paths();
        // 恢复会改写 settings.json，必须与设置保存串行，避免互相覆盖。
        let _write_guard = settings_write_mutex()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let safety = match claude_codex_pro_core::ccp_backup::restore_backup(&paths, &request.id) {
            Ok(entry) => entry,
            Err(error) => {
                return failed(
                    &format!("恢复备份失败：{error}"),
                    backup_list_fallback_payload(),
                );
            }
        };
        let dir = paths.backup_dir.to_string_lossy().to_string();
        match claude_codex_pro_core::ccp_backup::list_backups(&paths) {
            Ok(backups) => {
                let message = format!("已恢复备份，恢复前状态已另存为 {}。", safety.id);
                ok(&message, BackupListPayload { backups, dir })
            }
            Err(error) => failed(
                &format!("恢复备份失败：{error}"),
                BackupListPayload {
                    backups: Vec::new(),
                    dir,
                },
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("恢复备份失败：{join_error}"),
            backup_list_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn rename_database_backup(
    request: RenameBackupRequest,
) -> CommandResult<BackupListPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = backup_paths();
        if let Err(error) =
            claude_codex_pro_core::ccp_backup::rename_backup(&paths, &request.id, &request.name)
        {
            return failed(
                &format!("重命名备份失败：{error}"),
                backup_list_fallback_payload(),
            );
        }
        let dir = paths.backup_dir.to_string_lossy().to_string();
        match claude_codex_pro_core::ccp_backup::list_backups(&paths) {
            Ok(backups) => ok("备份已重命名。", BackupListPayload { backups, dir }),
            Err(error) => failed(
                &format!("重命名备份失败：{error}"),
                BackupListPayload {
                    backups: Vec::new(),
                    dir,
                },
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("重命名备份失败：{join_error}"),
            backup_list_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn delete_database_backup(request: BackupIdRequest) -> CommandResult<BackupListPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = backup_paths();
        if let Err(error) = claude_codex_pro_core::ccp_backup::delete_backup(&paths, &request.id) {
            return failed(
                &format!("删除备份失败：{error}"),
                backup_list_fallback_payload(),
            );
        }
        let dir = paths.backup_dir.to_string_lossy().to_string();
        match claude_codex_pro_core::ccp_backup::list_backups(&paths) {
            Ok(backups) => ok("备份已删除。", BackupListPayload { backups, dir }),
            Err(error) => failed(
                &format!("删除备份失败：{error}"),
                BackupListPayload {
                    backups: Vec::new(),
                    dir,
                },
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("删除备份失败：{join_error}"),
            backup_list_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn export_ccp_data(request: ExportDataRequest) -> CommandResult<DataTransferPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = backup_paths();
        let dest_dir = PathBuf::from(request.dir.trim());
        match claude_codex_pro_core::ccp_backup::export_bundle(&paths, &dest_dir) {
            Ok(path) => {
                let displayed = path.to_string_lossy().to_string();
                let message = format!("数据已导出到 {displayed}。");
                ok(
                    &message,
                    DataTransferPayload {
                        path: Some(displayed),
                        safety_backup_id: None,
                    },
                )
            }
            Err(error) => failed(
                &format!("导出数据失败：{error}"),
                data_transfer_fallback_payload(),
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("导出数据失败：{join_error}"),
            data_transfer_fallback_payload(),
        )
    })
}

#[tauri::command]
pub async fn import_ccp_data(request: ImportDataRequest) -> CommandResult<DataTransferPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = backup_paths();
        let source = PathBuf::from(request.path.trim());
        // 导入同样会改写 settings.json，必须与设置保存串行。
        let _write_guard = settings_write_mutex()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match claude_codex_pro_core::ccp_backup::import_bundle(&paths, &source) {
            Ok(safety) => {
                let message = format!("数据已导入，导入前状态已另存为 {}。", safety.id);
                ok(
                    &message,
                    DataTransferPayload {
                        path: None,
                        safety_backup_id: Some(safety.id),
                    },
                )
            }
            Err(error) => failed(
                &format!("导入数据失败：{error}"),
                data_transfer_fallback_payload(),
            ),
        }
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("导入数据失败：{join_error}"),
            data_transfer_fallback_payload(),
        )
    })
}

/// 启动自动备份后台任务：启动 60 秒后检查一次，之后每小时检查一次。
///
/// 是否该备份由 `backupIntervalHours` 决定，`0` 表示关闭；`backupRetainCount`
/// 决定保留数量。任何失败都只写一行 stderr，绝不 panic，也不影响管理器主流程。
pub fn spawn_auto_backup_task() {
    tauri::async_runtime::spawn(async move {
        // 启动阶段磁盘与数据库都还在预热，先让出 60 秒。
        tokio::time::sleep(Duration::from_secs(AUTO_BACKUP_STARTUP_DELAY_SECONDS)).await;
        loop {
            let outcome = tauri::async_runtime::spawn_blocking(auto_backup_once).await;
            if outcome.is_err() {
                eprintln!("自动备份任务执行失败。");
            }
            tokio::time::sleep(Duration::from_secs(AUTO_BACKUP_INTERVAL_SECONDS)).await;
        }
    });
}

/// 一次自动备份检查。返回是否真的创建了备份。
fn auto_backup_once() -> anyhow::Result<bool> {
    let preferences = app_preferences_store().load();
    let interval_hours = preferences.backup_interval_hours;
    let retain = preferences.backup_retain_count as usize;
    let paths = backup_paths();
    if !claude_codex_pro_core::ccp_backup::auto_backup_due(&paths, interval_hours) {
        return Ok(false);
    }
    claude_codex_pro_core::ccp_backup::create_backup(&paths, Some("auto"))?;
    claude_codex_pro_core::ccp_backup::prune_backups(&paths, retain)?;
    Ok(true)
}

/// 自动备份首次检查前的等待时间。
const AUTO_BACKUP_STARTUP_DELAY_SECONDS: u64 = 60;
/// 自动备份两次检查之间的间隔。
const AUTO_BACKUP_INTERVAL_SECONDS: u64 = 60 * 60;

// ============================================================================
// Agent 供应商（config 文件型 Agent，前端契约 agentProviderContract.ts）
// ============================================================================

use claude_codex_pro_core::agent_providers::{
    self as agent_providers, AgentProvider, AgentProviderState, AgentProviderStore, ApplyMode,
};

/// 串行化所有会写文件的 Agent 供应商命令，避免同一次写入被并发交错。
static AGENT_PROVIDER_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn agent_provider_lock() -> &'static Mutex<()> {
    AGENT_PROVIDER_LOCK.get_or_init(|| Mutex::new(()))
}

/// 前端 `AgentProvidersResult`：状态、文案、列表与每个 App 的状态。
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvidersPayload {
    pub providers: Vec<AgentProvider>,
    pub states: BTreeMap<String, AgentProviderState>,
}

/// 手动模式（Cursor）一次性返回的粘贴用值，只有 `apply` 会带上它。
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentProviderReveal {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

/// `apply_agent_provider` 的结果：列表负载 + 可选的一次性展示值。
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentApplyPayload {
    #[serde(flatten)]
    pub providers: AgentProvidersPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reveal: Option<AgentProviderReveal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAgentProviderRequest {
    pub provider: AgentProvider,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProviderIdRequest {
    pub app_id: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderAgentProvidersRequest {
    pub app_id: String,
    pub ids: Vec<String>,
}

/// Agent 的配置文件目录：优先用偏好设置里的覆盖值，否则用默认目录。
fn agent_provider_dir(app_id: &str) -> Option<PathBuf> {
    let preferences = app_preferences_store().load();
    if let Some(override_dir) = preferences
        .config_dirs
        .get(app_id)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return Some(PathBuf::from(override_dir));
    }
    agent_providers::default_dir(app_id)
}

/// 当前列表负载：所有供应商 + 每个 App 的状态。
fn agent_providers_snapshot(store: &AgentProviderStore) -> AgentProvidersPayload {
    let providers = store.list().unwrap_or_else(|error| {
        eprintln!("读取 Agent 供应商失败：{error}");
        Vec::new()
    });

    let mut states = BTreeMap::new();
    for app_id in agent_providers::AGENT_PROVIDER_APP_IDS {
        let dir = agent_provider_dir(app_id);
        let writer = agent_providers::writer_for(app_id);
        let (config_path, installed) = match (dir.as_deref(), writer.as_deref()) {
            (Some(dir), Some(writer)) => {
                let path = writer.config_path(dir);
                let installed = dir.is_dir();
                (path.to_string_lossy().into_owned(), installed)
            }
            (Some(dir), None) => (String::new(), dir.is_dir()),
            (None, _) => (String::new(), false),
        };

        let stored = store.state(app_id).unwrap_or_default();
        states.insert(
            app_id.to_string(),
            AgentProviderState {
                active_id: stored.active_id,
                applied_ids: stored.applied_ids,
                config_path,
                installed,
            },
        );
    }

    AgentProvidersPayload { providers, states }
}

/// 读取当前快照，失败时退化为空负载（`failed` 的兜底值）。
fn agent_providers_fallback() -> AgentProvidersPayload {
    match AgentProviderStore::open_default() {
        Ok(store) => agent_providers_snapshot(&store),
        Err(error) => {
            eprintln!("打开供应商数据库失败：{error}");
            AgentProvidersPayload::default()
        }
    }
}

/// 写入一个供应商到 Agent 自己的配置文件。
fn write_agent_provider(
    app_id: &str,
    provider: &AgentProvider,
    api_key: &str,
) -> anyhow::Result<()> {
    let dir =
        agent_provider_dir(app_id).ok_or_else(|| anyhow::anyhow!("未找到该 Agent 的配置目录"))?;
    let writer = agent_providers::writer_for(app_id)
        .ok_or_else(|| anyhow::anyhow!("该 Agent 不支持写入配置文件"))?;
    writer.apply(&dir, provider, api_key)
}

/// Switch 模式是否把这个供应商写进了当前文件。
fn agent_provider_is_active(store: &AgentProviderStore, app_id: &str, id: &str) -> bool {
    if !matches!(agent_providers::apply_mode(app_id), Some(ApplyMode::Switch)) {
        return false;
    }
    store
        .state(app_id)
        .map(|state| state.active_id.as_deref() == Some(id))
        .unwrap_or(false)
}

/// Additive 模式是否已经把这个供应商写进了当前文件。
fn agent_provider_is_applied(store: &AgentProviderStore, app_id: &str, id: &str) -> bool {
    if !matches!(
        agent_providers::apply_mode(app_id),
        Some(ApplyMode::Additive)
    ) {
        return false;
    }
    store
        .state(app_id)
        .map(|state| state.applied_ids.iter().any(|applied| applied == id))
        .unwrap_or(false)
}

/// 把 `id` 从 applied 列表中移除后的新列表。
fn applied_ids_without(store: &AgentProviderStore, app_id: &str, id: &str) -> Vec<String> {
    store
        .state(app_id)
        .map(|state| {
            state
                .applied_ids
                .into_iter()
                .filter(|applied| applied != id)
                .collect()
        })
        .unwrap_or_default()
}

fn agent_app_label(app_id: &str) -> String {
    match app_id {
        "gemini" => "Gemini",
        "grok" => "Grok Build",
        "opencode" => "OpenCode",
        "openclaw" => "OpenClaw",
        "hermes" => "Hermes",
        "pi" => "Pi",
        "mcode" => "MiniMax Code",
        "workbuddy" => "WorkBuddy",
        "cursor" => "Cursor",
        _ => app_id,
    }
    .to_string()
}

fn load_agent_providers_blocking() -> CommandResult<AgentProvidersPayload> {
    match AgentProviderStore::open_default() {
        Ok(store) => ok("已加载 Agent 供应商。", agent_providers_snapshot(&store)),
        Err(error) => failed(
            &format!("加载供应商失败：{error}"),
            AgentProvidersPayload::default(),
        ),
    }
}

/// 列出所有 Agent 供应商及其在每个 Agent 中的状态。
#[tauri::command]
pub async fn list_agent_providers() -> CommandResult<AgentProvidersPayload> {
    tauri::async_runtime::spawn_blocking(load_agent_providers_blocking)
        .await
        .unwrap_or_else(|join_error| {
            failed(
                &format!("加载供应商失败：{join_error}"),
                agent_providers_fallback(),
            )
        })
}

fn save_agent_provider_blocking(
    request: SaveAgentProviderRequest,
) -> CommandResult<AgentProvidersPayload> {
    let mut store = match AgentProviderStore::open_default() {
        Ok(store) => store,
        Err(error) => {
            return failed(
                &format!("保存供应商失败：{error}"),
                AgentProvidersPayload::default(),
            );
        }
    };

    let mut provider = request.provider;
    let incoming_key = provider.api_key.clone();
    let saved = match store.save(&mut provider) {
        Ok(saved) => saved,
        Err(error) => {
            return failed(
                &format!("保存供应商失败：{error}"),
                agent_providers_snapshot(&store),
            );
        }
    };

    // 已生效的供应商要在实时文件里同步更新，否则文件里还是旧值。
    let key = incoming_key.unwrap_or_else(|| {
        store
            .get_with_key(&saved.id)
            .ok()
            .flatten()
            .map(|(_, key)| key)
            .unwrap_or_default()
    });
    let active = agent_provider_is_active(&store, &saved.app_id, &saved.id);
    let applied = agent_provider_is_applied(&store, &saved.app_id, &saved.id);
    if (active || applied) && !key.is_empty() {
        let app_id = saved.app_id.clone();
        if let Err(error) = write_agent_provider(&app_id, &saved, &key) {
            return failed(
                &format!("保存供应商失败：{error}"),
                agent_providers_snapshot(&store),
            );
        }
    }

    ok("供应商已保存。", agent_providers_snapshot(&store))
}

/// 新增或更新一个 Agent 供应商；已生效时自动重新应用。
#[tauri::command]
pub async fn save_agent_provider(
    request: SaveAgentProviderRequest,
) -> CommandResult<AgentProvidersPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = agent_provider_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        save_agent_provider_blocking(request)
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("保存供应商失败：{join_error}"),
            agent_providers_fallback(),
        )
    })
}

fn delete_agent_provider_blocking(
    request: AgentProviderIdRequest,
) -> CommandResult<AgentProvidersPayload> {
    let mut store = match AgentProviderStore::open_default() {
        Ok(store) => store,
        Err(error) => {
            return failed(
                &format!("删除供应商失败：{error}"),
                AgentProvidersPayload::default(),
            );
        }
    };

    match agent_providers::apply_mode(&request.app_id) {
        Some(ApplyMode::Additive) => {
            // 先从文件里摘掉，失败就中止删除，避免文件里留下孤儿节点。
            if agent_provider_is_applied(&store, &request.app_id, &request.id)
                && let Some(dir) = agent_provider_dir(&request.app_id)
                && let Some(writer) = agent_providers::writer_for(&request.app_id)
                && let Err(error) = writer.unapply(&dir, &request.id)
            {
                return failed(
                    &format!("删除供应商失败：{error}"),
                    agent_providers_snapshot(&store),
                );
            }
        }
        Some(ApplyMode::Switch) => {
            if agent_provider_is_active(&store, &request.app_id, &request.id) {
                return failed(
                    "请先切换到其他供应商再删除",
                    agent_providers_snapshot(&store),
                );
            }
        }
        _ => {}
    }

    if let Err(error) = store.delete(&request.id) {
        return failed(
            &format!("删除供应商失败：{error}"),
            agent_providers_snapshot(&store),
        );
    }
    let remaining = applied_ids_without(&store, &request.app_id, &request.id);
    if let Err(error) = store.set_applied(&request.app_id, &remaining) {
        return failed(
            &format!("删除供应商失败：{error}"),
            agent_providers_snapshot(&store),
        );
    }

    ok("供应商已删除。", agent_providers_snapshot(&store))
}

/// 删除一个 Agent 供应商。
#[tauri::command]
pub async fn delete_agent_provider(
    request: AgentProviderIdRequest,
) -> CommandResult<AgentProvidersPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = agent_provider_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        delete_agent_provider_blocking(request)
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("删除供应商失败：{join_error}"),
            agent_providers_fallback(),
        )
    })
}

fn apply_agent_provider_blocking(
    request: AgentProviderIdRequest,
) -> CommandResult<AgentApplyPayload> {
    let mut store = match AgentProviderStore::open_default() {
        Ok(store) => store,
        Err(error) => {
            return failed(
                &format!("应用供应商失败：{error}"),
                AgentApplyPayload::default(),
            );
        }
    };

    let Some((provider, api_key)) = store.get_with_key(&request.id).ok().flatten() else {
        return failed(
            "供应商不存在",
            agent_apply_payload(agent_providers_snapshot(&store)),
        );
    };

    match agent_providers::apply_mode(&request.app_id) {
        Some(ApplyMode::Switch) => {
            if let Err(error) = write_agent_provider(&request.app_id, &provider, &api_key) {
                return failed(
                    &format!("应用供应商失败：{error}"),
                    agent_apply_payload(agent_providers_snapshot(&store)),
                );
            }
            if let Err(error) = store.set_active(&request.app_id, Some(&request.id)) {
                return failed(
                    &format!("应用供应商失败：{error}"),
                    agent_apply_payload(agent_providers_snapshot(&store)),
                );
            }
        }
        Some(ApplyMode::Additive) => {
            if let Err(error) = write_agent_provider(&request.app_id, &provider, &api_key) {
                return failed(
                    &format!("应用供应商失败：{error}"),
                    agent_apply_payload(agent_providers_snapshot(&store)),
                );
            }
            let mut applied = store.state(&request.app_id).unwrap_or_default().applied_ids;
            if !applied.iter().any(|id| id == &request.id) {
                applied.push(request.id.clone());
            }
            if let Err(error) = store.set_applied(&request.app_id, &applied) {
                return failed(
                    &format!("应用供应商失败：{error}"),
                    agent_apply_payload(agent_providers_snapshot(&store)),
                );
            }
        }
        // 手动模式：不写任何文件，只把值返回一次供用户粘贴。
        Some(ApplyMode::Manual) => {
            let model = if provider.default_model.trim().is_empty() {
                provider.models.first().cloned().unwrap_or_default()
            } else {
                provider.default_model.clone()
            };
            let label = agent_app_label(&request.app_id);
            return ok(
                &format!("已应用到 {label}。"),
                AgentApplyPayload {
                    providers: agent_providers_snapshot(&store),
                    reveal: Some(AgentProviderReveal {
                        base_url: provider.base_url.clone(),
                        api_key,
                        model,
                    }),
                },
            );
        }
        None => {
            return failed(
                "未知的 Agent 应用",
                agent_apply_payload(agent_providers_snapshot(&store)),
            );
        }
    }

    let label = agent_app_label(&request.app_id);
    ok(
        &format!("已应用到 {label}。"),
        agent_apply_payload(agent_providers_snapshot(&store)),
    )
}

fn agent_apply_payload(providers: AgentProvidersPayload) -> AgentApplyPayload {
    AgentApplyPayload {
        providers,
        reveal: None,
    }
}

/// 把一个供应商应用到 Agent 的实时配置。
#[tauri::command]
pub async fn apply_agent_provider(
    request: AgentProviderIdRequest,
) -> CommandResult<AgentApplyPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = agent_provider_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        apply_agent_provider_blocking(request)
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("应用供应商失败：{join_error}"),
            AgentApplyPayload {
                providers: agent_providers_fallback(),
                reveal: None,
            },
        )
    })
}

fn unapply_agent_provider_blocking(
    request: AgentProviderIdRequest,
) -> CommandResult<AgentProvidersPayload> {
    let mut store = match AgentProviderStore::open_default() {
        Ok(store) => store,
        Err(error) => {
            return failed(
                &format!("移除供应商失败：{error}"),
                AgentProvidersPayload::default(),
            );
        }
    };

    if !matches!(
        agent_providers::apply_mode(&request.app_id),
        Some(ApplyMode::Additive)
    ) {
        return failed("该 Agent 不支持移除操作", agent_providers_snapshot(&store));
    }

    let dir = match agent_provider_dir(&request.app_id) {
        Some(dir) => dir,
        None => {
            return failed(
                "未找到该 Agent 的配置目录",
                agent_providers_snapshot(&store),
            );
        }
    };
    let Some(writer) = agent_providers::writer_for(&request.app_id) else {
        return failed(
            "该 Agent 不支持写入配置文件",
            agent_providers_snapshot(&store),
        );
    };
    if let Err(error) = writer.unapply(&dir, &request.id) {
        return failed(
            &format!("移除供应商失败：{error}"),
            agent_providers_snapshot(&store),
        );
    }

    let remaining = applied_ids_without(&store, &request.app_id, &request.id);
    if let Err(error) = store.set_applied(&request.app_id, &remaining) {
        return failed(
            &format!("移除供应商失败：{error}"),
            agent_providers_snapshot(&store),
        );
    }

    let label = agent_app_label(&request.app_id);
    ok(
        &format!("已从 {label} 移除。"),
        agent_providers_snapshot(&store),
    )
}

/// 从 Agent 的实时配置中移除一个 additive 供应商。
#[tauri::command]
pub async fn unapply_agent_provider(
    request: AgentProviderIdRequest,
) -> CommandResult<AgentProvidersPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = agent_provider_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unapply_agent_provider_blocking(request)
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("移除供应商失败：{join_error}"),
            agent_providers_fallback(),
        )
    })
}

fn reorder_agent_providers_blocking(
    request: ReorderAgentProvidersRequest,
) -> CommandResult<AgentProvidersPayload> {
    let mut store = match AgentProviderStore::open_default() {
        Ok(store) => store,
        Err(error) => {
            return failed(
                &format!("排序失败：{error}"),
                AgentProvidersPayload::default(),
            );
        }
    };
    if let Err(error) = store.reorder(&request.app_id, &request.ids) {
        return failed(
            &format!("排序失败：{error}"),
            agent_providers_snapshot(&store),
        );
    }
    ok("排序已保存。", agent_providers_snapshot(&store))
}

/// 保存一个 Agent 内供应商的顺序。
#[tauri::command]
pub async fn reorder_agent_providers(
    request: ReorderAgentProvidersRequest,
) -> CommandResult<AgentProvidersPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = agent_provider_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reorder_agent_providers_blocking(request)
    })
    .await
    .unwrap_or_else(|join_error| {
        failed(
            &format!("排序失败：{join_error}"),
            agent_providers_fallback(),
        )
    })
}

#[cfg(test)]
mod agent_provider_command_tests {
    use super::*;

    /// 前端把 `apiKey` 作为可选字段发送，缺省与给出两种情况都必须能反序列化。
    #[test]
    fn save_request_accepts_an_optional_api_key() {
        let with_key = serde_json::json!({
            "provider": {
                "id": "ap-1234567890ab",
                "appId": "gemini",
                "name": "供应商",
                "baseUrl": "https://api.example.com/v1",
                "apiKey": "sk-test-placeholder",
                "hasApiKey": true,
                "apiFormat": "gemini-native",
                "models": ["model-a"],
                "defaultModel": "model-a",
                "notes": "",
                "sortIndex": 0
            }
        });
        let request: SaveAgentProviderRequest =
            serde_json::from_value(with_key).expect("带 apiKey 的请求必须反序列化成功");
        assert_eq!(
            request.provider.api_key.as_deref(),
            Some("sk-test-placeholder")
        );

        let without_key = serde_json::json!({
            "provider": {
                "id": "ap-1234567890ab",
                "appId": "gemini",
                "name": "供应商",
                "baseUrl": "https://api.example.com/v1",
                "hasApiKey": true,
                "apiFormat": "gemini-native",
                "models": ["model-a"],
                "defaultModel": "model-a",
                "notes": "",
                "sortIndex": 0
            }
        });
        let request: SaveAgentProviderRequest =
            serde_json::from_value(without_key).expect("缺少 apiKey 的请求必须反序列化成功");
        assert!(request.provider.api_key.is_none());
    }

    /// 负载形状必须与前端 `AgentProvidersResult` 一致：camelCase + 扁平化。
    #[test]
    fn payload_shape_matches_the_frontend_contract() {
        let mut states = BTreeMap::new();
        states.insert(
            "gemini".to_string(),
            AgentProviderState {
                active_id: Some("ap-1234567890ab".to_string()),
                applied_ids: Vec::new(),
                config_path: "C:/Users/x/.gemini/settings.json".to_string(),
                installed: true,
            },
        );
        let result = ok(
            "已加载 Agent 供应商。",
            AgentProvidersPayload {
                providers: Vec::new(),
                states,
            },
        );
        let value = serde_json::to_value(&result).expect("序列化负载");
        assert_eq!(value["status"], "ok");
        assert_eq!(value["message"], "已加载 Agent 供应商。");
        assert!(value["providers"].is_array());
        assert_eq!(value["states"]["gemini"]["activeId"], "ap-1234567890ab");
        assert_eq!(
            value["states"]["gemini"]["configPath"],
            "C:/Users/x/.gemini/settings.json"
        );
        assert_eq!(value["states"]["gemini"]["installed"], true);
        assert!(value["states"]["gemini"]["appliedIds"].is_array());

        // reveal 只在 apply 出现，并且是扁平化到同一层的字段。
        let reveal_result = ok(
            "已应用到 Cursor。",
            AgentApplyPayload {
                providers: AgentProvidersPayload::default(),
                reveal: Some(AgentProviderReveal {
                    base_url: "https://api.example.com/v1".to_string(),
                    api_key: "sk-test-placeholder".to_string(),
                    model: "model-a".to_string(),
                }),
            },
        );
        let value = serde_json::to_value(&reveal_result).expect("序列化 reveal 负载");
        assert_eq!(value["reveal"]["baseUrl"], "https://api.example.com/v1");
        assert_eq!(value["reveal"]["model"], "model-a");
        assert!(value["providers"].is_array(), "reveal 与列表同层");
        assert!(value["states"].is_object());
    }

    /// writer_for 必须覆盖全部 8 个可写 App，cursor 没有 writer。
    #[test]
    fn writer_lookup_covers_every_writable_app() {
        assert!(matches!(
            agent_providers::apply_mode("gemini"),
            Some(ApplyMode::Switch)
        ));
        assert!(matches!(
            agent_providers::apply_mode("hermes"),
            Some(ApplyMode::Additive)
        ));
        assert!(matches!(
            agent_providers::apply_mode("cursor"),
            Some(ApplyMode::Manual)
        ));
        assert!(agent_providers::writer_for("cursor").is_none());
        for app_id in [
            "gemini",
            "grok",
            "opencode",
            "openclaw",
            "hermes",
            "pi",
            "mcode",
            "workbuddy",
        ] {
            assert!(
                agent_providers::writer_for(app_id).is_some(),
                "{app_id} 必须有 writer"
            );
        }
        assert_eq!(agent_app_label("mcode"), "MiniMax Code");
    }
}
