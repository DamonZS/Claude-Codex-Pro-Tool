# Client enhancement Claude detection

## Background

The client enhancement screen previously filtered its client list with the global Agent scope. Selecting Codex therefore hid Claude Desktop and Claude Code entirely. The lightweight Claude status also used running-process paths as installation evidence, so an installed but stopped Claude Desktop was reported as not installed.

## Goals

- Always list Codex App, Claude Desktop, and Claude Code on the client enhancement screen.
- Use the global Agent scope only to choose the initial client selection.
- Discover the installed Claude Desktop executable when no Claude process is running.
- Keep installation state and running state independent.

## Non-goals

- Do not launch Claude during detection.
- Do not treat a Claude configuration directory as installation evidence.
- Do not change client actions, IPC command names, or proxy configuration.

## Technical requirements

- Reuse the existing Windows desktop and MSIX discovery paths and macOS bundle discovery.
- Preserve runtime process paths when Claude is running.
- Keep unsupported platforms returning no installed executable.
- The page must retain its existing master-detail interactions and actions.
