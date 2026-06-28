# Review Runtime Correctness Fixes

Verifies addendum requirements for `spec/overview-memory-injection-repair-regression.md`.

## Acceptance

1. Repair actions do not report fake all-green success.
   - Pass: payload exposes `codex_frontend_injected`, `claude_frontend_injected`, `codex_backend_online`, and `claude_backend_online`.
   - Pass: full success requires both Codex and Claude sides, while partial recovery returns `degraded`.
   - Evidence: `cargo test -p claude-codex-pro-manager --test windows_subsystem -- --nocapture`.

2. Helper port conflicts verify ownership.
   - Pass: `ensure_detached_helper` only accepts an occupied port when `/backend/status` proves it is the Claude Codex Pro HTTP helper.
   - Evidence: `cargo test -p claude-codex-pro-core --test launcher -- detached_helper_rejects_unverified_port_conflict --nocapture`.

3. Rust formatting passes.
   - Pass: `cargo fmt --check` succeeds.
   - Evidence: command output.
