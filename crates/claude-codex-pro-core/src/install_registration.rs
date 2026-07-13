use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::Duration;

pub const INSTALLATION_REGISTRATION_ENDPOINT: &str =
    "https://connect-worker.solitaryzj.workers.dev/api/tools/claude-codex-pro/register";

const INSTALLATION_HASH_NAMESPACE: &[u8] = b"claude-codex-pro-tool:installation:v1\n";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallationRegistrationRequest<'a> {
    installation_id: &'a str,
    app_version: &'a str,
    platform: &'a str,
}

pub async fn register_current_installation(app_version: &str) -> Result<()> {
    #[cfg(windows)]
    {
        let serial = read_windows_baseboard_serial()?;
        return register_baseboard_serial_at(
            &serial,
            app_version,
            INSTALLATION_REGISTRATION_ENDPOINT,
        )
        .await;
    }

    #[cfg(not(windows))]
    {
        let _ = app_version;
        anyhow::bail!("installation registration is only available on Windows")
    }
}

pub async fn register_baseboard_serial_at(
    raw_serial: &str,
    app_version: &str,
    endpoint: &str,
) -> Result<()> {
    let installation_id = installation_id_from_baseboard_serial(raw_serial)
        .context("Windows did not provide a usable baseboard serial number")?;
    validate_app_version(app_version)?;
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        anyhow::bail!("installation registration endpoint is empty");
    }

    let payload = InstallationRegistrationRequest {
        installation_id: &installation_id,
        app_version,
        platform: "windows",
    };
    let client = reqwest::Client::builder()
        .user_agent(format!("ClaudeCodexProInstaller/{app_version}"))
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("build installation registration client")?;
    let response = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .context("send installation registration")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "installation registration endpoint returned HTTP {}",
            response.status().as_u16()
        );
    }
    Ok(())
}

pub fn installation_id_from_baseboard_serial(raw_serial: &str) -> Option<String> {
    let normalized = normalize_baseboard_serial(raw_serial)?;
    let mut hasher = Sha256::new();
    hasher.update(INSTALLATION_HASH_NAMESPACE);
    hasher.update(normalized.as_bytes());
    Some(format!("{:x}", hasher.finalize()))
}

fn normalize_baseboard_serial(raw_serial: &str) -> Option<String> {
    let normalized = raw_serial
        .trim_start_matches('\u{feff}')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase();
    if normalized.is_empty() || normalized.len() > 256 {
        return None;
    }

    let placeholder = matches!(
        normalized.as_str(),
        "UNKNOWN"
            | "DEFAULT STRING"
            | "TO BE FILLED BY O.E.M."
            | "TO BE FILLED BY OEM"
            | "NONE"
            | "N/A"
            | "NOT APPLICABLE"
            | "SYSTEM SERIAL NUMBER"
    );
    let compact = normalized
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    if placeholder || compact.is_empty() || compact.chars().all(|character| character == '0') {
        return None;
    }
    Some(normalized)
}

fn validate_app_version(app_version: &str) -> Result<()> {
    let valid = !app_version.is_empty()
        && app_version.len() <= 32
        && app_version
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".+_-".contains(character));
    if !valid {
        anyhow::bail!("invalid application version for installation registration");
    }
    Ok(())
}

#[cfg(windows)]
fn read_windows_baseboard_serial() -> Result<String> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::Instant;

    const HARDWARE_QUERY_TIMEOUT: Duration = Duration::from_secs(6);
    const POWERSHELL_QUERY: &str = concat!(
        "$ErrorActionPreference = 'Stop'; ",
        "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); ",
        "$board = Get-CimInstance -ClassName Win32_BaseBoard -OperationTimeoutSec 3 | Select-Object -First 1; ",
        "if ($null -ne $board) { [Console]::Out.Write([string]$board.SerialNumber) }"
    );

    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            POWERSHELL_QUERY,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(crate::windows_integration::CREATE_NO_WINDOW);

    let mut child = command.spawn().context("start Windows baseboard query")?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .context("wait for Windows baseboard query")?
        {
            break status;
        }
        if started.elapsed() >= HARDWARE_QUERY_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("Windows baseboard query timed out");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    if !status.success() {
        anyhow::bail!("Windows baseboard query failed");
    }

    let mut output = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout
            .read_to_end(&mut output)
            .context("read Windows baseboard query output")?;
    }
    String::from_utf8(output).context("Windows baseboard query returned invalid UTF-8")
}
