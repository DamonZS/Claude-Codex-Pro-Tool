# Claude Codex Pro Tool Context

## Purpose

Claude Codex Pro Tool is a local operations console for Codex App and Claude Desktop. It combines Codex enhancement, provider/profile switching, plugin and skill management, memory assistance, launcher maintenance, update tooling, and release packaging in one Rust + Tauri + React workspace.

## Domain Vocabulary

| Term | Meaning |
| --- | --- |
| Manager | The Tauri management application in `apps/claude-codex-pro-manager/`. |
| Launcher | The silent Rust launcher in `apps/claude-codex-pro-launcher/`. |
| Core | Shared Rust logic in `crates/claude-codex-pro-core/`. |
| Data | Codex session, export, and Provider Sync data access in `crates/claude-codex-pro-data/`. |
| Pangu Memory | The memory assistance system backed by `memory_assist.sqlite`, capture logs, self-checks, and the inject summary cache. |
| Tier | A memory item's visibility layer: `active` (searchable, injectable) or `archived` (hidden from injection, recoverable, not physically deleted). See ADR 0001. |
| Retention | A 0..1 decay score per memory item (Ebbinghaus exponential, ~30-day half-life). A retrieval hit resets the timer and boosts strength; falling below the archive threshold (~0.12) moves the item to the `archived` tier. |
| Decay Exemption | Items never subjected to decay and pinned to the `active` tier: `source=manual` plus `category` in `safety-rule` / `project-rule`. Shown in the UI as "常驻" (pinned). |
| Lazy Decay | Retention/archival is computed on read (query, session summary, status) with a fingerprint debounce — no background timer thread. Mirrors the existing `STATUS_BACKFILL_FINGERPRINT` pattern. |
| Pangu MCP Server | A standalone stdio-transport MCP server (`apps/claude-codex-pro-mcp`) that exposes Pangu Memory read/write to any MCP-capable agent (Claude Code, Cursor, Codex CLI). Reads the same `memory_assist.sqlite` and settings file as the bridge — no extra IPC. Gated by `memoryAssistMcpEnabled` (default off) plus per-tool `memoryAssistEnabled` recheck. See ADR 0002. |
| Agent Workspace | Cross-agent workspace convention `agent://<agent-id>/<repo>` for non-Codex sources. The storage layer treats `workspace` as an opaque string, so this is a naming convention + MCP-layer helper — existing `codex:` keys stay compatible. See ADR 0002. |
| Provider/Profile | Codex API, relay, and official/hybrid configuration profiles managed by the app. |
| Plugin Hub | The catalog and installation surface for Codex, Claude, MCP, Ponytail, and skill resources. |
| Injection | JavaScript or bridge-driven augmentation loaded into Codex or wrapper windows. |
| Harness Engineering | The project workflow requiring specs, acceptance criteria, minimal implementation, and real verification evidence. |

## Current Architecture

- `apps/claude-codex-pro-manager/` owns the Tauri UI and command layer.
- `apps/claude-codex-pro-launcher/` owns the silent launcher entry point.
- `crates/claude-codex-pro-core/` owns shared operational behavior and should be treated as high blast-radius code.
- `crates/claude-codex-pro-data/` owns Codex session and provider sync data access.
- `assets/inject/` owns browser/window injection scripts.

## Working Rules

- Follow `AGENTS.md` before implementation work.
- Add or update `spec/` and `acceptance/` before non-trivial changes.
- Prefer minimal, scoped changes that preserve existing user-facing behavior.
- Do not log secrets or write API keys into memory, docs, tests, or logs.
- Do not modify Claude Chinese injection unless the task explicitly asks for it.
- Do not reset or delete local user databases without an explicit backup-covered request.

## Open Context Gaps

- ADR 0001 (memory decay/tiering) and ADR 0002 (MCP cross-agent) are recorded under `docs/adr/`; earlier decisions are still un-backfilled.
- Add ADRs under `docs/adr/` when a future change settles a durable architecture choice.
