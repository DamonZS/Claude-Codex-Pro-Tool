const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");

const source = fs.readFileSync(process.argv[2] || path.join(__dirname, "../assets/inject/renderer-inject.js"), "utf8");
function section(startText, endText) {
  const start = source.indexOf(startText);
  const end = source.indexOf(endText, start);
  assert.ok(start >= 0 && end > start);
  return source.slice(start, end);
}
function fixture({ enabled = true, disabled = false } = {}) {
  const context = vm.createContext({
    window: {},
    codexPluginMarketplaceUnlockVersion: source.match(/codexPluginMarketplaceUnlockVersion = "(\d+)"/)[1],
    pluginPatchDisabledInRelayMode: () => disabled,
    claudeCodexProSettings: () => ({ pluginMarketplaceUnlock: enabled }),
    sendClaudeCodexProDiagnostic: () => {},
  });
  vm.runInContext(section("  function restorePluginMarketplaceName(", "  function codexPluginMarketplaceRequestPatchStrategy(")
    + section("  function isCodexPluginBuildFlavorFilter(", "  function restorePluginMarketplaceRequestParams("), context);
  vm.runInContext(`
    const plugins = [
      { name: "LaTeX", marketplaceName: "openai-bundled" },
      { name: "Slides", marketplaceName: "openai-curated" },
      { name: "Sheets", marketplaceName: "openai-api-curated" },
      { name: "Docs", marketplaceName: "openai-primary-runtime" },
      { name: "Custom", marketplaceName: "community" },
    ];
    const u = name => name.startsWith("openai-");
    const ne = u, Mj = u, renamedCheck = u;
    const r = "openai-bundled", n = r, flavor = r;
    installPluginBuildFlavorFilterPatch();
  `, context);
  return expression => JSON.parse(vm.runInContext(`JSON.stringify(${expression})`, context));
}

test("old u/ne build flavor callbacks retain all five plugins", () => {
  const run = fixture();
  assert.equal(run("plugins.filter(e=>!u(e.marketplaceName)||e.marketplaceName===r).length"), 5);
  assert.equal(run("plugins.filter(e=>!ne(e.marketplaceName)||e.marketplaceName===n).length"), 5);
});
test("Codex 26.930 Mj and renamed callbacks retain all five plugins", () => {
  const run = fixture();
  assert.equal(run("plugins.filter(e=>!Mj(e.marketplaceName)||e.marketplaceName===n).length"), 5);
  assert.equal(run("plugins.filter((plugin) => !renamedCheck(plugin.marketplaceName) || plugin.marketplaceName === flavor).length"), 5);
});
test("search, additional predicates and thisArg keep native results", () => {
  const run = fixture();
  assert.deepEqual(run('plugins.filter(e=>e.name.includes("Slides")).map(e=>e.name)'), ["Slides"]);
  assert.equal(run('plugins.filter(e=>e.marketplaceName==="openai-curated").length'), 1);
  assert.equal(run('plugins.filter(e=>(!Mj(e.marketplaceName)||e.marketplaceName===n)&&e.name==="LaTeX").length'), 1);
  assert.equal(run('plugins.filter(e=>!u(e.marketplaceName)||e.marketplaceName===r&&e.name==="LaTeX").length'), 2);
  assert.equal(run('plugins.filter(function(e) { return e.name === this.name; }, { name: "Sheets" }).length'), 1);
});
test("no official sample, unlock off and disabled mode preserve native filter", () => {
  assert.equal(fixture()('[{marketplaceName:"community"}].filter(e=>!Mj(e.marketplaceName)||e.marketplaceName===n).length'), 1);
  for (const settings of [{ enabled: false }, { disabled: true }]) {
    assert.equal(fixture(settings)("plugins.filter(e=>!Mj(e.marketplaceName)||e.marketplaceName===n).length"), 2);
  }
});
