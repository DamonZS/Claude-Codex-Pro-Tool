# Multica API Adapter Matrix

Snapshot: 2026-09-18; run-controls contracts and focused evidence refreshed 2026-09-19. This is an implementation/contract inventory, not full three-page acceptance.
Upstream revision: `9fce92f427694d7d303258aa281b05c902a95ba9` (`UPSTREAM.md` and `vendor/multica/source-files.json`).

## Source Scope

Paths below are relative to `vendor/multica/packages/` unless prefixed with `src/`.

| Scope | Original sources |
| --- | --- |
| Tasks | `views/my-issues/components/my-issues-page.tsx`, shared IssueSurface, issue detail and `views/modals/create-issue.tsx` |
| Agents | `views/agents/components/agents-page.tsx`, agent detail/tabs, `views/agents/create/use-create-agent-submit.ts`, `core/agents/draft.ts`, `views/agents/components/create-agent-dialog.tsx` |
| Autopilots | `views/autopilots/components/autopilots-page.tsx`, detail/editor/trigger/run dependencies |
| Wire contracts | `core/api/client.ts`, `core/api/schemas.ts`, `core/types/{agent,autopilot,property}.ts` |
| Adapter | `src/multica-api-adapter.ts`, `src/multica-adapter-{dto,query,builder,webhooks,native}.ts` |
| Core authority | `crates/claude-codex-pro-core/src/{multica_workspace,routes,multica_execution,multica_execution_store,multica_builder,multica_webhooks,multica_native_domain}.rs` and `multica_workspace/metadata.rs` from repository root |

The tables distinguish mapped transport from native runtime proof. Vendoring a dependency does not make its API operational. Unknown endpoints return HTTP 503 `capability_unavailable`; unsupported request fields can return HTTP 400. No network fallback exists.

## Mapped API Surface

`GET/POST` denotes both methods, braces denote path parameters, and comma-separated endpoints in a cell are distinct routes. Bridge paths are relative to `/multica`.

| Upstream method and exact endpoint | Source scope | Bridge mapping / limits |
| --- | --- | --- |
| `GET /api/me`, `GET /api/workspaces`, `GET /api/workspaces/{workspaceId}/members` | Shared identity/actor selectors | `/workspace/bootstrap`; local user becomes one ordinary member, no elevated role or invented name/email |
| `GET /api/runtimes?owner=me` (also without query) | Manual Agent form | `/workspace/query` resource `runtimes`; only verified current `codex_page_host` inherits local user ownership. Verified Host capability maps to `metadata.nativeTaskHostSupported` and `nativeModelSelectionAuthoritative`; no fabricated CLI version/model catalog |
| `GET /api/issues`, `POST /api/issues/query`, `GET /api/issues/search`, `GET /api/issues/grouped`, `GET /api/issues/children`, `GET /api/issues/child-progress` | Tasks | `/workspace/query` resource `issues`; filter before pagination; grouped endpoint supports assignee grouping |
| `POST /api/issues/table/groups`, `POST /api/issues/table/rows`, `POST /api/issues/table/facets` | IssueSurface | Bounded complete resource scan, snapshot-bound cursors; property grouping supports select/checkbox, including empty/unset/removed-option buckets and catalog-bound cursor invalidation. Multi-valued grouping is excluded by the pinned upstream grouping contract |
| `POST /api/issues`, `GET/PUT/PATCH/DELETE /api/issues/{id}` | Task create/detail | `/workspace/query`, `/workspace/upsert`, `/workspace/delete`; CAS; Core owns assignment reservation/dispatch. Issue PUT/PATCH maps `suppress_run`/`handoff_note` to top-level `suppressRun`/`handoffNote`, never entity fields. Adapter note limit: 4096 JavaScript string units. Core upsert also accepts controls on creation; upstream-facing POST create rejects them with 400 |
| `POST /api/issues/quick-create` | Agent quick task / `views/modals/quick-create-issue.tsx`; `core/api/client.ts::quickCreateIssue` | Agent-only local mapping: issue upsert with full prompt in description and first line as title, then real `/executions/list` binding -> HTTP 202 `{task_id}`. No second execution create. Errors after persistence include `issue_id`; squad/attachments remain unsupported |
| `POST /api/issues/{id}/move` | Task drag/state/assignment | `/workspace/move-issue`; explicit null neighbors and assignment preserved. Upstream move rejects run-control fields with 400 |
| `POST /api/issues/batch-update`, `POST /api/issues/batch-delete` | Task bulk actions | Individual CAS writes; batch-update `updates` forwards run controls per Issue with separate command IDs/signatures. Partial error includes `completed_ids` and `failed_id`, not atomic |
| `GET/POST /api/issues/{id}/comments`, `GET/PUT/PATCH/DELETE /api/comments/{id}`, `GET /api/issues/{id}/timeline` | Issue detail | Comments store and Core timeline projection; comment-trigger execution is not implemented. Issue-write handoff is a distinct implemented run-intent feature, not a fabricated comment |
| `GET /api/issues/{id}/subscribers`, `POST /api/issues/{id}/subscribe`, `POST /api/issues/{id}/unsubscribe` | Issue detail | Persisted `subscribers`, deterministic identity, CAS and command receipts; Core validates target and mutation access |
| `POST/DELETE /api/issues/{id}/reactions`, `POST/DELETE /api/comments/{id}/reactions` | Issue/comment reactions | Persisted `reactions`, authoritative local actor, deterministic identity and CAS/receipts; reads use existing issue/comment projections |
| `GET/POST /api/labels`, `GET/PUT/PATCH/DELETE /api/labels/{id}`, `GET/POST /api/issues/{id}/labels`, `DELETE /api/issues/{id}/labels/{labelId}` | Tasks/label controls | Labels catalog and issue CAS; agent/skill label attachment is not mapped |
| `GET/POST /api/issue-statuses`, `PATCH/DELETE /api/issue-statuses/{id}` | Task status controls | Core catalog plus single-entity CAS writes; POST derives immutable key, PATCH preserves category/key, DELETE archives and returns the status instead of physically deleting it. System writes return 403 |
| `PATCH /api/issue-statuses/reorder` | Task status controls | Atomic `/workspace/reorder-statuses` `{category,ids,expectedRevisions,commandId,commandSignature}` -> `{status,statuses}`. Adapter adds the category's built-in row to the upstream custom-ID order; Core validates complete category/revisions under one store lock |
| `GET/POST /api/issue-views`, `GET/PUT/PATCH/DELETE /api/issue-views/{id}` | Saved task views | `issue_views` store; separate from view preferences |
| `GET/POST /api/properties`, `PATCH /api/properties/{id}` | Property catalog | Persisted `properties`; immutable type, options/archive validation, CAS/receipts, derived usage counts; list returns `{properties,total}` |
| `PUT/DELETE /api/issues/{id}/properties/{propertyId}` | Issue properties | Validated issue CAS preserving unrelated property entries; returns `{properties,issue_revision}` |
| `GET/PUT /api/issue-view-preferences` | View visibility/order | Persisted current-user `issue_view_preferences`; unique scope, CAS/receipts; default preferences only after a successful authoritative empty query |
| `GET /api/projects`, `GET /api/squads`, `GET /api/skills` | Shared selectors | Query corresponding existing resources; skills require genuine Host inventory |
| `GET/POST /api/agents`, `GET/PUT/PATCH/DELETE /api/agents/{id}`, `POST /api/agents/{id}/archive`, `POST /api/agents/{id}/restore` | Agents | `/agents/create` for create; workspace CAS otherwise. Core distinguishes invocation ACL from native execution permissions and validates authoritative targets; manual native selector uses default/runtime-managed model configuration |
| `GET/PUT /api/agents/{id}/skills`, `DELETE /api/agents/{id}/skills/{skillId}`, `PUT /api/agents/{id}/skills/{skillId}/enabled` | Agent skills tab | `/skills/bindings`, `/skills/bindings/replace`, `/skills/bind`, `/skills/unbind`; inventory/trust errors stay errors |
| `GET /api/agents/{id}/tasks`, `POST /api/agents/{id}/cancel-tasks`, `GET /api/issues/{id}/active-task`, `GET /api/issues/{id}/task-runs` | Agent/Task execution UI | `/executions/list`, `/executions/cancel`; task DTOs derive from native bindings |
| `POST /api/issues/{id}/rerun` | Task execution | `/executions/create`; no second create during assignment; dispatch depends on Core/parent worker |
| `POST /api/issues/{id}/tasks/{taskId}/cancel`, `POST /api/issues/{id}/tasks/{taskId}/continue`, `POST /api/issues/{id}/tasks/{taskId}/open` | Task execution | Native cancel/continue/open with binding revision and stable operation key |
| `GET /api/tasks/{id}/messages`, `GET /api/tasks/{id}/status`, `POST /api/tasks/{id}/cancel`, `POST /api/tasks/{id}/continue`, `POST /api/tasks/{id}/open` | Task detail | `/executions/messages/list`, `/executions/status`, `/executions/cancel`, `/executions/continue`, `/executions/open`; open calls parent `openThread` using Core thread ID; messages are summaries, not a full transcript |
| `GET /api/agent-task-snapshot`, `GET /api/working-agents`, `GET /api/agent-activity-30d`, `GET /api/agent-run-counts` | Agents/Task activity | Derived from real execution timestamps/states; no token or runtime metrics fabrication |
| `GET/POST /api/autopilots`, `GET/PUT/PATCH/DELETE /api/autopilots/{id}` | Autopilots | Workspace CAS; pause/resume are status patches; detail includes persisted triggers |
| `POST /api/autopilots/{id}/collaborators`, `DELETE /api/autopilots/{id}/collaborators/{userId}` | Automation access | Autopilot CAS/receipts; Core metadata enforces creator-only collaborator changes and validates local targets |
| `GET /api/autopilots/{id}/runs`, `GET /api/autopilots/{id}/runs/{runId}`, `POST /api/autopilots/{id}/trigger` | Automation runs | `/autopilots/runs`, `/autopilots/run`, `/autopilots/trigger`; manual trigger sends `occurrenceId`, not a made-up trigger ID |
| `POST /api/autopilots/{id}/triggers`, `PATCH/DELETE /api/autopilots/{id}/triggers/{triggerId}` | Automation editor | Schedule/API/webhook kinds via autopilot CAS; schedule validation/UTC default. API/webhook creation then calls `/webhooks/provision`; updates read redacted `/webhooks/trigger`. Event filters are bounded and validated |
| `GET /api/autopilots/cron-preview?expr=...&tz=...` | Schedule form | `/autopilots/cron-preview` `{expr,tz}` -> `{status,next_runs}`; route exists in Core source; actual scheduler/Host execution remains to verify |
| `POST /api/autopilots/{id}/triggers/{triggerId}/rotate-webhook-token` | Webhook credentials | `/webhooks/trigger` reads credential revision; `/webhooks/rotate` receives `{autopilotId,triggerId,expectedRevision,commandId,commandSignature}`. Core signed replay precedes new-write CAS; replay returns no plaintext token |
| `GET /api/autopilots/{id}/deliveries`, `GET /api/autopilots/{id}/deliveries/{deliveryId}`, `POST /api/autopilots/{id}/deliveries/{deliveryId}/replay` | Webhook delivery history | `/webhooks/deliveries`, `/webhooks/delivery`, `/webhooks/replay`; real stored delivery DTOs, bounded pagination and replay lineage. Queued/failed delivery is domain data, not execution completion |
| `GET/POST /api/agent-builder/sessions`, `PUT /api/agent-builder/sessions/{id}/draft`, `PATCH /api/agent-builder/sessions/{id}/runtime` | Builder | `/builder` operations `list/create/save_draft/switch_runtime`; authoritative creator/workspace, persisted sessions/drafts and revisions; native Host only |
| `GET/PATCH/DELETE /api/chat/sessions/{id}`, `PATCH /api/chat/sessions/{id}/archive`, `GET/POST /api/chat/sessions/{id}/messages`, `GET /api/chat/sessions/{id}/messages/page` | Builder chat | `/builder` operations `get/update/delete/archive/messages/send`; actual native messages/task IDs, adapter cursor paging, revision checks, send idempotency key; no parallel model executor |
| `GET /api/chat/sessions/{id}/pending-task`, `GET /api/chat/sessions/{id}/draft-restores`, `DELETE /api/chat/sessions/{id}/draft-restores/{restoreId}`, `POST /api/tasks/{id}/cancel` | Builder lifecycle | `/builder` operations `pending_task/draft_restores/consume_restore/cancel`; task/session association is reconstructed after adapter reload |
| `GET/POST /api/quick-actions`, `PATCH/DELETE /api/quick-actions/{id}` | Quick-action catalog | Persisted `quick_actions` via workspace query/CAS/receipts; validated prompt/visibility/target; native execution supports agent targets |
| `POST /api/issues/{id}/quick-actions/{actionId}/render`, `POST /api/issues/{id}/quick-actions/{actionId}/run` | Issue quick actions | `/quick-actions/render` -> `{content}`; `/quick-actions/run` takes issue/action revisions plus command ID/signature and returns a Comment: `type:"comment"`, `quick_action_id`, `trigger_outcomes`. `queued` includes real native binding/thread/turn IDs; dispatch failure is a stored `blocked` outcome, not fake success |
| `GET /api/issues/limit-usage`, `GET /api/autopilots/usage` | Local cloud-gate state | `/issues/limit-usage` -> `{usage:null}`; `/autopilots/usage` -> `{usage:{action:"off",...null counts/periods}}`. Adapter unwraps `usage`. This explicitly means no configured Multica Cloud entitlement gate, not unlimited Codex quota or measured billing |
| `POST /api/issues/preview-trigger` | Assignment preview | `/issues/preview-trigger` receives `{issueIds,isCreate,assigneeType?,assigneeId?,status?}` and returns `{triggers,total_count}`. Shared mutation trigger predicate, saved agents, access, durable intents and bindings determine the forecast; eligible native-agent triggers report `handoff_supported:true`. The original run-confirm modal accepts verified runtime `metadata.nativeHandoffSupported`; legacy runtime CLI gates remain. Actual execution admission remains authoritative |

Bridge queries scan 100-item pages with a 5,000-record ceiling and reject truncated/stale windows. Bodies are limited to 32 KiB. The adapter is intentionally not an arbitrary URL/header/path proxy.

## Known Gaps

| Exact upstream endpoint or request feature | Current outcome / required work |
| --- | --- |
| `GET/PUT /api/agents/{id}/env` | Adapter remains on hold under the native-authoritative environment boundary. GET is 503; PUT with `custom_env` is rejected by the input guard (400); no environment write is forwarded |
| Issue nonempty `attachment_ids` | 503; attachment side effects not implemented. Run-control support and create/move boundaries are listed above |
| `POST /api/runtimes/{id}/models`, `DELETE/PATCH /api/runtimes/{id}` | Direct daemon discovery/device management remains 503. The shared model query recognizes verified `nativeModelSelectionAuthoritative:true`, uses the existing runtime-managed/default state and makes no daemon discovery request. Other runtimes retain their original discovery behavior, not a newly supplied daemon backend |
| Nonempty model/thinking/service-tier overrides | Native execution rejects unsupported overrides; saving metadata is not permission to override the native Codex selector |
| Squad execution in quick-create, quick-action run or assignment preview | Unsupported; an agent is not substituted for a squad |
| Builder message attachments, source-context/upload flows, runtime local-skill import, duplication with `custom_args` | Not mapped; unsupported routes/fields fail explicitly rather than returning fabricated data |
| Cloud billing/quota and full runtime metrics | Not implemented. Local null/off usage projections are not cloud measurements; preview is not a guarantee of dispatch or completion |

This inventory does not imply every shared control is working. Full original-page acceptance, live permission checks and end-to-end native execution proof remain separate from route implementation and fixture coverage.

Pinned upstream boundary: quick-action prompts containing `{{...}}` are rejected
by `server/internal/handler/quick_action.go::validateQuickActionPrompt`. The
adapter retains this rule; template expansion is not a missing port feature.

Issue run controls: suppression saves the mutation without a new run and does
not cancel existing work. A new eligible assignment retains its handoff on a
durable intent through Host deferral, injects it into the opening prompt and
records a binding-linked activity. Ordinary edits, suppression and existing
assignment dedup drop the supplied note. Core admission checks the combined
native prompt before saving a new intent; dispatch validates again. The native
32 KiB prompt budget is distinct from the adapter's note/body limits. See
`spec/multica-issue-run-controls.md` and its matching acceptance evidence.

Quick-create semantic boundary: original Multica queues an agent that later invokes its CLI to create an issue. The local mapping instead persists an issue immediately and assigns native execution; it does not implement that CLI, inbox completion protocol, squad leader briefing, or uploads. The original modal now accepts verified `nativeTaskHostSupported` for both base and explicit-field gates; other runtimes retain CLI thresholds. An accepted response means a real binding exists, not execution completion.

Webhook partial submission: the original form saves the autopilot before its trigger, and adapter trigger creation persists the trigger before provisioning its credential. Provision failure returns the real error plus `trigger_saved:true`, `autopilot_id` and `trigger_id`; it does not erase saved state or mint a token. Retry uses a stable derived provision command ID. Trigger deletion removes the authoritative ingress target, but adapter does not call the separate credential-revoke route for cleanup. Plaintext credentials are returned only by Core on first creation/rotation, not recovered from durable receipts.

Status write limitations: Core protects system rows, key/category immutability, duplicate keys and CAS. Upstream's owner/admin catalog policy is not represented as a local membership role; the bootstrap's ordinary member projection is not an admin grant. Automatic key derivation scans the catalog before upsert, so concurrent name-only creates can still receive Core 409 rather than upstream's lock-scoped suffix retry.

## Manual Agent Contract for Core

The original `core/agents/draft.ts::buildCreateAgentRequest`, called by `views/agents/create/use-create-agent-submit.ts`, serializes this default request (undefined fields are omitted):

```json
{"name":"Manual agent","description":"","runtime_id":"HOST_RUNTIME_ID","permission_mode":"private","invocation_targets":[],"skill_ids":[]}
```

`template` is optional creation attribution. Nonempty instructions/model/thinking_level/service_tier/avatar_url and conversation starters are optional. Shared workspace access sends `permission_mode:"public_to"` with `invocation_targets:[{"target_type":"workspace"}]`; member/team targets use `{target_type,target_id}`. A private draft sends no targets.

Adapter bridge request is `/multica/agents/create` with `{entity,skills}`. `entity` adds local workspace/user ownership, ID and timestamps; `skills` contains `{id}` references. Core must keep invocation access distinct from execution approval (`default/accept_edits/full_access/plan`) and validate grants against authoritative local identity. Preserve existing execution configuration without treating it as a public access grant.

Primary proof required: original form payload accepted by real Core -> agent appears after query/reload -> issue assignment reserves one binding -> binding becomes visible in Task/Agent UI -> parent opens the exact Core thread. Mock bridge success alone is insufficient.

## Persisted Core Resources

The My Issues host also queries the read-only `codex_native_agents` resource
directly through `/multica/workspace/query`. These are real native child threads,
not Agent definitions or editable Issues. Metadata comes from session spawn
edges and the latest history turn; results carry `source:"codex_native"` and
`read_only:true`, use 100-row pages within a 500-row recent window, and never
include instruction-bearing thread titles. Native child and parent navigation
uses the existing Host opener. Tests and live evidence are tracked in
`acceptance/codex-native-subtasks-in-my-tasks.md`.

`MulticaWorkspaceResourceKey` and the local workspace store now include dedicated `properties` and `issue_view_preferences` resources. `settings` remains a computed projection; saved `issue_views` remain separate from preferences and property definitions.

These use existing `/multica/workspace/query` and `/multica/workspace/upsert` routes, default-empty persisted collections and workspace/revision validation. Atomic status reorder uses its dedicated route rather than sequential upserts.

| Implemented resource | Persisted fields and invariants |
| --- | --- |
| `issue_view_preferences` | `id`, `workspace_id`, authoritative `user_id`, `scope_type` (`workspace/my/project`), nullable `scope_id`, `prefs:{hidden:string[],order:string[]}`, `revision`, `created_at_ms`, `updated_at_ms`. Unique tuple `(workspace_id,user_id,scope_type,scope_id)`; server enforces local identity. Query returns only current user's rows. First write expects revision 0; updates use CAS. Missing row may produce default preferences only after a successful authoritative empty query |
| `properties` | `id`, `workspace_id`, `name`, immutable `type`, `description`, `icon`, `config`, `position`, `archived`, nullable `archived_at`, `revision`, `created_at_ms`, `updated_at_ms`. Types: `text/number/select/multi_select/date/checkbox/url/actor/multi_actor`. Select options are bounded unique `{id,name,color}[]`; Core derives `usage_count` from real issue bags. Local catalog policy does not mint an upstream admin role |

Original property create accepts `{name,type,description?,icon?,config?}`; patch accepts `{name?,description?,icon?,config?,archived?}`. List returns `{properties,total}`. Per-issue PUT accepts `{value}` and DELETE removes the entry; response is `{properties,issue_revision}`. Core validates values against definitions/options/archive state, including finite numbers, dates, checkboxes, local actor references and multi-actor maximum 20. Adapter preserves unrelated bag entries and CAS without scalar-to-array coercion.

The original PropertiesTab is mounted at `/{workspace}/my-issues/properties`,
reachable through My Issues' secondary “管理属性” button with an explicit return
control. Core bootstrap `permissions.managePropertyCatalog` reflects the existing
local workspace enablement policy enforced by the mutation routes. The adapter
projects it as `/api/config.feature_flags.local_property_catalog_management`;
missing/false is read-only and membership remains `member`. The original component
accepts this local capability explicitly instead of minting an owner/admin role.
New options from the original editor carry blank IDs; the adapter allocates stable
IDs per command/option before typed validation, retaining existing option IDs and
durable replay. Query failures render an error/retry instead of an empty catalog.

Preference GET uses `scope_type` and optional `scope_id`; PUT uses `{scope_type,scope_id?,prefs:{hidden,order}}`. Response is `{scope_type,scope_id,prefs,updated_at}` with revision retained as additive metadata. Core enforces the current user and unique scope; read failure never becomes a successful empty preference response.

## Durable Replay and Remaining Gaps

- Within one adapter instance, the same mutation key and request signature share one promise; a changed payload under that key returns 409. At most 1,024 command entries are retained. Ambiguous failures remain cached to avoid silently replaying writes with a different CAS revision.
- Curie's Core ledger now accepts `commandId` plus a 64-hex `commandSignature` on workspace upsert/delete/move and agent create. Adapter forwards the stable upstream-request SHA-256 signature and calls `/multica/workspace/command` with those two fields before supported durable writes. `{status:"ok",found:true,result:<backend envelope>}` restores the original result; `found:false` permits the write. Lookup errors/malformed replies stop the write, rather than being interpreted as a miss.
- Reload/unmount loses the adapter cache and generated operation IDs, but explicit original keys can recover Core receipts. Batch steps derive independent stable IDs and signatures from the parent key/signature plus operation and issue ID. Autopilot creation and trigger creation, and label attach/detach, require separate upstream keys; reusing one key for different requests returns 409. Quick-create performs one durable issue write and reads its real execution binding; it does not run a second create operation.
- Ledger-aware adapter fixtures cover new-instance create/delete/batch/quick-create/trigger/label replay and changed-request conflicts. This is not a live renderer restart or crash proof. Skills mutations have no ledger mapping. Generated keys still change after reload unless parent/UI preserves the original key.
- Receipts for label/comment/archive operations may still require their referenced entities/catalog to exist while reconstructing the upstream response. A separately deleted dependency can therefore prevent response recovery even when the durable write itself is not repeated. The 1,024-command window and ambiguous in-memory failures remain bounded operational limits.
- Native execution create/continue/cancel receive `idempotencyKey`; manual automation trigger receives `occurrenceId`. These use Core mechanisms, but the complete UI-to-native cross-refresh replay proof is still pending.
- Webhook rotation now forwards the stable upstream-request `commandSignature`. Core `rotate_with_signature` binds the receipt to target and signature before applying CAS for new writes, so a refreshed adapter's newly read credential revision does not rotate twice. Changed payload/target conflicts; replay returns `credential_replay:true` and a null token. Adapter recreation and Rust store-reopen tests exist; live renderer/restart proof is separate.
- Quick-action run journals its comment and dispatch, returns real `queued` or `blocked` outcomes, and recovers a matching native binding instead of repeating an ambiguous dispatch. Builder create/send/cancel use Core operation keys; other Builder updates use revisions and should not be described as universally replayable writes.
- Core now persists command receipts with mutation state and owns pending-assignment/agent-journal recovery. Parent/host must preserve the original key when retrying after reload. Full crash-point coverage and live UI-to-native cross-refresh acceptance remain open; unsupported routes do not use receipt lookup to masquerade as implemented operations.

## Verification Evidence

- 2026-09-19 property UI completion: `property-management-ui.test.tsx` clicks the
  original catalog create/edit/archive/restore controls, Issue value add/edit/clear,
  and ViewBar visibility plus pointer drag reorder with actual `builtin:*` and
  `view:<id>` IDs. Fixture remount verifies stored values/preferences. Capability
  false/missing, deep-link bootstrap loading/error returns, catalog read retry and
  failed writes are covered. This is DOM/adapter evidence; final embedded Release
  build and live property UI verification are tracked separately in
  `acceptance/multica-property-management-ui.md`.

Current source/test inventory, refreshed after the parent's quick-action DTO correction:

- Read-only review checkpoint, 2026-09-18: `npm --prefix apps/codex-workflow-surface test -- src/native-subtasks.test.tsx src/main.test.tsx src/multica-adapter-run-controls.test.tsx --maxWorkers=1` passed 3 files / 61 tests. This proves the included fixture and original-modal behavior, not live Host execution, and predates the subsequent native-subtask partial-read and mount-callback fixes. Run-control Core results are recorded separately in `acceptance/multica-issue-run-controls.md`.
- Run-controls checkpoint, 2026-09-19: `cargo test -p claude-codex-pro-core --lib issue_run_controls -j 1 -- --nocapture` passed 8 tests (5 module tests and 3 existing route Recording Host tests). Covers custom backlog/preview, CAS and denied-access zero writes, final prompt byte-limit admission, suppression, offline handoff and pending receipt recovery. This is not a new full-workspace or live UI result.

- `multica-api-adapter.test.ts` exercises original `ApiClient.getMe()` (authoritative ID survives schema defaults), property/preferences/subscription calls, atomic reorder payload `ids`, and actual Core projection shapes. No speculative identity DTO rewrite was made.
- `multica-adapter-{builder,webhooks,native,query}.test.ts` covers Builder lifecycle, persisted webhook/replay contracts, truthful usage/preview, property grouping and native quick-action responses. Quick-action fixtures now use `type:"comment"`, matching `quick_action_id`, and `queued` outcomes rather than execution-completed claims.
- `multica-adapter-{capability,quick-create,models}.test.ts[x]` covers native capability gates, the original modal's actual Create button and default/runtime-managed model behavior with no daemon discovery. `source-manifest.test.ts` verifies vendored hashes; generator and source-manifest entries include the gate/model patches.
- Worker-observed earlier focused run: eight files, 154 passing tests, plus `tsc --noEmit`. Subsequent focused models/webhooks/provenance run: three files, 15 passing tests. These are dated checkpoints, not a new aggregate result for the later parent changes.
- Core source includes metadata validation/reorder tests, Builder tests, native-domain dispatch/preview tests, signed-rotation store-reopen tests and route-level signed replay tests. Their presence establishes coverage intent; this documentation-only update did not run Rust or claim new pass counts.
- Parent reported a real manual Agent form creation and reproduced the quick-create CLI gate and model-discovery issues. Adapter fixes and fixture tests address those gates; real post-rebuild Host execution, scheduler/Webhook dispatch, renderer-reload replay and final Release acceptance require the parent's corresponding live evidence.

Earlier recorded integration evidence below is retained as historical context, not certification of the current dirty-worktree build:

- Adapter tests: 117/117 passed after quick-create, single-status writes and durable replay integration; includes actual upstream Agent payloads, Core DTO defaults, partial submission errors and ledger-aware fixtures.
- `cargo test -p claude-codex-pro-core upstream_agent_create_dto_accepts_access_modes_without_native_permission_overrides --lib -- --nocapture`: 1/1 passed after Curie's ACL fix, using actual Core persistence and create function.
- `cargo test -p claude-codex-pro-core queued_assignment_dispatches_once_to_the_current_codex_host --lib -- --nocapture`: 1/1 passed; Recording Host verifies one dispatch and replayed native thread mapping, not a live Codex session.
- `npm --prefix apps/codex-workflow-surface test -- src/main.test.tsx -t 'preserves a real agent form across sync and creates the agent through the bridge' --maxWorkers=1`: 1/1 passed (12 skipped), using original manual form and adapter with fixture bridge; native backend success is covered separately above.
- Workflow typecheck and production build passed; outputs are `dist/codex-workflow-surface.js` and `.css`.
- The additional `cargo test -p claude-codex-pro-core workspace_command --lib -- --nocapture` attempt waited on another build's Cargo lock and was cancelled before running; no result is claimed for that invocation. Existing Core create/assignment passes above are separate evidence.
- Earlier integrated Manager `vite:build` passed: workflow TypeScript check, 138 workflow tests (117 adapter, 13 page, 8 host/provenance), workflow build, production-IIFE smoke and Manager build. This predates later adapter/domain changes.
- Earlier Core library verification recorded 611 passed, 7 ignored. Capacity, same-binding Continue/Cancel exclusion, previous-turn polling reconciliation, explicit-null drag and status whitelist were included. This is not a fresh result for subsequently added domains.
- No new live Host, scheduler tick, cross-refresh execution or Release acceptance is certified by this matrix. Parent owns integrated primary-flow evidence; Rust workers own their Core test results.
