import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../apps/claude-codex-pro-manager/", import.meta.url));
const require = createRequire(`${root}/package.json`);
const { createServer } = require("vite");
const React = require("react");
const { renderToStaticMarkup } = require("react-dom/server");
const ts = require("typescript");
const transpile = (source) => ts.transpileModule(source, { compilerOptions: {
  target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX,
} }).outputText;
const emptyLogs = { logs: null, loading: false, error: null, updatedAtMs: null };
const server = await createServer({ root, server: { middlewareMode: true }, appType: "custom" });
try {
  const { RequestTimeline } = await server.ssrLoadModule("/src/components/RequestTimeline.tsx");
  const record = {
    id: "test-1", timestamp_ms: 1789722000000, source: "proxy", agent: "codex",
    provider: "fixture", model: "fixture-model", protocol: "Responses", upstream_protocol: "Chat Completions",
    status: "success", http_status: 200, duration_ms: 1240, first_byte_ms: 120,
    input_tokens: 1000, output_tokens: 100, cached_tokens: 600, cache_creation_tokens: 0,
    reasoning_tokens: 20, total_tokens: 1100, streaming: true,
  };
  const render = (records, extra = {}) => renderToStaticMarkup(React.createElement(RequestTimeline, {
    agentScope: "codex", timelineLogs: emptyLogs, onRefresh: async () => {},
    result: { records, warnings: [], status: "ok", message: "", observed_at_ms: record.timestamp_ms }, ...extra,
  }));
  const second = { ...record, id: "test-2", timestamp_ms: record.timestamp_ms + 1000, model: "second-model", status: "failed", http_status: 429 };
  const html = render([record, second]);
  for (const text of ["fixture-model", "second-model", "1,240 ms", "120 ms", "600", "429", "Provider 请求", "协议与代理", "Agent 响应", "缓存写入", "推理 Token"]) assert.ok(html.includes(text), text);
  for (const name of ["request-row", "request-toolbar", "request-tabs", "request-footer"]) assert.ok(!html.includes('class="' + name + '"'), name);
  assert.equal((html.match(/class="overview-timeline-track"/g) ?? []).length, 3);
  assert.equal((html.match(/class="overview-token-usage"/g) ?? []).length, 1);
  assert.equal((html.match(/data-record=/g) ?? []).length, 6, "each record appears in all three tracks");
  const tokenRow = (html) => html.match(/<div class="overview-token-usage".*?<\/div>/s)?.[0] ?? "";
  assert.ok(tokenRow(html).includes("second-model"), "latest usage selected even when records are unsorted");
  assert.ok(tokenRow(html).includes("输入 <b>1,000</b>"), "single record, not double counted");
  const missing = render([{ ...record, input_tokens: null, output_tokens: null, cached_tokens: null, duration_ms: null, first_byte_ms: null }]);
  assert.ok(tokenRow(missing).includes("输入 <b>未采集</b>"));
  assert.ok(missing.includes("耗时 未采集"));
  assert.ok(render([{ ...record, cached_tokens: 0 }]).includes("缓存读取 <b>0</b>"));
  assert.ok(!render([{ ...record, agent: "claude" }]).includes("fixture-model"));
  const scoped = render([record, { ...second, agent: "claude-desktop" }], { agentScope: "claude" });
  assert.ok(scoped.includes("second-model"));
  assert.ok(!scoped.includes("fixture-model"));
  const noUsage = { ...second, input_tokens: null, output_tokens: null, cached_tokens: null, cache_creation_tokens: null, total_tokens: null };
  assert.ok(tokenRow(render([noUsage, record])).includes("fixture-model"));
  assert.ok(tokenRow(render([noUsage])).includes("暂无用量"));
  const local = { ...second, source: "codex_rollout", status: "observed", http_status: null, protocol: null, duration_ms: null, first_byte_ms: null };
  const mixed = render([record, local]);
  assert.ok(tokenRow(mixed).includes("Codex 用量"));
  assert.ok(tokenRow(mixed).includes("输入 <b>1,000</b>"));
  assert.ok(mixed.includes("HTTP 未观测"));
  assert.ok(mixed.includes("Codex 用量 · 本地记录"));
  assert.ok(mixed.includes('class="overview-timeline-event muted"'));
  const many = render(Array.from({ length: 30 }, (_, index) => ({ ...record, timestamp_ms: record.timestamp_ms + (index % 5) * 60_000, id: `item-${index}` })));
  const chips = [...many.matchAll(/data-record="([^"]+)"[^>]*style="([^"]+)"/g)].map(([, id, style]) => ({
    id, lane: id.split("-").at(-1), ...Object.fromEntries([...style.matchAll(/(left|top|width|height):([\d.]+)px/g)].map(([, key, value]) => [key, Number(value)])),
  }));
  assert.equal(chips.length, 90, "no 25-record pagination truncation");
  for (const chip of chips) {
    assert.ok(Number.isFinite(chip.left) && chip.left >= 0 && chip.left + chip.width <= 640, "chip within chart");
    for (const other of chips.filter((other) => other !== chip && other.lane === chip.lane)) {
      assert.ok(chip.left + chip.width <= other.left || other.left + other.width <= chip.left || chip.top + chip.height <= other.top || other.top + other.height <= chip.top, "chips never overlap");
    }
  }
  for (let index = 0; index < 30; index++) {
    const group = chips.filter((chip) => chip.id.startsWith(`item-${index}-`));
    assert.equal(new Set(group.map((chip) => chip.left)).size, 1, "same request aligned across lanes");
  }
  for (const width of [480, 640, 1200]) {
    const responsiveModule = {};
    new Function("exports", "require", transpile(readFileSync(`${root}/src/components/RequestTimeline.tsx`, "utf8")))(responsiveModule, (name) => {
      if (name.endsWith(".css")) return {};
      if (name === "react") return { ...React, useState: (initial) => React.useState(initial === 640 ? width : initial) };
      return require(name);
    });
    const responsive = renderToStaticMarkup(React.createElement(responsiveModule.RequestTimeline, {
      agentScope: "codex", timelineLogs: emptyLogs, onRefresh: async () => {},
      result: { records: [record], warnings: [], status: "ok", message: "", observed_at_ms: record.timestamp_ms },
    }));
    const axis = responsive.match(/class="overview-timeline-axis">(.*?)<\/div>/s)?.[1] ?? "";
    const ticks = [...axis.matchAll(/left:([\d.]+)px/g)].map(([, value]) => Number(value));
    assert.ok(ticks.length >= 2);
    for (let index = 1; index < ticks.length; index++) assert.ok(ticks[index] - ticks[index - 1] >= 59.9, "time labels keep readable spacing");
    assert.ok(ticks.at(-1) + 224 <= width, "latest event fits the chart");
  }
  const css = readFileSync(`${root}/src/components/request-timeline.css`, "utf8");
  assert.ok(!css.includes(".overview-console-layout"), "original surrounding overview geometry stays intact");
  assert.ok(css.includes("height: 42px; overflow-x: hidden; overflow-y: auto"), "three compact tracks remain visible; collisions scroll inside each track");
  assert.ok(render([]).includes("暂无事件"));
  assert.ok(render([], { result: null }).includes("加载中"));
  const escaped = render([{ ...record, model: "<script>fixture</script>" }]);
  assert.ok(!escaped.includes("<script>fixture</script>"));
  assert.ok(escaped.includes("&lt;script&gt;"));

  const logs = { status: "ok", message: "", path: "fixture", lines: 1,
    text: JSON.stringify({ timestamp_ms: record.timestamp_ms - 120_000, event: "claude.desktop.status", detail: "PRIVATE_FIXTURE" }) };
  const loadedLogs = { logs, loading: false, error: null, updatedAtMs: record.timestamp_ms - 60_000 };
  const eventHtml = render([], { timelineLogs: loadedLogs });
  assert.ok(eventHtml.includes("全局运行事件"));
  assert.ok(eventHtml.includes("claude.desktop.status"));
  assert.ok(!eventHtml.includes("PRIVATE_FIXTURE"));
  assert.ok(render([], { timelineLogs: loadedLogs, agentScope: "claude" }).includes("claude.desktop.status"));
  const failure = "全局运行事件刷新失败，保留上次成功读取的结果。";
  const failed = render([], { timelineLogs: { ...emptyLogs, error: failure } });
  assert.ok(failed.includes('role="alert"'));
  assert.ok(failed.includes("读取失败"));
  const stale = render([record], { timelineLogs: { ...loadedLogs, error: failure } });
  assert.ok(stale.includes("claude.desktop.status") && stale.includes("fixture-model") && stale.includes(failure));
  assert.ok(render([], { result: { status: "failed", message: "request-only-error", records: [], warnings: [], observed_at_ms: 0 } }).includes("request-only-error"));
  const malformed = render([], { timelineLogs: { ...loadedLogs, logs: { ...logs, text: 'null\ninvalid\n' + JSON.stringify({ timestamp_ms: record.timestamp_ms, event: "memory.capture" }) } } });
  assert.ok(!malformed.includes("memory.capture"));
  assert.ok(!malformed.includes('data-record='));

  // Exercise the actual App refresh body, including independent failures.
  const app = readFileSync(`${root}/src/App.tsx`, "utf8");
  const start = app.indexOf("  const refreshRequestTimeline = async () => {");
  const end = app.indexOf("\n  const refreshLogs", start);
  assert.ok(start >= 0 && end > start);
  let timelineState = null;
  let logState = emptyLogs;
  let rejectRequest = false;
  let logResponse = logs;
  const inFlight = { current: false };
  const call = async (command) => {
    if (command === "read_latest_logs") {
      if (logResponse instanceof Error) throw logResponse;
      return logResponse;
    }
    if (rejectRequest) throw new Error("request failed");
    return { status: "ok", records: [record], warnings: [], observed_at_ms: record.timestamp_ms };
  };
  const refresh = new Function("call", "requestTimelineInFlight", "setRequestTimeline", "setTimelineLogs",
    `${transpile(app.slice(start, end))}; return refreshRequestTimeline;`)(call, inFlight,
    (value) => { timelineState = typeof value === "function" ? value(timelineState) : value; },
    (value) => { logState = typeof value === "function" ? value(logState) : value; });
  logResponse = new Error("log read failed");
  await refresh();
  assert.equal(timelineState.status, "ok");
  assert.equal(logState.updatedAtMs, null);
  assert.equal(logState.logs, null);
  assert.equal(logState.error, failure);
  logResponse = logs;
  rejectRequest = true;
  await refresh();
  assert.equal(timelineState.status, "failed");
  assert.equal(logState.logs, logs);
  assert.ok(logState.updatedAtMs > 0);
  assert.equal(logState.error, null);
  const lastLogTime = logState.updatedAtMs;
  for (const response of [new Error("log read failed"), { ...logs, status: "failed" }]) {
    logResponse = response;
    rejectRequest = false;
    await refresh();
    assert.equal(timelineState.status, "ok");
    assert.equal(logState.logs, logs);
    assert.equal(logState.updatedAtMs, lastLogTime);
    assert.equal(logState.error, failure);
    assert.equal(logState.loading, false);
    assert.equal(inFlight.current, false);
  }
  console.log("Request timeline SSR and refresh tests passed: original three-track layout, complete request chips, collision packing, latest usage, scope isolation and independent refresh failures.");
} finally {
  await server.close();
}
