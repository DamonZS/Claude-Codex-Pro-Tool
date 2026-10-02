//! Failover queue + circuit breaker for the local routing proxy.
//!
//! This module is the bridge between the routing configuration persisted in
//! `ccp.db` (`RoutingStore`) and the request forwarding code in
//! `protocol_proxy`. It answers three questions for every forwarded request:
//!
//! 1. Which providers may this request be sent to, in which order?
//!    ([`plan_for`] / [`candidate_order`])
//! 2. Is a provider's circuit currently allowed to accept traffic?
//!    ([`circuit_allows`])
//! 3. What does a response do to that circuit? ([`record_outcome`] /
//!    [`record`])
//!
//! Failover is strictly opt-in: it only kicks in when the routing config is
//! enabled **and** the app's own `autoFailoverEnabled` switch is on. Anything
//! else must behave exactly like a single-provider forward — that is the
//! compatibility contract this module is built around, and it is what makes it
//! safe to call from the hot path.

use crate::routing_config::{AppProxyConfig, CircuitRow, FailoverQueueRow, RoutingStore};

/// The provider id used by the circuit state machine (and only inside this
/// module) to mean "this row's state is not one of the three known states".
const STATE_CLOSED: &str = "closed";
const STATE_OPEN: &str = "open";
const STATE_HALF_OPEN: &str = "half_open";

/// Request-level failures that switching providers would not fix.
///
/// CC Switch treats these as the caller's mistake (malformed body, wrong
/// content type, unsupported media, bad range, oversize payload...) so the
/// failover queue must not be walked: the next provider would answer the same
/// way while burning a request.
fn is_request_level_error(status: u16) -> bool {
    matches!(status, 400 | 405 | 406 | 413 | 414 | 415 | 422 | 501)
}

/// Whether an upstream status code should make the routing proxy try the next
/// provider in the failover queue.
///
/// `true` for timeouts (408), rate limits (429), all 5xx except 501 (Not
/// Implemented is a permanent per-endpoint verdict, not a transient outage),
/// and the auth/route failures 401, 403 and 404 — a key that upstream rejects
/// or a route it does not serve is exactly what the next provider is for.
pub fn is_failover_status(status: u16) -> bool {
    if is_request_level_error(status) {
        return false;
    }
    matches!(status, 401 | 403 | 404 | 408 | 429) || (500..600).contains(&status)
}

/// Milliseconds since the circuit was opened, if it was opened with a
/// timestamp. Rows written by older code may have `None`; treating those as
/// "just opened" is the conservative choice (keep the circuit shut) and it
/// lets the timeout reopen traffic on the next evaluation.
fn elapsed_since_open(row: &CircuitRow, now_ms: i64) -> Option<i64> {
    row.opened_at_ms.map(|opened_at| now_ms - opened_at)
}

/// Whether the circuit's timeout has elapsed, i.e. an open circuit may be
/// treated as half-open.
fn open_timeout_elapsed(row: &CircuitRow, cfg: &AppProxyConfig, now_ms: i64) -> bool {
    let Some(elapsed) = elapsed_since_open(row, now_ms) else {
        return false;
    };
    elapsed >= i64::from(cfg.circuit_timeout_seconds) * 1000
}

/// Whether traffic may be sent to a provider right now.
///
/// * `closed` → yes.
/// * `half_open` → yes; the single probe request decides the outcome.
/// * `open` → no, until `circuit_timeout_seconds` have passed since
///   `opened_at_ms`; after that the provider gets a half-open probe.
pub fn circuit_allows(row: &CircuitRow, cfg: &AppProxyConfig, now_ms: i64) -> bool {
    match row.state.as_str() {
        STATE_OPEN => open_timeout_elapsed(row, cfg, now_ms),
        _ => true,
    }
}

/// Fold one request outcome into a provider's circuit state.
///
/// Successes and failures reset each other's streak, and the state transitions
/// follow the classic three-state breaker:
///
/// * a success while `half_open` (or while `open` past its timeout) needs
///   `circuit_success_threshold` in a row to close the circuit, otherwise the
///   circuit stays half-open and keeps probing;
/// * a failure while probing reopens the circuit immediately with a fresh
///   `opened_at_ms`, so the timeout starts over;
/// * a failure while `closed` opens the circuit once
///   `circuit_failure_threshold` is reached.
pub fn record_outcome(
    mut row: CircuitRow,
    cfg: &AppProxyConfig,
    success: bool,
    now_ms: i64,
) -> CircuitRow {
    // 探测中：half_open，或 open 但超时已经过去（此时按 half_open 处理）。
    let probing = row.state == STATE_HALF_OPEN
        || (row.state == STATE_OPEN && open_timeout_elapsed(&row, cfg, now_ms));

    if success {
        row.consecutive_failures = 0;
        row.consecutive_successes = row.consecutive_successes.saturating_add(1);
        if probing {
            if row.consecutive_successes >= cfg.circuit_success_threshold {
                row.state = STATE_CLOSED.to_string();
                row.consecutive_failures = 0;
                row.consecutive_successes = 0;
                row.opened_at_ms = None;
            } else {
                row.state = STATE_HALF_OPEN.to_string();
            }
        }
        return row;
    }

    row.consecutive_successes = 0;
    row.consecutive_failures = row.consecutive_failures.saturating_add(1);
    if probing
        || (row.state == STATE_CLOSED && row.consecutive_failures >= cfg.circuit_failure_threshold)
    {
        row.state = STATE_OPEN.to_string();
        row.opened_at_ms = Some(now_ms);
    }
    row
}

/// The order in which providers should be tried: the active provider first,
/// then the failover queue in priority order, de-duplicated.
///
/// An empty `active_id` (nothing configured yet) yields just the queue, and a
/// provider that is both active and queued is only tried once.
pub fn candidate_order(active_id: &str, queue: &[FailoverQueueRow]) -> Vec<String> {
    let mut candidates = Vec::new();
    let active_id = active_id.trim();
    if !active_id.is_empty() {
        candidates.push(active_id.to_string());
    }
    for entry in queue {
        let provider_id = entry.provider_id.trim();
        if provider_id.is_empty() || candidates.iter().any(|candidate| candidate == provider_id) {
            continue;
        }
        candidates.push(provider_id.to_string());
    }
    candidates
}

/// Everything the forwarding loop needs to know about one request.
#[derive(Debug, Clone, PartialEq)]
pub struct FailoverPlan {
    pub app_id: String,
    /// `false` means "forward exactly like before": one provider, no retries.
    pub enabled: bool,
    pub max_retries: u32,
    pub cfg: AppProxyConfig,
    /// The providers to try, in order. When disabled this is just the active
    /// provider.
    pub candidates: Vec<String>,
}

impl FailoverPlan {
    /// The disabled plan: exactly the active provider, no extra attempts.
    fn single(app_id: &str, active_id: &str, cfg: AppProxyConfig) -> Self {
        Self {
            app_id: app_id.to_string(),
            enabled: false,
            max_retries: cfg.max_retries,
            cfg,
            candidates: vec![active_id.to_string()],
        }
    }
}

/// Build a plan from an already-open store. This is the testable core of
/// [`plan_for`]; `now_ms` is injected so the circuit timeouts are deterministic.
pub fn plan_from_store(
    store: &RoutingStore,
    app_id: &str,
    active_id: &str,
    now_ms: i64,
) -> FailoverPlan {
    let config = store.load().unwrap_or_default();
    let cfg = config
        .apps
        .iter()
        .find(|app| app.app_id == app_id)
        .cloned()
        .unwrap_or_else(|| crate::routing_config::default_app_proxy_config(app_id));
    let enabled = config.enabled && cfg.auto_failover_enabled;
    if !enabled {
        return FailoverPlan::single(app_id, active_id, cfg);
    }
    let queue = store.queue(app_id).unwrap_or_default();
    let ordered = candidate_order(active_id, &queue);
    let mut candidates: Vec<String> = ordered
        .iter()
        .filter(|provider_id| {
            store
                .circuit(app_id, provider_id)
                .map(|row| circuit_allows(&row, &cfg, now_ms))
                // 读不到熔断状态时宁可放行，也不要因为一次读取失败就断掉全部候选。
                .unwrap_or(true)
        })
        .cloned()
        .collect();

    // 全部熔断时仍然要给主供应商一次机会，否则请求会被本地直接拒绝。
    if candidates.is_empty() {
        candidates.push(active_id.to_string());
    }
    candidates.truncate(cfg.max_retries as usize + 1);

    FailoverPlan {
        app_id: app_id.to_string(),
        enabled: true,
        max_retries: cfg.max_retries,
        cfg,
        candidates,
    }
}

/// Plan the provider sequence for one request.
///
/// Never creates the database: when `ccp.db` is absent, unreadable, or the
/// config cannot be read, this returns a disabled plan for the active provider
/// only. That keeps every caller that runs before routing is ever configured
/// on the exact pre-failover code path.
pub fn plan_for(app_id: &str, active_id: &str) -> FailoverPlan {
    let now_ms = now_ms();
    let path = crate::ccp_db::default_db_path();
    if !path.is_file() {
        return FailoverPlan::single(
            app_id,
            active_id,
            crate::routing_config::default_app_proxy_config(app_id),
        );
    }
    match RoutingStore::open(path) {
        Ok(store) => match store.load() {
            Ok(_) => plan_from_store(&store, app_id, active_id, now_ms),
            Err(error) => {
                eprintln!("读取路由配置失败，跳过故障转移：{error}");
                FailoverPlan::single(
                    app_id,
                    active_id,
                    crate::routing_config::default_app_proxy_config(app_id),
                )
            }
        },
        Err(error) => {
            eprintln!("打开路由数据库失败，跳过故障转移：{error}");
            FailoverPlan::single(
                app_id,
                active_id,
                crate::routing_config::default_app_proxy_config(app_id),
            )
        }
    }
}

/// Record one upstream outcome for a provider. Only meaningful — and only
/// called — for enabled plans.
///
/// Failures are logged and swallowed: a circuit that cannot be persisted must
/// never turn a proxied request into a local error.
pub fn record(app_id: &str, provider_id: &str, success: bool) {
    if let Err(error) = record_inner(app_id, provider_id, success, now_ms()) {
        eprintln!("写入熔断状态失败：{error}");
    }
}

fn record_inner(app_id: &str, provider_id: &str, success: bool, now_ms: i64) -> anyhow::Result<()> {
    let path = crate::ccp_db::default_db_path();
    if !path.is_file() {
        return Ok(());
    }
    let mut store = RoutingStore::open(path)?;
    let config = store.load()?;
    let Some(cfg) = config.apps.iter().find(|app| app.app_id == app_id) else {
        return Ok(());
    };
    let cfg = cfg.clone();
    let row = store.circuit(app_id, provider_id)?;
    let updated = record_outcome(row, &cfg, success, now_ms);
    store.put_circuit(app_id, provider_id, &updated)?;
    Ok(())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing_config::{AppProxyConfig, RoutingConfig};
    use std::path::{Path, PathBuf};

    fn temp_db(label: &str) -> PathBuf {
        let unique = format!(
            "ccp-routing-failover-{label}-{}-{}",
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

    fn cleanup(path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    fn config_for(app_id: &str) -> AppProxyConfig {
        AppProxyConfig {
            app_id: app_id.to_string(),
            auto_failover_enabled: true,
            max_retries: 3,
            circuit_failure_threshold: 3,
            circuit_success_threshold: 2,
            circuit_timeout_seconds: 60,
            ..AppProxyConfig::default()
        }
    }

    fn closed() -> CircuitRow {
        CircuitRow::closed()
    }

    #[test]
    fn failover_status_table_matches_ccswitch_classification() {
        for status in [408u16, 429, 500, 502, 503, 504, 599, 401, 403, 404] {
            assert!(is_failover_status(status), "{status} 应触发故障转移");
        }
        for status in [400u16, 405, 406, 413, 414, 415, 422, 501] {
            assert!(!is_failover_status(status), "{status} 不应触发故障转移");
        }
        for status in [200u16, 201, 204, 226, 301, 302, 304, 307, 308] {
            assert!(!is_failover_status(status), "{status} 不应触发故障转移");
        }
        // 400 与 4xx/5xx 的边界必须明确区分。
        assert!(!is_failover_status(499));
        assert!(is_failover_status(500));
        assert!(!is_failover_status(501));
        assert!(is_failover_status(502));
    }

    #[test]
    fn circuit_machine_walks_closed_open_half_open_closed() {
        let cfg = config_for("codex");
        let now = 1_000_000_i64;

        // closed → 连续失败达到阈值 → open。
        let mut row = closed();
        for _ in 0..cfg.circuit_failure_threshold - 1 {
            row = record_outcome(row, &cfg, false, now);
            assert_eq!(row.state, "closed");
        }
        row = record_outcome(row, &cfg, false, now);
        assert_eq!(row.state, "open");
        assert_eq!(row.consecutive_failures, cfg.circuit_failure_threshold);
        assert_eq!(row.consecutive_successes, 0);
        assert_eq!(row.opened_at_ms, Some(now));

        // open 且未超时：拒绝。
        assert!(!circuit_allows(&row, &cfg, now));
        assert!(!circuit_allows(&row, &cfg, now + 59_999));

        // open 且超时：放行（按 half_open 处理）。
        let half_open_at = now + 60_000;
        assert!(circuit_allows(&row, &cfg, half_open_at));

        // 探测期成功不足成功阈值：仍停在 half_open。
        let probed = record_outcome(row.clone(), &cfg, true, half_open_at);
        assert_eq!(probed.state, "half_open");
        assert_eq!(probed.consecutive_successes, 1);
        assert_eq!(probed.consecutive_failures, 0);
        assert!(circuit_allows(&probed, &cfg, half_open_at));

        // 达到成功阈值：closed 且计数清零。
        let recovered = record_outcome(probed, &cfg, true, half_open_at);
        assert_eq!(recovered.state, "closed");
        assert_eq!(recovered.consecutive_successes, 0);
        assert_eq!(recovered.consecutive_failures, 0);
        assert_eq!(recovered.opened_at_ms, None);
        assert!(circuit_allows(&recovered, &cfg, half_open_at));

        // 5xx 里 501 不参与熔断的判定由 is_failover_status 负责，这里验证
        // half_open 期间失败会立刻回到 open 并重置计时。
        let failed_probe = record_outcome(
            CircuitRow {
                state: "half_open".to_string(),
                consecutive_failures: 3,
                consecutive_successes: 1,
                opened_at_ms: Some(now),
            },
            &cfg,
            false,
            half_open_at,
        );
        assert_eq!(failed_probe.state, "open");
        assert_eq!(failed_probe.opened_at_ms, Some(half_open_at));
        assert_eq!(failed_probe.consecutive_failures, 4);
        assert_eq!(failed_probe.consecutive_successes, 0);
        assert!(!circuit_allows(&failed_probe, &cfg, half_open_at));
        assert!(circuit_allows(&failed_probe, &cfg, half_open_at + 60_000));
    }

    #[test]
    fn circuit_success_before_timeout_stays_shut() {
        let cfg = config_for("codex");
        let now = 5_000_000_i64;
        let row = CircuitRow {
            state: "open".to_string(),
            consecutive_failures: 4,
            consecutive_successes: 0,
            opened_at_ms: Some(now),
        };

        // 未超时：直接拒绝，不产生 half_open。
        assert!(!circuit_allows(&row, &cfg, now + 1));
        let rejected = record_outcome(row.clone(), &cfg, true, now + 1);
        assert_eq!(rejected.state, "open");
        assert_eq!(rejected.consecutive_successes, 1);
        assert_eq!(rejected.consecutive_failures, 0);
        assert_eq!(
            rejected.opened_at_ms,
            Some(now),
            "未超时的成功不得改写开启时间"
        );
    }

    #[test]
    fn circuit_success_resets_failure_streak_while_closed() {
        let cfg = config_for("codex");
        let now = 7_000_000_i64;

        let mut row = record_outcome(closed(), &cfg, false, now);
        row = record_outcome(row, &cfg, false, now);
        assert_eq!(row.consecutive_failures, 2);
        row = record_outcome(row, &cfg, true, now);
        assert_eq!(row.state, "closed");
        assert_eq!(row.consecutive_failures, 0);
        assert_eq!(row.consecutive_successes, 1);

        // 计数器清零后需要重新累计到阈值才会熔断。
        let mut row = closed();
        for _ in 0..cfg.circuit_failure_threshold {
            row = record_outcome(row, &cfg, false, now);
        }
        assert_eq!(row.state, "open");
        let closed_again = record_outcome(
            CircuitRow {
                state: "closed".to_string(),
                consecutive_failures: 0,
                consecutive_successes: 9,
                opened_at_ms: None,
            },
            &cfg,
            true,
            now,
        );
        assert_eq!(closed_again.state, "closed");
        assert_eq!(closed_again.consecutive_successes, 10);
    }

    #[test]
    fn circuit_zero_timeout_allows_immediately() {
        let cfg = AppProxyConfig {
            circuit_timeout_seconds: 0,
            ..config_for("codex")
        };
        let now = 9_000_000_i64;
        let row = CircuitRow {
            state: "open".to_string(),
            consecutive_failures: 3,
            consecutive_successes: 0,
            opened_at_ms: Some(now),
        };
        assert!(circuit_allows(&row, &cfg, now));
    }

    #[test]
    fn candidate_order_puts_active_first_and_dedupes() {
        let queue = vec![
            FailoverQueueRow {
                provider_id: "beta".to_string(),
                priority: 1,
                circuit_state: "closed".to_string(),
            },
            FailoverQueueRow {
                provider_id: "alpha".to_string(),
                priority: 2,
                circuit_state: "closed".to_string(),
            },
            FailoverQueueRow {
                provider_id: "beta".to_string(),
                priority: 3,
                circuit_state: "closed".to_string(),
            },
        ];

        assert_eq!(
            candidate_order("alpha", &queue),
            vec!["alpha".to_string(), "beta".to_string()],
            "主动供应商排在最前，队列顺序保留，重复项只出现一次"
        );
        assert_eq!(
            candidate_order("gamma", &queue),
            vec!["gamma".to_string(), "beta".to_string(), "alpha".to_string()]
        );
        assert_eq!(
            candidate_order("", &queue),
            vec!["beta".to_string(), "alpha".to_string()],
            "没有主动供应商时只用队列"
        );
        assert_eq!(candidate_order("alpha", &[]), vec!["alpha".to_string()]);
        assert!(candidate_order("", &[]).is_empty());
    }

    fn enabled_config(app_id: &str) -> RoutingConfig {
        let mut config = RoutingConfig::default();
        config.enabled = true;
        for app in &mut config.apps {
            if app.app_id == app_id {
                *app = config_for(app_id);
            }
        }
        config
    }

    fn open_store(path: &Path) -> RoutingStore {
        RoutingStore::open(path.to_path_buf()).expect("打开路由存储")
    }

    #[test]
    fn plan_from_store_disabled_returns_single_active_provider() {
        let path = temp_db("plan-disabled");
        let mut store = open_store(&path);
        for provider in ["alpha", "beta", "gamma"] {
            store.add_to_queue("codex", provider).expect("加入队列");
        }

        // 路由总开关关闭。
        let plan = plan_from_store(&store, "codex", "alpha", 0);
        assert!(!plan.enabled);
        assert_eq!(plan.candidates, vec!["alpha".to_string()]);

        // 路由打开但该应用的自动故障转移关闭。
        let config = enabled_config("codex");
        let mut config = config;
        for app in &mut config.apps {
            if app.app_id == "codex" {
                app.auto_failover_enabled = false;
            }
        }
        store.save(config).expect("保存路由配置");
        let plan = plan_from_store(&store, "codex", "alpha", 0);
        assert!(!plan.enabled);
        assert_eq!(plan.candidates, vec!["alpha".to_string()]);

        // 未知应用读不到行时使用默认配置，同样是关闭状态。
        let plan = plan_from_store(&store, "claude", "alpha", 0);
        assert!(!plan.enabled);
        assert_eq!(plan.candidates, vec!["alpha".to_string()]);

        drop(store);
        cleanup(&path);
    }

    #[test]
    fn plan_from_store_enabled_uses_queue_order_and_skips_open_circuits() {
        let path = temp_db("plan-enabled");
        let mut store = open_store(&path);
        store.save(enabled_config("codex")).expect("保存路由配置");
        for provider in ["alpha", "beta", "gamma", "delta"] {
            store.add_to_queue("codex", provider).expect("加入队列");
        }

        // 全部闭合：主动供应商在前，其后是队列顺序。
        let plan = plan_from_store(&store, "codex", "alpha", 0);
        assert!(plan.enabled);
        assert_eq!(plan.max_retries, 3);
        assert_eq!(
            plan.candidates,
            vec![
                "alpha".to_string(),
                "beta".to_string(),
                "gamma".to_string(),
                "delta".to_string()
            ]
        );

        // 主动供应商和一个队列候选熔断后应被跳过。
        let open_row = CircuitRow {
            state: "open".to_string(),
            consecutive_failures: 3,
            consecutive_successes: 0,
            opened_at_ms: Some(1),
        };
        store
            .put_circuit("codex", "alpha", &open_row)
            .expect("写入熔断状态");
        store
            .put_circuit("codex", "beta", &open_row)
            .expect("写入熔断状态");
        let plan = plan_from_store(&store, "codex", "alpha", 2_000);
        assert_eq!(
            plan.candidates,
            vec!["gamma".to_string(), "delta".to_string()]
        );

        // 超时之后熔断放行，主动供应商回到队首。
        let plan = plan_from_store(&store, "codex", "alpha", 60_001);
        assert_eq!(
            plan.candidates,
            vec![
                "alpha".to_string(),
                "beta".to_string(),
                "gamma".to_string(),
                "delta".to_string()
            ]
        );

        // 所有熔断都在冷却期：仍然回退到主动供应商。
        for provider in ["gamma", "delta"] {
            store
                .put_circuit("codex", provider, &open_row)
                .expect("写入熔断状态");
        }
        let plan = plan_from_store(&store, "codex", "alpha", 2_000);
        assert!(plan.enabled);
        assert_eq!(plan.candidates, vec!["alpha".to_string()]);

        drop(store);
        cleanup(&path);
    }

    #[test]
    fn plan_from_store_truncates_to_max_retries_plus_one() {
        let path = temp_db("plan-truncate");
        let mut store = open_store(&path);
        let mut config = enabled_config("codex");
        for app in &mut config.apps {
            if app.app_id == "codex" {
                app.max_retries = 1;
            }
        }
        store.save(config).expect("保存路由配置");
        for provider in ["alpha", "beta", "gamma", "delta"] {
            store.add_to_queue("codex", provider).expect("加入队列");
        }

        let plan = plan_from_store(&store, "codex", "alpha", 0);
        assert_eq!(plan.max_retries, 1);
        assert_eq!(
            plan.candidates,
            vec!["alpha".to_string(), "beta".to_string()],
            "候选数量不得超过 max_retries + 1"
        );

        drop(store);
        cleanup(&path);
    }

    #[test]
    fn plan_from_store_with_empty_queue_keeps_active_provider() {
        let path = temp_db("plan-empty-queue");
        let mut store = open_store(&path);
        store.save(enabled_config("codex")).expect("保存路由配置");

        let plan = plan_from_store(&store, "codex", "alpha", 0);
        assert!(plan.enabled);
        assert_eq!(plan.candidates, vec!["alpha".to_string()]);

        drop(store);
        cleanup(&path);
    }
}
