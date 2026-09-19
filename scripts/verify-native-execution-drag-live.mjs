import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { boardStep } from "./verify-unified-execution-board-live.mjs";
import { pageStep } from "./verify-native-subtasks-live.mjs";

// Explicit opt-in: this verifier starts two real turns on a dedicated fixture
// child. It never resumes an arbitrary business task or records message text.
assert.ok(process.argv.includes("--run-live"));
const threadId = process.argv.find(a => a.startsWith("--thread="))?.slice(9);
assert.match(threadId || "", /^[a-f0-9-]{36}$/);
const targets = await (await fetch("http://127.0.0.1:9230/json")).json();
const page = targets.find(t => t.type === "page" && t.url === "app://-/index.html");
assert.ok(page);
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
let sequence = 0;
const pending = new Map();
socket.onmessage = ({ data }) => {
  const value = JSON.parse(data), entry = pending.get(value.id);
  if (!entry) return;
  pending.delete(value.id); clearTimeout(entry.timer);
  value.error ? entry.reject(new Error("cdp_error")) : entry.resolve(value.result);
};
const request = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++sequence, timer = setTimeout(() => { pending.delete(id); reject(new Error("cdp_timeout")); }, 30000);
  pending.set(id, { resolve, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
});
const evaluate = async (fn, arg) => {
  const result = await request("Runtime.evaluate", { expression: `(${fn.toString()})(${JSON.stringify(arg)})`, returnByValue: true, awaitPromise: true });
  assert.ok(!result.exceptionDetails, "page_evaluation_failed"); return result.result?.value;
};
const step = (action, arg = {}) => evaluate(async ({ fn, action, arg }) => (0, eval)(`(${fn})`)(action, arg), { fn: boardStep.toString(), action, arg });
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const wait = async (read, accept, label, timeout = 60000) => {
  const until = Date.now() + timeout;
  while (Date.now() < until) { const value = await read(); if (accept(value)) return value; await pause(500); }
  throw new Error(label);
};
const click = async box => {
  assert.ok(box, "control_missing");
  const point = { x: box.x + box.width / 2, y: box.y + box.height / 2, button: "left", clickCount: 1 };
  await request("Input.dispatchMouseEvent", { type: "mousePressed", ...point });
  await request("Input.dispatchMouseEvent", { type: "mouseReleased", ...point });
};
const readThread = id => evaluate(async id => {
  const result = await window.__claudeCodexProCodexPageHostRequest("thread/read", { threadId: id, includeTurns: true });
  const thread = result.thread;
  return { id: thread.id, parent: thread.parentThreadId,
    fixture: thread.turns?.some(turn => turn.items?.some(item => item.type === "userMessage" && JSON.stringify(item.content).includes("专用验收子任务"))),
    turns: thread.turns?.map(turn => ({ id: turn.id, status: turn.status,
      marker: turn.items?.some(item => item.type === "agentMessage" && item.text?.trim() === "CCP_NATIVE_DRAG_OK") })) };
}, id);
let workspaceId;
const readBinding = id => evaluate(async ({ id, workspaceId }) => {
  const result = await window.__CODEX_WORKFLOW_BRIDGE__.postJson("/multica/executions/list", { workspaceId, limit: 100, offset: 0 });
  const b = result.items?.find(item => item.codexThreadId === id);
  return b ? { id: b.bindingId, thread: b.codexThreadId, turn: b.codexExecutionId, state: b.state, nativeResume: b.nativeResume } : null;
}, { id, workspaceId });
const report = { ok: false, restored: false, threadId, checks: [] };
const evidence = fileURLToPath(new URL(`../acceptance/evidence/unified-execution-board/drag-${new Date().toISOString().replace(/[:.]/g, "-")}/`, import.meta.url));
let original;
try {
  const before = await readThread(threadId);
  assert.equal(before.fixture, true, "dedicated_fixture_required");
  assert.ok(before.parent);
  assert.equal(before.turns.at(-1)?.status, "completed");
  assert.equal(before.turns.at(-1)?.marker, true);
  const parentBefore = await readThread(before.parent);
  original = await evaluate(() => [...document.querySelectorAll('[data-app-action-sidebar-thread-active="true"]')].map(e => e.getAttribute("data-app-action-sidebar-thread-id")).find(Boolean));
  assert.ok(original);
  await click(await step("entry"));
  if ((await step("state")).detail) await click(await step("back"));
  workspaceId = await evaluate(async () => (await window.__CODEX_WORKFLOW_BRIDGE__.postJson("/multica/workspace/query", { resource: "codex_native_agents", limit: 1, offset: 0 })).workspaceId);
  assert.ok(workspaceId);
  const id = `codex-native:${threadId}`;
  await wait(() => step("state"), s => s.cards.some(card => card.id === id), "fixture_card_missing");
  let previous = before;
  for (const category of ["in_progress", "todo"]) {
    report.stage = `drag:${category}`;
    const source = await wait(() => step("card", { id }), Boolean, "fixture_not_visible", 15000);
    const start = { x: source.x + source.width / 2, y: source.y + source.height / 2 };
    await request("Input.dispatchMouseEvent", { type: "mouseMoved", ...start });
    await request("Input.dispatchMouseEvent", { type: "mousePressed", ...start, button: "left", buttons: 1, clickCount: 1 });
    try {
      await request("Input.dispatchMouseEvent", { type: "mouseMoved", x: start.x - 12, y: start.y + 10, buttons: 1 });
      await pause(200);
      const dest = await step("column", { category });
      assert.ok(dest, "target_column_missing");
      const end = { x: dest.x + dest.width / 2, y: dest.y + Math.min(45, dest.height / 2) };
      await request("Input.dispatchMouseEvent", { type: "mouseMoved", ...end, buttons: 1 });
      await pause(300);
      await request("Input.dispatchMouseEvent", { type: "mouseReleased", ...end, button: "left", buttons: 0, clickCount: 1 });
    } catch (error) {
      await request("Input.dispatchMouseEvent", { type: "mouseReleased", ...start, button: "left", buttons: 0, clickCount: 1 });
      throw error;
    }
    const observed = [];
    const result = await wait(async () => {
      const binding = await readBinding(threadId);
      if (binding && observed.at(-1) !== binding.state) observed.push(binding.state);
      const thread = await readThread(threadId);
      return { binding, thread };
    }, value => value.thread.turns.length > previous.turns.length && value.thread.turns.at(-1).status === "completed" && value.binding?.state === "completed", "execution_completion_timeout", 120000);
    assert.equal(result.thread.turns.length, previous.turns.length + 1, "duplicate_turn");
    assert.equal(result.thread.turns.at(-1).marker, true);
    assert.equal(result.binding.thread, threadId);
    assert.equal(result.binding.nativeResume, true);
    const state = await wait(() => step("state"), s => s.cards.some(card => card.id === id && card.state === "completed" && card.column === "done"), "completion_projection_timeout");
    assert.equal(state.cards.filter(card => card.id === id).length, 1);
    report.checks.push({ category, states: observed, binding: result.binding, beforeTurnCount: previous.turns.length, afterTurnCount: result.thread.turns.length, marker: true });
    console.log(JSON.stringify({ category, states: observed, oneTurn: true, completedColumn: true }));
    previous = result.thread;
  }
  const parentAfter = await readThread(before.parent);
  assert.deepEqual(parentAfter.turns.map(t => t.id), parentBefore.turns.map(t => t.id), "parent_received_new_turn");
  report.parentTurnsUnchanged = true;
  report.ok = true;
} catch (error) {
  report.failure = error instanceof assert.AssertionError ? error.message.split("\n")[0] : error.message;
  process.exitCode = 1;
} finally {
  if (original) {
    try {
      const box = await evaluate(({ fn, id }) => (0, eval)(`(${fn})`)("locate", { kind: "native", id }), { fn: pageStep.toString(), id: original });
      await click(box);
      await wait(() => evaluate(({ fn, id }) => (0, eval)(`(${fn})`)("active", { id }), { fn: pageStep.toString(), id: original }), s => s.active && !s.overlayVisible, "restore_timeout", 15000);
      report.restored = true;
    } catch { report.ok = false; report.restoreFailed = true; process.exitCode = 1; }
  }
  await mkdir(evidence, { recursive: true });
  await writeFile(`${evidence}/report.json`, JSON.stringify(report, null, 2));
  for (const entry of pending.values()) clearTimeout(entry.timer);
  socket.close();
  console.log(JSON.stringify({ ok: report.ok, restored: report.restored, stage: report.stage, failure: report.failure, evidence }));
}
