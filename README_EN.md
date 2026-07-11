# Claude Codex Pro Tool

<p align="center">
  <img src="assets/images/claude-codex-pro.png" alt="Claude Codex Pro Tool icon" width="160">
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

Turn Codex App and Claude Desktop into an operable, maintainable, and extensible local AI workstation.

Claude Codex Pro Tool is more than a launcher. It brings Codex enhancements, local Claude Desktop integration, provider and relay switching, Plugin Hub, Ponytail, multi-tool Skills, session repair, Pangu Memory, the script market, prompt optimization, Zed Remote, automatic updates, and installation maintenance into one Tauri operations console. Think of it as a local control room for Codex and Claude Desktop: capabilities that would otherwise be scattered across configuration files, command-line tools, plugin directories, and script catalogs are managed in one place.

Canonical repository:

<https://github.com/DamonZS/Claude-Codex-Pro-Tool>

> **Highlighted capability: fix GPT-5.6 selection for third-party Codex APIs through injection.** When a third-party API or relay already provides `gpt-5.6` / `gpt-5.6-*`, but Codex Desktop still hides it behind a frontend model whitelist, CCP reads the active provider and local model catalog, then uses injection to synchronize Codex model configuration, model-request paths, and the model picker. This makes GPT-5.6 visible and selectable in the UI. The fix removes a Codex client-side selection restriction; successful inference still depends on whether the upstream API supports the selected model ID.

## Who This Is For

- Users who want a more complete local enhancement layer for Codex App.
- Users who switch between multiple API relays or OpenAI-compatible providers.
- Users who want Claude Desktop, Claude Code, Codex, MCP, Skills, and plugins in one management interface.
- Users who frequently repair sessions, export history, move projects, or manage scripts and plugins.
- Users who expect features to be observable and verifiable instead of decorative buttons without working behavior.

## Core Principles

- Local first: configuration, memory, plugin records, logs, and backups are stored locally whenever possible.
- Reviewable: commands or diffs are shown before installing plugins, writing MCP configuration, trusting hooks, or changing important settings.
- Recoverable: critical configuration is backed up when practical, and the Claude Chinese resource patch includes a restore path.
- No silent trust: third-party Ponytail or Codex hooks require separate review and explicit trust.
- No simulated capability: actions that cannot be automated or still require user confirmation are labeled clearly.

## Download

Download the latest release from [GitHub Releases](https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases):

- Windows: `claude-codex-pro-*-windows-x64-setup.exe`
- macOS Intel: `claude-codex-pro-*-macos-x64.dmg`
- macOS Apple Silicon: `claude-codex-pro-*-macos-arm64.dmg`

The installer provides two entry points:

- `Claude Codex Pro`: a silent launcher that starts Codex and loads the enhancement layer.
- `Claude Codex Pro Manager`: the operations console for Codex, Claude, providers, plugins, scripts, memory, logs, installation maintenance, and updates.

The Windows installer creates Desktop and Start Menu shortcuts. The macOS DMG contains `Claude Codex Pro.app` and `Claude Codex Pro 管理工具.app`.

## Feature Overview

### 1. Codex Launch and Enhancements

- Launch Codex through an external launcher.
- Manage CDP and local helper connections automatically.
- Inject a top status badge into Codex pages.
- Unlock Codex plugin and plugin-marketplace entry points.
- Adapt Codex plugin installation channels, including newer requests such as `vscode://codex/list-plugins` and `vscode://codex/plugin/install`.
- Unlock the frontend model whitelist so third-party APIs that already support GPT-5.6 can expose it in the Codex model picker.
- Provide service-tier controls.
- Support image overlay configuration.
- Restore session scroll positions.
- Enhance session timelines and conversation views.
- Adjust native menu placement.
- Write Codex Goals configuration.
- Provide Computer Use Guard to reduce accidental high-risk automation.

### 2. Codex Session Management and Repair

- List local Codex sessions.
- Delete local sessions.
- Export sessions as Markdown.
- Move sessions between project assignments.
- Show the active session database location.
- Detect both newer `~/.codex/sqlite/*.db` databases and the older `~/.codex/state_5.sqlite` database.
- Use Provider Sync to restore historical-session visibility after switching providers.
- Backfill provider settings from the active configuration so switching does not overwrite an older profile unexpectedly.

### 3. Providers, Relays, and Model Routing

- Support official mode, official mode with a mixed API key, and pure API mode.
- Support OpenAI Responses and Chat Completions protocols.
- Manage separate provider profiles for Codex, Claude, and Claude Desktop.
- Keep Codex routing independent while Claude and Claude Desktop use the Claude routing group.
- Switch target applications, drag provider cards to reorder them, and import compatible cc-switch provider data including API keys.
- Configure Base URL, API Key, model, User-Agent, context window, and automatic compaction threshold.
- Fetch models from the provider, add or delete model rows, and preserve menu names, upstream model IDs, context windows, and ordering.
- Configure Claude role mappings for Sonnet, Opus, Fable, Haiku, and Subagent, including local 1M capability declarations.
- Choose a custom User-Agent from cc-switch-compatible presets or enter one manually.
- Split common configuration from context-specific configuration.
- Select MCP servers, Skills, and Plugins for the active context.
- Backfill settings from the current `~/.codex/config.toml` and `auth.json`.
- Test provider connectivity.
- Clear API mode and return to official-login configuration.
- Configure self-hosted or third-party compatible API relays.

Example configuration written to `~/.codex/config.toml`:

```toml
model_provider = "custom"

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://example.com/v1"
experimental_bearer_token = "sk-..."
```

### 4. Claude Desktop Management

- Launch the official Claude Desktop application.
- Focus the Claude Desktop window.
- Open Claude Desktop DevTools.
- Create a new Claude Desktop conversation.
- Paste a draft into Claude Desktop.
- Submit text to Claude Desktop.
- Detect the installation path, process state, and integrity status.
- Report official MSIX and CDP limitations instead of pretending that injection succeeded.

### 5. Chinese Support for Claude

The project provides two localization paths for different risk preferences:

- Claude Chinese wrapper window: an independent WebView loads `https://claude.ai/new` and injects Chinese text coverage plus a top status badge during window creation. This is the recommended path because it does not modify official installation files.
- Claude Desktop Chinese resource patch: an optional local resource patch based on public resources from `Jyy1529/claude-desktop_win-zh_cn`. It writes `zh-CN.json`, locale configuration, and the required frontend language support. The manager creates a backup before applying the patch and provides a restore action.

The official Claude Desktop MSIX package, signatures, and integrity checks can restrict direct DOM injection or file patching. The wrapper window is the lower-risk option. The resource patch is a local modification that runs only after explicit user selection.

### 6. Plugin Hub

Plugin Hub combines multiple sources into one catalog:

- The official Claude plugin marketplace.
- Claude Desktop MCP configuration entries.
- GitHub MCP Registry.
- Awesome Claude Code resources.
- The OpenAI Codex Plugins repository.
- Ponytail multi-tool plugins.
- Recognizable Skill bundles.
- Community resource links.

Each item can show:

- Source, category, author, and license.
- Installation status.
- Risk notes.
- Dependency requirements.
- Installation-command preview.
- Configuration diff.
- Install, uninstall, and source actions.

Installation policy:

- Official Claude plugins use `claude plugin marketplace add/install`.
- Claude Desktop MCP entries are written to `claude_desktop_config.json` after a backup.
- Codex plugins use `codex plugin marketplace add/list/add`.
- Skill bundles are installed only when their structure can be recognized.
- Unknown community MCP entries are displayed by default without automatically executing scripts.

### 7. Ponytail Integration

[DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail) is integrated for multiple tools:

- Ponytail for Codex: call Codex CLI to add the marketplace and install `ponytail@ponytail`.
- Ponytail Codex hooks: preview hooks that require trust and write trust state only after confirmation.
- Ponytail Skills for Codex: copy Ponytail Skills into the Codex skills directory, creating a backup before replacement.
- Ponytail for Claude Code: install through Claude Code CLI.
- Ponytail MCP for Claude Desktop: write the Claude Desktop MCP configuration.
- Ponytail Organization Plugin for Claude Desktop: write an organization-plugin directory readable by Claude Desktop developer mode.
- Ponytail MCPB: build a `.mcpb` package and hand it to the official Claude Desktop confirmation flow.
- Ponytail for GitHub Copilot CLI: install through the Copilot CLI plugin system.

The local Claude Desktop plugin-bundle flow does not require signing in through Claude CLI. It configures developer mode, writes Codex/Ponytail MCP entries, and copies Ponytail skills into the organization-plugin directory.

### 8. OpenAI Codex Plugin Repository

- Download the official `openai/plugins` repository ZIP.
- Enforce a download-size limit.
- Extract safely and prevent ZIP path traversal.
- Validate `.agents/plugins/marketplace.json`.
- Validate `.codex-plugin/plugin.json` in every plugin directory.
- Register `[marketplaces.openai-curated]` in `~/.codex/config.toml`.
- Report the exact failure instead of registering a damaged repository as successful.

### 9. Pangu Memory

Pangu Memory uses SQLite and does not require cloud embeddings or an external vector database.

Capabilities include:

- Manually writing long-term memories.
- Automatically producing memories that require confirmation.
- Moving a memory into long-term storage only after confirmation.
- Workspace isolation.
- Global memories under `global`.
- Combined current-workspace and global queries.
- Keyword normalization and lightweight similarity ranking.
- Updating access count and last-access time after a hit.
- Redacting secrets so API keys, Bearer tokens, and `sk-` values are not stored as plaintext.
- Self-check and repair.
- JSON export.
- JSON import with merge or replace modes.

Default database:

```text
~/.claude-codex-pro/memory_assist.sqlite
```

### 10. Script Market and User Scripts

- Refresh the script market.
- Download and install scripts.
- Manage local user scripts.
- Enable or disable individual scripts.
- Delete user scripts.
- Build a bundle from enabled scripts.
- Extend the frontend through the Codex injection script.

### 11. Prompt Optimizer

The project integrates the prompt-optimization workflow from [linshenkx/prompt-optimizer](https://github.com/linshenkx/prompt-optimizer).

- Open the prompt optimizer inside the manager.
- Open an independent prompt-optimizer window.
- Use it as part of the unified tool system without spawning duplicate control windows.

### 12. Zed Remote

- Detect the Zed installation path.
- Parse SSH host, user, and port.
- Resolve remote projects from Codex global state and thread context.
- Maintain a recent-remote-project registry.
- Build `zed://ssh/...` remote-open links.
- Support default open, window reuse, new window, and append-to-current-window strategies.
- Forget remote projects.

### 13. Upstream Worktree

- Read Git remotes, branches, and worktrees.
- Create a worktree from the latest remote-tracking branch.
- Support local and remote projects.
- Validate branch names and base branches.
- Avoid deriving task branches from a stale local HEAD.

### 14. Watcher and Self-Recovery

- Install the watcher on Windows.
- Monitor the Codex process and CDP port.
- Recover a failed launcher.
- Enable, disable, install, or uninstall the watcher.
- Stop launcher or Codex-related processes when requested.

### 15. Installation Maintenance and Updates

- Installation and uninstallation entry points.
- Shortcut repair.
- Backend-configuration repair.
- Update checks.
- GitHub Release asset downloads.
- Installer launch.
- Latest-log reading.
- Diagnostic information copy.
- Settings reset.
- Image-overlay settings reset.

### 16. Automated Builds and Releases

- `Auto release installers`: after a push to `main` or a manual trigger, calculate the next `V0.01`-series version, create the tag, build the Windows installer, macOS x64 DMG, and macOS arm64 DMG, then upload `latest.json`.
- `PR build artifacts`: build verification artifacts for pull requests and routine validation.
- `release-assets`: retained for manually triggered GitHub Releases.

Automatic version progression:

```text
V0.01 -> V0.02 -> ... -> V0.99 -> V1.00
```

## Manager Pages

- Overview: runtime status, Codex/Claude quick actions, log summary, and the official relay entry point.
- Providers: Codex, Claude, and Claude Desktop provider profiles, routing, model catalogs, and mappings.
- Tools and Plugins: Plugin Hub, Ponytail, the Codex plugin repository, and local Claude Desktop plugins.
- Session Management: history repair plus Codex and Claude session management.
- Pangu Memory: memory status, review, maintenance, and related controls.
- Maintenance: diagnostics, repair, watcher, installation, and update actions.
- Settings: real runtime switches, launch arguments, enhancement matrix, memory, Zed, and watcher settings.
- About: project, version, repository, and contact information.

## Safety Boundaries

- Do not silently modify the private Claude Desktop plugin store.
- Do not silently trust third-party hooks.
- Do not automatically execute unknown community MCP installation scripts.
- Do not write API keys, Bearer tokens, or complete authorization configuration into normal logs.
- Do not treat third-party GitHub content as trusted executable code by default.
- The Claude Chinese wrapper window does not modify official Claude Desktop files.
- The Claude Desktop Chinese resource patch is an explicitly triggered local patch with backup and restore support.

## Data Locations

- Codex configuration: `~/.codex/config.toml`
- Codex login state: `~/.codex/auth.json`
- Codex database: prefer `~/.codex/sqlite/*.db`, with fallback to the older `~/.codex/state_5.sqlite`
- Codex plugin repository cache: `~/.codex/.tmp/plugins`
- Codex skills: `~/.codex/skills`
- Claude Desktop MCP configuration on Windows: usually `%APPDATA%\Claude\claude_desktop_config.json`
- Claude Desktop 3P configuration on Windows: usually `%LOCALAPPDATA%\Claude-3p`
- Claude Codex Pro state: `~/.claude-codex-pro/`
- Pangu Memory database: `~/.claude-codex-pro/memory_assist.sqlite`
- Provider Sync backups: `~/.codex/backups_state/provider-sync`

## FAQ

### Why does one commit trigger two GitHub Actions runs?

Two workflows currently listen for pushes to `main`:

- `Auto release installers`: builds and publishes installers.
- `PR build artifacts`: runs routine build validation.

This does not publish two versions. The same commit simply triggers two pipelines.

### Why does a Release contain only source-code archives?

If the installer build jobs succeed but the publishing job fails, GitHub shows only its automatically generated source ZIP and tarball. Inspect the `Publish release and latest.json` step in `Auto release installers`. The current workflow publishes a draft release first and then generates and uploads `latest.json`, avoiding failures caused by querying a draft release by tag too early.

### The Codex enhancement badge does not appear

Launch Codex through the `Claude Codex Pro` entry point instead of starting the original Codex executable directly. If the badge still does not appear, open Diagnostics and Logs in the manager and check the helper port, CDP connection, and `renderer.script_loaded` records.

### My third-party API supports GPT-5.6, but Codex still cannot select it

Some Codex versions continue to use an official frontend model whitelist. Even when the active third-party API returns GPT-5.6, the model picker can still show only older models. CCP's model-whitelist unlock patches Codex model-catalog responses, model-request paths, and the model picker through injection, then adds saved `gpt-5.6` / `gpt-5.6-*` models from the active provider.

Use it as follows:

1. On the Providers page, fetch models or add the exact GPT-5.6 model ID supported by the upstream service, then select `Save and Use`.
2. In Settings, confirm that the model-whitelist unlock is enabled. It is enabled by default.
3. Start or restart Codex through `Claude Codex Pro`. If Codex is already running, use `Repair Frontend Connection` in the manager and wait for the injection badge to return before opening the model picker.
4. Reopen the model picker and select the GPT-5.6 model.

The catalog also supports `model_catalog_json` in Codex `config.toml`, plus `~/.codex/model-catalog.gpt-5.6.json` and `~/.codex/model-catalog.json`. Injection only removes the client-side “not visible / not selectable” restriction. It cannot give GPT-5.6 to an API that does not support it. If the request fails after selection, verify the provider model ID, protocol, and upstream availability.

### Claude is not shown in Chinese

Use `Open Claude Chinese Window` first. It is an independent WebView wrapper, not the original official Claude Desktop window. If you use the resource patch, make sure Claude Desktop has fully exited and the installation directory is writable. On failure, inspect patch status or run the restore action.

### Plugin installation failed

Open the installation preview and identify the installation type:

- Official Claude plugins require the `claude` CLI.
- Claude Desktop MCP requires writing `claude_desktop_config.json`.
- Local Claude Desktop organization plugins require developer mode and directory write permission.
- Codex plugins require the `codex` CLI.
- Ponytail hooks require separate review and trust.
- Community MCP and Skill entries require a recognizable structure.

### macOS says the app cannot be opened or is damaged

Unsigned or unnotarized builds can be blocked by Gatekeeper. Allow the application under System Settings -> Privacy & Security. If macOS still reports that the app is damaged, run:

```bash
sudo xattr -rd com.apple.quarantine /Applications/Claude\ Codex\ Pro.app
sudo xattr -rd com.apple.quarantine /Applications/Claude\ Codex\ Pro\ 管理工具.app
```

### Does it support Intel Macs?

Yes. Releases provide both `macos-x64.dmg` and `macos-arm64.dmg`. Intel Macs use the x64 package; Apple Silicon Macs use the arm64 package.

## Build and Development

This project is a Rust workspace with a Tauri manager and a Vite/React frontend. The repository-root `package.json` comes from an upstream structure and is not used to build this project. Install and build the actual frontend from `apps/claude-codex-pro-manager`.

### Requirements

- Git.
- Node.js 22 or newer.
- npm.
- A stable Rust toolchain including `cargo`, `rustc`, and `rustfmt`.
- Visual Studio Build Tools / the MSVC C++ toolchain for Windows builds.
- NSIS for Windows installer packaging.
- Xcode Command Line Tools for macOS builds.
- The macOS DMG script uses system tools including `sips`, `iconutil`, `codesign`, and `hdiutil`.

Install NSIS on Windows:

```powershell
choco install nsis -y
```

Install Rust targets for macOS:

```bash
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

### Install Dependencies

```bash
cd apps/claude-codex-pro-manager
npm install --package-lock=false
cd ../..
```

To use the lockfile strictly, replace the first command with `npm ci`. CI currently uses `npm install --package-lock=false`.

### Start Local Development

```bash
cd apps/claude-codex-pro-manager
npm run dev
```

Tauri CLI starts the manager and launches the Vite development server automatically. Vite listens on:

```text
http://localhost:1420
```

For frontend-only development:

```bash
cd apps/claude-codex-pro-manager
npm run vite:dev
```

A regular browser preview has no Tauri backend. Buttons that depend on system configuration, processes, plugin installation, or Claude localization will return preview behavior or cannot execute. Use `npm run dev` to validate real application behavior.

### Local Verification

Recommended checks before committing:

```bash
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo test --workspace
cargo build --release
```

Common targeted Rust checks:

```bash
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml plugin_hub -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml memory_assist -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml relay_config -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
```

### Production Binaries

```bash
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build --release
```

Primary outputs:

```text
target/release/claude-codex-pro.exe
target/release/claude-codex-pro-manager.exe
```

On macOS or Linux, the files do not have an `.exe` suffix:

```text
target/release/claude-codex-pro
target/release/claude-codex-pro-manager
```

You can also build from the manager directory:

```bash
cd apps/claude-codex-pro-manager
npm run build
```

This script builds the silent launcher first and then runs `tauri build`. Official installers still use the repository's NSIS and DMG packaging scripts.

### Windows Installer

```powershell
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test --workspace
cargo build --release

New-Item -ItemType Directory -Force dist/windows/app | Out-Null
Copy-Item target/release/claude-codex-pro.exe dist/windows/app/
Copy-Item target/release/claude-codex-pro-manager.exe dist/windows/app/

$version = "0.12"
$makensis = "${env:ProgramFiles(x86)}\NSIS\makensis.exe"
if (-not (Test-Path $makensis)) { $makensis = "makensis" }
Push-Location scripts/installer/windows
& $makensis "/INPUTCHARSET" "UTF8" "/DVERSION=$version" ClaudeCodexPro.nsi
Pop-Location
```

Output:

```text
dist/windows/claude-codex-pro-0.12-windows-x64-setup.exe
```

### macOS DMG

Apple Silicon:

```bash
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
npm --prefix apps/claude-codex-pro-manager run vite:build
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
BINARY_DIR="$PWD/target/aarch64-apple-darwin/release" bash scripts/installer/macos/package-dmg.sh 0.12 arm64
```

Intel Mac:

```bash
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
npm --prefix apps/claude-codex-pro-manager run vite:build
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
BINARY_DIR="$PWD/target/x86_64-apple-darwin/release" bash scripts/installer/macos/package-dmg.sh 0.12 x64
```

Outputs:

```text
dist/macos/claude-codex-pro-0.12-macos-arm64.dmg
dist/macos/claude-codex-pro-0.12-macos-x64.dmg
```

The local script uses ad-hoc code signing and does not apply Apple Developer ID signing or notarization. Gatekeeper can therefore warn about locally built DMGs; use the macOS instructions in the FAQ above.

## GitHub Actions

Primary workflows:

- `.github/workflows/auto-release-installers.yml`: automatically releases after a push to `main` or a manual trigger.
- `.github/workflows/pr-build.yml`: builds validation artifacts for pull requests, pushes to `main`, and manual runs.
- `.github/workflows/release-assets.yml`: retained for manually managed GitHub Releases.

Automatic release flow:

1. Push to `main` or run `Auto release installers` manually.
2. `scripts/release/next-release-tag.js` reads existing tags.
3. Generate the next `V0.01`-series tag.
4. Create the tag and a draft Release.
5. Build the Windows `.exe` installer.
6. Build the macOS Intel x64 DMG.
7. Build the macOS Apple Silicon arm64 DMG.
8. Upload installers.
9. Publish the Release.
10. Generate and upload `latest.json`.

Example automatic-release assets:

```text
claude-codex-pro-0.01-windows-x64-setup.exe
claude-codex-pro-0.01-macos-x64.dmg
claude-codex-pro-0.01-macos-arm64.dmg
latest.json
```

## Project Structure

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
  macos/package-dmg.sh                macOS DMG packaging script
docs/
  code-knowledge-graph.md             Code knowledge graph
  full-code-review.md                 Full code-review record
```

## Feedback

- Issues: <https://github.com/DamonZS/Claude-Codex-Pro-Tool/issues>
- Discussion-group QR code: <https://kcnl7iasnc4t.feishu.cn/wiki/O4T8wAodLiz05MkpqVkcoI7SnRd?from=from_copylink>

## License and Repository Rules

This repository uses a custom source-available restrictive license and is not licensed under an OSI-approved open-source license. Without written permission from DamonZS or an authorized maintainer, modifying, publishing, distributing, renaming, repackaging, or hiding the origin of this project is prohibited. This restriction covers manual edits, AI-assisted edits, scripts, codemods, bulk replacement, automated rewrites, binary patches, and metadata changes.

Author information, repository URLs, copyright notices, product names, branding, publisher identity, sponsorship or payment identity, license files, and rule files must not be removed, replaced, hidden, or weakened.

These restrictions do not apply to DamonZS, the repository owner, authorized maintainers, or AI assistants, scripts, CI, codemods, formatters, and automation working under their direction. Official project development may continue to use AI and automation.

See [MAINTAINERS.md](MAINTAINERS.md) for the authorized maintainer list. See [LICENSE](LICENSE) and [RULES.md](RULES.md) for the complete terms.

## Disclaimer

Claude Codex Pro Tool is an external enhancement tool. It is not an official project of OpenAI, Anthropic, Claude, or Codex. When official applications change page structure, protocols, CLI behavior, plugin formats, or configuration paths, this project's injection scripts and adapters may need corresponding updates.
