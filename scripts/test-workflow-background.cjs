const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");

const source = fs.readFileSync(path.join(__dirname, "../assets/inject/renderer-inject.js"), "utf8");
const start = source.indexOf("  function multicaWorkspaceStartBackgroundSync(");
const end = source.indexOf("  function multicaWorkspaceScheduleAnchorRetry(", start);
assert.ok(start > 0 && end > start);

function fixture() {
  const timers = new Map();
  let sequence = 0;
  const state = {};
  const window = { __claudeCodexProMulticaWorkspaceGeneration: 1 };
  const context = {
    window, multicaWorkspaceState: state,
    claudeCodexProMulticaWorkspaceGeneration: 1,
    multicaWorkspaceBackgroundIntervalMs: 1000,
    multicaWorkspaceBackgroundSync: async () => { throw new Error("fixture disconnected"); },
    setTimeout: (callback) => { const id = ++sequence; timers.set(id, callback); return id; },
    clearTimeout: (id) => timers.delete(id),
  };
  vm.createContext(context);
  vm.runInContext(source.slice(start, end), context);
  const tick = async () => {
    const [id, callback] = timers.entries().next().value;
    timers.delete(id);
    await callback();
  };
  return { ...context, state, timers, tick };
}

test("transient sync error schedules a retry instead of terminating the worker", async () => {
  const f = fixture();
  f.multicaWorkspaceStartBackgroundSync();
  f.multicaWorkspaceStartBackgroundSync();
  assert.equal(f.timers.size, 1);
  await f.tick();
  assert.equal(f.timers.size, 1);
  assert.ok(f.state.executionsError);
  f.multicaWorkspaceStopBackgroundSync();
  assert.equal(f.timers.size, 0);
});

test("old generation does not restart its worker after an in-flight rejection", async () => {
  const f = fixture();
  f.multicaWorkspaceStartBackgroundSync();
  f.window.__claudeCodexProMulticaWorkspaceGeneration = 2;
  await f.tick();
  assert.equal(f.timers.size, 0);
  assert.equal(f.state.executionsError, undefined);
});

test("scheduler outage does not block existing execution updates or query invalidation", async () => {
  const calls = [];
  const surface = {};
  const context = {
    window: { __claudeCodexProMulticaWorkspaceGeneration: 1 },
    claudeCodexProMulticaWorkspaceGeneration: 1,
    multicaWorkspaceBackgroundTimeoutMs: 15000,
    multicaWorkspaceFeatureEnabled: () => true,
    moduleForMulticaWorkspace: (route) => route,
    multicaWorkspaceQuery: async (route) => calls.push(route),
    multicaWorkspaceCall: async (path, payload) => {
      assert.equal(path, "/multica/autopilots/tick");
      assert.equal(Object.keys(payload).length, 0);
      calls.push("tick");
      throw new Error("fixture schedule error");
    },
    multicaWorkspaceErrorMessage: (error) => error.message,
    multicaWorkspaceLoadExecutions: async () => calls.push("list"),
    multicaWorkspaceDispatchQueuedAssignments: async () => calls.push("dispatch"),
    multicaWorkspaceSyncExecutionStatuses: async () => calls.push("status"),
    multicaWorkspaceState: {
      workspaceId: "fixture", upstreamSurface: surface,
      upstreamRuntime: { invalidate: (node) => { assert.equal(node, surface); calls.push("invalidate"); } },
    },
  };
  vm.createContext(context);
  const syncStart = source.indexOf("  async function multicaWorkspaceBackgroundSync(");
  vm.runInContext(source.slice(syncStart, start), context);
  await context.multicaWorkspaceBackgroundSync();
  assert.deepEqual(calls, ["issues", "my-issues", "tick", "list", "dispatch", "status", "invalidate"]);
  assert.equal(context.multicaWorkspaceState.backgroundBusy, false);
});

test("queue dispatcher resumes marked native bindings and preserves ordinary assignment guards", async () => {
  const calls = [];
  const bindings = [
    { bindingId: "native", nativeResume: true, codexThreadId: "child", state: "queued", revision: 2 },
    { bindingId: "ordinary", agentId: "agent", state: "binding_pending", revision: 1 },
    { bindingId: "unmarked", codexThreadId: "child", state: "queued", revision: 1 },
    { bindingId: "running", nativeResume: true, codexThreadId: "child", codexExecutionId: "turn", state: "running", revision: 3 },
  ];
  const context = {
    window: { __claudeCodexProMulticaWorkspaceGeneration: 1 },
    claudeCodexProMulticaWorkspaceGeneration: 1,
    multicaWorkspaceQueueDispatchIntervalMs: 5000,
    multicaWorkspaceFeatureEnabled: () => true,
    multicaWorkspaceState: { workspaceId: "ws", executions: bindings, executionBusy: new Set() },
    multicaWorkspaceObjectValue: (o, ...keys) => keys.map(k => o[k]).find(v => v !== undefined),
    multicaWorkspaceExecutionState: b => b.state,
    multicaWorkspaceExecutionBindingId: b => b.bindingId,
    multicaWorkspaceEntityRevision: b => b.revision,
    multicaWorkspaceCall: async (route, data) => { calls.push([route, data.bindingId]); return {}; },
    multicaWorkspaceErrorMessage: e => e.message,
  };
  vm.createContext(context);
  const begin = source.indexOf("  async function multicaWorkspaceDispatchQueuedAssignments(");
  const end = source.indexOf("  function multicaWorkspaceOpenExecutionDraft(", begin);
  vm.runInContext(source.slice(begin, end), context);
  await context.multicaWorkspaceDispatchQueuedAssignments(true);
  assert.deepEqual(calls, [["/multica/executions/dispatch", "native"], ["/multica/executions/dispatch", "ordinary"]]);
});
