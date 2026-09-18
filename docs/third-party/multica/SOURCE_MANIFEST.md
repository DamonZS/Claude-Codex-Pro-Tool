# Multica Source Manifest

## Locked Source

- Upstream: <https://github.com/multica-ai/multica>
- Revision: `9fce92f427694d7d303258aa281b05c902a95ba9`
- Original review: 2026-09-04; closure port: 2026-09-18.
- Audit input: clean `.upstream-multica` Git checkout at that revision.
- Pinned Git LICENSE SHA-256: `0e42d37bb02dc61f270c5a0528d489da76e5a578b209856f2e95ee4d60aacdbe`
- Pinned Git NOTICE SHA-256: `763619b43ae4f18c43bef5284c04a5739f84cd9f935c0123cf67834541ec3d9a`
- The existing documentation copies use CRLF; their LF-normalized text equals
  the pinned Git files. Vendor files and the embedded dialog use Git bytes.

## Included Source

The authoritative per-file inventory is
`apps/codex-workflow-surface/vendor/multica/source-files.json`. Every row records
the upstream path, original SHA-256, destination, derived SHA-256 and modifications.
The current closure contains 824 files. npm package resolution is recorded in
`apps/codex-workflow-surface/package-lock.json`.
`apps/codex-workflow-surface/vendor/.gitattributes` preserves vendored bytes across
Windows Git checkouts, so automatic newline conversion does not invalidate hashes.
The committed closure is the build input, not `D:/Project/multica` or the audit
checkout. It includes MyIssuesPage, AutopilotsPage, AgentsPage, their detail and
creation routes, shared UI/core imports, locale resources, styles and local assets.
The original PropertiesTab and its settings layout/color picker closure are included
for the secondary property catalog route inside My Issues.
Upstream apps, server, daemon, CLI and test suites are excluded.

`apps/codex-workflow-surface/scripts/vendor-upstream.mjs` regenerates that inventory
and closure from pinned Git objects, follows TypeScript imports and CSS imports,
and applies the modifications listed below. Tests verify derived hashes and that
the three top-level original pages remain byte-identical to the pinned source.

## CCP Modifications

- PropertiesTab accepts an explicit local catalog capability and explanatory copy
  from Core bootstrap, while preserving its original role gate when that prop is
  absent. It exposes catalog query failure/retry with two locale additions. Local
  member identity is unchanged; the three top-level page files remain original.

- API-client HTTP calls use the CCP local transport; no ambient auth headers,
  cookies or arbitrary remote URLs cross that boundary.
- Socket construction is blocked; the local host supplies invalidation/events.
- The realtime context is exported for that local provider, without upstream WS
  authentication or connection effects.
- Base UI and direct body portals target the workflow's Shadow DOM container.
- Durable UI storage keys are prefixed with `ccp.workflow.`.
- IssueDetailRoute forwards the host's existing leading-action slot to the detail,
  loading skeleton and not-found state so embedded task pages have a return path.
- Quick-create modal and runtime gate accept Core's verified native task-host
  capability instead of requiring a Multica daemon CLI version. Other runtimes
  retain the original version checks; no CLI version is synthesized.
- Shared model discovery reads the native runtime's authoritative-selector
  metadata and uses the existing runtime-managed state without daemon discovery
  or invented model IDs. Other runtimes retain the daemon discovery path.
- Upstream CSS token defaults apply to `:host` and the workflow surface.
- Runtime mirrors the host theme inside the shadow boundary; dark variants and
  tokens apply to both page content and contained portals.
- Host mount, navigation, auth bootstrap, error boundary, styles and attribution
  are CCP code outside the vendor directory, documented in the package UPSTREAM.md.

Modified upstream files retain source text/copyright and carry CCP notices. Exact
LICENSE and NOTICE copies exist both here and inside the vendor closure. The UI
retains Multica name, original icon, copyright and a locally embedded full-license
dialog. Installer inclusion is owned by the parent packaging workflow; this manifest
does not certify installer execution, commercial permission or live UI acceptance.
