const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { createRequire } = require("node:module");
const { test } = require("node:test");

const root = path.resolve(__dirname, "../apps/codex-workflow-surface");
const { JSDOM } = createRequire(path.join(root, "package.json"))("jsdom");

test("built workflow mounts offline and disposes the previous injected runtime", async () => {
  const bundle = fs.readFileSync(path.join(root, "dist/codex-workflow-surface.js"), "utf8");
  const styles = fs.readFileSync(path.join(root, "dist/codex-workflow-surface.css"), "utf8");
  const dom = new JSDOM("<!doctype html><html><body><main id='native'>Native content</main></body></html>", {
    url: "https://workflow.invalid", runScripts: "outside-only", pretendToBeVisual: true,
  });
  const window = dom.window;
  let networkCalls = 0;
  let disposed = 0;
  // jsdom omits these browser globals; the artifact itself supplies no shims.
  Object.assign(window, { TextEncoder, TextDecoder, structuredClone, Headers, Response, Request });
  window.matchMedia = () => ({ matches: false, addEventListener() {}, removeEventListener() {} });
  window.ResizeObserver = class { observe() {} unobserve() {} disconnect() {} };
  window.fetch = () => { networkCalls++; throw new Error("Unexpected network"); };
  window.WebSocket = class { constructor() { networkCalls++; throw new Error("Unexpected socket"); } };
  window.__CODEX_WORKFLOW_SURFACE__ = { dispose() { disposed++; } };
  try {
    window.eval(bundle);
    assert.equal(disposed, 1);
    const first = window.__CODEX_WORKFLOW_SURFACE__;
    assert.deepEqual(Object.keys(first).sort(), ["dispose", "event", "invalidate", "mount", "navigate", "unmount"]);
    for (const value of Object.values(first)) assert.equal(typeof value, "function");
    const originalDispose = first.dispose;
    first.dispose = () => { disposed++; originalDispose(); };
    window.eval(bundle);
    assert.equal(disposed, 2);
    assert.notEqual(window.__CODEX_WORKFLOW_SURFACE__, first);
    // CDP injection can permit feature probes; later event callbacks obey CSP.
    window.Function = new Proxy(window.Function, {
      construct() { throw new EvalError("Dynamic code compilation blocked by CSP"); },
      apply() { throw new EvalError("Dynamic code compilation blocked by CSP"); },
    });
    window.__CODEX_WORKFLOW_BRIDGE__ = { postJson: async (path) => {
      if (path === "/multica/workspace/bootstrap") return {
        status: "degraded", workspace: { id: "fixture", slug: "fixture", name: "Fixture" },
        user: { id: "fixture-user", kind: "local_control_plane" }, runtime: { available: false },
      };
      if (path === "/multica/workspace/query") return { status: "ok", items: [], total: 0 };
      return { status: "failed", code: "capability_unavailable" };
    } };
    const host = window.document.createElement("div");
    window.document.body.append(host);
    const container = window.document.createElement("div");
    host.attachShadow({ mode: "open" }).append(container);
    window.__CODEX_WORKFLOW_SURFACE__.mount(container, { route: "agents", workspaceId: "fixture", workspaceSlug: "fixture" });
    for (let attempt = 0; attempt < 100 && !container.querySelector("h1"); attempt++) {
      await new Promise((resolve) => setTimeout(resolve, 20));
    }
    assert.ok(container.querySelector("h1"), container.textContent);
    window.__CODEX_WORKFLOW_SURFACE__.dispose();
    await new Promise((resolve) => setTimeout(resolve, 100));
    assert.equal(networkCalls, 0);
    assert.equal(window.document.querySelector("#native").textContent, "Native content");
    assert.ok(styles.includes("ccp-workflow-surface"));
  } catch (error) {
    // Avoid dumping the entire embedded production source in a Node failure.
    throw new Error(String(error.message));
  } finally {
    window.close();
  }
});
