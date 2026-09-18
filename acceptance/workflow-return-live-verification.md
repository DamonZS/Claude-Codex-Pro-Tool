# Acceptance: Workflow Return Live Verification

Spec: `spec/workflow-return-live-verification.md`.

- [x] Offline syntax and fixture tests pass.
- [x] Post-Release issue detail opens from queried ID, matches the real record,
      returns via mouse input and shows My Issues with host selection synced.
- [x] Same evidence for Autopilot → Autopilots.
- [x] Same evidence for Agent → Agents.
- [x] Each case includes return button, list heading and sidebar screenshots,
      geometry and boolean assertions, without credential/title content logs.

Commands:

```text
node --check scripts/verify-workflow-return-live.mjs
node --test scripts/verify-workflow-return-live.test.mjs
node scripts/verify-workflow-return-live.mjs --run-live
```

The last command is reserved for the coordinated final Release verification.
Offline fixtures do not constitute live acceptance. Host synchronization here
means observable sidebar selection; the private host path is not introspected.

## Preparation Evidence (2026-09-19)

Final live execution passed on default Release
`EDA0174FE2EB4FC8300FA6B3E851BFD7E2D99E09BFDE61FD4B160DD512D06F7D`.
`evidence/workflow-return-live/2026-09-18T17-09-51-620Z/report.json` records
`ok:true` for issues/autopilots/agents with all nine cropped screenshots.
The final verifier has 23 passing offline tests. Async Runtime.evaluate reads
allow up to 45s within the unchanged total budget; other CDP calls remain 5s.
Readonly Issue titles and scrolled-out sidebar entries are explicitly covered.
Earlier preparation-only limitations below are historical.

`node --check scripts/verify-workflow-return-live.mjs` passed.
`node --test scripts/verify-workflow-return-live.test.mjs` passed 11/11 tests,
including success, loading, unavailable records, wrong detail/list, stale host
selection, occlusion, stuck return, active editing and visible error scenarios.
The default invocation test forbids network/socket access and verifies that
explicit `--run-live` is required. No live CDP navigation or click was performed.

At live execution, open an already mounted workflow collection after final
Release repair and close editors/dialogs. Evidence contains 9 cropped PNGs plus
`report.json` under a new timestamped `acceptance/evidence/workflow-return-live/`
directory. Success leaves the Agents collection visible. Each CDP request has a
5-second deadline, each state poll 15 seconds, verification 90 seconds, and the
process an overall 110-second watchdog. A missing fixture or stale host state is
a failed run, not a skipped/pass result.
