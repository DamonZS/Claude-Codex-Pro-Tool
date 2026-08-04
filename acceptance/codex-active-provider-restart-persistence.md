# Acceptance: Codex Supplier Persistence When CCP Routing Is Disabled

Verification target: `spec/codex-active-provider-restart-persistence.md`.

## Pass criteria

1. Disabling routing for the active CCP Codex supplier calls
   `switchSupplierProfile("codex", activeProfileId, nextSettings)` after the
   affected profiles have been updated to `routeEnabled=false`.
2. The route-disable handler does not call `clearRelayMode()` and therefore
   does not intentionally restore official Codex mode.
3. A failed supplier reapplication is not followed by a settings-only save or
   a success notice.
4. The route toggle is disabled while an operation is in flight, and an
   immediate duplicate handler call is ignored.
5. A route-enabled live config points at the local CCP proxy while its settings
   record retains the real upstream `/v1` URL; reloading settings and switching
   the same supplier to direct mode writes that upstream URL without manually
   restoring transient fields.
6. Read-only provider-environment lookup leaves live `config.toml` and
   `auth.json` byte-for-byte unchanged, covering the restart-facing read path.
7. The separate explicit "Clear API mode" action remains available and keeps
   its existing official-mode cleanup behavior.
8. CC Switch import/category logic and Claude/Claude Desktop route behavior are
   unchanged by this task.
9. Tests and diagnostics do not expose a real API credential.

## Required evidence

- Red/green output for the route-toggle regression test in `cdp_bridge`.
- Passing targeted commands:

  ```text
  cargo test -p claude-codex-pro-core --test cdp_bridge manager_disabling_active_codex_routing_reapplies_supplier_without_clearing_api_mode -- --nocapture
  cargo test -p claude-codex-pro-core --test relay_switch -- --nocapture
  npm --prefix apps/claude-codex-pro-manager run check
  cargo fmt --check
  git diff --check
  ```

- A focused diff review showing that no CC Switch classification or generic
  provider-repair behavior was added.
- If broader checks are run, report their exact status and separate unrelated
  pre-existing failures.

## Non-scope checks

- No live request to a user's provider is required.
- No modification of the user's real `~/.codex` files is required.
- No installer or release build is required unless separately requested.
- No change to CC Switch Profile classification is expected.
