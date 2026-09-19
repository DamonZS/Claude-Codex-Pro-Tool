const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");

const source = fs.readFileSync(path.join(__dirname, "../assets/inject/renderer-inject.js"), "utf8");
const start = source.indexOf("  async function codexPageHostClientFromAppInitial(");
const end = source.indexOf("  async function currentCodexPageHostClient(", start);
assert.ok(start > 0 && end > start);

function fixture(asset = "app-initial-f61fcec072b5.js", skills = { data: [] }) {
  const calls = [];
  const scope = { get() {} };
  const client = { sendRequest: async (method, params) => { calls.push([method, params]); return skills; } };
  const context = {
    codexAppAssetUrl: () => `https://codex.invalid/assets/${asset}`,
    loadCodexAppModule: async () => ({ [asset === "app-initial-6c4523b43a11.js" ? "Rpn" : "Jpn"]: (received, host) => { assert.equal(received, scope); assert.equal(host, "local"); return client; } }),
    codexPageHostAppScopeFromReactRoot: () => scope,
    codexPageHostIdFromActiveThread: () => "",
  };
  vm.createContext(context);
  vm.runInContext(source.slice(start, end), context);
  return { ...context, calls, client };
}

test("audited asset uses existing page client without reinitialization or model calls", async () => {
  const f = fixture();
  const result = await f.codexPageHostClientFromAppInitial();
  assert.equal(result.client, f.client);
  assert.deepEqual(f.calls.map(([method]) => method), ["skills/list"]);
  assert.equal(result.initializeResponse.pageHostProbe.skillInput, true);
  assert.ok(result.initializeResponse.pageHostProbe.methods.includes("turn/interrupt"));
  assert.equal(result.initializeResponse.capabilities.length, 0);
});

test("current audited Codex asset reuses its Rpn page client after reinjection", async () => {
  const f = fixture("app-initial-6c4523b43a11.js");
  const result = await f.codexPageHostClientFromAppInitial();
  assert.equal(result.client, f.client);
  assert.deepEqual(f.calls.map(([method]) => method), ["skills/list"]);
  assert.equal(result.initializeResponse.pageHostProbe.asset, "app-initial-6c4523b43a11.js");
});

test("unknown assets never call a guessed minified accessor", async () => {
  const f = fixture("app-initial-unknown.js");
  await assert.rejects(f.codexPageHostClientFromAppInitial(), /version_unsupported/);
  assert.equal(f.calls.length, 0);
});

test("missing audited export does not invoke a similarly named factory", async () => {
  const f = fixture();
  let invoked = false;
  f.loadCodexAppModule = async () => ({ Qfe: () => { invoked = true; } });
  // The context function reads the original VM globals.
  const context = { ...f };
  vm.createContext(context);
  vm.runInContext(source.slice(start, end), context);
  await assert.rejects(context.codexPageHostClientFromAppInitial(), /factory_unavailable/);
  assert.equal(invoked, false);
});

test("failed read-only probe does not advertise lifecycle support", async () => {
  const f = fixture(undefined, { error: "disconnected" });
  await assert.rejects(f.codexPageHostClientFromAppInitial(), /probe_failed/);
});
