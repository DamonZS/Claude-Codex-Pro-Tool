# Workflow Detail Return Live Verification

Prepare `scripts/verify-workflow-return-live.mjs` for a separate post-Release
run against port 9230 and exactly one `app://-/index.html` page. This supplements
`multica-issue-return-navigation.md`; preparation is not live acceptance.

- Default invocation shows usage. Only `--run-live` enables CDP connections.
- Use the installed workflow surface and bridge, never reload, inject assets,
  replace callbacks, create fixtures, write business data or start execution.
- Bootstrap contributes only workspace slug. Select one existing record per
  issues/autopilots/agents from at most 20 rows each at offset zero. Missing
  accessible fixtures fail explicitly; no creation or unbounded scanning.
- Public surface navigation may open the selected detail. Its real return
  button must be clicked using CDP input after visibility and hit testing.
- Match loaded detail to its record inside the page, without returning title,
  name, credentials or full query responses to the Node process or logs.
- Verify expected collection heading, viewport-clipped surface and heading
  geometry, no visible alerts, return-control removal, and exactly one matching
  host sidebar entry with aria-current=page and data-state=active.
- Screenshot only the return button before clicking, and the collection header
  and selected sidebar entry afterwards. Evidence is timestamped under
  acceptance/evidence/workflow-return-live; never capture task content or whole
  desktop as a failure fallback.
- Bound target discovery, socket connection, each CDP call, polling and the full
  run. Emit only fixed failure stages and sanitized assertions in report JSON.
- Require an already mounted visible workflow surface (open a collection after
  final Release repair). End at Agents on success; on failure leave the current
  page for inspection. Native thread selection is outside this script.

Offline checks must exercise real page-step code with a synthetic DOM, route
callbacks and CDP input adapter, including stale host state, missing fixtures,
wrong detail, loading, and occluded controls. No real CDP access during checks.
