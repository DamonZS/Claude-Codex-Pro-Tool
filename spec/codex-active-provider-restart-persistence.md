# Codex Supplier Persistence When CCP Routing Is Disabled

## Background

CCP uses a supplier profile for two independent choices:

- which Codex API supplier is active; and
- whether requests reach that supplier directly or through CCP's local route.

The supplier-page route toggle currently conflates those choices. When routing
is disabled for the active Codex supplier, `toggleVisibleSupplierRouting(false)`
calls the explicit `clearRelayMode()` action. That action intentionally restores
official Codex mode by removing the root `model_provider` selection. The UI then
saves the same custom `activeRelayId`, leaving CCP settings and the live Codex
configuration inconsistent. A running Codex process may continue with cached
state, but after restart it falls back to the official OpenAI provider.

## Goals

- Treat the Codex route toggle as a transport-mode change, not a supplier
  selection change.
- When routing is disabled for the active Codex supplier, reapply that same
  supplier with `routeEnabled=false` through the normal supplier-switch action.
- Keep `activeRelayId`, the root `model_provider`, the provider table, upstream
  URL (including `/v1`), and credential aligned across CCP settings and live
  Codex files.
- Preserve launcher and provider-environment reads as read-only operations so a
  restart cannot silently change the selected supplier.
- Preserve the explicit "Clear API mode" command as the deliberate way to
  restore official Codex mode.

## Non-goals

- Do not change CC Switch import, profile classification, or profile categories.
- Do not reinterpret an `official` profile as `pureApi` based on its import
  source, endpoint, or provider table.
- Do not add a generic low-level TOML repair that chooses a provider table.
- Do not change Claude or Claude Desktop routing behavior.
- Do not change upstream URLs, protocol conversion, model routing, credentials,
  or API service behavior.
- Do not modify the user's real `~/.codex` files during automated tests.
- Do not revert unrelated working-tree changes.

## User workflow

1. The user selects a CCP Codex supplier backed by a third-party API.
2. The supplier is active and CCP routing is enabled.
3. The user turns off "Enable routing" for Codex.
4. CCP reapplies the active supplier in direct mode and persists the updated
   profile state.
5. Codex continues to use the same provider and credential immediately and
   after a full restart.
6. Only the separate explicit "Clear API mode" action restores official mode.

## Functional requirements

- `toggleVisibleSupplierRouting(false)` must compute the updated profiles with
  `routeEnabled=false` before applying the active Codex profile.
- If the active Codex profile is among the profiles affected by the toggle,
  CCP must call `switchSupplierProfile("codex", activeProfileId, nextSettings)`.
- That path must not call `clearRelayMode()`.
- The supplier-switch result remains authoritative: failures must stop the flow
  and must not be reported as a successful route change.
- While a route change is in flight, the toggle must be disabled and duplicate
  handler calls must be ignored so settings-only saves cannot race live writes.
- A successful switch must keep the same active profile id while writing a live
  Codex config whose root `model_provider` selects that supplier.
- Direct mode must use the supplier's stored upstream endpoint rather than the
  local CCP proxy endpoint.
- Loading or saving a route-enabled profile must not replace its persisted
  upstream endpoint with the generated local CCP proxy URL.
- The supplier credential and `/v1` endpoint must remain available after a
  settings reload and read-only provider-environment lookup.
- Enabling routing and toggling non-active profiles must retain their existing
  behavior.
- The explicit `clearRelayMode()` UI action and backend command must retain
  their official-mode cleanup semantics.

## UI and interaction requirements

- Keep the existing route toggle, labels, loading notice, success notice, and
  error handling.
- Disable the route toggle until the current route operation finishes.
- Do not add a confirmation dialog or a new control.
- On success, report that Codex supplier routing is disabled; do not claim that
  the API supplier itself was cleared.

## Data and interface requirements

- Reuse `BackendSettings`, `RelayProfile`, the existing `nextProfiles` value,
  and `switchSupplierProfile` action.
- Keep the existing Tauri command payload and response types.
- Do not serialize or log raw credentials in new diagnostics or tests.

## Technical constraints

- Keep the route action fix local to the supplier page. Core storage changes
  must be limited to preserving the upstream URL proven lost by the restart
  regression test.
- Do not introduce a new dependency or a new Tauri command.
- Preserve backup and atomic-write behavior in the existing switch path.
- Use synthetic providers and credentials in tests.

## Delivery scope

- This specification and its matching acceptance document.
- The minimal supplier-page behavior change.
- A regression test proving route disable reapplies the active Codex supplier
  and no longer invokes the official cleanup action.
- Targeted manager/core verification, formatting, and diff checks.
