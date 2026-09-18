import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { cases, failureCode, requestBudget, verify } from "./verify-workflow-return-live.mjs";

const require = createRequire(new URL("../apps/codex-workflow-surface/package.json", import.meta.url));
const { JSDOM } = require("jsdom");

function fixture(scenario = "ok") {
  const dom = new JSDOM('<div id="ccp-multica-workspace-root"></div>', { runScripts: "outside-only" });
  const { window } = dom, doc = window.document;
  const host = doc.querySelector("#ccp-multica-workspace-root");
  const shadow = host.attachShadow({ mode: "open" });
  const surface = doc.createElement("div");
  surface.className = "ccp-upstream-workflow-container";
  shadow.append(surface);
  const entries = new Map(), saved = new Map(), calls = [], clicked = [], navigated = [];
  const sensitive = "PRIVATE_FIXTURE_TITLE_DO_NOT_LOG";
  let clock = 0, button, held, detailReads = 0, captures = 0, scrolledEntry;
  const revealed = new Set();
  window.HTMLElement.prototype.getBoundingClientRect = function () {
    const width = this === surface ? 600 : 80, height = this === surface ? 450 : 30;
    const y = ["sidebar-offscreen", "sidebar-scrolls-away"].includes(scenario) && this.dataset.ccpMulticaNavRoute && !revealed.has(this) ? -493 : 20;
    return { x: 20, y, width, height, right: 20 + width, bottom: y + height };
  };
  window.HTMLElement.prototype.scrollIntoView = function () { scrolledEntry = this; revealed.add(this); };
  window.HTMLElement.prototype.getClientRects = function () { return this.isConnected ? [this.getBoundingClientRect()] : []; };
  doc.elementFromPoint = () => scenario === "occluded" || (scenario === "sidebar-covered" && scrolledEntry) ? doc.body : scrolledEntry || host;
  shadow.elementFromPoint = () => button;
  for (const item of cases) {
    const e = doc.createElement("button");
    e.dataset.ccpMulticaNavRoute = item.route;
    doc.body.append(e);
    entries.set(item.route, e);
  }
  function select(route) {
    for (const [key, e] of entries) {
      e.setAttribute("aria-current", key === route ? "page" : "false");
      e.setAttribute("data-state", key === route ? "active" : "inactive");
    }
  }
  function list(item) {
    surface.innerHTML = `<h1>${item.headings[0]}</h1>`;
    if (scenario === "host-stale") select("unmatched");
    if (scenario === "wrong-list") surface.querySelector("h1").textContent = sensitive;
    if (scenario === "alert") surface.insertAdjacentHTML("beforeend", '<div role="alert">private error</div>');
  }
  const row = (resource) => ({ id: `fixture-${resource}`, title: sensitive, name: sensitive, revision: 1 });
  window.__CODEX_WORKFLOW_BRIDGE__ = {
    async postJson(route, payload) {
      calls.push({ route, payload });
      if (route === "/multica/workspace/bootstrap") {
        if (scenario === "slow-bootstrap") clock += 20000;
        if (scenario === "deadline") clock += 90001;
        if (scenario === "private-error") throw new Error(sensitive);
        if (scenario === "bridge-timeout") throw new Error("bridge_timeout");
        assert.deepEqual(JSON.parse(JSON.stringify(payload)), {});
        return { status: "ok", workspace: { slug: "local", name: sensitive }, credential: sensitive };
      }
      assert.equal(route, "/multica/workspace/query");
      assert.deepEqual(JSON.parse(JSON.stringify(payload)), { resource: payload.resource, limit: 20, offset: 0 });
      assert.ok(cases.some((c) => c.resource === payload.resource));
      if (scenario === "missing") return { status: "ok", items: [] };
      return { status: "ok", items: [row(payload.resource)] };
    },
  };
  window.__CODEX_WORKFLOW_SURFACE__ = {
    navigate(container, options) {
      assert.equal(container, surface);
      const item = cases.find((c) => options.path === `/local/${c.resource}/fixture-${c.resource}`);
      assert.ok(item);
      assert.equal(scrolledEntry, entries.get(item.route), "scroll the matching entry before detail navigation");
      assert.ok(revealed.has(entries.get(item.route)));
      if (scenario === "sidebar-scrolls-away") revealed.delete(entries.get(item.route));
      navigated.push(item.resource);
      scrolledEntry = null;
      select(item.route);
      surface.replaceChildren();
      button = doc.createElement("button");
      button.setAttribute("aria-label", item.back);
      button.onclick = () => { clicked.push(item.resource); if (scenario !== "stuck") list(item); };
      surface.append(button);
      const title = doc.createElement(item.resource === "issues" ? "div" : "h1");
      if (item.resource === "issues") {
        title.className = scenario === "title-editor" ? "tiptap ProseMirror title-editor outline-none"
          : "w-full cursor-text text-display-sm font-bold leading-snug tracking-tight";
        title.setAttribute("role", scenario === "title-editor" ? "textbox" : "button");
        title.setAttribute("tabindex", "0");
        title.onclick = () => assert.fail("verification must not activate the title editor");
        // Real detail has non-title headings and a separate rich-text editor.
        const heading = doc.createElement("h2");
        heading.textContent = "Activity";
        const body = doc.createElement("div");
        body.className = "tiptap ProseMirror flex-1 rich-text-editor text-body outline-none";
        body.contentEditable = "true";
        body.textContent = sensitive;
        surface.append(heading, body);
      }
      title.textContent = scenario === "wrong-detail" ? "different record" : sensitive;
      if (scenario !== "body-only") surface.append(title);
    },
  };
  if (scenario === "editing") {
    const input = doc.createElement("input");
    surface.append(input);
    input.focus();
  }
  if (scenario === "hidden") host.style.display = "none";
  return { saved, clicked, navigated, calls, close: () => dom.window.close(),
    run: () => verify({
      now: () => clock, pause: async (ms) => { clock += ms; },
      save: async (name, data) => { saved.set(name, data); },
      request: async (method, params, timeout) => {
        assert.equal(timeout, requestBudget(method, params, 90000 - clock));
        assert.ok(["Runtime.evaluate", "Input.dispatchMouseEvent", "Page.captureScreenshot"].includes(method));
        if (method === "Runtime.evaluate") {
          assert.ok(!/turn\/start|executions\/|upsert|\.click\(|\.mount\(|\.dispose\(|renderer-inject/.test(params.expression));
          const value = await window.eval(params.expression);
          const serialized = JSON.stringify(value);
          assert.ok(!serialized?.includes(sensitive), "private title/credential escaped page context");
          if (params.expression.includes(')("detail",')) {
            detailReads++;
            if (scenario === "loading" && detailReads < 3) value.value.detailMatches = false;
          }
          return { result: { value } };
        }
        if (method === "Input.dispatchMouseEvent") {
          if (params.type === "mousePressed") held = button;
          else { assert.equal(held, button); held.click(); }
          return {};
        }
        assert.equal(params.captureBeyondViewport, false);
        assert.ok(params.clip.width <= 80 && params.clip.height <= 30, "capture exposed detail content");
        captures++;
        return { data: Buffer.from(`synthetic crop ${captures}`).toString("base64") };
      },
    }),
  };
}

for (const scenario of ["ok", "title-editor", "loading", "slow-bootstrap", "sidebar-offscreen", "sidebar-scrolls-away"]) test(`${scenario}: three actual controls, read-only queries and sanitized evidence`, async () => {
  const f = fixture(scenario);
  try {
    const report = await f.run();
    assert.equal(report.ok, true);
    assert.deepEqual(f.clicked, ["issues", "autopilots", "agents"]);
    assert.deepEqual(f.navigated, f.clicked);
    assert.equal(f.saved.size, 10);
    assert.ok(report.checks.every((c) => c.clicked && c.detail.detailMatches && c.list.headingMatches && c.list.hostSynced));
    assert.equal(report.checks[0].detail.heading, null, "Issue detail must not require an h1");
    if (scenario === "sidebar-offscreen") {
      assert.ok(report.checks.every((c) => c.detail.hostSynced && c.detail.sidebar?.height > 0));
    }
    if (scenario === "sidebar-scrolls-away") {
      assert.ok(report.checks.every((c) => c.detail.hostSynced && c.detail.sidebar === null));
      assert.ok(report.checks.every((c) => c.list.sidebar?.height > 0));
    }
    assert.equal(f.calls.filter((c) => c.route.endsWith("bootstrap")).length, 1);
    assert.ok(!f.saved.get("report.json").includes("PRIVATE_FIXTURE"));
  } finally { f.close(); }
});

test("awaitPromise reads get 45s, other calls retain 5s, all fit remaining budget", () => {
  assert.equal(requestBudget("Runtime.evaluate", { awaitPromise: true }), 45000);
  assert.equal(requestBudget("Runtime.evaluate", { awaitPromise: false }), 5000);
  assert.equal(requestBudget("Page.captureScreenshot", {}), 5000);
  assert.equal(requestBudget("Input.dispatchMouseEvent", {}), 5000);
  assert.equal(requestBudget("Runtime.evaluate", { awaitPromise: true }, 12000), 12000);
  assert.equal(requestBudget("Runtime.evaluate", { awaitPromise: true }, -1), 0);
});

for (const [scenario, expected] of [["hidden", "surface_missing"], ["deadline", "deadline"], ["private-error", "verification_failed"], ["bridge-timeout", "bridge_timeout"]]) {
  test(`${scenario}: records only a whitelisted failure code before clicks`, async () => {
    const f = fixture(scenario);
    try {
      await assert.rejects(f.run(), (error) => error.message === expected);
      const report = JSON.parse(f.saved.get("report.json"));
      assert.equal(report.failureCode, expected);
      assert.equal(report.stage, "prepare");
      assert.deepEqual(f.clicked, []);
      assert.ok(!JSON.stringify(report).includes("PRIVATE_FIXTURE"));
    } finally { f.close(); }
  });
}

test("failure diagnostics discard arbitrary codes, messages and exception content", () => {
  assert.equal(failureCode({ code: "cdp_timeout", message: "private content" }), "cdp_timeout");
  assert.equal(failureCode({ code: "private content", message: "private content" }), "verification_failed");
  assert.equal(failureCode({ message: "read_failed\nprivate content" }), "read_failed");
});

for (const scenario of ["missing", "wrong-detail", "body-only", "wrong-list", "host-stale", "occluded", "sidebar-covered", "stuck", "editing", "alert"]) {
  test(`${scenario}: fails closed with bounded, sanitized stage report`, async () => {
    const f = fixture(scenario);
    try {
      await assert.rejects(f.run());
      const report = JSON.parse(f.saved.get("report.json"));
      assert.equal(report.ok, false);
      assert.notEqual(report.stage, "complete");
      if (scenario === "sidebar-covered") assert.equal(report.failureCode, "sidebar_obscured");
      if (["missing", "wrong-detail", "body-only", "occluded", "editing"].includes(scenario)) assert.deepEqual(f.clicked, []);
    } finally { f.close(); }
  });
}

test("default invocation has no CDP access and requests explicit live flag", () => {
  const script = fileURLToPath(new URL("./verify-workflow-return-live.mjs", import.meta.url));
  const guard = "globalThis.fetch=()=>{throw Error('unexpected network')};globalThis.WebSocket=class{constructor(){throw Error('unexpected socket')}};";
  const result = spawnSync(process.execPath, ["--import", `data:text/javascript,${encodeURIComponent(guard)}`, script], { encoding: "utf8", timeout: 5000 });
  assert.equal(result.status, 0);
  assert.match(result.stdout, /--run-live/);
  assert.equal(result.stderr, "");
});
