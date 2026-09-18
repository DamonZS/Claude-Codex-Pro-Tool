# Acceptance: Request Observability and Titlebar Anchor

Validates `spec/request-observability-and-titlebar-anchor.md`.

- Recorded requests expose timestamp, source/provider/model/protocol when observed, outcome, latency, and optional input/output/cache tokens without raw messages or credentials.
- Missing metrics display unknown; event counts never imply full unobserved traffic coverage.
- Success, upstream failure, streaming usage, cache usage, and malformed/missing usage have focused tests. Collection does not corrupt streamed responses or change routing.
- Overview retains its horizontal three-track chart, time axis, event chips and legend; only a compact Token row is added above it. No list, tabs, filters, search or pagination remains. All collected request records appear in the tracks, aligned by timestamp, with provider/model, protocol/proxy and Agent outcome/latency visible. Dense records stack without overlap; direct HTTP unknowns remain explicit. No removed memory lane.
- Usage selects the latest reported record for the current Agent, labels source/time, preserves zero/unknown and never combines overlapping sources. Verify unsorted records, scope switching, missing values, dense event layout, stale data and independent refresh failures.
- The version text's last digit has a small positive gap to the minimize control, with no overlap across resize and variable version lengths. No deleted memory-control width is reserved.
- Verify current source/build and actual injected version/geometry separately. Capture live evidence if accessible; state any verification limit explicitly.
- Run relevant Rust tests, TypeScript check, Vite build, renderer syntax checks, and Release build. Retain user data and all unrelated dirty changes.

## Evidence: 2026-09-18

### Timeline Presentation Follow-up

The user's later request supersedes the list/tab presentation described in the earlier evidence below. The overview now uses the original horizontal three-lane chart and surrounding layout, plus a single latest-usage Token row. Each collected record has aligned Provider, protocol and Agent chips; colliding chips occupy separate scrollable subrows inside each compact track. Local usage explicitly marks HTTP as unobserved. The row displays one observed record, not an all-time total or combined proxy/local sum.

- `node scripts/test-request-timeline.mjs`: passed. Covers all 30 fixture records across three lanes (90 chips), no overlap, same-request time alignment, adaptive ticks at 480/640/1200 chart widths, single-source latest usage, unknown/zero values, Agent scope, log sanitization and independent request/log failures.
- `npm --prefix apps/claude-codex-pro-manager run check`: passed.
- `npm --prefix apps/claude-codex-pro-manager run vite:build`: passed, including 15 workflow integration checks, 138 workflow tests and the artifact smoke test. Existing bundle/CSS warnings remain. A final Manager-only `npm exec -- vite build` also passed after shortening protocol chip labels.
- The existing preview at `http://127.0.0.1:1420/` returned HTTP 200.
- Browser access again returned `unsupported Codex auth method: apikey`; no live screenshot or native visual acceptance is claimed. Layout evidence above is generated-markup and packing-geometry testing, not browser pixel verification.
- Only the timeline component/style, its focused tests and this spec/acceptance pair changed in this follow-up. Collection, titlebar injection, Multica, user databases and other dirty work are preserved.
- `cargo build --release -p claude-codex-pro-manager --bin claude-codex-pro -j 1`: passed in 5m 21s, with existing unused-code warnings. Default artifact: `D:\Project\Claude-Codex-Pro-Tool\target\release\claude-codex-pro.exe`, 86,052,864 bytes, modified 2026-09-18 18:09:43. SHA256: `F24E8871F1164191549442AF9C00C405E05271A2FD5ADE2D79F1337B78397025`. Final timeline tests and `git diff --check` passed. The app was not relaunched and Codex was not restarted.

### Earlier Collection and Titlebar Evidence

- `cargo test -p claude-codex-pro-core --test request_telemetry --test protocol_proxy -p claude-codex-pro-data --test request_history`: passed, 15 telemetry + 59 protocol + 14 local-history tests; one real-data smoke test is opt-in. Telemetry tests include loopback HTTP, exact SSE forwarding, network failure, interrupted streams, cache counters, bounded storage and cross-process writes.
- The data worker separately ran the opt-in read-only smoke: 200 Codex records, 198 with positive usage, 173 ms. No HTTP timing/status was inferred from these local records.
- `npm --prefix apps/claude-codex-pro-manager run check`: passed.
- `node scripts/test-request-timeline.mjs`: passed, including records, unknown/zero metrics, paging, global event scope and independent request/log refresh failures and timestamps.
- `npm --prefix apps/claude-codex-pro-manager run vite:build`: passed; existing large-chunk warning remains.
- `node --check assets/inject/renderer-inject.js`: passed.
- `node --test scripts/test-titlebar-anchor.cjs`: four tests passed, including 45 version/width/zoom combinations. See `acceptance/titlebar-anchor-evidence.md` for the distinction between geometry fixtures and live rendering.
- `rustfmt --edition 2024 --check` on the four new telemetry/history source and test files: passed. `git diff --check`: passed.
- `cargo build --release`: passed in 1m 45s. Latest application: `D:\Project\Claude-Codex-Pro-Tool\target\release\claude-codex-pro.exe`. Existing compiler warnings remain. The two old CCP processes were stopped only after their executable paths matched this exact project artifact; Codex was left running. Manager was relaunched from the new artifact to restore its helper.
- Frontend preview at `http://127.0.0.1:1421` returned HTTP 200. This is a browser preview, not evidence of Tauri request collection or native rendering.
- Cache inventory: `target/debug` 53.95 GiB and `target/release` 3.46 GiB during build. No cache, user database or configuration cleanup was performed.

## Open Verification Items

- `cargo fmt --check` still reports formatting differences in existing modified code outside this task (commands, manager startup, session routes/tests and protocol request repair). Those sections were left intact.
- `cargo test --workspace` compiled but stopped before execution because the core unit-test executable was missing (`os error 2`). No claim of a full workspace pass is made.
- Expanded `cdp_bridge` suite: 94 passed, two failed on existing Projects/contact source assertions. Expanded `windows_subsystem` suite: 88 passed, four failed on Claude patch/release-workflow source assertions. The required assertion strings are also absent from the corresponding HEAD sources; these unrelated contracts were not changed to obtain a green result.
- Native browser/app access reported `unsupported Codex auth method: apikey`. No live screenshot, current injected menu version or minimize/text geometry was obtained. Codex itself was not restarted. Menu version 13 and the final-digit gap still require verification after loading the newly built payload.
- Coverage is deliberately bounded: latest 500 persisted proxy records and 200 local usage records in the UI, with separate sources and no combined token total. Historical direct HTTP latency/status and missing upstream usage remain unknown. This is not a complete historical audit of every direct request.
