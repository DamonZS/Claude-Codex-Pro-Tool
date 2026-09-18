import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

export const cases = [
  { resource: "issues", route: "my-issues", back: "返回我的任务", headings: ["我的任务", "My issues"] },
  { resource: "autopilots", route: "autopilots", back: "返回自动化", headings: ["自动化", "Autopilots"] },
  { resource: "agents", route: "agents", back: "返回智能体", headings: ["智能体", "Agents"] },
];

export function failureCode(error) {
  const allowed = new Set([
    "bridge_missing", "bridge_timeout", "read_failed", "resource_invalid", "query_invalid",
    "surface_missing", "editing_active", "slug_invalid", "route_invalid", "action_invalid",
    "deadline", "page_step_failed", "visible_alert", "state_timeout", "capture_geometry",
    "fixture_missing", "return_control_obscured", "return_control_moved", "return_not_stable", "sidebar_obscured",
    "invalid_arguments", "discovery_failed", "target_count", "target_endpoint",
    "connect_timeout", "connect_failed", "cdp_failed", "cdp_timeout", "verification_failed",
  ]);
  for (const value of [error?.code, typeof error?.message === "string" ? error.message.split("\n")[0] : null]) {
    if (allowed.has(value)) return value;
  }
  return "verification_failed";
}

export function requestBudget(method, params, remaining = Infinity) {
  return Math.max(0, Math.min(remaining, method === "Runtime.evaluate" && params?.awaitPromise === true ? 45000 : 5000));
}

// Runs against existing public APIs and DOM only. Query contents never leave
// this function: exported evidence contains IDs, booleans and geometry only.
export async function pageStep(action, arg = {}) {
  const host = document.querySelector("#ccp-multica-workspace-root");
  const shadow = host?.shadowRoot;
  const surface = shadow?.querySelector(".ccp-upstream-workflow-container");
  const visible = (e) => !!e?.getClientRects().length && getComputedStyle(e).visibility !== "hidden"
    && getComputedStyle(e).display !== "none";
  const rect = (e) => {
    if (!visible(e)) return null;
    const r = e.getBoundingClientRect();
    const x = Math.max(0, r.x), y = Math.max(0, r.y);
    const width = Math.min(innerWidth, r.right) - x, height = Math.min(innerHeight, r.bottom) - y;
    return width > 0 && height > 0 ? { x, y, width, height } : null;
  };
  const read = async (route, payload) => {
    const bridge = window.__CODEX_WORKFLOW_BRIDGE__;
    if (!bridge?.postJson) throw new Error("bridge_missing");
    const value = await bridge.postJson(route, payload);
    if (value?.status !== "ok" || value.stale) throw new Error("read_failed");
    return value;
  };
  const query = async () => {
    if (!["issues", "autopilots", "agents"].includes(arg.resource)) throw new Error("resource_invalid");
    const result = await read("/multica/workspace/query", { resource: arg.resource, limit: 20, offset: 0 });
    if (!Array.isArray(result.items) || result.items.length > 20) throw new Error("query_invalid");
    return result.items;
  };
  const validId = (id) => typeof id === "string" && /^[A-Za-z0-9_-]{1,128}$/.test(id);
  if (action === "prepare") {
    if (!visible(host) || !rect(surface) || !window.__CODEX_WORKFLOW_SURFACE__?.navigate) throw new Error("surface_missing");
    // Avoid committing an in-progress editor merely by moving keyboard focus.
    const focused = shadow.activeElement || document.activeElement;
    if (focused?.matches('input, textarea, [contenteditable="true"]') || shadow.querySelector('[role="dialog"]')) throw new Error("editing_active");
    const result = await read("/multica/workspace/bootstrap", {});
    if (!validId(result.workspace?.slug)) throw new Error("slug_invalid");
    return { slug: result.workspace.slug };
  }
  if (action === "select") {
    const items = await query();
    const row = items.find((item) => validId(item.id) && !item.deleted_at && !item.archived_at && item.status !== "archived"
      && typeof item[arg.resource === "agents" ? "name" : "title"] === "string"
      && item[arg.resource === "agents" ? "name" : "title"].trim());
    return row ? { id: row.id } : null;
  }
  if (action === "navigate") {
    if (!validId(arg.id) || !validId(arg.slug) || !["issues", "autopilots", "agents"].includes(arg.resource)) throw new Error("route_invalid");
    const route = arg.resource === "issues" ? "my-issues" : arg.resource;
    const entry = document.querySelector(`[data-ccp-multica-nav-route="${route}"]`);
    if (!entry) throw new Error("sidebar_obscured");
    entry.scrollIntoView({ block: "center", inline: "nearest", behavior: "instant" });
    window.__CODEX_WORKFLOW_SURFACE__.navigate(surface, {
      path: `/${encodeURIComponent(arg.slug)}/${arg.resource}/${encodeURIComponent(arg.id)}`,
    });
    return true;
  }
  const controls = Array.from(surface?.querySelectorAll(`button[aria-label="${arg.back}"]`) || []).filter(visible);
  const heading = Array.from(surface?.querySelectorAll("h1") || []).find(visible);
  const entries = Array.from(document.querySelectorAll("[data-ccp-multica-nav-route]"));
  const active = entries.filter((e) => e.getAttribute("aria-current") === "page");
  const selected = active.length === 1 && active[0].dataset.ccpMulticaNavRoute === arg.route
    && active[0].getAttribute("data-state") === "active";
  if (action === "sidebar") {
    if (!selected) return null;
    const entry = active[0];
    entry.scrollIntoView({ block: "center", inline: "nearest", behavior: "instant" });
    const box = rect(entry);
    if (!box) return null;
    const hit = document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2);
    return hit === entry || entry.contains(hit) ? box : null;
  }
  const alerts = Array.from(shadow?.querySelectorAll('[role="alert"]') || []).filter(visible).length;
  const state = { surface: rect(surface), alerts, backCount: controls.length, hostSynced: selected,
    heading: rect(heading), sidebar: selected ? rect(active[0]) : null,
    headingMatches: !!heading && arg.headings.some((label) => label.toLowerCase() === heading.textContent.trim().toLowerCase()) };
  if (action === "list") return state;
  if (action === "detail") {
    // Pinned IssueDetail renders a readonly button first; TitleEditor mounts
    // only after edit intent. Neither title state requires an h1.
    const title = arg.resource === "issues" ? Array.from(surface?.querySelectorAll(
      '.title-editor, div[role="button"][tabindex="0"].cursor-text.text-display-sm.font-bold',
    ) || []).find(visible) : heading;
    let detailMatches = false;
    if (controls.length === 1 && title && !alerts) {
      const row = (await query()).find((item) => item.id === arg.id);
      const expected = row?.[arg.resource === "agents" ? "name" : "title"];
      detailMatches = typeof expected === "string" && title.textContent.trim() === expected.trim();
    }
    return { ...state, detailMatches };
  }
  if (action === "locate") {
    if (controls.length !== 1 || controls[0].disabled || controls[0].getAttribute("aria-disabled") === "true") return null;
    const control = controls[0];
    const box = rect(control);
    if (!box) return null;
    const x = box.x + box.width / 2, y = box.y + box.height / 2;
    const outerHit = document.elementFromPoint(x, y);
    const hit = shadow.elementFromPoint(x, y);
    if (!(outerHit === host || host?.contains(outerHit)) || !(hit === control || control.contains(hit))) return null;
    return box;
  }
  throw new Error("action_invalid");
}

export async function verify({ request, save, pause = (ms) => new Promise((r) => setTimeout(r, ms)), now = Date.now }) {
  const deadline = now() + 90000;
  const report = { ok: false, checks: [], stage: "prepare" };
  const boundedRequest = async (method, params) => {
    const timeout = requestBudget(method, params, deadline - now());
    assert.ok(timeout > 0, "deadline");
    const result = await request(method, params, timeout);
    assert.ok(now() < deadline, "deadline");
    return result;
  };
  const evaluate = async (action, arg = {}) => {
    assert.ok(now() < deadline, "deadline");
    const result = await boundedRequest("Runtime.evaluate", {
      expression: `(${pageStep.toString()})(${JSON.stringify(action)},${JSON.stringify(arg)}).then(value => ({ value }), error => ({ failureCode: (${failureCode.toString()})(error) }))`,
      awaitPromise: true, returnByValue: true,
    });
    assert.ok(!result.exceptionDetails, "page_step_failed");
    const envelope = result.result?.value;
    if (envelope?.failureCode) throw new Error(failureCode({ code: envelope.failureCode }));
    return envelope?.value;
  };
  const wait = async (action, arg, predicate) => {
    const until = Math.min(deadline, now() + 15000);
    do {
      const result = await evaluate(action, arg);
      assert.equal(result.alerts, 0, "visible_alert");
      if (predicate(result)) return result;
      await pause(300);
    } while (now() < until);
    throw new Error("state_timeout");
  };
  const layout = (s) => s.surface?.width > 100 && s.surface?.height > 100;
  const capture = async (name, clip) => {
    assert.ok(clip?.width > 0 && clip?.height > 0, "capture_geometry");
    const result = await boundedRequest("Page.captureScreenshot", { format: "png", clip: { ...clip, scale: 1 }, captureBeyondViewport: false });
    await save(`${name}.png`, Buffer.from(result.data, "base64"));
  };
  try {
    const { slug } = await evaluate("prepare");
    for (const item of cases) {
      report.stage = `${item.resource}:select`;
      const selected = await evaluate("select", item);
      assert.ok(selected?.id, "fixture_missing");
      const arg = { ...item, ...selected, slug };
      report.stage = `${item.resource}:detail`;
      await evaluate("navigate", arg);
      const detail = await wait("detail", arg, (s) => s.detailMatches && s.backCount === 1 && s.hostSynced && layout(s));
      const check = { resource: item.resource, id: selected.id, detail, clicked: false };
      report.checks.push(check);
      report.stage = `${item.resource}:click`;
      const button = await evaluate("locate", arg);
      assert.ok(button, "return_control_obscured");
      await capture(`${item.resource}-return`, button);
      // Recheck hit-test after capture; avoid clicking stale geometry.
      const current = await evaluate("locate", arg);
      assert.deepEqual(current, button, "return_control_moved");
      const point = { x: button.x + button.width / 2, y: button.y + button.height / 2, button: "left", clickCount: 1 };
      await boundedRequest("Input.dispatchMouseEvent", { type: "mousePressed", ...point });
      await boundedRequest("Input.dispatchMouseEvent", { type: "mouseReleased", ...point });
      check.clicked = true;
      report.stage = `${item.resource}:returned`;
      const accept = (s) => s.headingMatches && s.heading?.width > 0 && s.heading?.height > 0 && s.hostSynced && s.backCount === 0 && layout(s);
      const list = await wait("list", arg, accept);
      await pause(500);
      assert.ok(accept(await evaluate("list", arg)), "return_not_stable");
      check.list = list;
      check.returnButton = button;
      await capture(`${item.resource}-list-heading`, list.heading);
      report.stage = `${item.resource}:sidebar`;
      const sidebar = await evaluate("sidebar", arg);
      assert.ok(sidebar, "sidebar_obscured");
      check.list.sidebar = sidebar;
      await capture(`${item.resource}-host-selection`, sidebar);
    }
    report.ok = true;
    report.stage = "complete";
  } catch (error) {
    report.failureCode = failureCode(error);
    throw new Error(report.failureCode);
  } finally {
    await save("report.json", JSON.stringify(report, null, 2));
  }
  return report;
}

async function main() {
  if (process.argv.length === 2 || (process.argv.length === 3 && process.argv[2] === "--help")) {
    console.log("Prepared only. After final Release repair, open a workflow collection, close editors, then run: node scripts/verify-workflow-return-live.mjs --run-live");
    return;
  }
  assert.ok(process.argv.length === 3 && process.argv[2] === "--run-live", "invalid_arguments");
  const evidence = fileURLToPath(new URL(`../acceptance/evidence/workflow-return-live/${new Date().toISOString().replace(/[:.]/g, "-")}/`, import.meta.url));
  const pending = new Map();
  let socket, sequence = 0;
  const watchdog = setTimeout(() => { console.error("Workflow return verification reached its 110s limit."); process.exit(1); }, 110000);
  try {
    const response = await fetch("http://127.0.0.1:9230/json", { signal: AbortSignal.timeout(5000) });
    assert.ok(response.ok, "discovery_failed");
    const pages = (await response.json()).filter((t) => t.type === "page" && t.url === "app://-/index.html");
    assert.equal(pages.length, 1, "target_count");
    const endpoint = new URL(pages[0].webSocketDebuggerUrl);
    assert.ok(endpoint.protocol === "ws:" && ["127.0.0.1", "localhost", "[::1]"].includes(endpoint.hostname)
      && endpoint.port === "9230" && !endpoint.username && !endpoint.password && !endpoint.search && !endpoint.hash, "target_endpoint");
    socket = new WebSocket(endpoint);
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("connect_timeout")), 5000);
      socket.onopen = () => { clearTimeout(timer); resolve(); };
      socket.onerror = () => { clearTimeout(timer); reject(new Error("connect_failed")); };
    });
    socket.onmessage = ({ data }) => {
      const message = JSON.parse(data), entry = pending.get(message.id);
      if (!entry) return;
      pending.delete(message.id);
      clearTimeout(entry.timer);
      if (message.error) entry.reject(new Error("cdp_failed"));
      else entry.resolve(message.result);
    };
    const request = (method, params, timeout = requestBudget(method, params)) => new Promise((resolve, reject) => {
      const id = ++sequence;
      const timer = setTimeout(() => { pending.delete(id); reject(new Error("cdp_timeout")); }, timeout);
      pending.set(id, { resolve, reject, timer });
      socket.send(JSON.stringify({ id, method, params }));
    });
    await mkdir(evidence, { recursive: true });
    console.log(JSON.stringify({ evidence }));
    const report = await verify({ request, save: (name, data) => writeFile(path.join(evidence, name), data) });
    console.log(JSON.stringify({ ok: report.ok, checked: report.checks.map((c) => c.resource), evidence }));
  } finally {
    clearTimeout(watchdog);
    for (const entry of pending.values()) clearTimeout(entry.timer);
    socket?.close();
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => { console.error(JSON.stringify({ ok: false, failureCode: failureCode(error) })); process.exitCode = 1; });
}
