# Titlebar Anchor Evidence

Date: 2026-09-18

Validates the titlebar portion of `spec/request-observability-and-titlebar-anchor.md`
and `acceptance/request-observability-and-titlebar-anchor.md` only.

## Changes

- Removed the Windows-only `window.innerWidth - menuWidth - 260` early-return offset.
- Windows now uses the existing WCO boundary, or the observed DOM minimize control.
- A DOM Range measures the version text through its final digit. Its right edge is
  placed 8 CSS pixels before the control boundary (or farther only if trigger
  padding needs more space to avoid an overlapping click target).
- No memory-control width is reserved. The label stays on one line, and the
  current version replaces a stale version label on reinjection.
- Existing resize and WCO geometry-change handlers continue to reposition.
  A ResizeObserver also handles menu width/font/content changes and disconnects
  the old observer when replacing the menu. Menu schema version is now 13.
- With no measured Windows anchor, or insufficient horizontal space for the
  entire label, the entry stays hidden; a later valid layout restores it.
- macOS positioning logic is unchanged.

## Executed Checks

- `node --check assets/inject/renderer-inject.js`: passed.
- `node --test scripts/test-titlebar-anchor.cjs`: 4 passed, 0 failed.
  The production positioning functions run against simulated CSS-pixel geometry:
  3 version lengths x 3 window widths x 5 zoom factors = 45 combinations,
  each positioned twice to check drift. Also covers DOM fallback, hidden WCO,
  missing anchor, narrow-space hiding/restoration, and the macOS header fallback.
- `cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_anchors_status_entry_to_right_titlebar_controls -- --nocapture`:
  1 passed, 0 failed, 95 filtered out. Compilation emitted an unrelated existing
  unused-import warning for `LEGACY_MANAGER_NAME` in `src/install/macos.rs`.
- `rustfmt --edition 2024 --check crates/claude-codex-pro-core/tests/cdp_bridge.rs`:
  passed. The initial standalone invocation without `--edition 2024` defaulted
  to Rust 2015 and rejected the existing async tests; the edition-correct check
  above succeeded without rewriting the file.
- `git diff --check -- assets/inject/renderer-inject.js crates/claude-codex-pro-core/tests/cdp_bridge.rs scripts/test-titlebar-anchor.cjs`:
  passed (Git reported line-ending conversion warnings only).

## Live Evidence and Limits

The required `cua_repl` inventory call returned no accessible browser or app and
reported `unsupported Codex auth method: apikey`. No live injected version,
WCO rectangle, text Range rectangle, screenshot, or minimize rectangle was
retrieved. The Node tests are geometry fixtures, not browser rendering evidence.

Codex was not restarted or reinjected. No external browser automation was used.
The running renderer may still use its previous payload. Real Windows titlebar
geometry remains pending until the browser connection is available; verify menu
version 13, the final-digit gap, and the full trigger bounds after loading the
new payload. Release building and telemetry validation belong to the main task
and were not run by this titlebar-only work.

Existing memory-removal and session changes were preserved. No user database,
settings, timeline, or request-telemetry source was edited.

## Subsequent Live Verification: 2026-09-18

After project Release reinjection, project CDP inspection measured the actual
`.claude-codex-pro-window-status-title` DOM Range against the native WCO boundary:
text `CCP dev-0.12.0`, final text edge 1135 CSS px, control boundary 1143 CSS px,
gap 8 CSS px. No Pangu Memory text was present. This supersedes the earlier
live-access gap for this desktop viewport; resize/zoom combinations remain covered
by the four passing geometry tests, not claimed as additional live measurements.
