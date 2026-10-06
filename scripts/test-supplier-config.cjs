const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");
const ts = require("../apps/claude-codex-pro-manager/node_modules/typescript");
const root = path.resolve(process.argv[2] || path.join(__dirname, ".."));
const sourcePath = path.join(root, "apps/claude-codex-pro-manager/src/lib/supplier.ts");
const code = ts.transpileModule(fs.readFileSync(sourcePath, "utf8"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText;
const exportsObject = {};
vm.runInNewContext(code, {
  exports: exportsObject,
  require(id) {
    assert.equal(id, "@/constants");
    return { AGGREGATE_STRATEGIES: [] };
  },
});
const { buildSupplierConfigToml, withSupplierGeneratedFiles } = exportsObject;

function profile(overrides = {}) {
  return {
    id: "fixture", name: "Fixture", targetApp: "codex", model: "fixture-model",
    baseUrl: "https://fixture.invalid/v1", upstreamBaseUrl: "", apiKey: "",
    protocol: "responses", relayMode: "direct", officialMixApiKey: false,
    testModel: "", configContents: "", authContents: "", useCommonConfig: false,
    contextSelection: "", contextSelectionInitialized: false, contextWindow: 0,
    autoCompactLimit: 0, modelList: "", userAgent: "", ...overrides,
  };
}

function assertInlineToken(config, key) {
  const line = config.split("\n").find((line) => line.startsWith("experimental_bearer_token = "));
  assert.ok(line, "current credential must be stored in provider config");
  assert.equal(JSON.parse(line.slice(line.indexOf("=") + 1).trim()), key);
  assert.doesNotMatch(config, /^env_key\s*=/m);
}

test("Codex writes the current explicit key into TOML without an environment dependency", () => {
  const config = buildSupplierConfigToml(profile({ apiKey: "fixture-current-key", apiKeyExplicit: true }));
  assertInlineToken(config, "fixture-current-key");
  assert.match(config, /^model = "fixture-model"$/m);
  assert.match(config, /^base_url = "https:\/\/fixture.invalid\/v1"$/m);
});

test("Codex migrates credentials retained in auth contents", () => {
  const generated = withSupplierGeneratedFiles(profile({
    authContents: JSON.stringify({ OPENAI_API_KEY: "fixture-auth-key" }),
  }));
  assertInlineToken(generated.configContents, "fixture-auth-key");
  assert.equal(JSON.parse(generated.authContents).OPENAI_API_KEY, "fixture-auth-key");
});

test("Codex retains credentials recovered from existing provider config", () => {
  const generated = withSupplierGeneratedFiles(profile({
    configContents: 'experimental_bearer_token = "fixture-config-key"',
  }));
  assertInlineToken(generated.configContents, "fixture-config-key");
});

test("explicitly clearing a key does not resurrect stale auth or provider credentials", () => {
  const generated = withSupplierGeneratedFiles(profile({
    apiKeyExplicit: true,
    authContents: JSON.stringify({ OPENAI_API_KEY: "fixture-old-auth" }),
    configContents: 'experimental_bearer_token = "fixture-old-config"',
  }));
  assert.doesNotMatch(generated.configContents, /experimental_bearer_token/);
  assert.match(generated.configContents, /^env_key = "OPENAI_API_KEY"$/m);
  assert.equal(generated.authContents, "");
  assert.equal(generated.apiKey, "");
});

test("credential-free profiles preserve environment credential compatibility", () => {
  const config = buildSupplierConfigToml(profile());
  assert.doesNotMatch(config, /experimental_bearer_token/);
  assert.match(config, /^env_key = "OPENAI_API_KEY"$/m);
});

test("inline credentials escape quotes, backslashes and control characters", () => {
  const key = 'fixture-"quote"\\slash\nline\ttab';
  const generated = withSupplierGeneratedFiles(profile({ apiKey: key, apiKeyExplicit: true }));
  assertInlineToken(generated.configContents, key);
  assert.equal(JSON.parse(generated.authContents).OPENAI_API_KEY, key);
});

test("Claude credentials retain their existing JSON configuration format", () => {
  const generated = withSupplierGeneratedFiles(profile({ targetApp: "claude", apiKey: "fixture-claude-key" }));
  assert.equal(JSON.parse(generated.configContents).env.ANTHROPIC_AUTH_TOKEN, "fixture-claude-key");
  assert.doesNotMatch(generated.configContents, /experimental_bearer_token/);
});
