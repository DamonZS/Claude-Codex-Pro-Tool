# Pinned Multica Workflow Surface

Upstream: https://github.com/multica-ai/multica
Revision: `9fce92f427694d7d303258aa281b05c902a95ba9`.

`vendor/multica/source-files.json` records every included source path, original
SHA-256, derived SHA-256 and modification. The three list pages and their detail
and form components are original upstream React components, not replacements.
The closure includes their static imports (including barrel dependencies),
Chinese/English locale resources, editor CSS imports, and referenced assets.
It excludes upstream apps, server, daemon, CLI, fixtures and test suites.

## Reproduction

`npm run vendor -- <upstream-git-checkout>` reads Git objects at the pinned commit,
not mutable working-tree files. The default audit input is `.upstream-multica` at
the CCP root. Normal install/check/test/build only uses this committed closure;
neither a local upstream checkout nor a network source download is required.
Use `npm ci --legacy-peer-deps` for the committed dependency lockfile.

The generator applies narrowly documented host changes: route HTTP through the
local adapter, block socket constructors, export the realtime context for local
events, contain portals, namespace preference storage and scope CSS tokens to the
Shadow DOM. Original copyright text is retained; modified files have CCP notices.

The pinned PropertiesTab closure is mounted as a secondary My Issues route at
`/{workspace}/my-issues/properties`. Its optional local capability and explanatory
copy come from Core bootstrap, preserving membership identity and the original
role policy outside this host. Catalog query failures expose a retry; every
derived file and the two added translation entries are recorded in the manifest.

## Host Interface

`dist/codex-workflow-surface.js` is a self-contained IIFE. It exposes:

```ts
window.__CODEX_WORKFLOW_SURFACE__.mount(container, {
  route: "my-issues" | "autopilots" | "agents",
  workspaceSlug, workspaceId,
  path?,
  openThread?: async (threadId) => {},
  onNavigate?: (route, path) => {},
});
runtime.navigate(container, { route, path? });
runtime.invalidate(container);
runtime.event(container, { type, payload });
runtime.unmount(container);
runtime.dispose();
```

The host installs `window.__CODEX_WORKFLOW_BRIDGE__ = { postJson, openThread? }`
before mounting. The package instantiates `MulticaApiAdapter(bridge)` and calls
`transport(path, init): Promise<Response>`. All API errors remain errors. Host
bootstrap supplies the current workspace identity; `/api/me` and `/api/workspaces`
must agree with it before pages mount. No upstream AuthInitializer, CoreProvider,
WSProvider, telemetry reporter, browser fetch or socket fallback is started.

Repeated mount with the same workspace and route preserves the root, Query cache,
forms and internal navigation. Only explicit path/route changes navigate. The
surface owns detail/create navigation even without onNavigate; that callback lets
the renderer preserve deep links. Upstream singleton stores allow one active host.
Unmount cancels cached queries, removes event subscribers and disconnects transport.
The chat store is created once per bundle, matching upstream's lifetime callback
registry. Internal navigation and back both update the mount route/path and notify
the parent callback. Host transport preserves retry identity and only forwards
content-type, accept, idempotency-key, x-request-id and x-workspace-slug headers.

The parent injects `window.__CODEX_WORKFLOW_STYLES__` into its ShadowRoot once.
The string contains the complete `dist/codex-workflow-surface.css`, including
editor styles and bundled font data. Mount does not insert duplicate styles.
The parent owns that global; the IIFE does not redefine it or embed a second CSS copy.
Token defaults target the host and surface; runtime mirrors the host light/dark
theme onto the dedicated container and releases theme observers on unmount.
The dedicated container is also the portal target. Native Codex navigation and
React state are not changed by this package.

Multica name, original icon, copyright and a local LICENSE/NOTICE dialog remain
visible. Full license texts are embedded in the JS and retained in
`docs/third-party/multica/` for packaging. Commercial distribution conditions are
independent of this source/build record.

## Verification Scope

`npm run check` performs real TypeScript checking. `npm test` exercises the local
boundary, provenance and actual components in jsdom. `npm run build` produces the
fixed JS/CSS names. These checks are not a substitute for live Codex injection,
native execution, process-tree and configuration evidence required by acceptance.

Verified DOM actions include Issue creation, manual Agent creation, Autopilot
creation with the nested Agent picker and an enabled schedule trigger, and
Autopilot pause, plus route/back, repeated mount, attribution and theme
synchronization. Mutation fixtures implement the command receipt ledger and
revision checks used by the local adapter.

The nested picker regression exposed recursive native-top-layer selector matching
in jsdom/NWSAPI, not a demonstrated production focus loop. Test setup supplies
jsdom's missing native top-layer states for Floating UI measurements. All 13 real
page tests remain enabled. Live Shadow DOM picker validation remains required
alongside the browser evidence above; fixture coverage is not full CRUD acceptance.
