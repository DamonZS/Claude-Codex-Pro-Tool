use std::path::Path;

use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopStatus {
    pub status: String,
    pub message: String,
    pub process_count: usize,
    pub executable_paths: Vec<String>,
    pub install_kind: String,
    pub cdp_status: String,
    pub cdp_blocker: String,
    pub debug_flags_present: bool,
    pub debug_ports: Vec<u16>,
    pub listening_ports: Vec<u16>,
    pub debug_evidence: Vec<String>,
    pub supported_integration: String,
    pub integrity_status: String,
    pub integrity_message: String,
    pub executable_audits: Vec<ClaudeDesktopExecutableAudit>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopActionResult {
    pub status: String,
    pub message: String,
    pub process_id: Option<u32>,
    pub action: String,
    pub foreground_verified: bool,
    pub foreground_process_id: Option<u32>,
    pub foreground_title: Option<String>,
    pub observed_window_titles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopDraftResult {
    pub status: String,
    pub message: String,
    pub process_id: Option<u32>,
    pub action: String,
    pub input_chars: usize,
    pub auto_submitted: bool,
    pub foreground_verified: bool,
    pub foreground_process_id: Option<u32>,
    pub foreground_title: Option<String>,
    pub observed_window_titles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopExecutableAudit {
    pub path: String,
    pub exists: bool,
    pub file_size_bytes: Option<u64>,
    pub modified_unix_ms: Option<u128>,
    pub sha256: Option<String>,
    pub pe_format: Option<String>,
    pub pe_machine: Option<String>,
    pub pe_subsystem: Option<String>,
    pub pe_timestamp_unix: Option<u64>,
    pub pe_entry_point_rva: Option<u32>,
    pub pe_image_base: Option<u64>,
    pub pe_section_count: Option<u16>,
    pub pe_certificate_table_bytes: Option<u32>,
    pub pe_sections: Vec<ClaudeDesktopPeSectionAudit>,
    pub signature_status: Option<String>,
    pub signature_message: Option<String>,
    pub signer_subject: Option<String>,
    pub signer_issuer: Option<String>,
    pub signer_thumbprint: Option<String>,
    pub signer_serial_number: Option<String>,
    pub signer_not_before: Option<String>,
    pub signer_not_after: Option<String>,
    pub signer_chain_status: Option<String>,
    pub product_name: Option<String>,
    pub file_description: Option<String>,
    pub file_version: Option<String>,
    pub original_filename: Option<String>,
    pub install_kind: String,
    pub trust_basis: String,
    pub integrity_level: String,
    pub risk_level: String,
    pub verification_scope: String,
    pub mutation_policy: String,
    pub patch_eligible: bool,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopPeSectionAudit {
    pub name: String,
    pub virtual_address: u32,
    pub virtual_size: u32,
    pub raw_size: u32,
    pub raw_sha256: Option<String>,
    pub characteristics: String,
}

pub fn status_response() -> Value {
    json!(detect_status())
}

pub fn integrity_response() -> Value {
    json!(detect_integrity())
}

pub fn focus_response() -> Value {
    json!(focus_claude_window())
}

pub fn verify_response() -> Value {
    json!(verify_claude_target())
}

pub fn open_response() -> Value {
    json!(open_claude_desktop())
}

pub fn new_chat_response() -> Value {
    json!(new_claude_chat())
}

pub fn open_devtools_response() -> Value {
    json!(open_claude_devtools())
}

pub fn enter_desktop_devtools_response() -> Value {
    json!(enter_claude_desktop_devtools())
}

pub fn draft_response(payload: &Value) -> Value {
    let text = payload
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    json!(paste_draft_to_claude(text))
}

pub fn submit_response(payload: &Value) -> Value {
    let text = payload
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    json!(submit_text_to_claude(text))
}

pub fn detect_status() -> ClaudeDesktopStatus {
    let (process_count, executable_paths) = claude_process_inventory();
    let install_kind = install_kind(&executable_paths);
    let process_ids = claude_process_ids();
    let debug_probe = detect_debug_probe(&process_ids);
    let cdp_blocker = cdp_blocker_for_install_kind(&install_kind);
    let (integrity_status, integrity_message, executable_audits) =
        integrity_for_paths(&executable_paths);
    let cdp_status = if debug_probe.debug_flags_present || !debug_probe.debug_ports.is_empty() {
        "observed_but_unverified".to_string()
    } else {
        "blocked".to_string()
    };
    let status = if process_count > 0 {
        "ok"
    } else {
        "not_running"
    }
    .to_string();
    let message = if process_count > 0 {
        format!("Claude Desktop is running ({process_count} process group entries detected).")
    } else {
        "Claude Desktop is not running.".to_string()
    };

    ClaudeDesktopStatus {
        status,
        message,
        process_count,
        executable_paths,
        install_kind,
        cdp_status,
        cdp_blocker,
        debug_flags_present: debug_probe.debug_flags_present,
        debug_ports: debug_probe.debug_ports,
        listening_ports: debug_probe.listening_ports,
        debug_evidence: debug_probe.evidence,
        supported_integration: "external_automation".to_string(),
        integrity_status,
        integrity_message,
        executable_audits,
    }
}

pub fn detect_integrity() -> Value {
    let (_, executable_paths) = claude_process_inventory();
    json!(detect_integrity_status_for_paths(&executable_paths))
}

pub fn detect_integrity_status() -> ClaudeDesktopIntegrityStatus {
    let (_, executable_paths) = claude_process_inventory();
    detect_integrity_status_for_paths(&executable_paths)
}

fn detect_integrity_status_for_paths(executable_paths: &[String]) -> ClaudeDesktopIntegrityStatus {
    let (status, message, executable_audits) = integrity_for_paths(&executable_paths);
    ClaudeDesktopIntegrityStatus {
        status,
        message,
        executable_audits,
        policy: "read_only_audit_no_executable_or_asar_patch".to_string(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopIntegrityStatus {
    pub status: String,
    pub message: String,
    pub executable_audits: Vec<ClaudeDesktopExecutableAudit>,
    pub policy: String,
}

pub fn focus_claude_window() -> ClaudeDesktopActionResult {
    let candidates = claude_process_ids();
    if candidates.is_empty() {
        return ClaudeDesktopActionResult {
            status: "failed".to_string(),
            message: "Claude Desktop is not running.".to_string(),
            process_id: None,
            action: "focus".to_string(),
            foreground_verified: false,
            foreground_process_id: None,
            foreground_title: None,
            observed_window_titles: Vec::new(),
        };
    }

    for process_id in candidates {
        if activate_process_window(process_id) {
            let foreground = claude_foreground_evidence(&[process_id]);
            return ClaudeDesktopActionResult {
                status: "ok".to_string(),
                message: "Claude Desktop window focused.".to_string(),
                process_id: Some(process_id),
                action: "focus".to_string(),
                foreground_verified: foreground.verified,
                foreground_process_id: foreground.process_id,
                foreground_title: foreground.title,
                observed_window_titles: visible_window_titles_for_process(process_id),
            };
        }
    }

    ClaudeDesktopActionResult {
        status: "failed".to_string(),
        message: "Claude Desktop is running, but no visible window could be focused.".to_string(),
        process_id: None,
        action: "focus".to_string(),
        foreground_verified: false,
        foreground_process_id: None,
        foreground_title: None,
        observed_window_titles: Vec::new(),
    }
}

pub fn verify_claude_target() -> ClaudeDesktopActionResult {
    let focus = focus_claude_window();
    let foreground = claude_process_ids();
    let evidence = claude_foreground_evidence(&foreground);
    let status = if focus.status == "ok" && evidence.verified {
        "ok"
    } else {
        "failed"
    }
    .to_string();
    let message = if evidence.verified {
        "Claude Desktop target window verified without sending input.".to_string()
    } else if focus.status == "ok" {
        "Claude Desktop was focused, but foreground ownership could not be verified.".to_string()
    } else {
        focus.message
    };
    ClaudeDesktopActionResult {
        status,
        message,
        process_id: focus.process_id,
        action: "verify_target".to_string(),
        foreground_verified: evidence.verified,
        foreground_process_id: evidence.process_id,
        foreground_title: evidence.title,
        observed_window_titles: focus
            .process_id
            .map(visible_window_titles_for_process)
            .unwrap_or_default(),
    }
}

pub fn open_claude_desktop() -> ClaudeDesktopActionResult {
    let existing_process_ids = claude_process_ids();
    let is_restart = !existing_process_ids.is_empty();
    if is_restart {
        let terminated = terminate_claude_processes(&existing_process_ids);
        if terminated == 0 {
            return ClaudeDesktopActionResult {
                status: "failed".to_string(),
                message: "Claude Desktop is running, but existing processes could not be terminated before restart.".to_string(),
                process_id: existing_process_ids.first().copied(),
                action: "restart".to_string(),
                foreground_verified: false,
                foreground_process_id: None,
                foreground_title: None,
                observed_window_titles: Vec::new(),
            };
        }
        if !wait_for_claude_process_exit(&existing_process_ids, std::time::Duration::from_secs(5)) {
            return ClaudeDesktopActionResult {
                status: "failed".to_string(),
                message: "Claude Desktop restart was requested, but existing processes did not exit in time.".to_string(),
                process_id: existing_process_ids.first().copied(),
                action: "restart".to_string(),
                foreground_verified: false,
                foreground_process_id: None,
                foreground_title: None,
                observed_window_titles: Vec::new(),
            };
        }
    }

    match launch_claude_desktop_app() {
        Ok(()) => ClaudeDesktopActionResult {
            status: "accepted".to_string(),
            message: if is_restart {
                "Claude Desktop was closed and restart was requested through the Windows app registry."
                    .to_string()
            } else {
                "Claude Desktop launch was requested through the Windows app registry.".to_string()
            },
            process_id: None,
            action: if is_restart { "restart" } else { "open" }.to_string(),
            foreground_verified: false,
            foreground_process_id: None,
            foreground_title: None,
            observed_window_titles: Vec::new(),
        },
        Err(error) => ClaudeDesktopActionResult {
            status: "failed".to_string(),
            message: if is_restart {
                format!("Claude Desktop was closed, but restart failed: {error}")
            } else {
                format!("Unable to launch Claude Desktop: {error}")
            },
            process_id: None,
            action: if is_restart { "restart" } else { "open" }.to_string(),
            foreground_verified: false,
            foreground_process_id: None,
            foreground_title: None,
            observed_window_titles: Vec::new(),
        },
    }
}

pub fn enter_claude_desktop_devtools() -> ClaudeDesktopActionResult {
    let opened = open_claude_desktop();
    if matches!(opened.status.as_str(), "failed" | "not_implemented") {
        return ClaudeDesktopActionResult {
            action: "enter_desktop_devtools".to_string(),
            ..opened
        };
    }

    let focus = match wait_for_claude_focus() {
        Some(focus) => focus,
        None => {
            let status = detect_status();
            return ClaudeDesktopActionResult {
                status: "accepted".to_string(),
                message: format!(
                    "Claude Desktop launch was requested, but no foreground window was verified yet. CDP remains {}: {}",
                    status.cdp_status, status.cdp_blocker
                ),
                process_id: opened.process_id,
                action: "enter_desktop_devtools".to_string(),
                foreground_verified: false,
                foreground_process_id: None,
                foreground_title: None,
                observed_window_titles: opened.observed_window_titles,
            };
        }
    };

    if let Err(error) = open_devtools_in_foreground_window(focus.process_id) {
        return ClaudeDesktopActionResult {
            status: "failed".to_string(),
            message: format!(
                "Claude Desktop was focused, but opening developer tools failed: {error}"
            ),
            process_id: focus.process_id,
            action: "enter_desktop_devtools".to_string(),
            foreground_verified: false,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: focus.observed_window_titles,
        };
    }

    let process_id = focus.process_id.unwrap_or_default();
    let devtools = observe_devtools_window(process_id);
    if devtools.matched {
        let status = detect_status();
        return ClaudeDesktopActionResult {
            status: "ok".to_string(),
            message: format!(
                "Developer tools shortcut was sent and a DevTools window title was observed. CDP is still {}: {}",
                status.cdp_status, status.cdp_blocker
            ),
            process_id: focus.process_id,
            action: "enter_desktop_devtools".to_string(),
            foreground_verified: true,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: devtools.titles,
        };
    }

    std::thread::sleep(std::time::Duration::from_millis(300));
    if send_f12_devtools_shortcut() {
        let second = observe_devtools_window(process_id);
        if second.matched {
            let status = detect_status();
            return ClaudeDesktopActionResult {
                status: "ok".to_string(),
                message: format!(
                    "F12 was sent and a DevTools window title was observed. CDP is still {}: {}",
                    status.cdp_status, status.cdp_blocker
                ),
                process_id: focus.process_id,
                action: "enter_desktop_devtools".to_string(),
                foreground_verified: true,
                foreground_process_id: focus.foreground_process_id,
                foreground_title: focus.foreground_title,
                observed_window_titles: second.titles,
            };
        }
        let status = detect_status();
        let observed_suffix = if second.titles.is_empty() {
            String::new()
        } else {
            format!(" Observed windows: {}.", second.titles.join(" | "))
        };
        return ClaudeDesktopActionResult {
            status: "failed".to_string(),
            message: format!(
                "Ctrl+Shift+I did not produce a DevTools window, and F12 did not either. CDP is still {}: {}{}",
                status.cdp_status, status.cdp_blocker, observed_suffix
            ),
            process_id: focus.process_id,
            action: "enter_desktop_devtools".to_string(),
            foreground_verified: focus.foreground_verified,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: second.titles,
        };
    }

    let status = detect_status();
    let observed_suffix = if devtools.titles.is_empty() {
        String::new()
    } else {
        format!(" Observed windows: {}.", devtools.titles.join(" | "))
    };
    ClaudeDesktopActionResult {
        status: "failed".to_string(),
        message: format!(
            "Ctrl+Shift+I did not produce a DevTools window. CDP is still {}: {}{}",
            status.cdp_status, status.cdp_blocker, observed_suffix
        ),
        process_id: focus.process_id,
        action: "enter_desktop_devtools".to_string(),
        foreground_verified: focus.foreground_verified,
        foreground_process_id: focus.foreground_process_id,
        foreground_title: focus.foreground_title,
        observed_window_titles: devtools.titles,
    }
}

pub fn new_claude_chat() -> ClaudeDesktopActionResult {
    let focus = focus_claude_window();
    if focus.status != "ok" {
        return ClaudeDesktopActionResult {
            status: "failed".to_string(),
            message: focus.message,
            process_id: focus.process_id,
            action: "new_chat".to_string(),
            foreground_verified: focus.foreground_verified,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: focus.observed_window_titles,
        };
    }

    match new_chat_in_foreground_window(focus.process_id) {
        Ok(()) => ClaudeDesktopActionResult {
            status: "ok".to_string(),
            message: "Claude Desktop new chat shortcut was sent.".to_string(),
            process_id: focus.process_id,
            action: "new_chat".to_string(),
            foreground_verified: true,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: focus.observed_window_titles,
        },
        Err(error) => ClaudeDesktopActionResult {
            status: "failed".to_string(),
            message: format!("Claude Desktop was focused, but new chat failed: {error}"),
            process_id: focus.process_id,
            action: "new_chat".to_string(),
            foreground_verified: false,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: focus.observed_window_titles,
        },
    }
}

pub fn open_claude_devtools() -> ClaudeDesktopActionResult {
    let focus = focus_claude_window();
    if focus.status != "ok" {
        return ClaudeDesktopActionResult {
            status: "failed".to_string(),
            message: focus.message,
            process_id: focus.process_id,
            action: "open_devtools".to_string(),
            foreground_verified: focus.foreground_verified,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: focus.observed_window_titles,
        };
    }

    match open_devtools_in_foreground_window(focus.process_id) {
        Ok(()) => ClaudeDesktopActionResult {
            status: "ok".to_string(),
            message: "Claude Desktop developer tools shortcut was sent.".to_string(),
            process_id: focus.process_id,
            action: "open_devtools".to_string(),
            foreground_verified: true,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: focus.observed_window_titles,
        },
        Err(error) => ClaudeDesktopActionResult {
            status: "failed".to_string(),
            message: format!("Claude Desktop was focused, but developer tools failed: {error}"),
            process_id: focus.process_id,
            action: "open_devtools".to_string(),
            foreground_verified: false,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: focus.observed_window_titles,
        },
    }
}

pub fn paste_draft_to_claude(text: &str) -> ClaudeDesktopDraftResult {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ClaudeDesktopDraftResult {
            status: "failed".to_string(),
            message: "Draft text is empty.".to_string(),
            process_id: None,
            action: "paste_draft".to_string(),
            input_chars: 0,
            auto_submitted: false,
            foreground_verified: false,
            foreground_process_id: None,
            foreground_title: None,
            observed_window_titles: Vec::new(),
        };
    }

    let focus = focus_claude_window();
    if focus.status != "ok" {
        return ClaudeDesktopDraftResult {
            status: "failed".to_string(),
            message: focus.message,
            process_id: focus.process_id,
            action: "paste_draft".to_string(),
            input_chars: trimmed.chars().count(),
            auto_submitted: false,
            foreground_verified: focus.foreground_verified,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: focus.observed_window_titles,
        };
    }

    match paste_text_to_foreground_window(trimmed, focus.process_id) {
        Ok(()) => ClaudeDesktopDraftResult {
            status: "ok".to_string(),
            message: "Draft text pasted into Claude Desktop. Review and send manually.".to_string(),
            process_id: focus.process_id,
            action: "paste_draft".to_string(),
            input_chars: trimmed.chars().count(),
            auto_submitted: false,
            foreground_verified: true,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: focus.observed_window_titles,
        },
        Err(error) => ClaudeDesktopDraftResult {
            status: "failed".to_string(),
            message: format!("Claude Desktop was focused, but paste failed: {error}"),
            process_id: focus.process_id,
            action: "paste_draft".to_string(),
            input_chars: trimmed.chars().count(),
            auto_submitted: false,
            foreground_verified: false,
            foreground_process_id: focus.foreground_process_id,
            foreground_title: focus.foreground_title,
            observed_window_titles: focus.observed_window_titles,
        },
    }
}

pub fn submit_text_to_claude(text: &str) -> ClaudeDesktopDraftResult {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ClaudeDesktopDraftResult {
            status: "failed".to_string(),
            message: "Submit text is empty.".to_string(),
            process_id: None,
            action: "paste_and_submit".to_string(),
            input_chars: 0,
            auto_submitted: false,
            foreground_verified: false,
            foreground_process_id: None,
            foreground_title: None,
            observed_window_titles: Vec::new(),
        };
    }

    let pasted = paste_draft_to_claude(trimmed);
    if pasted.status != "ok" {
        return ClaudeDesktopDraftResult {
            action: "paste_and_submit".to_string(),
            ..pasted
        };
    }

    match submit_foreground_window(pasted.process_id) {
        Ok(()) => ClaudeDesktopDraftResult {
            status: "ok".to_string(),
            message: "Text pasted and submitted to Claude Desktop.".to_string(),
            process_id: pasted.process_id,
            action: "paste_and_submit".to_string(),
            input_chars: pasted.input_chars,
            auto_submitted: true,
            foreground_verified: true,
            foreground_process_id: pasted.foreground_process_id,
            foreground_title: pasted.foreground_title,
            observed_window_titles: pasted.observed_window_titles,
        },
        Err(error) => ClaudeDesktopDraftResult {
            status: "failed".to_string(),
            message: format!("Text was pasted, but submit failed: {error}"),
            process_id: pasted.process_id,
            action: "paste_and_submit".to_string(),
            input_chars: pasted.input_chars,
            auto_submitted: false,
            foreground_verified: false,
            foreground_process_id: pasted.foreground_process_id,
            foreground_title: pasted.foreground_title,
            observed_window_titles: pasted.observed_window_titles,
        },
    }
}

#[cfg(windows)]
fn claude_process_inventory() -> (usize, Vec<String>) {
    let processes = crate::windows_integration::enumerate_processes()
        .into_iter()
        .filter(|process| process.exe_file.eq_ignore_ascii_case("claude.exe"))
        .collect::<Vec<_>>();
    let mut paths = processes
        .iter()
        .filter_map(|process| {
            process
                .executable_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    (processes.len(), paths)
}

pub fn close_claude_desktop_for_patch() -> bool {
    let process_ids = claude_process_ids();
    if process_ids.is_empty() {
        return true;
    }
    if terminate_claude_processes(&process_ids) == 0 {
        return false;
    }
    wait_for_claude_process_exit(&process_ids, std::time::Duration::from_secs(5))
}

#[cfg(windows)]
fn claude_process_ids() -> Vec<u32> {
    let mut ids = crate::windows_integration::enumerate_processes()
        .into_iter()
        .filter(|process| process.exe_file.eq_ignore_ascii_case("claude.exe"))
        .map(|process| process.process_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

#[cfg(not(windows))]
fn claude_process_ids() -> Vec<u32> {
    Vec::new()
}

#[cfg(windows)]
fn terminate_claude_processes(process_ids: &[u32]) -> usize {
    process_ids
        .iter()
        .copied()
        .filter(|process_id| crate::windows_integration::terminate_process(*process_id))
        .count()
}

#[cfg(not(windows))]
fn terminate_claude_processes(_process_ids: &[u32]) -> usize {
    0
}

fn wait_for_claude_process_exit(process_ids: &[u32], timeout: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let running = claude_process_ids();
        if process_ids
            .iter()
            .all(|process_id| !running.contains(process_id))
        {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
}

#[cfg(windows)]
fn activate_process_window(process_id: u32) -> bool {
    crate::windows_activate_process_window(process_id)
}

#[cfg(not(windows))]
fn activate_process_window(_process_id: u32) -> bool {
    false
}

#[cfg(windows)]
fn paste_text_to_foreground_window(text: &str, process_id: Option<u32>) -> anyhow::Result<()> {
    ensure_claude_foreground(process_id)?;
    crate::windows_integration::set_clipboard_text(text)?;
    std::thread::sleep(std::time::Duration::from_millis(120));
    ensure_claude_foreground(process_id)?;
    if !crate::windows_integration::send_ctrl_v() {
        anyhow::bail!("Ctrl+V input was not accepted by Windows");
    }
    Ok(())
}

#[cfg(windows)]
fn submit_foreground_window(process_id: Option<u32>) -> anyhow::Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(180));
    ensure_claude_foreground(process_id)?;
    if !crate::windows_integration::send_enter() {
        anyhow::bail!("Enter input was not accepted by Windows");
    }
    Ok(())
}

#[cfg(windows)]
fn new_chat_in_foreground_window(process_id: Option<u32>) -> anyhow::Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(120));
    ensure_claude_foreground(process_id)?;
    if !crate::windows_integration::send_ctrl_n() {
        anyhow::bail!("Ctrl+N input was not accepted by Windows");
    }
    Ok(())
}

#[cfg(windows)]
fn open_devtools_in_foreground_window(process_id: Option<u32>) -> anyhow::Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(120));
    ensure_claude_foreground(process_id)?;
    if !crate::windows_integration::send_ctrl_shift_i() {
        anyhow::bail!("Ctrl+Shift+I input was not accepted by Windows");
    }
    Ok(())
}

struct DevtoolsWindowObservation {
    matched: bool,
    titles: Vec<String>,
}

#[cfg(windows)]
fn observe_devtools_window(process_id: u32) -> DevtoolsWindowObservation {
    let mut last_titles = Vec::new();
    for _ in 0..12 {
        let titles = visible_window_titles_for_process(process_id);
        let found = titles.iter().any(|title| {
            let lowered = title.to_ascii_lowercase();
            lowered.contains("devtools") || lowered.contains("developer tools")
        });
        if found {
            return DevtoolsWindowObservation {
                matched: true,
                titles,
            };
        }
        last_titles = titles;
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    DevtoolsWindowObservation {
        matched: false,
        titles: last_titles,
    }
}

#[cfg(not(windows))]
fn observe_devtools_window(_process_id: u32) -> DevtoolsWindowObservation {
    DevtoolsWindowObservation {
        matched: false,
        titles: Vec::new(),
    }
}

#[cfg(windows)]
fn send_f12_devtools_shortcut() -> bool {
    crate::windows_integration::send_f12()
}

#[cfg(not(windows))]
fn send_f12_devtools_shortcut() -> bool {
    false
}

#[cfg(windows)]
fn wait_for_claude_focus() -> Option<ClaudeDesktopActionResult> {
    for _ in 0..30 {
        let focus = focus_claude_window();
        if focus.status == "ok" && focus.foreground_verified {
            return Some(focus);
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    None
}

#[cfg(not(windows))]
fn paste_text_to_foreground_window(_text: &str, _process_id: Option<u32>) -> anyhow::Result<()> {
    anyhow::bail!("Claude Desktop draft paste is only supported on Windows")
}

#[cfg(not(windows))]
fn submit_foreground_window(_process_id: Option<u32>) -> anyhow::Result<()> {
    anyhow::bail!("Claude Desktop submit is only supported on Windows")
}

#[cfg(not(windows))]
fn new_chat_in_foreground_window(_process_id: Option<u32>) -> anyhow::Result<()> {
    anyhow::bail!("Claude Desktop new chat is only supported on Windows")
}

#[cfg(not(windows))]
fn open_devtools_in_foreground_window(_process_id: Option<u32>) -> anyhow::Result<()> {
    anyhow::bail!("Claude Desktop developer tools are only supported on Windows")
}

#[cfg(not(windows))]
fn wait_for_claude_focus() -> Option<ClaudeDesktopActionResult> {
    None
}

#[cfg(windows)]
fn claude_foreground_verified(process_ids: &[u32]) -> bool {
    claude_foreground_evidence(process_ids).verified
}

#[cfg(not(windows))]
fn claude_foreground_verified(_process_ids: &[u32]) -> bool {
    false
}

#[derive(Clone, Debug, Default)]
struct ForegroundEvidence {
    verified: bool,
    process_id: Option<u32>,
    title: Option<String>,
}

#[cfg(windows)]
fn claude_foreground_evidence(process_ids: &[u32]) -> ForegroundEvidence {
    let Some(info) = crate::windows_integration::foreground_window_info() else {
        return ForegroundEvidence::default();
    };
    ForegroundEvidence {
        verified: process_ids.contains(&info.process_id),
        process_id: Some(info.process_id),
        title: info.title,
    }
}

#[cfg(not(windows))]
fn claude_foreground_evidence(_process_ids: &[u32]) -> ForegroundEvidence {
    ForegroundEvidence::default()
}

#[cfg(windows)]
fn visible_window_titles_for_process(process_id: u32) -> Vec<String> {
    let mut titles = crate::windows_integration::visible_window_infos_for_process(process_id)
        .into_iter()
        .filter_map(|window| window.title)
        .collect::<Vec<_>>();
    titles.sort();
    titles.dedup();
    titles
}

#[cfg(not(windows))]
fn visible_window_titles_for_process(_process_id: u32) -> Vec<String> {
    Vec::new()
}

#[cfg(windows)]
fn ensure_claude_foreground(process_id: Option<u32>) -> anyhow::Result<()> {
    let Some(process_id) = process_id else {
        anyhow::bail!("Claude Desktop process id is not available for foreground verification");
    };
    if claude_foreground_verified(&[process_id]) {
        return Ok(());
    }
    anyhow::bail!("foreground window is no longer owned by Claude Desktop")
}

#[cfg(windows)]
fn launch_claude_desktop_app() -> anyhow::Result<()> {
    let script = r#"
$app = Get-StartApps |
  Where-Object { $_.Name -eq 'Claude' -or $_.AppID -like 'Claude_*!Claude' } |
  Select-Object -First 1
if (-not $app) {
  $app = Get-StartApps |
    Where-Object { $_.Name -like '*Claude*' -or $_.AppID -like '*Claude*' } |
    Select-Object -First 1
}
if (-not $app) { throw 'Claude Desktop Start menu app entry was not found.' }
Start-Process ('shell:AppsFolder\' + $app.AppID)
"#;
    let mut command = std::process::Command::new("powershell.exe");
    command.args(["-NoProfile", "-Command", script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::windows_create_no_window());
    }
    let output = command.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let message = if !stderr.is_empty() { stderr } else { stdout };
        let message = if message.is_empty() {
            "PowerShell launch failed".to_string()
        } else {
            message
        };
        anyhow::bail!("{message}");
    }
    Ok(())
}

#[cfg(not(windows))]
fn launch_claude_desktop_app() -> anyhow::Result<()> {
    anyhow::bail!("Claude Desktop launch is only supported on Windows")
}

#[cfg(not(windows))]
fn claude_process_inventory() -> (usize, Vec<String>) {
    (0, Vec::new())
}

fn install_kind(paths: &[String]) -> String {
    if paths.is_empty() {
        return "unknown".to_string();
    }
    if paths
        .iter()
        .any(|path| path.to_ascii_lowercase().contains("\\windowsapps\\"))
    {
        return "msix".to_string();
    }
    "desktop".to_string()
}

fn cdp_blocker_for_install_kind(install_kind: &str) -> String {
    match install_kind {
        "msix" => "Claude Desktop requires a short-lived CLAUDE_CDP_AUTH token before it accepts Chromium remote debugging flags.".to_string(),
        "desktop" => "CDP availability is not assumed; launch-time probing must prove a Claude-owned debugging port before DOM injection is attempted.".to_string(),
        _ => "Claude Desktop is not running or its executable path is unknown.".to_string(),
    }
}

#[cfg(windows)]
fn detect_debug_probe(process_ids: &[u32]) -> DebugProbe {
    if process_ids.is_empty() {
        return DebugProbe::default();
    }

    let command_lines = windows_process_command_lines(process_ids);
    let mut debug_flags_present = false;
    let mut debug_ports = Vec::new();
    let mut evidence = Vec::new();

    for (pid, command_line) in command_lines {
        let lowered = command_line.to_ascii_lowercase();
        let mut pid_has_debug_flag = false;

        for needle in ["--remote-debugging-port=", "--remote-debugging-pipe"] {
            if lowered.contains(needle) {
                pid_has_debug_flag = true;
                debug_flags_present = true;
            }
        }

        if let Some(port) = extract_remote_debugging_port(&command_line) {
            debug_ports.push(port);
            evidence.push(format!(
                "pid {pid} command line includes --remote-debugging-port={port}"
            ));
        } else if pid_has_debug_flag {
            evidence.push(format!(
                "pid {pid} command line includes remote debugging flags"
            ));
        }
    }

    let mut listening_ports = windows_listening_ports_for_pids(process_ids);
    listening_ports.sort_unstable();
    listening_ports.dedup();
    debug_ports.sort_unstable();
    debug_ports.dedup();

    for port in &listening_ports {
        evidence.push(format!("Claude pid is listening on local TCP port {port}"));
    }

    DebugProbe {
        debug_flags_present,
        debug_ports,
        listening_ports,
        evidence,
    }
}

#[cfg(not(windows))]
fn detect_debug_probe(_process_ids: &[u32]) -> DebugProbe {
    DebugProbe::default()
}

#[cfg(windows)]
fn windows_process_command_lines(process_ids: &[u32]) -> Vec<(u32, String)> {
    let pid_csv = process_ids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let script = format!(
        r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [Console]::OutputEncoding
$pids = @({pid_csv})
Get-CimInstance Win32_Process |
  Where-Object {{ $pids -contains $_.ProcessId }} |
  Select-Object ProcessId, CommandLine |
  ConvertTo-Json -Depth 3 -Compress
"#
    );
    let Ok(output) = powershell_json(&script) else {
        return Vec::new();
    };
    let Some(value) = output else {
        return Vec::new();
    };
    json_rows_to_pid_string_pairs(&value, "ProcessId", "CommandLine")
}

#[cfg(windows)]
fn windows_listening_ports_for_pids(process_ids: &[u32]) -> Vec<u16> {
    let pid_csv = process_ids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let script = format!(
        r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [Console]::OutputEncoding
$pids = @({pid_csv})
Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue |
  Where-Object {{ $pids -contains $_.OwningProcess }} |
  Select-Object OwningProcess, LocalPort |
  ConvertTo-Json -Depth 3 -Compress
"#
    );
    let Ok(output) = powershell_json(&script) else {
        return Vec::new();
    };
    let Some(value) = output else {
        return Vec::new();
    };

    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.get("LocalPort").and_then(Value::as_u64))
            .filter_map(|port| u16::try_from(port).ok())
            .collect(),
        Value::Object(_) => value
            .get("LocalPort")
            .and_then(Value::as_u64)
            .and_then(|port| u16::try_from(port).ok())
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(windows)]
fn powershell_json(script: &str) -> anyhow::Result<Option<Value>> {
    let mut command = std::process::Command::new("powershell.exe");
    command.args(["-NoProfile", "-Command", script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::windows_create_no_window());
    }
    let output = command.output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(None);
    }
    Ok(serde_json::from_str::<Value>(&stdout).ok())
}

#[cfg(windows)]
fn json_rows_to_pid_string_pairs(
    value: &Value,
    pid_key: &str,
    text_key: &str,
) -> Vec<(u32, String)> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                let pid = item.get(pid_key).and_then(Value::as_u64)?;
                let text = item.get(text_key).and_then(Value::as_str)?.to_string();
                Some((u32::try_from(pid).ok()?, text))
            })
            .collect(),
        Value::Object(_) => {
            let pid = value.get(pid_key).and_then(Value::as_u64);
            let text = value.get(text_key).and_then(Value::as_str);
            match (pid.and_then(|pid| u32::try_from(pid).ok()), text) {
                (Some(pid), Some(text)) => vec![(pid, text.to_string())],
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

fn extract_remote_debugging_port(command_line: &str) -> Option<u16> {
    let flag = "--remote-debugging-port=";
    let index = command_line.to_ascii_lowercase().find(flag)?;
    let port_text = &command_line[index + flag.len()..];
    let digits = port_text
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u16>().ok()
}

fn integrity_for_paths(paths: &[String]) -> (String, String, Vec<ClaudeDesktopExecutableAudit>) {
    if paths.is_empty() {
        return (
            "not_checked".to_string(),
            "No Claude Desktop executable path was available to audit.".to_string(),
            Vec::new(),
        );
    }

    let audits = paths
        .iter()
        .map(|path| audit_executable_path(path))
        .collect::<Vec<_>>();
    let status = if audits.iter().any(|audit| {
        !audit.exists
            || audit.sha256.is_none()
            || !audit
                .signature_status
                .as_deref()
                .is_some_and(|status| status.eq_ignore_ascii_case("valid"))
    }) {
        "warning"
    } else {
        "ok"
    }
    .to_string();
    let message = format!(
        "Audited {} Claude Desktop executable path(s) without modifying app files.",
        audits.len()
    );
    (status, message, audits)
}

fn audit_executable_path(path: &str) -> ClaudeDesktopExecutableAudit {
    let install_kind = install_kind(&[path.to_string()]);
    let path_ref = Path::new(path);
    let metadata = std::fs::metadata(path_ref).ok();
    let sha256 = if metadata.as_ref().is_some_and(|item| item.is_file()) {
        sha256_file(path_ref).ok()
    } else {
        None
    };
    let pe = if metadata.as_ref().is_some_and(|item| item.is_file()) {
        pe_audit(path_ref)
    } else {
        PeAudit::default()
    };
    let signature = if metadata.as_ref().is_some_and(|item| item.is_file()) {
        signature_audit(path_ref)
    } else {
        SignatureAudit::default()
    };
    let modified_unix_ms = metadata
        .as_ref()
        .and_then(|item| item.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis());
    let has_valid_signature = signature
        .signature_status
        .as_deref()
        .is_some_and(|status| status.eq_ignore_ascii_case("valid"));
    let trust_basis = match (install_kind.as_str(), has_valid_signature) {
        ("msix", true) => "msix_protected_install_location_and_valid_authenticode",
        ("msix", false) => "msix_protected_install_location_signature_unverified",
        ("desktop", true) => "local_file_hash_and_valid_authenticode",
        ("desktop", false) => "local_file_hash_signature_unverified",
        _ => "unknown_path",
    }
    .to_string();
    let pe_is_parseable = pe
        .pe_format
        .as_deref()
        .is_some_and(|format| format == "pe32" || format == "pe32_plus");
    let risk_level = if metadata.is_none() || sha256.is_none() || !pe_is_parseable {
        "high"
    } else if has_valid_signature {
        "controlled"
    } else {
        "elevated"
    }
    .to_string();
    let mut notes = vec![
        "Read-only audit only; executable, app.asar, and integrity metadata are not modified."
            .to_string(),
        "Executable patching, signature bypass, and integrity metadata edits are blocked by policy."
            .to_string(),
    ];
    if install_kind == "msix" {
        notes.push(
            "MSIX package integrity and CLAUDE_CDP_AUTH remain authoritative blockers.".to_string(),
        );
    }
    if sha256.is_none() {
        notes.push("SHA-256 could not be computed for this path.".to_string());
    }
    if signature.signature_status.is_none() {
        notes.push(
            "Authenticode signature metadata could not be collected for this path.".to_string(),
        );
    }
    if !pe_is_parseable {
        notes.push(
            "PE header structure could not be parsed as a normal Windows executable.".to_string(),
        );
    }

    ClaudeDesktopExecutableAudit {
        path: path.to_string(),
        exists: metadata.is_some(),
        file_size_bytes: metadata.as_ref().map(|item| item.len()),
        modified_unix_ms,
        sha256,
        pe_format: pe.pe_format,
        pe_machine: pe.pe_machine,
        pe_subsystem: pe.pe_subsystem,
        pe_timestamp_unix: pe.pe_timestamp_unix,
        pe_entry_point_rva: pe.pe_entry_point_rva,
        pe_image_base: pe.pe_image_base,
        pe_section_count: pe.pe_section_count,
        pe_certificate_table_bytes: pe.pe_certificate_table_bytes,
        pe_sections: pe.pe_sections,
        signature_status: signature.signature_status,
        signature_message: signature.signature_message,
        signer_subject: signature.signer_subject,
        signer_issuer: signature.signer_issuer,
        signer_thumbprint: signature.signer_thumbprint,
        signer_serial_number: signature.signer_serial_number,
        signer_not_before: signature.signer_not_before,
        signer_not_after: signature.signer_not_after,
        signer_chain_status: signature.signer_chain_status,
        product_name: signature.product_name,
        file_description: signature.file_description,
        file_version: signature.file_version,
        original_filename: signature.original_filename,
        install_kind,
        trust_basis,
        integrity_level: "executable_hash_authenticode_pe_header_section_audit".to_string(),
        risk_level,
        verification_scope: "sha256_file_hash_authenticode_certificate_window_version_resource_pe_header_section_hashes_install_path"
            .to_string(),
        mutation_policy: "blocked_no_executable_asar_signature_or_integrity_metadata_changes"
            .to_string(),
        patch_eligible: false,
        notes,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SignatureAudit {
    signature_status: Option<String>,
    signature_message: Option<String>,
    signer_subject: Option<String>,
    signer_issuer: Option<String>,
    signer_thumbprint: Option<String>,
    signer_serial_number: Option<String>,
    signer_not_before: Option<String>,
    signer_not_after: Option<String>,
    signer_chain_status: Option<String>,
    product_name: Option<String>,
    file_description: Option<String>,
    file_version: Option<String>,
    original_filename: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DebugProbe {
    debug_flags_present: bool,
    debug_ports: Vec<u16>,
    listening_ports: Vec<u16>,
    evidence: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PeAudit {
    pe_format: Option<String>,
    pe_machine: Option<String>,
    pe_subsystem: Option<String>,
    pe_timestamp_unix: Option<u64>,
    pe_entry_point_rva: Option<u32>,
    pe_image_base: Option<u64>,
    pe_section_count: Option<u16>,
    pe_certificate_table_bytes: Option<u32>,
    pe_sections: Vec<ClaudeDesktopPeSectionAudit>,
}

#[cfg(windows)]
fn signature_audit(path: &Path) -> SignatureAudit {
    let script = r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [Console]::OutputEncoding
$path = $env:CLAUDE_CODEX_AUDIT_PATH
$signature = Get-AuthenticodeSignature -LiteralPath $path
$item = Get-Item -LiteralPath $path
$version = $item.VersionInfo
[pscustomobject]@{
  SignatureStatus = [string]$signature.Status
  SignatureMessage = [string]$signature.StatusMessage
  SignerSubject = if ($signature.SignerCertificate) { $signature.SignerCertificate.Subject } else { $null }
  SignerIssuer = if ($signature.SignerCertificate) { $signature.SignerCertificate.Issuer } else { $null }
  SignerThumbprint = if ($signature.SignerCertificate) { $signature.SignerCertificate.Thumbprint } else { $null }
  SignerSerialNumber = if ($signature.SignerCertificate) { $signature.SignerCertificate.SerialNumber } else { $null }
  SignerNotBefore = if ($signature.SignerCertificate) { $signature.SignerCertificate.NotBefore.ToUniversalTime().ToString('o') } else { $null }
  SignerNotAfter = if ($signature.SignerCertificate) { $signature.SignerCertificate.NotAfter.ToUniversalTime().ToString('o') } else { $null }
  SignerChainStatus = if ($signature.SignerCertificate) {
    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    [void]$chain.Build($signature.SignerCertificate)
    ($chain.ChainStatus | ForEach-Object { $_.Status.ToString() + ':' + $_.StatusInformation.Trim() }) -join '; '
  } else { $null }
  ProductName = [string]$version.ProductName
  FileDescription = [string]$version.FileDescription
  FileVersion = [string]$version.FileVersion
  OriginalFilename = [string]$version.OriginalFilename
} | ConvertTo-Json -Compress
"#;
    let mut command = std::process::Command::new("powershell.exe");
    command.args(["-NoProfile", "-Command", script]);
    command.env("CLAUDE_CODEX_AUDIT_PATH", path);
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::windows_create_no_window());
    }
    let Ok(output) = command.output() else {
        return SignatureAudit::default();
    };
    if !output.status.success() {
        return SignatureAudit::default();
    }
    let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) else {
        return SignatureAudit::default();
    };
    SignatureAudit {
        signature_status: json_string(&value, "SignatureStatus"),
        signature_message: json_string(&value, "SignatureMessage"),
        signer_subject: json_string(&value, "SignerSubject"),
        signer_issuer: json_string(&value, "SignerIssuer"),
        signer_thumbprint: json_string(&value, "SignerThumbprint"),
        signer_serial_number: json_string(&value, "SignerSerialNumber"),
        signer_not_before: json_string(&value, "SignerNotBefore"),
        signer_not_after: json_string(&value, "SignerNotAfter"),
        signer_chain_status: json_string(&value, "SignerChainStatus"),
        product_name: json_string(&value, "ProductName"),
        file_description: json_string(&value, "FileDescription"),
        file_version: json_string(&value, "FileVersion"),
        original_filename: json_string(&value, "OriginalFilename"),
    }
}

#[cfg(not(windows))]
fn signature_audit(_path: &Path) -> SignatureAudit {
    SignatureAudit::default()
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
}

fn pe_audit(path: &Path) -> PeAudit {
    let Ok(bytes) = std::fs::read(path) else {
        return PeAudit::default();
    };
    parse_pe_bytes(&bytes).unwrap_or_default()
}

fn parse_pe_bytes(bytes: &[u8]) -> Option<PeAudit> {
    if read_u16(bytes, 0)? != 0x5a4d {
        return None;
    }
    let pe_offset = usize::try_from(read_u32(bytes, 0x3c)?).ok()?;
    if read_u32(bytes, pe_offset)? != 0x0000_4550 {
        return None;
    }

    let file_header_offset = pe_offset.checked_add(4)?;
    let machine = read_u16(bytes, file_header_offset)?;
    let section_count = read_u16(bytes, file_header_offset + 2)?;
    let timestamp = read_u32(bytes, file_header_offset + 4)?;
    let optional_header_size = usize::from(read_u16(bytes, file_header_offset + 16)?);
    let optional_header_offset = file_header_offset.checked_add(20)?;
    let optional_magic = read_u16(bytes, optional_header_offset)?;
    let (format, image_base, subsystem, data_directory_offset) = match optional_magic {
        0x10b => (
            "pe32",
            u64::from(read_u32(bytes, optional_header_offset + 28)?),
            read_u16(bytes, optional_header_offset + 68)?,
            optional_header_offset + 96,
        ),
        0x20b => (
            "pe32_plus",
            read_u64(bytes, optional_header_offset + 24)?,
            read_u16(bytes, optional_header_offset + 88)?,
            optional_header_offset + 112,
        ),
        _ => return None,
    };
    let entry_point_rva = read_u32(bytes, optional_header_offset + 16)?;
    let certificate_table_bytes = read_u32(bytes, data_directory_offset + 4 * 8 + 4);
    let section_table_offset = optional_header_offset.checked_add(optional_header_size)?;
    let mut pe_sections = Vec::new();
    for index in 0..section_count {
        let offset = section_table_offset.checked_add(usize::from(index) * 40)?;
        let raw_name = bytes.get(offset..offset + 8)?;
        let name_end = raw_name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(raw_name.len());
        let name = String::from_utf8_lossy(&raw_name[..name_end]).to_string();
        let virtual_size = read_u32(bytes, offset + 8)?;
        let virtual_address = read_u32(bytes, offset + 12)?;
        let raw_size = read_u32(bytes, offset + 16)?;
        let raw_pointer = usize::try_from(read_u32(bytes, offset + 20)?).ok()?;
        let characteristics = read_u32(bytes, offset + 36)?;
        let raw_sha256 = section_sha256(bytes, raw_pointer, raw_size);
        pe_sections.push(ClaudeDesktopPeSectionAudit {
            name,
            virtual_address,
            virtual_size,
            raw_size,
            raw_sha256,
            characteristics: format!("0x{characteristics:08x}"),
        });
    }

    Some(PeAudit {
        pe_format: Some(format.to_string()),
        pe_machine: Some(machine_name(machine).to_string()),
        pe_subsystem: Some(subsystem_name(subsystem).to_string()),
        pe_timestamp_unix: Some(u64::from(timestamp)),
        pe_entry_point_rva: Some(entry_point_rva),
        pe_image_base: Some(image_base),
        pe_section_count: Some(section_count),
        pe_certificate_table_bytes: certificate_table_bytes,
        pe_sections,
    })
}

fn section_sha256(bytes: &[u8], offset: usize, raw_size: u32) -> Option<String> {
    let size = usize::try_from(raw_size).ok()?;
    if size == 0 {
        return None;
    }
    let end = offset.checked_add(size)?;
    let section = bytes.get(offset..end)?;
    let mut hasher = Sha256::new();
    hasher.update(section);
    Some(format!("{:x}", hasher.finalize()))
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn machine_name(machine: u16) -> &'static str {
    match machine {
        0x014c => "x86",
        0x8664 => "x64",
        0xaa64 => "arm64",
        _ => "unknown",
    }
}

fn subsystem_name(subsystem: u16) -> &'static str {
    match subsystem {
        2 => "windows_gui",
        3 => "windows_console",
        9 => "windows_ce_gui",
        10 => "efi_application",
        11 => "efi_boot_service_driver",
        12 => "efi_runtime_driver",
        14 => "xbox",
        16 => "windows_boot_application",
        _ => "unknown",
    }
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = std::io::Read::read(&mut file, &mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_kind_detects_msix_windowsapps_path() {
        assert_eq!(
            install_kind(&[
                "C:\\Program Files\\WindowsApps\\Claude_1.0\\app\\Claude.exe".to_string()
            ]),
            "msix"
        );
    }

    #[test]
    fn install_kind_detects_desktop_path() {
        assert_eq!(
            install_kind(&[
                "C:\\Users\\me\\AppData\\Local\\Programs\\Claude\\Claude.exe".to_string()
            ]),
            "desktop"
        );
    }

    #[test]
    fn cdp_blocker_documents_auth_gate_for_msix() {
        assert!(cdp_blocker_for_install_kind("msix").contains("CLAUDE_CDP_AUTH"));
    }

    #[test]
    fn integrity_for_missing_paths_is_not_checked() {
        let (status, _, audits) = integrity_for_paths(&[]);

        assert_eq!(status, "not_checked");
        assert!(audits.is_empty());
    }

    #[test]
    fn audit_executable_path_marks_third_party_patch_ineligible() {
        let audit =
            audit_executable_path("C:\\Program Files\\WindowsApps\\Claude_1.0\\app\\Claude.exe");

        assert_eq!(audit.install_kind, "msix");
        assert!(!audit.patch_eligible);
        assert_eq!(
            audit.mutation_policy,
            "blocked_no_executable_asar_signature_or_integrity_metadata_changes"
        );
        assert!(
            audit
                .notes
                .iter()
                .any(|note| note.contains("Read-only audit"))
        );
    }

    #[test]
    fn parse_pe_bytes_extracts_header_and_section_evidence() {
        let mut bytes = vec![0u8; 0x240];
        bytes[0] = b'M';
        bytes[1] = b'Z';
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(&0x0000_4550u32.to_le_bytes());
        bytes[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x88..0x8c].copy_from_slice(&1_700_000_000u32.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xf0u16.to_le_bytes());
        bytes[0x98..0x9a].copy_from_slice(&0x20bu16.to_le_bytes());
        bytes[0xa8..0xac].copy_from_slice(&0x1234u32.to_le_bytes());
        bytes[0xb0..0xb8].copy_from_slice(&0x1400_0000u64.to_le_bytes());
        bytes[0xf0..0xf2].copy_from_slice(&2u16.to_le_bytes());
        bytes[0x12c..0x130].copy_from_slice(&256u32.to_le_bytes());
        bytes[0x188..0x18d].copy_from_slice(b".text");
        bytes[0x190..0x194].copy_from_slice(&0x20u32.to_le_bytes());
        bytes[0x194..0x198].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[0x198..0x19c].copy_from_slice(&0x10u32.to_le_bytes());
        bytes[0x19c..0x1a0].copy_from_slice(&0x200u32.to_le_bytes());
        bytes[0x1ac..0x1b0].copy_from_slice(&0x6000_0020u32.to_le_bytes());
        bytes[0x200..0x210].copy_from_slice(b"0123456789abcdef");

        let audit = parse_pe_bytes(&bytes).expect("minimal PE should parse");

        assert_eq!(audit.pe_format.as_deref(), Some("pe32_plus"));
        assert_eq!(audit.pe_machine.as_deref(), Some("x64"));
        assert_eq!(audit.pe_subsystem.as_deref(), Some("windows_gui"));
        assert_eq!(audit.pe_section_count, Some(1));
        assert_eq!(audit.pe_certificate_table_bytes, Some(256));
        assert_eq!(audit.pe_sections[0].name, ".text");
        assert!(audit.pe_sections[0].raw_sha256.is_some());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires a running local Claude Desktop process"]
    fn live_claude_status_collects_executable_integrity_evidence() {
        let status = detect_status();

        if status.process_count == 0 {
            eprintln!("Claude Desktop is not running; live audit skipped.");
            return;
        }

        assert_eq!(status.supported_integration, "external_automation");
        assert_eq!(status.cdp_status, "blocked");
        assert!(!status.executable_audits.is_empty());
        let audit = &status.executable_audits[0];
        eprintln!(
            "Claude audit: path={}, risk={}, pe={:?}/{:?}, sections={:?}, signature={:?}",
            audit.path,
            audit.risk_level,
            audit.pe_format,
            audit.pe_machine,
            audit.pe_section_count,
            audit.signature_status
        );
        assert!(audit.exists);
        assert!(audit.sha256.as_deref().is_some_and(|hash| hash.len() == 64));
        assert!(
            audit
                .pe_format
                .as_deref()
                .is_some_and(|format| { format == "pe32" || format == "pe32_plus" })
        );
        assert!(audit.pe_section_count.is_some_and(|count| count > 0));
        assert!(!audit.pe_sections.is_empty());
        assert!(
            audit
                .signature_status
                .as_deref()
                .is_some_and(|status| status.eq_ignore_ascii_case("valid"))
        );
        assert!(audit.signer_not_after.is_some());
        assert_eq!(audit.risk_level, "controlled");
        assert_eq!(audit.patch_eligible, false);
        assert_eq!(
            audit.mutation_policy,
            "blocked_no_executable_asar_signature_or_integrity_metadata_changes"
        );
    }

    #[test]
    fn paste_draft_rejects_empty_text() {
        let result = paste_draft_to_claude("  ");

        assert_eq!(result.status, "failed");
        assert_eq!(result.input_chars, 0);
        assert!(!result.auto_submitted);
    }

    #[test]
    fn submit_rejects_empty_text() {
        let result = submit_text_to_claude("  ");

        assert_eq!(result.status, "failed");
        assert_eq!(result.action, "paste_and_submit");
        assert_eq!(result.input_chars, 0);
        assert!(!result.auto_submitted);
    }

    #[test]
    fn open_devtools_uses_devtools_action_when_not_running() {
        if !claude_process_ids().is_empty() {
            return;
        }
        let result = open_claude_devtools();

        if result.status == "failed" {
            assert_eq!(result.action, "open_devtools");
        }
    }

    #[test]
    fn enter_desktop_devtools_uses_expected_action_when_not_running() {
        if !claude_process_ids().is_empty() {
            return;
        }
        let result = enter_claude_desktop_devtools();

        if matches!(result.status.as_str(), "failed" | "accepted") {
            assert_eq!(result.action, "enter_desktop_devtools");
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires a running local Claude Desktop process"]
    fn live_open_claude_devtools_sends_shortcut() {
        let result = open_claude_devtools();

        if result.status == "failed" && result.message.contains("not running") {
            eprintln!("Claude Desktop is not running; live devtools skipped.");
            return;
        }

        eprintln!(
            "open devtools: status={}, action={}, foreground_verified={}, pid={:?}, title={:?}",
            result.status,
            result.action,
            result.foreground_verified,
            result.foreground_process_id,
            result.foreground_title
        );
        assert_eq!(result.action, "open_devtools");
        assert_eq!(result.status, "ok");
        assert!(result.foreground_verified);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires a running local Claude Desktop process"]
    fn live_enter_desktop_devtools_sends_shortcut() {
        let result = enter_claude_desktop_devtools();

        if result.status == "failed" && result.message.contains("not running") {
            eprintln!("Claude Desktop is not running; live desktop devtools skipped.");
            return;
        }

        eprintln!(
            "enter desktop devtools: status={}, action={}, foreground_verified={}, pid={:?}, title={:?}",
            result.status,
            result.action,
            result.foreground_verified,
            result.foreground_process_id,
            result.foreground_title
        );
        assert_eq!(result.action, "enter_desktop_devtools");
        assert_eq!(result.status, "ok");
        assert!(result.foreground_verified);
    }

    #[test]
    fn open_response_uses_open_action_when_launch_fails_without_process() {
        if !claude_process_ids().is_empty() {
            return;
        }
        let result = open_claude_desktop();

        if result.status == "failed" {
            assert_eq!(result.action, "open");
        }
    }

    #[test]
    fn new_chat_uses_new_chat_action_when_not_running() {
        if !claude_process_ids().is_empty() {
            return;
        }
        let result = new_claude_chat();

        if result.status == "failed" {
            assert_eq!(result.action, "new_chat");
        }
    }
}
