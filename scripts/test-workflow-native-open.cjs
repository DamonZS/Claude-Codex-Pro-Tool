const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");

const source = fs.readFileSync(path.join(__dirname, "../assets/inject/renderer-inject.js"), "utf8");
const start = source.indexOf("  function multicaWorkspaceNativeSubagentOpener(");
const end = source.indexOf("  async function multicaWorkspaceRunExecutionAction(", start);
assert.ok(start > 0 && end > start);

function fixture({ collapsed = false, mismatch = false, subagent = false, allowed = true,
  panelKind = "background-agent", selectedId = "thread-1", loading = false, selected = true, openerMissing = false } = {}) {
  const calls = [];
  let visible = !collapsed && !subagent;
  let active = false;
  let panelActive = false;
  let clock = 0;
  const row = { matches: () => true, click: () => { calls.push("thread-click"); active = true; } };
  const tabId = panelKind === "subagents" ? "subagents:parent-1" : `background-agent:${selectedId}`;
  const tab = {
    isConnected: true, getClientRects: () => [{}],
    closest: () => ({ getAttribute: () => tabId }),
    fiber: { memoizedProps: { tab: { tabId, durableRoute: { kind: panelKind }, props: { requestedConversationId: selectedId, isLoading: loading } } } },
  };
  const opener = {
    canOpen: (id) => { calls.push(["canOpen", id]); return allowed; },
    open: (id) => { calls.push(["native-open", id]); panelActive = selected; },
  };
  const button = { fiber: { memoizedProps: { backgroundAgentOpener: opener } } };
  const state = {};
  const context = {
    Date: { now: () => clock }, Promise, setTimeout: (callback, ms) => { clock += ms; queueMicrotask(callback); },
    document: { querySelectorAll: (selector) => selector.includes("multi-agent-action-rows")
      ? openerMissing ? [] : [button] : panelActive ? [tab] : [] },
    reactFiberFrom: (node) => node.fiber,
    multicaWorkspaceState: state,
    multicaWorkspaceNativeThreadRow: () => visible ? row : null,
    multicaWorkspaceThreadIdMatches: (actual, expected) => actual === expected,
    multicaWorkspaceNativeThreadIsActive: () => active,
    multicaWorkspaceCurrentNativeThreadId: () => active ? "thread-1" : "",
    multicaWorkspaceHide: () => { assert.ok(active || context.multicaWorkspaceNativeSubagentIsActive("thread-1")); calls.push("hide"); },
    normalizeWorkspacePath: (value) => String(value).toLowerCase(),
    codexPageHostRequest: async (method, params) => {
      calls.push([method, params]);
      return { thread: { id: mismatch ? "wrong-thread" : "thread-1", cwd: "D:/project" } };
    },
    nativeProjectTargets: () => [{
      path: "D:/project", row: {
        getAttribute: () => "true",
        click: () => { assert.equal(state.nativeThreadActivation, true); calls.push("project-expand"); visible = !subagent; },
      },
    }],
  };
  vm.createContext(context);
  vm.runInContext(source.slice(start, end), context);
  return { ...context, calls, state };
}

test("visible native row is activated before the workflow is hidden", async () => {
  const f = fixture();
  await f.multicaWorkspaceActivateNativeThread("thread-1");
  assert.deepEqual(f.calls, ["thread-click", "hide"]);
  assert.equal(f.state.nativeThreadActivation, false);
});

test("collapsed project is selected from authoritative thread metadata", async () => {
  const f = fixture({ collapsed: true });
  await f.multicaWorkspaceActivateNativeThread("thread-1");
  assert.equal(f.calls[0][0], "thread/read");
  assert.equal(f.calls[0][1].includeTurns, false);
  assert.deepEqual(f.calls.slice(1), ["project-expand", "thread-click", "hide"]);
  assert.equal(f.state.nativeThreadActivation, false);
});

test("mismatched native thread response does not navigate or hide", async () => {
  const f = fixture({ collapsed: true, mismatch: true });
  await assert.rejects(f.multicaWorkspaceActivateNativeThread("thread-1"));
  assert.equal(f.calls.length, 1);
  assert.equal(f.state.nativeThreadActivation, false);
});

test("subagent absent from sidebar uses the existing native opener and exact selected tab", async () => {
  const f = fixture({ subagent: true });
  await f.multicaWorkspaceActivateNativeThread("thread-1");
  assert.equal(f.calls[0][0], "thread/read");
  assert.equal(f.calls[0][1].includeTurns, false);
  assert.ok(f.calls.some((call) => call[0] === "native-open" && call[1] === "thread-1"));
  assert.equal(f.calls.at(-1), "hide");
  assert.equal(f.state.nativeThreadActivation, false);
  assert.equal(f.calls.includes("thread-click"), false);
});

test("loaded subagents overview confirms its requested child, not just the parent tab", async () => {
  const f = fixture({ subagent: true, panelKind: "subagents" });
  await f.multicaWorkspaceActivateNativeThread("thread-1");
  assert.equal(f.calls.at(-1), "hide");
});

for (const [name, options] of [
  ["native capability rejects the child", { allowed: false }],
  ["mounted opener is missing", { openerMissing: true }],
  ["another child tab is selected", { selectedId: "other-thread" }],
  ["native opener has not selected a tab", { selected: false }],
  ["subagents panel is still loading", { panelKind: "subagents", loading: true }],
  ["subagents panel requested another child", { panelKind: "subagents", selectedId: "other-thread" }],
]) test(`${name}: keep workflow visible and report failure`, async () => {
  const f = fixture({ subagent: true, ...options });
  await assert.rejects(f.multicaWorkspaceActivateNativeThread("thread-1"));
  assert.equal(f.calls.includes("hide"), false);
  assert.equal(f.state.nativeThreadActivation, false);
  if (options.allowed === false || options.openerMissing) assert.equal(f.calls.some((call) => call[0] === "native-open"), false);
});
