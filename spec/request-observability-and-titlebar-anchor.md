# Request Observability and Titlebar Anchor

## Goal and Scope

The overview currently shows sparse diagnostic events rather than request details. Show real request/provider, protocol/proxy, and agent outcome information, including reported input/output tokens, cached tokens, status, and measured latency. Missing telemetry stays unknown, never synthesized as zero or success. Preserve direct routing choices and all user data. Do not log credentials, message bodies, or raw upstream error bodies.

The Codex CCP titlebar entry must sit immediately before the native minimize control: anchor the right edge of the final version digit against the minimize button's left boundary with a small stable gap. Do not reserve width for removed memory controls. Avoid overlap under resize, zoom, different version lengths, or reinjection; preserve native window controls and macOS behavior.

## UI and Data

- Overview preserves the horizontal Provider / protocol / Agent timeline, time axis, event chips and status legend. Remove the replacement list, tabs, source filter, search and pagination. Add only one compact Token row above the tracks; preserve surrounding overview styles.
- The Token row shows the current Agent's latest record with reported usage: input, output, cache read/write and total, with source and timestamp. Never sum overlapping local/proxy sources. Missing counters stay unknown; zero remains zero.
- Show every collected request in the three lanes at its observed timestamp, with provider/model, protocol/proxy, Agent outcome and latency directly in the chips. Local usage is identified as such and does not imply observed HTTP behavior. Same-record chips align by time; dense chips stack without overlap. Scroll within the existing timeline region. Tooltips provide remaining metrics. Runtime events remain distinctly labeled; collection and automatic refresh stay intact.
- Reuse existing proxy, diagnostic, and local usage infrastructure where practical; bound memory, file reads, and displayed rows. Identify source/coverage and unknown values honestly.
- Do not change provider selection or force direct requests through a proxy solely for telemetry.
- Titlebar uses measured text/control geometry and existing overlay APIs, not a fixed-width guess for the version label. Recompute on relevant geometry and content changes.

## Verification and Delivery

Focused parsing/metric tests and proxy tests, Manager type checking and build, renderer syntax and geometry tests, and a current default Release executable. Report coverage limits and live verification evidence separately. Preserve all existing uncommitted work.
