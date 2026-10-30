//! CCP-Bot integration.
//!
//! CCP-Bot (formerly OpenBot) is a separate web application stack: a Vite/React
//! `app` on :3010 and a Hono `server` on :3001, run either from source with
//! `bun run dev` or as a Docker Compose deployment. CCP does not bundle it: this
//! module only *controls and observes* a stack the user already has on disk, so
//! that the manager can show its UI in an embedded frame and turn it on or off
//! without a second window.
//!
//! Everything here is deliberately additive and read-mostly: the config lives in
//! its own `<app state dir>/ccpbot.json` (not in `app-preferences.json`) so an
//! unrelated preferences migration can never drop the user's CCP-Bot address,
//! and the process control writes a pid file rather than keeping the child
//! handle in memory, because the manager is restarted far more often than the
//! dev stack is.

use std::env;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// File holding the persisted CCP-Bot settings, next to `app-preferences.json`.
pub const CCPBOT_CONFIG_FILE: &str = "ccpbot.json";
/// Stdout/stderr of a source-mode `bun run dev`, appended across restarts.
pub const CCPBOT_DEV_LOG_FILE: &str = "ccpbot-dev.log";
/// Stdout/stderr of `docker compose up -d` / `down`.
pub const CCPBOT_COMPOSE_LOG_FILE: &str = "ccpbot-compose.log";
/// Pid of the source-mode dev process tree root.
pub const CCPBOT_DEV_PID_FILE: &str = "ccpbot-dev.pid";

pub const DEFAULT_ADDRESS: &str = "http://127.0.0.1:3010";
/// Empty on purpose. A checkout path from the machine this was developed on is
/// wrong for every other CCP user, so the page asks for the directory once and
/// stores it under that user own state dir; empty reads as "not chosen yet".
pub const DEFAULT_PROJECT_DIR: &str = "";
/// The published CCP-Bot repository. It is the localized build (zh-CN interface,
/// CCP-Bot naming), so "download CCP-Bot" and "download what this page embeds" are
/// the same artifact instead of two products that drift apart.
///
/// Releases live on the tag matching `package.json` (`v0.2.0` today), and that
/// release carries `container-images.json`: one digest per image, which is what
/// the desktop installer and the chart both pin.
pub const DEFAULT_REPO_URL: &str = "https://github.com/DamonZS/CCP-Bot.git";
/// Log of the download / install sequence, kept apart from the dev-server log so a
/// failed clone does not bury the reason under a later startup trace.
pub const CCPBOT_PREPARE_LOG_FILE: &str = "ccpbot-prepare.log";
/// The Hono server port used for the "is the stack up?" port probe.
pub const DEFAULT_SERVER_PORT: u16 = 3001;
/// Total budget for one health probe. The page must never look hung, so this is
/// shorter than the shared `http_client` timeout by design.
const PROBE_TIMEOUT: Duration = Duration::from_millis(3200);
// Windows can spend ~2s before it actively refuses a closed loopback port (measured
// on this machine: 2067 ms), so a 2s connect budget misreports "refused" as
// "timed out". 2.5s keeps that refusal inside the 3.2s total budget.
const PROBE_CONNECT_TIMEOUT: Duration = Duration::from_millis(2500);
const PORT_PROBE_TIMEOUT: Duration = Duration::from_millis(500);
const CCPBOT_USER_AGENT: &str = "CCP-Manager-CCPBot-Probe";

/// How the stack is expected to run on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CcpBotMode {
    /// `bun run dev` inside the source checkout (app + server on the host).
    Source,
    /// `docker compose up -d` inside the checkout.
    Docker,
}

impl Default for CcpBotMode {
    fn default() -> Self {
        Self::Source
    }
}

impl CcpBotMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Docker => "docker",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "source" => Some(Self::Source),
            "docker" => Some(Self::Docker),
            _ => None,
        }
    }
}

/// Persisted settings for the embedded CCP-Bot workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CcpBotConfig {
    pub address: String,
    pub mode: CcpBotMode,
    pub project_dir: String,
}

impl Default for CcpBotConfig {
    fn default() -> Self {
        Self {
            address: DEFAULT_ADDRESS.to_string(),
            mode: CcpBotMode::Source,
            project_dir: DEFAULT_PROJECT_DIR.to_string(),
        }
    }
}

/// Trim, drop the trailing slash and require an explicit http(s) scheme.
///
/// The scheme check is not pedantry: an iframe `src` without one is resolved as
/// a path relative to the manager's own origin, which silently loads `dist/` and
/// looks like "CCP-Bot is broken" instead of "the address is wrong".
pub fn normalize_address(raw: &str) -> Result<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        bail!("地址不能为空，例如 http://127.0.0.1:3010。");
    }
    let rest = if let Some(rest) = trimmed.strip_prefix("https://") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        rest
    } else {
        bail!("地址必须以 http:// 或 https:// 开头，例如 http://127.0.0.1:3010。");
    };
    if rest.is_empty() || rest.starts_with('/') {
        bail!("地址缺少主机名，例如 http://127.0.0.1:3010。");
    }
    if rest.contains(char::is_whitespace) {
        bail!("地址中间不能有空格。");
    }
    Ok(trimmed.to_string())
}

impl CcpBotConfig {
    /// Repair what `load` can repair instead of refusing to start: an address
    /// the user broke by hand falls back to the default, an unknown mode to
    /// source, so the page always renders something actionable.
    pub fn sanitized(mut self) -> Self {
        self.address =
            normalize_address(&self.address).unwrap_or_else(|_| DEFAULT_ADDRESS.to_string());
        self.project_dir = self.project_dir.trim().to_string();

        self
    }
}

/// Config file access. Mirrors `PreferencesStore`: a corrupt file is preserved
/// as `.corrupt-<millis>.bak` rather than silently rewritten over the user's
/// (possibly hand-edited) file.
pub struct CcpBotStore {
    path: PathBuf,
}

impl CcpBotStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> PathBuf {
        crate::paths::default_app_state_dir().join(CCPBOT_CONFIG_FILE)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> CcpBotConfig {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return CcpBotConfig::default();
        };
        match serde_json::from_str::<CcpBotConfig>(&text) {
            Ok(config) => config.sanitized(),
            Err(_) => {
                self.preserve_corrupt();
                CcpBotConfig::default()
            }
        }
    }

    pub fn save(&self, config: CcpBotConfig) -> Result<CcpBotConfig> {
        let config = config.sanitized();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("创建配置目录失败：{}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(&config).context("序列化 CCP-Bot 配置失败")?;
        fs::write(&self.path, text)
            .with_context(|| format!("写入配置失败：{}", self.path.display()))?;
        Ok(config)
    }

    fn preserve_corrupt(&self) {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default();
        let backup = self
            .path
            .with_extension(format!("json.corrupt-{millis}.bak"));
        let _ = fs::rename(&self.path, backup);
    }
}

/// Result of one `GET {address}/health` attempt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcpBotProbe {
    pub address: String,
    pub reachable: bool,
    pub status: Option<u16>,
    pub detail: String,
    pub elapsed_ms: u64,
}

/// What this machine can currently do, without spawning anything.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcpBotRuntime {
    pub bun_version: Option<String>,
    pub docker_cli: Option<String>,
    pub docker_daemon: bool,
    pub project_dir_present: bool,
    pub env_ready: bool,
    pub app_port_open: bool,
    pub server_port_open: bool,
}

/// Outcome of start/stop. `changed` is false when the request was a no-op
/// (already running, or nothing to stop) — the UI shows the message verbatim.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcpBotAction {
    pub changed: bool,
    pub pid: Option<u32>,
    pub message: String,
    pub log_path: String,
}

fn log_path(file: &str) -> PathBuf {
    crate::paths::default_app_state_dir().join(file)
}

fn pid_path() -> PathBuf {
    crate::paths::default_app_state_dir().join(CCPBOT_DEV_PID_FILE)
}

/// `host:port` of the configured address, so the port probe follows the address
/// instead of assuming :3010.
fn host_port(address: &str) -> Option<(String, u16)> {
    let (scheme, rest) = address
        .strip_prefix("https://")
        .map(|rest| ("https", rest))
        .or_else(|| address.strip_prefix("http://").map(|rest| ("http", rest)))?;
    let rest = rest.split('/').next().unwrap_or(rest);
    let default_port = if scheme == "https" { 443 } else { 80 };
    if let Some(idx) = rest.rfind(':') {
        let (host, port) = rest.split_at(idx);
        if let Ok(port) = port.trim_start_matches(':').parse::<u16>() {
            return Some((host.to_string(), port));
        }
    }
    if rest.is_empty() {
        None
    } else {
        Some((rest.to_string(), default_port))
    }
}

fn tcp_open(host: &str, port: u16) -> bool {
    let Ok(addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    let addrs: Vec<SocketAddr> = addrs.collect();
    addrs
        .into_iter()
        .any(|addr| TcpStream::connect_timeout(&addr, PORT_PROBE_TIMEOUT).is_ok())
}

fn first_line(output: &std::process::Output) -> Option<String> {
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().find(|line| !line.trim().is_empty())?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

/// Locates the bun executable this machine actually has.
///
/// `Command::new("bun")` is not enough on Windows: an npm-installed bun leaves
/// `bun.cmd` and `bun.ps1` shims on PATH plus the real `bun.exe` inside
/// `node_modules/bun/bin`, and CreateProcess only ever appends `.exe` to a bare
/// name. So a shell finds bun while a spawned child does not. We look in the
/// places an installation writes to, then fall back to a PATH scan for `.exe`.
fn resolve_bun() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(install) = env::var_os("BUN_INSTALL") {
        candidates.push(PathBuf::from(install).join("bin").join("bun.exe"));
    }
    if let Some(home) = env::var_os("USERPROFILE") {
        candidates.push(
            PathBuf::from(&home)
                .join(".bun")
                .join("bin")
                .join("bun.exe"),
        );
    }
    if let Some(appdata) = env::var_os("APPDATA") {
        candidates.push(
            PathBuf::from(&appdata)
                .join("npm")
                .join("node_modules")
                .join("bun")
                .join("bin")
                .join("bun.exe"),
        );
    }
    if let Some(path) = env::var_os("PATH") {
        candidates.extend(env::split_paths(&path).map(|dir| dir.join("bun.exe")));
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}
fn command_version(program: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(crate::windows_create_no_window());
    first_line(&command.output().ok()?)
}

fn docker_daemon_running() -> bool {
    let mut command = Command::new("docker");
    command
        .arg("info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(crate::windows_create_no_window());
    matches!(command.status(), Ok(status) if status.success())
}

/// Probe the machine once. Every field is best-effort: a missing tool must read
/// as `None`/`false`, never as an error, or the page would have nothing to show
/// on a machine that has not installed bun yet.
pub fn runtime(config: &CcpBotConfig) -> CcpBotRuntime {
    let project = Path::new(&config.project_dir);
    let app = host_port(&config.address);
    CcpBotRuntime {
        bun_version: match resolve_bun() {
            Some(bun) => command_version(&bun.display().to_string(), &["--version"]),
            None => None,
        },
        docker_cli: command_version("docker", &["--version"]),
        docker_daemon: docker_daemon_running(),
        project_dir_present: project.join("package.json").exists()
            && project.join("docker-compose.yml").exists(),
        env_ready: project.join(".env").exists(),
        app_port_open: app
            .as_ref()
            .map(|(host, port)| tcp_open(host, *port))
            .unwrap_or(false),
        server_port_open: app
            .as_ref()
            .map(|(host, _)| tcp_open(host, DEFAULT_SERVER_PORT))
            .unwrap_or(false),
    }
}

fn classify_probe_error(err: &reqwest::Error) -> String {
    if err.is_timeout() {
        return "探测超时：3 秒内没有响应，服务可能正在启动。".to_string();
    }
    if err.is_connect() {
        let text = err.to_string();
        if text.contains("dns")
            || text.contains("failed to lookup")
            || text.contains("Name or service not known")
        {
            return format!("域名解析失败：{text}");
        }
        return "连接被拒：该地址没有服务在监听，CCP-Bot 估计还没启动。".to_string();
    }
    format!("探测失败：{err}")
}

/// `GET {address}/health` with a hard 3s budget.
///
/// The client is built here rather than taken from `http_client` for two
/// reasons: the shared client's connect timeout (15s) is far longer than a
/// health probe may take, and `no_proxy()` is exactly the loopback bypass
/// `http_client::proxied_client_for(_, true)` performs — a local HTTP proxy
/// would otherwise intercept the probe and answer for the dead service.
pub async fn probe(address: &str) -> CcpBotProbe {
    let started = Instant::now();
    let address_owned = address.to_string();
    let client = match reqwest::Client::builder()
        .user_agent(CCPBOT_USER_AGENT)
        .no_proxy()
        .connect_timeout(PROBE_CONNECT_TIMEOUT)
        .timeout(PROBE_TIMEOUT)
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return CcpBotProbe {
                address: address_owned,
                reachable: false,
                status: None,
                detail: format!("HTTP 客户端创建失败：{err}"),
                elapsed_ms: started.elapsed().as_millis() as u64,
            };
        }
    };
    let url = format!("{}/health", address.trim_end_matches('/'));
    let response = client.get(&url).send().await;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    match response {
        Ok(response) => {
            let status = response.status();
            let code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            let reachable = status.is_success() && body.contains("\"ok\"");
            let detail = if reachable {
                "CCP-Bot 可达。".to_string()
            } else if status.is_success() {
                format!("HTTP {code} 但响应不是 CCP-Bot 的健康检查（缺少 ok 字段）。")
            } else {
                format!("HTTP {code}：地址有服务在监听，但不是 CCP-Bot 的健康端点。")
            };
            CcpBotProbe {
                address: address_owned,
                reachable,
                status: Some(code),
                detail,
                elapsed_ms,
            }
        }
        Err(err) => CcpBotProbe {
            address: address_owned,
            reachable: false,
            status: None,
            detail: classify_probe_error(&err),
            elapsed_ms,
        },
    }
}

fn read_pid(path: &Path) -> Option<u32> {
    let text = fs::read_to_string(path).ok()?;
    text.trim().parse::<u32>().ok()
}

/// Is the pid still alive? `tasklist` is used on Windows because it is always
/// present and needs no extra crate feature, and because a pid we wrote can be
/// reused by an unrelated process after a reboot — the pid file is deleted on
/// stop, so a live pid there is the stack we started.
fn process_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let mut command = Command::new("tasklist");
        command
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        command.creation_flags(crate::windows_create_no_window());
        let Ok(output) = command.output() else {
            return false;
        };
        let text = String::from_utf8_lossy(&output.stdout);
        text.contains(&pid.to_string())
    }
    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

fn kill_tree(pid: u32) -> Result<()> {
    #[cfg(windows)]
    {
        let mut command = Command::new("taskkill");
        // `/T` is the whole point: `bun run dev` exits immediately while its
        // vite/hono children keep the ports, so killing the root alone leaks
        // two listeners and the next start reports "port already in use".
        command
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null());
        command.creation_flags(crate::windows_create_no_window());
        let output = command.output().context("结束 CCP-Bot 进程失败")?;
        if !output.status.success() {
            let text = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if !text.is_empty() && text.contains("not found") {
                return Ok(());
            }
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-TERM", &format!("-{pid}")])
            .status()
            .context("结束 CCP-Bot 进程失败")?;
        Ok(())
    }
}

fn run_compose(config: &CcpBotConfig, args: &[&str], log_file: &str) -> Result<CcpBotAction> {
    let log = log_path(log_file);
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent).ok();
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .with_context(|| format!("打开日志失败：{}", log.display()))?;
    let mut command = Command::new("docker");
    command
        .arg("compose")
        .args(args)
        .current_dir(&config.project_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file.try_clone()?))
        .stderr(Stdio::from(file));
    #[cfg(windows)]
    command.creation_flags(crate::windows_create_no_window());
    let status = command
        .status()
        .context("执行 docker compose 失败，请确认 Docker Desktop 已启动。")?;
    let log_path_text = log.display().to_string();
    if !status.success() {
        bail!(
            "docker compose {} 失败（退出码 {:?}），详见 {}。",
            args.join(" "),
            status.code(),
            log_path_text
        );
    }
    Ok(CcpBotAction {
        changed: true,
        pid: None,
        message: format!("docker compose {} 完成。", args.join(" ")),
        log_path: log_path_text,
    })
}

/// Start the stack, or report that it is already up.
pub fn start(config: &CcpBotConfig) -> Result<CcpBotAction> {
    start_at(config, &pid_path())
}

/// Same as [`start`], with the pid file injected so tests never touch the real
/// one: the unit tests and the running manager share a state dir, and a test
/// that wrote `<state>/ccpbot-dev.pid` would make the next real start think the
/// stack was already up.
fn start_at(config: &CcpBotConfig, pid_file: &Path) -> Result<CcpBotAction> {
    let config = config.clone().sanitized();
    let project = Path::new(&config.project_dir);
    let dev_log = log_path(CCPBOT_DEV_LOG_FILE);
    let dev_log_text = dev_log.display().to_string();
    if config.project_dir.is_empty() {
        bail!("还没有选择 CCP-Bot 目录：先点「选择目录」，或只填地址去看已在运行的那个实例。");
    }
    if !project.join("package.json").exists() {
        bail!(
            "找不到 CCP-Bot 源码目录：{} 下没有 package.json。请先在页面里改成正确的目录。",
            config.project_dir
        );
    }
    if let Some(pid) = read_pid(pid_file) {
        if process_alive(pid) {
            return Ok(CcpBotAction {
                changed: false,
                pid: Some(pid),
                message: "CCP-Bot 已在运行（源码模式）。".to_string(),
                log_path: dev_log_text,
            });
        }
        let _ = fs::remove_file(pid_file);
    }
    match config.mode {
        CcpBotMode::Source => {
            if !project.join(".env").exists() {
                bail!(
                    "源码模式需要 {}/.env：请先把 .env.example 复制成 .env，再点启动。",
                    config.project_dir
                );
            }
            let Some(bun) = resolve_bun() else {
                bail!(
                    "未找到 bun 可执行文件，无法以源码模式启动 CCP-Bot。请先安装 bun（https://bun.sh）。"
                );
            };
            if let Some(parent) = dev_log.parent() {
                fs::create_dir_all(parent).ok();
            }
            let file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&dev_log)
                .with_context(|| format!("打开日志失败：{}", dev_log.display()))?;
            let mut command = Command::new(&bun);
            command
                .args(["run", "dev"])
                .current_dir(project)
                .stdin(Stdio::null())
                .stdout(Stdio::from(file.try_clone()?))
                .stderr(Stdio::from(file));
            #[cfg(windows)]
            command.creation_flags(crate::windows_create_no_window());
            let child = command
                .spawn()
                .context("启动 bun run dev 失败，请确认 bun 可用。")?;
            let pid = child.id();
            fs::write(pid_file, pid.to_string())
                .with_context(|| format!("写入 pid 文件失败：{}", pid_file.display()))?;
            Ok(CcpBotAction {
                changed: true,
                pid: Some(pid),
                message: format!(
                    "已在 {} 启动 bun run dev（pid {pid}），首次启动要等 Vite 就绪。",
                    config.project_dir
                ),
                log_path: dev_log_text,
            })
        }
        CcpBotMode::Docker => {
            if command_version("docker", &["--version"]).is_none() {
                bail!("未找到 docker CLI，无法以 Docker 模式启动 CCP-Bot。");
            }
            if !docker_daemon_running() {
                bail!("Docker daemon 没有在运行：请先启动 Docker Desktop 再试。");
            }
            run_compose(&config, &["up", "-d"], CCPBOT_COMPOSE_LOG_FILE)
        }
    }
}

/// Stop whatever `start` started.
pub fn stop(config: &CcpBotConfig) -> Result<CcpBotAction> {
    stop_at(config, &pid_path())
}

fn stop_at(config: &CcpBotConfig, pid_file: &Path) -> Result<CcpBotAction> {
    let config = config.clone().sanitized();
    match config.mode {
        CcpBotMode::Source => {
            let path = pid_file.to_path_buf();
            let Some(pid) = read_pid(&path) else {
                return Ok(CcpBotAction {
                    changed: false,
                    pid: None,
                    message: "CCP-Bot 当前没有由 CCP 启动的进程。".to_string(),
                    log_path: log_path(CCPBOT_DEV_LOG_FILE).display().to_string(),
                });
            };
            if !process_alive(pid) {
                let _ = fs::remove_file(&path);
                return Ok(CcpBotAction {
                    changed: false,
                    pid: None,
                    message: "CCP-Bot 进程已退出，清理了残留的 pid 文件。".to_string(),
                    log_path: log_path(CCPBOT_DEV_LOG_FILE).display().to_string(),
                });
            }
            kill_tree(pid)?;
            let _ = fs::remove_file(&path);
            Ok(CcpBotAction {
                changed: true,
                pid: Some(pid),
                message: format!("已结束 CCP-Bot 进程树（pid {pid}）。"),
                log_path: log_path(CCPBOT_DEV_LOG_FILE).display().to_string(),
            })
        }
        CcpBotMode::Docker => {
            if command_version("docker", &["--version"]).is_none() {
                bail!("未找到 docker CLI，无法停止 Docker 模式的 CCP-Bot。");
            }
            run_compose(&config, &["down"], CCPBOT_COMPOSE_LOG_FILE)
        }
    }
}

/// Pid of the stack CCP started, if that process is still alive.
/// Stage 2: give a CCP user who has no CCP-Bot at all a working checkout without
/// sending them to a build guide.
///
/// The clone target is [DEFAULT_REPO_URL], the localized build, so what lands on
/// disk is exactly the application this module embeds. Every step is named in the
/// returned message: on a cold machine this runs for minutes, and a silent spinner
/// gives the user no way to tell "still downloading" from "stuck".
pub fn prepare(config: &CcpBotConfig) -> Result<CcpBotAction> {
    let config = config.clone().sanitized();
    if config.project_dir.trim().is_empty() {
        bail!(
            "还没有选择 CCP-Bot 目录：先点「选择目录」挑一个空文件夹（或已有的 CCP-Bot 目录），再点「下载并准备」。"
        );
    }
    let project = PathBuf::from(config.project_dir.trim());
    let log = log_path(CCPBOT_PREPARE_LOG_FILE);
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut steps: Vec<String> = Vec::new();

    if project.join("package.json").is_file() {
        steps.push("目录里已有 CCP-Bot，跳过下载".to_string());
    } else {
        // git clone only accepts an empty directory, so check before touching it:
        // silently downloading into a folder full of somebody's files is worse than
        // refusing and asking again.
        if project.exists() {
            let mut entries = fs::read_dir(&project)
                .with_context(|| format!("读取目录失败：{}", project.display()))?;
            if entries.next().is_some() {
                bail!(
                    "{} 不是空目录，里面也没有 package.json —— 请挑一个空文件夹，或直接选已有的 CCP-Bot 目录。",
                    project.display()
                );
            }
        }
        fs::create_dir_all(&project)
            .with_context(|| format!("创建目录失败：{}", project.display()))?;
        run_logged(
            Path::new("git"),
            &["clone", "--depth", "1", DEFAULT_REPO_URL, "."],
            &project,
            CCPBOT_PREPARE_LOG_FILE,
        )
        .with_context(|| {
            format!("下载 CCP-Bot 失败：{DEFAULT_REPO_URL} 需要本机安装 git 并能访问 GitHub。")
        })?;
        steps.push(format!("从 {DEFAULT_REPO_URL} 下载完成"));
    }

    if project.join("node_modules").exists() {
        steps.push("依赖已存在，跳过安装".to_string());
    } else {
        let bun = resolve_bun()
            .context("未找到 bun 可执行文件，无法安装依赖。请先安装 bun（https://bun.sh）。")?;
        run_logged(&bun, &["install"], &project, CCPBOT_PREPARE_LOG_FILE)
            .context("安装依赖失败")?;
        steps.push("依赖安装完成".to_string());
    }

    let env_file = project.join(".env");
    if env_file.exists() {
        steps.push(".env 已存在，未覆盖".to_string());
    } else {
        let template = project.join(".env.example");
        if template.is_file() {
            fs::copy(&template, &env_file)
                .with_context(|| format!("复制 .env 失败：{}", template.display()))?;
            steps.push("已从 .env.example 生成 .env（密钥还需自己填）".to_string());
        } else {
            steps.push("没有 .env.example，跳过 .env".to_string());
        }
    }

    Ok(CcpBotAction {
        changed: true,
        pid: None,
        message: format!("{}。日志：{}", steps.join("；"), log.display()),
        log_path: log.display().to_string(),
    })
}

/// Run one preparation step with stdout/stderr appended to [CCPBOT_PREPARE_LOG_FILE].
fn run_logged(program: &Path, args: &[&str], dir: &Path, log_file: &str) -> Result<()> {
    use std::io::Write as _;

    let log = log_path(log_file);
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .with_context(|| format!("打开日志失败：{}", log.display()))?;
    writeln!(file, "\n$ {} {}", program.display(), args.join(" ")).ok();
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file.try_clone()?))
        .stderr(Stdio::from(file));
    #[cfg(windows)]
    command.creation_flags(crate::windows_create_no_window());
    let status = command
        .status()
        .with_context(|| format!("执行 {} 失败", program.display()))?;
    if !status.success() {
        bail!(
            "{} {} 失败（退出码 {:?}），详见 {}。",
            program.display(),
            args.join(" "),
            status.code(),
            log.display()
        );
    }
    Ok(())
}

/// Where a one-click install looks for the version to install.
///
/// The release itself, not its tags: only a published release carries the installers a person can
/// run, and only a published release is what somebody without a token can download. `latest` is
/// GitHub's own pointer to the newest non-draft release, so the button installs the newest version
/// without CCP holding a version number that goes stale.
pub const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/DamonZS/CCP-Bot/releases/latest";

/// Log for a one-click install, kept apart from the preparation log so a failed download and a
/// failed `bun install` are not interleaved in one file.
pub const CCPBOT_INSTALL_LOG_FILE: &str = "ccpbot-install.log";

/// Where the downloaded installer is kept before it runs.
pub const CCPBOT_INSTALL_FOLDER: &str = "ccpbot-installers";

/// Which release assets a one-click install may take on the machine it runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Platform {
    Windows,
    MacOs,
    Unsupported,
}

fn host_platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Unsupported
    }
}

/// One file attached to a release, as the GitHub API spells it.
#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ReleaseInfo {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

/// The architecture tokens a bundle's filename carries, most specific first.
///
/// The bundler writes one file per architecture and nothing else distinguishes them — two DMGs with
/// the same version, and installing the one for the other machine does not run at all. So the pick
/// reads the architecture out of the filename rather than "first file that ends in .dmg".
fn arch_tokens() -> &'static [&'static str] {
    if cfg!(target_arch = "aarch64") {
        &["aarch64", "arm64"]
    } else {
        &["x64", "x86_64", "amd64"]
    }
}

/// The tokens that name the *other* architecture, so a foreign bundle can be recognised as foreign.
fn other_arch_tokens() -> &'static [&'static str] {
    if cfg!(target_arch = "aarch64") {
        &["x64", "x86_64", "amd64"]
    } else {
        &["aarch64", "arm64"]
    }
}

/// Pick the installer for `platform` out of a release's assets.
///
/// Windows prefers the NSIS `.exe` over the `.msi` for one reason: the `.exe` takes `/S` and
/// returns, while `msiexec` needs a log path and a restart policy to fail as quietly. Both are
/// published, so this is a preference and not a requirement. macOS has only the `.dmg`.
fn pick_installer(assets: &[ReleaseAsset], platform: Platform) -> Result<ReleaseAsset> {
    let lower = |name: &str| name.to_lowercase();
    let candidates: Vec<&ReleaseAsset> = match platform {
        Platform::Windows => {
            let setup: Vec<&ReleaseAsset> = assets
                .iter()
                .filter(|asset| lower(&asset.name).ends_with("-setup.exe"))
                .collect();
            if setup.is_empty() {
                assets
                    .iter()
                    .filter(|asset| lower(&asset.name).ends_with(".msi"))
                    .collect()
            } else {
                setup
            }
        }
        Platform::MacOs => assets
            .iter()
            .filter(|asset| lower(&asset.name).ends_with(".dmg"))
            .collect(),
        Platform::Unsupported => Vec::new(),
    };
    if candidates.is_empty() {
        bail!(
            "这个版本没有适用于当前系统的安装包（Windows 要 .exe/.msi，macOS 要 .dmg）。可以在 CCP-Bot 目录里用「下载并准备」从源码启动。"
        );
    }
    let tokens = arch_tokens();
    if let Some(found) = candidates.iter().find(|asset| {
        let name = lower(&asset.name);
        tokens.iter().any(|token| name.contains(token))
    }) {
        return Ok((*found).clone());
    }
    // Nothing matched this machine's architecture. Before falling back to "first candidate", check
    // whether the release publishes *only* the other architecture: an Apple-silicon `.dmg` on an
    // Intel Mac is not a preference, it is a bundle that refuses to launch, so saying so beats
    // installing it. A name with no architecture in it at all (a universal build) still passes.
    let foreign: Vec<String> = candidates
        .iter()
        .map(|asset| lower(&asset.name))
        .filter(|name| other_arch_tokens().iter().any(|token| name.contains(token)))
        .collect();
    if foreign.len() == candidates.len() {
        let wanted = if cfg!(target_arch = "aarch64") {
            "Apple 芯片（aarch64）"
        } else {
            "Intel（x64）"
        };
        bail!(
            "这个版本只发布了给另一种架构的安装包（{}），而本机是 {wanted}：装了也起不来。可以在 CCP-Bot 目录里用「下载并准备」从源码启动。",
            foreign.join("、")
        );
    }
    Ok(candidates[0].clone())
}

fn installers_dir() -> PathBuf {
    log_path(CCPBOT_INSTALL_FOLDER)
}

fn write_install_log(line: &str) {
    use std::io::Write as _;

    let log = log_path(CCPBOT_INSTALL_LOG_FILE);
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
    {
        writeln!(file, "{line}").ok();
    }
}

/// Download the newest published installer and hand it to the system.
///
/// This is the one-click path: no git, no toolchain, no directory to choose. What it installs is
/// the desktop app, which is also what brings the stack up on the user's machine — so CCP's job
/// stops at "get the newest build onto this computer and run its installer".
pub async fn install_latest() -> Result<CcpBotAction> {
    let (tag, asset, target) = download_latest_installer().await?;
    let log = log_path(CCPBOT_INSTALL_LOG_FILE);
    let outcome = run_installer(&target)?;
    Ok(CcpBotAction {
        changed: true,
        pid: None,
        message: format!(
            "已下载 CCP-Bot {} 的 {}。{outcome}。文件：{}；日志：{}",
            tag,
            asset,
            target.display(),
            log.display()
        ),
        log_path: log.display().to_string(),
    })
}

/// Fetch the newest installer and return `(tag, asset name, path on disk)`.
///
/// Split out from [install_latest] so the network half — which release, which asset, how big — can
/// be proven without running an installer on the machine doing the proving.
pub async fn download_latest_installer() -> Result<(String, String, PathBuf)> {
    let platform = host_platform();
    if matches!(platform, Platform::Unsupported) {
        bail!(
            "一键安装只提供 Windows 与 macOS 的安装包；这套系统请用「下载并准备」从源码启动。"
        );
    }
    let client = reqwest::Client::builder()
        .user_agent(CCPBOT_USER_AGENT)
        .no_proxy()
        .connect_timeout(PROBE_CONNECT_TIMEOUT * 4)
        .timeout(Duration::from_secs(600))
        .build()
        .context("创建 HTTP 客户端失败")?;

    write_install_log(&format!("GET {LATEST_RELEASE_URL}"));
    let response = client
        .get(LATEST_RELEASE_URL)
        .send()
        .await
        .with_context(|| format!("读取最新版本信息失败：{LATEST_RELEASE_URL}"))?;
    let status = response.status();
    if !status.is_success() {
        bail!("读取最新版本信息失败：HTTP {status}（{LATEST_RELEASE_URL}）");
    }
    let body = response.bytes().await.context("读取最新版本信息失败")?;
    let release: ReleaseInfo =
        serde_json::from_slice(&body).context("解析最新版本信息失败：响应不是预期的 JSON")?;
    let asset = pick_installer(&release.assets, platform)?;

    let folder = installers_dir();
    fs::create_dir_all(&folder)
        .with_context(|| format!("创建下载目录失败：{}", folder.display()))?;
    let target = folder.join(&asset.name);
    write_install_log(&format!("GET {}", asset.browser_download_url));
    let response = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .with_context(|| format!("下载安装包失败：{}", asset.browser_download_url))?;
    let status = response.status();
    if !status.is_success() {
        bail!("下载安装包失败：HTTP {status}（{}）", asset.name);
    }
    let bytes = response.bytes().await.context("下载安装包失败")?;
    // A release asset can be a redirect or an error page; 100 KB is below every real bundle and
    // above every message GitHub would answer with.
    if bytes.len() < 100_000 {
        bail!(
            "下载到的 {} 只有 {} 字节，不像是完整的安装包。",
            asset.name,
            bytes.len()
        );
    }
    fs::write(&target, &bytes)
        .with_context(|| format!("写入安装包失败：{}", target.display()))?;
    write_install_log(&format!("saved {} bytes to {}", bytes.len(), target.display()));
    Ok((release.tag_name, asset.name, target))
}

/// Hand a downloaded installer to the system, and say what the person still has to do.
fn run_installer(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    #[cfg(windows)]
    {
        let mut command = if name.ends_with(".msi") {
            let mut command = Command::new("msiexec");
            command.args([
                "/i",
                &path.display().to_string(),
                "/qn",
                "/norestart",
            ]);
            command
        } else {
            // `/S` is the NSIS silent flag: present the installer's own progress UI would need a
            // window CCP cannot see, and the user asked for it by pressing the button.
            let mut command = Command::new(path);
            command.arg("/S");
            command
        };
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.creation_flags(crate::windows_create_no_window());
        let status = command
            .status()
            .with_context(|| format!("运行安装包失败：{}", path.display()))?;
        if !status.success() {
            bail!(
                "安装包返回退出码 {:?}。可以手动双击 {} 再试一次。",
                status.code(),
                path.display()
            );
        }
        return Ok("安装程序已静默完成".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(path)
            .status()
            .with_context(|| format!("打开安装包失败：{}", path.display()))?;
        if !status.success() {
            bail!("打开 .dmg 失败，可以手动双击 {}", path.display());
        }
        // A DMG cannot install itself: mounting it is the furthest a process may go without the
        // person dragging the app, so the message says exactly that instead of claiming success.
        return Ok("已挂载磁盘映像，把 CCP-Bot 拖进「应用程序」即完成");
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = name;
        bail!("这套系统没有可执行的安装包：请用「下载并准备」从源码启动。")
    }
}

pub fn install_log_file() -> &'static str {
    CCPBOT_INSTALL_LOG_FILE
}

pub fn running_pid() -> Option<u32> {
    let pid = read_pid(&pid_path())?;
    process_alive(pid).then_some(pid)
}

/// Last `lines` lines of a log file, plus its path.
pub fn log_tail(file: &str, lines: usize) -> (String, String) {
    let path = log_path(file);
    let path_text = path.display().to_string();
    let Ok(mut handle) = fs::File::open(&path) else {
        return (path_text, String::new());
    };
    let Ok(len) = handle.metadata().map(|meta| meta.len()) else {
        return (path_text, String::new());
    };
    // Read at most the last 256 KiB: the log grows for the life of the
    // checkout, and the page only shows a tail.
    let window = 256 * 1024u64;
    let start = len.saturating_sub(window);
    if handle.seek(SeekFrom::Start(start)).is_err() {
        return (path_text, String::new());
    }
    let mut buffer = Vec::new();
    if handle.read_to_end(&mut buffer).is_err() {
        return (path_text, String::new());
    }
    let text = String::from_utf8_lossy(&buffer);
    let collected: Vec<&str> = text.lines().collect();
    let tail = if collected.len() > lines {
        collected[collected.len() - lines..].join("\n")
    } else {
        collected.join("\n")
    };
    (path_text, tail)
}

pub fn dev_log_file() -> &'static str {
    CCPBOT_DEV_LOG_FILE
}

pub fn prepare_log_file() -> &'static str {
    CCPBOT_PREPARE_LOG_FILE
}

pub fn compose_log_file() -> &'static str {
    CCPBOT_COMPOSE_LOG_FILE
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;

    /// The probe is async (it is awaited from a Tauri command), but its tests run
    /// in a plain `#[test]` so a failure reads as an assertion rather than as a
    /// runtime panic. One current-thread runtime per test is enough: the probe
    /// resolves real sockets and needs a reactor, not parallelism.
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build test runtime")
            .block_on(future)
    }

    fn temp_store(name: &str) -> (CcpBotStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "ccpbot-test-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join(CCPBOT_CONFIG_FILE);
        (CcpBotStore::new(path.clone()), dir)
    }

    #[test]
    fn normalize_address_trims_and_requires_a_scheme() {
        assert_eq!(
            normalize_address("  http://127.0.0.1:3010/  ").unwrap(),
            "http://127.0.0.1:3010"
        );
        assert_eq!(
            normalize_address("https://ccpbot.local/").unwrap(),
            "https://ccpbot.local"
        );
        assert!(normalize_address("127.0.0.1:3010").is_err());
        assert!(normalize_address("ftp://127.0.0.1").is_err());
        assert!(normalize_address("   ").is_err());
        assert!(normalize_address("http://").is_err());
    }

    #[test]
    fn store_round_trips_the_config() {
        let (store, dir) = temp_store("roundtrip");
        let saved = store
            .save(CcpBotConfig {
                address: "http://127.0.0.1:3999/".to_string(),
                mode: CcpBotMode::Docker,
                project_dir: "D:/Somewhere/CCP-Bot".to_string(),
            })
            .expect("save config");
        assert_eq!(saved.address, "http://127.0.0.1:3999");
        let loaded = store.load();
        assert_eq!(loaded.address, "http://127.0.0.1:3999");
        assert_eq!(loaded.mode, CcpBotMode::Docker);
        assert_eq!(loaded.project_dir, "D:/Somewhere/CCP-Bot");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_config_is_backed_up_and_defaults_return() {
        let (store, dir) = temp_store("corrupt");
        fs::write(store.path(), "not json at all").expect("write corrupt");
        let loaded = store.load();
        assert_eq!(loaded.address, DEFAULT_ADDRESS);
        let backups: Vec<PathBuf> = fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.to_string_lossy().contains(".corrupt-"))
            .collect();
        assert_eq!(backups.len(), 1, "expected exactly one corrupt backup");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn host_port_follows_the_address() {
        assert_eq!(
            host_port("http://127.0.0.1:3010"),
            Some(("127.0.0.1".to_string(), 3010))
        );
        assert_eq!(
            host_port("https://ccpbot.local/"),
            Some(("ccpbot.local".to_string(), 443))
        );
        assert_eq!(host_port("nonsense"), None);
    }

    #[test]
    fn each_failure_kind_has_its_own_detail() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        // Closed port: bind to learn a free one, then drop the listener.
        drop(listener);
        let closed = block_on(probe(&format!("http://127.0.0.1:{port}")));
        assert!(!closed.reachable);
        assert!(
            closed.detail.contains("连接被拒"),
            "unexpected detail: {}",
            closed.detail
        );

        // A live stub that answers 404 must be distinguished from "no service".
        let stub = TcpListener::bind("127.0.0.1:0").expect("bind stub");
        let stub_port = stub.local_addr().expect("stub addr").port();
        thread::spawn(move || {
            for stream in stub.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = [0u8; 512];
                let _ = stream.read(&mut request);
                let body = "not found";
                let response = format!(
                    "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        let wrong_endpoint = block_on(probe(&format!("http://127.0.0.1:{stub_port}")));
        assert!(!wrong_endpoint.reachable);
        assert_eq!(wrong_endpoint.status, Some(404));
        assert!(
            wrong_endpoint.detail.contains("404"),
            "unexpected detail: {}",
            wrong_endpoint.detail
        );
        assert_ne!(closed.detail, wrong_endpoint.detail);

        // A socket that accepts and then says nothing is the third kind: the
        // page must say "still starting", not "refused".
        let blackhole = TcpListener::bind("127.0.0.1:0").expect("bind blackhole");
        let blackhole_port = blackhole.local_addr().expect("blackhole addr").port();
        thread::spawn(move || {
            let mut held = Vec::new();
            for stream in blackhole.incoming() {
                let Ok(stream) = stream else { continue };
                held.push(stream);
            }
        });
        let silent = block_on(probe(&format!("http://127.0.0.1:{blackhole_port}")));
        assert!(!silent.reachable);
        assert!(
            silent.detail.contains("超时"),
            "unexpected detail: {}",
            silent.detail
        );
        assert_ne!(silent.detail, closed.detail);
        assert_ne!(silent.detail, wrong_endpoint.detail);
    }

    #[test]
    fn healthy_endpoint_is_reachable() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind healthy");
        let port = listener.local_addr().expect("healthy addr").port();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = [0u8; 512];
                let _ = stream.read(&mut request);
                let body = r#"{"status":"ok"}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        let probe = block_on(probe(&format!("http://127.0.0.1:{port}")));
        assert!(probe.reachable, "detail: {}", probe.detail);
        assert_eq!(probe.status, Some(200));
        assert!(
            probe.elapsed_ms < 3_500,
            "probe took {}ms",
            probe.elapsed_ms
        );
    }

    #[test]
    fn source_start_without_env_is_refused() {
        let dir = std::env::temp_dir().join(format!(
            "ccpbot-test-noenv-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&dir).expect("create dir");
        fs::write(dir.join("package.json"), "{}").expect("write package.json");
        let pid_file = dir.join(CCPBOT_DEV_PID_FILE);
        let error = start_at(
            &CcpBotConfig {
                address: DEFAULT_ADDRESS.to_string(),
                mode: CcpBotMode::Source,
                project_dir: dir.display().to_string(),
            },
            &pid_file,
        )
        .expect_err("start without .env must fail");
        assert!(
            error.to_string().contains(".env"),
            "unexpected error: {error}"
        );
        assert!(
            !pid_file.exists(),
            "a refused start must not leave a pid file"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn start_reports_already_running() {
        let dir = std::env::temp_dir().join(format!(
            "ccpbot-test-running-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&dir).expect("create dir");
        fs::write(dir.join("package.json"), "{}").expect("write package.json");
        let path = dir.join(CCPBOT_DEV_PID_FILE);
        // The test process itself stands in for a live dev process: the pid file
        // is the only state `start` consults, so this proves the early return
        // without spawning a real vite server.
        fs::write(&path, std::process::id().to_string()).expect("write pid");
        let action = start_at(
            &CcpBotConfig {
                address: DEFAULT_ADDRESS.to_string(),
                mode: CcpBotMode::Source,
                project_dir: dir.display().to_string(),
            },
            &path,
        )
        .expect("start should short-circuit");
        let _ = fs::remove_file(&path);
        assert!(!action.changed, "expected the no-op branch");
        assert_eq!(action.pid, Some(std::process::id()));
        assert!(action.message.contains("已在运行"), "{}", action.message);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn log_tail_returns_the_last_lines() {
        let path = log_path("ccpbot-test-tail.log");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let body: String = (1..=40).map(|n| format!("line {n}\n")).collect();
        fs::write(&path, body).expect("write log");
        let (_, text) = log_tail("ccpbot-test-tail.log", 5);
        assert_eq!(text.lines().count(), 5);
        assert!(text.ends_with("line 40"));
        let _ = fs::remove_file(&path);
    }
    /// Live check, deliberately outside the default run: it spawns a real
    /// `bun run dev` against a scratch project, so the pid file, the log capture
    /// and the port release it asserts are the ones the manager really sees. Run
    /// it with `cargo test -p claude-codex-pro-core ccpbot -- --ignored --nocapture`.
    #[test]
    #[ignore = "spawns bun; run explicitly to produce the acceptance evidence"]
    fn live_start_serves_health_and_stop_frees_the_port() {
        let dir = std::env::temp_dir().join(format!("ccpbot-live-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create scratch project");
        fs::write(
            dir.join("package.json"),
            r#"{"name":"ccpbot-scratch","private":true,"scripts":{"dev":"bun server.ts"}}"#,
        )
        .expect("write scratch package.json");
        fs::write(
            dir.join("server.ts"),
            r#"Bun.serve({ port: Number(process.env.PORT ?? 3199), hostname: "127.0.0.1", fetch: () => Response.json({ status: "ok" }) });"#,
        )
        .expect("write scratch server");
        fs::write(dir.join(".env"), "PORT=3199\n").expect("write scratch env");

        let config = CcpBotConfig {
            address: "http://127.0.0.1:3199".to_string(),
            mode: CcpBotMode::Source,
            project_dir: dir.display().to_string(),
        };
        let pid_file = dir.join("scratch.pid");

        let started = start_at(&config, &pid_file).expect("start the scratch stack");
        assert!(started.changed, "start should report a change");
        let pid = started.pid.expect("a pid once started");
        assert_eq!(read_pid(&pid_file), Some(pid));

        let again = start_at(&config, &pid_file).expect("the second start is a no-op");
        assert!(!again.changed, "a live stack must not be started twice");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut probe = block_on(super::probe(&config.address));
        while !probe.reachable && std::time::Instant::now() < deadline {
            thread::sleep(std::time::Duration::from_millis(300));
            probe = block_on(super::probe(&config.address));
        }
        assert!(probe.reachable, "health never answered: {}", probe.detail);
        assert_eq!(probe.status, Some(200), "health status");

        let (path, text) = log_tail(CCPBOT_DEV_LOG_FILE, 40);
        println!("dev log {path}:\n{text}");
        assert!(!text.is_empty(), "the dev log should hold the child output");

        let stopped = stop_at(&config, &pid_file).expect("stop the scratch stack");
        assert!(
            stopped.changed,
            "stop should report a change: {}",
            stopped.message
        );

        let stop_deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while tcp_open("127.0.0.1", 3199) && std::time::Instant::now() < stop_deadline {
            thread::sleep(std::time::Duration::from_millis(200));
        }
        assert!(
            !tcp_open("127.0.0.1", 3199),
            "the port is still listening after stop"
        );
        assert!(!process_alive(pid), "pid {pid} survived stop");

        fs::remove_dir_all(&dir).ok();
    }
    #[test]
    fn default_config_asks_for_a_directory_instead_of_assuming_one() {
        let config = CcpBotConfig::default();
        assert_eq!(config.address, DEFAULT_ADDRESS);
        assert!(
            config.project_dir.is_empty(),
            "a checkout path from the development machine must not ship as the default"
        );
        assert!(config.sanitized().project_dir.is_empty());
    }

    #[test]
    fn start_without_a_directory_explains_the_two_ways_in() {
        let (_, dir) = temp_store("no-dir");
        let pid_file = dir.join("scratch.pid");
        let config = CcpBotConfig {
            address: DEFAULT_ADDRESS.to_string(),
            mode: CcpBotMode::Source,
            project_dir: "  ".to_string(),
        };
        let error = start_at(&config, &pid_file).expect_err("no directory, no start");
        let message = error.to_string();
        assert!(message.contains("还没有选择 CCP-Bot 目录"), "{message}");
        assert!(message.contains("选择目录"), "{message}");
        assert!(
            !pid_file.exists(),
            "a refused start must not leave a pid file"
        );
    }

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccpbot-prepare-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    fn configured(project_dir: &str) -> CcpBotConfig {
        CcpBotConfig {
            address: DEFAULT_ADDRESS.to_string(),
            mode: CcpBotMode::Source,
            project_dir: project_dir.to_string(),
        }
    }

    #[test]
    fn prepare_without_a_directory_asks_for_one() {
        let error = prepare(&configured("  ")).expect_err("no directory, no download");
        let message = error.to_string();
        assert!(message.contains("还没有选择 CCP-Bot 目录"), "{message}");
        assert!(message.contains("下载并准备"), "{message}");
    }

    #[test]
    fn prepare_refuses_a_directory_that_holds_something_else() {
        let dir = scratch_dir("busy");
        fs::write(dir.join("notes.txt"), "not a checkout").expect("seed the directory");
        let error = prepare(&configured(&dir.display().to_string()))
            .expect_err("a foreign directory is not a checkout");
        let message = error.to_string();
        assert!(message.contains("不是空目录"), "{message}");
        assert!(
            !dir.join("package.json").exists(),
            "a refused prepare must not invent a checkout"
        );
    }

    /// Network plus un install; run explicitly to produce the acceptance evidence.
    #[ignore = "clones the published repository and installs its dependencies"]
    #[test]
    fn live_prepare_downloads_and_seeds_env() {
        let dir = scratch_dir("live");
        let action = prepare(&configured(&dir.display().to_string()))
            .expect("prepare the published checkout");
        assert!(dir.join("package.json").is_file(), "{}", action.message);
        assert!(dir.join("node_modules").exists(), "{}", action.message);
        assert!(action.message.contains("下载完成"), "{}", action.message);
    }

    /// Re-running the button must be cheap: pointing it at a checkout that is already
    /// there has to skip the clone instead of downloading the repository again.
    #[test]
    fn prepare_reuses_an_existing_checkout_without_touching_the_network() {
        let dir = scratch_dir("existing");
        fs::write(dir.join("package.json"), "{\"name\":\"ccpbot\"}").expect("seed package.json");
        fs::create_dir_all(dir.join("node_modules")).expect("seed node_modules");
        fs::write(dir.join(".env"), "PORT=3010\n").expect("seed .env");
        let action = prepare(&configured(&dir.display().to_string())).expect("reuse the checkout");
        assert!(action.message.contains("跳过下载"), "{}", action.message);
        assert!(action.message.contains("依赖已存在"), "{}", action.message);
        assert!(action.message.contains(".env 已存在"), "{}", action.message);
    }

    fn asset(name: &str) -> ReleaseAsset {
        ReleaseAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example.invalid/{name}"),
        }
    }

    #[test]
    fn a_windows_install_prefers_the_silent_setup_over_the_msi() {
        let assets = vec![
            asset("CCP-Bot_0.2.3_x64_en-US.msi"),
            asset("CCP-Bot_0.2.3_x64-setup.exe"),
        ];
        let picked = pick_installer(&assets, Platform::Windows).expect("pick a Windows installer");
        assert_eq!(picked.name, "CCP-Bot_0.2.3_x64-setup.exe");
    }

    #[test]
    fn a_windows_install_falls_back_to_the_msi() {
        let assets = vec![asset("CCP-Bot_0.2.3_x64_en-US.msi")];
        let picked = pick_installer(&assets, Platform::Windows).expect("pick the msi");
        assert_eq!(picked.name, "CCP-Bot_0.2.3_x64_en-US.msi");
    }

    #[test]
    fn a_mac_install_takes_the_bundle_for_this_architecture() {
        // Two DMGs with the same version differ only by architecture, so the pick has to read the
        // filename rather than the extension.
        let assets = vec![
            asset("CCP-Bot_0.2.3_x64.dmg"),
            asset("CCP-Bot_0.2.3_aarch64.dmg"),
        ];
        let picked = pick_installer(&assets, Platform::MacOs).expect("pick a dmg");
        let wanted = if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            "x64"
        };
        assert!(
            picked.name.contains(wanted),
            "{} should match {wanted}",
            picked.name
        );
    }

    #[test]
    fn an_installer_for_the_other_architecture_is_refused_not_offered() {
        // The release publishes only the other architecture: an Apple-silicon `.dmg` cannot run on
        // an Intel Mac, so the honest answer is "no installer for this machine" rather than a
        // download that fails at launch.
        let foreign = if cfg!(target_arch = "aarch64") {
            "CCP-Bot_0.2.3_x64.dmg"
        } else {
            "CCP-Bot_0.2.3_aarch64.dmg"
        };
        let assets = vec![asset(foreign)];
        let error = pick_installer(&assets, Platform::MacOs).expect_err("refuse a foreign bundle");
        let text = error.to_string();
        assert!(text.contains("另一种架构"), "{text}");
        assert!(text.to_lowercase().contains(&foreign.to_lowercase()), "{text}");
    }
    #[test]
    fn a_release_without_an_installer_does_not_pretend_it_has_one() {
        let assets = vec![asset("container-images.json"), asset("ccpbot.tar.gz")];
        let error = pick_installer(&assets, Platform::Windows).expect_err("no installer here");
        assert!(error.to_string().contains("没有适用于当前系统的安装包"));
    }

    /// Downloads the newest published installer for this machine, without running it.
    ///
    /// Ignored: it fetches about a gigabyte over the network. Run it on purpose, against a release
    /// that actually publishes installers, to prove the selection and the download end to end —
    /// the running half is exercised by `install_latest` when a person presses the button.
    #[tokio::test]
    #[ignore = "downloads the published installer (~1 GB)"]
    async fn live_latest_release_offers_this_machines_installer() {
        let (tag, asset, path) = download_latest_installer().await.expect("download it");
        assert!(tag.starts_with('v'), "{tag}");
        let bytes = fs::metadata(&path).expect("the installer is on disk").len();
        assert!(bytes > 100_000, "{asset} is only {bytes} bytes");
        let lower = asset.to_lowercase();
        assert!(
            lower.ends_with("-setup.exe") || lower.ends_with(".msi"),
            "{asset} is not a Windows installer"
        );
    }
}
