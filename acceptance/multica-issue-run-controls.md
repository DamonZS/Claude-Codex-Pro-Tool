# Issue Run Controls Acceptance

Specification: `spec/multica-issue-run-controls.md`.

- Suppressed Core upsert create/update persists fields and CAS revision, creates no binding,
  sends no native request, and remains suppressed after command recovery/replay.
- Same command with changed controls conflicts, including an original revision.
- Handoff enters exactly one native opening request and one binding-linked audit;
  replay and later ordinary edits do not inject or replace it.
- Offline dispatch retains the original handoff through store/runtime recreation.
- Backlog/custom backlog and ordinary edits do not dispatch; preview agrees with
  creation, reassignment and backlog promotion decisions.
- Invalid controls, embedded controls, CAS errors and denied Agent access create
  neither Issue changes nor run intents. Existing capacity enforcement still holds.
- An eligible final native prompt exceeding 32 KiB, including handoff and agent
  instructions, fails before Issue/receipt/intent persistence. Test the exact
  byte boundary and an oversized combined prompt whose individual fields fit.
- Adapter PUT/PATCH and batch updates forward controls outside the Issue.
  Upstream-facing create/move reject these fields with 400. The original modal
  uses the verified native capability, retaining the legacy runtime gate.
- Run focused Core tests with `-j 1`; record commands and actual results below.
  Real UI, Release, Squad execution and historical attempt changes are excluded.

## Evidence

Frontend checkpoint observed during the read-only review on 2026-09-18:

```text
npm --prefix apps/codex-workflow-surface test -- src/native-subtasks.test.tsx src/main.test.tsx src/multica-adapter-run-controls.test.tsx --maxWorkers=1
3 files passed; 61 tests passed
```

This verifies fixture adapter/modal behavior and the included page regressions,
not live native dispatch. It predates the parent's subsequent native-subtask
partial-read and mount-callback fixes.

Core focused run observed on 2026-09-19:

```text
cargo test -p claude-codex-pro-core --lib issue_run_controls -j 1 -- --nocapture
8 passed; 0 failed; 0 ignored; 703 filtered out
```

The run includes three existing route Recording Host tests (suppression/replay,
offline handoff dispatch, invalid controls, backlog promotion and pending receipt
recovery) and five tests in `multica_workspace/issue_run_controls.rs`:

- Custom backlog creation and edits leave no intent; custom active promotion
  agrees with preview and stores the original note once. Terminal promotion
  forecasts no run; a later ordinary edit neither replaces the note nor creates
  another intent.
- Wrong CAS on creation or update leaves workspace bytes and receipts unchanged,
  preserves the existing revision and creates no intent or execution store.
- Private Agent access rejection on creation/update, including suppression,
  leaves workspace bytes/receipts/intents unchanged and creates no execution store.
- An independently counted combined prompt, with multibyte labels and handoff,
  succeeds at exactly 32 KiB and fails at 32 KiB + 1 before creation/update writes.
  Workspace bytes, including receipts, remain unchanged on rejection.
- A suppressed oversized assignment still saves without reserving a native
  intent, because no native prompt is dispatched.

The only compiler warning in this run was the existing unused
`LEGACY_MANAGER_NAME` import in `install/macos.rs`. No broader workspace,
capacity suite, Release build or real UI run is claimed by this focused result;
those checks remain with parent integration.
