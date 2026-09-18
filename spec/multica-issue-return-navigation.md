# Multica Detail Return Navigation

## Goal

Second- and third-level Multica pages need a deterministic way back to their
collection page when opened from the Codex host, including a direct link. This
spec covers task/issue detail, automation detail, and agent detail while
preserving the upstream layout and Codex shell.

## Requirements

- Route the host return control through the existing leading/leadingAction slots
  for IssueDetailRoute, AutopilotDetailPage, and AgentDetailPage.
- Preserve a usable return path in each detail page's loaded, query-loading,
  not-found, forbidden, and query-error state. The normal loaded/loading route
  uses the host `leadingAction`; an upstream `BackHeader` is acceptable for an
  exceptional state only when its click is covered through the host navigation
  adapter and `onNavigate` contract.
- Direct paths must resolve without relying on a prior list-page mount:
  `/{workspace}/issues/{id}`, `/{workspace}/autopilots/{id}`, and
  `/{workspace}/agents/{id}`. Unknown IDs render a stable not-found state with
  the return control rather than a blank page or uncaught error.
- While the workflow host is still bootstrapping a direct detail path, retain a
  usable return affordance in the host loading/error shell or document and test
  an equivalent route-level fallback. The generic bootstrap status alone is
  insufficient for a pasted detail URL.
- Use the existing icon button and tooltip components with an accessible name.
- Update the host navigation callback so sidebar state follows every returned
  collection page, including returns from loading, not-found, forbidden, and
  error states.
- Do not silently change a direct detail URL into a list URL before the user
  activates return. Canonicalization of issue identifiers remains a replace of
  the same detail page.
- Do not change task data, native navigation, execution, authentication or styling.
- Keep the vendoring generator and provenance manifest reproducible.

## Review Evidence And Open Gaps

The current source has these verified partial paths:

- Issue detail forwards the control in loaded, loading, and not-found branches
  in `vendor/multica/packages/views/issues/components/issue-detail-route.tsx`.
- Automation detail forwards it in its loading, missing-data, and loaded header
  branches in `vendor/multica/packages/views/autopilots/components/autopilot-detail-page.tsx`.
- Agent detail forwards it in the loading and loaded branches. Its forbidden
  and not-found/query-error branches use an internal `BackHeader` without the
  supplied `leadingAction`; that is a presentation-consistency gap, while the
  `AppLink` still supplies a return path through the host navigation adapter.
- Existing behavior tests assert Issue return clicks for loaded, loading, and
  missing states, but only assert the presence of Autopilot and Agent loaded
  controls. They do not click those controls or assert `onNavigate`.
- There are no behavior tests for direct Autopilot/Agent mounts, Autopilot
  loading/missing-data returns, or Agent loading/not-found/forbidden/query-error
  returns. The Agent exceptional-state `BackHeader` paths are therefore
  unverified even though the source supplies a link.
- The host `Surface` replaces all detail content with a generic bootstrap
  loading or failure alert until the bridge is ready; those states currently
  have no detail-aware return control in `src/main.tsx`.

These are review findings, not completion claims. Source changes and live UI
verification are outside this read-only review.

## Delivery

Host mount, narrowly adapted upstream routes, generator, regression tests for
all three detail families and their state branches, native UI evidence, and the
latest default Release executable.
