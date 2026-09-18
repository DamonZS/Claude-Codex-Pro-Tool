# Acceptance: Launcher Helper Probe Deadlines

Target: `spec/launcher-helper-probe-timeouts.md`.

## Pass Criteria

- The existing unverified-port conflict test completes with an error within a
  bounded outer timeout, even if Windows ownership lookup does not respond.
- A helper on the same current-thread Tokio runtime is verified and reused by
  both detached-helper and default launcher-hook entry points.
- Silent and trickling synthetic peers are rejected within the probe deadline
  plus scheduling tolerance; valid status before connection close still passes.
- An intentionally sleeping Windows query times out, releases its child, and
  permits another task on the same runtime to advance while it is pending.
- Focused serial launcher tests pass; production response identity and recovery
  ownership checks remain intact. No real Codex process/configuration is touched.

## Evidence

Baseline: focused `detached_helper_rejects_unverified_port_conflict` reproduced
the stall after compilation, with a new PowerShell process pending. The verified
project test invocation was interrupted. Post-change results are pending.

## Observed Results (2026-09-18)

- Core launcher unit probe tests: 2 passed, including silent/trickling peer
  deadlines and response identity checks.
- Core launcher integration detached-helper filter: 2 passed in 3.79 seconds;
  the formerly stalled conflict test completed with its expected error.
- Same-runtime default-helper reuse integration test: 1 passed in 0.76 seconds.
- Windows ownership-query timeout test: 1 passed in 3.08 seconds, including
  runtime progress and child cleanup checks.
- `git diff --check` passed with only the existing LF/CRLF conversion notice.
- Existing Core warning about `LEGACY_MANAGER_NAME` remains outside this task.

## Exclusions

No whole-workspace or Release requirement. The parent owns the Manager
`windows_subsystem` source assertion for the newly asynchronous probe call.
