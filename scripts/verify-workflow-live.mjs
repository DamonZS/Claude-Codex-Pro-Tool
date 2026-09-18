import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = fileURLToPath(new URL("../", import.meta.url));
// Raw renderer source lacks the launcher's template values and bridge setup.
// Repair through the running Manager, which owns the embedded assets and credentials.
if (process.argv.includes("--reload")) {
  const managerEndpoint = process.argv.find((arg) => arg.startsWith("--manager-cdp="))?.split("=").slice(1).join("=");
  assert.ok(managerEndpoint, "--reload requires --manager-cdp=http://127.0.0.1:PORT for the running Manager; otherwise use Manager's Repair frontend button, then run without --reload");
  const endpoint = new URL(managerEndpoint);
  assert.ok(endpoint.protocol === "http:" && ["127.0.0.1", "localhost", "[::1]"].includes(endpoint.hostname)
    && !endpoint.username && !endpoint.password && !endpoint.search && !endpoint.hash, "Manager CDP must be a local endpoint without credentials");
  const managerTargets = await (await fetch(new URL("/json", endpoint), { signal: AbortSignal.timeout(5000) })).json();
  const managers = managerTargets.filter((item) => item.type === "page" && /^https?:\/\/tauri\.localhost(?:\/|$)|^tauri:\/\/localhost(?:\/|$)/.test(item.url));
  assert.equal(managers.length, 1, "Exactly one running Tauri Manager page is required");
  const managerSocket = new WebSocket(managers[0].webSocketDebuggerUrl);
  try {
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("Manager CDP connection timed out")), 5000);
      managerSocket.onopen = () => { clearTimeout(timer); resolve(); };
      managerSocket.onerror = () => { clearTimeout(timer); reject(new Error("Manager CDP connection failed")); };
    });
    const repaired = await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("Manager frontend repair timed out")), 90000);
      managerSocket.onmessage = ({ data }) => {
        const message = JSON.parse(data);
        if (message.id !== 1) return;
        clearTimeout(timer);
        // Never print the full command result, exception details or credentials.
        resolve(!message.error && !message.result?.exceptionDetails && message.result?.result?.value === true);
      };
      managerSocket.send(JSON.stringify({ id: 1, method: "Runtime.evaluate", params: {
        expression: "(async()=>{const r=await window.__TAURI_INTERNALS__.invoke('repair_frontend_connection',{});return r?.status==='ok'&&r.codexFrontendInjected===true&&r.codexBackendOnline===true})()",
        awaitPromise: true, returnByValue: true,
      } }));
    });
    assert.equal(repaired, true, "Manager did not confirm frontend repair");
    console.log(JSON.stringify({ managerRepair: "ok" }));
  } finally {
    managerSocket.close();
  }
}
const targets = await (await fetch("http://127.0.0.1:9230/json")).json();
const target = targets.find((item) => item.type === "page" && item.url === "app://-/index.html");
assert.ok(target?.webSocketDebuggerUrl, "Current Codex page is required");
const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
let sequence = 0;
const pending = new Map();
socket.onmessage = ({ data }) => {
  const message = JSON.parse(data);
  if (!pending.has(message.id)) return;
  const { resolve, reject, timer } = pending.get(message.id);
  pending.delete(message.id);
  clearTimeout(timer);
  if (message.error) reject(new Error(`CDP error ${message.error.code}`));
  else resolve(message.result);
};
function request(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`${method} timed out`)); }, 45000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const result = await request("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  assert.ok(!result.exceptionDetails, "Live workflow evaluation failed");
  return result.result?.value;
}
const surface = "document.querySelector('#ccp-multica-workspace-root')?.shadowRoot?.querySelector('.ccp-upstream-workflow-container')";
try {
  const capabilities = await evaluate("(async()=>{const r=await window.__claudeCodexProCodexPageHostRequest('initialize',{});return {provider:r.provider,methods:r.pageHostProbe?.methods}})()");
  assert.equal(capabilities.provider, "codex");
  console.log(JSON.stringify({ nativeHost: capabilities }));
  const workspacePrepared = await evaluate(`(()=>{const box=${surface};const rect=box?.getBoundingClientRect();if(rect?.width>100&&rect?.height>100)return true;const plugin=Array.from(document.querySelectorAll('button')).find((button)=>button.getClientRects().length&&button.textContent?.trim()==='插件');plugin?.click();return !!plugin})()`);
  assert.ok(workspacePrepared, "Codex 插件入口未找到");
  for (const route of ["my-issues", "autopilots", "agents"]) {
    const expectedHeading = { "my-issues": "我的任务", autopilots: "自动化", agents: "智能体" }[route];
    const englishHeading = { "my-issues": "My issues", autopilots: "Autopilots", agents: "Agents" }[route];
    assert.equal(await evaluate(`(()=>{const entry=document.querySelector('[data-ccp-multica-nav-route="${route}"]');entry?.click();return !!entry})()`), true);
    let state;
    let backClicked = false;
    for (let attempt = 0; attempt < 90; attempt++) {
      await new Promise((resolve) => setTimeout(resolve, 500));
      // Selecting an already active section preserves its detail path. Use the
      // real back control; an arbitrary h1 is not evidence of the list loading.
      if (!backClicked) backClicked = await evaluate(`(()=>{const root=${surface};const back=root?.querySelector('button[aria-label="返回${expectedHeading}"]');if(!back||!back.getClientRects().length)return false;back.click();return true})()`);
      state = await evaluate(`(()=>{const root=${surface};const rect=root?.getBoundingClientRect();return {heading:root?.querySelector('h1')?.textContent?.trim(),alerts:Array.from(root?.querySelectorAll('[role="alert"]')||[]).filter(e=>e.getClientRects().length).length,buttons:root?.querySelectorAll('button').length,rect:rect?{x:rect.x,y:rect.y,width:rect.width,height:rect.height}:null}})()`);
      if (([expectedHeading, englishHeading].some((heading) => heading.toLowerCase() === state.heading?.toLowerCase()) && state.rect?.width > 100 && state.rect?.height > 100) || state.alerts) break;
    }
    assert.ok([expectedHeading, englishHeading].some((heading) => heading.toLowerCase() === state.heading?.toLowerCase()), `${route}: expected list heading; ${JSON.stringify(state)}`);
    assert.equal(state.alerts, 0, `${route}: visible error`);
    assert.ok(state.rect.width > 100 && state.rect.height > 100, `${route}: empty layout ${JSON.stringify(state.rect)}`);
    console.log(JSON.stringify({ route, backClicked, ...state }));
    if (process.argv.includes("--screenshots")) {
      const dir = path.join(root, "acceptance/evidence/multica-live");
      await mkdir(dir, { recursive: true });
      const capture = await request("Page.captureScreenshot", { format: "png", clip: { ...state.rect, scale: 1 } });
      await writeFile(path.join(dir, `${route}.png`), Buffer.from(capture.data, "base64"));
    }
  }
} finally {
  for (const entry of pending.values()) clearTimeout(entry.timer);
  socket.close();
}
