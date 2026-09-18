# Acceptance: Remove Pangu Memory

Validates `spec/remove-pangu-memory.md`.

## Pass criteria

- A scan for `盘古记忆|MemoryAssist|memory_assist|memoryAssist|codexMemory|/memory/|claude-codex-pro-mcp|overview-memory|\.memory-` finds no production implementation, Manager UI, Tauri command, injection, or workspace-manifest integration. Negative regression tests, database packaging exclusion guards, and historical design/spec material may retain these strings. The unrelated Pangu control-deck branding stays unchanged.
- The MCP crate is absent from the workspace and no release script packages it.
- Manager and core compile without memory-related symbols.
- Existing local `memory_assist.sqlite` files are not deleted or rewritten by the change.
- Relevant Rust tests and Manager checks pass.

## Evidence

Run:

```text
cargo fmt --check
cargo test -p claude-codex-pro-core
cargo test -p claude-codex-pro-launcher
cargo test -p claude-codex-pro-manager
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build --release
node scripts/release/verify-release-workflow.js
node --check assets/inject/renderer-inject.js
git diff --check
```

Also record a source search showing no remaining executable integration and `git status --short` showing only intended files.

## Observed Results: 2026-09-18

Feature removal and data-preservation checks passed. Full-suite acceptance remains incomplete because of the pre-existing failures listed below; these unrelated implementations and assertions were preserved. The only production reference to the retired MCP executable is targeted Windows upgrade/uninstall cleanup, not packaging or runtime integration.

### Passed

- Production-source search across `apps`, `crates`, `assets`, `scripts`, `.github`, `Cargo.toml`, and `Cargo.lock`: only negative regression tests, database packaging exclusion guards, and targeted Windows legacy executable cleanup remain. A broader case-insensitive memory search shows unrelated third-party MCP, Claude translations, and ordinary in-memory operations.
- `cargo metadata --no-deps --format-version 1`: exactly core, data, launcher, and manager workspace packages; no standalone MCP package.
- Manager regression `pangu_memory_public_surfaces_are_removed`: passed, including both CSS files, frontend sources, Tauri registration, commands, renderer, settings, routes, launcher, and workspace manifest.
- Core bridge integration: 35 passed, including `removed_memory_routes_are_unknown` for status, capture, workspace resolution, and query.
- Launcher: 7 unit tests and 7 integration tests passed.
- Core full run with `--no-fail-fast`: library 582 passed, 7 ignored; all integration targets except `cdp_bridge` passed. After renderer line-ending normalization, `cdp_bridge` rerun: 94 passed, 2 pre-existing failures.
- Manager `--no-fail-fast`: library 66 passed, 2 failed, 2 ignored; integration 87 passed, 4 failed; doctests 4 failed. Task-specific removal, layout, settings, plugin, and session regression assertions passed.
- Frontend TypeScript check and Vite production build passed (1,614 modules). Vite retains its bundle-size warning.
- Release workflow validator, renderer JavaScript syntax check, and `git diff --check` passed.
- `workspace.css` PostCSS validation: non-memory rules and declarations match HEAD after removing memory selectors; 1,345 retained rules and 5,243 declarations.
- Independent `styles.css` review: all 1,594 non-memory selector records preserve their order, conditions, and declarations compared with HEAD.
- Browser preview at `http://127.0.0.1:1421`: overview, navigation, and settings rendered with no Pangu Memory entry. This uses the browser preview bridge, not native Tauri runtime verification.
- `cargo build --release` passed after stopping two processes whose executable paths were verified to be the project Release executable. Output: `D:/Project/Claude-Codex-Pro-Tool/target/release/claude-codex-pro.exe`, 43,663,360 bytes. The latest frontend build preceded this build.
- Existing session-deletion edits remain in the worktree; no commit, reset, or user-data migration was performed.

### Remaining Failures

Source comparison against HEAD identified these existing mismatches; a separate clean-HEAD full test run was not performed.

- Core CDP: `injection_script_exposes_contact_tab_with_qq_groups_and_wechat_qr` expects absent `nativeProjectReadOnly`; `codex_multica_projects_route_is_a_read_only_codex_projection` expects an absent Projects comment marker.
- Manager library: `normalize_settings_before_save_preserves_official_profile_auth` and `normalize_settings_before_save_does_not_treat_claude_json_as_codex_toml` fail existing configuration expectations. Their test bodies and normalization function are unchanged.
- Manager integration: `claude_zh_patch_closes_claude_before_elevation_branch`, `claude_zh_patch_manual_install_falls_back_to_uac_when_direct_msix_write_fails`, and `claude_zh_patch_primary_action_does_not_prompt_for_directory` still assert `install_root` where implementation uses `validated_root`. `github_auto_release_workflow_builds_installers_with_v0_tags` expects a missing PR workflow `run: npm run check` step.
- Manager doctests: examples for `sanitize_url_for_logging`, `sanitize_auth_header`, `format_error_chain`, and `failed_with_context` fail compilation; examples are unchanged.
- `cargo fmt --check` reports existing formatting in Manager commands/lib/tests, core protocol proxy, and the preserved session-availability route/tests. No blanket formatting was applied to unrelated changes.
- Review of the preserved session-availability changes found that hidden rows leave subsequent availability queries, so a temporarily unavailable session can stay hidden after recovery. This belongs to the existing session-deletion work and was not changed as part of memory removal.

Windows installer cleanup is checked by the release validator: exactly two targeted `Delete /REBOOTOK` commands and no other retired MCP reference in packaging inputs. An actual Windows installer upgrade and macOS packaging run were not performed on this machine.

### Data Preservation

Read-only SHA-256 checks match the continuation baseline for both existing databases:

- `C:/Users/Damon/.claude-codex-pro/memory_assist.sqlite`: `720992EB0449DC26B5A4B1A9842FF5DFFC18D5C5933B075F6256963A5101A75A`.
- `F:/pangu/memory_assist.sqlite`: `108C1EECDE1278663186706D3D5BA3F9E64643B479152B36BF9031EE8EB848A2`.

These hashes establish preservation during the continuation, not a baseline before the original interrupted task. No database deletion or migration code was added. Previously loaded Codex renderer code requires a Codex restart using the new CCP build; live native reinjection was not exercised in this verification.

## Runtime Recheck: 2026-09-18 Evening

- The reappearing UI came from `H:/Claude Codex Pro/claude-codex-pro.exe`,
  modified 2026-09-13 19:07:16, not the current project build. The old installation
  is preserved pending the user's decision about updating that directory.
- Explicitly launched the project Release and queried its native Tauri WebView
  at `http://tauri.localhost/`: exact Pangu Memory label count was zero. Screenshot:
  `acceptance/evidence/multica-live/manager-no-memory.png`.
- Re-ran the removal source contract: 1 passed. A regex used to exclude historical
  memory events had matched the test's retired-route string; replacing it with
  an equivalent lowercase string check retains the filtering and fixes the false
  positive. Timeline SSR/refresh tests also passed.
- Read-only database checks: the user-directory database still matches
  `720992EB0449DC26B5A4B1A9842FF5DFFC18D5C5933B075F6256963A5101A75A`.
  `F:/pangu/memory_assist.sqlite` now hashes to
  `D5F00A98D612FDCDB8FED4765A3DE05CE1FFB0275BC8F3C0F72F9FB1AB76A210`,
  modified 18:22:49. This differs from the earlier baseline; this continuation
  only read the database and does not claim its contents were unchanged.

## Non-goals

- Do not delete user data, local settings, or unrelated historical documentation.
