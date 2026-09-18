# Webhook Predispatch Failure Acceptance

Spec: [multica-webhook-predispatch-failure.md](../spec/multica-webhook-predispatch-failure.md).

- Reproduce a pending occurrence transitioning to failed before a task exists;
  enqueue projects failed, preserves the run ID, diagnostics and dispatch count.
- Verify absent diagnostics, persisted reload, repeated recovery, duplicate ingress
  and explicit replay. Failed ingress receipts match their delivery response status.
- Verify execution failure with an existing task binding remains dispatched with
  one handoff count, including recovery before and after the first projection.
- Existing webhook tests continue to pass using temporary fixture stores only.

Command: `cargo test -p claude-codex-pro-core --lib multica_webhooks::tests -j 1 -- --test-threads=1`.

## Verified Evidence

- Before the fix, `cargo test -p claude-codex-pro-core --lib
  multica_webhooks::tests::failed_ -j 1 -- --test-threads=1` reproduced the bug:
  one passed, one failed; the predispatch test observed `dispatched` instead of
  `failed`.
- After the fix, the focused module command above passed all 17 tests.
- `cargo test -p claude-codex-pro-core --lib webhook -j 1 -- --test-threads=1`
  passed all 27 tests, including helper HTTP, occurrence deduplication and route
  rotation coverage. Cargo waited for the shared build lock; no extra target
  directory or concurrent linker was introduced.
- The new tests cover both absent diagnostics and a persisted failure code,
  unchanged zero/existing counters, store reload, duplicate ingress, explicit
  replay and failures with a task binding before/after delivery projection.
- Existing `install/macos.rs` unused-import warning remains unrelated.

Live UI, Release builds, historical delivery repair and full-port acceptance
are outside this focused regression.
