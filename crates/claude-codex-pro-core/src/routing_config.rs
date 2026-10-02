//! Local routing configuration (routing takeover, failover queues, circuits).
//!
//! The JSON shapes mirror `AppProxyConfig`, `RectifierConfig`, `RoutingConfig`
//! and `FailoverQueueEntry` in
//! `apps/claude-codex-pro-manager/src/components/settings/contract.ts`
//! (camelCase keys, same defaults and the same numeric ranges as the frontend's
//! `invalidFailoverFields` checks).
//!
//! Persistence is split the same way the contract is: the routing-wide fields
//! live as one JSON document under the `routing.config` key of the `kv` table,
//! while each app's proxy parameters live in their own `proxy_config` row. The
//! global proxy password is stored under its own key so it never travels in a
//! serialized config and never reaches the frontend.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// Agents whose traffic can be taken over by the local routing proxy.
pub const PROXY_APP_IDS: [&str; 4] = ["claude", "claude-desktop", "codex", "gemini"];

/// `kv` key holding the serialized routing config (without `apps`/password).
const ROUTING_CONFIG_KEY: &str = "routing.config";
/// `kv` key holding the global outbound proxy password.
const ROUTING_PASSWORD_KEY: &str = "routing.globalProxyPassword";

const DEFAULT_LISTEN_ADDRESS: &str = "127.0.0.1";

/// Inclusive numeric bounds, matching `invalidFailoverFields` in `RoutingTab.tsx`.
const MAX_RETRIES: (u32, u32) = (0, 10);
const STREAMING_FIRST_BYTE_TIMEOUT: (u32, u32) = (1, 120);
const STREAMING_IDLE_TIMEOUT_MIN: u32 = 60;
const STREAMING_IDLE_TIMEOUT_MAX: u32 = 600;
const NON_STREAMING_TIMEOUT: (u32, u32) = (60, 1200);
const CIRCUIT_FAILURE_THRESHOLD: (u32, u32) = (1, 20);
const CIRCUIT_SUCCESS_THRESHOLD: (u32, u32) = (1, 10);
const CIRCUIT_TIMEOUT_SECONDS: (u32, u32) = (0, 300);
const CIRCUIT_MIN_REQUESTS: (u32, u32) = (5, 100);

/// Per-app routing parameters (`proxy_config` table row).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppProxyConfig {
    pub app_id: String,
    pub takeover: bool,
    pub auto_failover_enabled: bool,
    pub max_retries: u32,
    pub streaming_first_byte_timeout: u32,
    pub streaming_idle_timeout: u32,
    pub non_streaming_timeout: u32,
    pub circuit_failure_threshold: u32,
    pub circuit_success_threshold: u32,
    pub circuit_timeout_seconds: u32,
    pub circuit_error_rate_threshold: f64,
    pub circuit_min_requests: u32,
}

impl Default for AppProxyConfig {
    fn default() -> Self {
        default_app_proxy_config("")
    }
}

/// Contract defaults for one app: takeover and failover are always off, and the
/// numeric parameters depend on which agent the row belongs to.
pub fn default_app_proxy_config(app_id: &str) -> AppProxyConfig {
    let claude = app_id == "claude";
    AppProxyConfig {
        app_id: app_id.to_string(),
        takeover: false,
        auto_failover_enabled: false,
        max_retries: if claude { 6 } else { 3 },
        streaming_first_byte_timeout: if claude { 90 } else { 60 },
        streaming_idle_timeout: if claude { 180 } else { 120 },
        non_streaming_timeout: 600,
        circuit_failure_threshold: if claude { 8 } else { 4 },
        circuit_success_threshold: if claude { 3 } else { 2 },
        circuit_timeout_seconds: if claude { 90 } else { 60 },
        circuit_error_rate_threshold: if claude { 0.7 } else { 0.6 },
        circuit_min_requests: if claude { 15 } else { 10 },
    }
}

/// Response rectifier switches. Every one of them defaults to enabled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RectifierConfig {
    pub enabled: bool,
    pub thinking_signature: bool,
    pub thinking_budget: bool,
    pub media_fallback: bool,
    pub media_heuristic: bool,
}

impl Default for RectifierConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            thinking_signature: true,
            thinking_budget: true,
            media_fallback: true,
            media_heuristic: true,
        }
    }
}

/// The routing-wide configuration exchanged with the settings UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RoutingConfig {
    pub enabled: bool,
    pub listen_address: String,
    pub listen_port: u16,
    pub enable_logging: bool,
    pub show_routing_toggle_on_main: bool,
    pub show_failover_toggle_on_main: bool,
    pub apps: Vec<AppProxyConfig>,
    pub rectifier: RectifierConfig,
    pub global_proxy_url: String,
    pub global_proxy_username: String,
    /// Never serialized: the password only ever travels frontend → backend.
    #[serde(default, skip_serializing)]
    pub global_proxy_password: Option<String>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            listen_address: DEFAULT_LISTEN_ADDRESS.to_string(),
            listen_port: crate::protocol_proxy::DEFAULT_PROTOCOL_PROXY_PORT,
            enable_logging: true,
            show_routing_toggle_on_main: false,
            show_failover_toggle_on_main: false,
            apps: default_apps(),
            rectifier: RectifierConfig::default(),
            global_proxy_url: String::new(),
            global_proxy_username: String::new(),
            global_proxy_password: None,
        }
    }
}

/// One app's default row per `PROXY_APP_IDS`, in contract order.
fn default_apps() -> Vec<AppProxyConfig> {
    PROXY_APP_IDS
        .iter()
        .map(|app_id| default_app_proxy_config(app_id))
        .collect()
}

/// Whether `app_id` is one of the routable agents.
pub fn is_proxy_app_id(app_id: &str) -> bool {
    PROXY_APP_IDS.contains(&app_id)
}

/// Reject an app id outside `PROXY_APP_IDS`.
fn validate_app_id(app_id: &str) -> anyhow::Result<()> {
    if is_proxy_app_id(app_id) {
        Ok(())
    } else {
        anyhow::bail!("未知的路由应用：{app_id}")
    }
}

/// Reject a provider id that is empty once trimmed.
fn validate_provider_id(provider_id: &str) -> anyhow::Result<()> {
    if provider_id.trim().is_empty() {
        anyhow::bail!("供应商 ID 不能为空");
    }
    Ok(())
}

/// The global outbound proxy only supports HTTP(S).
pub fn validate_global_proxy_url(url: &str) -> anyhow::Result<()> {
    let url = url.trim();
    if url.is_empty() || url.starts_with("http://") || url.starts_with("https://") {
        return Ok(());
    }
    anyhow::bail!("全局代理仅支持 http:// 或 https://")
}

fn clamp_u32(value: u32, (min, max): (u32, u32)) -> u32 {
    value.clamp(min, max)
}

/// `streamingIdleTimeout` is special: `0` means "no idle timeout" and is kept
/// as-is, while anything below 60 (but not 0) is snapped up to 60.
fn normalize_streaming_idle_timeout(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        clamp_u32(
            value,
            (STREAMING_IDLE_TIMEOUT_MIN, STREAMING_IDLE_TIMEOUT_MAX),
        )
    }
}

fn clamp_error_rate_threshold(value: f64, fallback: f64) -> f64 {
    if value.is_nan() {
        fallback
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// Clean up a configuration coming from disk or from the UI.
///
/// Invalid listen addresses and privileged ports fall back to the defaults,
/// the app list is reduced to known ids (deduplicated and re-sorted into
/// `PROXY_APP_IDS` order, filling in anything missing), every numeric
/// parameter is clamped into the range the frontend accepts, and the global
/// proxy URL is trimmed.
pub fn normalize(config: RoutingConfig) -> RoutingConfig {
    let defaults = RoutingConfig::default();

    let listen_address = config
        .listen_address
        .trim()
        .parse::<std::net::Ipv4Addr>()
        .map(|address| address.to_string())
        .unwrap_or_else(|_| defaults.listen_address.clone());

    let listen_port = if config.listen_port < 1024 {
        defaults.listen_port
    } else {
        config.listen_port
    };

    let mut by_id: BTreeMap<String, AppProxyConfig> = BTreeMap::new();
    for app in config.apps {
        if !is_proxy_app_id(&app.app_id) {
            continue;
        }
        // 重复 id 保留第一个。
        by_id.entry(app.app_id.clone()).or_insert(app);
    }
    let apps = PROXY_APP_IDS
        .iter()
        .map(|app_id| match by_id.remove(*app_id) {
            Some(app) => normalize_app_proxy_config(*app_id, app),
            None => default_app_proxy_config(app_id),
        })
        .collect();

    RoutingConfig {
        enabled: config.enabled,
        listen_address,
        listen_port,
        enable_logging: config.enable_logging,
        show_routing_toggle_on_main: config.show_routing_toggle_on_main,
        show_failover_toggle_on_main: config.show_failover_toggle_on_main,
        apps,
        rectifier: config.rectifier,
        global_proxy_url: config.global_proxy_url.trim().to_string(),
        global_proxy_username: config.global_proxy_username,
        global_proxy_password: config.global_proxy_password,
    }
}

fn normalize_app_proxy_config(app_id: &str, app: AppProxyConfig) -> AppProxyConfig {
    let fallback = default_app_proxy_config(app_id);
    AppProxyConfig {
        app_id: app_id.to_string(),
        takeover: app.takeover,
        auto_failover_enabled: app.auto_failover_enabled,
        max_retries: clamp_u32(app.max_retries, MAX_RETRIES),
        streaming_first_byte_timeout: clamp_u32(
            app.streaming_first_byte_timeout,
            STREAMING_FIRST_BYTE_TIMEOUT,
        ),
        streaming_idle_timeout: normalize_streaming_idle_timeout(app.streaming_idle_timeout),
        non_streaming_timeout: clamp_u32(app.non_streaming_timeout, NON_STREAMING_TIMEOUT),
        circuit_failure_threshold: clamp_u32(
            app.circuit_failure_threshold,
            CIRCUIT_FAILURE_THRESHOLD,
        ),
        circuit_success_threshold: clamp_u32(
            app.circuit_success_threshold,
            CIRCUIT_SUCCESS_THRESHOLD,
        ),
        circuit_timeout_seconds: clamp_u32(app.circuit_timeout_seconds, CIRCUIT_TIMEOUT_SECONDS),
        circuit_error_rate_threshold: clamp_error_rate_threshold(
            app.circuit_error_rate_threshold,
            fallback.circuit_error_rate_threshold,
        ),
        circuit_min_requests: clamp_u32(app.circuit_min_requests, CIRCUIT_MIN_REQUESTS),
    }
}

/// The routing-wide fields as stored in the `kv` table: everything except the
/// per-app rows and the password.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RoutingConfigDocument {
    enabled: bool,
    listen_address: String,
    listen_port: u16,
    enable_logging: bool,
    show_routing_toggle_on_main: bool,
    show_failover_toggle_on_main: bool,
    rectifier: RectifierConfig,
    global_proxy_url: String,
    global_proxy_username: String,
    /// Tolerated for forward compatibility: older documents may still contain
    /// an app list, but the `proxy_config` table is authoritative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    apps: Option<Vec<AppProxyConfig>>,
}

impl Default for RoutingConfigDocument {
    fn default() -> Self {
        let defaults = RoutingConfig::default();
        Self::from_config(&defaults)
    }
}

impl RoutingConfigDocument {
    fn from_config(config: &RoutingConfig) -> Self {
        Self {
            enabled: config.enabled,
            listen_address: config.listen_address.clone(),
            listen_port: config.listen_port,
            enable_logging: config.enable_logging,
            show_routing_toggle_on_main: config.show_routing_toggle_on_main,
            show_failover_toggle_on_main: config.show_failover_toggle_on_main,
            rectifier: config.rectifier.clone(),
            global_proxy_url: config.global_proxy_url.clone(),
            global_proxy_username: config.global_proxy_username.clone(),
            apps: None,
        }
    }

    fn into_config(self) -> RoutingConfig {
        RoutingConfig {
            enabled: self.enabled,
            listen_address: self.listen_address,
            listen_port: self.listen_port,
            enable_logging: self.enable_logging,
            show_routing_toggle_on_main: self.show_routing_toggle_on_main,
            show_failover_toggle_on_main: self.show_failover_toggle_on_main,
            apps: Vec::new(),
            rectifier: self.rectifier,
            global_proxy_url: self.global_proxy_url,
            global_proxy_username: self.global_proxy_username,
            global_proxy_password: None,
        }
    }
}

/// One failover queue row together with its circuit state.
#[derive(Debug, Clone, PartialEq)]
pub struct FailoverQueueRow {
    pub provider_id: String,
    pub priority: i64,
    pub circuit_state: String,
}

/// SQLite-backed routing configuration store.
pub struct RoutingStore {
    conn: rusqlite::Connection,
}

impl RoutingStore {
    /// Open the store at `<app state dir>/ccp.db`.
    pub fn open_default() -> anyhow::Result<Self> {
        Self::open(crate::ccp_db::default_db_path())
    }

    /// Open the store at an explicit database path.
    pub fn open(path: PathBuf) -> anyhow::Result<Self> {
        Self::from_connection(crate::ccp_db::open(path.as_path())?)
    }

    /// Wrap an already-open CCP database connection.
    pub fn from_connection(conn: rusqlite::Connection) -> anyhow::Result<Self> {
        Ok(Self { conn })
    }

    pub fn database_path() -> PathBuf {
        crate::ccp_db::default_db_path()
    }

    /// Load the full configuration: routing-wide fields from `kv`, per-app rows
    /// from `proxy_config`. Missing or unparsable values fall back to defaults.
    pub fn load(&self) -> anyhow::Result<RoutingConfig> {
        let document = match crate::ccp_db::kv_get(&self.conn, ROUTING_CONFIG_KEY)? {
            Some(raw) => match serde_json::from_str::<RoutingConfigDocument>(&raw) {
                Ok(document) => document,
                Err(error) => {
                    eprintln!("路由配置解析失败，使用默认值：{error}");
                    RoutingConfigDocument::default()
                }
            },
            None => RoutingConfigDocument::default(),
        };

        let mut config = document.into_config();
        let mut apps = Vec::new();
        for app_id in PROXY_APP_IDS {
            apps.push(self.load_app_config(app_id)?);
        }
        config.apps = apps;
        Ok(normalize(config))
    }

    fn load_app_config(&self, app_id: &str) -> anyhow::Result<AppProxyConfig> {
        let mut statement = self
            .conn
            .prepare("SELECT config_json FROM proxy_config WHERE app_id = ?1")?;
        let mut rows = statement.query([app_id])?;
        let Some(row) = rows.next()? else {
            return Ok(default_app_proxy_config(app_id));
        };
        let raw: String = row.get(0)?;
        match serde_json::from_str::<AppProxyConfig>(&raw) {
            Ok(app) => Ok(app),
            Err(error) => {
                eprintln!("路由应用 {app_id} 配置解析失败，使用默认值：{error}");
                Ok(default_app_proxy_config(app_id))
            }
        }
    }

    /// The stored global outbound proxy password, if any.
    pub fn global_proxy_password(&self) -> anyhow::Result<Option<String>> {
        crate::ccp_db::kv_get(&self.conn, ROUTING_PASSWORD_KEY)
    }

    /// Validate, normalize and persist `config` in a single transaction.
    ///
    /// Password rule: a non-empty `globalProxyPassword` replaces the stored
    /// one; if the global proxy URL ends up empty the stored password is
    /// deleted; otherwise the stored password is left untouched. The returned
    /// configuration always carries `global_proxy_password = None`.
    pub fn save(&mut self, config: RoutingConfig) -> anyhow::Result<RoutingConfig> {
        validate_global_proxy_url(&config.global_proxy_url)?;
        let config = normalize(config);

        let requested_password = config
            .global_proxy_password
            .as_deref()
            .map(str::trim)
            .filter(|password| !password.is_empty())
            .map(str::to_string);

        let document = RoutingConfigDocument::from_config(&config);
        let encoded = serde_json::to_string(&document).context("序列化路由配置失败")?;
        let apps = config.apps.clone();

        let transaction = self.conn.transaction()?;
        transaction.execute(
            "INSERT INTO kv (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![ROUTING_CONFIG_KEY, encoded],
        )?;
        for app in &apps {
            let encoded_app = serde_json::to_string(app).context("序列化路由应用配置失败")?;
            transaction.execute(
                "INSERT INTO proxy_config (app_id, config_json) VALUES (?1, ?2)
                 ON CONFLICT(app_id) DO UPDATE SET config_json = excluded.config_json",
                rusqlite::params![app.app_id, encoded_app],
            )?;
        }

        if let Some(password) = requested_password {
            transaction.execute(
                "INSERT INTO kv (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![ROUTING_PASSWORD_KEY, password],
            )?;
        } else if config.global_proxy_url.is_empty() {
            transaction.execute(
                "DELETE FROM kv WHERE key = ?1",
                rusqlite::params![ROUTING_PASSWORD_KEY],
            )?;
        }
        transaction.commit()?;

        let mut saved = config;
        saved.global_proxy_password = None;
        Ok(saved)
    }

    /// Turn the local routing proxy on or off.
    pub fn set_enabled(&mut self, enabled: bool) -> anyhow::Result<RoutingConfig> {
        let mut config = self.load()?;
        config.enabled = enabled;
        self.save(config)
    }

    /// Turn takeover on or off for a single app.
    pub fn set_app_takeover(
        &mut self,
        app_id: &str,
        takeover: bool,
    ) -> anyhow::Result<RoutingConfig> {
        validate_app_id(app_id)?;
        let mut config = self.load()?;
        for app in &mut config.apps {
            if app.app_id == app_id {
                app.takeover = takeover;
            }
        }
        self.save(config)
    }

    /// The failover queue for one app, ordered by priority ascending.
    pub fn queue(&self, app_id: &str) -> anyhow::Result<Vec<FailoverQueueRow>> {
        validate_app_id(app_id)?;
        let mut statement = self.conn.prepare(
            "SELECT q.provider_id, q.priority,
                    COALESCE(c.state, 'closed') AS circuit_state
             FROM failover_queue q
             LEFT JOIN circuit_state c
               ON c.app_id = q.app_id AND c.provider_id = q.provider_id
             WHERE q.app_id = ?1
             ORDER BY q.priority ASC, q.provider_id ASC",
        )?;
        let rows = statement.query_map([app_id], |row| {
            Ok(FailoverQueueRow {
                provider_id: row.get(0)?,
                priority: row.get(1)?,
                circuit_state: row.get(2)?,
            })
        })?;
        let mut queue = Vec::new();
        for row in rows {
            queue.push(row?);
        }
        Ok(queue)
    }

    /// Append a provider to an app's failover queue. Existing entries keep
    /// their priority.
    pub fn add_to_queue(&mut self, app_id: &str, provider_id: &str) -> anyhow::Result<()> {
        validate_app_id(app_id)?;
        validate_provider_id(provider_id)?;
        let provider_id = provider_id.trim().to_string();

        let exists: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM failover_queue WHERE app_id = ?1 AND provider_id = ?2",
            rusqlite::params![app_id, provider_id],
            |row| row.get(0),
        )?;
        if exists > 0 {
            return Ok(());
        }

        let next_priority: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(priority), 0) + 1 FROM failover_queue WHERE app_id = ?1",
            [app_id],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO failover_queue (app_id, provider_id, priority) VALUES (?1, ?2, ?3)",
            rusqlite::params![app_id, provider_id, next_priority],
        )?;
        Ok(())
    }

    /// Remove a provider from an app's queue, renumber the remaining entries
    /// from 1, and drop its circuit state.
    pub fn remove_from_queue(&mut self, app_id: &str, provider_id: &str) -> anyhow::Result<()> {
        validate_app_id(app_id)?;
        validate_provider_id(provider_id)?;
        let provider_id = provider_id.trim().to_string();

        let transaction = self.conn.transaction()?;
        transaction.execute(
            "DELETE FROM failover_queue WHERE app_id = ?1 AND provider_id = ?2",
            rusqlite::params![app_id, provider_id],
        )?;
        transaction.execute(
            "DELETE FROM circuit_state WHERE app_id = ?1 AND provider_id = ?2",
            rusqlite::params![app_id, provider_id],
        )?;

        let remaining: Vec<String> = {
            let mut statement = transaction.prepare(
                "SELECT provider_id FROM failover_queue
                 WHERE app_id = ?1
                 ORDER BY priority ASC, provider_id ASC",
            )?;
            let rows = statement.query_map([app_id], |row| row.get::<_, String>(0))?;
            let mut remaining = Vec::new();
            for row in rows {
                remaining.push(row?);
            }
            remaining
        };
        for (index, remaining_provider) in remaining.iter().enumerate() {
            transaction.execute(
                "UPDATE failover_queue SET priority = ?3
                 WHERE app_id = ?1 AND provider_id = ?2",
                rusqlite::params![app_id, remaining_provider, (index as i64) + 1],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Force a provider's circuit back to `closed` and zero its counters.
    pub fn reset_circuit(&mut self, app_id: &str, provider_id: &str) -> anyhow::Result<()> {
        validate_app_id(app_id)?;
        validate_provider_id(provider_id)?;
        let provider_id = provider_id.trim().to_string();

        self.conn.execute(
            "INSERT INTO circuit_state
                 (app_id, provider_id, state, consecutive_failures, consecutive_successes, opened_at_ms)
             VALUES (?1, ?2, 'closed', 0, 0, NULL)
             ON CONFLICT(app_id, provider_id) DO UPDATE SET
                 state = 'closed',
                 consecutive_failures = 0,
                 consecutive_successes = 0,
                 opened_at_ms = NULL",
            rusqlite::params![app_id, provider_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn temp_db(label: &str) -> PathBuf {
        let unique = format!(
            "ccp-routing-config-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).expect("创建测试临时目录");
        dir.join("ccp.db")
    }

    fn open_store(path: &Path) -> RoutingStore {
        RoutingStore::open(path.to_path_buf()).expect("打开路由存储")
    }

    fn cleanup(path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn routing_config_default_matches_contract() {
        let config = RoutingConfig::default();

        assert_eq!(config.apps.len(), 4);
        let ids: Vec<&str> = config.apps.iter().map(|app| app.app_id.as_str()).collect();
        assert_eq!(ids, PROXY_APP_IDS.to_vec());
        assert_eq!(config.apps[0].max_retries, 6);
        assert_eq!(config.apps[2].max_retries, 3);
        assert_eq!(config.apps[0].streaming_first_byte_timeout, 90);
        assert_eq!(config.apps[0].streaming_idle_timeout, 180);
        assert_eq!(config.apps[0].non_streaming_timeout, 600);
        assert_eq!(config.apps[0].circuit_failure_threshold, 8);
        assert_eq!(config.apps[0].circuit_success_threshold, 3);
        assert_eq!(config.apps[0].circuit_timeout_seconds, 90);
        assert_eq!(config.apps[0].circuit_error_rate_threshold, 0.7);
        assert_eq!(config.apps[0].circuit_min_requests, 15);
        assert_eq!(config.apps[1].circuit_error_rate_threshold, 0.6);
        assert_eq!(config.apps[3].circuit_min_requests, 10);
        assert!(config.apps.iter().all(|app| !app.takeover));
        assert!(config.apps.iter().all(|app| !app.auto_failover_enabled));
        assert_eq!(config.listen_port, 57321);
        assert_eq!(config.listen_address, "127.0.0.1");
        assert!(!config.enabled);
        assert!(config.enable_logging);
        assert!(config.rectifier.enabled);
        assert!(config.rectifier.thinking_signature);
        assert!(config.rectifier.thinking_budget);
        assert!(config.rectifier.media_fallback);
        assert!(config.rectifier.media_heuristic);

        let encoded = serde_json::to_string(&config).expect("序列化默认配置");
        assert!(!encoded.contains("globalProxyPassword"));
        assert!(encoded.contains("\"listenPort\""));
        assert!(encoded.contains("\"autoFailoverEnabled\""));
        assert!(encoded.contains("\"mediaHeuristic\""));
        assert!(!encoded.contains("listen_port"));
    }

    #[test]
    fn routing_config_normalize_clamps_and_fills_apps() {
        let mut config = RoutingConfig::default();
        config.listen_address = "not-an-address".to_string();
        config.listen_port = 80;
        config.apps = vec![
            AppProxyConfig {
                app_id: "codex".to_string(),
                max_retries: 99,
                streaming_first_byte_timeout: 0,
                streaming_idle_timeout: 30,
                non_streaming_timeout: 5,
                circuit_failure_threshold: 0,
                circuit_success_threshold: 50,
                circuit_timeout_seconds: 900,
                circuit_error_rate_threshold: 2.5,
                circuit_min_requests: 1,
                ..AppProxyConfig::default()
            },
            AppProxyConfig {
                app_id: "codex".to_string(),
                max_retries: 1,
                ..AppProxyConfig::default()
            },
            AppProxyConfig {
                app_id: "gemini".to_string(),
                streaming_idle_timeout: 0,
                circuit_error_rate_threshold: f64::NAN,
                ..AppProxyConfig::default()
            },
            AppProxyConfig {
                app_id: "unknown-app".to_string(),
                ..AppProxyConfig::default()
            },
        ];

        let normalized = normalize(config);

        assert_eq!(normalized.listen_address, "127.0.0.1");
        assert_eq!(normalized.listen_port, 57321);
        assert_eq!(normalized.apps.len(), 4);
        let ids: Vec<&str> = normalized
            .apps
            .iter()
            .map(|app| app.app_id.as_str())
            .collect();
        assert_eq!(ids, PROXY_APP_IDS.to_vec());

        // 重复的 codex 保留第一个，其余被剔除；claude / claude-desktop 补齐默认值。
        let codex = &normalized.apps[2];
        assert_eq!(codex.max_retries, 10);
        assert_eq!(codex.streaming_first_byte_timeout, 1);
        assert_eq!(codex.streaming_idle_timeout, 60);
        assert_eq!(codex.non_streaming_timeout, 60);
        assert_eq!(codex.circuit_failure_threshold, 1);
        assert_eq!(codex.circuit_success_threshold, 10);
        assert_eq!(codex.circuit_timeout_seconds, 300);
        assert_eq!(codex.circuit_error_rate_threshold, 1.0);
        assert_eq!(codex.circuit_min_requests, 5);
        assert_eq!(normalized.apps[0], default_app_proxy_config("claude"));
        assert_eq!(
            normalized.apps[1],
            default_app_proxy_config("claude-desktop")
        );

        // idle = 0 表示"不限制"，必须原样保留；NaN 回到默认值。
        let gemini = &normalized.apps[3];
        assert_eq!(gemini.streaming_idle_timeout, 0);
        assert_eq!(
            gemini.circuit_error_rate_threshold,
            default_app_proxy_config("gemini").circuit_error_rate_threshold
        );

        // 空列表同样补齐全部应用。
        let mut empty = RoutingConfig::default();
        empty.apps = Vec::new();
        assert_eq!(normalize(empty).apps, default_apps());
    }

    #[test]
    fn routing_store_roundtrip_and_password_rules() {
        let path = temp_db("roundtrip");
        let mut store = open_store(&path);

        // 从未写入过的数据库返回默认配置。
        assert_eq!(
            store.load().expect("读取默认配置"),
            RoutingConfig::default()
        );

        let mut config = RoutingConfig::default();
        config.enabled = true;
        config.listen_port = 58000;
        config.apps[1].takeover = true;
        config.apps[1].max_retries = 9;
        config.global_proxy_url = "http://127.0.0.1:7890".to_string();
        config.global_proxy_username = "user".to_string();
        config.global_proxy_password = Some("hunter2".to_string());

        let saved = store.save(config.clone()).expect("保存路由配置");
        assert_eq!(saved.global_proxy_password, None);

        let loaded = store.load().expect("读取路由配置");
        assert_eq!(loaded, saved);
        assert_eq!(loaded.global_proxy_password, None);
        assert_eq!(loaded.apps[1].max_retries, 9);
        assert!(loaded.apps[1].takeover);
        assert_eq!(
            store.global_proxy_password().expect("读取代理密码"),
            Some("hunter2".to_string())
        );

        // 再次保存但不带密码：已有密码保持不变。
        let mut again = saved.clone();
        again.global_proxy_password = None;
        again.listen_port = 58001;
        let saved_again = store.save(again).expect("二次保存路由配置");
        assert_eq!(saved_again.listen_port, 58001);
        assert_eq!(
            store.global_proxy_password().expect("读取代理密码"),
            Some("hunter2".to_string())
        );

        // 清空代理地址：密码被删除。
        let mut cleared = saved_again.clone();
        cleared.global_proxy_url = String::new();
        cleared.global_proxy_username = String::new();
        let saved_cleared = store.save(cleared).expect("保存清空代理的配置");
        assert_eq!(saved_cleared.global_proxy_url, "");
        assert_eq!(store.global_proxy_password().expect("读取代理密码"), None);

        // 非 http(s) 的代理地址直接报错，不做静默清理。
        let mut socks = saved_cleared.clone();
        socks.global_proxy_url = "socks5://127.0.0.1:1080".to_string();
        let error = store.save(socks).expect_err("socks5 代理应被拒绝");
        assert_eq!(error.to_string(), "全局代理仅支持 http:// 或 https://");
        assert_eq!(
            store.load().expect("读取路由配置").global_proxy_url,
            String::new()
        );

        // 开关类操作同样是 load → 修改 → save。
        let enabled = store.set_enabled(true).expect("启用路由");
        assert!(enabled.enabled);
        let takeover = store
            .set_app_takeover("gemini", true)
            .expect("开启 gemini 接管");
        assert!(takeover.apps[3].takeover);
        let error = store
            .set_app_takeover("unknown-app", true)
            .expect_err("未知应用应报错");
        assert_eq!(error.to_string(), "未知的路由应用：unknown-app");

        drop(store);
        cleanup(&path);
    }

    #[test]
    fn routing_failover_queue_priorities_and_circuit_state() {
        let path = temp_db("queue");
        let mut store = open_store(&path);

        for provider in ["alpha", "beta", "gamma"] {
            store.add_to_queue("codex", provider).expect("加入队列");
        }
        let queue = store.queue("codex").expect("读取队列");
        let priorities: Vec<i64> = queue.iter().map(|entry| entry.priority).collect();
        assert_eq!(priorities, vec![1, 2, 3]);
        assert!(queue.iter().all(|entry| entry.circuit_state == "closed"));
        assert_eq!(
            queue
                .iter()
                .map(|entry| entry.provider_id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "beta", "gamma"]
        );

        // 重复加入不改变任何优先级。
        store.add_to_queue("codex", "beta").expect("重复加入队列");
        let queue = store.queue("codex").expect("读取队列");
        assert_eq!(
            queue.iter().map(|entry| entry.priority).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );

        // 熔断器重置为 closed 并清零计数。
        store.reset_circuit("codex", "beta").expect("重置熔断器");
        let state: (String, i64, i64) = store
            .conn
            .query_row(
                "SELECT state, consecutive_failures, consecutive_successes
                 FROM circuit_state WHERE app_id = ?1 AND provider_id = ?2",
                rusqlite::params!["codex", "beta"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("读取熔断状态");
        assert_eq!(state, ("closed".to_string(), 0, 0));

        // 移除中间一项后重新编号为 1、2，并清掉对应的熔断状态。
        store.remove_from_queue("codex", "beta").expect("移出队列");
        let queue = store.queue("codex").expect("读取队列");
        assert_eq!(
            queue
                .iter()
                .map(|entry| (entry.provider_id.as_str(), entry.priority))
                .collect::<Vec<_>>(),
            vec![("alpha", 1), ("gamma", 2)]
        );
        let circuit_rows: i64 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM circuit_state WHERE provider_id = ?1",
                ["beta"],
                |row| row.get(0),
            )
            .expect("统计熔断状态行数");
        assert_eq!(circuit_rows, 0);

        // 队列是按应用隔离的。
        assert!(store.queue("claude").expect("读取 claude 队列").is_empty());

        // 非法 app_id / provider_id 都是错误。
        let error = store.queue("unknown-app").expect_err("未知应用应报错");
        assert_eq!(error.to_string(), "未知的路由应用：unknown-app");
        let error = store
            .add_to_queue("codex", "   ")
            .expect_err("空供应商应报错");
        assert_eq!(error.to_string(), "供应商 ID 不能为空");
        let error = store
            .remove_from_queue("nope", "alpha")
            .expect_err("未知应用应报错");
        assert_eq!(error.to_string(), "未知的路由应用：nope");

        drop(store);
        cleanup(&path);
    }
}
