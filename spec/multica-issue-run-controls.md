# Issue Run Controls

## Context and Scope

Extend the existing local Issue upsert and native preview, without a second
executor. The pinned upstream is `9fce92f427694d7d303258aa281b05c902a95ba9`:
`server/internal/handler/issue.go` UpdateIssueRequest and
`server/internal/service/issue_trigger.go` WillEnqueueRun. Upstream update
controls are also accepted on Core local creation through
`/multica/workspace/upsert`, as requested for this port. The upstream-facing
adapter accepts them on Issue PUT/PATCH and in batch-update `updates` only;
POST `/api/issues` creation and POST `/api/issues/{id}/move` reject these fields
with HTTP 400, preserving the pinned upstream request boundaries.

## Contract

`/multica/workspace/upsert` accepts top-level camelCase `suppressRun` (optional
boolean, default false) and `handoffNote` (optional string). They are not Issue
fields. Reject embedded snake/camel control fields rather than persisting them.
Controls are valid only for Issues and participate in the command payload hash.
Use the existing request-size limit. The adapter bounds `handoff_note` to 4096
JavaScript string units; Core bounds a supplied note to 32 KiB of UTF-8. For a
new eligible run intent, validate the final title/description/agent-instructions
plus handoff prompt with `CodexThreadRequest::validate` before saving the Issue,
receipt or intent. The final native prompt limit is 32 KiB of UTF-8, including
section labels and separators. Reuse the existing assignment prompt formatter
for admission and dispatch, and retain dispatch-time validation. Suppressed,
non-triggering and deduplicated writes do not inject or validate a discarded note
as a native prompt; the control-field validation still applies.

The user may save an assignment/status change without starting a run. Suppression
does not cancel existing work. An omitted control retains normal trigger behavior:
creation/reassignment outside the backlog category, or promotion from backlog to
a nonterminal, non-backlog category. Ordinary edits do not trigger a run. Custom
statuses inherit their category. Existing local stable assignment dedup remains;
this task does not change historical attempt counts or implement Squad dispatch.

Only a new eligible assignment reserves a durable run intent. Handoff text is
stored on that intent, never on the Issue or a fabricated comment. Suppressed,
non-triggering and already-reserved writes drop the note. Host/capacity deferral
retains the original intent. Opening native context includes it once; successful
dispatch records one binding-linked activity. Replays neither rewrite the note
nor call Host again. Recovering a pending command uses its saved trigger decision.
Agent access, CAS and native capacity checks remain authoritative.

## UI and Delivery

The adapter maps upstream `suppress_run`/`handoff_note` to top-level bridge
`suppressRun`/`handoffNote`, never Issue fields. Batch updates use per-Issue
command IDs/signatures and report partial failures; they are not atomic.
The original run confirmation UI accepts verified runtime metadata
`nativeHandoffSupported`; other runtimes retain the upstream CLI gate. Preview
shares the mutation trigger predicate and reports `handoff_supported:true` for
eligible native-agent triggers. It is a forecast, not execution admission or
completion. Comment-trigger semantics and nonempty attachments remain separate
unsupported features. Deliver Core changes, recording-service
tests and the matching acceptance document. Real UI and Release builds belong
to parent integration, not this task.
