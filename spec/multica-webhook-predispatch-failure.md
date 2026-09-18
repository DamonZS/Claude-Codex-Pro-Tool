# Webhook Predispatch Failure Projection

This narrow correction extends `multica-upstream-ui-runtime-port.md`. The locked
upstream revision is `9fce92f427694d7d303258aa281b05c902a95ba9`; its webhook delivery
worker projects dispatch failures to failed deliveries.

## Contract

- When an admitted occurrence has status `failed` and no `task_id`, enqueue
  recovery persists delivery status `failed`, its run ID and failure diagnostics.
  It preserves `dispatch_attempts`: recovery did not dispatch a native execution.
- Missing diagnostics use `webhook_dispatch_failed`. The delivery response status
  is 503, matching existing dispatch failures; ingress receipts reflect that status.
- Reopening either store, duplicate ingress and repeated recovery reuse the same
  occurrence without another dispatch or counter increment. Failed deliveries
  leave the pending queue. Explicit replay creates a separate queued delivery.
- An occurrence with a task binding retains the existing dispatched projection,
  even if execution subsequently failed. Normal queued/handoff counting is unchanged.

## Scope

Only webhook persistence/projection and focused Rust tests change. No route,
workspace, frontend, native executor, user database or historical data migration
changes are included. The original delivery UI consumes the existing DTO fields.
