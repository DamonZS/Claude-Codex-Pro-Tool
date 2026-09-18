import assert from "node:assert/strict";
import { createRequire } from "node:module";
import test from "node:test";
import { parentId, subjects, verify } from "./verify-native-subtasks-live.mjs";

const require = createRequire(new URL("../apps/codex-workflow-surface/package.json", import.meta.url));
const { JSDOM } = require("jsdom");

function fixture(scenario = "ok") {
  const dom = new JSDOM('<button data-ccp-multica-nav-route="my-issues">我的任务</button><div id="ccp-multica-workspace-root"></div>', { runScripts: "outside-only" });
  const { window } = dom;
  const doc = window.document;
  const host = doc.querySelector("#ccp-multica-workspace-root");
  const root = host.attachShadow({ mode: "open" });
  let selected = parentId, expanded = false, clock = 0, hit, held;
  const clicked = [], saved = new Map();
  window.__codexSessionDeleteBridge = () => {};
  window.__claudeCodexProMulticaWorkspaceGeneration = 20;
  function visible(element) {
    return element.isConnected && element.style.display !== "none"
      && (element.getRootNode() !== root || host.style.display !== "none");
  }
  window.HTMLElement.prototype.getBoundingClientRect = function () {
    const yes = visible(this);
    return { x: 10, y: 10, width: yes ? 500 : 0, height: yes ? 80 : 0 };
  };
  window.HTMLElement.prototype.getClientRects = function () { return visible(this) ? [this.getBoundingClientRect()] : []; };
  window.HTMLElement.prototype.scrollIntoView = function () { hit = this; };
  doc.elementFromPoint = root.elementFromPoint = () => hit;
  const native = new Map();
  function select(id) {
    selected = id;
    for (const [key, row] of native) row.setAttribute("data-app-action-sidebar-thread-active", String(key === id));
    host.style.display = "none";
  }
  for (const id of scenario === "native-panel" ? [parentId] : [parentId, subjects[0].id]) {
    const button = doc.createElement("button");
    button.dataset.appActionSidebarThreadId = id;
    button.textContent = "Native row";
    button.onclick = () => { clicked.push(`native:${id}`); if (scenario !== "restore-stuck") select(id); };
    doc.body.append(button);
    native.set(id, button);
  }
  function render() {
    root.innerHTML = '<h1>我的任务</h1><section aria-label="Codex 原生子任务"><ul></ul><button aria-expanded="false">展开近期子任务</button></section>';
    const region = root.querySelector("section");
    if (scenario === "alert") region.insertAdjacentHTML("beforeend", '<p role="alert">fixture error</p>');
    const expand = region.querySelector("button");
    expand.setAttribute("aria-expanded", String(expanded));
    expand.onclick = () => { expanded = true; clicked.push("expand"); render(); };
    const rows = expanded ? subjects : subjects.slice(0, 2);
    for (const subject of rows) {
      if (scenario === "missing" && subject.name === "Mendel") continue;
      const row = doc.createElement("li");
      row.dataset.nativeThreadId = subject.id;
      row.innerHTML = `<button class="ccp-native-subtask-name">${subject.name}</button><span>${subject.name === "Mendel" ? "状态待确认" : "已完成"}</span><button aria-label="打开 ${subject.name} 的父会话">父会话</button><time datetime="2026-09-18T00:00:00Z"></time>`;
      row.querySelector(".ccp-native-subtask-name").onclick = () => {
        clicked.push(`child:${subject.id}`);
        if (scenario === "native-panel") {
          const panel = doc.createElement("div");
          panel.dataset.tabId = `background-agent:${subject.id}`;
          panel.innerHTML = '<button role="tab" aria-selected="true">Child</button><button aria-label="关闭Child标签页"></button>';
          panel.querySelector('[aria-label]').onclick = () => { clicked.push("close-panel"); panel.remove(); };
          doc.body.append(panel);
          host.style.display = "none";
        } else if (!["child-stuck", "restore-stuck"].includes(scenario)) select(subject.id);
      };
      row.querySelector("[aria-label]").onclick = () => {
        clicked.push(`parent:${subject.id}`);
        if (scenario !== "parent-stuck") select(parentId);
      };
      region.querySelector("ul").append(row);
    }
  }
  doc.querySelector("[data-ccp-multica-nav-route]").onclick = () => { host.style.display = "block"; clicked.push("entry"); render(); };
  render();
  select(scenario === "wrong-start" ? subjects[0].id : parentId);
  return { clicked, saved, dom, active: () => selected,
    run: () => verify({
      now: () => clock, pause: async (ms) => { clock += ms; },
      save: async (name, data) => { saved.set(name, data); },
      request: async (method, params) => {
        assert.ok(["Runtime.evaluate", "Input.dispatchMouseEvent", "Page.captureScreenshot"].includes(method));
        if (method === "Runtime.evaluate") {
          assert.ok(!/turn\/start|thread\/start|repair_frontend_connection|renderer-inject|__CODEX_WORKFLOW_BRIDGE__/.test(params.expression));
          return { result: { value: JSON.parse(JSON.stringify(window.eval(params.expression))) } };
        }
        if (method === "Input.dispatchMouseEvent") {
          if (params.type === "mousePressed") held = hit;
          else { assert.equal(held, hit); held.click(); }
          return {};
        }
        assert.ok(params.clip.width > 0 && params.clip.height > 0);
        return { data: Buffer.from("fixture screenshot").toString("base64") };
      },
    }),
  };
}

test("expands exact IDs, captures bounded evidence, clicks child and parent, restores parent", async () => {
  const f = fixture();
  try {
    const report = await f.run();
    assert.equal(report.ok, true);
    assert.equal(report.restored, true);
    assert.deepEqual(report.checks[0].rows.map((r) => r.id), subjects.map((s) => s.id));
    assert.equal(report.checks[0].rows[3].status, "状态待确认");
    assert.ok(f.clicked.includes("expand"));
    assert.ok(f.clicked.includes(`child:${subjects[0].id}`));
    assert.ok(f.clicked.includes(`parent:${subjects[0].id}`));
    assert.equal(f.active(), parentId);
    assert.equal(f.saved.size, 7);
  } finally { f.dom.window.close(); }
});

for (const [scenario, error] of [
  ["missing", /Expected native thread IDs missing/],
  ["alert", /visible alerts/],
  ["child-stuck", /Native thread activation not confirmed/],
  ["parent-stuck", /Native thread activation not confirmed/],
]) test(`${scenario} fails and restores the original parent`, async () => {
  const f = fixture(scenario);
  try {
    await assert.rejects(f.run(), error);
    const report = JSON.parse(f.saved.get("report.json"));
    assert.equal(report.ok, false);
    assert.equal(report.restored, true);
    assert.equal(f.active(), parentId);
  } finally { f.dom.window.close(); }
});

test("a different starting thread stops before any click", async () => {
  const f = fixture("wrong-start");
  try {
    await assert.rejects(f.run(), /Start from the expected parent/);
    assert.equal(f.clicked.length, 0);
    assert.equal(f.saved.size, 0);
  } finally { f.dom.window.close(); }
});

test("restoration failure is reported separately and never marked successful", async () => {
  const f = fixture("restore-stuck");
  try {
    await assert.rejects(f.run(), /Restore of parent thread not confirmed/);
    const report = JSON.parse(f.saved.get("report.json"));
    assert.equal(report.ok, false);
    assert.equal(report.restored, false);
  } finally { f.dom.window.close(); }
});

test("native child opens in a side tab without a sidebar row; cleanup closes only that new tab", async () => {
  const f = fixture("native-panel");
  try {
    const report = await f.run();
    const child = report.checks.find((check) => check.step === "child");
    assert.equal(child.sidebarActive, false);
    assert.equal(child.nativeTabId, `background-agent:${subjects[0].id}`);
    assert.equal(child.active, true);
    assert.equal(report.restored, true);
    assert.ok(f.clicked.includes("close-panel"));
    assert.equal(f.dom.window.document.querySelector('[role="tab"]'), null);
  } finally { f.dom.window.close(); }
});
