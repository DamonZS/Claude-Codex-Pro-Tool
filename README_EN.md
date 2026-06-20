# Claude Codex Pro Tool

<p align="center">
  <img src="docs/images/claude-codex-pro.png" alt="Claude Codex Pro Tool icon" width="160">
</p>

<p align="center">
  <a href="README.md">中文</a> | English
</p>

<p align="center">
  <img alt="Release" src="https://img.shields.io/github/v/release/DamonZS/Claude-Codex-Pro-Tool">
  <img alt="Stars" src="https://img.shields.io/github/stars/DamonZS/Claude-Codex-Pro-Tool">
  <img alt="License" src="https://img.shields.io/github/license/DamonZS/Claude-Codex-Pro-Tool">
  <img alt="Rust" src="https://img.shields.io/badge/rust-1.85%2B-orange">
  <img alt="Tauri" src="https://img.shields.io/badge/tauri-2.x-24C8DB">
</p>

Claude Codex Pro Tool is a local operations console for Codex App and Claude Desktop. It provides Codex launch enhancements, a Chinese Claude wrapper window, Claude Desktop MCP installation, Plugin Hub, provider configuration, session maintenance, script management, prompt optimization, diagnostics, and updates.

The safety boundary is intentional: the tool does not modify official Codex or Claude installation directories, does not patch `app.asar`, and does not change signatures or integrity files. Enhancements are applied through an external launcher, local user configuration, a WebView wrapper window, or reviewable config writes.

## Download and Entry Points

Download the latest package from [GitHub Releases](https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases):

- Windows: `claude-codex-pro-*-windows-x64-setup.exe`
- macOS Intel: `claude-codex-pro-*-macos-x64.dmg`
- macOS Apple Silicon: `claude-codex-pro-*-macos-arm64.dmg`

After installation, two entry points are available:

- `Claude Codex Pro`: a silent launcher that starts Codex and loads this tool's enhancement layer.
- `Claude Codex Pro Manager`: a Tauri operations console for Codex, Claude, providers, plugins, scripts, logs, installation maintenance, and updates.

The Windows installer creates desktop and Start Menu shortcuts. The macOS DMG installs `Claude Codex Pro.app` and `Claude Codex Pro Manager.app`.

## Core Features

- Codex enhancements: inject status badges, quick actions, session tools, user scripts, and enhancement entry points through CDP and a local helper.
- Claude Chinese window: open `https://claude.ai/new` in an independent WebView and inject Chinese text coverage plus a top status badge when the window is created.
- Claude Desktop integration: launch the official Claude Desktop app and write MCP configuration to the Claude Desktop user config file.
- Plugin Hub: browse official Claude plugins, GitHub MCP resources, Claude Code resources, Skills, and community resources; preview commands or config diffs before installation.
- Provider configuration: manage compatible APIs, relay profiles, models, context selection, and Codex provider writes.
- Session maintenance: delete, restore, export Markdown, move projects, inspect timelines, and diagnose local data.
- Script market: manage built-in scripts, local user scripts, and remote script catalogs.
- Prompt optimization: integrate the `linshenkx/prompt-optimizer` workflow.
- Provider Sync: keep historical sessions visible after switching providers.
- Zed Remote: detect SSH contexts and open matching remote files from Codex in Zed.
- Upstream worktree: create worktrees from fresh remote tracking branches instead of stale local HEAD state.
- Maintenance: logs, diagnostics, repairs, version checks, and GitHub Release updates.

## Claude Desktop

The manager keeps Claude and Codex actions separate:

- `Launch Claude`: launches the official Claude Desktop app without modifying its installation files.
- `Open Claude Chinese Window`: opens the independent WebView wrapper, loads Claude Web, and applies Chinese coverage.
- `Restart Codex`: controls only the Codex enhancement launcher.
- `Plugin Hub`: routes inside the manager instead of opening duplicate control windows.

The Claude Chinese window is not DOM injection into the official Claude Desktop window. The official desktop app's MSIX package, signatures, and integrity checks block that high-risk path, so this project uses a safe wrapper window. Users log in to Claude Web inside the wrapper, and the Chinese coverage script only affects that wrapper window.

## Plugin Hub

Plugin Hub presents multiple resource types in one place:

- Official Claude plugin marketplace entries.
- GitHub MCP and community MCP resources.
- Awesome Claude Code resources.
- Skill bundles with recognizable structure.
- Claude Desktop MCP entries, including Codex-related MCP.

The install flow is review-first:

1. Refresh the catalog and inspect source, type, license, risk notes, and install status.
2. Preview the command or config diff.
3. Confirm installation; config writes create backups when possible.
4. Restart Claude Desktop when the installed MCP must be loaded by the desktop app.

Official Claude plugins usually require the local `claude` CLI. Community MCP and Skill entries only fetch metadata by default and expose install actions only when the structure and install method are recognized.

## Codex Providers and Relay

Provider configuration is for compatible APIs or relay services. The manager writes Codex provider configuration similar to:

```toml
model_provider = "custom"

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://example.com/v1"
experimental_bearer_token = "sk-..."
```

Recommended workflow:

1. Confirm that the Base URL is reachable and supports the selected protocol.
2. Test the key with a minimal request.
3. Never put real keys in logs, screenshots, or issues.
4. Confirm that `~/.codex/config.toml` is backed up before writing.
5. Use the manager to clear API mode when returning to official login mode.

## Safety

- No modification of official Codex App, Claude Desktop, MSIX packages, `app.asar`, signatures, or integrity files.
- API keys, Bearer tokens, and full auth configs are not written to normal logs.
- Third-party GitHub content is metadata-only by default and does not run scripts automatically.
- Plugins, MCP entries, and Skills show commands or config diffs before installation.
- Older entry points, shortcuts, and data locations are migrated or cleaned by installer and maintenance flows; public entry points use the current names.

## Data Locations

- Codex config: `~/.codex/config.toml`
- Codex auth state: `~/.codex/auth.json`
- Codex local database: prefers `~/.codex/sqlite/*.db`, falls back to older `~/.codex/state_5.sqlite`
- Claude Desktop MCP config: on Windows, usually `%APPDATA%\Claude\claude_desktop_config.json`
- Claude Codex Pro state and logs: `~/.claude-codex-pro/`
- Provider Sync backups: `~/.codex/backups_state/provider-sync`

## FAQ

### The Codex enhancement badge does not appear

Make sure Codex was launched through `Claude Codex Pro`, not the original Codex entry. If it still does not appear, open Diagnostics and Logs in the manager and check the helper port, CDP connection, and `renderer.script_loaded` records.

### Claude is not shown in Chinese

Chinese coverage targets the independent WebView created by `Open Claude Chinese Window`. The official Claude Desktop window is not forcibly modified. If the wrapper window also does not show Chinese coverage, inspect the Claude Chinese window status and injection script errors in the manager logs.

### Plugin installation failed

Open the install preview first and confirm the install type:

- Official Claude plugins require the local `claude` CLI.
- Claude Desktop MCP requires a writable `claude_desktop_config.json`.
- Community MCP and Skill entries require recognizable structure.
- Claude Desktop must be restarted after installing MCP entries that should be loaded by the desktop app.

### macOS says the app cannot be opened or is damaged

Unsigned or unnotarized builds may be blocked by Gatekeeper. Allow the app in System Settings -> Privacy & Security. If macOS still reports that the app is damaged, run:

```bash
sudo xattr -rd com.apple.quarantine /Applications/Claude\ Codex\ Pro.app
sudo xattr -rd com.apple.quarantine /Applications/Claude\ Codex\ Pro\ Manager.app
```

### Does it support Intel Macs?

Yes. Releases provide both `macos-x64.dmg` and `macos-arm64.dmg`. Intel Macs should use the x64 package, while Apple Silicon Macs should use the arm64 package.

## Development

```bash
# Frontend checks
cd apps/claude-codex-pro-manager
npm install
npm run check
npm run vite:build

# Rust checks
cd ../..
cargo fmt --check
cargo test
cargo build --release
```

Project structure:

```text
apps/
  claude-codex-pro-launcher/          Silent launcher
  claude-codex-pro-manager/           Tauri manager
assets/inject/
  renderer-inject.js                  Codex enhancement script
  claude-chinese-inject.js            Claude Chinese wrapper script
crates/
  claude-codex-pro-core/              Launch, injection, config, plugins, updates, install, bridge
  claude-codex-pro-data/              Session data, export, Provider Sync
scripts/installer/
  windows/ClaudeCodexPro.nsi          Windows NSIS installer
  macos/package-dmg.sh                macOS DMG packager
```

## Feedback

- Issues: <https://github.com/DamonZS/Claude-Codex-Pro-Tool/issues>
- Discussion group QR code: <https://docs.qq.com/doc/DQ2VOanZTTFZJcUpZ#>

## Notes

Claude Codex Pro Tool is an external enhancement tool. It is not an official OpenAI, Anthropic, Claude, or Codex project. If official apps change page structure, protocols, or config formats, this project's injection scripts and adapters may need updates.
