import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

// Fixed acceptance subjects, confirmed from read-only native parent/child metadata.
export const parentId = "01a0b2ac-7535-7481-a638-70d793805812";
export const subjects = [
  { id: "01a0b405-80ed-7662-b24d-5c22329d64ac", name: "Descartes" },
  { id: "01a0b405-81bf-7401-a1c2-18c0dddc5ff7", name: "Tesla" },
  { id: "01a0b405-8314-7503-8170-ff860eb0aa6c", name: "Faraday" },
  { id: "01a0b405-8422-7582-b796-4cff65cd2e8a", name: "Mendel" },
];

// Evaluated in the existing page; reads DOM and locates controls only. No injection,
// model calls, bridge replacement, route mutation, or database access.
export function pageStep(action, arg = {}) {
  const host = document.querySelector("#ccp-multica-workspace-root");
  const root = host?.shadowRoot;
  const region = root?.querySelector('section[aria-label="Codex 原生子任务"]');
  const matches = (value, id) => value === id || value?.endsWith(`:${id}`);
  const sidebarRow = (id) => Array.from(document.querySelectorAll("[data-app-action-sidebar-thread-id]"))
    .find((e) => matches(e.getAttribute("data-app-action-sidebar-thread-id"), id));
  const active = (e) => !!e && (e.matches('[data-app-action-sidebar-thread-active="true"], [aria-current="page"]')
    || !!e.querySelector('[data-app-action-sidebar-thread-active="true"], [aria-current="page"]'));
  const rect = (e) => {
    const r = e?.getBoundingClientRect();
    return r ? { x: r.x, y: r.y, width: r.width, height: r.height } : null;
  };
  const visible = (e) => !!e?.getClientRects().length && getComputedStyle(e).visibility !== "hidden";
  const selectedPanel = (id) => Array.from(document.querySelectorAll('[role="tab"][aria-selected="true"]')).find((tab) => {
    const tabId = tab.closest('[data-tab-id]')?.getAttribute('data-tab-id');
    if (tabId === `background-agent:${id}`) return true;
    if (!tabId?.startsWith('subagents:')) return false;
    let fiber = tab[Object.keys(tab).find((key) => key.startsWith('__reactFiber'))];
    for (let depth = 0; fiber && depth < 16; depth++, fiber = fiber.return) {
      const nativeTab = fiber.memoizedProps?.tab;
      if (nativeTab?.tabId === tabId && nativeTab.durableRoute?.kind === 'subagents'
        && nativeTab.props?.requestedConversationId === id && nativeTab.props?.isLoading === false) return true;
    }
    return false;
  });
  if (action === "tabs") return Array.from(document.querySelectorAll('[role="tab"]'))
    .map((tab) => tab.closest('[data-tab-id]')?.getAttribute('data-tab-id')).filter(Boolean);
  if (action === "state") return {
    bridge: typeof window.__codexSessionDeleteBridge === "function",
    generation: window.__claudeCodexProMulticaWorkspaceGeneration,
    heading: root?.querySelector("h1")?.textContent?.trim(),
    region: rect(region),
    alerts: Array.from(region?.querySelectorAll('[role="alert"]') || []).filter(visible).length,
    expanded: region?.querySelector("button[aria-expanded]")?.getAttribute("aria-expanded"),
    rows: arg.subjects.map(({ id }) => {
      const row = region?.querySelector(`[data-native-thread-id="${id}"]`);
      return row ? { id, name: row.querySelector(".ccp-native-subtask-name")?.textContent?.trim(),
        status: row.querySelector("span")?.textContent?.trim(),
        updatedAt: row.querySelector("time")?.getAttribute("datetime"), rect: rect(row) } : { id, missing: true };
    }),
  };
  if (action === "active") {
    const panel = selectedPanel(arg.id);
    return { active: active(sidebarRow(arg.id)) || !!panel, sidebarActive: active(sidebarRow(arg.id)),
      nativeTabId: panel?.closest('[data-tab-id]')?.getAttribute('data-tab-id') || null, overlayVisible: visible(host) };
  }
  let control;
  if (arg.kind === "entry") control = document.querySelector('[data-ccp-multica-nav-route="my-issues"]');
  if (arg.kind === "back") control = root?.querySelector('button[aria-label="返回我的任务"]');
  if (arg.kind === "expand") control = region?.querySelector('button[aria-expanded="false"]');
  if (arg.kind === "child" || arg.kind === "parent") {
    const row = region?.querySelector(`[data-native-thread-id="${arg.id}"]`);
    control = row?.querySelector(arg.kind === "child" ? ".ccp-native-subtask-name" : 'button[aria-label]');
  }
  if (arg.kind === "native") {
    const row = sidebarRow(arg.id);
    control = row?.matches('button, a[href], [role="button"], [role="link"]') ? row
      : row?.closest('button, a[href], [role="button"], [role="link"]') || row || selectedPanel(arg.id);
  }
  if (arg.kind === "close-panel") {
    const tab = Array.from(document.querySelectorAll('[role="tab"]'))
      .find((e) => e.closest('[data-tab-id]')?.getAttribute('data-tab-id') === arg.id);
    control = Array.from(tab?.closest('[data-tab-id]')?.querySelectorAll('button[aria-label]') || [])
      .find((e) => /^(关闭|Close\b)/i.test(e.getAttribute('aria-label')));
  }
  if (!visible(control) || control.disabled) return null;
  control.scrollIntoView({ block: "center", inline: "nearest", behavior: "instant" });
  const box = rect(control);
  const x = box.x + box.width / 2, y = box.y + box.height / 2;
  const hit = (control.getRootNode().elementFromPoint?.(x, y) || document.elementFromPoint(x, y));
  if (x < 0 || y < 0 || x >= innerWidth || y >= innerHeight || !hit || !(hit === control || control.contains(hit))) return null;
  return box;
}

export async function verify({ request, save, pause = (ms) => new Promise((r) => setTimeout(r, ms)), now = Date.now }) {
  const report = { parentId, subjects, checks: [], restored: false };
  let stage = "list";
  let deadline = now() + 120000;
  const evaluate = async (action, arg) => {
    assert.ok(now() < deadline, "Verification deadline reached");
    const result = await request("Runtime.evaluate", {
      expression: `(${pageStep.toString()})(${JSON.stringify(action)},${JSON.stringify(arg || {})})`,
      returnByValue: true,
    });
    assert.ok(!result.exceptionDetails, "Page inspection failed");
    return result.result?.value;
  };
  const waitFor = async (read, accept, label) => {
    const until = Math.min(deadline, now() + 12000);
    do {
      const value = await read();
      if (accept(value)) return value;
      await pause(250);
    } while (now() < until);
    throw new Error(label);
  };
  const click = async (kind, id, optional = false) => {
    const box = await evaluate("locate", { kind, id });
    if (!box && optional) return false;
    assert.ok(box, `Visible control required: ${kind}`);
    const point = { x: box.x + box.width / 2, y: box.y + box.height / 2, button: "left", clickCount: 1 };
    await request("Input.dispatchMouseEvent", { type: "mousePressed", ...point });
    await request("Input.dispatchMouseEvent", { type: "mouseReleased", ...point });
    return box;
  };
  const waitActive = (id) => waitFor(() => evaluate("active", { id }), (s) => s.active && !s.overlayVisible, "Native thread activation not confirmed");
  const state = () => evaluate("state", { subjects });
  const openList = async () => {
    await click("entry");
    await waitFor(async () => {
      await click("back", undefined, true);
      const s = await state();
      return s;
    }, (s) => ["我的任务", "My issues"].includes(s.heading) && s.region?.width > 100 && s.region?.height > 20, "Native subtasks region not visible");
    const s = await waitFor(async () => {
      await click("expand", undefined, true);
      return state();
    }, (s) => s.rows.every((row) => !row.missing), "Expected native thread IDs missing");
    assert.equal(s.alerts, 0, "Native subtasks has visible alerts");
    assert.equal(s.bridge, true, "Live bridge missing");
    for (const subject of subjects) {
      const row = s.rows.find((r) => r.id === subject.id);
      assert.equal(row.name, subject.name, "Native nickname mismatch");
      assert.ok(row.status && row.rect?.width > 0 && row.rect?.height > 0, "Native status or row geometry missing");
    }
    return s;
  };
  const capture = async (name, clip) => {
    assert.ok(clip?.width > 0 && clip?.height > 0, "Screenshot geometry missing");
    const result = await request("Page.captureScreenshot", { format: "png", clip: { ...clip, scale: 1 } });
    await save(`${name}.png`, Buffer.from(result.data, "base64"));
  };
  // Require the known parent before changing UI; all subsequent exits attempt restoration.
  assert.equal((await evaluate("active", { id: parentId })).active, true, "Start from the expected parent thread");
  const originalTabs = await evaluate("tabs");
  let failure;
  let childTabId;
  try {
    report.checks.push({ step: "list", ...await openList() });
    for (const subject of subjects) {
      // Scrolling each row into view also covers lists larger than the default four.
      assert.ok(await evaluate("locate", { kind: "child", id: subject.id }), "Expected child control is obscured");
      const s = await state();
      report.checks.push({ step: "row", ...s.rows.find((row) => row.id === subject.id), region: s.region });
      await capture(subject.name.toLowerCase(), s.region);
    }
    stage = "child";
    await click("child", subjects[0].id);
    const childState = await waitActive(subjects[0].id);
    childTabId = childState.nativeTabId;
    report.checks.push({ step: "child", id: subjects[0].id, ...childState });
    await capture("child-active", await evaluate("locate", { kind: "native", id: subjects[0].id }));
    stage = "parent";
    await openList();
    await click("parent", subjects[0].id);
    report.checks.push({ step: "parent", id: parentId, ...await waitActive(parentId) });
    await capture("parent-active", await evaluate("locate", { kind: "native", id: parentId }));
  } catch (error) {
    failure = error;
    report.failedStep = stage;
  } finally {
    deadline = now() + 15000;
    try {
      childTabId ||= (await evaluate("active", { id: subjects[0].id })).nativeTabId;
      if (childTabId && !originalTabs.includes(childTabId)) {
        await click("close-panel", childTabId);
        await waitFor(() => evaluate("tabs"), (tabs) => !tabs.includes(childTabId), "Temporary native tab did not close");
      }
      const s = await evaluate("active", { id: parentId });
      if (!s.active || s.overlayVisible) await click("native", parentId);
      await waitActive(parentId);
      report.restored = true;
    } catch {
      failure = new Error("Restore of parent thread not confirmed");
      report.restoreFailed = true;
    }
    report.ok = !failure;
    await save("report.json", JSON.stringify(report, null, 2));
  }
  if (failure) throw failure;
  return report;
}

async function main() {
  assert.equal(process.argv.length, 2, "Usage: node scripts/verify-native-subtasks-live.mjs (after Release repair)");
  const targets = await (await fetch("http://127.0.0.1:9230/json", { signal: AbortSignal.timeout(5000) })).json();
  const pages = targets.filter((t) => t.type === "page" && t.url === "app://-/index.html");
  assert.equal(pages.length, 1, "Exactly one current Codex page is required");
  const socket = new WebSocket(pages[0].webSocketDebuggerUrl);
  const pending = new Map();
  let sequence = 0;
  const evidence = fileURLToPath(new URL(`../acceptance/evidence/native-subtasks-live/${new Date().toISOString().replace(/[:.]/g, "-")}/`, import.meta.url));
  try {
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("CDP connection timeout")), 5000);
      socket.onopen = () => { clearTimeout(timer); resolve(); };
      socket.onerror = () => { clearTimeout(timer); reject(new Error("CDP connection failed")); };
    });
    socket.onmessage = ({ data }) => {
      const message = JSON.parse(data);
      const entry = pending.get(message.id);
      if (!entry) return;
      pending.delete(message.id);
      clearTimeout(entry.timer);
      if (message.error) entry.reject(new Error("CDP request failed"));
      else entry.resolve(message.result);
    };
    const request = (method, params) => new Promise((resolve, reject) => {
      const id = ++sequence;
      const timer = setTimeout(() => { pending.delete(id); reject(new Error("CDP request timeout")); }, 5000);
      pending.set(id, { resolve, reject, timer });
      socket.send(JSON.stringify({ id, method, params }));
    });
    await mkdir(evidence, { recursive: true });
    console.log(JSON.stringify({ evidence }));
    const report = await verify({ request, save: (name, data) => writeFile(path.join(evidence, name), data) });
    console.log(JSON.stringify({ ok: report.ok, restored: report.restored, evidence, checks: report.checks }));
  } finally {
    for (const entry of pending.values()) clearTimeout(entry.timer);
    socket.close();
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(() => { console.error("Native subtasks verification failed; inspect the bounded evidence report and confirm the parent thread."); process.exitCode = 1; });
}
