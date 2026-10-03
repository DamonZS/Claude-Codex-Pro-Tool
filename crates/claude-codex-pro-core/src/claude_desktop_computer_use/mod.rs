//! Claude Desktop Computer Use MCP server.
//!
//! The CCP executable runs this as a stdio MCP server (`--mcp-computer-use`).
//! Claude Desktop launches it from `mcpServers`, sends newline-delimited
//! JSON-RPC 2.0 messages, and receives screenshots as MCP image content.

mod platform;
mod register;
mod server;
mod tools;
pub mod uia;

pub use register::{
    COMPUTER_USE_MCP_ARG, COMPUTER_USE_SERVER_NAME, ComputerUseStatus, computer_use_status,
    computer_use_status_for_paths, register_computer_use_at, set_computer_use_enabled,
    unregister_computer_use_at,
};
pub use server::{ComputerUseServer, Gate, SettingsGate};
pub use tools::{Backend, MouseButton, Screen, ScreenSize, normalize_key, screenshot_size};

use std::io::{BufRead, Write};

/// Entry point for `claude-codex-pro --mcp-computer-use`.
pub fn run_stdio_server() -> anyhow::Result<()> {
    platform::prepare_process();
    let gate = SettingsGate::new(crate::settings::SettingsStore::new(
        crate::paths::default_settings_path(),
    ));
    let mut server = ComputerUseServer::new(platform::NativeBackend::default(), gate);
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = server.handle_line(&line) {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
