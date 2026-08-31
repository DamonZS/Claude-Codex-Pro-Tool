# Client enhancement Claude detection

## Background

The client enhancement screen previously filtered its client list with the global Agent scope. Selecting Codex therefore hid Claude Desktop and Claude Code entirely. The lightweight Claude status also used running-process paths as installation evidence, so an installed but stopped Claude Desktop was reported as not installed.

## Goals

- Always list Codex App, Claude Desktop, and Claude Code on the client enhancement screen.
- Use the global Agent scope only to choose the initial client selection.
- Discover the installed Claude Desktop executable when no Claude process is running.
- Keep installation state and running state independent.
- Make persisted enhancement capabilities actionable from the same screen: users can enable or disable each setting-backed capability without editing Settings separately.
- Expose the Codex "我的任务" workspace as a setting-backed capability using `multicaWorkspaceEnabled`, with the existing default-on behavior preserved for older settings files.

## Non-goals

- Do not launch Claude during detection.
- Do not treat a Claude configuration directory as installation evidence.
- Do not change client actions, IPC command names, or proxy configuration.
- Do not add toggles to capabilities whose value is detection-only (for example process-derived watcher/session status); those rows remain read-only status indicators.

## Technical requirements

- Reuse the existing Windows desktop and MSIX discovery paths and macOS bundle discovery.
- Preserve runtime process paths when Claude is running.
- Keep unsupported platforms returning no installed executable.
- The page must retain its existing master-detail interactions and actions.
- Settings-backed rows must render an accessible enable/disable control and persist boolean changes through `actions.saveSettingBoolean` without sending the complete settings object.
- While a settings write is pending, controls must be disabled so concurrent writes cannot overwrite one another.
- The "我的任务" row must read and write only `multicaWorkspaceEnabled`; it must not rewrite supplier, proxy, or Codex base URL settings.
