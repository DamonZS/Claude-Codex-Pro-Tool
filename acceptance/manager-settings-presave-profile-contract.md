# Acceptance: Manager Settings Pre-Save Profile Contract

Target: `spec/manager-settings-presave-profile-contract.md`.

## Required Checks

- The Claude pre-save test asserts byte-for-byte preservation of config/auth,
  including unknown JSON fields, and retention of the explicit key/upstream.
- The Official pre-save test asserts byte-for-byte preservation of config/auth
  instead of expecting an implicit config wipe.
- All six Manager `normalize_settings_before_save` tests pass.
- Existing Core `normalize_claude_profile` tests pass, retaining credential
  synchronization and explicit-clear behavior at their actual owning layer.
- Only test code in `commands.rs` changes for this task; production code,
  user data, native configuration, and other workers' edits remain untouched.

## Commands and Evidence

```text
cargo test -p claude-codex-pro-manager --lib normalize_settings_before_save -j 1 -- --nocapture
cargo test -p claude-codex-pro-core --test relay_config normalize_claude_profile -j 1 -- --nocapture
```

Baseline reproduced: Manager 4 passed, 2 failed. The failures were the stale
current-key rewrite and Official empty-config assertions. Record post-change
results after execution; a passing build alone is not behavioral evidence.

## Observed Results (2026-09-18)

- Manager command above: 6 passed, 0 failed, 64 filtered out.
- Core command above: 7 passed, 0 failed, 102 filtered out, including current
  editor-key synchronization, explicit clearing, and unknown-field preservation.
- Scoped `git diff --check` passed; Git emitted only its LF/CRLF conversion
  notice. Existing unused-import/dead-code warnings remain outside this task.
- Source diff review confirms that this task changes only the two Manager
  tests and these two documents. No production behavior was changed.

## Non-Scope

Full library/workspace runs, Release compilation, live UI, and changes to Core
storage or supplier activation are outside this focused test correction.
