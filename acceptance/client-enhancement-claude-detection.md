# Client enhancement Claude detection acceptance

Validates `spec/client-enhancement-claude-detection.md`.

## Pass criteria

- Codex Agent scope still renders all three local client records.
- Claude Agent scope still renders all three local client records and initially selects Claude Desktop.
- An installed but stopped Claude Desktop returns an executable path and install kind while retaining `processCount = 0` and `status = not_running`.
- Running-process executable paths are retained without invoking fallback discovery.
- Existing client actions and Tauri commands remain unchanged.

## Evidence

- Core unit tests for executable-path merging.
- Manager Windows contract test.
- Type check, frontend build, Rust formatting check, and default Release build.

## Non-goals

- Claude account state and remote service availability are not checked.
