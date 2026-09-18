# Launcher Helper Probe Deadlines

## Background

The serial Core launcher suite stalls at
`detached_helper_rejects_unverified_port_conflict`. The occupied-port path
performs synchronous socket I/O inside Tokio and then runs PowerShell
`Get-NetTCPConnection` through unbounded `Command::output`. Socket read timeouts
do not bound that subprocess. Per-read timeouts also permit a trickling peer to
extend the synchronous probe indefinitely. A same-runtime helper cannot respond
while a current-thread executor is blocked by the probe.

## Requirements

- Async helper creation/reuse uses async socket I/O with a 750 ms total deadline
  covering connect, request write, and response reads.
- The synchronous public probe retains its API and receives the same total
  deadline, not a fresh timeout per response chunk.
- Preserve the existing response identity checks, 64 KiB response bound, and
  acceptance of a verified response before the peer closes its connection.
- Windows ownership queries run asynchronously with a 3 second deadline and
  child cancellation on timeout. A failed ownership query does not authorize
  recovery or termination of an unknown port owner; return an explicit error.
- Update all callers affected by the private async ownership-query signature.
  Preserve existing process classification, recovery and port-selection rules.
- Tests use ephemeral loopback listeners and synthetic status payloads. Cover
  a silent peer, slow trickle, current-thread runtime reuse and query timeout.
  Test fixture cleanup and waits must be bounded.

## Scope and Interfaces

Only `crates/claude-codex-pro-core/src/launcher.rs`, its launcher tests, and this
specification/acceptance pair are owned. No dependencies, user settings, actual
Codex launches, UI changes, or Release builds. Public launcher APIs are unchanged.
The Manager source-contract test for helper reuse should follow the async probe
call; that separate file remains owned by the parent worker.

## Verification

Run focused tests with Cargo `-j 1` and serial test execution. Record observed
results, separating compilation/lock waits from test execution time.
