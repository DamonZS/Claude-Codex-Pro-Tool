# Client enhancement Claude detection acceptance

Validates `spec/client-enhancement-claude-detection.md`.

## Pass criteria

- Codex Agent scope still renders all three local client records.
- Claude Agent scope still renders all three local client records and initially selects Claude Desktop.
- An installed but stopped Claude Desktop returns an executable path and install kind while retaining `processCount = 0` and `status = not_running`.
- Running-process executable paths are retained without invoking fallback discovery.
- Existing client actions and Tauri commands remain unchanged.
- Settings-backed capabilities show an accessible enable/disable switch and persist the changed boolean through `actions.saveSettingBoolean`.
- The Codex client lists "我的任务" and binds its switch to `multicaWorkspaceEnabled`; clearing or enabling that switch does not alter supplier or proxy URL fields.
- Detection-only rows remain status-only and do not expose a fake write control.
- A pending settings write disables the capability switches until the save promise settles.

## Evidence

- Core unit tests for executable-path merging.
- Manager Windows contract test.
- Manager source contract assertions for settings-backed switches, the `multicaWorkspaceEnabled` binding, and save-path usage.
- Type check, frontend build, Rust formatting check, and default Release build.

## Non-goals

- Claude account state and remote service availability are not checked.
