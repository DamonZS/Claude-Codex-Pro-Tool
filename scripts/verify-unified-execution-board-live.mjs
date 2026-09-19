import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { pageStep as nativeStep } from "./verify-native-subtasks-live.mjs";

// Existing query/list endpoints and real UI controls only. JSON excludes
// titles, prompts, messages, bridge credentials and raw exception details;
// screenshots are cropped to native cards (nickname and execution metadata).
export async function boardStep(action, arg = {}) {
  const host = document.querySelector("#ccp-multica-workspace-root");
  const root = host?.shadowRoot;
  const surface = root?.querySelector(".ccp-upstream-workflow-container");
  const rect = (element) => {
    if (!element?.getClientRects().length) return null;
    const r = element.getBoundingClientRect();
    const x = Math.max(0, r.x), y = Math.max(0, r.y);
    const width = Math.min(innerWidth, r.right) - x, height = Math.min(innerHeight, r.bottom) - y;
    return width > 0 && height > 0 ? { x, y, width, height } : null;
  };
  if (action === "sources") {
    const bridge = window.__CODEX_WORKFLOW_BRIDGE__;
    if (!bridge?.postJson) throw new Error("bridge_missing");
    const native = [], executions = [], issues = [];
    const deadline = Date.now() + 20000;
    const checkPage = (result, rows, total, cap) => {
      if (Date.now() > deadline) throw new Error("source_deadline");
      if (!Array.isArray(result.items) || result.stale || !Number.isSafeInteger(result.total) || result.total > cap
        || total !== undefined && result.total !== total || rows.length + result.items.length > result.total)
        throw new Error("source_snapshot_invalid");
      if (rows.length + result.items.length < result.total && result.items.length !== 100) throw new Error("source_incomplete");
      return result.total;
    };
    let workspaceId;
    let total;
    for (let offset = 0; offset < 500; offset += 100) {
      const result = await bridge.postJson("/multica/workspace/query", { resource: "codex_native_agents", limit: 100, offset });
      total = checkPage(result, native, total, 500);
      workspaceId = result.workspaceId;
      native.push(...result.items.map((item) => ({ id: item.id, parent: item.parent_thread_id, state: item.status })));
      if (native.length >= result.total) break;
      if (result.items.length !== 100) throw new Error("native_source_incomplete");
    }
    if (!workspaceId) throw new Error("workspace_missing");
    total = undefined;
    for (let offset = 0; offset < 5000; offset += 100) {
      const result = await bridge.postJson("/multica/executions/list", { workspaceId, limit: 100, offset });
      total = checkPage(result, executions, total, 5000);
      executions.push(...result.items.map((item) => ({ id: item.bindingId, issue: item.issueId, thread: item.codexThreadId, nativeResume: item.nativeResume === true, state: item.state, created: item.createdAtMs, attempt: item.attemptNo })));
      if (executions.length >= result.total) break;
      if (result.items.length !== 100) throw new Error("execution_source_incomplete");
    }
    total = undefined;
    for (let offset = 0; offset < 5000; offset += 100) {
      const result = await bridge.postJson("/multica/workspace/query", { resource: "issues", limit: 100, offset });
      total = checkPage(result, issues, total, 5000);
      issues.push(...result.items.map((item) => ({ id: item.id, thread: item.metadata?.ccp_thread_id, binding: item.metadata?.ccp_binding_id })));
      if (issues.length >= result.total) break;
    }
    for (const rows of [native, executions, issues]) if (new Set(rows.map((row) => row.id)).size !== rows.length) throw new Error("source_duplicate");
    return { native, executions, issues };
  }
  if (action === "state") return {
    generation: window.__claudeCodexProMulticaWorkspaceGeneration,
    surface: rect(surface), oldRegion: !!root?.querySelector('section[aria-label="Codex 原生子任务"]'),
    alerts: Array.from(root?.querySelectorAll('[role="alert"]') || []).filter(rect).length,
    heading: surface?.querySelector("h1")?.textContent?.trim(),
    detail: !!surface?.querySelector('section[aria-label="执行详情"]'),
    cards: Array.from(surface?.querySelectorAll("[data-board-card][data-ccp-issue-id]") || []).map((card) => ({
      id: card.getAttribute("data-ccp-issue-id"), state: card.getAttribute("data-ccp-execution-state"),
      source: card.getAttribute("data-ccp-source"), column: card.closest("[data-ccp-status-category]")?.getAttribute("data-ccp-status-category"),
      draggable: card.getAttribute("aria-roledescription") === "sortable", rect: rect(card),
    })),
  };
  let control;
  if (action === "entry") control = document.querySelector('[data-ccp-multica-nav-route="my-issues"]');
  if (action === "card") control = Array.from(surface?.querySelectorAll("[data-board-card][data-ccp-issue-id]") || [])
    .find((card) => card.getAttribute("data-ccp-issue-id") === arg.id)?.querySelector("a");
  if (action === "column") control = Array.from(surface?.querySelectorAll("[data-ccp-status-category]") || [])
    .find((column) => column.getAttribute("data-ccp-status-category") === arg.category);
  if (action === "back") control = surface?.querySelector('button[aria-label="返回我的任务"]');
  if (action === "child" || action === "parent") control = Array.from(surface?.querySelectorAll("button") || [])
    .find((button) => button.textContent.trim() === (action === "child" ? "打开子会话" : "打开父会话"));
  if (!control || control.disabled) return null;
  control.scrollIntoView({ block: "center", inline: "center", behavior: "instant" });
  // rAF pauses in a background Codex window; keep this probe bounded there too.
  await new Promise((resolve) => setTimeout(resolve, 50));
  if (action === "card") control = Array.from(surface?.querySelectorAll("[data-board-card][data-ccp-issue-id]") || [])
    .find((card) => card.getAttribute("data-ccp-issue-id") === arg.id)?.querySelector("a");
  const box = rect(control);
  if (action === "column") return box;
  if (!box) return null;
  const x = box.x + box.width / 2, y = box.y + box.height / 2;
  const hit = control.getRootNode().elementFromPoint(x, y);
  return hit === control || control.contains(hit) ? box : null;
}

export function expectedColumn(state) {
  if (["binding_pending", "queued", "pending", "dispatched", "waiting_local_directory"].includes(state)) return "todo";
  if (["running", "inProgress", "cancel_pending"].includes(state)) return "in_progress";
  if (state === "completed") return "done";
  if (state === "cancelled") return "cancelled";
  return "blocked";
}

export function verifySourceCards(cards, sources, requiredIds = []) {
  const issueIds = new Set(sources.issues.map((issue) => issue.id));
  const claimedThreads = new Set(sources.executions.map((run) => run.thread).filter(Boolean));
  const latest = new Map();
  const ordered = [...sources.executions].sort((a, b) => (a.created - b.created) || (a.attempt || 0) - (b.attempt || 0) || a.id.localeCompare(b.id));
  const threadIssues = new Map(sources.issues.filter((issue) => issue.thread).map((issue) => [issue.thread, issue.id]));
  const bindingIssues = new Map(sources.issues.filter((issue) => issue.binding).map((issue) => [issue.binding, issue.id]));
  const directlyLinked = new Set();
  for (const run of ordered) if (issueIds.has(run.issue)) {
    directlyLinked.add(run.issue);
    if (run.thread) threadIssues.set(run.thread, run.issue);
    bindingIssues.set(run.id, run.issue);
  }
  for (const run of ordered) {
    const issue = issueIds.has(run.issue) ? run.issue : bindingIssues.get(run.id) || threadIssues.get(run.thread);
    if (!issueIds.has(run.issue) && directlyLinked.has(issue)) continue;
    latest.set(issue || run.thread || run.id, { ...run, projected: issue || (run.nativeResume ? `codex-native:${run.thread}` : `ccp-execution:${run.id}`) });
  }
  const expected = new Map([...latest.values()].map((run) => [run.projected, run.state]));
  for (const row of sources.native) {
    const associated = threadIssues.get(row.id);
    if (associated && !expected.has(associated)) expected.set(associated, row.state);
    else if (!associated && !claimedThreads.has(row.id)) expected.set(`codex-native:${row.id}`, row.state);
  }
  for (const id of requiredIds) assert.ok(cards.some((card) => card.id === id), "required_source_card_missing");
  for (const card of cards.filter((card) => card.source)) {
    assert.ok(expected.has(card.id), "unexpected_projected_card");
    assert.equal(card.state, expected.get(card.id), "source_state_mismatch");
    assert.equal(card.column, expectedColumn(expected.get(card.id)), "source_column_mismatch");
  }
  return { projectedTotal: expected.size, renderedChecked: cards.filter((card) => card.source).length };
}

async function main() {
  assert.ok(process.argv.includes("--run-live"), "Use --run-live after the final Release repair");
  const parent = process.argv.find((arg) => arg.startsWith("--parent="))?.slice(9);
  assert.match(parent || "", /^[a-f0-9-]{36}$/, "Provide --parent=<current parent thread ID>");
  const targets = await (await fetch("http://127.0.0.1:9230/json", { signal: AbortSignal.timeout(5000) })).json();
  const pages = targets.filter((item) => item.type === "page" && item.url === "app://-/index.html");
  assert.equal(pages.length, 1);
  const socket = new WebSocket(pages[0].webSocketDebuggerUrl);
  const pending = new Map();
  let sequence = 0;
  const evidence = fileURLToPath(new URL(`../acceptance/evidence/unified-execution-board/${new Date().toISOString().replace(/[:.]/g, "-")}/`, import.meta.url));
  const report = { ok: false, restored: false, checks: [] };
  let originalTabs = [], childTab, childId, ready = false;
  try {
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("connect_timeout")), 5000);
      socket.onopen = () => { clearTimeout(timer); resolve(); };
      socket.onerror = () => { clearTimeout(timer); reject(new Error("connect_failed")); };
    });
    socket.onmessage = ({ data }) => {
      const message = JSON.parse(data), entry = pending.get(message.id);
      if (!entry) return;
      pending.delete(message.id); clearTimeout(entry.timer);
      message.error ? entry.reject(new Error("cdp_failed")) : entry.resolve(message.result);
    };
    const request = (method, params) => new Promise((resolve, reject) => {
      const id = ++sequence, timer = setTimeout(() => { pending.delete(id); reject(new Error("cdp_timeout")); }, 30000);
      pending.set(id, { resolve, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
    });
    const evaluate = async (fn, action, arg = {}) => {
      const result = await request("Runtime.evaluate", { expression: `(${fn.toString()})(${JSON.stringify(action)},${JSON.stringify(arg)})`, awaitPromise: true, returnByValue: true });
      assert.ok(!result.exceptionDetails, "page_step_failed"); return result.result?.value;
    };
    const wait = async (read, accept) => {
      const until = Date.now() + 15000;
      while (Date.now() < until) { const result = await read(); if (accept(result)) return result; await new Promise((r) => setTimeout(r, 250)); }
      throw new Error("state_timeout");
    };
    const click = async (fn, action, arg) => {
      report.stage = `click:${action}`;
      const r = await wait(() => evaluate(fn, action, arg), Boolean);
      const point = { x: r.x + r.width / 2, y: r.y + r.height / 2, button: "left", clickCount: 1 };
      await request("Input.dispatchMouseEvent", { type: "mousePressed", ...point });
      await request("Input.dispatchMouseEvent", { type: "mouseReleased", ...point });
    };
    const state = () => evaluate(boardStep, "state");
    const openBoard = async () => {
      await click(boardStep, "entry");
      if ((await state()).detail) await click(boardStep, "back");
      return wait(state, (s) => s.cards.length > 0 && s.surface?.width > 100);
    };
    await mkdir(evidence, { recursive: true });
    assert.equal((await evaluate(nativeStep, "active", { id: parent })).active, true, "parent_not_active");
    originalTabs = await evaluate(nativeStep, "tabs"); ready = true;
    try {
      await openBoard();
      report.stage = "sources";
      const sources = await evaluate(boardStep, "sources");
      report.sources = sources;
      const s = await state();
      report.stage = "source_comparison";
      assert.equal(s.oldRegion, false); assert.equal(s.alerts, 0);
      const projected = s.cards.filter((card) => card.source);
      assert.ok(projected.some((card) => card.source === "codex-native"), "native_cards_missing");
      assert.equal(new Set(s.cards.map((card) => card.id)).size, s.cards.length, "duplicate_cards");
      for (const card of projected) assert.equal(card.column, expectedColumn(card.state), "wrong_column");
      const required = sources.native.filter((item) => item.parent === parent && !sources.executions.some((run) => run.thread === item.id)).map((item) => `codex-native:${item.id}`);
      assert.ok(required.length, "current_parent_children_missing");
      report.sourceComparison = verifySourceCards(s.cards, sources, required);
      report.checks.push({ step: "board", ...s });
      for (const category of [...new Set(projected.filter((card) => card.source === "codex-native").map((card) => card.column))]) {
        const selected = projected.find((card) => card.source === "codex-native" && card.column === category);
        await evaluate(boardStep, "card", { id: selected.id });
        const snapshot = await state();
        const clip = snapshot.cards.find((card) => card.id === selected.id)?.rect;
        assert.ok(clip, "native_card_geometry");
        const capture = await request("Page.captureScreenshot", { format: "png", clip: { ...clip, scale: 1 } });
        await writeFile(path.join(evidence, `${category}.png`), Buffer.from(capture.data, "base64"));
      }
      const child = sources.native.find((item) => item.parent === parent && projected.some((card) => card.id === `codex-native:${item.id}`));
      assert.ok(child, "current_parent_child_missing");
      childId = child.id;
      const id = `codex-native:${child.id}`;
      await click(boardStep, "card", { id });
      await wait(state, (s) => s.detail && !s.alerts);
      await click(boardStep, "back"); await wait(state, (s) => !s.detail && s.cards.length > 0);
      await click(boardStep, "card", { id }); await wait(state, (s) => s.detail);
      await click(boardStep, "child");
      const childState = await wait(() => evaluate(nativeStep, "active", { id: child.id }), (s) => s.active && !s.overlayVisible);
      childTab = childState.nativeTabId;
      report.checks.push({ step: "child", id: child.id, ...childState });
      await openBoard(); await click(boardStep, "card", { id }); await wait(state, (s) => s.detail);
      await click(boardStep, "parent");
      report.checks.push({ step: "parent", ...await wait(() => evaluate(nativeStep, "active", { id: parent }), (s) => s.active && !s.overlayVisible) });
      report.checksComplete = true;
    } catch (error) {
      report.failedStage = report.stage;
      report.failure = error instanceof assert.AssertionError ? error.message.split("\n")[0] : error.message;
      throw error;
    } finally {
      if (ready) {
        if (!childTab && childId) childTab = (await evaluate(nativeStep, "active", { id: childId })).nativeTabId;
        if (childTab && !originalTabs.includes(childTab)) {
          await click(nativeStep, "locate", { kind: "close-panel", id: childTab });
          await wait(() => evaluate(nativeStep, "tabs"), (tabs) => !tabs.includes(childTab));
        }
        const active = await evaluate(nativeStep, "active", { id: parent });
        if (!active.active || active.overlayVisible) await click(nativeStep, "locate", { kind: "native", id: parent });
        await wait(() => evaluate(nativeStep, "active", { id: parent }), (s) => s.active && !s.overlayVisible);
        report.restored = true;
        report.ok = report.checksComplete === true;
      }
    }
  } finally {
    await mkdir(evidence, { recursive: true });
    await writeFile(path.join(evidence, "report.json"), JSON.stringify(report, null, 2));
    for (const entry of pending.values()) clearTimeout(entry.timer);
    socket.close(); console.log(JSON.stringify({ ok: report.ok, restored: report.restored, evidence }));
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => { console.error(error instanceof assert.AssertionError ? error.message.split("\n")[0] : "Live board verification failed; inspect bounded report."); process.exitCode = 1; });
}
