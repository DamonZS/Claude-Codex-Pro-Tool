import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const script = fileURLToPath(new URL("./verify-workflow-live.mjs", import.meta.url));

function run(args = [], scenario = "detail") {
  const fixture = `
    import assert from 'node:assert/strict';
    const scenario = ${JSON.stringify(scenario)};
    const timeout = globalThis.setTimeout;
    globalThis.setTimeout = (fn, ms, ...args) => timeout(fn, ms === 500 ? 0 : ms, ...args);
    let route = 'my-issues';
    let detail = true;
    globalThis.fetch = async (url) => {
      const manager = String(url).includes(':9333/');
      return { json: async () => [{ type: 'page', url: manager ? 'http://tauri.localhost/' : 'app://-/index.html', webSocketDebuggerUrl: manager ? 'ws://127.0.0.1:9333/manager' : 'ws://127.0.0.1:9230/codex' }] };
    };
    globalThis.WebSocket = class {
      constructor(url) { this.manager = url.endsWith('/manager'); queueMicrotask(() => this.onopen?.()); }
      close() {}
      send(data) {
        const {id, method, params} = JSON.parse(data);
        assert.equal(method, 'Runtime.evaluate');
        const expression = params.expression;
        assert.ok(!expression.includes('renderer-inject.js'));
        let value;
        if (this.manager) {
          assert.ok(expression.includes("__TAURI_INTERNALS__.invoke('repair_frontend_connection',{})"));
          const reply = {status:'ok',codexFrontendInjected:true,codexBackendOnline:scenario !== 'repair-failed'};
          new Function('window', 'return ' + expression)({__TAURI_INTERNALS__:{invoke:async()=>reply}}).then(value =>
            this.onmessage({data:JSON.stringify({id,result:{result:{value}}})}));
          return;
        } else if (expression.includes("'initialize'")) value = {provider:'codex',pageHostProbe:{methods:[]}};
        else if (expression.includes('data-ccp-multica-nav-route')) {
          route = expression.match(/data-ccp-multica-nav-route="([^"]+)"/)[1];
          value = true;
        } else if (expression.includes('back.click()')) {
          value = detail;
          if (scenario !== 'stuck-detail') detail = false;
        } else {
          value = {heading:detail ? 'Existing detail' : {'my-issues':'我的任务',autopilots:'自动化',agents:'智能体'}[route],alerts:scenario === 'alert' ? 1 : 0,buttons:4,rect:{x:0,y:0,width:1004,height:731}};
        }
        queueMicrotask(() => this.onmessage({data:JSON.stringify({id,result:{result:{value}}})}));
      }
    };
  `;
  return spawnSync(process.execPath, ["--import", `data:text/javascript,${encodeURIComponent(fixture)}`, script, ...args], {
    encoding: "utf8", timeout: 10000,
  });
}

test("reload without a Manager endpoint stops before touching the page", () => {
  const result = run(["--reload"]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /--reload requires --manager-cdp/);
  assert.equal(result.stdout, "");
});

test("reload uses only the Manager repair command before verification", () => {
  const result = run(["--reload", "--manager-cdp=http://127.0.0.1:9333"]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /"managerRepair":"ok"/);
  assert.equal(result.stdout.split('\n').filter((line) => line.includes('"route":')).length, 3);
});

test("failed Manager repair stops before Codex verification", () => {
  const result = run(["--reload", "--manager-cdp=http://127.0.0.1:9333"], "repair-failed");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Manager did not confirm frontend repair/);
  assert.equal(result.stdout, "");
});

test("Manager endpoint rejects remote hosts and credential-bearing URLs", () => {
  for (const endpoint of ["http://example.invalid:9333", "http://user:placeholder@127.0.0.1:9333", "http://127.0.0.1:9333?key=placeholder"]) {
    const result = run(["--reload", `--manager-cdp=${endpoint}`]);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /Manager CDP must be a local endpoint/);
  }
});

test("active detail section uses its back button and verifies the list", () => {
  const result = run();
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /"route":"my-issues","backClicked":true,"heading":"我的任务"/);
});

test("clicking back without loading the list is a failed verification", () => {
  const result = run([], "stuck-detail");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /expected list heading/);
});

test("visible alerts fail verification", () => {
  const result = run([], "alert");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /visible error/);
});
