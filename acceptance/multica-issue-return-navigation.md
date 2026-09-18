# Multica Detail Return Navigation Acceptance

Spec: `spec/multica-issue-return-navigation.md`.

- [x] Loaded Issue detail shows one host header return arrow with accessible name
      and tooltip; clicking it renders My Issues and calls `onNavigate` with the
      workspace My Issues path.
- [x] Loaded Autopilot detail shows one host return control; clicking it renders
      the Autopilots collection and calls `onNavigate` with that path.
- [x] Loaded Agent detail shows one host return control; clicking it renders
      the Agents collection and calls `onNavigate` with that path.
- [x] Direct Issue, Autopilot, and Agent URLs render their detail route without
      first mounting the collection page.
- [x] Query-loading states for all three details keep the host return control.
- [x] Issue not-found, Autopilot missing-data, Agent not-found, Agent forbidden,
      and detail query-error states keep a usable return path and do not throw
      or render a blank surface. Exceptional Agent states may use `BackHeader`,
      provided its click updates the host through `onNavigate`.
- [x] A direct detail URL during workflow bootstrap loading or bootstrap error
      has a tested return affordance, or the host contract explicitly defers
      detail navigation while preserving an equivalent return action.
- [x] Existing task fields, actions and native Codex navigation remain unchanged.
- [x] Returning from every listed state updates the host route/sidebar through
      `onNavigate`; internal collection links do not bypass host synchronization.
- [x] Type checking, original-page regressions and source manifest checks pass.
- [x] Default Release is built and reinjected; actual click and geometry verified.

## Resolved Review Findings

- [x] Agent forbidden/not-found/query-error branches use an internal
      `BackHeader` rather than the normal host `leadingAction`; the return link
      and now have click and host-synchronization tests.
- [x] Loaded Autopilot and Agent tests click the return controls and assert the
      collection route and `onNavigate`.
- [x] Direct mounts, loading/missing/error states and Agent forbidden state have
      behavior tests.
- [x] Bootstrap loading/error shells now retain the detail-aware return control.

## Executed Evidence

- Final property-inclusive Release SHA-256
  `EDA0174FE2EB4FC8300FA6B3E851BFD7E2D99E09BFDE61FD4B160DD512D06F7D`:
  `node scripts/verify-workflow-return-live.mjs --run-live` exited 0 on
  2026-09-19. All three real details matched queried records, actual return
  clicks opened their collection, and sidebar selection synchronized.
  Evidence: `evidence/workflow-return-live/2026-09-18T17-09-51-620Z/`.
- The verifier now recognizes the original readonly Issue title as well as its
  editor, allows bounded 45s async bridge reads, and scrolls sidebar entries
  before screenshot checks. Its latest 23 offline tests passed; these changes
  correct verification assumptions without changing product behavior.

- `npm --prefix apps/codex-workflow-surface test -- main.test.tsx
  native-subtasks.test.tsx`: 46 passed (35 original-page integration tests and
  11 native-subtask behavior cases). All return-state cases passed.
- Manager `npm run vite:build`: passed with 248 workflow tests, source-manifest
  checks, 17 renderer tests, production-IIFE smoke and both Vite builds before
  the final native-subtask partial-read regression was added.
- Workflow and Manager TypeScript checks passed. Final Release/live evidence
  is recorded separately after rebuilding and reinjecting embedded assets.

This task does not change run controls, attachment support, property management,
or the detail-page source implementation itself.
