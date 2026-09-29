const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");
const ts = require("../apps/claude-codex-pro-manager/node_modules/typescript");
const root = path.resolve(process.argv[2] || path.join(__dirname, ".."));
const frontend = path.join(root, "apps/claude-codex-pro-manager/src");
const helperPath = path.join(frontend, "lib/overviewUsage.ts");
const exportsObject = {};
if (fs.existsSync(helperPath)) {
  const code = ts.transpileModule(fs.readFileSync(helperPath, "utf8"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText;
  vm.runInNewContext(code, { exports: exportsObject, Date, Math, Map });
}
const { compactMetric, overviewPeriod, overviewTrend, overviewBarPosition } = exportsObject;

test("7/30 day periods use complete local calendar days and adjacent baselines", () => {
  for (const range of ["7d", "30d"]) {
    const now = new Date(2026, 8, 29, 15, 45).getTime();
    const count = range === "7d" ? 7 : 30;
    const period = overviewPeriod(range, now);
    assert.equal(period.boundaries.length, count + 1);
    assert.equal(new Date(period.start).getHours(), 0);
    assert.equal(new Date(period.boundaries.at(-1)).getDate(), 30);
    const previous = overviewPeriod(range, period.start - 1);
    assert.equal(previous.start, period.previousStart);
    assert.equal(previous.boundaries.at(-1), period.start);
  }
});

test("24 hour comparison has 24 adjacent hourly buckets", () => {
  const period = overviewPeriod("24h", new Date(2026, 8, 29, 15, 45).getTime());
  const previous = overviewPeriod("24h", period.start - 1);
  assert.equal(period.boundaries.length, 25);
  assert.equal(previous.start, period.previousStart);
  assert.equal(previous.boundaries.at(-1), period.start);
});

test("trend preserves zero buckets, first/last usage and cache creation", () => {
  const rows = [
    { timestamp_ms: 10, input_tokens: 10, cached_tokens: 20, cache_creation_tokens: 5, output_tokens: 3 },
    { timestamp_ms: 29, input_tokens: 2, cached_tokens: 3, output_tokens: 1, total_tokens: 6 },
    { timestamp_ms: 30, total_tokens: 999 },
  ];
  const trend = overviewTrend(rows, [10, 20, 25, 30]);
  assert.deepEqual(JSON.parse(JSON.stringify(trend)), [
    { requests: 1, tokens: 38, input: 15, cached: 20, output: 3, sessions: 0 },
    { requests: 0, tokens: 0, input: 0, cached: 0, output: 0, sessions: 0 },
    { requests: 1, tokens: 6, input: 2, cached: 3, output: 1, sessions: 0 },
  ]);
});

test("every bar stays inside the chart including first and last slots", () => {
  for (const count of [7, 24, 30]) {
    for (let index = 0; index < count; index++) {
      const bar = overviewBarPosition(index, count);
      assert.ok(bar.left > 0 && bar.left + bar.width < 100);
      assert.ok(bar.center > bar.left && bar.center < bar.left + bar.width);
      assert.ok(bar.width <= 100 / count * .25);
      assert.ok(Math.abs(bar.center - (bar.left + bar.width / 2)) < 1e-10);
    }
  }
});

test("token values switch to B at one billion", () => {
  assert.equal(compactMetric(1_830_000_000), "1.83B");
  assert.equal(compactMetric(1_000_000_000), "1B");
  assert.equal(compactMetric(999_000_000), "999M");
});

test("overview source keeps table, calendar tooltip, cache denominator and one scroll owner", () => {
  const source = fs.readFileSync(path.join(frontend, "screens.tsx"), "utf8");
  const css = fs.readFileSync(path.join(frontend, "components/request-timeline.css"), "utf8");
  assert.ok(source.includes('<table><thead><tr><th>项目名称</th>'));
  assert.ok(source.includes("比较周期 · {baselineLabel}"));
  assert.ok(source.includes('useState<OverviewRange>("7d")'));
  assert.ok(source.includes("(record.input_tokens ?? 0) + (record.cache_creation_tokens ?? 0)"));
  assert.ok(source.includes('className="aitracker-calendar-tooltip"'));
  assert.ok(source.includes('aitracker-calendar-panel" ref={calendarRef}'));
  assert.ok(css.includes(".overview-data-layout .overview-main { display: block;"));
  assert.ok(css.includes("gap: 6px; overflow: visible;"));
  assert.ok(css.includes(".aitracker-project-controls button { display: block; width: auto;"));
  assert.ok(source.includes('style={{ left: `${overviewBarPosition(index, trend.length).center}%` }}'));
  assert.ok(source.includes("{modelUsage.map((item, index) =>"));
  assert.ok(!source.includes("modelUsage.slice(0, 8)"));
  assert.ok(css.includes(".aitracker-trend-replacement .overview-data-chart svg { bottom: 34px;"));
  assert.ok(css.includes(".aitracker-trend-replacement .overview-data-axis { bottom: 0; height: 20px;"));
  assert.ok(css.includes(".aitracker-trend-replacement .overview-data-axis span { position: absolute; top: 0;"));
  assert.ok(css.includes(".aitracker-trend-replacement > footer { margin-top: 8px; }"));
  assert.ok(css.includes(".aitracker-overview-layout .aitracker-project-table table"));
});

test("periodic collection is single-flight and Agent overview reads the latest snapshot", () => {
  const app = fs.readFileSync(path.join(frontend, "App.tsx"), "utf8");
  const agent = fs.readFileSync(path.join(frontend, "components/AgentOverview.tsx"), "utf8");
  assert.ok(app.includes("if (capabilitiesInFlight.current) return null;"));
  assert.ok(!app.includes("Promise.all([refreshRequestTimeline(), refreshAitrackerCapabilities(true)])"));
  const timer = app.slice(app.indexOf('if (route !== "overview") return;'));
  assert.ok(!timer.slice(0, timer.indexOf("}, [route, actions]);")).includes("refreshAitrackerCapabilities(true)"));
  assert.ok(agent.includes("timeline?.usage_snapshot ?? capabilities?.usage"));
});
