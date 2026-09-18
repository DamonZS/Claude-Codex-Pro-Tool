const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");

const source = fs.readFileSync(path.join(__dirname, "../assets/inject/renderer-inject.js"), "utf8");
test("native conversation markers stay outside an open workflow", () => {
  const begin = source.indexOf("  function refreshConversationTimeline()");
  const finish = source.indexOf("  const conversationViewContentClasses", begin);
  let removed = 0;
  const context = {
    multicaWorkspaceState: { opened: true },
    claudeCodexProSettings: () => ({ conversationTimeline: true }),
    removeConversationTimeline: () => { removed++; },
    conversationTimelineQuestions: () => { throw new Error("Native thread should not be scanned"); },
  };
  vm.createContext(context);
  vm.runInContext(source.slice(begin, finish), context);
  context.refreshConversationTimeline();
  assert.equal(removed, 1);
});
const start = source.indexOf("  function multicaWorkspaceUnmountUpstreamSurface(");
const end = source.indexOf("  function multicaWorkspaceRenderContent(", start);
assert.ok(start > 0 && end > start);

function fixture() {
  const node = () => ({
    dataset: {}, children: [],
    appendChild(child) { this.children.push(child); },
    replaceChildren() { this.children = []; },
    setAttribute() {}, addEventListener() {}, remove() { this.removed = true; },
  });
  const calls = [];
  const runtime = {
    mount(container, options) { calls.push(["mount", container, options]); },
    unmount(container) { calls.push(["unmount", container]); },
  };
  const state = { entries: new Map(), bootstrap: { workspace: { id: "fixture", slug: "local" } } };
  const window = { __CODEX_WORKFLOW_SURFACE__: runtime, __claudeCodexProMulticaWorkspaceGeneration: 1 };
  const context = {
    window, multicaWorkspaceState: state, claudeCodexProMulticaWorkspaceGeneration: 1,
    multicaWorkspaceFeatureEnabled: () => true,
    multicaWorkspaceEl: node,
    multicaWorkspaceLoadBootstrap: async () => {},
    multicaWorkspaceActivateNativeThread: async (id) => calls.push(["open", id]),
    multicaWorkspaceRequest: (route, payload) => ({ promise: Promise.resolve({ route, payload }) }),
  };
  vm.createContext(context);
  vm.runInContext(source.slice(start, end), context);
  return { ...context, state, calls, content: node() };
}

test("all three routes reuse one upstream mount without handwritten fallback", () => {
  const f = fixture();
  for (const key of ["my-issues", "autopilots", "agents", "agents"]) {
    f.multicaWorkspaceRenderUpstreamSurface(f.content, { key });
  }
  assert.equal(f.content.children.length, 1);
  assert.equal(new Set(f.calls.map((call) => call[1])).size, 1);
  assert.deepEqual(f.calls.map((call) => call[2].route), ["my-issues", "autopilots", "agents", "agents"]);
  assert.equal(f.content.dataset.upstream, "true");
});

test("missing bundle or bootstrap stays an explicit state rather than old board", () => {
  const f = fixture();
  f.state.bootstrap = null;
  f.multicaWorkspaceRenderUpstreamSurface(f.content, { key: "my-issues" });
  assert.equal(f.calls.length, 0);
  assert.equal(f.content.children.length, 1);
  f.state.bootstrap = { workspace: { id: "fixture" } };
  delete f.window.__CODEX_WORKFLOW_SURFACE__;
  f.multicaWorkspaceRenderUpstreamSurface(f.content, { key: "agents" });
  assert.equal(f.calls.length, 0);
});

test("bridge rejects unrelated endpoints and stale generations; native open is explicit", async () => {
  const f = fixture();
  f.multicaWorkspaceRenderUpstreamSurface(f.content, { key: "agents" });
  const bridge = f.window.__CODEX_WORKFLOW_BRIDGE__;
  assert.equal((await bridge.postJson("/multica/workspace/query", { resource: "agents" })).route, "/multica/workspace/query");
  assert.equal((await bridge.postJson("/multica/workspace/command", { commandId: "retry", commandSignature: "fixture" })).route, "/multica/workspace/command");
  for (const path of ["/multica/workspace/reorder-statuses", "/multica/builder", "/multica/webhooks/provision", "/multica/webhooks/trigger", "/multica/webhooks/rotate", "/multica/webhooks/deliveries", "/multica/webhooks/delivery", "/multica/webhooks/replay", "/multica/webhooks/revoke", "/multica/agents/env", "/multica/issues/limit-usage", "/multica/autopilots/usage", "/multica/issues/preview-trigger", "/multica/quick-actions/render", "/multica/quick-actions/run"]) {
    assert.equal((await bridge.postJson(path, {})).route, path);
  }
  await assert.rejects(bridge.postJson("/multica/webhooks/ingress/fixture/trigger", {}), /workflow_request_invalid/);
  await assert.rejects(bridge.postJson("/settings/set", {}), /workflow_request_invalid/);
  await assert.rejects(bridge.postJson("https://external.invalid", {}), /workflow_request_invalid/);
  await bridge.openThread("fixture-thread");
  assert.equal(f.calls.at(-1)[0], "open");
  f.window.__claudeCodexProMulticaWorkspaceGeneration = 2;
  await assert.rejects(bridge.postJson("/multica/executions/create", {}), /workflow_generation_stale/);
  await assert.rejects(bridge.openThread("fixture-thread"), /workflow_generation_stale/);
});

test("internal navigation keeps detail path during refresh and ignores stale callbacks", () => {
  const f = fixture();
  f.multicaWorkspaceRenderUpstreamSurface(f.content, { key: "agents" });
  const navigate = f.calls.at(-1)[2].onNavigate;
  navigate("agents", "/local/agents/fixture-agent");
  f.multicaWorkspaceRenderUpstreamSurface(f.content, { key: f.state.route });
  assert.equal(f.calls.at(-1)[2].path, "/local/agents/fixture-agent");
  f.window.__claudeCodexProMulticaWorkspaceGeneration = 2;
  navigate("autopilots", "/local/autopilots");
  assert.equal(f.state.route, "agents");
});

test("replacing runtime unmounts the prior root and cleanup releases the current root", () => {
  const f = fixture();
  f.multicaWorkspaceRenderUpstreamSurface(f.content, { key: "my-issues" });
  const previous = f.state.upstreamSurface;
  f.window.__CODEX_WORKFLOW_SURFACE__ = { mount() {}, unmount() {} };
  f.multicaWorkspaceRenderUpstreamSurface(f.content, { key: "autopilots" });
  assert.ok(previous.removed);
  assert.equal(f.calls.at(-1)[0], "unmount");
  f.multicaWorkspaceUnmountUpstreamSurface();
  assert.equal(f.state.upstreamSurface, null);
  assert.equal(f.state.upstreamRuntime, null);
});

test("mount and unmount exceptions leave a retry state and release the container", () => {
  const f = fixture();
  f.window.__CODEX_WORKFLOW_SURFACE__ = {
    mount() { throw new Error("fixture mount failure"); },
    unmount() { throw new Error("fixture teardown failure"); },
  };
  f.multicaWorkspaceRenderUpstreamSurface(f.content, { key: "agents" });
  assert.equal(f.state.upstreamSurface, null);
  assert.equal(f.state.upstreamRuntime, null);
  assert.equal(f.content.children.length, 1);
  assert.equal(f.content.children[0].children.length, 1);
});
