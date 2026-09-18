# Manager Settings Pre-Save Profile Contract

## Background

Two Manager unit tests still expect `normalize_settings_before_save` to run
Core profile normalization. Commit `9448f2e` (2026-08-04) deliberately removed
that call and documented preservation of submitted supplier records. The
Claude JSON test was introduced by `19912c52`; the Official assertion predates
that change. Both fail against the current helper implementation.

## Goal and Scope

Align these two tests with the existing Manager pre-save boundary, without
changing production normalization or user configuration. Changes are limited
to the tests in `commands.rs` and this specification/acceptance pair.

## Requirements

- Manager pre-save processing preserves Claude JSON config/auth text exactly,
  even with common config enabled and a different explicit editor API key.
  The explicit key and upstream URL must also survive this boundary.
- Official profile auth and config text remain intact at this boundary when
  no common-config extraction applies. This does not redefine Core storage or
  explicit official-mode activation behavior.
- Existing Codex common/context extraction tests continue to pass.
- Claude credential synchronization remains a Core normalization requirement;
  preserving input at the Manager boundary does not mean persisting stale
  credentials. Run the existing Core tests for current-key precedence, explicit
  clearing, and unknown-field preservation.
- `settings-direct-write` only changes persistence I/O. Its normalization and
  credential recovery requirements stay intact, as do
  `claude-desktop-proxy-upstream-models`,
  `supplier-target-routing-and-live-validation`, and supplier key preservation.

## Interaction and Data Constraints

No UI, command DTO, authentication, routing, file-write policy, or public API
changes. Use synthetic fixtures only; leave real settings and Codex/Claude
configuration untouched. Preserve unrelated shared-worktree edits.

## Delivery

Strengthened pre-save preservation tests, a matching acceptance document, and
focused Manager/Core test results. No Release build is required.
