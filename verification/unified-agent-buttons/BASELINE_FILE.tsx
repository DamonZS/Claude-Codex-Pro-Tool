import { type CSSProperties, type Dispatch, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent, type ReactNode, type SetStateAction, memo, useEffect, useId, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { compactMetric, overviewBarPosition, overviewPeriod, overviewTrend } from "./lib/overviewUsage";
import {
  Activity,
  ArrowLeft,
  ChevronLeft,
  ChevronRight,
  AlertTriangle,
  ArchiveRestore,
  BarChart3,
  CheckCircle2,
  Check,
  Database,
  Copy,
  Cpu,
  Download,
  Edit,
  ExternalLink,
  Eye,
  EyeOff,
  FileCode2,
  GripVertical,
  Flame,
  Info,
  KeyRound,
  MessageCircle,
  Network,
  Pencil,
  Play,
  Plus,
  Power,
  RefreshCw,
  Search,
  Save,
  Server,
  ShieldCheck,
  Sparkles,
  Timer,
  Trash2,
  UserRound,
  Wrench,
  X,
  Zap,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { AgentOverview } from "@/components/AgentOverview";
import contactWechatQr from "@/assets/contact-wechat-qr.jpg";
import claudeLogo from "@/assets/claude.svg";
import codexLogo from "@/assets/openai.svg";
import workbuddyLogo from "@/assets/agent-brands/workbuddy.svg";
import cursorLogo from "@/assets/agent-brands/cursor.svg";
import deepseekLogo from "@/assets/agent-brands/deepseek.svg";
import openclawLogo from "@/assets/agent-brands/openclaw.svg";
import {
  AGGREGATE_STRATEGIES,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_LOCAL_SOURCE,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_NAME,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_SOURCE,
  CODEX_MATT_POCOCK_SKILLS_MARKETPLACE_LOCAL_SOURCE,
  CODEX_MATT_POCOCK_SKILLS_MARKETPLACE_NAME,
  CODEX_MATT_POCOCK_SKILLS_MARKETPLACE_SOURCE,
  CODEX_THIRD_PARTY_PLUGIN_MARKETPLACE_NAME,
  CODEX_THIRD_PARTY_PLUGIN_REPOSITORY_URL,
  SUPPLIER_PRESETS,
} from "@/constants";
import type { AppActions } from "@/lib/actions";
import {
  ActionButton,
  Empty,
  InfoRow,
  Panel,
  StatusActionTile,
  StatusRow,
  StatusTile,
  ToggleSwitch,
} from "@/components/ui/ops";
import {
  compactDisplayPath,
  compactPath,
  formatSessionRelativeTime,
  groupClaudeSessionsByProject,
  groupLocalSessionsByProject,
  statusFailed,
  statusOk,
} from "@/lib/helpers";
import {
  aggregateStrategyLabel,
  createAggregateSupplierProfile,
  createSupplierProfile,
  normalizeSupplierProfile,
  redactSupplierConfig,
  supplierIdFromName,
  supplierProfileCanActivate,
  supplierProfileIsCodexOfficialLogin,
  supplierProfileIsCcswitch,
  supplierProtocolLabel,
  supplierRelayModeLabel,
  supplierTargetAppLabel,
  supplierApiFormatLabel,
  supplierApiFormatOption,
  supplierApiFormatRequiresRoute,
  SUPPLIER_API_FORMAT_OPTIONS,
  supplierCodexCatalogJson,
  supplierCodexCatalogModelList,
  supplierCodexCatalogRows,
  type SupplierCodexCatalogRow,
  supplierDirectModelIsClaudeDesktopSafe,
  supplierDirectModelList,
  supplierDirectModelRows,
  type SupplierDirectModelRow,
  supplierModelMappingJson,
  supplierModelMappingRows,
  supplierModelMappingText,
  uniqueSupplierProfileId,
  withSupplierGeneratedFiles,
  withSupplierPreservedImportedFiles,
} from "@/lib/supplier";
import { contextKindLabel, defaultClaudeContextBody, defaultContextToml } from "@/lib/context";
import {
  pluginCanInstall,
  pluginInstallButtonLabel,
  pluginKindLabel,
  pluginStatusLabel,
} from "@/lib/plugin";
import {
  claudeDesktopVersionLabel,
  compactUpdateError,
  displayAssetName,
  formatDownloadBytes,
  updateInfoToRelease,
  updateProgressLabel,
  updateStatusLabel,
  trustedUpdateAssetUrl,
} from "@/lib/update";
import type {
  AitrackerCapabilitiesResult,
  AitrackerSessionDetailResult,
  AitrackerSessionRange,
  BackendSettings,
  ClaudeDesktopDevModeStatusResult,
  ClaudeDesktopMarketplaceStatusResult,
  ClaudeDesktopOrgPluginStatusResult,
  ClaudeDesktopProviderApplyResult,
  ClaudeDesktopProviderPreviewResult,
  CredentialEnvironmentResult,
  ClaudeDesktopResult,
  CodexPluginMarketplaceStatusResult,
  ContextKind,
  DistillationCandidatesResult,
  AitrackerSessionSummary,
  LogsResult,
  RequestTimelineResult,
  RequestRecord,
  MulticaConnectionConfig,
  MulticaConnectionStatus,
  MulticaConnectionStatusResult,
  MulticaConnectionView,
  MulticaConnectionsResult,
  MulticaManagedConnectionUpdate,
  MulticaManagedRuntimePayload,
  MulticaManagedRuntimeResult,
  MulticaRuntimeItem,
  MulticaRuntimeSnapshot,
  MulticaSidecarConfig,
  MulticaSidecarStatus,
  MulticaSnapshotResult,
  OverviewResult,
  PluginCatalogItem,
  PluginHubResult,
  PluginInstallPreviewResult,
  RelayProfile,
  RelayProfileModelsResult,
  SettingsResult,
  Status,
  SupplierPreset,
  SupplierSaveResult,
  SupplierTargetApp,
  UpdateResult,
  UnifiedToolAsset,
  UnifiedToolInventoryResult,
  WatcherResult,
} from "@/types";

type SupplierDirectModelDraftRow = SupplierDirectModelRow & {
  rowId: string;
};

type SupplierCodexCatalogDraftRow = SupplierCodexCatalogRow & {
  rowId: string;
};

const SUPPLIER_USER_AGENT_PRESETS = [
  "claude-cli/2.1.161 (external, cli)",
  "claude-cli/2.1.161",
  "claude-code/1.0.0",
  "claude-code/0.1.0",
  "Kilo-Code/1.0",
] as const;

type OverviewAgentScope = "codex" | "claude";
type OverviewRange = "24h" | "7d" | "30d";
const overviewRangeLabels: Record<OverviewRange, string> = { "24h": "24 小时", "7d": "7 天", "30d": "30 天" };

function overviewRecordTokens(record: RequestRecord) {
  if (record.total_tokens != null) return record.total_tokens;
  return [record.input_tokens, record.output_tokens, record.cached_tokens, record.cache_creation_tokens, record.reasoning_tokens]
    .reduce<number>((sum, value) => sum + (value ?? 0), 0);
}

function smoothTrendPath(values: number[], max: number) {
  const points = values.map((value, index) => ({ x: overviewBarPosition(index, values.length).center, y: 38 - value / max * 32 }));
  if (!points.length) return "";
  if (points.length === 1) return `M ${points[0].x} ${points[0].y}`;
  return points.map((point, index) => {
    if (index === 0) return `M ${point.x} ${point.y}`;
    const previous = points[index - 1];
    const midpoint = (previous.x + point.x) / 2;
    return `C ${midpoint} ${previous.y}, ${midpoint} ${point.y}, ${point.x} ${point.y}`;
  }).join(" ");
}

function overviewLocalDayKey(date: Date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

function overviewSnapshotRecords(usage: RequestTimelineResult["usage_snapshot"]): RequestRecord[] | null {
  const details = usage?.details;
  if (!details) return null;
  return details.map((detail) => {
    const timestamp = Date.parse(detail.timestamp);
    const known = (value: string) => value && value !== "unknown" ? value : null;
    return {
      id: detail.id,
      timestamp_ms: Number.isFinite(timestamp) ? timestamp : 0,
      source: detail.source,
      agent: detail.agent,
      session_id: detail.sessionId,
      project: known(detail.project),
      provider: known(detail.provider),
      model: known(detail.model),
      protocol: null,
      upstream_protocol: null,
      status: detail.status,
      http_status: null,
      duration_ms: detail.durationMs,
      first_byte_ms: null,
      input_tokens: detail.inputTokens,
      output_tokens: detail.outputTokens,
      cached_tokens: detail.cachedInputTokens,
      cache_creation_tokens: detail.cacheCreationInputTokens,
      reasoning_tokens: detail.reasoningOutputTokens,
      total_tokens: detail.totalTokens,
      streaming: false,
    };
  });
}

function OverviewAnalysisPanels({
  records,
  trend,
  previousTrend,
  range,
  axisLabels,
  maxStackedTokens,
  onRefresh,
}: {
  records: RequestRecord[];
  trend: Array<{ requests: number; tokens: number; input: number; cached: number; output: number }>;
  previousTrend: number[];
  range: OverviewRange;
  axisLabels: string[];
  maxStackedTokens: number;
  onRefresh: () => Promise<unknown>;
}) {
  const [hovered, setHovered] = useState<number | null>(null);
  const agentUsage = useMemo(() => {
    const grouped = new Map<string, { tokens: number; events: number }>();
    records.forEach((record) => {
      const key = record.agent || "未采集 Agent";
      const current = grouped.get(key) ?? { tokens: 0, events: 0 };
      current.tokens += overviewRecordTokens(record);
      current.events += 1;
      grouped.set(key, current);
    });
    return [...grouped.entries()].sort((a, b) => b[1].tokens - a[1].tokens).slice(0, 6);
  }, [records]);
  const providerUsage = useMemo(() => {
    const grouped = new Map<string, number>();
    records.forEach((record) => {
      const key = record.provider?.trim() || "未采集 Provider";
      grouped.set(key, (grouped.get(key) ?? 0) + 1);
    });
    return [...grouped.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6);
  }, [records]);
  const hourly = useMemo(() => {
    const cells = Array.from({ length: 28 }, () => 0);
    records.forEach((record) => {
      const date = new Date(record.timestamp_ms);
      const day = (date.getDay() + 6) % 7;
      cells[day * 4 + Math.min(3, Math.floor(date.getHours() / 6))] += 1;
    });
    return cells;
  }, [records]);
  const totalProviderEvents = providerUsage.reduce((sum, [, count]) => sum + count, 0);
  const maxAgentTokens = Math.max(...agentUsage.map(([, item]) => item.tokens), 1);
  const maxHourly = Math.max(...hourly, 1);
  const maxRequests = Math.max(...trend.map((item) => item.requests), 1);
  const maxPrevious = Math.max(...previousTrend, 1);
  const x = (index: number) => index * 100 / Math.max(1, trend.length - 1);
  const y = (value: number, max: number) => 38 - value / max * 32;
  const previousPath = smoothTrendPath(previousTrend, maxPrevious);
  const days = ["一", "二", "三", "四", "五", "六", "日"];
  return <>
    <div className="overview-data-grid-main aitracker-analysis-main">
      <section className="overview-data-panel overview-data-trend aitracker-trend-panel aitracker-trend-replacement">
        <header><div><strong>Token 消耗趋势</strong><small>缓存读取、输入、输出与环比 · 最近 {overviewRangeLabels[range]}</small></div><button type="button" aria-label="刷新数据" title="刷新数据" onClick={() => void onRefresh()}><RefreshCw aria-hidden="true" /></button></header>
        <div className="overview-data-chart"><div className="overview-chart-yaxis"><span>{compactMetric(maxStackedTokens)}</span><span>{compactMetric(Math.round(maxStackedTokens * .75))}</span><span>{compactMetric(Math.round(maxStackedTokens * .5))}</span><span>{compactMetric(Math.round(maxStackedTokens * .25))}</span><span>0</span></div><svg viewBox="0 0 100 42" preserveAspectRatio="none" aria-label="Token 消耗趋势图">
          {trend.map((item, index) => { const width = 2.6; const left = x(index) - width / 2; const outputY = y(item.output, maxStackedTokens); const inputY = outputY - item.input / maxStackedTokens * 32; const cachedY = inputY - item.cached / maxStackedTokens * 32; return <g key={`bar-${index}`} onMouseEnter={() => setHovered(index)} onMouseLeave={() => setHovered(null)}><rect className="trend-bar-cache" x={left} y={cachedY} width={width} height={item.cached / maxStackedTokens * 32} /><rect className="trend-bar-input" x={left} y={inputY} width={width} height={item.input / maxStackedTokens * 32} /><rect className="trend-bar-output" x={left} y={outputY} width={width} height={item.output / maxStackedTokens * 32} /><rect className="trend-hit" x={Math.max(0, left - 2)} y="0" width={width + 4} height="42" /></g>; })}
          <path className="trend-previous" d={previousPath} />
          {hovered != null ? <g className="overview-trend-tooltip" pointerEvents="none"><line x1={x(hovered)} x2={x(hovered)} y1="0" y2="38" /><circle className="token-dot" cx={x(hovered)} cy={y(trend[hovered].tokens, maxStackedTokens)} r="1.4" /><rect x={Math.min(Math.max(2, x(hovered) - 16), 74)} y="2" width="24" height="11" rx="2" /><text x={Math.min(Math.max(2, x(hovered) - 16), 74) + 12} y="6.5" textAnchor="middle">{axisLabels[hovered]}</text><text x={Math.min(Math.max(2, x(hovered) - 16), 74) + 12} y="10.5" textAnchor="middle">{compactMetric(trend[hovered].tokens)} Token · {trend[hovered].requests} 请求</text></g> : null}
        </svg><div className="overview-data-axis">{axisLabels.slice(0, 6).map((label, index) => <span key={`${label}-${index}`}>{label}</span>)}</div></div>
        <footer><span><i className="trend-swatch cache" />缓存读取</span><span><i className="trend-swatch input" />输入</span><span><i className="trend-swatch output" />输出</span><span><i className="trend-swatch previous" />环比</span><em>数据来源：本地采集器</em></footer>
      </section>
      <section className="overview-data-panel overview-data-agent-rank"><header><div><strong>Agent 使用排行</strong><small>按 Token 消耗排序 · {records.length} 个事件</small></div></header><div>{agentUsage.map(([name, item], index) => <div className="overview-agent-rank" key={name}><b>{index + 1}</b><span className="overview-agent-badge">{name.slice(0, 1).toUpperCase()}</span><div><strong>{name}</strong><small>{item.events} 次调用</small><i><em style={{ width: `${Math.max(3, item.tokens / maxAgentTokens * 100)}%` }} /></i></div><strong>{compactMetric(item.tokens)}</strong></div>)}{!agentUsage.length && <p className="overview-data-empty">暂无 Agent 用量记录</p>}</div></section>
    </div>
    <div className="overview-data-grid-lower aitracker-analysis-lower">
      <section className="overview-data-panel overview-data-provider"><header><div><strong>Provider 请求占比</strong><small>按本地请求事件统计</small></div></header><div className="overview-provider-chart"><div className="overview-donut" style={{ background: providerUsage.length ? `conic-gradient(${providerUsage.map(([, count], index) => `${["#41d8c0", "#9d83ff", "#ff9a72", "#e96987", "#7eb8ff", "#f2b856"][index % 6]} ${providerUsage.slice(0, index).reduce((sum, [, value]) => sum + value, 0) / totalProviderEvents * 100}% ${(providerUsage.slice(0, index + 1).reduce((sum, [, value]) => sum + value, 0) / totalProviderEvents * 100)}%`).join(",")}` : undefined }}><span>{totalProviderEvents || "未采集"}<small>请求</small></span></div><div className="overview-provider-legend">{providerUsage.map(([name, count], index) => <div key={name}><i style={{ background: ["#41d8c0", "#9d83ff", "#ff9a72", "#e96987", "#7eb8ff", "#f2b856"][index % 6] }} /><span>{name}</span><b>{totalProviderEvents ? `${(count / totalProviderEvents * 100).toFixed(1)}%` : "未采集"}</b></div>)}</div></div></section>
      <section className="overview-data-panel overview-data-heat"><header><div><strong>Agent 活跃度</strong><small>按星期与时段统计事件</small></div></header><div className="overview-heat-summary"><span>周一</span><span>周日</span></div><div className="overview-heatmap">{hourly.map((count, index) => <i key={index} data-level={count ? Math.max(1, Math.ceil(count / maxHourly * 4)) : 0} title={`${days[Math.floor(index / 4)]} · ${Math.floor(index % 4) * 6}:00 · ${count} 个事件`} />)}</div><footer><span>少</span><i data-level="1" /><i data-level="2" /><i data-level="3" /><i data-level="4" /><span>多</span></footer></section>
      <section className="overview-data-panel overview-data-burning"><header><div><strong>Token 燃烧榜</strong><small>按 Agent Token 消耗排行</small></div></header><div>{agentUsage.slice(0, 5).map(([name, item], index) => <div className="overview-burning-row" key={name}><b>#{index + 1}</b><Flame aria-hidden="true" /><div><strong>{name}</strong><small>{item.events} 次调用 · 本地记录</small></div><strong>{compactMetric(item.tokens)}</strong></div>)}{!agentUsage.length && <p className="overview-data-empty">暂无燃烧数据</p>}</div></section>
    </div>
  </>;
}

function OverviewDataDashboard({ agentScope, range, requestTimeline, aitrackerCapabilities, distillationCandidates, onRefresh }: {
  agentScope: OverviewAgentScope;
  range: OverviewRange;
  requestTimeline: RequestTimelineResult | null;
  aitrackerCapabilities: AitrackerCapabilitiesResult | null;
  distillationCandidates: DistillationCandidatesResult | null;
  onRefresh: () => Promise<unknown>;
}) {
  const [hoveredTrendIndex, setHoveredTrendIndex] = useState<number | null>(null);
  const [selectedAgent, setSelectedAgent] = useState("all");
  const [projectTopN, setProjectTopN] = useState(5);
  const [calendarHover, setCalendarHover] = useState<{ date: string; tokens: number; events: number; sessions: number; left: number; top: number } | null>(null);
  const calendarRef = useRef<HTMLElement>(null);
  const snapshotRecords = useMemo(() => overviewSnapshotRecords(requestTimeline?.usage_snapshot ?? aitrackerCapabilities?.usage), [aitrackerCapabilities, requestTimeline]);
  const sourceRecords = snapshotRecords ?? requestTimeline?.records ?? [];
  const scopedRecords = useMemo(() => aitrackerCapabilities ? sourceRecords : sourceRecords.filter((record) => agentScope === "codex" ? record.agent === "codex" : record.agent.startsWith("claude")), [agentScope, aitrackerCapabilities, sourceRecords]);
  const availableAgents = useMemo(() => {
    const names = new Set<string>();
    scopedRecords.forEach((record) => { if (record.agent) names.add(record.agent); });
    aitrackerCapabilities?.snapshot.registry.filter((item) => item.detected).forEach((item) => names.add(item.id));
    return [...names].sort((a, b) => a.localeCompare(b));
  }, [aitrackerCapabilities, scopedRecords]);
  useEffect(() => {
    if (selectedAgent !== "all" && !availableAgents.includes(selectedAgent)) setSelectedAgent("all");
  }, [availableAgents, selectedAgent]);
  const agentScopedRecords = useMemo(() => selectedAgent === "all" ? scopedRecords : scopedRecords.filter((record) => record.agent === selectedAgent), [selectedAgent, scopedRecords]);
  const now = Date.now();
  const period = overviewPeriod(range, now);
  const periodStart = period.start;
  const records = useMemo(() => agentScopedRecords.filter((record) => record.timestamp_ms >= periodStart && record.timestamp_ms <= now), [agentScopedRecords, now, periodStart]);
  const previousRecords = useMemo(() => agentScopedRecords.filter((record) => record.timestamp_ms >= period.previousStart && record.timestamp_ms < periodStart), [agentScopedRecords, period.previousStart, periodStart]);
  const totalTokens = records.reduce((sum, record) => sum + overviewRecordTokens(record), 0);
  const judgedRecords = records.filter((record) => ["success", "failed", "interrupted"].includes(record.status));
  const successCount = judgedRecords.filter((record) => record.status === "success").length;
  const latencyValues = records.map((record) => record.duration_ms).filter((value): value is number => value != null);
  const averageLatency = latencyValues.length ? latencyValues.reduce((sum, value) => sum + value, 0) / latencyValues.length : null;
  const agentUsage = useMemo(() => {
    const grouped = new Map<string, { tokens: number; events: number; model?: string }>();
    records.forEach((record) => {
      const key = record.agent || "未知 Agent";
      const item = grouped.get(key) ?? { tokens: 0, events: 0, model: record.model ?? undefined };
      item.tokens += overviewRecordTokens(record); item.events += 1; grouped.set(key, item);
    });
    return [...grouped.entries()].sort((a, b) => b[1].tokens - a[1].tokens).slice(0, 5);
  }, [records]);
  const modelUsage = useMemo(() => {
    const grouped = new Map<string, { tokens: number; events: number; sessions: Set<string> }>();
    records.forEach((record) => {
      const key = record.model?.trim() || "未采集模型";
      const item = grouped.get(key) ?? { tokens: 0, events: 0, sessions: new Set<string>() };
      item.tokens += overviewRecordTokens(record);
      item.events += 1;
      if (record.session_id) item.sessions.add(`${record.agent}:${record.session_id}`);
      grouped.set(key, item);
    });
    const total = [...grouped.values()].reduce((sum, item) => sum + item.tokens, 0);
    return [...grouped.entries()]
      .map(([key, item]) => ({ key, ...item, share: total ? item.tokens / total * 100 : 0 }))
      .sort((a, b) => b.tokens - a.tokens);
  }, [records]);
  const projectUsage = useMemo(() => {
    const grouped = new Map<string, { tokens: number; events: number; sessions: Set<string> }>();
    records.forEach((record) => {
      const key = record.project?.trim() || "未采集项目";
      const item = grouped.get(key) ?? { tokens: 0, events: 0, sessions: new Set<string>() };
      item.tokens += overviewRecordTokens(record);
      item.events += 1;
      if (record.session_id) item.sessions.add(`${record.agent}:${record.session_id}`);
      grouped.set(key, item);
    });
    const total = [...grouped.values()].reduce((sum, item) => sum + item.tokens, 0);
    return [...grouped.entries()]
      .map(([key, item]) => ({ key, ...item, share: total ? item.tokens / total * 100 : 0 }))
      .sort((a, b) => b.tokens - a.tokens);
  }, [records]);
  const providerUsage = useMemo(() => {
    const grouped = new Map<string, number>();
    records.forEach((record) => {
      const name = record.provider?.trim() || "未知 Provider";
      grouped.set(name, (grouped.get(name) ?? 0) + 1);
    });
    const sorted = [...grouped.entries()].sort((a, b) => b[1] - a[1]);
    return sorted.length > 4 ? [...sorted.slice(0, 3), ["其他", sorted.slice(3).reduce((sum, [, count]) => sum + count, 0)] as [string, number]] : sorted;
  }, [records]);
  const trend = overviewTrend(records, period.boundaries);
  const previousPeriod = overviewPeriod(range, periodStart - 1);
  const previousTrend = overviewTrend(previousRecords, previousPeriod.boundaries).map((item) => item.tokens);
  const maxStackedTokens = Math.max(...trend.map((item) => item.input + item.cached + item.output), ...previousTrend, 1);
  const calendarCounts = useMemo(() => {
    const grouped = new Map<string, { events: number; tokens: number; sessions: Set<string> }>();
    agentScopedRecords.forEach((record) => {
      if (record.timestamp_ms < now - 365 * 24 * 60 * 60 * 1000 || record.timestamp_ms > now) return;
      const key = overviewLocalDayKey(new Date(record.timestamp_ms));
      const current = grouped.get(key) ?? { events: 0, tokens: 0, sessions: new Set<string>() };
      current.events += 1;
      current.tokens += overviewRecordTokens(record);
      if (record.session_id) current.sessions.add(`${record.agent}:${record.session_id}`);
      grouped.set(key, current);
    });
    return [...grouped.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [agentScopedRecords, now]);
  const calendarMax = Math.max(...calendarCounts.map(([, value]) => value.tokens), 1);
  const hourlyActivity = useMemo(() => {
    const cells = Array.from({ length: 28 }, () => 0);
    records.forEach((record) => {
      const date = new Date(record.timestamp_ms);
      const day = (date.getDay() + 6) % 7;
      const hourBand = Math.min(3, Math.floor(date.getHours() / 6));
      cells[day * 4 + hourBand] += 1;
    });
    return cells;
  }, [records]);
  const maxHourlyActivity = Math.max(...hourlyActivity, 1);
  const calendarDays = useMemo(() => {
    const counts = new Map(calendarCounts);
    const start = new Date(now);
    start.setHours(0, 0, 0, 0);
    start.setDate(start.getDate() - 364);
    return Array.from({ length: 365 }, (_, index) => {
      const date = new Date(start);
      date.setDate(start.getDate() + index);
      const key = overviewLocalDayKey(date);
      return { date: key, events: counts.get(key)?.events ?? 0, tokens: counts.get(key)?.tokens ?? 0, sessions: counts.get(key)?.sessions.size ?? 0 };
    });
  }, [calendarCounts, now]);
  const calendarWeeks = useMemo(() => {
    const first = new Date(`${calendarDays[0].date}T00:00:00`);
    const last = new Date(`${calendarDays[calendarDays.length - 1].date}T00:00:00`);
    first.setDate(first.getDate() - first.getDay());
    last.setDate(last.getDate() + 6 - last.getDay());
    const byDate = new Map(calendarDays.map((day) => [day.date, day]));
    const weeks = [];
    for (let start = first; start <= last; start.setDate(start.getDate() + 7)) {
      const days = Array.from({ length: 7 }, (_, index) => {
        const date = new Date(start);
        date.setDate(start.getDate() + index);
        const key = overviewLocalDayKey(date);
        return { date: key, data: byDate.get(key) ?? null };
      });
      weeks.push(days);
    }
    return weeks;
  }, [calendarDays]);
  const calendarMonthTicks = calendarWeeks.flatMap((week, index) => index === 0 || week[0].date.slice(5, 7) !== calendarWeeks[index - 1][0].date.slice(5, 7) ? [{ index, label: new Date(`${week[0].date}T00:00:00`).toLocaleDateString("zh-CN", { month: "short" }) }] : []);
  const projectTop = projectUsage.slice(0, projectTopN);
  const projectTopShare = projectTop.reduce((sum, item) => sum + item.share, 0);
  const projectRingCircumference = 2 * Math.PI * 42;
  const projectRing = projectTop.reduce<{ segments: { key: string; color: string; length: number; offset: number }[]; offset: number }>((result, item, index) => {
    const length = item.share / 100 * projectRingCircumference;
    const color = ["#41d8c0", "#9d83ff", "#ff9a72", "#e96987", "#7eb8ff"][index % 5];
    result.segments.push({ key: item.key, color, length, offset: result.offset });
    result.offset += length;
    return result;
  }, { segments: [], offset: 0 });
  const displayTotalTokens = records.length ? compactMetric(totalTokens) : "未采集";
  const displayRequests = records.length.toLocaleString("zh-CN");
  const sessionCount = new Set(records.filter((record) => record.session_id).map((record) => `${record.agent}:${record.session_id}`)).size;
  const todayStart = new Date(now); todayStart.setHours(0, 0, 0, 0);
  const todayRecords = agentScopedRecords.filter((record) => record.timestamp_ms >= todayStart.getTime() && record.timestamp_ms <= now);
  const todayTokens = todayRecords.reduce((sum, record) => sum + overviewRecordTokens(record), 0);
  const cachedInputTokens = records.reduce((sum, record) => sum + (record.cached_tokens ?? 0), 0);
  const inputTokens = records.reduce((sum, record) => sum + (record.input_tokens ?? 0) + (record.cache_creation_tokens ?? 0), 0);
  const cacheHitRate = inputTokens + cachedInputTokens > 0 ? cachedInputTokens / (inputTokens + cachedInputTokens) * 100 : null;
  const registry = aitrackerCapabilities?.snapshot.registry ?? [];
  const detectedAgents = registry.filter((item) => item.detected);
  const activeAgentIds = new Set(agentScopedRecords.filter((record) => record.timestamp_ms >= periodStart && record.timestamp_ms <= now).map((record) => record.agent));
  const agentCoverage = aitrackerCapabilities ? detectedAgents.length : new Set(scopedRecords.map((record) => record.agent).filter(Boolean)).size;
  const activeAgentCount = activeAgentIds.size;
  const idleAgentCount = Math.max(0, agentCoverage - activeAgentCount);
  const distillation = distillationCandidates?.candidates ?? null;
  const distillationAssetCount = distillation?.length ?? null;
  const distillationOutputCount = distillation ? distillation.filter((item) => item.status === "approved").length : null;
  const trendText = (current: number, previous: number, suffix = "较上一周期") => previous > 0 ? `${current >= previous ? "↑" : "↓"} ${Math.abs((current - previous) / previous * 100).toFixed(1)}%　${suffix}` : "暂无对比数据";
  const previousTokens = previousRecords.reduce((sum, record) => sum + overviewRecordTokens(record), 0);
  const previousJudgedRecords = previousRecords.filter((record) => ["success", "failed", "interrupted"].includes(record.status));
  const previousSuccess = previousJudgedRecords.length ? previousJudgedRecords.filter((record) => record.status === "success").length / previousJudgedRecords.length * 100 : 0;
  const successRate = judgedRecords.length ? successCount / judgedRecords.length * 100 : null;
  const previousLatencyValues = previousRecords.map((record) => record.duration_ms).filter((value): value is number => value != null);
  const previousLatency = previousLatencyValues.length ? previousLatencyValues.reduce((sum, value) => sum + value, 0) / previousLatencyValues.length : 0;
  const axisLabels = trend.map((_, index) => range === "24h" ? new Date(period.boundaries[index]).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) : new Date(period.boundaries[index]).toLocaleDateString("zh-CN", { month: "numeric", day: "numeric" }));
  const peakIndex = trend.reduce((best, item, index) => item.tokens > (trend[best]?.tokens ?? 0) ? index : best, 0);
  const baselineLabel = `较前 ${range === "24h" ? "24 小时" : range === "7d" ? "7 天" : "30 天"}`;
  const usageDelta = (key: string, current: number, field: "project" | "model") => {
    const previous = previousRecords.filter((record) => record[field] === key).reduce((sum, record) => sum + overviewRecordTokens(record), 0);
    return previous > 0 ? `${current >= previous ? "↑" : "↓"} ${Math.abs((current - previous) / previous * 100).toFixed(1)}%` : current > 0 ? "↑ +100%" : "--";
  };
  const refresh = async () => { await onRefresh(); };
  const calendarSummaryPeriod = overviewPeriod("7d", now);
  const calendarSummaryRecords = agentScopedRecords.filter((record) => record.timestamp_ms >= calendarSummaryPeriod.start && record.timestamp_ms <= now);
  const focusedCalendarCounts = new Map<string, number>();
  calendarSummaryRecords.forEach((record) => {
    const day = overviewLocalDayKey(new Date(record.timestamp_ms));
    focusedCalendarCounts.set(day, (focusedCalendarCounts.get(day) ?? 0) + 1);
  });
  const activeCalendarDays = focusedCalendarCounts.size;
  const calendarTokenTotal = calendarSummaryRecords.reduce((sum, record) => sum + overviewRecordTokens(record), 0);
  let currentStreak = 0;
  let longestCalendarStreak = 0;
  const calendarSummaryStart = new Date(now);
  calendarSummaryStart.setHours(0, 0, 0, 0);
  calendarSummaryStart.setDate(calendarSummaryStart.getDate() - 6);
  for (let index = 0; index < 7; index += 1) {
    const date = new Date(calendarSummaryStart);
    date.setDate(calendarSummaryStart.getDate() + index);
    currentStreak = focusedCalendarCounts.has(overviewLocalDayKey(date)) ? currentStreak + 1 : 0;
    longestCalendarStreak = Math.max(longestCalendarStreak, currentStreak);
  }
  return <div className="overview-data-dashboard aitracker-overview-layout">
    <section className="overview-summary-cards aitracker-spotlight-grid" aria-label="概览摘要">
      <article><div><span>Agent 覆盖</span><strong>{aitrackerCapabilities ? agentCoverage.toLocaleString("zh-CN") : "未采集"}</strong><small>{aitrackerCapabilities ? `${detectedAgents.length} 已检测 · ${activeAgentCount} 活跃 · ${idleAgentCount} 休眠` : "本地采集器未返回 Agent registry"}</small></div><button className="overview-summary-action" type="button" onClick={() => setSelectedAgent("all")}>Agent 视角</button></article>
      <article><div><span>蒸馏资产</span><strong>{distillationAssetCount == null ? "未采集" : distillationAssetCount.toLocaleString("zh-CN")}</strong><small>{distillation ? `${distillation.filter((item) => item.status === "pending").length} 待处理 · ${distillationOutputCount ?? 0} 已产出` : "未加载蒸馏候选"}</small></div><span className="overview-summary-action">本地候选</span></article>
      <article><div><span>今日消费</span><strong>{todayRecords.length ? compactMetric(todayTokens) : "未采集"}</strong><small>{todayRecords.length ? `Token · ${cacheHitRate == null ? "缓存命中率未采集" : `缓存命中率 ${cacheHitRate.toFixed(0)}%`} · 费用未计价` : "本地记录未覆盖今日"}</small></div><span className="overview-summary-action">消费明细</span></article>
    </section>
    <section className="overview-data-kpis overview-kpi-grid" aria-label="概览指标">
      <article><span>Token 消耗</span><strong>{displayTotalTokens}</strong><small className="overview-kpi-trend">{trendText(totalTokens, previousTokens)} · {overviewRangeLabels[range]}</small></article>
      <article><span>费用估算</span><strong>未计价</strong><small className="overview-kpi-trend is-warning">本地记录没有价格来源</small></article>
      <article><span>会话总数</span><strong>{sessionCount ? sessionCount.toLocaleString("zh-CN") : "未采集"}</strong><small className="overview-kpi-trend">{sessionCount ? `按 session_id 去重 · ${overviewRangeLabels[range]}` : "本地记录未提供 session_id"}</small></article>
      <article><span>缓存命中率</span><strong>{cacheHitRate == null ? "未采集" : `${cacheHitRate.toFixed(1)}%`}</strong><small className="overview-kpi-trend">{cacheHitRate == null ? "本地记录未提供缓存 Token" : "按输入与缓存 Token 计算"}</small></article>
      <article><span>Agent 活跃</span><strong>{aitrackerCapabilities ? activeAgentCount.toLocaleString("zh-CN") : (activeAgentCount || "未采集")}</strong><small className="overview-kpi-trend">{aitrackerCapabilities ? `${idleAgentCount} 休眠 · ${overviewRangeLabels[range]}` : "按本地请求记录聚合"}</small></article>
      <article><span>蒸馏产出</span><strong>{distillationOutputCount == null ? "未采集" : distillationOutputCount.toLocaleString("zh-CN")}</strong><small className="overview-kpi-trend">{distillation ? "已审批候选 · 本地状态" : "未加载蒸馏候选"}</small></article>
    </section>
    <div className="overview-agent-filters aitracker-tool-switcher" role="group" aria-label="Agent 筛选"><button type="button" className={selectedAgent === "all" ? "active" : ""} onClick={() => setSelectedAgent("all")}>全部 Agent</button>{availableAgents.map((agent) => <button type="button" key={agent} className={selectedAgent === agent ? "active" : ""} onClick={() => setSelectedAgent(agent)}>{agent}</button>)}</div>
    <section className="overview-data-panel overview-data-trend aitracker-trend-panel aitracker-trend-replacement">
      <header><div><strong>Token 消耗趋势</strong><small>{records.length ? `${range === "24h" ? "时均" : "日均"} ${compactMetric(Math.round(totalTokens / trend.length))}　${trendText(totalTokens, previousTokens)}　峰值 ${axisLabels[peakIndex]} · ${compactMetric(trend[peakIndex].tokens)}` : `暂无 ${overviewRangeLabels[range]} 用量记录`}</small></div><div className="aitracker-trend-header-actions"><span>缓存命中率 {cacheHitRate == null ? "未采集" : `${cacheHitRate.toFixed(0)}%`}</span><button type="button" aria-label="刷新数据" title="刷新数据" onClick={() => void refresh()}><RefreshCw aria-hidden="true" /></button></div></header>
      <div className="overview-data-chart" onMouseLeave={() => setHoveredTrendIndex(null)}>
        <div className="overview-chart-yaxis">{[1, .75, .5, .25, 0].map((scale) => <span key={scale}>{compactMetric(Math.round(maxStackedTokens * scale))}</span>)}</div>
        <svg viewBox="0 0 100 42" preserveAspectRatio="none" aria-label="Token 消耗趋势图">
          {trend.map((item, index) => {
            const { left, width } = overviewBarPosition(index, trend.length);
            const cachedY = 38 - item.cached / maxStackedTokens * 32;
            const inputY = cachedY - item.input / maxStackedTokens * 32;
            const outputY = inputY - item.output / maxStackedTokens * 32;
            return <g key={`trend-bar-${index}`}>
              <rect className="trend-bar-cache" x={left} y={cachedY} width={width} height={item.cached / maxStackedTokens * 32} />
              <rect className="trend-bar-input" x={left} y={inputY} width={width} height={item.input / maxStackedTokens * 32} />
              <rect className="trend-bar-output" x={left} y={outputY} width={width} height={item.output / maxStackedTokens * 32} />
            </g>;
          })}
          <path className="trend-previous" d={smoothTrendPath(previousTrend, maxStackedTokens)} pointerEvents="none" />
          {trend.map((_, index) => <rect key={`trend-hit-${index}`} className="trend-hit" x={index * 100 / trend.length} y="0" width={100 / trend.length} height="42" onMouseEnter={() => setHoveredTrendIndex(index)} />)}
          {hoveredTrendIndex != null && hoveredTrendIndex < trend.length ? <line className="trend-hover-line" x1={overviewBarPosition(hoveredTrendIndex, trend.length).center} x2={overviewBarPosition(hoveredTrendIndex, trend.length).center} y1="0" y2="38" pointerEvents="none" /> : null}
        </svg>
        {hoveredTrendIndex != null && hoveredTrendIndex < trend.length ? <div className="overview-trend-tooltip-html" style={{ left: `${Math.min(82, Math.max(18, overviewBarPosition(hoveredTrendIndex, trend.length).center))}%` }}><strong>{axisLabels[hoveredTrendIndex]}</strong><span>Token 消耗　{compactMetric(trend[hoveredTrendIndex].tokens)}</span><span>缓存读取　{compactMetric(trend[hoveredTrendIndex].cached)}</span><span>输入　{compactMetric(trend[hoveredTrendIndex].input)}</span><span>输出　{compactMetric(trend[hoveredTrendIndex].output)}</span><span>会话总数　{trend[hoveredTrendIndex].sessions}</span></div> : null}
        <div className="overview-data-axis">{axisLabels.map((label, index) => <span key={`${label}-${index}`} style={{ left: `${overviewBarPosition(index, trend.length).center}%` }}>{range === "30d" && index % 5 !== 0 && index !== 29 || range === "24h" && index % 4 !== 0 && index !== 23 ? "" : label}</span>)}</div>
      </div>
      <footer><span><i className="trend-swatch cache" />缓存读取</span><span><i className="trend-swatch input" />输入</span><span><i className="trend-swatch output" />输出</span><span><i className="trend-swatch previous" />vs 上一区间</span><em>比较周期 · {baselineLabel}</em></footer>
    </section>
    <section className="overview-data-panel aitracker-model-panel">
      <header><div><strong>模型消耗</strong><small>{modelUsage.length ? `${modelUsage.length} 个模型 · 按使用量排序 · ${overviewRangeLabels[range]}` : "暂无模型用量记录"}</small></div><span className="overview-panel-meta">按 Token</span></header>
      <div className="aitracker-model-list">
        {modelUsage.map((item, index) => <div className="aitracker-model-row" key={item.key}>
          <div className="aitracker-model-heading"><strong>{item.key}</strong><span>{item.events.toLocaleString("zh-CN")} 次调用 · {item.sessions.size || "未采集"} 会话</span><b>{compactMetric(item.tokens)}</b><span>{item.share.toFixed(1)}%</span><span>未计价</span><span>{usageDelta(item.key, item.tokens, "model")}</span></div>
          <div className="aitracker-model-meter"><i style={{ width: `${modelUsage[0]?.tokens ? Math.max(2, item.tokens / modelUsage[0].tokens * 100) : 0}%`, background: ["#41d8c0", "#9d83ff", "#ff9a72", "#e96987"][index % 4] }} /></div>
        </div>)}
        {!modelUsage.length && <p className="overview-data-empty">暂无模型用量记录</p>}
      </div>
    </section>
    <section className="overview-data-panel aitracker-project-panel">
      <header><div><strong>项目消耗总览</strong><small>{projectUsage.length ? `${projectUsage.length} 个项目 · ${overviewRangeLabels[range]}` : "暂无项目用量记录"}</small></div><div className="aitracker-project-controls"><span className="overview-panel-meta">比较周期 · {baselineLabel}</span><div className="aitracker-project-segments">{([3, 5, 10] as const).map((count) => <button key={count} type="button" className={projectTopN === count ? "active" : ""} onClick={() => setProjectTopN(count)}>TOP {count}</button>)}</div></div></header>
      <div className="aitracker-project-layout">
        <div className="aitracker-project-summary"><div className="aitracker-project-ring"><svg viewBox="0 0 100 100" aria-hidden="true"><circle cx="50" cy="50" r="42" fill="none" stroke="rgb(190 210 225 / .32)" strokeWidth="10" />{projectRing.segments.map((segment) => <circle key={segment.key} cx="50" cy="50" r="42" fill="none" stroke={segment.color} strokeWidth="10" strokeDasharray={`${Math.max(0, segment.length - 1.4)} ${projectRingCircumference}`} strokeDashoffset={-segment.offset} />)}</svg><span>{projectUsage.length.toLocaleString("zh-CN")}<small>项目</small></span></div><div className="aitracker-project-total"><span>总消耗额度</span><b>{compactMetric(totalTokens)}</b></div><div className="aitracker-project-share-meter"><i style={{ width: `${projectTopShare}%` }} /></div><div className="aitracker-project-share">TOP {projectTopN} 占比 <b>{projectTopShare.toFixed(1)}%</b></div></div>
        <div className="aitracker-project-table">
          <table><thead><tr><th>项目名称</th><th>消耗占比</th><th>消耗额度</th><th>会话</th><th>环比</th></tr></thead><tbody>
            {projectTop.map((item, index) => <tr key={item.key}><td><div><i style={{ background: ["#41d8c0", "#9d83ff", "#ff9a72", "#e96987", "#7eb8ff"][index % 5] }} /><span title={item.key}>{item.key}</span></div></td><td><b>{item.share.toFixed(1)}%</b></td><td>{compactMetric(item.tokens)}</td><td>{item.sessions.size || "未采集"}</td><td>{usageDelta(item.key, item.tokens, "project")}</td></tr>)}
            {projectUsage.length > projectTopN && <tr className="aitracker-project-other"><td>其他 {projectUsage.length - projectTopN} 个项目</td><td>{Math.max(0, 100 - projectTopShare).toFixed(1)}%</td><td>--</td><td>--</td><td>--</td></tr>}
          </tbody></table>
          {!projectUsage.length && <p className="overview-data-empty">暂无项目用量记录</p>}
        </div>
      </div>
      <footer className="aitracker-project-footer"><span>共 {projectUsage.length} 个项目</span><button type="button" onClick={() => setProjectTopN(10)}>查看项目消耗明细 →</button></footer>
    </section>
    <section className="overview-data-panel aitracker-calendar-panel" ref={calendarRef}>
      <header><div className="aitracker-calendar-heading"><strong>活跃日历 <small className="aitracker-calendar-period">近 12 个月</small></strong><span className="aitracker-calendar-range-chip">近 7 天</span></div><span className="overview-panel-meta">{activeCalendarDays} 天活跃 · 最长连续 {longestCalendarStreak} 天 · 合计 {compactMetric(calendarTokenTotal)} tokens</span></header>
      <div className="aitracker-calendar-chart">
        <div className="aitracker-calendar-month-labels">{calendarMonthTicks.map(({ index, label }) => <span key={index} style={{ left: `${index / calendarWeeks.length * 100}%` }}>{label}</span>)}</div>
        <div className="aitracker-calendar-body">
          <div className="aitracker-calendar-weekdays"><span /><span>一</span><span /><span>三</span><span /><span>五</span><span /></div>
          <div className="aitracker-calendar-grid" style={{ gridTemplateColumns: `repeat(${calendarWeeks.length}, minmax(0, 1fr))`, gridTemplateRows: "repeat(7, auto)" }}>{calendarWeeks.flat().map(({ date, data }) => <span key={date} data-level={data?.events ? Math.max(1, Math.ceil(data.tokens / calendarMax * 4)) : 0} data-outside={date < overviewLocalDayKey(new Date(periodStart))} data-padding={!data} title={data ? `${date} · ${compactMetric(data.tokens)} · ${data.events} 用量事件` : undefined} onMouseEnter={data ? (event) => {
            const rect = event.currentTarget.getBoundingClientRect();
            const container = calendarRef.current?.getBoundingClientRect();
            if (container) setCalendarHover({ ...data, left: rect.left + rect.width / 2 - container.left, top: rect.top - container.top });
          } : undefined} onMouseLeave={() => setCalendarHover(null)} />)}</div>
        </div>
      </div>
      {calendarHover ? <div className="aitracker-calendar-tooltip" style={{ left: `${Math.min(82, Math.max(18, calendarHover.left / Math.max(1, calendarRef.current?.clientWidth ?? 1) * 100))}%`, top: `${calendarHover.top - 8}px` }}><div><span>{calendarHover.date.replaceAll("-", "/")}</span><b>Level {calendarHover.events ? Math.max(1, Math.ceil(calendarHover.tokens / calendarMax * 4)) : 0}</b></div><strong>{compactMetric(calendarHover.tokens)}</strong><small>{calendarHover.events.toLocaleString("zh-CN")} 用量事件　会话总数 {calendarHover.sessions}</small></div> : null}
      <footer className="aitracker-calendar-legend"><span className="aitracker-calendar-note">颜色深浅表示当日 Token 消耗量</span><span>少</span><i data-level="0" /><i data-level="1" /><i data-level="2" /><i data-level="3" /><i data-level="4" /><span>多</span></footer>
      {!calendarCounts.length && <p className="overview-data-empty">暂无活跃事件</p>}
    </section>
  </div>;
}

export function OverviewScreen({
  actions,
  agentScope,
  aitrackerCapabilities,
  distillationCandidates,
  requestTimeline,
}: {
  actions: AppActions;
  agentScope: OverviewAgentScope;
  aitrackerCapabilities: AitrackerCapabilitiesResult | null;
  distillationCandidates: DistillationCandidatesResult | null;
  requestTimeline: RequestTimelineResult | null;
}) {
  const [overviewView, setOverviewView] = useState<"total" | "agent">("total");
  const [overviewRange, setOverviewRange] = useState<OverviewRange>("7d");
  return (
    <div className="overview-control-plane">
      <div className="overview-console-layout overview-data-layout">
        <main className={overviewView === "agent" ? "overview-main overview-agent-scroll" : "overview-main"}>
          <div className="overview-view-toolbar"><div className="overview-view-tabs" role="group" aria-label="概览视图"><button type="button" className={overviewView === "total" ? "active" : ""} aria-pressed={overviewView === "total"} onClick={() => setOverviewView("total")}>总览</button><button type="button" className={overviewView === "agent" ? "active" : ""} aria-pressed={overviewView === "agent"} onClick={() => setOverviewView("agent")}>Agent 概览</button></div>{overviewView === "total" && <div className="overview-data-range" role="group" aria-label="概览时间范围">{(Object.keys(overviewRangeLabels) as OverviewRange[]).map((key) => <button key={key} className={overviewRange === key ? "active" : ""} type="button" onClick={() => setOverviewRange(key)}>{overviewRangeLabels[key]}</button>)}</div>}</div>
          {overviewView === "total" ? <OverviewDataDashboard agentScope={agentScope} range={overviewRange} requestTimeline={requestTimeline} aitrackerCapabilities={aitrackerCapabilities} distillationCandidates={distillationCandidates} onRefresh={actions.refreshRequestTimeline} /> : <AgentOverview capabilities={aitrackerCapabilities} timeline={requestTimeline} />}
        </main>
      </div>
    </div>
  );
}

const CONTACT_QQ_GROUP_PRIMARY_URL = "https://qm.qq.com/cgi-bin/qm/qr?k=uwNon9opx0Arfovyo5qJQQ2jUvlxSpmf&jump_from=webapi&authKey=El8Xwz9ZqefrpE4BhW9xWQsEAUFvptw74MBsRKRJTw5x5QiEPiG0fmdVIf9VuMWg";
function SupplierModelDropdown({
  options,
  value,
  placeholder,
  onChange,
  compact = false,
  iconOnly = false,
  showAvailabilityWarning = true,
  triggerLabel,
}: {
  options: string[];
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  compact?: boolean;
  iconOnly?: boolean;
  showAvailabilityWarning?: boolean;
  triggerLabel?: string;
}) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [position, setPosition] = useState<CSSProperties>({});
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const menuId = useId();
  const valueAvailable = !showAvailabilityWarning || !value || options.includes(value);

  const openMenu = (direction: 1 | -1 = 1) => {
    const selectedIndex = options.indexOf(value);
    setActiveIndex(selectedIndex >= 0 ? selectedIndex : direction > 0 ? (options.length ? 0 : -1) : options.length - 1);
    setOpen(true);
  };

  const closeMenu = (restoreFocus = false) => {
    setOpen(false);
    if (restoreFocus) requestAnimationFrame(() => triggerRef.current?.focus());
  };

  const selectOption = (option: string) => {
    onChange(option);
    closeMenu(true);
  };

  const handleTriggerKeyDown = (event: ReactKeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const direction = event.key === "ArrowDown" ? 1 : -1;
      if (!open) {
        openMenu(direction);
        return;
      }
      setActiveIndex((index) => {
        if (!options.length) return -1;
        const start = index < 0 ? (direction > 0 ? -1 : 0) : index;
        return (start + direction + options.length) % options.length;
      });
      return;
    }
    if (event.key === "Home" && open) {
      event.preventDefault();
      setActiveIndex(options.length ? 0 : -1);
      return;
    }
    if (event.key === "End" && open) {
      event.preventDefault();
      setActiveIndex(options.length - 1);
      return;
    }
    if ((event.key === "Enter" || event.key === " ") && open) {
      event.preventDefault();
      const option = options[activeIndex];
      if (option) selectOption(option);
      return;
    }
    if (event.key === "Escape" && open) {
      event.preventDefault();
      closeMenu(true);
    }
  };

  useEffect(() => {
    if (!open) return;
    const updatePosition = () => {
      const trigger = triggerRef.current;
      if (!trigger) return;
      const anchor = iconOnly ? trigger.closest<HTMLElement>(".supplier-model-input-dropdown") ?? trigger : trigger;
      const rect = anchor.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;
      const gap = 8;
      const spaceBelow = Math.max(0, viewportHeight - rect.bottom - gap);
      const spaceAbove = Math.max(0, rect.top - gap);
      const opensUp = spaceBelow < 180 && spaceAbove > spaceBelow;
      const availableSpace = opensUp ? spaceAbove : spaceBelow;
      const width = Math.min(Math.max(rect.width, 280), Math.max(160, viewportWidth - 16));
      const left = Math.min(Math.max(8, rect.left), Math.max(8, viewportWidth - width - 8));
      setPosition({
        left,
        width,
        maxHeight: Math.max(44, Math.min(320, availableSpace)),
        ...(opensUp
          ? { bottom: Math.max(8, viewportHeight - rect.top + gap) }
          : { top: Math.min(viewportHeight - 8, rect.bottom + gap) }),
      });
    };
    const closeOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!triggerRef.current?.contains(target) && !menuRef.current?.contains(target)) setOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) {
        event.preventDefault();
        closeMenu(true);
      }
    };
    updatePosition();
    document.addEventListener("pointerdown", closeOnOutsidePointer, true);
    document.addEventListener("keydown", closeOnEscape, true);
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      document.removeEventListener("pointerdown", closeOnOutsidePointer, true);
      document.removeEventListener("keydown", closeOnEscape, true);
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [iconOnly, open]);

  useEffect(() => {
    if (!open || activeIndex < 0) return;
    menuRef.current
      ?.querySelector<HTMLElement>(`[data-option-index="${activeIndex}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, open]);

  const menu = open ? createPortal(
    <div aria-label={triggerLabel || placeholder} className="supplier-model-dropdown-menu" id={menuId} ref={menuRef} style={{ ...position, position: "fixed" }} role="listbox">
      {!valueAvailable && value ? <div className="supplier-model-dropdown-warning">当前配置不可用：{value}</div> : null}
      {options.length ? options.map((option, index) => (
        <button
          aria-selected={option === value}
          className={`${option === value ? "selected" : ""}${index === activeIndex ? " active" : ""}`.trim()}
          data-option-index={index}
          id={`${menuId}-option-${index}`}
          key={option}
          onClick={() => selectOption(option)}
          onMouseDown={(event) => event.preventDefault()}
          role="option"
          tabIndex={-1}
          title={option}
          type="button"
        >{option}</button>
      )) : <div className="supplier-model-dropdown-empty">暂无可用模型</div>}
    </div>,
    document.body,
  ) : null;

  return (
    <div className={`supplier-model-dropdown ${compact ? "compact" : ""} ${iconOnly ? "icon-only" : ""}`}>
      <button
        aria-activedescendant={open && activeIndex >= 0 ? `${menuId}-option-${activeIndex}` : undefined}
        aria-controls={menuId}
        aria-label={triggerLabel || placeholder}
        aria-expanded={open}
        aria-haspopup="listbox"
        className={`supplier-model-dropdown-trigger ${!valueAvailable ? "unavailable" : ""}`}
        onClick={() => open ? closeMenu() : openMenu()}
        onKeyDown={handleTriggerKeyDown}
        ref={triggerRef}
        type="button"
      >
        {iconOnly ? <span aria-hidden="true">▾</span> : <><span>{triggerLabel || value || placeholder}</span><span aria-hidden="true">▾</span></>}
      </button>
      {!valueAvailable && value ? <small className="supplier-model-dropdown-warning-inline">当前配置不可用</small> : null}
      {menu}
    </div>
  );
}

function credentialEnvironmentScopeLabel(scope: string): string {
  switch (scope) {
    case "windows-user-environment":
      return "当前 CCP 进程和 Windows 当前用户环境";
    case "macos-launchd-user-session":
      return "当前 CCP 进程和 macOS launchd 用户会话";
    case "linux-systemd-user-manager":
      return "当前 CCP 进程和 Linux systemd 用户会话";
    default:
      return "当前 CCP 进程和可管理的用户会话环境";
  }
}

export function SupplierScreen({
  // 供应商主列表 / 编辑 / 聚合配置
  actions,
  settings,
  claudeDesktopDevMode,
  claudeDesktopProviderPreview,
  claudeDesktopProviderApply,
  claudeDesktopProviderDraft,
  credentialEnvironment,
  focusProfileId,
  onClaudeDesktopProviderDraftChange,
}: {
  actions: AppActions;
  settings: SettingsResult | null;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeDesktopProviderPreview: ClaudeDesktopProviderPreviewResult | null;
  claudeDesktopProviderApply: ClaudeDesktopProviderApplyResult | null;
  credentialEnvironment: CredentialEnvironmentResult | null;
  focusProfileId?: string | null;
  claudeDesktopProviderDraft: {
    name: string;
    baseUrl: string;
    apiKey: string;
    modelList: string;
  };
  onClaudeDesktopProviderDraftChange: Dispatch<SetStateAction<{
    name: string;
    baseUrl: string;
    apiKey: string;
    modelList: string;
  }>>;
}) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState<RelayProfile | null>(null);
  const [modelFetch, setModelFetch] = useState<RelayProfileModelsResult | null>(null);
  const [supplierSaveBusy, setSupplierSaveBusy] = useState(false);
  const [supplierRouteToggleBusy, setSupplierRouteToggleBusy] = useState(false);
  const [supplierRefreshBusy, setSupplierRefreshBusy] = useState(false);
  const [credentialEnvironmentBusy, setCredentialEnvironmentBusy] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [showSupplierApiKey, setShowSupplierApiKey] = useState(false);
  const [supplierTestConfigOpen, setSupplierTestConfigOpen] = useState(false);
  const [supplierPricingConfigOpen, setSupplierPricingConfigOpen] = useState(false);
  const [supplierDirectModelsOpen, setSupplierDirectModelsOpen] = useState(true);
  const [supplierDirectModels, setSupplierDirectModels] = useState<SupplierDirectModelDraftRow[]>([]);
  const [supplierCodexCatalogModels, setSupplierCodexCatalogModels] = useState<SupplierCodexCatalogDraftRow[]>([]);
  const [supplierTargetFilter, setSupplierTargetFilter] = useState<SupplierTargetApp>("codex");
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const [dragOverId, setDragOverId] = useState<string | null>(null);
  const [supplierOrderIds, setSupplierOrderIds] = useState<string[]>([]);
  const [supplierDragOverlay, setSupplierDragOverlay] = useState<{
    profileId: string;
    top: number;
    left: number;
    width: number;
    height: number;
    offsetY: number;
  } | null>(null);
  const supplierCardRefs = useRef<Map<string, HTMLDivElement>>(new Map());
  const lastFocusedProfileIdRef = useRef<string | null>(null);
  const supplierRouteToggleInFlightRef = useRef(false);
  const supplierModelFetchRequestRef = useRef(0);
  const supplierDirectModelRowIdRef = useRef(0);
  const supplierCodexCatalogRowIdRef = useRef(0);
  const supplierPointerDragRef = useRef<{
    sourceId: string;
    latestIds: string[];
    lastTargetId: string | null;
  } | null>(null);
  const appSettings = settings?.settings ?? null;
  const profiles = useMemo(() => appSettings?.relayProfiles ?? [], [appSettings]);
  const profileIdsKey = profiles.map((profile) => profile.id).join("\u001f");
  const editingExisting = draft && editingId ? profiles.find((profile) => profile.id === editingId) : null;
  const isNewDraft = !!draft && !editingExisting;
  const aggregateProfiles = useMemo(() => profiles.filter((profile) => profile.aggregateEnabled), [profiles]);
  const apiProfiles = useMemo(() => profiles.filter((profile) => !profile.aggregateEnabled && profile.relayMode !== "official"), [profiles]);
  const supplierTargetForProfile = (profile: RelayProfile): SupplierTargetApp => profile.targetApp || "codex";
  const activeSupplierIdForTarget = (targetApp: SupplierTargetApp) => {
    if (!appSettings) return "";
    return targetApp === "claude"
      ? appSettings.activeClaudeRelayId
      : targetApp === "claude-desktop"
        ? appSettings.activeClaudeDesktopRelayId
        : appSettings.activeRelayId;
  };
  const supplierRoutingEnabledForTarget = (targetApp: SupplierTargetApp, sourceProfiles = profiles) => {
    return sourceProfiles.some((profile) => supplierTargetForProfile(profile) === targetApp && !!profile.routeEnabled);
  };
  const withSupplierRoutingState = (profile: RelayProfile, targetApp: SupplierTargetApp, enabled: boolean) => {
    const claudeDesktopMode = targetApp === "codex" ? "" : enabled ? "proxy" : "direct";
    let configContents = profile.configContents ?? "";
    if (targetApp !== "codex") {
      let config: Record<string, unknown> = {};
      try {
        const parsed: unknown = JSON.parse(configContents);
        if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
          config = { ...parsed as Record<string, unknown> };
        }
      } catch {
        // Invalid imported JSON is replaced with the current form state below.
      }
      const existingMeta = config.meta;
      const meta = existingMeta && typeof existingMeta === "object" && !Array.isArray(existingMeta)
        ? { ...existingMeta as Record<string, unknown> }
        : {};
      delete meta.claude_desktop_mode;
      meta.claudeDesktopMode = claudeDesktopMode;
      config.meta = meta;
      configContents = `${JSON.stringify(config, null, 2)}\n`;
    }
    return normalizeSupplierProfile({
      ...profile,
      targetApp,
      configContents,
      routeEnabled: enabled,
      claudeDesktopMode,
      routeMode: targetApp === "codex"
        ? (enabled ? "Codex Proxy" : "Codex Direct")
        : (enabled ? "Claude Desktop Proxy" : "Claude Desktop Direct"),
    });
  };
  const withActiveSupplierId = (current: BackendSettings, targetApp: SupplierTargetApp, profileId: string): BackendSettings => {
    if (targetApp === "claude") return { ...current, activeClaudeRelayId: profileId };
    if (targetApp === "claude-desktop") return { ...current, activeClaudeDesktopRelayId: profileId };
    return { ...current, activeRelayId: profileId };
  };
  const updateClaudeDraft = (field: keyof typeof claudeDesktopProviderDraft, value: string) => {
    onClaudeDesktopProviderDraftChange((current) => ({ ...current, [field]: value }));
  };
  const createSupplierDirectModelRows = (rows: SupplierDirectModelRow[]): SupplierDirectModelDraftRow[] => rows.map((row) => ({
    ...row,
    rowId: `direct-model-${supplierDirectModelRowIdRef.current += 1}`,
  }));
  const createSupplierCodexCatalogModelRows = (rows: SupplierCodexCatalogRow[]): SupplierCodexCatalogDraftRow[] => rows.map((row) => ({
    ...row,
    rowId: `codex-catalog-${supplierCodexCatalogRowIdRef.current += 1}`,
  }));
  useEffect(() => {
    setSupplierOrderIds(profiles.map((profile) => profile.id));
  }, [profileIdsKey]);
  const saveSupplierSettings = async (next: BackendSettings) => {
    const result = await actions.saveSettings(next);
    if (!result) return null;
    if (!statusOk(result.status)) {
      actions.showNotice({ title: "供应商保存", message: result.message || "保存设置失败。", status: "failed" });
      return null;
    }
    return result.settings;
  };
  const openProfileEditor = (profile: RelayProfile) => {
    supplierModelFetchRequestRef.current += 1;
    setModelFetch(null);
    setShowSupplierApiKey(false);
    setSupplierDirectModels(createSupplierDirectModelRows(supplierDirectModelRows(profile.modelList)));
    setSupplierCodexCatalogModels(createSupplierCodexCatalogModelRows(supplierCodexCatalogRows(profile)));
    setSupplierDirectModelsOpen(true);
    setEditingId(profile.id);
    const targetApp = supplierTargetForProfile(profile);
    setDraft(withSupplierRoutingState(profile, targetApp, !!profile.routeEnabled));
  };
  useEffect(() => {
    if (!focusProfileId || lastFocusedProfileIdRef.current === focusProfileId) return;
    const profile = profiles.find((item) => item.id === focusProfileId);
    if (!profile) return;
    lastFocusedProfileIdRef.current = focusProfileId;
    setSupplierTargetFilter(supplierTargetForProfile(profile));
    openProfileEditor(profile);
  }, [focusProfileId, profileIdsKey]);
  const createProfile = () => {
    if (!appSettings) return;
    supplierModelFetchRequestRef.current += 1;
    setModelFetch(null);
    setShowSupplierApiKey(false);
    setEditingId(null);
    const targetApp = supplierTargetFilter;
    const profile = withSupplierRoutingState({
      ...createSupplierProfile(appSettings),
      id: uniqueSupplierProfileId(appSettings.relayProfiles, "provider"),
      name: "供应商",
      targetApp,
    }, targetApp, supplierRoutingEnabledForTarget(targetApp));
    setSupplierDirectModels(createSupplierDirectModelRows(supplierDirectModelRows(profile.modelList)));
    setSupplierCodexCatalogModels(createSupplierCodexCatalogModelRows(supplierCodexCatalogRows(profile)));
    setSupplierDirectModelsOpen(true);
    setDraft(profile);
  };
  const createAggregateProfile = () => {
    if (!appSettings) return;
    supplierModelFetchRequestRef.current += 1;
    const profile = createAggregateSupplierProfile(appSettings);
    setModelFetch(null);
    setShowSupplierApiKey(false);
    setEditingId(null);
    setDraft(profile);
    if (!apiProfiles.length) {
      actions.showNotice({ title: "添加聚合供应商", message: "已打开聚合供应商详情；请先添加或选择至少 1 个普通 API 供应商的 Base URL / Key，再勾选为成员。", status: "failed" });
    }
  };
  const duplicateProfile = (profile: RelayProfile) => {
    if (!appSettings) return;
    supplierModelFetchRequestRef.current += 1;
    setShowSupplierApiKey(false);
    const targetApp = supplierTargetForProfile(profile);
    const copy = {
      ...withSupplierRoutingState(profile, targetApp, !!profile.routeEnabled),
      id: uniqueSupplierProfileId(appSettings.relayProfiles, `${profile.id || "provider"}-copy`),
      name: `${profile.name || profile.id || "供应商"} 副本`,
    };
    setModelFetch(null);
    setSupplierDirectModels(createSupplierDirectModelRows(supplierDirectModelRows(copy.modelList)));
    setSupplierCodexCatalogModels(createSupplierCodexCatalogModelRows(supplierCodexCatalogRows(copy)));
    setSupplierDirectModelsOpen(true);
    setEditingId(null);
    setDraft(copy);
  };
  const normalizeDraftProfile = (profile: RelayProfile) => supplierProfileIsCcswitch(profile)
    ? normalizeSupplierProfile(profile)
    : normalizeSupplierProfile(withSupplierGeneratedFiles(profile));
  const updateDraft = (patch: Partial<RelayProfile>) => {
    supplierModelFetchRequestRef.current += 1;
    setDraft((current) => current ? normalizeDraftProfile({ ...current, ...patch }) : current);
  };
  // Keep the name field editable without regenerating the profile on every
  // keystroke. Generated config normalization can otherwise race the input
  // value and make imported supplier names appear immutable.
  const updateSupplierName = (name: string) => {
    setDraft((current) => current ? { ...current, name } : current);
  };
  const closeSupplierEditor = () => {
    supplierModelFetchRequestRef.current += 1;
    setModelFetch(null);
    setDraft(null);
    setEditingId(null);
    setShowSupplierApiKey(false);
    setSupplierCodexCatalogModels([]);
  };
  const refreshCredentialEnvironment = async () => {
    if (credentialEnvironmentBusy) return;
    setCredentialEnvironmentBusy(true);
    try {
      await actions.diagnoseCodexCredentialEnvironment(false);
    } finally {
      setCredentialEnvironmentBusy(false);
    }
  };
  const clearCredentialEnvironment = async () => {
    if (!credentialEnvironment?.canClearUser || credentialEnvironmentBusy) return;
    const variableName = credentialEnvironment.variableName;
    const scopeLabel = credentialEnvironmentScopeLabel(credentialEnvironment.userScope);
    if (!window.confirm(`确认从${scopeLabel}删除环境变量「${variableName}」？不会删除 auth.json 凭据、修改 CODEX_HOME 或系统级环境变量。`)) return;
    setCredentialEnvironmentBusy(true);
    try {
      await actions.clearCodexUserCredentialEnvironment(variableName);
    } finally {
      setCredentialEnvironmentBusy(false);
    }
  };
  const updateNewDraftIdFromName = (value: string) => {
    if (!isNewDraft) return;
    supplierModelFetchRequestRef.current += 1;
    setDraft((current) => {
      if (!current) return current;
      const nextId = uniqueSupplierProfileId(profiles, value || current.name);
      const next = normalizeDraftProfile({ ...current, id: nextId });
      return normalizeSupplierProfile(next);
    });
  };
  const updateSupplierModelMapping = (role: string, field: "routeId" | "displayName" | "requestModel" | "supports1m", value: string | boolean) => {
    if (!draft) return;
    const rows = supplierModelMappingRows(draft).map((row) => row.role === role ? { ...row, [field]: value } : row);
    updateDraft({
      modelMappingEnabled: true,
      modelMappingJson: supplierModelMappingJson(rows),
      modelMapping: supplierModelMappingText(rows),
    });
  };
  const writeSupplierDirectModels = (rows: SupplierDirectModelDraftRow[]) => {
    setSupplierDirectModels(rows);
    const modelList = supplierDirectModelList(rows);
    const firstModel = rows.find((row) => row.model.trim())?.model.trim() || "";
    updateDraft({
      modelList,
      ...(firstModel && (!draft?.model.trim() || !rows.some((row) => row.model.trim() === draft.model.trim()))
        ? { model: firstModel, testModel: firstModel }
        : {}),
    });
  };
  const addSupplierDirectModel = () => {
    writeSupplierDirectModels([...supplierDirectModels, {
      model: "",
      rowId: `direct-model-${supplierDirectModelRowIdRef.current += 1}`,
      supports1m: false,
    }]);
  };
  const updateSupplierDirectModel = (rowId: string, patch: Partial<SupplierDirectModelRow>) => {
    const nextRows = supplierDirectModels.map((row) => row.rowId === rowId ? { ...row, ...patch } : row);
    writeSupplierDirectModels(nextRows);
  };
  const removeSupplierDirectModel = (rowId: string) => {
    writeSupplierDirectModels(supplierDirectModels.filter((row) => row.rowId !== rowId));
  };
  const writeSupplierCodexCatalogModels = (rows: SupplierCodexCatalogDraftRow[]) => {
    setSupplierCodexCatalogModels(rows);
    const codexCatalogJson = supplierCodexCatalogJson(rows);
    const modelList = supplierCodexCatalogModelList(rows);
    const firstRow = rows.find((row) => row.model.trim());
    updateDraft({
      codexCatalogJson,
      modelList,
      ...(firstRow ? {
        model: firstRow.model.trim(),
        testModel: firstRow.model.trim(),
        contextWindow: firstRow.contextWindow,
      } : {}),
    });
  };
  const addSupplierCodexCatalogModel = () => {
    writeSupplierCodexCatalogModels([...supplierCodexCatalogModels, {
      displayName: "",
      model: "",
      contextWindow: "1000000",
      rowId: `codex-catalog-${supplierCodexCatalogRowIdRef.current += 1}`,
    }]);
  };
  const updateSupplierCodexCatalogModel = (rowId: string, patch: Partial<SupplierCodexCatalogRow>) => {
    writeSupplierCodexCatalogModels(supplierCodexCatalogModels.map((row) => row.rowId === rowId ? { ...row, ...patch } : row));
  };
  const removeSupplierCodexCatalogModel = (rowId: string) => {
    writeSupplierCodexCatalogModels(supplierCodexCatalogModels.filter((row) => row.rowId !== rowId));
  };

  const saveDraft = async (options: { stayInEditor?: boolean; applySupplier?: boolean } = {}): Promise<SupplierSaveResult | null> => {
    if (!appSettings || !draft || supplierSaveBusy) return null;
    const aggregateDraft = !!draft.aggregateEnabled;
    const requestedId = draft.id.trim();
    const normalizedId = supplierIdFromName(requestedId || draft.name);
    const idWasNormalized = requestedId !== normalizedId;
    const targetApp = supplierTargetForProfile(draft);
    const saveName = draft.name.trim() || normalizedId;
    const routedDraft = withSupplierRoutingState({ ...draft, id: normalizedId, name: saveName }, targetApp, !!draft.routeEnabled);
    const normalized = supplierProfileIsCcswitch(routedDraft)
      ? withSupplierPreservedImportedFiles(routedDraft)
      : normalizeSupplierProfile(withSupplierGeneratedFiles(routedDraft));
    const isCodexOfficialLogin = supplierProfileIsCodexOfficialLogin(normalized);
    if (!normalized.name.trim() || (!aggregateDraft && !isCodexOfficialLogin && !normalized.baseUrl.trim())) {
      window.alert(aggregateDraft ? "请填写聚合供应商名称后再保存。" : "请填写供应商名称和 Base URL 后再保存。API Key 可以后续补入。");
      return null;
    }
    if (aggregateDraft && !(normalized.aggregateMembers ?? []).length) {
      actions.showNotice({ title: "添加聚合供应商", message: "请先添加或选择至少 1 个普通 API 供应商的 Base URL / Key，再勾选为成员。", status: "failed" });
      return null;
    }
    if (targetApp === "claude-desktop" && !normalized.modelMappingEnabled) {
      const invalidModel = supplierDirectModelRows(normalized.modelList)
        .find((row) => !supplierDirectModelIsClaudeDesktopSafe(row.model));
      if (invalidModel) {
        actions.showNotice({
          title: "供应商保存",
          message: `Claude Desktop 直连模型 ID 无效：${invalidModel.model}。请使用 claude-/anthropic/claude- 的 Sonnet、Opus、Haiku 或 Fable 模型，或开启模型映射。`,
          status: "failed",
        });
        return null;
      }
    }
    const originalId = editingId;
    const conflicts = profiles.some((profile) => profile.id === normalized.id && profile.id !== originalId);
    if (conflicts) {
      window.alert(`供应商 ID「${normalized.id}」已存在，请换一个 ID。`);
      return null;
    }
    const nextProfiles = originalId && profiles.some((profile) => profile.id === originalId)
      ? profiles.map((profile) => (profile.id === originalId ? normalized : profile))
      : profiles.some((profile) => profile.id === normalized.id)
        ? profiles.map((profile) => (profile.id === normalized.id ? normalized : profile))
        : [...profiles, normalized];
    const currentActiveId = activeSupplierIdForTarget(targetApp);
    const nextActiveRelayId = !aggregateDraft && originalId && currentActiveId === originalId
      ? normalized.id
      : currentActiveId;
    const nextSettings = withActiveSupplierId({
      ...appSettings,
      relayProfilesEnabled: true,
      relayProfiles: nextProfiles,
    }, targetApp, nextActiveRelayId);
    const shouldApplySupplier = !aggregateDraft && (
      options.applySupplier === true
      || (targetApp === "claude-desktop" && !!originalId && currentActiveId === originalId)
    );
    if (shouldApplySupplier && !supplierProfileCanActivate(normalized)) {
      actions.showNotice({
        title: "供应商应用",
        message: "该供应商缺少 API Key，未修改当前生效配置。可取消“保存并使用”，先作为非活动配置保存。",
        status: "failed",
      });
      return null;
    }
    setSupplierSaveBusy(true);
    try {
      actions.showNotice({
        title: shouldApplySupplier ? "供应商保存并应用" : "供应商保存",
        message: shouldApplySupplier
          ? `正在保存并应用供应商「${normalized.name || normalized.id}」...`
          : `正在保存供应商「${normalized.name || normalized.id}」...`,
        status: "running",
      });
      const applied = shouldApplySupplier
        ? await actions.switchSupplierProfile(targetApp, normalized.id, nextSettings)
        : null;
      const saved = shouldApplySupplier
        ? applied && !statusFailed(applied.status) ? applied.settings : null
        : await saveSupplierSettings(nextSettings);
      if (saved) {
        const savedProfile = saved.relayProfiles.find((profile) => profile.id === normalized.id) ?? normalized;
        if (options.stayInEditor) {
          setEditingId(savedProfile.id);
          setDraft(normalizeDraftProfile(savedProfile));
          setSupplierCodexCatalogModels(createSupplierCodexCatalogModelRows(supplierCodexCatalogRows(savedProfile)));
        } else {
          setSupplierTargetFilter(supplierTargetForProfile(savedProfile));
          closeSupplierEditor();
        }
        actions.showNotice({
          title: shouldApplySupplier ? "供应商保存并应用" : "供应商保存",
          message: shouldApplySupplier
            ? `已保存并应用供应商「${savedProfile.name || savedProfile.id}」。`
            : `已保存供应商「${savedProfile.name || savedProfile.id}」。`,
          status: "ok",
        });
        if (idWasNormalized) {
          actions.showNotice({ title: "供应商保存", message: `供应商 ID 已自动整理为「${savedProfile.id}」。`, status: "ok" });
        }
        return { settings: saved, profile: savedProfile };
      }
      return null;
    } finally {
      setSupplierSaveBusy(false);
    }
  };
  const saveAndSwitchDraft = async () => {
    if (!draft) return;
    if (draft.aggregateEnabled) {
      actions.showNotice({ title: "供应商切换", message: "聚合供应商已经保存为真实配置记录；当前版本还没有聚合轮转代理，不能直接写入 Codex。", status: "failed" });
      return;
    }
    await saveDraft({ applySupplier: true });
  };
  const removeProfile = async (profile: RelayProfile) => {
    if (!appSettings) {
      window.alert("设置尚未加载，无法删除供应商。");
      return;
    }
    if (!window.confirm(`确认删除供应商「${profile.name || profile.id}」？`)) return;
    const deletingActiveCodex = supplierTargetForProfile(profile) === "codex"
      && appSettings.activeRelayId === profile.id;
    if (deletingActiveCodex) {
      // 删除活动 Codex profile 前先清理 live config/auth，避免列表已删但
      // 独立启动 Codex 仍继续使用已删除的中转供应商。
      await actions.clearRelayMode();
    }
    const nextProfiles = profiles
      .filter((item) => item.id !== profile.id)
      .map((item) => item.aggregateEnabled ? { ...item, aggregateMembers: (item.aggregateMembers ?? []).filter((id) => id !== profile.id) } : item);
    const nextForTarget = (targetApp: SupplierTargetApp, currentId: string) => {
      if (currentId !== profile.id) return currentId;
      return nextProfiles.find((item) => supplierTargetForProfile(item) === targetApp)?.id ?? "";
    };
    const saved = await saveSupplierSettings({
      ...appSettings,
      relayProfiles: nextProfiles,
      activeRelayId: nextForTarget("codex", appSettings.activeRelayId),
      activeClaudeRelayId: nextForTarget("claude", appSettings.activeClaudeRelayId || ""),
      activeClaudeDesktopRelayId: nextForTarget("claude-desktop", appSettings.activeClaudeDesktopRelayId || ""),
    });
    if (saved && editingId === profile.id) {
      setEditingId(null);
      setDraft(null);
    }
  };
  const applyPreset = (preset: SupplierPreset) => {
    if (!draft) return;
    setModelFetch(null);
    const targetApp = preset.targetApp ?? "codex";
    const modelList = preset.modelList?.join("\n") ?? preset.model;
    const codexCatalogJson = targetApp === "codex" ? "" : draft.codexCatalogJson ?? "";
    setSupplierDirectModels(createSupplierDirectModelRows(supplierDirectModelRows(modelList)));
    setSupplierCodexCatalogModels(targetApp === "codex"
      ? createSupplierCodexCatalogModelRows(supplierCodexCatalogRows({
        ...draft,
        targetApp,
        model: preset.model,
        testModel: preset.model,
        modelList,
        codexCatalogJson,
      }))
      : []);
    updateDraft({
      id: isNewDraft ? uniqueSupplierProfileId(profiles, preset.id) : draft.id,
      name: preset.name,
      baseUrl: preset.baseUrl,
      upstreamBaseUrl: preset.baseUrl,
      protocol: preset.protocol,
      targetApp,
      apiFormat: preset.apiFormat ?? "",
      routeEnabled: supplierRoutingEnabledForTarget(targetApp),
      claudeDesktopMode: targetApp === "codex" ? "" : supplierRoutingEnabledForTarget(targetApp) ? "proxy" : "direct",
      routeMode: targetApp === "codex"
        ? (supplierRoutingEnabledForTarget(targetApp) ? "Codex Proxy" : "Codex Direct")
        : (supplierRoutingEnabledForTarget(targetApp) ? "Claude Desktop Proxy" : "Claude Desktop Direct"),
      modelMappingEnabled: preset.modelMappingEnabled ?? false,
      modelMappingJson: preset.modelMappingJson ?? "",
      modelMapping: preset.modelMappingJson ? supplierModelMappingText(supplierModelMappingRows({ ...draft, modelMappingJson: preset.modelMappingJson })) : "",
      relayMode: "pureApi",
      aggregateEnabled: false,
      aggregateMembers: [],
      aggregateStrategy: "",
      model: preset.model,
      testModel: preset.model,
      modelList,
      codexCatalogJson,
    });
  };
  const fetchModels = async () => {
    if (!draft) return;
    const requestId = ++supplierModelFetchRequestRef.current;
    const normalized = normalizeSupplierProfile(withSupplierGeneratedFiles(draft));
    const result = await actions.fetchRelayProfileModels(normalized);
    if (requestId !== supplierModelFetchRequestRef.current) return;
    if (result) {
      setModelFetch(result);
      if (result.models.length) {
        if (draft.targetApp === "codex") return;
        const isClaudeTarget = draft.targetApp === "claude" || draft.targetApp === "claude-desktop";
        if (isClaudeTarget && draft.modelMappingEnabled) return;
        const existingRows = supplierDirectModels.length
          ? supplierDirectModels
          : createSupplierDirectModelRows(supplierDirectModelRows(draft.modelList));
        const existingModels = new Set(existingRows.map((row) => row.model.trim().toLowerCase()).filter(Boolean));
        const mergedRows = [
          ...existingRows,
          ...result.models
            .map((model) => model.trim())
            .filter((model) => model && !existingModels.has(model.toLowerCase()))
            .map((model) => ({
              model,
              rowId: `direct-model-${supplierDirectModelRowIdRef.current += 1}`,
              supports1m: false,
            })),
        ];
        setSupplierDirectModels(mergedRows);
        updateDraft({
          modelList: supplierDirectModelList(mergedRows),
          model: normalized.model || result.models[0],
          testModel: normalized.testModel || result.models[0],
        });
      }
    }
  };
  const toggleVisibleSupplierRouting = async (enabled: boolean) => {
    if (!appSettings || !routableSupplierProfiles.length || supplierRouteToggleInFlightRef.current) return;
    supplierRouteToggleInFlightRef.current = true;
    setSupplierRouteToggleBusy(true);
    try {
      const visibleIds = new Set(routableSupplierProfiles.map((profile) => profile.id));
      const nextProfiles = appSettings.relayProfiles.map((profile) => {
        if (!visibleIds.has(profile.id)) return profile;
        return withSupplierRoutingState(profile, supplierRouteGroup, enabled);
      });
      const nextSettings = { ...appSettings, relayProfiles: nextProfiles };
      actions.showNotice({ title: "供应商路由", message: enabled ? `正在开启 ${supplierRouteGroupLabel} 供应商路由...` : `正在关闭 ${supplierRouteGroupLabel} 供应商路由...`, status: "running" });
      const activeProfileId = activeSupplierIdForTarget(supplierRouteGroup);
      const isDisablingActiveRoute = !enabled && visibleIds.has(activeProfileId);
      if (isDisablingActiveRoute && supplierRouteGroup === "codex") {
        const switched = await actions.switchSupplierProfile("codex", activeProfileId, nextSettings);
        if (!switched || !statusOk(switched.status)) return;
        actions.showNotice({ title: "供应商路由", message: "已关闭 Codex 供应商路由，运行中的代理配置已撤销。", status: "ok" });
        return;
      }
      if (isDisablingActiveRoute && supplierRouteGroup === "claude-desktop") {
        const restored = await (actions.restoreClaudeDesktopProviderOfficial as unknown as (skipConfirm?: boolean) => Promise<{ status?: Status } | null>)(true);
        if (!restored || !statusOk(restored.status)) return;
        const saved = await saveSupplierSettings({ ...appSettings, relayProfiles: nextProfiles });
        if (!saved) return;
        actions.showNotice({ title: "供应商路由", message: "已关闭 Claude Desktop 供应商路由，运行中的代理配置已撤销。", status: "ok" });
        return;
      }
      const saved = await saveSupplierSettings(nextSettings);
      if (saved) {
        actions.showNotice({ title: "供应商路由", message: enabled ? `已开启 ${supplierRouteGroupLabel} 供应商路由。` : `已关闭 ${supplierRouteGroupLabel} 供应商路由。`, status: "ok" });
      }
    } finally {
      supplierRouteToggleInFlightRef.current = false;
      setSupplierRouteToggleBusy(false);
    }
  };
  const supplierOrderFromIds = (ids: string[]) => {
    const byId = new Map(profiles.map((profile) => [profile.id, profile]));
    const ordered = ids
      .map((id) => byId.get(id))
      .filter((profile): profile is RelayProfile => !!profile);
    const used = new Set(ordered.map((profile) => profile.id));
    return [...ordered, ...profiles.filter((profile) => !used.has(profile.id))];
  };
  // 渲染用的排序结果：drag 期间 dragOverId 频繁变化，避免每次重建 Map + 重排。
  // supplierOrderFromIds 是纯函数，仅依赖 profiles 与传入的 ids。
  // 必须置于任何条件 return 之前以遵守 Hooks 规则。
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const orderedProfiles = useMemo(() => supplierOrderFromIds(supplierOrderIds), [profiles, supplierOrderIds]);
  const filteredOrderedProfiles = useMemo(() => orderedProfiles.filter((profile) => supplierTargetForProfile(profile) === supplierTargetFilter), [orderedProfiles, supplierTargetFilter]);
  const visibleSupplierOrderIds = useMemo(() => filteredOrderedProfiles.map((profile) => profile.id), [filteredOrderedProfiles]);
  const supplierRouteGroup = supplierTargetFilter;
  const supplierRouteGroupLabel = supplierRouteGroup === "claude-desktop"
    ? "Claude Desktop"
    : supplierRouteGroup === "claude"
      ? "Claude"
      : "Codex";
  const routableSupplierProfiles = useMemo(
    () => profiles.filter((profile) => supplierTargetForProfile(profile) === supplierRouteGroup),
    [profiles, supplierRouteGroup],
  );
  const supplierRouteSwitchEnabled = routableSupplierProfiles.some((profile) => !!profile.routeEnabled);
  const supplierRouteSwitchDisabled = supplierRouteToggleBusy || !appSettings || !routableSupplierProfiles.length;
  const setSupplierCardRef = (profileId: string) => (node: HTMLDivElement | null) => {
    if (node) {
      supplierCardRefs.current.set(profileId, node);
    } else {
      supplierCardRefs.current.delete(profileId);
    }
  };
  // 目标应用过滤后渲染卡片；保持全量 supplierOrderIds 用于跨过滤视图稳定排序。
  const reorderSupplierIds = (sourceId: string, targetId: string, ids = supplierOrderIds) => {
    const currentIds = supplierOrderFromIds(ids.length ? ids : profiles.map((profile) => profile.id)).map((profile) => profile.id);
    const fromIndex = currentIds.indexOf(sourceId);
    const toIndex = currentIds.indexOf(targetId);
    if (fromIndex < 0 || toIndex < 0) return;
    const nextIds = [...currentIds];
    const [moved] = nextIds.splice(fromIndex, 1);
    nextIds.splice(toIndex, 0, moved);
    return nextIds;
  };
  const saveSupplierOrder = async (orderedIds: string[]) => {
    if (!appSettings) return;
    const reordered = supplierOrderFromIds(orderedIds);
    const previousIds = profiles.map((profile) => profile.id);
    const nextIds = reordered.map((profile) => profile.id);
    if (nextIds.join("\u001f") === previousIds.join("\u001f")) return;
    actions.showNotice({ title: "供应商排序", message: "正在保存供应商顺序...", status: "running" });
    const saved = await saveSupplierSettings({ ...appSettings, relayProfiles: reordered });
    if (saved) {
      setSupplierOrderIds(saved.relayProfiles.map((profile) => profile.id));
      actions.showNotice({ title: "供应商排序", message: "供应商顺序已保存。", status: "ok" });
    } else {
      setSupplierOrderIds(previousIds);
      actions.showNotice({ title: "供应商排序", message: "供应商顺序保存失败，已恢复原顺序。", status: "failed" });
    }
  };
  const supplierTargetIdFromPointer = (clientY: number) => {
    if (!visibleSupplierOrderIds.length) return null;
    for (const profileId of visibleSupplierOrderIds) {
      const node = supplierCardRefs.current.get(profileId);
      if (!node) continue;
      const rect = node.getBoundingClientRect();
      if (clientY < rect.top + rect.height / 2) return profileId;
    }
    return visibleSupplierOrderIds[visibleSupplierOrderIds.length - 1] ?? null;
  };
  const beginSupplierPointerDrag = (event: ReactPointerEvent<HTMLElement>, profileId: string) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    try {
      event.currentTarget.setPointerCapture(event.pointerId);
    } catch {
      // Pointer capture may be unavailable in some WebView states; window listeners still finish sorting.
    }
    const baselineIds = supplierOrderFromIds(supplierOrderIds.length ? supplierOrderIds : profiles.map((profile) => profile.id))
      .map((profile) => profile.id);
    const dragHandle = event.currentTarget;
    const sourceNode = supplierCardRefs.current.get(profileId);
    const sourceRect = sourceNode?.getBoundingClientRect();
    supplierPointerDragRef.current = {
      sourceId: profileId,
      latestIds: baselineIds,
      lastTargetId: profileId,
    };
    if (sourceRect) {
      setSupplierDragOverlay({
        profileId,
        top: sourceRect.top,
        left: sourceRect.left,
        width: sourceRect.width,
        height: sourceRect.height,
        offsetY: event.clientY - sourceRect.top,
      });
    }
    setDraggedId(profileId);
    setDragOverId(profileId);

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const current = supplierPointerDragRef.current;
      if (!current) return;
      moveEvent.preventDefault();
      setSupplierDragOverlay((overlay) => overlay && overlay.profileId === current.sourceId
        ? { ...overlay, top: moveEvent.clientY - overlay.offsetY }
        : overlay);
      const targetId = supplierTargetIdFromPointer(moveEvent.clientY);
      if (!targetId || targetId === current.lastTargetId) return;
      const nextIds = reorderSupplierIds(current.sourceId, targetId, current.latestIds) ?? current.latestIds;
      current.latestIds = nextIds;
      current.lastTargetId = targetId;
      setDragOverId(targetId);
      setSupplierOrderIds(nextIds);
    };

    const finishPointerDrag = () => {
      const current = supplierPointerDragRef.current;
      supplierPointerDragRef.current = null;
      window.removeEventListener("pointermove", handlePointerMove, true);
      window.removeEventListener("pointerup", finishPointerDrag, true);
      window.removeEventListener("pointercancel", finishPointerDrag, true);
      try {
        dragHandle.releasePointerCapture(event.pointerId);
      } catch {
        // Pointer capture may already be released by the WebView.
      }
      setDraggedId(null);
      setDragOverId(null);
      setSupplierDragOverlay(null);
      if (current) {
        setSupplierOrderIds(current.latestIds);
        void saveSupplierOrder(current.latestIds);
      }
    };

    window.addEventListener("pointermove", handlePointerMove, true);
    window.addEventListener("pointerup", finishPointerDrag, { capture: true, once: true });
    window.addEventListener("pointercancel", finishPointerDrag, { capture: true, once: true });
  };
  const pinSupplierToTop = (profileId: string) => {
    const currentIds = supplierOrderFromIds(supplierOrderIds.length ? supplierOrderIds : profiles.map((profile) => profile.id))
      .map((profile) => profile.id);
    if (!currentIds.length || currentIds[0] === profileId) return;
    const nextIds = [profileId, ...currentIds.filter((id) => id !== profileId)];
    setSupplierOrderIds(nextIds);
    void saveSupplierOrder(nextIds);
  };
  const importFromCcswitch = async () => {
    if (!appSettings) return;
    setImportOpen(false);
    const result = await actions.importCcswitchCodexProviders();
    if (!result || !statusOk(result.status)) return;
    const imported = result.profiles.map((profile) => normalizeSupplierProfile(profile));
    const importedById = new Map(imported.map((profile) => [profile.id, profile]));
    let updatedCount = 0;
    const nextProfiles = appSettings.relayProfiles.map((profile) => {
      const importedProfile = importedById.get(profile.id);
      if (importedProfile && supplierProfileIsCcswitch(profile)) {
        importedById.delete(profile.id);
        updatedCount += 1;
        return importedProfile;
      }
      return profile;
    });
    const existingIds = new Set(nextProfiles.map((profile) => profile.id));
    let addedCount = 0;
    for (const profile of importedById.values()) {
      const nextProfile = existingIds.has(profile.id)
        ? normalizeSupplierProfile({ ...profile, id: uniqueSupplierProfileId(nextProfiles, profile.id) })
        : profile;
      existingIds.add(nextProfile.id);
      nextProfiles.push(nextProfile);
      addedCount += 1;
    }
    const saved = await saveSupplierSettings({ ...appSettings, relayProfiles: nextProfiles });
    if (!saved) return;
    actions.showNotice({ title: "CC-switch 导入", message: `已从 cc-switch 更新 ${updatedCount} 个、新增 ${addedCount} 个供应商配置。`, status: "ok" });
  };

  const refreshSupplierList = async () => {
    if (supplierRefreshBusy) return;
    setSupplierRefreshBusy(true);
    actions.showNotice({ title: "刷新供应商列表", message: "正在刷新供应商配置和路由状态...", status: "running" });
    try {
      await actions.refreshRoute("supplier", { notify: true });
      actions.showNotice({ title: "刷新供应商列表", message: "供应商列表已刷新。", status: "ok" });
      setImportOpen(false);
    } catch (error) {
      actions.showNotice({
        title: "刷新供应商列表失败",
        message: error instanceof Error ? error.message : String(error),
        status: "failed",
      });
    } finally {
      setSupplierRefreshBusy(false);
    }
  };

  const supplierDisplayUrl = (profile: RelayProfile) => {
    const configBaseUrl = profile.configContents.match(/\bbase_url\s*=\s*["']([^"']+)["']/i)?.[1]?.trim() ?? "";
    const rawUrl = profile.upstreamBaseUrl || profile.baseUrl || configBaseUrl;
    if (!rawUrl.trim()) return "未配置接口地址";
    return rawUrl.trim().replace(/\/v1\/?$/i, "");
  };

  const renderSupplierCard = (profile: RelayProfile, options: { overlay?: boolean; style?: CSSProperties } = {}) => {
    const targetApp = supplierTargetForProfile(profile);
    const selected = profile.id === activeSupplierIdForTarget(targetApp);
    const aggregate = !!profile.aggregateEnabled;
    const imported = supplierProfileIsCcswitch(profile);
    const appLabel = imported ? supplierTargetAppLabel(profile.targetApp) : supplierRelayModeLabel(profile.relayMode);
    const protocolLabel = imported ? supplierApiFormatLabel(profile) : supplierProtocolLabel(profile.protocol);
    const summary = aggregate
      ? `${aggregateStrategyLabel(profile.aggregateStrategy)} / ${profile.aggregateMembers?.length ?? 0} \u4e2a\u6210\u5458`
      : `${appLabel} / ${protocolLabel}`;
    const displayUrl = supplierDisplayUrl(profile);
    const dragSource = draggedId === profile.id && !options.overlay;
    return (
      <div
        className={`supplier-card ${selected ? "selected" : ""} ${draggedId === profile.id ? "dragging" : ""} ${dragOverId === profile.id ? "drag-over" : ""} ${dragSource ? "drag-source" : ""} ${options.overlay ? "drag-overlay-card" : ""}`}
        key={options.overlay ? `${profile.id}-overlay` : profile.id}
        ref={options.overlay ? undefined : setSupplierCardRef(profile.id)}
        style={options.style}
      >
        <button aria-label={"\u62d6\u62fd\u6392\u5e8f"} className="supplier-drag-handle" disabled={options.overlay} onPointerDown={options.overlay ? undefined : (event) => beginSupplierPointerDrag(event, profile.id)} title={"\u6309\u4f4f\u62d6\u62fd\u6392\u5e8f"} type="button">
          <GripVertical className="h-4 w-4" focusable="false" />
        </button>
        <div className="supplier-avatar">{aggregate ? "\u805a" : (profile.name || profile.id || "P").slice(0, 1).toUpperCase()}</div>
        <div className="supplier-card-main">
          <div className="supplier-title-line">
            <strong>{profile.name || profile.id}</strong>
            {aggregate ? <span className="supplier-badge">\u805a\u5408</span> : null}
            {imported ? <span className="supplier-badge">cc-switch</span> : null}
          </div>
          {aggregate ? <span className="supplier-card-subtitle">{summary}</span> : null}
          <button className="supplier-url-link" disabled type="button">{displayUrl}</button>
        </div>
        <div className="supplier-card-actions">
          <button className={`supplier-card-action-button supplier-card-use-button ${selected ? "current" : ""}`} disabled={selected || aggregate || appSettings?.relayProfilesEnabled === false || options.overlay} onClick={() => void actions.switchSupplierProfile(targetApp, profile.id)} type="button">
            <Play className="h-4 w-4" />
            {selected ? "\u4f7f\u7528\u4e2d" : "\u4f7f\u7528"}
          </button>
          <button className="supplier-card-action-button" disabled={options.overlay} onClick={() => openProfileEditor(profile)} title="\u7f16\u8f91" type="button"><Edit className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled={options.overlay} onClick={() => duplicateProfile(profile)} title="\u590d\u5236" type="button"><Copy className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled title="\u68c0\u6d4b\u8fde\u901a" type="button"><Activity className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled title="\u7528\u91cf\u914d\u7f6e" type="button"><BarChart3 className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled={options.overlay} onClick={() => void removeProfile(profile)} title="\u5220\u9664\u4f9b\u5e94\u5546" type="button"><Trash2 className="h-4 w-4" /></button>
        </div>
      </div>
    );
  };
  const supplierDragOverlayProfile = supplierDragOverlay ? profiles.find((profile) => profile.id === supplierDragOverlay.profileId) : null;


  if (draft?.aggregateEnabled) {
    const generated = normalizeSupplierProfile(withSupplierGeneratedFiles(draft));
    const members = generated.aggregateMembers ?? [];
    return (
      <div className="supplier-workbench">
        <Panel title={generated.name || "聚合供应商1"} detail="聚合供应商会保存策略和成员关系；当前版本不直接写入 Codex，后续聚合代理会读取这些字段。">
          <div className="supplier-editor-toolbar sticky">
            <Button onClick={closeSupplierEditor} variant="outline">返回列表</Button>
            <Button disabled={supplierSaveBusy} onClick={() => void saveDraft()} type="button">
              <Save className="h-4 w-4" />
              {supplierSaveBusy ? "保存中" : "保存"}
            </Button>
          </div>
          <div className="supplier-editor-card">
            <div className="supplier-editor-titleline"><strong>{generated.name}</strong><span className="supplier-badge">聚合</span></div>
            <div className="supplier-form-grid">
              <label className="ops-form-field"><span>名称</span><input onChange={(event) => updateSupplierName(event.currentTarget.value)} value={generated.name} /></label>
              <label className="ops-form-field"><span>测试模型</span><input onChange={(event) => updateDraft({ testModel: event.currentTarget.value, model: event.currentTarget.value })} value={generated.testModel || generated.model} /></label>
              <label className="ops-form-field span-2"><span>聚合策略</span><select className="ops-select" onChange={(event) => updateDraft({ aggregateStrategy: event.currentTarget.value })} value={generated.aggregateStrategy || "failover"}>{AGGREGATE_STRATEGIES.map((strategy) => <option key={strategy.id} value={strategy.id}>{strategy.label}</option>)}</select></label>
            </div>
            <div className="supplier-aggregate-grid">
              {AGGREGATE_STRATEGIES.map((strategy) => <button className={strategy.id === (generated.aggregateStrategy || "failover") ? "selected" : ""} key={strategy.id} onClick={() => updateDraft({ aggregateStrategy: strategy.id })} type="button"><strong>{strategy.label}</strong><span>{strategy.detail}</span></button>)}
            </div>
            <div className="supplier-member-box">
              <div className="supplier-member-head"><strong>成员供应商</strong><span>{members.length}/{apiProfiles.length}</span></div>
              {apiProfiles.length ? apiProfiles.map((profile) => {
                const checked = members.includes(profile.id);
                return <label className="supplier-member-row" key={profile.id}><input checked={checked} onChange={(event) => updateDraft({ aggregateMembers: event.currentTarget.checked ? [...members, profile.id] : members.filter((id) => id !== profile.id) })} type="checkbox" /><span>{profile.name || profile.id}</span><small>{profile.baseUrl || "未配置 Base URL"}</small></label>;
              }) : <p>请先添加或选择至少 1 个普通 API 供应商的 Base URL / Key，再勾选为成员。</p>}
            </div>
            <div className="info-grid compact supplier-aggregate-summary">
              <InfoRow label="策略" value={aggregateStrategyLabel(generated.aggregateStrategy)} />
              <InfoRow label="成员数量" value={`${members.length} 个`} />
              <InfoRow label="总权重" value={`${members.length || 0}`} />
              <InfoRow label="序列化字段" value="aggregate.strategy / aggregate.members" />
            </div>
          </div>
        </Panel>
      </div>
    );
  }

  if (draft) {
    const generated = normalizeDraftProfile(draft);
    const canSwitch = !generated.aggregateEnabled;
    const isClaudeSupplier = generated.targetApp === "claude" || generated.targetApp === "claude-desktop";
    const isCodexSupplier = generated.targetApp === "codex" || !generated.targetApp;
    const apiFormatOption = supplierApiFormatOption(generated.apiFormat || "Anthropic Messages");
    const selectedApiFormat = isCodexSupplier
      ? (generated.apiFormat === "openai_chat" || generated.protocol === "chatCompletions" ? "openai_chat" : "openai_responses")
      : generated.apiFormat;
    const routeRequired = isCodexSupplier
      ? selectedApiFormat === "openai_chat"
      : supplierApiFormatRequiresRoute(selectedApiFormat);
    const routeEnabled = !!generated.routeEnabled;
    const authField = generated.authField || "ANTHROPIC_AUTH_TOKEN";
    const defaultModel = generated.model || generated.testModel || (isCodexSupplier ? "gpt-5.1" : "claude-sonnet");
    const modelRowsForDraft = supplierModelMappingRows(generated);
    const supplierModelOptions = Array.from(new Set((modelFetch !== null
      ? modelFetch.models
      : isCodexSupplier
        ? supplierCodexCatalogModels.map((row) => row.model)
        : supplierDirectModelRows(generated.modelList).map((row) => row.model))
      .map((model) => String(model || "").trim())
      .filter(Boolean)));
    const routePrompt = routeRequired && !routeEnabled
      ? `当前 API 格式需要路由。请返回供应商列表开启${isCodexSupplier ? " Codex" : " Claude / Claude Desktop"} 路由。`
      : "";
    const applyOneClickModelMapping = () => {
      if (!supplierModelOptions.length) {
        actions.showNotice({ title: "一键设置失败", message: "请先获取模型，或在保存的模型列表中配置可用模型。", status: "failed" });
        return;
      }
      const lowerOptions = supplierModelOptions.map((option) => ({ option, lower: option.toLowerCase() }));
      const rows = modelRowsForDraft.map((row) => {
        const current = row.requestModel.trim();
        const selected = supplierModelOptions.includes(current)
          ? current
          : lowerOptions.find(({ lower }) => lower.includes(row.role))?.option
            || (supplierModelOptions.includes(defaultModel) ? defaultModel : supplierModelOptions[0]);
        return { ...row, displayName: row.displayName || selected, requestModel: selected };
      });
      updateDraft({ modelMappingEnabled: true, modelMappingJson: supplierModelMappingJson(rows), modelMapping: supplierModelMappingText(rows) });
      actions.showNotice({ title: "一键设置完成", message: `已为 ${rows.length} 个 Claude 角色设置有效的实际请求模型。`, status: "ok" });
    };
    const cleanName = generated.name.replace(/\s*\(ccswitch\)$/i, "");
    const editorTitle = isNewDraft ? "添加供应商" : "编辑供应商";
    const editorAppLabel = supplierTargetAppLabel(generated.targetApp || "codex");
    const formAvatar = (cleanName || generated.id || "P").slice(0, 1).toUpperCase();
    const baseEndpointLabel = isCodexSupplier ? "API 请求地址" : "请求地址";
    const baseEndpointHint = isCodexSupplier
      ? "填写兼容 OpenAI Responses 或 Chat Completions 格式的服务端点地址；Chat Completions 按 cc-switch 语义启用路由接管。"
      : "填写兼容 Claude API 的服务端点地址，不要以斜杠结尾。";
    const claudeConfigJson = JSON.stringify({
      env: {
        [authField]: generated.apiKey,
        ANTHROPIC_BASE_URL: generated.baseUrl || generated.upstreamBaseUrl,
        ...(generated.modelMappingEnabled ? {
          ANTHROPIC_DEFAULT_HAIKU_MODEL: modelRowsForDraft.find((row) => row.role === "haiku")?.requestModel || defaultModel,
          ANTHROPIC_DEFAULT_OPUS_MODEL: modelRowsForDraft.find((row) => row.role === "opus")?.requestModel || defaultModel,
          ANTHROPIC_DEFAULT_FABLE_MODEL: modelRowsForDraft.find((row) => row.role === "fable")?.requestModel || defaultModel,
          ANTHROPIC_DEFAULT_SONNET_MODEL: modelRowsForDraft.find((row) => row.role === "sonnet")?.requestModel || defaultModel,
          CLAUDE_CODE_SUBAGENT_MODEL: modelRowsForDraft.find((row) => row.role === "subagent")?.requestModel || "",
        } : {}),
        ANTHROPIC_MODEL: defaultModel,
      },
      ...(generated.headerOverride?.trim() || generated.bodyOverride?.trim()
        ? { localProxyOverrides: { headers: generated.headerOverride || "{}", body: generated.bodyOverride || "{}" } }
        : {}),
    }, null, 2);
    const supplierConfigJson = generated.configContents || claudeConfigJson;
    const visibleSupplierConfigJson = showSupplierApiKey
      ? supplierConfigJson
      : redactSupplierConfig(supplierConfigJson);
    const codexAuthJson = generated.authContents || JSON.stringify({ OPENAI_API_KEY: generated.apiKey }, null, 2);
    const codexConfigToml = generated.configContents || `model = "${generated.model || defaultModel}"
model_provider = "${generated.id || "custom"}"

[model_providers.${generated.id || "custom"}]
name = "${cleanName || "Custom Provider"}"
base_url = "${generated.baseUrl || generated.upstreamBaseUrl || "https://api.example.com/v1"}"
wire_api = "${generated.apiFormat === "openai_chat" || generated.protocol === "chatCompletions" ? "chat" : "responses"}"
env_key = "OPENAI_API_KEY"
`;
    const visibleCodexAuthJson = redactSupplierConfig(codexAuthJson);
    const visibleCodexConfigToml = redactSupplierConfig(codexConfigToml);
    const visibleHeaderOverride = showSupplierApiKey
      ? generated.headerOverride || ""
      : redactSupplierConfig(generated.headerOverride || "");
    const visibleBodyOverride = showSupplierApiKey
      ? generated.bodyOverride || ""
      : redactSupplierConfig(generated.bodyOverride || "");
    const renderSourceCollapse = (open: boolean, setOpen: Dispatch<SetStateAction<boolean>>, icon: ReactNode, title: string, children: ReactNode) => (
      <div className={`supplier-ccswitch-collapse-card ${open ? "expanded" : ""}`}>
        <div className="supplier-ccswitch-collapse-head" onClick={() => setOpen((value) => !value)} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); setOpen((value) => !value); } }} role="button" tabIndex={0}>
          <span className="supplier-collapse-title">{icon}{title}</span>
          <span className="supplier-collapse-right"><span>使用单独配置</span><ToggleSwitch checked={false} disabled onChange={() => undefined} /><span className="supplier-collapse-chevron">{open ? "v" : ">"}</span></span>
        </div>
        {open ? <div className="supplier-ccswitch-collapse-body">{children}</div> : null}
      </div>
    );
    return (
      <div className="supplier-ccswitch-editor source-parity">
        <div className="supplier-ccswitch-editor-head"><button className="supplier-back-button" onClick={closeSupplierEditor} type="button" aria-label="返回供应商列表" title="返回"><ArrowLeft className="h-5 w-5" /></button><strong>{editorTitle}</strong></div>
        <div className="supplier-ccswitch-editor-body"><section className="supplier-ccswitch-form-card">
          <div className="supplier-form-avatar-shell"><div className="supplier-form-avatar">{formAvatar}</div></div>
          <div className="supplier-preset-strip">{SUPPLIER_PRESETS.filter((preset) => preset.id === "openai" || preset.id === "anthropic").map((preset) => <button className={preset.id === generated.id ? "active" : ""} key={preset.id} onClick={() => applyPreset(preset)} type="button"><strong>{preset.name}</strong><span>{preset.targetApp === "claude-desktop" ? "Claude Desktop" : preset.targetApp === "claude" ? "Claude" : "Codex"}</span></button>)}</div>
          <label className="ops-form-field"><span>供应商名称</span><input onBlur={(event) => updateNewDraftIdFromName(event.currentTarget.value)} onChange={(event) => updateSupplierName(event.currentTarget.value)} value={draft.name} /></label>
          <label className="ops-form-field"><span>备注</span><input onChange={(event) => updateDraft({ notes: event.currentTarget.value })} placeholder="例如：公司专用账号" value={generated.notes || ""} /></label>
          <label className="ops-form-field"><span>官网链接</span><input onChange={(event) => updateDraft({ websiteUrl: event.currentTarget.value })} placeholder="https://example.com" value={generated.websiteUrl || ""} /></label>
          <label className="ops-form-field"><span>API Key</span><div className="supplier-secret-input"><input onChange={(event) => updateDraft({ apiKey: event.currentTarget.value, apiKeyExplicit: true })} type={showSupplierApiKey ? "text" : "password"} value={generated.apiKey} /><button aria-label={showSupplierApiKey ? "隐藏密钥" : "显示密钥"} onClick={() => setShowSupplierApiKey((value) => !value)} title={showSupplierApiKey ? "隐藏密钥" : "显示密钥"} type="button">{showSupplierApiKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}</button></div></label>
          <label className="ops-form-field"><span>{baseEndpointLabel} <span className="supplier-url-toggle">完整 URL</span></span><input onChange={(event) => updateDraft({ baseUrl: event.currentTarget.value, upstreamBaseUrl: event.currentTarget.value })} placeholder={isCodexSupplier ? "https://api.example.com/v1" : "https://api.example.com"} value={generated.baseUrl || generated.upstreamBaseUrl} /></label>
          <div className="supplier-route-note">提示：{baseEndpointHint}</div>
          {isClaudeSupplier ? <section className="supplier-mapping-card"><div><strong>需要模型映射</strong><p>关闭时按原始模型 ID 直传；供应商不接受 Claude 安全路由 ID 时请开启映射。</p></div><ToggleSwitch checked={!!generated.modelMappingEnabled} onChange={(value) => updateDraft({ modelMappingEnabled: value })} /></section> : null}
          <details className="supplier-ccswitch-section supplier-advanced-card" open><summary><span>&gt;</span>高级选项</summary>
            {isClaudeSupplier ? (
              generated.modelMappingEnabled ? (
                <>
                  <label className="ops-form-field">
                    <span>API 格式</span>
                    <select className="ops-select" onChange={(event) => updateDraft({ apiFormat: event.currentTarget.value })} value={generated.apiFormat || "Anthropic Messages"}>
                      {SUPPLIER_API_FORMAT_OPTIONS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
                    </select>
                    <small>{apiFormatOption?.detail || "选择供应商 API 的输入格式"}</small>
                  </label>
                  {routePrompt ? <div className="supplier-route-note">{routePrompt}</div> : null}
                  <div className="supplier-ccswitch-divider" />
                  <div className="supplier-model-map-head">
                    <strong>模型映射</strong>
                    <div className="supplier-toolbar">
                      <Button onClick={applyOneClickModelMapping} type="button" variant="outline"><Wrench className="h-4 w-4" />一键设置</Button>
                      <Button onClick={() => void fetchModels()} type="button" variant="outline"><Download className="h-4 w-4" />获取模型</Button>
                    </div>
                  </div>
                  <p className="supplier-inline-note">显示名称只影响模型菜单；实际请求模型会发送到上游；1M 是本地能力声明。</p>
                  <div className="supplier-model-map-grid header claude"><span>模型角色</span><span>显示名称</span><span>实际请求模型</span><span>声明支持 1M</span></div>
                  {modelRowsForDraft.map((row) => (
                    <div className="supplier-model-map-grid claude" key={row.role}>
                      <input disabled value={row.label} />
                      <input onChange={(event) => updateSupplierModelMapping(row.role, "displayName", event.currentTarget.value)} placeholder={defaultModel} value={row.displayName || ""} />
                      <div className="supplier-model-input-dropdown">
                        <input
                          aria-label={`${row.label} 实际请求模型`}
                          onChange={(event) => updateSupplierModelMapping(row.role, "requestModel", event.currentTarget.value)}
                          placeholder="例如: claude-sonnet-4-6"
                          value={row.requestModel || ""}
                        />
                        <SupplierModelDropdown
                          compact
                          iconOnly
                          onChange={(value) => updateSupplierModelMapping(row.role, "requestModel", value)}
                          options={supplierModelOptions}
                          placeholder="选择已获取模型"
                          showAvailabilityWarning={false}
                          triggerLabel="选择已获取模型"
                          value={row.requestModel || ""}
                        />
                      </div>
                      <label><input checked={row.supports1m} onChange={(event) => updateSupplierModelMapping(row.role, "supports1m", event.currentTarget.checked)} type="checkbox" />1M</label>
                    </div>
                  ))}
                  <label className="ops-form-field"><span>默认兜底模型</span><input onChange={(event) => updateDraft({ model: event.currentTarget.value, testModel: event.currentTarget.value })} value={defaultModel} /></label>
                </>
              ) : (
                <details className="supplier-direct-model-list" onToggle={(event) => setSupplierDirectModelsOpen(event.currentTarget.open)} open={supplierDirectModelsOpen}>
                  <summary><span>{supplierDirectModelsOpen ? "⌄" : ">"}</span>手动指定 Claude Desktop 模型列表（高级，可选）</summary>
                  <div className="supplier-direct-model-list-body">
                    <div className="supplier-direct-model-list-head">
                      <p>仅当供应商的 /v1/models 不可用或没有返回 Claude Desktop 可识别的 Sonnet / Opus / Haiku 模型名时填写；勾选 1M 会向 Claude Desktop 声明支持 1M 上下文。</p>
                      <div className="supplier-toolbar">
                        <Button onClick={() => void fetchModels()} type="button" variant="outline"><Download className="h-4 w-4" />获取模型列表</Button>
                        <Button onClick={addSupplierDirectModel} type="button" variant="outline"><Plus className="h-4 w-4" />添加模型</Button>
                      </div>
                    </div>
                    {supplierDirectModels.length ? <div className="supplier-direct-model-rows">
                      {supplierDirectModels.map((row, index) => (
                        <div className="supplier-direct-model-row" key={row.rowId}>
                          <input aria-label={`Claude Desktop 模型 ${index + 1}`} onChange={(event) => updateSupplierDirectModel(row.rowId, { model: event.currentTarget.value })} placeholder="claude-sonnet-4-6" value={row.model} />
                          <label><input checked={row.supports1m} onChange={(event) => updateSupplierDirectModel(row.rowId, { supports1m: event.currentTarget.checked })} type="checkbox" />1M</label>
                          <button aria-label="删除模型" className="supplier-direct-model-remove" onClick={() => removeSupplierDirectModel(row.rowId)} title="删除模型" type="button"><Trash2 className="h-4 w-4" /></button>
                        </div>
                      ))}
                    </div> : <p className="supplier-direct-model-empty">尚未指定手动模型；Claude Desktop 会优先读取供应商模型目录。</p>}
                  </div>
                </details>
              )
            ) : (
              <>
                <label className="ops-form-field">
                  <span>上游格式</span>
                  <select className="ops-select" onChange={(event) => {
                    const next = event.currentTarget.value;
                    updateDraft({ apiFormat: next, protocol: next === "openai_chat" ? "chatCompletions" : "responses" });
                  }} value={generated.apiFormat === "openai_chat" || generated.protocol === "chatCompletions" ? "openai_chat" : "openai_responses"}>
                    <option value="openai_chat">Chat Completions（需开启路由）</option>
                    <option value="openai_responses">Responses（原生）</option>
                  </select>
                  <small>Responses 可直连；Chat Completions 需要路由接管。</small>
                </label>
                {routePrompt ? <div className="supplier-route-note">{routePrompt}</div> : null}
                <div className="supplier-ccswitch-divider" />
                <div className="supplier-model-map-head">
                  <strong>模型映射</strong>
                  <div className="supplier-toolbar">
                    <Button onClick={() => void fetchModels()} type="button" variant="outline"><Download className="h-4 w-4" />获取模型</Button>
                    <Button onClick={addSupplierCodexCatalogModel} type="button" variant="outline"><Plus className="h-4 w-4" />添加模型</Button>
                  </div>
                </div>
                <p className="supplier-inline-note">菜单显示名用于 Codex 模型选择；实际请求模型发送给上游；上下文窗口用于本地模型能力说明。</p>
                <div className="supplier-codex-catalog-grid header"><span>菜单显示名</span><span>实际请求模型</span><span>上下文窗口</span><span /></div>
                {supplierCodexCatalogModels.length ? supplierCodexCatalogModels.map((row, index) => (
                  <div className="supplier-codex-catalog-grid" key={row.rowId}>
                    <input
                      aria-label={`菜单显示名 ${index + 1}`}
                      onChange={(event) => updateSupplierCodexCatalogModel(row.rowId, { displayName: event.currentTarget.value })}
                      placeholder="例如: DeepSeek V4 Flash"
                      value={row.displayName}
                    />
                    <div className="supplier-model-input-dropdown">
                      <input
                        aria-label={`实际请求模型 ${index + 1}`}
                        onChange={(event) => updateSupplierCodexCatalogModel(row.rowId, { model: event.currentTarget.value })}
                        placeholder="例如: deepseek-v4-flash"
                        value={row.model}
                      />
                      <SupplierModelDropdown
                        compact
                        iconOnly
                        onChange={(value) => updateSupplierCodexCatalogModel(row.rowId, {
                          model: value,
                          ...(row.displayName.trim() ? {} : { displayName: value }),
                        })}
                        options={supplierModelOptions}
                        placeholder="选择已获取模型"
                        showAvailabilityWarning={false}
                        triggerLabel="选择已获取模型"
                        value={row.model}
                      />
                    </div>
                    <input
                      aria-label={`上下文窗口 ${index + 1}`}
                      inputMode="numeric"
                      onChange={(event) => updateSupplierCodexCatalogModel(row.rowId, { contextWindow: event.currentTarget.value.replace(/[^\d]/g, "") })}
                      placeholder="例如: 128000"
                      value={row.contextWindow}
                    />
                    <button aria-label="删除模型" className="supplier-codex-catalog-remove" onClick={() => removeSupplierCodexCatalogModel(row.rowId)} title="删除模型" type="button"><Trash2 className="h-4 w-4" /></button>
                  </div>
                )) : <p className="supplier-codex-catalog-empty">尚未添加模型；可手动添加，或先获取上游模型后从列表选择。</p>}
                <label className="ops-form-field">
                  <span>自定义 User-Agent</span>
                  <div className="supplier-user-agent-control">
                    <input onChange={(event) => updateDraft({ userAgent: event.currentTarget.value })} placeholder="Mozilla/5.0 ..." value={generated.userAgent || ""} />
                    <SupplierModelDropdown
                      compact
                      onChange={(value) => updateDraft({ userAgent: value })}
                      options={[...SUPPLIER_USER_AGENT_PRESETS]}
                      placeholder="选择 User-Agent 预设"
                      showAvailabilityWarning={false}
                      triggerLabel="预设"
                      value={generated.userAgent || ""}
                    />
                  </div>
                  <small>仅在本地路由或代理接管后生效，用于替换发送到供应商 API 的 User-Agent。</small>
                </label>
              </>
            )}
            <div className="supplier-ccswitch-divider" /><strong>本地代理请求覆盖</strong><p className="supplier-inline-note">仅在本地路由 / 代理接管后生效，应用于协议转换后的上游请求。</p><div className="supplier-ccswitch-form-grid two"><label className="ops-form-field"><span>Header 覆盖</span><textarea className="ops-textarea mono" onChange={(event) => updateDraft({ headerOverride: event.currentTarget.value })} readOnly={!showSupplierApiKey} rows={6} value={visibleHeaderOverride} placeholder={'{\n  "X-Provider": "cc-switch"\n}'} /></label><label className="ops-form-field"><span>Body 覆盖</span><textarea className="ops-textarea mono" onChange={(event) => updateDraft({ bodyOverride: event.currentTarget.value })} readOnly={!showSupplierApiKey} rows={6} value={visibleBodyOverride} placeholder={'{\n  "temperature": 0.2\n}'} /></label></div>{isClaudeSupplier ? <label className="ops-form-field"><span>配置 JSON</span><textarea className="ops-textarea mono supplier-config-json" onChange={(event) => updateDraft({ configContents: event.currentTarget.value })} readOnly={!showSupplierApiKey} value={visibleSupplierConfigJson} /></label> : <><label className="ops-form-field"><span>auth.json</span><textarea className="ops-textarea mono supplier-config-json compact" readOnly value={visibleCodexAuthJson} /></label><label className="ops-form-field"><span>config.toml</span><textarea className="ops-textarea mono supplier-config-json" readOnly value={visibleCodexConfigToml} /></label></>}{renderSourceCollapse(supplierTestConfigOpen, setSupplierTestConfigOpen, <Activity className="h-4 w-4" />, "模型 Test Config", <><p>为此供应商配置单独的模型测试参数。</p><div className="supplier-ccswitch-form-grid two"><label className="ops-form-field"><span>超时时间（秒）</span><input disabled placeholder="8" /></label><label className="ops-form-field"><span>降级阈值（毫秒）</span><input disabled placeholder="6000" /></label><label className="ops-form-field"><span>最大重试次数</span><input disabled placeholder="1" /></label></div></>)}{renderSourceCollapse(supplierPricingConfigOpen, setSupplierPricingConfigOpen, <BarChart3 className="h-4 w-4" />, "计费配置", <><p>为此供应商配置单独的计费参数。</p><div className="supplier-ccswitch-form-grid two"><label className="ops-form-field"><span>成本倍率</span><input disabled placeholder="留空使用全局默认" /></label><label className="ops-form-field"><span>计费模式</span><select className="ops-select" disabled value="inherit"><option value="inherit">继承全局默认</option><option value="request">请求模型</option><option value="response">返回模型</option></select></label></div></>)}
          </details></section></div>
        <div className="supplier-ccswitch-savebar"><span>{modelFetch?.models.length ? `已获取 ${modelFetch.models.length} 个模型，来源：${modelFetch.endpoint || "模型接口"}` : "请检查并保存供应商配置"}</span><div className="action-row"><Button onClick={closeSupplierEditor} type="button" variant="outline">取消</Button><Button disabled={supplierSaveBusy} onClick={() => void saveDraft()} type="button"><Save className="h-4 w-4" />{supplierSaveBusy ? "保存中" : "保存"}</Button><Button disabled={!canSwitch || supplierSaveBusy} onClick={() => void saveAndSwitchDraft()} type="button"><KeyRound className="h-4 w-4" />保存并使用</Button></div></div>
      </div>
    );
  }

  const credentialEnvironmentExternalSource = Boolean(
    credentialEnvironment?.externalSourceLikely,
  );
  const credentialEnvironmentScopeUnavailable = Boolean(
    credentialEnvironment?.present && !credentialEnvironment.userScopeAvailable,
  );


  return (
    <div className="supplier-list-shell">
      {credentialEnvironment && (credentialEnvironment.present || credentialEnvironment.restartRequired) ? <div className="supplier-env-card"><ShieldCheck className="h-5 w-5" /><div><strong>{credentialEnvironment.conflict ? "检测到凭据环境变量冲突" : "检测到凭据环境变量"}</strong><p>{credentialEnvironment.conflict ? `${credentialEnvironment.variableName} 与当前 Codex 供应商凭据不一致，可能覆盖 config.toml / auth.json 并导致 401；不会清理 CODEX_HOME。` : credentialEnvironment.present ? `${credentialEnvironment.variableName} 已存在，当前未发现与活动供应商的值冲突。` : `${credentialEnvironment.variableName} 已从 CCP 可管理的作用域清理。`}{credentialEnvironmentScopeUnavailable ? " 用户会话环境暂不可访问，CCP 未执行扩大范围的清理。" : ""}{credentialEnvironmentExternalSource ? " 该值来自 CCP 外部启动环境，需在原设置来源中清理。" : ""}{credentialEnvironment.restartRequired ? " 请完全退出并重新启动 Codex。" : ""}</p><span className="supplier-env-chip">{credentialEnvironment.variableName} {credentialEnvironment.userPresent ? "用户会话" : credentialEnvironment.systemPresent ? "系统环境" : credentialEnvironment.processPresent ? "当前进程" : "已清理"}</span></div><div className="supplier-env-actions"><Button disabled={!credentialEnvironment.canClearUser || credentialEnvironmentBusy} onClick={() => void clearCredentialEnvironment()} size="sm" variant="outline"><Trash2 className="h-4 w-4" />删除</Button><Button disabled={credentialEnvironmentBusy} onClick={() => void refreshCredentialEnvironment()} size="sm" variant="outline"><RefreshCw className={`h-4 w-4 ${credentialEnvironmentBusy ? "spin" : ""}`} />{credentialEnvironmentBusy ? "检测中" : "检测"}</Button></div></div> : null}
      <div className="supplier-control-row"><div className="supplier-route-master-toggle"><Network className="h-4 w-4" /><span>开启路由</span><ToggleSwitch checked={supplierRouteSwitchEnabled} disabled={supplierRouteSwitchDisabled} onChange={(value) => void toggleVisibleSupplierRouting(value)} /></div><div className="supplier-toolbar right"><div className="supplier-target-filter" aria-label="供应商目标应用过滤"><button className={supplierTargetFilter === "codex" ? "active" : ""} onClick={() => setSupplierTargetFilter("codex")} type="button">Codex</button><button className={supplierTargetFilter === "claude" ? "active" : ""} onClick={() => setSupplierTargetFilter("claude")} type="button">Claude</button><button className={supplierTargetFilter === "claude-desktop" ? "active" : ""} onClick={() => setSupplierTargetFilter("claude-desktop")} type="button">Claude Desktop</button></div><Button disabled={!appSettings} onClick={createProfile}><Plus className="h-4 w-4" />添加供应商</Button><Button disabled={!appSettings} onClick={createAggregateProfile} variant="outline"><Plus className="h-4 w-4" />添加聚合供应商</Button><div className="supplier-import-wrap"><Button onClick={() => setImportOpen((value) => !value)} variant="outline"><Download className="h-4 w-4" />从第三方导入</Button>{importOpen ? <div className="supplier-drop-popover"><button onClick={() => void importFromCcswitch()} type="button"><strong>ccswitch</strong><span>发现并导入 Codex / Claude / Claude Desktop 配置</span></button><button className={`supplier-menu-action ${supplierRefreshBusy ? "busy" : ""}`} disabled={supplierRefreshBusy} onClick={() => void refreshSupplierList()} type="button"><RefreshCw className={`h-4 w-4 ${supplierRefreshBusy ? "spin" : ""}`} />{supplierRefreshBusy ? "刷新中..." : "刷新列表"}</button></div> : null}</div></div></div>
      <div className="supplier-card-list">
        {filteredOrderedProfiles.length ? filteredOrderedProfiles.map((profile) => renderSupplierCard(profile)) : <Empty text="\u6682\u65e0\u4f9b\u5e94\u5546\u914d\u7f6e\uff0c\u70b9\u51fb\u201c\u6dfb\u52a0\u4f9b\u5e94\u5546\u201d\u521b\u5efa\u4e00\u4e2a\u771f\u5b9e\u53ef\u5207\u6362\u7684 Codex API \u914d\u7f6e\u3002" />}
      </div>
      {supplierDragOverlay && supplierDragOverlayProfile ? renderSupplierCard(supplierDragOverlayProfile, {
        overlay: true,
        style: {
          left: supplierDragOverlay.left,
          minHeight: supplierDragOverlay.height,
          top: supplierDragOverlay.top,
          width: supplierDragOverlay.width,
        },
      }) : null}
    </div>
  );
}
export function LegacySupplierScreen({
  actions,
  settings,
  claudeDesktopDevMode,
  claudeDesktopProviderPreview,
  claudeDesktopProviderApply,
  claudeDesktopProviderDraft,
  onClaudeDesktopProviderDraftChange,
}: {
  actions: AppActions;
  settings: SettingsResult | null;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeDesktopProviderPreview: ClaudeDesktopProviderPreviewResult | null;
  claudeDesktopProviderApply: ClaudeDesktopProviderApplyResult | null;
  claudeDesktopProviderDraft: {
    name: string;
    baseUrl: string;
    apiKey: string;
    modelList: string;
  };
  onClaudeDesktopProviderDraftChange: Dispatch<SetStateAction<{
    name: string;
    baseUrl: string;
    apiKey: string;
    modelList: string;
  }>>;
}) {
  const profiles = settings?.settings.relayProfiles ?? [];
  const active = profiles.find((profile) => profile.id === settings?.settings.activeRelayId) ?? profiles[0];
  const updateClaudeDraft = (field: keyof typeof claudeDesktopProviderDraft, value: string) => {
    onClaudeDesktopProviderDraftChange((draft) => ({ ...draft, [field]: value }));
  };
  return (
    <div className="ops-two-column">
      <div className="ops-wide-column">
        <Panel title="Codex 供应商" detail="复用现有 RelayProfile 真实写入 ~/.codex/config.toml 和 auth.json，失败会回滚设置。">
          <div className="info-grid">
            <InfoRow label="当前供应商" value={active?.name || active?.id || "未配置"} />
            <InfoRow label="模式" value={active?.relayMode || "official"} />
            <InfoRow label="模型" value={active?.model || settings?.settings.relayTestModel || "默认"} />
            <InfoRow label="接口地址" value={active?.baseUrl || settings?.settings.relayBaseUrl || "官方登录"} />
            <InfoRow label="配置路径" value={settings?.settings_path ?? "未加载"} />
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.applyRelayMode()}>
              <KeyRound className="h-4 w-4" />
              官方混入 API Key
            </Button>
            <Button onClick={() => void actions.applyPureApiMode()} variant="outline">
              <Network className="h-4 w-4" />
              纯 API
            </Button>
            <Button onClick={() => void actions.clearRelayMode()} variant="outline">
              <Trash2 className="h-4 w-4" />
              清除 API 模式
            </Button>
          </div>
        </Panel>
        <Panel title="Codex 供应商列表" detail={`${profiles.length} 个配置；点击切换会写入 Codex live 配置，不只是改 UI 状态。`}>
          <div className="ops-status-list">
            {profiles.length ? profiles.map((profile) => {
              const selected = profile.id === settings?.settings.activeRelayId;
              return (
                <div className="supplier-profile-row" key={profile.id}>
                  <StatusRow
                    label={profile.name || profile.id}
                    status={selected ? "running" : "not_checked"}
                    value={`${profile.relayMode || "official"} · ${profile.model || profile.testModel || "默认模型"}`}
                  />
                  <Button disabled={selected} onClick={() => void actions.switchCodexRelayProfile(profile.id)} variant="outline">
                    {selected ? "当前" : "切换"}
                  </Button>
                </div>
              );
            }) : <Empty text="暂无供应商配置。可在设置文件中添加 RelayProfile 后回到这里切换。" />}
          </div>
        </Panel>
        <Panel title="Claude Desktop 开发模式供应商" detail="写入 Claude Desktop 3P gateway profile；不修改 MSIX，不需要 Claude CLI 登录。">
          <div className="supplier-form-grid">
            <label className="ops-form-field">
              <span>显示名称</span>
              <input onChange={(event) => updateClaudeDraft("name", event.currentTarget.value)} value={claudeDesktopProviderDraft.name} />
            </label>
            <label className="ops-form-field">
              <span>Gateway Base URL</span>
              <input onChange={(event) => updateClaudeDraft("baseUrl", event.currentTarget.value)} placeholder="https://api.toporeduce.cn" value={claudeDesktopProviderDraft.baseUrl} />
            </label>
            <label className="ops-form-field">
              <span>API Key / Bearer Token</span>
              <input onChange={(event) => updateClaudeDraft("apiKey", event.currentTarget.value)} placeholder="写入前不会出现在日志和预览中" type="password" value={claudeDesktopProviderDraft.apiKey} />
            </label>
            <label className="ops-form-field span-2">
              <span>Claude Desktop 模型菜单，可选；一行一个，支持 [1m]</span>
              <textarea className="ops-textarea mono" onChange={(event) => updateClaudeDraft("modelList", event.currentTarget.value)} rows={5} value={claudeDesktopProviderDraft.modelList} />
            </label>
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.previewClaudeDesktopProvider(claudeDesktopProviderDraft)} variant="outline">
              <FileCode2 className="h-4 w-4" />
              预览写入
            </Button>
            <Button onClick={() => void actions.applyClaudeDesktopProvider(claudeDesktopProviderDraft)}>
              <KeyRound className="h-4 w-4" />
              写入 Claude Desktop
            </Button>
            <Button onClick={() => void actions.restoreClaudeDesktopProviderOfficial()} variant="outline">
              <Trash2 className="h-4 w-4" />
              恢复官方模式
            </Button>
          </div>
          {claudeDesktopProviderPreview?.preview.configDiff ? (
            <pre className="preview-box">{claudeDesktopProviderPreview.preview.configDiff}</pre>
          ) : null}
          {claudeDesktopProviderApply?.outcome.backupPaths?.length ? (
            <div className="risk-box">
              <strong>已创建备份</strong>
              <span>{claudeDesktopProviderApply.outcome.backupPaths.map(compactPath).join("；")}</span>
            </div>
          ) : null}
        </Panel>
      </div>
      <div className="stack">
        <Panel title="Codex 写入模式" detail="按使用场景选择，不混淆 Claude Desktop 插件安装。">
          <div className="ops-status-list">
            <StatusRow label="官方混入 API Key" status={active?.officialMixApiKey ? "running" : "not_checked"} value="保留官方账号能力，把模型请求转到自定义兼容 API。" />
            <StatusRow label="纯 API" status={active?.relayMode === "pure_api" ? "running" : "not_checked"} value="写入当前供应商 ID，并将 auth 状态切换到当前供应商。" />
            <StatusRow label="清除 API 模式" status="not_checked" value="移除中转 API 配置，回到官方 ChatGPT 登录态。" />
          </div>
        </Panel>
        <Panel title="Claude Desktop 3P 状态" detail="开发模式和 profile 写入状态，配置后需要重启 Claude Desktop。">
          <div className="info-grid compact">
            <InfoRow label="开发模式" value={claudeDesktopDevMode?.devModeStatus.configured ? "已配置" : "未配置"} />
            <InfoRow label="普通配置" value={compactPath(claudeDesktopDevMode?.devModeStatus.normalConfigPath)} />
            <InfoRow label="3P 配置" value={compactPath(claudeDesktopDevMode?.devModeStatus.threepConfigPath)} />
            <InfoRow label="Profile 元数据" value={compactPath(claudeDesktopDevMode?.devModeStatus.profileMetaPath)} />
          </div>
        </Panel>
        <Panel title="当前配置摘录" detail="只展示路径和非敏感字段。">
          <div className="info-grid compact">
            <InfoRow label="协议" value={active?.protocol || "responses"} />
            <InfoRow label="测试模型" value={active?.testModel || settings?.settings.relayTestModel || "默认"} />
          </div>
        </Panel>
      </div>
    </div>
  );
}

export const ToolsAndPluginsScreen = memo(function ToolsAndPluginsScreen({
  actions,
  aitrackerCapabilities,
  claudeDesktopMarketplace,
  codexPluginMarketplace,
  settings,
  unifiedInventory,
}: {
  actions: AppActions;
  aitrackerCapabilities: AitrackerCapabilitiesResult | null;
  claudeDesktopMarketplace: ClaudeDesktopMarketplaceStatusResult | null;
  codexPluginMarketplace: CodexPluginMarketplaceStatusResult | null;
  settings: SettingsResult | null;
  unifiedInventory: UnifiedToolInventoryResult | null;
}) {
  return (
    <div className="stack">
      <UnifiedToolInventoryPanel
        actions={actions}
        aitrackerCapabilities={aitrackerCapabilities}
        result={unifiedInventory}
        settings={settings?.settings ?? null}
      />
      <div className="repository-status-grid">
        <CodexPluginRepositoryPanel actions={actions} marketplace={codexPluginMarketplace} />
        <ClaudePluginRepositoryPanel actions={actions} marketplace={claudeDesktopMarketplace} />
      </div>
    </div>
  );
});

function UnifiedToolInventoryPanel({
  actions,
  aitrackerCapabilities,
  result,
  settings,
}: {
  actions: AppActions;
  aitrackerCapabilities: AitrackerCapabilitiesResult | null;
  result: UnifiedToolInventoryResult | null;
  settings: BackendSettings | null;
}) {
  const [tab, setTab] = useState<ContextKind>("mcp");
  const [pending, setPending] = useState<string | null>(null);
  const [creatingMcp, setCreatingMcp] = useState(false);
  const [creatingSkill, setCreatingSkill] = useState(false);
  const [skillTarget, setSkillTarget] = useState<"claude" | "codex">("codex");
  const [skillId, setSkillId] = useState("");
  const [skillBody, setSkillBody] = useState(defaultSkillBody());
  const [mcpTarget, setMcpTarget] = useState<"claude" | "codex">("codex");
  const [mcpId, setMcpId] = useState("");
  const [mcpBody, setMcpBody] = useState(defaultContextToml("mcp"));
  const inventory = result?.inventory;
  const inventoryAgents = useMemo(
    () => {
      const preferred = ["claude-code", "codex", "workbuddy", "cursor", "openclaw"];
      return [...(aitrackerCapabilities?.snapshot.registry ?? []).filter((agent) => agent.detected || (agent.skillCount ?? 0) > 0)].sort((left, right) => {
        const leftOrder = preferred.indexOf(left.id);
        const rightOrder = preferred.indexOf(right.id);
        return (leftOrder < 0 ? preferred.length : leftOrder) - (rightOrder < 0 ? preferred.length : rightOrder);
      });
    },
    [aitrackerCapabilities],
  );
  const entries = useMemo(
    () => (inventory?.assets ?? []).filter((asset) => asset.kind === tab),
    [inventory, tab],
  );
  const countFor = (kind: ContextKind) => {
    if (kind === "skill") return inventory?.counts.skills ?? 0;
    if (kind === "plugin") return inventory?.counts.plugins ?? 0;
    return inventory?.counts.mcp ?? 0;
  };
  const toggle = async (asset: UnifiedToolAsset, app: string, enabled: boolean) => {
    const key = `${asset.kind}:${asset.id}:${app}`;
    setPending(key);
    try {
      await actions.toggleUnifiedToolAsset(asset.id, asset.kind, app, enabled);
    } finally {
      setPending(null);
    }
  };
  const resetMcpDraft = () => {
    setCreatingMcp(false);
    setMcpTarget("codex");
    setMcpId("");
    setMcpBody(defaultContextToml("mcp"));
  };
  const resetSkillDraft = () => {
    setCreatingSkill(false);
    setSkillTarget("codex");
    setSkillId("");
    setSkillBody(defaultSkillBody());
  };
  const beginCreateMcp = () => {
    resetSkillDraft();
    setTab("mcp");
    setCreatingMcp(true);
    setMcpTarget("codex");
    setMcpId("");
    setMcpBody(defaultContextToml("mcp"));
  };
  const beginCreateSkill = () => {
    resetMcpDraft();
    setTab("skill");
    setCreatingSkill(true);
  };
  const saveMcp = async () => {
    const id = mcpId.trim();
    if (!id || !mcpBody.trim() || pending !== null) return;
    setPending("create:mcp");
    try {
      const saved = mcpTarget === "codex"
        ? settings
          ? await actions.saveContextEntry("mcp", id, mcpBody, settings)
          : null
        : await actions.saveClaudeContextEntry("mcp", id, mcpBody);
      if (saved) {
        await actions.refreshUnifiedToolInventory(true);
        if (statusOk(saved.status)) resetMcpDraft();
      }
    } finally {
      setPending(null);
    }
  };
  const saveSkill = async () => {
    const id = skillId.trim();
    if (!id || !skillBody.trim() || pending !== null) return;
    setPending("create:skill");
    try {
      const saved = await actions.createSkill(skillTarget, id, skillBody);
      if (saved && statusOk(saved.status)) resetSkillDraft();
    } finally {
      setPending(null);
    }
  };

  return (
    <section className="context-manager-card unified-tool-inventory">
      <header className="context-manager-head">
        <div>
          <h2>Claude、Codex 工具与插件</h2>
          <p>完整检测两端本地资产；同一资产只显示一行，点亮应用图标即启用到对应应用。</p>
        </div>
        <div className="action-row">
          <Button
            aria-controls={tab === "mcp" ? "unified-mcp-editor" : "unified-skill-editor"}
            aria-expanded={tab === "mcp" ? creatingMcp : creatingSkill}
            disabled={pending !== null}
            onClick={tab === "mcp" ? (creatingMcp ? resetMcpDraft : beginCreateMcp) : (creatingSkill ? resetSkillDraft : beginCreateSkill)}
            size="sm"
          >
            <Plus className="h-4 w-4" />
            {tab === "mcp" ? (creatingMcp ? "收起新增 MCP" : "新增 MCP") : (creatingSkill ? "收起新增 Skill" : "新增 Skill")}
          </Button>
          <Button disabled={pending !== null} onClick={async () => {
            setPending("scan");
            try {
              await actions.refreshUnifiedToolInventory(false);
            } finally {
              setPending(null);
            }
          }} size="sm" variant="outline">
            <RefreshCw className={`h-4 w-4${pending === "scan" ? " spin" : ""}`} />
            {pending === "scan" ? "检测中" : "重新检测"}
          </Button>
        </div>
      </header>
      {creatingMcp ? (
        <div aria-label="新增 MCP" className="context-editor" id="unified-mcp-editor" role="region">
          <div className="context-editor-grid">
            <label className="ops-form-field">
              <span>目标应用</span>
              <select
                className="ops-select"
                disabled={pending !== null}
                onChange={(event) => {
                  const target = event.currentTarget.value as "claude" | "codex";
                  setMcpTarget(target);
                  setMcpBody(target === "codex" ? defaultContextToml("mcp") : defaultClaudeContextBody("mcp"));
                }}
                value={mcpTarget}
              >
                <option value="codex">Codex</option>
                <option value="claude">Claude</option>
              </select>
            </label>
            <label className="ops-form-field">
              <span>MCP ID</span>
              <input
                autoComplete="off"
                disabled={pending !== null}
                onChange={(event) => setMcpId(event.currentTarget.value)}
                placeholder="例如：filesystem"
                value={mcpId}
              />
            </label>
          </div>
          <label className="ops-form-field">
            <span>{mcpTarget === "codex" ? "TOML 配置体" : "JSON 配置体"}</span>
            <textarea
              className="ops-textarea context-toml-editor mono"
              disabled={pending !== null}
              onChange={(event) => setMcpBody(event.currentTarget.value)}
              spellCheck={false}
              value={mcpBody}
            />
          </label>
          <p className={`context-manager-note${mcpTarget === "codex" && !settings ? " warning" : ""}`}>
            {mcpTarget === "codex" && !settings
              ? "Codex 设置尚未加载，重新检测后再保存。"
              : `只写入 ${mcpTarget === "codex" ? "Codex" : "Claude"}，不会改变另一端。`}
          </p>
          <div className="action-row">
            <Button
              disabled={pending !== null || !mcpId.trim() || !mcpBody.trim() || (mcpTarget === "codex" && !settings)}
              onClick={() => void saveMcp()}
              size="sm"
            >
              <Save className="h-4 w-4" />
              {pending === "create:mcp" ? "保存中" : "保存 MCP"}
            </Button>
            <Button disabled={pending !== null} onClick={resetMcpDraft} size="sm" variant="outline">
              取消
            </Button>
          </div>
        </div>
      ) : null}
      {creatingSkill ? (
        <div aria-label="新增 Skill" className="context-editor" id="unified-skill-editor" role="region">
          <div className="context-editor-grid">
            <label className="ops-form-field">
              <span>目标应用</span>
              <select className="ops-select" disabled={pending !== null} onChange={(event) => setSkillTarget(event.currentTarget.value as "claude" | "codex")} value={skillTarget}>
                <option value="codex">Codex</option>
                <option value="claude">Claude</option>
              </select>
            </label>
            <label className="ops-form-field">
              <span>Skill ID</span>
              <input autoComplete="off" disabled={pending !== null} onChange={(event) => setSkillId(event.currentTarget.value)} placeholder="例如：review-helper" value={skillId} />
            </label>
          </div>
          <label className="ops-form-field">
            <span>SKILL.md</span>
            <textarea className="ops-textarea context-toml-editor mono" disabled={pending !== null} onChange={(event) => setSkillBody(event.currentTarget.value)} spellCheck={false} value={skillBody} />
          </label>
          <p className="context-manager-note">将写入 {skillTarget === "codex" ? "~/.codex/skills" : "~/.claude/skills"} 的真实 Skill 目录。</p>
          <div className="action-row">
            <Button disabled={pending !== null || !skillId.trim() || !skillBody.trim()} onClick={() => void saveSkill()} size="sm"><Save className="h-4 w-4" />{pending === "create:skill" ? "保存中" : "保存 Skill"}</Button>
            <Button disabled={pending !== null} onClick={resetSkillDraft} size="sm" variant="outline">取消</Button>
          </div>
        </div>
      ) : null}
      <div className="unified-tool-countbar">
        <span>共 {inventory?.counts.total ?? 0} 项</span>
        <span>原始发现 {inventory?.counts.rawDiscoveries ?? 0}</span>
        <span>已合并 {inventory?.counts.deduplicated ?? 0}</span>
        <span className="claude-count">Claude {inventory?.counts.claudeEnabled ?? 0}</span>
        <span className="codex-count">Codex {inventory?.counts.codexEnabled ?? 0}</span>
      </div>
      <div className="context-tabs">
        {(["mcp", "skill", "plugin"] as ContextKind[]).map((kind) => (
          <button className={tab === kind ? "active" : ""} key={kind} onClick={() => setTab(kind)} type="button">
            <strong>{contextKindLabel(kind)}</strong>
            <span>{countFor(kind)}</span>
          </button>
        ))}
      </div>
      {inventory?.diagnostics.length ? (
        <p className="context-manager-note warning">检测到 {inventory.diagnostics.length} 项非致命诊断；其余可读来源仍已加载。</p>
      ) : (
        <p className="context-manager-note">已扫描 {inventory?.scannedSources.length ?? 0} 个配置或目录来源。</p>
      )}
      <div className="context-entry-list unified-tool-list">
        {entries.length ? entries.map((asset) => (
          <div className="context-entry-row unified-tool-row" key={`${asset.kind}:${asset.id}`}>
            <div className="unified-tool-copy">
              <strong>{asset.title || asset.id}</strong>
              {asset.summary ? <span title={asset.summary}>{asset.summary}</span> : null}
              {asset.source ? <small title={asset.source}>{compactPath(asset.source)}</small> : null}
            </div>
            <div className="agent-toggle-group" aria-label={`${asset.title} 应用状态`}>
              {(asset.kind === "skill"
                ? inventoryAgents
                : inventoryAgents.filter((agent) => agent.id === "claude-code" || agent.id === "codex")
              ).map((agent) => {
                const app = agent.id === "claude-code" ? "claude" : agent.id === "codex" ? "codex" : agent.id;
                const state = agent.id === "claude-code" ? asset.claude : agent.id === "codex" ? asset.codex : asset.agents?.[agent.id] ?? {
                  enabled: false,
                  available: false,
                  toggleSupported: false,
                  sourcePath: "",
                };
                const appName = agent.nameZh || agent.name;
                const key = `${asset.kind}:${asset.id}:${app}`;
                const isPending = pending === key;
                return (
                  <button
                    aria-label={`${state.enabled ? "关闭" : "启用"} ${appName}：${asset.title}`}
                    aria-pressed={state.enabled}
                    className={`agent-toggle ${app} ${state.enabled ? "enabled" : "disabled"}${isPending ? " pending" : ""}`}
                    disabled={!state.toggleSupported || pending !== null}
                    key={agent.id}
                    onClick={() => void toggle(asset, app, !state.enabled)}
                    style={agent.id === "claude-code" || agent.id === "codex" ? undefined : { color: agent.color }}
                    title={`${appName}${state.enabled ? " ✓（点击关闭）" : state.available ? "（点击启用）" : "（未发现可用来源）"}`}
                    type="button"
                  >
                    <AgentInventoryIcon agent={agent} />
                  </button>
                );
              })}
            </div>
          </div>
          )) : <Empty text={result ? `未发现${contextKindLabel(tab)}；可点击重新检测查看最新本地状态。` : "尚未检测本地工具与插件。"} />}
      </div>
    </section>
  );
}

export function CodexPluginRepositoryPanel({
  actions,
  marketplace,
}: {
  actions: AppActions;
  marketplace: CodexPluginMarketplaceStatusResult | null;
}) {
  const status = marketplace?.marketplace;
  const health: Status = !marketplace ? "not_checked" : status?.needsRepair ? "needs_review" : statusOk(marketplace.status) ? "ok" : marketplace.status;
  const repositories = status?.repositories?.length
    ? status.repositories
    : status
      ? [
          {
            label: "第三方插件仓库",
            name: CODEX_THIRD_PARTY_PLUGIN_MARKETPLACE_NAME,
            sourceType: "git",
            source: CODEX_THIRD_PARTY_PLUGIN_REPOSITORY_URL,
            configured: false,
          },
          {
            label: "产品设计技能仓库",
            name: CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_NAME,
            sourceType: "local",
            source: `${CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_LOCAL_SOURCE} / ${CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_SOURCE}`,
            configured: false,
          },
          {
            label: "Matt Pocock Skills 仓库",
            name: CODEX_MATT_POCOCK_SKILLS_MARKETPLACE_NAME,
            sourceType: "local",
            source: `${CODEX_MATT_POCOCK_SKILLS_MARKETPLACE_LOCAL_SOURCE} / ${CODEX_MATT_POCOCK_SKILLS_MARKETPLACE_SOURCE}`,
            configured: false,
          },
        ]
      : [];
  return (
    <Panel title="Codex 插件仓库" detail="自动下载、校验并把 OpenAI 与第三方插件仓库注册到 Codex 配置；具体插件安装仍在 Codex 内确认。">
      <div className="ops-status-list">
        <StatusRow label="仓库状态" status={health} value={status?.message || marketplace?.message || "尚未检测 Codex 插件仓库"} />
        <StatusRow label="配置写入" status={status?.configRegistered ? "ok" : status?.needsRepair ? "needs_review" : "not_checked"} value={status?.configRegistered ? "已写入 Codex 配置" : "未写入或待检测"} />
        <StatusRow label="本地来源" status={status?.localSourcesReady ? "ok" : "needs_review"} value={status?.localSourcesReady ? "仓库快照存在并可读取" : "部分仓库只有配置，缺少本地来源"} />
        <StatusRow label="应用可见" status={status?.configRegistered && status?.localSourcesReady ? "needs_review" : "not_checked"} value={status?.runtimeConfirmation || "尚未确认"} />
        {repositories.map((repository) => (
          <StatusRow
            key={`${repository.name}:${repository.source}`}
            label={repository.label}
            status={repository.configured ? "ok" : "needs_review"}
            value={`${repository.name} / ${repository.sourceType} / ${repository.configured ? "配置已写入" : "配置未写入"} / ${repository.source}`}
          />
        ))}
        <StatusRow label="本地目录" status={status?.marketplaceRoot ? "found" : "not_checked"} value={compactPath(status?.marketplaceRoot)} />
      </div>
      <div className="action-row">
        <Button onClick={() => void actions.refreshCodexPluginMarketplace()} variant="outline">
          <RefreshCw className="h-4 w-4" />
          刷新 Codex 插件仓库
        </Button>
        <Button onClick={() => void actions.repairCodexPluginMarketplace()}>
          <Download className="h-4 w-4" />
          修复 Codex 插件仓库
        </Button>
      </div>
    </Panel>
  );
}

export function ClaudePluginRepositoryPanel({
  actions,
  marketplace,
}: {
  actions: AppActions;
  marketplace: ClaudeDesktopMarketplaceStatusResult | null;
}) {
  const status = marketplace?.marketplaceStatus;
  const repositories = status?.repositories ?? [];
  const allConfigured = repositories.length > 0 && repositories.every((repository) => repository.configured);
  const health: Status = !marketplace ? "not_checked" : allConfigured ? "ok" : status?.supported ? "needs_review" : statusOk(marketplace.status) ? "ok" : marketplace.status;
  const repositorySummary = repositories.length
    ? repositories.map((repository) => `${repository.label}: ${repository.repository}`).join("；")
    : "尚未检测";
  return (
    <Panel title="Claude 插件仓库" detail="自动写入 Claude 开发配置中的已知插件仓库；具体插件安装仍由 Claude 官方流程确认。">
      <div className="ops-status-list">
        <StatusRow label="仓库状态" status={health} value={status?.message || marketplace?.message || "尚未检测 Claude 插件仓库"} />
        <StatusRow label="配置方式" status={status?.canAutoWrite ? "ok" : status?.supported ? "needs_review" : "not_checked"} value={status?.canAutoWrite ? "可自动写入" : status?.supported ? "待修复" : "未检测"} />
        <StatusRow label="应用可见" status={allConfigured ? "needs_review" : "not_checked"} value={allConfigured ? "配置已写入，待重启 Claude 确认" : "尚未确认"} />
        <StatusRow label="仓库列表" status={repositories.length ? (allConfigured ? "ok" : "needs_review") : "not_checked"} value={repositorySummary} />
        {repositories.map((repository) => (
          <StatusRow
            key={repository.repository}
            label={repository.label}
            status={repository.configured ? "ok" : "needs_review"}
            value={`${repository.repository} / ${repository.configured ? "配置已写入" : "配置未写入"}`}
          />
        ))}
        <StatusRow label="配置路径" status={status?.configPath ? "found" : "not_checked"} value={compactPath(status?.configPath)} />
      </div>
      <div className="action-row">
        <Button onClick={() => void actions.refreshClaudeDesktopMarketplace()} variant="outline">
          <RefreshCw className="h-4 w-4" />
          刷新 Claude 插件仓库
        </Button>
        <Button onClick={() => void actions.repairClaudeDesktopMarketplaces()}>
          <Wrench className="h-4 w-4" />
          修复 Claude 插件仓库
        </Button>
      </div>
    </Panel>
  );
}
function formatAitrackerTokens(value: number) {
  return value.toLocaleString("zh-CN");
}

function aitrackerDateKey(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toISOString().slice(0, 10);
}

function aitrackerDateLabel(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    weekday: "short",
  });
}

function aitrackerStatusLabel(value: string) {
  switch (value.toLowerCase()) {
    case "ok":
    case "success":
    case "completed":
    case "observed":
      return "可恢复";
    case "running":
      return "运行中";
    case "failed":
    case "error":
      return "异常";
    default:
      return value || "已采集";
  }
}

const sessionAgentLogos: Record<string, string> = {
  codex: codexLogo,
  "claude-code": claudeLogo,
  workbuddy: workbuddyLogo,
  cursor: cursorLogo,
  "deepseek-harness": deepseekLogo,
  openclaw: openclawLogo,
};

const inventoryAgentLogos: Record<string, string> = {
  codex: codexLogo,
  "claude-code": claudeLogo,
  workbuddy: workbuddyLogo,
  cursor: cursorLogo,
  "deepseek-harness": deepseekLogo,
  openclaw: openclawLogo,
};

function defaultSkillBody() {
  return "---\nname: new-skill\ndescription: Describe what this skill does.\n---\n\n# New Skill\n\nAdd the instructions for this skill here.\n";
}

function AgentInventoryIcon({ agent }: { agent: { id: string; name: string; color: string } }) {
  const logo = inventoryAgentLogos[agent.id];
  return logo
    ? <img alt="" aria-hidden="true" src={logo} />
    : <span aria-hidden="true" style={{ color: agent.color }}>{agent.name.slice(0, 1).toUpperCase()}</span>;
}

function SessionAgentIcon({ agent, label }: { agent: string; label: string }) {
  const logo = sessionAgentLogos[agent];
  return logo
    ? <img alt="" aria-hidden="true" className="aitracker-session-tool-icon" src={logo} />
    : <span aria-hidden="true" className="aitracker-session-tool-icon aitracker-session-tool-mark">{label.slice(0, 1).toUpperCase()}</span>;
}

function AitrackerSessionPanel({
  actions,
  capabilities,
  distillationResult,
}: {
  actions: AppActions;
  capabilities: AitrackerCapabilitiesResult | null;
  distillationResult: DistillationCandidatesResult | null;
}) {
  const [agent, setAgent] = useState("");
  const [keyword, setKeyword] = useState("");
  const [range, setRange] = useState<AitrackerSessionRange>("30d");
  const [page, setPage] = useState(1);
  const [selected, setSelected] = useState<AitrackerSessionSummary | null>(null);
  const [detail, setDetail] = useState<AitrackerSessionDetailResult | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [queryLoading, setQueryLoading] = useState(false);
  const [historyKeyword, setHistoryKeyword] = useState("");
  const detailEpoch = useRef(0);

  const sessions = capabilities?.snapshot.sessions ?? [];
  const candidates = distillationResult?.candidates ?? [];
  const pageSize = 20;
  const filteredSessions = useMemo(() => {
    const search = keyword.trim().toLocaleLowerCase();
    const days = range === "all" ? 0 : range === "7d" ? 7 : range === "30d" ? 30 : 90;
    const cutoff = days ? Date.now() - days * 86_400_000 : 0;
    return sessions.filter((session) => {
      const started = Date.parse(session.startedAt);
      return (!agent || session.agent === agent)
        && (!cutoff || Number.isNaN(started) || started >= cutoff)
        && (!search || [session.agent, session.provider, session.model, session.project, session.sessionId]
          .some((value) => value.toLocaleLowerCase().includes(search)));
    });
  }, [sessions, agent, keyword, range]);
  const total = filteredSessions.length;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const currentPage = Math.min(page, pageCount);
  const visibleSessions = filteredSessions.slice((currentPage - 1) * pageSize, currentPage * pageSize);
  const agentRegistry = capabilities?.snapshot.registry ?? [];
  const agentDefinitions = useMemo(() => new Map(agentRegistry.map((item) => [item.id, item])), [agentRegistry]);
  const agents = useMemo(() => {
    const observed = new Set([
      ...(capabilities?.snapshot.sessions ?? []).map((item) => item.agent),
      ...sessions.map((item) => item.agent),
    ].filter(Boolean));
    const detected = agentRegistry.filter((item) => item.detected || item.events > 0).map((item) => item.id);
    const priority = new Map([["codex", 0], ["claude-code", 1], ["workbuddy", 2]]);
    return Array.from(new Set([...observed, ...detected])).sort((left, right) => {
      const priorityDelta = (priority.get(left) ?? 10) - (priority.get(right) ?? 10);
      if (priorityDelta) return priorityDelta;
      return (agentDefinitions.get(left)?.nameZh || agentDefinitions.get(left)?.name || left).localeCompare(
        agentDefinitions.get(right)?.nameZh || agentDefinitions.get(right)?.name || right,
        "zh-CN",
      );
    });
  }, [agentDefinitions, agentRegistry, capabilities, sessions]);
  const agentLabel = (name: string) => agentDefinitions.get(name)?.nameZh || agentDefinitions.get(name)?.name || name;
  const selectedAgentDetail = useMemo(
    () => capabilities?.snapshot.details.find((item) => item.id === selected?.agent) ?? null,
    [capabilities, selected],
  );
  const toolUsage = useMemo(() => {
    const counts = new Map<string, number>();
    for (const event of detail?.detail?.events ?? []) {
      if (event.toolName) counts.set(event.toolName, (counts.get(event.toolName) ?? 0) + 1);
    }
    return Array.from(counts.entries()).sort((a, b) => b[1] - a[1]);
  }, [detail]);
  const sessionGroups = useMemo(() => {
    const groups = new Map<string, AitrackerSessionSummary[]>();
    for (const session of visibleSessions) {
      const key = aitrackerDateKey(session.startedAt);
      const group = groups.get(key) ?? [];
      group.push(session);
      groups.set(key, group);
    }
    const today = aitrackerDateKey(new Date().toISOString());
    const yesterdayDate = new Date();
    yesterdayDate.setDate(yesterdayDate.getDate() - 1);
    const yesterday = aitrackerDateKey(yesterdayDate.toISOString());
    return Array.from(groups.entries()).map(([dateKey, items]) => ({
      dateKey,
      items,
      label: aitrackerDateLabel(items[0]?.startedAt ?? dateKey),
      suffix: dateKey === today ? "今天" : dateKey === yesterday ? "昨天" : "",
    }));
  }, [visibleSessions]);

  const historyGroups = useMemo(() => {
    const search = historyKeyword.trim().toLocaleLowerCase();
    const source = [selected, ...(capabilities?.snapshot.sessions ?? sessions)]
      .filter((session): session is AitrackerSessionSummary => Boolean(session))
      .filter((session) => session.agent === selected?.agent)
      .filter((session) => !search || [session.project, session.sessionId, session.model].some((value) => value.toLocaleLowerCase().includes(search)))
      .sort((left, right) => right.startedAt.localeCompare(left.startedAt));
    const groups = new Map<string, AitrackerSessionSummary[]>();
    const seen = new Set<string>();
    for (const session of source) {
      if (seen.has(session.sessionId)) continue;
      seen.add(session.sessionId);
      const key = aitrackerDateKey(session.startedAt);
      groups.set(key, [...(groups.get(key) ?? []), session]);
    }
    return Array.from(groups.entries());
  }, [capabilities, historyKeyword, selected, sessions]);

  const refresh = async () => {
    setQueryLoading(true);
    try {
      await actions.refreshAitrackerCapabilities(true);
    } finally {
      setQueryLoading(false);
    }
  };
  const openSession = async (session: AitrackerSessionSummary) => {
    const epoch = ++detailEpoch.current;
    const localDetail = {
      summary: session,
      events: (capabilities?.usage?.details ?? []).filter((event) => event.agent === session.agent && event.sessionId === session.sessionId),
    };
    setSelected(session);
    setDetail({ status: "ok", message: "本地事件已加载。", detail: localDetail, transcript: null });
    setDetailLoading(true);
    const result = await actions.readAitrackerSessionDetail({ agent: session.agent, sessionId: session.sessionId, detail: localDetail });
    if (epoch === detailEpoch.current) {
      if (result) setDetail({ ...result, detail: result.detail ?? localDetail });
      setDetailLoading(false);
    }
  };
  const selectedCandidate = selected
    ? candidates.find((item) => item.agent === selected.agent && item.sessionId === selected.sessionId && item.status === "pending")
    : null;

  if (selected) return (
    <div className="aitracker-session-history" aria-label="会话历史">
      <aside className="aitracker-session-history-sidebar" aria-label="会话历史列表">
        <header><MessageCircle aria-hidden="true" /><strong>会话历史</strong><span>{historyGroups.reduce((count, [, items]) => count + items.length, 0)}</span></header>
        <label className="aitracker-session-history-search"><Search aria-hidden="true" /><input aria-label="搜索会话历史" onChange={(event) => setHistoryKeyword(event.currentTarget.value)} placeholder="搜索项目 / 会话 ID / 模型" value={historyKeyword} /></label>
        <div className="aitracker-session-history-items">
          {historyGroups.length ? historyGroups.map(([date, items]) => (
            <div className="aitracker-session-history-group" key={date}>
              <time>{aitrackerDateLabel(items[0].startedAt)}</time>
              {items.map((session) => (
                <button className={selected.sessionId === session.sessionId && selected.agent === session.agent ? "active" : ""} key={`${session.agent}:${session.sessionId}`} onClick={() => void openSession(session)} title={session.project || session.sessionId} type="button">
                  <strong>{session.sessionId === selected.sessionId ? detail?.transcript?.title || session.project || "未采集标题" : session.project || "未采集标题"}</strong>
                  <small>{new Date(session.startedAt).toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" })} · {session.model || agentLabel(session.agent)}</small>
                </button>
              ))}
            </div>
          )) : <Empty text="没有匹配的会话。" />}
        </div>
      </aside>
      <section className="aitracker-session-history-main" aria-label="会话内容">
        <header className="aitracker-session-history-header">
          <button aria-label="返回会话列表" className="aitracker-session-history-back" onClick={() => { detailEpoch.current += 1; setSelected(null); setDetail(null); }} title="返回会话列表" type="button"><ArrowLeft aria-hidden="true" /></button>
          <div><h2>{detail?.transcript?.title || selected.project || "未采集标题"}</h2><p><SessionAgentIcon agent={selected.agent} label={agentLabel(selected.agent)} />{agentLabel(selected.agent)} <span>·</span> {new Date(selected.startedAt).toLocaleString("zh-CN")} <span>·</span> {selected.project || "项目未采集"} <span>·</span> {detail?.transcript?.totalMessages ?? selected.events} 条记录</p></div>
          <Button disabled={Boolean(selectedCandidate)} onClick={() => void actions.createDistillationCandidate({ agent: selected.agent, sessionId: selected.sessionId })} size="sm" variant="outline"><Sparkles className="h-4 w-4" />{selectedCandidate ? "已加入蒸馏" : "蒸馏"}</Button>
        </header>
        <div className="aitracker-session-history-scroll">
          {detail?.detail ? (
            <>
              <div className="aitracker-session-history-source"><strong>{detail.transcript ? "本地对话记录" : "本地用量事件"}</strong><span>{detail.transcript ? `${detail.transcript.totalMessages} 条消息` : `${detail.detail.events.length} 条事件`}</span><small>{detailLoading ? "正在读取本地对话正文..." : detail.transcript ? "对话仅从本机读取" : "该工具尚未采集对话正文，以下是实际采集到的请求和工具事件"}</small></div>
              {detail.transcript?.hasMoreBefore ? <p className="aitracker-session-history-note">当前展示最近 200 条消息，较早消息未加载。</p> : null}
              {detail.transcript ? (
                detail.transcript.messages.length ? <div className="aitracker-session-messages">{detail.transcript.messages.map((message, index) => (
                  <article className={`aitracker-session-message ${message.role === "user" ? "user" : message.role === "assistant" ? "assistant" : "other"}`} key={`${selected.sessionId}:${index}`}>
                    <header className="aitracker-session-message-identity">
                      {message.role === "user" ? <UserRound aria-hidden="true" /> : message.role === "assistant" ? <SessionAgentIcon agent={selected.agent} label={agentLabel(selected.agent)} /> : <Info aria-hidden="true" />}
                      <strong>{message.role === "user" ? "用户" : message.role === "assistant" ? `AI 智能体 · ${agentLabel(selected.agent)}` : `来源 · ${message.role}`}</strong>
                    </header>
                    <p>{message.text}</p>
                  </article>
                ))}</div> : <Empty text="该会话没有可读取的对话消息。" />
              ) : detail.detail.events.length ? <div className="aitracker-session-events">{detail.detail.events.map((event) => (
                <article className="aitracker-session-event" key={event.id}><div><strong>{event.toolName || event.model || "请求事件"}</strong><time>{event.timestamp}</time></div><span>{event.status || "已采集"} · {formatAitrackerTokens(event.totalTokens)} tokens{event.provider ? ` · ${event.provider}` : ""}</span></article>
              ))}</div> : detailLoading ? <div className="aitracker-session-detail-loading"><RefreshCw className="h-5 w-5 animate-spin" />正在读取本地对话...</div> : <Empty text="该会话暂无可解析事件。" />}
              {detail.transcript && detail.detail.events.length ? <details className="aitracker-session-history-analysis"><summary>工具调用与用量分析</summary><p>{formatAitrackerTokens(detail.detail.summary.totals.totalTokens)} tokens · {detail.detail.summary.toolCalls} 次工具调用{selectedAgentDetail ? ` · Agent 累计 ${selectedAgentDetail.sessions} 场会话` : ""}</p>{toolUsage.map(([name, count]) => <span key={name}>{name} ×{count}</span>)}</details> : null}
            </>
          ) : <Empty text={detail?.message || "该会话暂无可读取详情。"} />}
        </div>
      </section>
    </div>
  );

  return (
    <div className="aitracker-session-page">
      <div className="aitracker-session-summary" aria-label="会话统计">
        <div><span>会话数</span><strong>{total.toLocaleString("zh-CN")}</strong><small>近 30 天的会话</small><MessageCircle aria-hidden="true" /></div>
        <div><span>会话工具数</span><strong>{capabilities?.snapshot.toolCalls.length.toLocaleString("zh-CN") ?? "0"}</strong><small>覆盖的 Agent 工具</small><Wrench aria-hidden="true" /></div>
        <div><span>对话轮次数</span><strong>{sessions.reduce((sum, item) => sum + item.events, 0).toLocaleString("zh-CN")}</strong><small>累计会话轮次</small><Sparkles aria-hidden="true" /></div>
      </div>
      <section className="aitracker-session-content" aria-label="会话与 Agent 数据">
        <div className="codex-session-toolbar">
          <label className="aitracker-session-search"><Search aria-hidden="true" /><input onChange={(event) => { setKeyword(event.currentTarget.value); setPage(1); }} placeholder="搜索标题 / 项目 / 会话 ID" value={keyword} /></label>
          <div className="aitracker-session-range" role="group" aria-label="会话时间范围">
            {(["7d", "30d", "90d", "all"] as const).map((value) => (
              <button className={range === value ? "active" : ""} key={value} onClick={() => { setRange(value); setPage(1); }} type="button">
                {value === "7d" ? "近 7 天" : value === "30d" ? "近 30 天" : value === "90d" ? "近 90 天" : "全部"}
              </button>
            ))}
            <Button disabled={queryLoading} onClick={() => void refresh()} size="sm" variant="outline"><RefreshCw className="h-4 w-4" />立即刷新</Button>
          </div>
        </div>
        <div className="aitracker-session-tools" role="tablist" aria-label="Agent 工具筛选">
          {["", ...agents].map((value) => {
            const label = value ? agentLabel(value) : "全部工具";
            return <button className={agent === value ? "active" : ""} key={label} onClick={() => { setAgent(value); setPage(1); }} type="button">
              {value ? <SessionAgentIcon agent={value} label={label} /> : null}
              {label}
            </button>;
          })}
        </div>
        {!capabilities ? <div className="aitracker-session-detail-loading"><RefreshCw className="h-5 w-5 animate-spin" />正在读取会话快照...</div> : visibleSessions.length ? (
          <div className="aitracker-session-groups" aria-label="按日期分组的会话列表">
            {sessionGroups.map((group) => (
              <section className="aitracker-session-day" key={group.dateKey}>
                <div className="aitracker-session-day-heading"><strong>{group.label}{group.suffix ? ` · ${group.suffix}` : ""}</strong><span>{group.items.length} 场</span></div>
                <div className="aitracker-session-day-list">
                  {group.items.map((session) => (
                    <article className="aitracker-session-card" key={`${session.agent}:${session.sessionId}`}>
                      <button className="aitracker-session-card-main" onClick={() => void openSession(session)} title={`${session.agent} / ${session.sessionId}`} type="button">
                        <div className="aitracker-session-card-source"><SessionAgentIcon agent={session.agent} label={agentLabel(session.agent)} /><span>{agentLabel(session.agent)}</span><em>{aitrackerStatusLabel(session.status)}</em></div>
                        <strong>{session.project || "未采集项目"}</strong>
                        <small>{session.provider || "Provider 未采集"} · {session.model || "模型未采集"} · {session.events} 轮 · {session.toolCalls} 次工具调用 · {formatAitrackerTokens(session.totals.totalTokens)} tokens</small>
                      </button>
                      <Button className="aitracker-session-resume" onClick={() => void openSession(session)} size="sm"><MessageCircle className="h-3.5 w-3.5" />查看会话</Button>
                    </article>
                  ))}
                </div>
              </section>
            ))}
            <div className="aitracker-session-pagination">
              <Button disabled={currentPage <= 1} onClick={() => setPage(currentPage - 1)} size="sm" variant="outline"><ChevronLeft className="h-4 w-4" />上一页</Button>
              <span>第 {currentPage} / {pageCount} 页，共 {total} 个会话</span>
              <Button disabled={currentPage >= pageCount} onClick={() => setPage(currentPage + 1)} size="sm" variant="outline">下一页<ChevronRight className="h-4 w-4" /></Button>
            </div>
          </div>
        ) : <Empty text={agent ? `暂无 ${agentLabel(agent)} 的本地采集会话。` : "暂无本地采集会话。"} />}
      </section>
    </div>
  );
}

export const SessionManagementScreen = memo(function SessionManagementScreen({
  actions,
  aitrackerCapabilities,
  distillationCandidates,
}: {
  actions: AppActions;
  aitrackerCapabilities: AitrackerCapabilitiesResult | null;
  distillationCandidates: DistillationCandidatesResult | null;
}) {
  return (
    <div className="stack">
      <AitrackerSessionPanel actions={actions} capabilities={aitrackerCapabilities} distillationResult={distillationCandidates} />
    </div>
  );
});

export const PluginListItem = memo(function PluginListItem({
  item,
  isSelected,
  onSelect,
}: {
  item: PluginCatalogItem;
  isSelected: boolean;
  onSelect: (id: string) => void;
}) {
  return (
    <button className={isSelected ? "active" : ""} onClick={() => onSelect(item.id)} type="button">
      <div>
        <strong>{item.name}</strong>
        <p>{item.description || item.homepage}</p>
      </div>
      <span className={`status-chip ${item.installStatus}`}>{pluginStatusLabel(item.installStatus)}</span>
    </button>
  );
});

export function PluginHubScreen({
  actions,
  devMode,
  hub,
  preview,
  orgPlugin,
  marketplace,
}: {
  actions: AppActions;
  devMode: ClaudeDesktopDevModeStatusResult | null;
  hub: PluginHubResult | null;
  preview: PluginInstallPreviewResult | null;
  orgPlugin: ClaudeDesktopOrgPluginStatusResult | null;
  marketplace: ClaudeDesktopMarketplaceStatusResult | null;
}) {
  const [filter, setFilter] = useState<"all" | "official" | "ponytail" | "codex" | "mcp" | "skill" | "installed" | "review">("all");
  const [selectedId, setSelectedId] = useState("");
  const items = useMemo(() => hub?.catalog?.items ?? [], [hub]);
  const visible = useMemo(() => items.filter((item) => {
    if (filter === "official") return item.sourceId === "official";
    if (filter === "ponytail") return item.sourceId === "ponytail" || item.tags.includes("ponytail");
    if (filter === "codex") return item.sourceId === "codex-plugins" || item.category === "codex" || item.installKind === "codex_plugin" || item.tags.includes("codex");
    if (filter === "mcp") return item.installKind === "mcp_server" || item.installKind === "claude_desktop_mcp" || item.installKind === "claude_desktop_org_plugin";
    if (filter === "skill") return item.installKind === "skill_bundle" || item.installKind === "managed_skill_bundle";
    if (filter === "installed") return item.installStatus === "installed";
    if (filter === "review") return item.installStatus === "needsReview";
    return true;
  }), [items, filter]);
  const selected = items.find((item) => item.id === selectedId) ?? visible[0] ?? null;
  const selectedPreview = preview?.item.id === selected?.id ? preview : null;
  const selectedCanInstall = selected ? pluginCanInstall(selected.installKind) : false;
  const installButtonLabel = selected ? pluginInstallButtonLabel(selected.installKind) : "安装";
  return (
    <div className="stack">
      <div className="plugin-layout">
      <Panel title="插件目录" detail="Claude 插件、Codex 插件仓库、MCP Registry 与 awesome-claude-code 社区资源。">
        <div className="filter-row">
          {[
            ["all", "全部"],
            ["official", "官方插件"],
            ["codex", "Codex 插件"],
            ["mcp", "MCP"],
            ["skill", "Skills"],
            ["installed", "已安装"],
            ["review", "需审查"],
          ].map(([id, label]) => (
            <button className={filter === id ? "active" : ""} key={id} onClick={() => setFilter(id as typeof filter)} type="button">
              {label}
            </button>
          ))}
          <button className={filter === "ponytail" ? "active" : ""} onClick={() => setFilter("ponytail")} type="button">
            Ponytail
          </button>
          <Button onClick={() => void actions.refreshPluginHub()} size="sm" variant="outline">
            <RefreshCw className="h-4 w-4" />
            刷新
          </Button>
        </div>
        <div className="source-strip">
          {(hub?.catalog?.sources ?? []).map((source) => (
            <div className={`source-pill ${source.status}`} key={source.id}>
              <strong>{source.label}</strong>
              <span>{source.itemCount} 项 · {source.message}</span>
            </div>
          ))}
        </div>
        <div className="plugin-list">
          {visible.length ? visible.slice(0, 220).map((item) => (
            <PluginListItem
              key={item.id}
              item={item}
              isSelected={selected?.id === item.id}
              onSelect={setSelectedId}
            />
          )) : <Empty text="暂无目录数据，点击刷新。" />}
        </div>
      </Panel>
      <Panel title={selected?.name ?? "插件详情"} detail={selected ? selected.sourceLabel : "选择条目后查看安装预览。"}>
        {selected ? (
          <div className="detail-stack">
            <p>{selected.description || "暂无描述。"}</p>
            <div className="info-grid compact">
              <InfoRow label="类型" value={pluginKindLabel(selected.installKind)} />
              <InfoRow label="状态" value={pluginStatusLabel(selected.installStatus)} />
              <InfoRow label="分类" value={selected.category || "-"} />
              <InfoRow label="作者" value={selected.author || "-"} />
              <InfoRow label="许可证" value={selected.license || "-"} />
            </div>
            <div className="tag-row">
              {selected.requirements.map((item) => <span key={item}>{item}</span>)}
              {selected.tags.map((item) => <span key={item}>{item}</span>)}
            </div>
            <div className="risk-box">{selected.risk}</div>
            {selectedPreview ? (
              <div className="preview-box">
                <strong>安装预览</strong>
                <span>{selectedPreview.message}</span>
                {selectedPreview.command.length ? <pre>{selectedPreview.command.join(" ")}</pre> : null}
                {selectedPreview.configDiff ? <pre>{selectedPreview.configDiff}</pre> : null}
              </div>
            ) : null}
            <div className="action-row">
              <Button onClick={() => void actions.previewPlugin(selected.id)} variant="outline">
                <ShieldCheck className="h-4 w-4" />
                预览安装
              </Button>
              {selected.installStatus === "installed" ? (
                <Button onClick={() => void actions.uninstallPlugin(selected.id)} variant="outline">
                  <Trash2 className="h-4 w-4" />
                  卸载
                </Button>
              ) : selectedCanInstall ? (
                <Button onClick={() => void actions.installPlugin(selected.id)}>
                  <Download className="h-4 w-4" />
                  <span className="desktop-install-label">{installButtonLabel}</span>
                </Button>
              ) : (
                <Button disabled variant="outline">
                  <ShieldCheck className="h-4 w-4" />
                  Review required
                </Button>
              )}
              {selected.id === "ponytail:codex-plugin" ? (
                <>
                  <Button onClick={() => void actions.previewPonytailCodexHooks()} variant="outline">
                    <ShieldCheck className="h-4 w-4" />
                    Review hooks
                  </Button>
                  <Button onClick={() => void actions.trustPonytailCodexHooks()} variant="outline">
                    <ShieldCheck className="h-4 w-4" />
                    Trust hooks
                  </Button>
                </>
              ) : null}
              {selected.id === "ponytail:claude-desktop-mcp" ? (
                <Button onClick={() => void actions.generatePonytailMcpbInstaller()} variant="outline">
                  <Download className="h-4 w-4" />
                  Generate MCPB
                </Button>
              ) : null}
              {selected.homepage ? (
                <Button onClick={() => void actions.openExternalUrl(selected.homepage)} variant="outline">
                  <ExternalLink className="h-4 w-4" />
                  来源
                </Button>
              ) : null}
            </div>
          </div>
        ) : <Empty text="还没有选择插件。" />}
      </Panel>
      </div>
    </div>
  );
}

export function MaintenanceToolsPanel({
  actions,
  claudeDesktop,
  overview,
  settings,
}: {
  actions: AppActions;
  claudeDesktop: ClaudeDesktopResult | null;
  overview: OverviewResult | null;
  settings: SettingsResult | null;
}) {
  const savedCodexPath = settings?.settings.codexAppPath?.trim() || "";
  const detectedCodexPath = overview?.codex_app.path || "";
  const detectedClaudePath = claudeDesktop?.executablePaths?.[0] || "";
  return (
    <div className="stack">
      <Panel title="Codex 应用路径" detail="免安装版或绿色版只需要选择一次，之后静默启动会自动复用。">
        <div className="ops-status-list">
          <StatusRow label="保存路径" status={savedCodexPath ? "ok" : "not_checked"} value={savedCodexPath ? compactDisplayPath(savedCodexPath) : "未记录路径"} />
          <StatusRow label="当前识别" status={overview?.codex_app.status ?? "not_checked"} value={compactDisplayPath(detectedCodexPath)} />
        </div>
        <label className="ops-form-field">
          <span>保存的应用路径</span>
          <input readOnly value={savedCodexPath || "选择 Codex.exe、Codex.app、app 目录或绿色目录"} />
        </label>
      </Panel>

      <Panel title="Claude 应用路径" detail="用于核对 Claude Desktop 安装位置和开发模式相关操作。">
        <div className="ops-status-list">
          <StatusRow label="当前识别" status={detectedClaudePath ? "found" : claudeDesktop?.status ?? "not_checked"} value={detectedClaudePath ? compactDisplayPath(detectedClaudePath) : "未检测到 Claude 路径"} />
          <StatusRow label="安装类型" status={claudeDesktop?.installKind ? "ok" : "not_checked"} value={claudeDesktop?.installKind ?? "未检测"} />
          <StatusRow label="CDP 状态" status={claudeDesktop?.cdpStatus === "blocked" || claudeDesktop?.cdpStatus === "failed" ? "failed" : claudeDesktop?.cdpStatus ?? "not_checked"} value={claudeDesktop?.cdpStatus ?? "未检测"} />
        </div>
        <div className="action-row">
          <Button onClick={() => void actions.launchClaudeDesktop()} size="sm" variant="outline">启动/重启Claude</Button>
          <Button onClick={() => void actions.configureClaudeDesktopDevMode()} size="sm" variant="outline">Claude 一键开发模式</Button>
        </div>
      </Panel>
    </div>
  );
}

export function LogsScreen({ actions, logs }: { actions: AppActions; logs: LogsResult | null }) {
  return (
    <Panel title="运行日志" detail={logs?.path ?? "读取最近 240 行诊断日志。"}>
      <div className="action-row">
        <Button onClick={() => void actions.refreshLogs()}>
          <RefreshCw className="h-4 w-4" />
          刷新日志
        </Button>
      </div>
      <pre className="ops-code tall">{logs?.text || "暂无日志。"}</pre>
    </Panel>
  );
}

export const MaintenanceScreen = memo(function MaintenanceScreen({
  actions,
  claudeDesktop,
  overview,
  settings,
}: {
  actions: AppActions;
  claudeDesktop: ClaudeDesktopResult | null;
  overview: OverviewResult | null;
  settings: SettingsResult | null;
}) {
  return (
    <div className="stack">
      <MaintenanceToolsPanel actions={actions} claudeDesktop={claudeDesktop} overview={overview} settings={settings} />
    </div>
  );
});

export const SettingsScreen = memo(function SettingsScreen({
  actions,
  claudeDesktop,
  logs,
  overview,
  updateInfo,
}: {
  actions: AppActions;
  claudeDesktop: ClaudeDesktopResult | null;
  logs: LogsResult | null;
  overview: OverviewResult | null;
  updateInfo: UpdateResult | null;
}) {
  return (
    <div className="stack">
      <AboutScreen actions={actions} claudeDesktop={claudeDesktop} overview={overview} updateInfo={updateInfo} />
      <LogsScreen actions={actions} logs={logs} />
    </div>
  );
});

export const AboutScreen = memo(function AboutScreen({
  actions,
  claudeDesktop,
  overview,
  updateInfo,
}: {
  actions: AppActions;
  claudeDesktop: ClaudeDesktopResult | null;
  overview: OverviewResult | null;
  updateInfo: UpdateResult | null;
}) {
  const release = updateInfoToRelease(updateInfo);
  const updateRunning = updateInfo?.status === "running";
  const progress = Math.max(0, Math.min(100, updateInfo?.progress ?? 0));
  const showDownloadProgress = Boolean(updateInfo?.phase && updateInfo.phase !== "checking");
  const progressLabel = updateProgressLabel(updateInfo?.phase, updateInfo?.progress);
  const browserDownloadUrl = trustedUpdateAssetUrl(updateInfo);
  return (
    <div className="ops-two-column">
      <div className="ops-wide-column">
        <Panel title="关于 CCP" detail="CCP 本地供应商、客户端、会话与维护控制台。">
          <div className="info-grid compact">
            <InfoRow label="Claude Codex Pro 版本" value={overview?.current_version ?? updateInfo?.currentVersion ?? "未加载"} />
            <InfoRow label="Codex 版本" value={overview?.codex_version ?? "未检测"} />
            <InfoRow label="Claude 版本" value={claudeDesktopVersionLabel(claudeDesktop)} />
            <InfoRow label="资源名称" value={displayAssetName(updateInfo?.assetName)} />
            <InfoRow label="项目地址" value="github.com/DamonZS/Claude-Codex-Pro-Tool" />
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.openExternalUrl("https://github.com/DamonZS/Claude-Codex-Pro-Tool")} variant="outline">
              <ExternalLink className="h-4 w-4" />
              打开项目
            </Button>
            <Button onClick={() => void actions.openExternalUrl("https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases")} variant="outline">
              <ExternalLink className="h-4 w-4" />
              Release
            </Button>
          </div>
        </Panel>
        <Panel title="合作请联系微信" detail="扫码联系微信洽谈合作，也可一键加入官方 QQ 群。">
          <div className="contact-card">
            <div className="contact-line">
              <span className="contact-label">官方QQ群：</span>
              <span className="contact-group-number">10061615</span>
              <button className="contact-link" type="button" onClick={() => void actions.openExternalUrl(CONTACT_QQ_GROUP_PRIMARY_URL)}>一键添加</button>
            </div>
            <div className="contact-wechat">
              <div>
                <strong>合作请联系微信</strong>
                <p>扫码添加微信，备注合作代理。</p>
              </div>
              <img className="contact-qr" src={contactWechatQr} alt="合作代理微信二维码" />
            </div>
          </div>
        </Panel>
      </div>
      <div className="stack">
        <Panel title="GitHub Release 更新" detail="调用后端真实检查更新；有安装包时可下载并运行。">
          <div className="ops-status-list">
            <StatusRow label="更新状态" status={updateInfo?.status ?? "not_checked"} value={updateStatusLabel(updateInfo)} />
            <StatusRow label="当前版本" status={overview?.current_version || updateInfo?.currentVersion ? "ok" : "not_checked"} value={updateInfo?.currentVersion ?? overview?.current_version ?? "未加载"} />
            <StatusRow label="最新版本" status={updateInfo?.latestVersion ? "ok" : "not_checked"} value={updateInfo?.latestVersion ?? "未检查"} />
            <StatusRow label="安装资源" status={updateInfo?.assetUrl ? "ok" : "not_checked"} value={displayAssetName(updateInfo?.assetName)} />
          </div>
          {updateInfo?.releaseSummary ? (
            <pre className="ops-code compact">{updateInfo.releaseSummary}</pre>
          ) : (
            <Empty text={updateInfo?.latestVersion ? "已获取最新 Release 版本与安装资源。" : "暂未检查到 Release 信息。"} />
          )}
          {showDownloadProgress ? (
            <div className="update-download-progress" aria-live="polite">
              <div className="update-download-progress-head">
                <strong>{progressLabel}</strong>
                <span>
                  {formatDownloadBytes(updateInfo?.downloadedBytes)}
                  {updateInfo?.totalBytes ? ` / ${formatDownloadBytes(updateInfo.totalBytes)}` : ""}
                </span>
              </div>
              <div
                aria-label="安装包下载进度"
                aria-valuemax={100}
                aria-valuemin={0}
                aria-valuenow={updateInfo?.totalBytes ? progress : undefined}
                className="update-download-progress-track"
                role="progressbar"
              >
                <span
                  className={updateInfo?.totalBytes ? "update-download-progress-fill" : "update-download-progress-fill indeterminate"}
                  style={updateInfo?.totalBytes ? { width: `${progress}%` } : undefined}
                />
              </div>
            </div>
          ) : null}
          {updateInfo?.phase === "failed" ? (
            <Empty text={compactUpdateError(updateInfo.message)} />
          ) : null}
          <div className="action-row">
            <Button disabled={updateRunning} onClick={() => void actions.checkUpdate()}>
              <RefreshCw className={`h-4 w-4${updateInfo?.phase === "checking" ? " animate-spin" : ""}`} />
              {updateInfo?.phase === "checking" ? "检查中" : "检查更新"}
            </Button>
            <Button disabled={updateRunning || !release || !updateInfo?.assetUrl} onClick={() => void actions.performUpdate(release)} variant="outline">
              <Download className={`h-4 w-4${updateRunning && updateInfo?.phase !== "checking" ? " animate-pulse" : ""}`} />
              {updateRunning && updateInfo?.phase !== "checking" ? progressLabel : "下载并运行安装包"}
            </Button>
            {updateInfo?.phase === "failed" && browserDownloadUrl ? (
              <Button onClick={() => void actions.openExternalUrl(browserDownloadUrl)} variant="outline">
                <ExternalLink className="h-4 w-4" />
                用系统浏览器下载
              </Button>
            ) : null}
          </div>
        </Panel>
      </div>
    </div>
  );
});

const MULTICA_STATUS_LABELS: Record<string, string> = {
  unconfigured: "未配置",
  not_checked: "未检查",
  checking: "检查中",
  healthy: "健康",
  degraded: "部分可用",
  unreachable: "无法连接",
  unauthorized: "未授权",
  invalid_response: "响应无效",
  stopped: "已停止",
  stale: "数据可能已过期",
  authenticated: "已登录",
  needs_login: "需要登录",
  unknown: "未知",
};

function multicaStatusLabel(value: unknown) {
  const status = typeof value === "string" && value.trim() ? value.trim().toLowerCase() : "unknown";
  return MULTICA_STATUS_LABELS[status] ?? status;
}

function multicaStatusTone(value: unknown) {
  const status = typeof value === "string" ? value.toLowerCase() : "unknown";
  if (status === "healthy") return "ok";
  if (status === "checking") return "running";
  if (status === "unconfigured" || status === "stopped" || status === "unknown") return "not_checked";
  return "failed";
}

function multicaRuntimeInstallLabel(value: unknown) {
  const state = typeof value === "string" && value.trim() ? value.trim().toLowerCase() : "unknown";
  const labels: Record<string, string> = {
    ready: "已就绪",
    installing: "准备中",
    unavailable: "不可用",
    unsupported_platform: "当前平台不受支持",
    download_failed: "下载失败",
    verification_failed: "校验失败",
    cancelled: "已取消",
    restart_exhausted: "重启次数已耗尽",
    unknown: "未检查",
  };
  return labels[state] ?? state;
}

const MULTICA_INSTALL_PHASE_LABELS: Record<string, string> = {
  preparing: "准备安装",
  checking_bundle: "检查内置资源",
  downloading_archive: "下载运行时",
  downloading_checksums: "下载校验清单",
  verifying: "校验下载内容",
  extracting: "解压运行时",
  staging: "准备安装目录",
  probing: "探测 CLI 能力",
  activating: "激活版本",
  complete: "安装完成",
};

function multicaInstallPhaseLabel(phase: unknown, state: unknown) {
  const normalized = typeof phase === "string" && phase.trim()
    ? phase.trim().toLowerCase()
    : typeof state === "string" && state.trim()
      ? state.trim().toLowerCase()
      : "unknown";
  return MULTICA_INSTALL_PHASE_LABELS[normalized] ?? multicaRuntimeInstallLabel(normalized);
}

function multicaFormatBytes(value: unknown) {
  const bytes = typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : 0;
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  const units = ["KB", "MB", "GB"];
  let scaled = bytes;
  let unit = -1;
  while (scaled >= 1024 && unit < units.length - 1) {
    scaled /= 1024;
    unit += 1;
  }
  return `${scaled >= 100 ? scaled.toFixed(0) : scaled >= 10 ? scaled.toFixed(1) : scaled.toFixed(2)} ${units[unit]}`;
}

function multicaRuntimeInstallTone(value: unknown) {
  const state = typeof value === "string" ? value.trim().toLowerCase() : "unknown";
  if (state === "ready") return "ok";
  if (state === "installing") return "checking";
  if (state === "unknown" || state === "unavailable" || state === "cancelled") return "not_checked";
  return "failed";
}

function multicaTimestamp(value: number | null | undefined) {
  if (!value) return "未检查";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "时间未知" : date.toLocaleString();
}

function multicaRedactUrl(value: string) {
  const text = value.trim();
  if (!text) return "";
  try {
    const parsed = new URL(text);
    // Cards and diagnostics only need the origin.  Keep a path marker so the
    // user can distinguish a root endpoint without exposing request details,
    // query parameters, credentials, or fragments.
    return `${parsed.protocol}//${parsed.host}${parsed.pathname && parsed.pathname !== "/" ? "/..." : ""}`;
  } catch {
    return text
      .replace(/([?&](?:api[_-]?key|token|access[_-]?token|refresh[_-]?token|authorization)=)[^&#\s]*/gi, "$1[redacted]")
      .replace(/(https?:\/\/)([^/\s?#]+)(?:[^\s"'<>]*)?/gi, "$1$2/...");
  }
}

function multicaDisplayAddress(value: unknown) {
  // Keep static status rows compact and redact credentials, paths, queries,
  // and fragments. The managed editor itself deliberately binds the saved
  // value directly so it can preserve an explicit empty string byte-for-byte.
  return typeof value === "string" && value.trim() ? multicaRedactUrl(value) : "地址未配置";
}

function multicaSafeText(value: unknown, limit = 220) {
  if (typeof value !== "string" || !value.trim()) return "";
  const text = value
    .trim()
    // Never surface complete request URLs in diagnostics or list details.
    .replace(/https?:\/\/[^\s"'<>]+/gi, (match) => multicaRedactUrl(match))
    // Header-style credentials can contain spaces (especially Cookie), so
    // handle those before the generic key/value matcher.
    .replace(/(\bcookie\s*[:=]\s*)[^\r\n]*/gi, "$1[redacted]")
    .replace(/\bBearer\s+[^\s,;]+/gi, "Bearer [redacted]")
    .replace(/((?:["']?)(?:authorization|api[_-]?key|access[_-]?token|refresh[_-]?token|token|secret|password)(?:["']?)\s*[:=]\s*)(?:"[^"]*"|'[^']*'|[^\s,;}]*)/gi, "$1[redacted]")
    .replace(/([?&](?:api[_-]?key|token|access[_-]?token|refresh[_-]?token|authorization)=)[^&#\s]*/gi, "$1[redacted]")
    // Windows drive paths, UNC paths, and common Unix absolute paths are
    // local implementation details and are not useful in the UI diagnosis.
    .replace(/(?:[A-Za-z]:[\\/]|\\\\|\/(?:Users|home|tmp|var|opt|workspace|workspaces|mnt|private|etc|root)(?:\/|$))[^\s"'<>;,]*/gi, "[path]");
  return text.length <= limit ? text : `${text.slice(0, limit)}...`;
}

function multicaNewConnectionDraft(): MulticaConnectionConfig {
  return {
    connectionId: null,
    displayName: "",
    serverUrl: "",
    apiPrefix: "",
    workspaceId: null,
    workspaceSlug: null,
    tokenEnvVar: undefined,
    enabled: true,
    allowInsecureLanHttp: false,
    sidecar: null,
  };
}

function multicaDraftFromView(connection: MulticaConnectionView): MulticaConnectionConfig {
  return {
    connectionId: connection.connectionId,
    displayName: connection.displayName,
    // Connection views deliberately omit the original URL.  An empty field
    // means "preserve the saved address" for an existing connection, while a
    // newly entered value explicitly replaces it on save.
    serverUrl: "",
    apiPrefix: connection.apiPrefix ?? "",
    workspaceId: connection.workspaceId ?? "",
    workspaceSlug: connection.workspaceSlug ?? "",
    // The backend intentionally does not return the token environment name.
    tokenEnvVar: undefined,
    enabled: connection.enabled,
    allowInsecureLanHttp: connection.allowInsecureLanHttp,
    // The backend intentionally returns only a sidecar summary. Omit this
    // field when a sidecar exists so the save command preserves its verified
    // executable path, working directory, and arguments. An explicit null is
    // used only when the user turns the sidecar configuration off.
    sidecar: connection.sidecarConfigured ? undefined : null,
  };
}

export type MulticaRuntimeScreenProps = {
  actions: AppActions;
  connections?: MulticaConnectionView[] | null;
  connectionsResult?: MulticaConnectionsResult | null;
  statuses?: Record<string, MulticaConnectionStatus> | null;
  status?: MulticaConnectionStatus | null;
  statusResult?: MulticaConnectionStatusResult | null;
  snapshot?: MulticaRuntimeSnapshot | null;
  snapshotResult?: MulticaSnapshotResult | null;
  sidecars?: Record<string, MulticaSidecarStatus> | null;
  loading?: boolean;
  error?: string | null;
  managedRuntime?: MulticaManagedRuntimePayload | null;
  managedRuntimeResult?: MulticaManagedRuntimeResult | null;
  managedRuntimeLoading?: boolean;
  managedRuntimeError?: string | null;
};

/**
 * Read-only Multica control-plane UI. Task mutation is intentionally absent;
 * this page only manages isolated connection records and sidecar lifecycle.
 */
export function MulticaRuntimeScreen({
  actions,
  connections,
  connectionsResult,
  statuses,
  status,
  statusResult,
  snapshot,
  snapshotResult,
  sidecars,
  loading = false,
  error,
  managedRuntime,
  managedRuntimeResult,
  managedRuntimeLoading = false,
  managedRuntimeError,
}: MulticaRuntimeScreenProps) {
  const connectionList = connections ?? connectionsResult?.connections ?? [];
  const hasConnectionData = connections !== undefined || Boolean(connectionsResult);
  const connectionLoadFailed = Boolean(connectionsResult && !statusOk(connectionsResult.status)) || Boolean(error && !hasConnectionData);
  const connectionListLoading = loading && !hasConnectionData;
  const knownStatuses = statuses ?? connectionsResult?.statuses ?? snapshotResult?.statuses ?? null;
  const [selectedId, setSelectedId] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState<MulticaConnectionConfig>(multicaNewConnectionDraft);
  const [tokenEnvVarDraft, setTokenEnvVarDraft] = useState("");
  const [clearToken, setClearToken] = useState(false);
  const [tab, setTab] = useState<"runtimes" | "agents" | "tasks">("runtimes");
  const [pending, setPending] = useState<string | null>(null);
  const [managedPending, setManagedPending] = useState<string | null>(null);
  const [managedEditing, setManagedEditing] = useState(false);
  const [managedDraft, setManagedDraft] = useState<MulticaManagedConnectionUpdate>({
    displayName: "",
    serverUrl: "",
    enabled: true,
  });

  // An empty selectedId normally means "use the first connection", except
  // while the explicit new-connection editor is open. Keeping that state
  // distinct prevents an existing token/sidecar from leaking into a new draft.
  const selected = editingId === ""
    ? null
    : selectedId
      ? connectionList.find((connection) => connection.connectionId === selectedId) ?? null
      : connectionList[0] ?? null;
  const selectedStatus = selected && selected.connectionId === status?.connectionId
    ? status
    : selected && selected.connectionId === statusResult?.connectionId
      ? statusResult
      : selected
        ? knownStatuses?.[selected.connectionId] ?? null
        : null;
  const receivedSnapshot = snapshot ?? snapshotResult?.snapshot ?? null;
  const currentSnapshot = receivedSnapshot && (!selected || receivedSnapshot.sourceConnectionId === selected.connectionId)
    ? receivedSnapshot
    : null;
  const isEditing = editingId !== null;
  const isNew = editingId === "";
  const sidecarEnabled = draft.sidecar !== null
    && (Boolean(draft.sidecar) || Boolean(selected?.sidecarConfigured));

  useEffect(() => {
    if (selectedId && connectionList.some((connection) => connection.connectionId === selectedId)) return;
    setSelectedId(connectionList[0]?.connectionId ?? "");
  }, [connectionList, selectedId]);

  useEffect(() => {
    if (editingId !== null) return;
    if (!selected) {
      setDraft(multicaNewConnectionDraft());
      return;
    }
    setDraft(multicaDraftFromView(selected));
    setTokenEnvVarDraft("");
    setClearToken(false);
  }, [editingId, selected]);

  const beginNewConnection = () => {
    setSelectedId("");
    setEditingId("");
    setDraft(multicaNewConnectionDraft());
    setTokenEnvVarDraft("");
    setClearToken(false);
  };

  const beginEditConnection = (connection: MulticaConnectionView) => {
    setSelectedId(connection.connectionId);
    setEditingId(connection.connectionId);
    setDraft(multicaDraftFromView(connection));
    setTokenEnvVarDraft("");
    setClearToken(false);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setTokenEnvVarDraft("");
    setClearToken(false);
    if (selected) setDraft(multicaDraftFromView(selected));
  };

  const updateDraft = (patch: Partial<MulticaConnectionConfig>) => {
    setDraft((current) => ({ ...current, ...patch }));
  };

  const updateSidecar = (patch: Partial<MulticaSidecarConfig>) => {
    setDraft((current) => ({
      ...current,
      sidecar: current.sidecar ? { ...current.sidecar, ...patch } : { executable: "", workingDir: "", args: [], autoStart: false, ...patch },
    }));
  };

  const toggleSidecarConfig = (enabled: boolean) => {
    if (!enabled) {
      if (!window.confirm("停用该工作流旁路服务？只影响此连接，不会停止或修改 CCP 供应商、代理、Codex 或 Claude。")) return;
      updateDraft({ sidecar: null });
      return;
    }
    updateSidecar({});
  };

  const toggleConnectionEnabled = (enabled: boolean) => {
    if (!enabled && draft.enabled) {
      const confirmed = window.confirm(
        "确认停用该工作流连接？只影响此工作流连接，不会停止或修改 CCP 供应商、代理、Codex 或 Claude。",
      );
      if (!confirmed) return;
    }
    updateDraft({ enabled });
  };

  const toggleInsecureLanHttp = (enabled: boolean) => {
    if (enabled && !draft.allowInsecureLanHttp) {
      const confirmed = window.confirm(
        "确认允许此工作流连接使用非加密局域网 HTTP？仅限已确认的私有局域网地址，传输可能被同一网络中的其他设备读取。不会影响 CCP 供应商、代理、Codex 或 Claude。",
      );
      if (!confirmed) return;
    }
    updateDraft({ allowInsecureLanHttp: enabled });
  };

  const saveConnection = async () => {
    // Existing connections may be saved with an empty URL: the backend keeps
    // the persisted original address, which was intentionally never sent to
    // this renderer. A new connection still requires an address.
    if ((isNew && !draft.serverUrl.trim()) || pending) return;
    setPending("save");
    try {
      const tokenEnvVar = tokenEnvVarDraft.trim();
      const payload: MulticaConnectionConfig = {
        ...draft,
        // Preserve a newly entered URL exactly. For an existing connection an
        // empty string is a deliberate "keep existing URL" sentinel handled
        // by the command boundary, not a URL normalization request.
        tokenEnvVar: tokenEnvVar ? tokenEnvVar : clearToken ? null : undefined,
        apiPrefix: draft.apiPrefix || null,
        allowInsecureLanHttp: Boolean(draft.allowInsecureLanHttp),
      };
      const result = await actions.saveMulticaConnection(payload);
      // Keep the editor open after a rejected save so the user can correct
      // the input and retry; the action already reports the failure.
      if (!result || !statusOk(result.status)) return;
      setEditingId(null);
      setClearToken(false);
      setTokenEnvVarDraft("");
      await actions.listMulticaConnections();
    } finally {
      setPending(null);
    }
  };

  const deleteConnection = async (connection: MulticaConnectionView) => {
    if (pending) return;
    const daemonStatus = selectedStatus?.daemon.status;
    if (daemonStatus === "healthy" || daemonStatus === "checking") {
      actions.showNotice({ title: "无法删除连接", message: "请先停止该工作流旁路服务；删除只影响工作流连接。", status: "failed" });
      return;
    }
    if (!window.confirm(`确认删除工作流连接“${connection.displayName || connection.connectionId}”？不会影响 CCP 供应商、代理、Codex 或 Claude。`)) return;
    setPending("delete");
    try {
      const result = await actions.deleteMulticaConnection(connection.connectionId);
      if (!result || !statusOk(result.status)) return;
      setEditingId(null);
      setSelectedId("");
      await actions.listMulticaConnections();
    } finally {
      setPending(null);
    }
  };

  const checkConnection = async () => {
    if (!selected || pending) return;
    setPending("check");
    try {
      await actions.checkMulticaConnection(selected.connectionId);
    } finally {
      setPending(null);
    }
  };

  const refreshSnapshot = async () => {
    if (!selected || pending) return;
    setPending("snapshot");
    try {
      await actions.getMulticaSnapshot(selected.connectionId);
    } finally {
      setPending(null);
    }
  };

  const sidecarAction = async (kind: "start" | "stop" | "restart") => {
    if (!selected || pending) return;
    const confirmation: Record<typeof kind, string> = {
      start: "确认启动该工作流旁路服务？只会启动此连接已保存的旁路服务，不会启动或修改 CCP 供应商、代理、Codex 或 Claude。",
      stop: "确认停止该工作流旁路服务？只影响该旁路服务，不会停止或修改 CCP 供应商、代理、Codex 或 Claude。",
      restart: "确认重启该工作流旁路服务？只影响该旁路服务，不会停止或修改 CCP 供应商、代理、Codex 或 Claude。",
    };
    if (!window.confirm(confirmation[kind])) return;
    setPending(`sidecar:${kind}`);
    try {
      if (kind === "start") await actions.startMulticaSidecar(selected.connectionId);
      if (kind === "stop") await actions.stopMulticaSidecar(selected.connectionId);
      if (kind === "restart") await actions.restartMulticaSidecar(selected.connectionId);
    } finally {
      setPending(null);
    }
  };

  const copyStableId = async (id: string) => {
    try {
      if (!navigator.clipboard?.writeText) throw new Error("clipboard_unavailable");
      await navigator.clipboard.writeText(id);
      actions.showNotice({ title: "已复制稳定 ID", message: id, status: "ok" });
    } catch {
      actions.showNotice({ title: "复制失败", message: "系统剪贴板不可用，请手动选择稳定 ID。", status: "failed" });
    }
  };

  const items: MulticaRuntimeItem[] = tab === "runtimes"
    ? currentSnapshot?.runtimes ?? []
    : tab === "agents"
      ? currentSnapshot?.agents ?? []
      : currentSnapshot?.tasks ?? [];
  const snapshotDiagnostic = multicaSafeText(currentSnapshot?.diagnostic || currentSnapshot?.error || error);
  const serverStatus = selectedStatus?.server;
  // Daemon status is keyed by connection ID. Never fall back to a global
  // status because doing so can display another connection's PID/state after
  // the user switches rows.
  const daemonStatus = selected
    ? selectedStatus?.daemon ?? sidecars?.[selected.connectionId] ?? null
    : null;
  const sidecarConfigured = draft.sidecar === null
    ? false
    : Boolean(selected?.sidecarConfigured || draft.sidecar);
  const daemonState = typeof daemonStatus?.status === "string" && daemonStatus.status.trim()
    ? daemonStatus.status.trim().toLowerCase()
    : sidecarConfigured ? "stopped" : "unconfigured";
  const daemonRunning = daemonState === "healthy"
    || daemonState === "checking"
    || (daemonState === "degraded" && Boolean(daemonStatus?.pid));
  const snapshotIsStale = Boolean(currentSnapshot?.stale || selectedStatus?.stale);
  const checkedAtMs = serverStatus?.checkedAtMs ?? daemonStatus?.checkedAtMs;

  const refreshConnections = async () => {
    if (pending) return;
    setPending("list");
    try {
      await actions.listMulticaConnections();
    } finally {
      setPending(null);
    }
  };

  const managedPayload = managedRuntime ?? (managedRuntimeResult && statusOk(managedRuntimeResult.status)
    ? {
      runtime: managedRuntimeResult.runtime,
      connection: managedRuntimeResult.connection ?? null,
      connectionStatus: managedRuntimeResult.connectionStatus ?? null,
      loginStatus: managedRuntimeResult.loginStatus || "unknown",
    }
    : null);
  const managedInstall = managedPayload?.runtime;
  const managedConnection = managedPayload?.connection ?? null;
  const managedConnectionStatus = managedPayload?.connectionStatus ?? null;
  const managedDaemon = managedConnectionStatus?.daemon ?? null;
  const managedServer = managedConnectionStatus?.server ?? null;
  const managedInstallState = managedInstall?.installState ?? "unknown";
  const managedInstalling = managedInstallState.toLowerCase() === "installing";
  const managedDaemonState = managedDaemon?.status?.toLowerCase() ?? "unconfigured";
  const managedDaemonRunning = managedDaemonState === "healthy"
    || managedDaemonState === "checking"
    || (managedDaemonState === "degraded" && Boolean(managedDaemon?.pid));
  const managedBusy = managedRuntimeLoading || Boolean(managedPending);
  const managedCanOperate = Boolean(managedConnection && managedInstallState.toLowerCase() === "ready");
  const managedError = multicaSafeText(managedRuntimeError || (!statusOk(managedRuntimeResult?.status) ? managedRuntimeResult?.message : ""));

  // Poll only while an ensure request is active. The App action accepts an
  // internal preserve-generation flag; keep the cast local so the public
  // action shape remains backwards compatible while status reads cannot make
  // the in-flight ensure response stale.
  const managedProgressPollRef = useRef(false);
  useEffect(() => {
    if (managedPending !== "ensure") return;
    let disposed = false;
    const poll = async () => {
      if (disposed || managedProgressPollRef.current) return;
      managedProgressPollRef.current = true;
      try {
        const getStatus = actions.getMulticaManagedRuntime as unknown as (
          silent?: boolean,
          preserveRequest?: boolean,
        ) => Promise<MulticaManagedRuntimeResult | null>;
        await getStatus(true, true);
      } finally {
        managedProgressPollRef.current = false;
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 700);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [actions, managedPending]);

  // Keep a user's in-progress edit stable across periodic Runtime refreshes.
  // When the editor is closed, sync the next persisted managed-only view.
  useEffect(() => {
    if (managedEditing) return;
    setManagedDraft({
      displayName: managedConnection?.displayName ?? "",
      serverUrl: managedConnection?.serverUrl ?? "",
      enabled: managedConnection?.enabled ?? true,
    });
  }, [
    managedConnection?.connectionId,
    managedConnection?.displayName,
    managedConnection?.enabled,
    managedConnection?.serverUrl,
    managedEditing,
  ]);

  const runManagedAction = async (
    action: "refresh" | "ensure" | "cancel" | "rollback" | "login" | "logout" | "enable" | "start" | "stop" | "restart" | "check",
  ) => {
    if ((managedPending && !(action === "cancel" && managedPending === "ensure")) || (managedRuntimeLoading && action !== "cancel")) return;
    const confirmations: Partial<Record<typeof action, string>> = {
      ensure: "确认准备托管工作流运行时？将只下载、校验并安装固定版本的工作流 CLI，不会修改供应商、代理、Codex 或 Claude。",
      rollback: "确认回滚托管工作流运行时？只会切换已验证的托管版本，不会修改供应商、代理、Codex 或 Claude。",
      stop: "确认停止托管工作流守护进程？只影响内置工作流运行时，不会停止或修改供应商、代理、Codex 或 Claude。",
      restart: "确认重启托管工作流守护进程？只影响内置工作流运行时，不会停止或修改供应商、代理、Codex 或 Claude。",
    };
    if (confirmations[action] && !window.confirm(confirmations[action]!)) return;
    setManagedPending(action);
    try {
      if (action === "refresh") await actions.getMulticaManagedRuntime();
      if (action === "ensure") await actions.ensureMulticaRuntime();
      if (action === "cancel") await actions.cancelMulticaRuntimeInstall();
      if (action === "rollback") await actions.rollbackMulticaRuntime();
      if (action === "login") await actions.loginMulticaManaged();
      if (action === "logout") await actions.logoutMulticaManaged();
      if (action === "enable") await actions.setMulticaManagedEnabled(!Boolean(managedConnection?.enabled));
      if (action === "start") await actions.startMulticaManagedRuntime();
      if (action === "stop") await actions.stopMulticaManagedRuntime();
      if (action === "restart") await actions.restartMulticaManagedRuntime();
      if (action === "check") await actions.checkMulticaManagedRuntime();
    } finally {
      setManagedPending(null);
    }
  };

  const beginManagedConnectionEdit = () => {
    setManagedDraft({
      displayName: managedConnection?.displayName ?? "",
      serverUrl: managedConnection?.serverUrl ?? "",
      enabled: managedConnection?.enabled ?? true,
    });
    setManagedEditing(true);
  };

  const cancelManagedConnectionEdit = () => {
    setManagedEditing(false);
  };

  const saveManagedConnection = async () => {
    if (managedBusy) return;
    setManagedPending("save");
    try {
      const result = await actions.saveMulticaManagedConnection({
        displayName: managedDraft.displayName,
        serverUrl: managedDraft.serverUrl,
        enabled: managedDraft.enabled,
      });
      if (result && statusOk(result.status)) setManagedEditing(false);
    } finally {
      setManagedPending(null);
    }
  };

  return (
    <div className="multica-runtime-screen">
      <div className="ops-page-heading">
        <div>
          <h1>工作流运行时</h1>
          <p>外部工作流控制平面连接、健康检查和只读运行时快照；不影响供应商、代理、Codex、Claude。</p>
        </div>
        <div className="action-row">
          <Button disabled={Boolean(pending)} onClick={() => void refreshConnections()} variant="outline">
            <RefreshCw className={`h-4 w-4${pending === "list" ? " spin" : ""}`} />
            {pending === "list" ? "刷新中" : "刷新连接"}
          </Button>
          <Button disabled={Boolean(pending)} onClick={beginNewConnection}>
            <Plus className="h-4 w-4" />
            新增连接
          </Button>
        </div>
      </div>

      <Panel title="托管工作流运行时" detail="内置 CLI、独立 profile 与守护进程监管。所有操作只影响托管运行时，不影响供应商、代理、Codex 或 Claude。">
        <div className="ops-status-list">
          <StatusRow label="安装状态" status={multicaRuntimeInstallTone(managedInstallState)} value={multicaRuntimeInstallLabel(managedInstallState)} />
          <StatusRow label="固定版本" status={managedInstall?.installedVersion ? "ok" : "not_checked"} value={managedInstall?.installedVersion ?? "尚未安装"} />
          <StatusRow label="目标平台" status={managedInstall?.targetTriple ? "ok" : "not_checked"} value={managedInstall?.targetTriple ?? "未识别"} />
          <StatusRow label="安装来源" status={managedInstall?.assetSource ? "ok" : "not_checked"} value={managedInstall?.assetSource ?? "未使用资源"} />
          <StatusRow label="SHA-256 校验" status={managedInstall?.sha256Verified ? "ok" : "not_checked"} value={managedInstall?.sha256Verified ? "已验证" : "尚未验证"} />
          <StatusRow label="连接" status={managedConnection?.enabled ? "ok" : "unconfigured"} value={managedConnection ? managedConnection.displayName || "（未命名）" : "托管连接未创建"} />
          <StatusRow label="Server 地址" status={managedConnection?.serverUrl ? "ok" : "not_checked"} value={multicaDisplayAddress(managedConnection?.serverUrl)} />
          <StatusRow label="Profile" status={managedConnection ? "ok" : "unconfigured"} value={managedConnection?.profile || "ccp-managed（独立）"} />
          <StatusRow label="Daemon" status={managedDaemon?.status ?? "unconfigured"} value={multicaStatusLabel(managedDaemon?.status ?? "unconfigured")} />
          <StatusRow label="登录状态" status={managedPayload?.loginStatus === "authenticated" ? "ok" : managedPayload?.loginStatus === "needs_login" ? "unauthorized" : "not_checked"} value={multicaStatusLabel(managedPayload?.loginStatus ?? "unconfigured")} />
          <StatusRow label="最近检查" status={managedServer?.checkedAtMs || managedDaemon?.checkedAtMs || managedInstall?.updatedAtMs ? "ok" : "not_checked"} value={multicaTimestamp(managedServer?.checkedAtMs ?? managedDaemon?.checkedAtMs ?? managedInstall?.updatedAtMs)} />
        </div>
        {managedInstall && (managedInstalling || managedInstall.progressPercent != null || managedInstall.installPhase) ? (() => {
          const rawPercent = managedInstall.progressPercent;
          const progressPercent = typeof rawPercent === "number" && Number.isFinite(rawPercent)
            ? Math.max(0, Math.min(100, Math.round(rawPercent)))
            : undefined;
          const downloadedBytes = managedInstall.downloadedBytes ?? 0;
          const totalBytes = managedInstall.totalBytes;
          const transfer = totalBytes && totalBytes > 0
            ? `${multicaFormatBytes(downloadedBytes)} / ${multicaFormatBytes(totalBytes)}`
            : downloadedBytes > 0 ? multicaFormatBytes(downloadedBytes) : "等待下载数据";
          return (
            <div className="multica-install-progress" aria-live="polite">
              <div className="multica-install-progress-heading">
                <span>准备进度</span>
                <strong>{multicaInstallPhaseLabel(managedInstall.installPhase, managedInstallState)}</strong>
                {progressPercent != null ? <span>{progressPercent}%</span> : <span>进行中</span>}
              </div>
              <progress
                aria-label={`托管工作流安装进度：${multicaInstallPhaseLabel(managedInstall.installPhase, managedInstallState)}`}
                className="multica-install-progress-bar"
                max={100}
                value={progressPercent}
              />
              <div className="multica-install-progress-meta">
                <span>{transfer}</span>
                {managedInstall.installedVersion ? <span>固定版本 {multicaSafeText(managedInstall.installedVersion)}</span> : null}
              </div>
            </div>
          );
        })() : null}
        {managedDaemon?.pid ? <p className="multica-muted">托管 daemon PID {managedDaemon.pid} · 启动于 {multicaTimestamp(managedDaemon.startedAtMs)}</p> : null}
        {managedInstall?.diagnostic || managedInstall?.lastInstallErrorCode || managedServer?.diagnostic || managedDaemon?.diagnostic || managedError ? (
          <p className="multica-diagnostic" role="status">{multicaSafeText(managedError || managedInstall?.diagnostic || managedInstall?.lastInstallErrorCode || managedServer?.diagnostic || managedDaemon?.diagnostic)}</p>
        ) : null}
        {managedEditing ? (
          <form
            className="multica-form-grid"
            onSubmit={(event) => {
              event.preventDefault();
              void saveManagedConnection();
            }}
          >
            <label className="ops-form-field">
              <span>显示名称</span>
              <input
                autoComplete="off"
                disabled={managedBusy}
                onChange={(event) => setManagedDraft((current) => ({ ...current, displayName: event.currentTarget.value }))}
                value={managedDraft.displayName}
              />
            </label>
            <label className="ops-form-field">
              <span>Server URL</span>
              <input
                autoComplete="url"
                disabled={managedBusy}
                onChange={(event) => setManagedDraft((current) => ({ ...current, serverUrl: event.currentTarget.value }))}
                value={managedDraft.serverUrl}
              />
            </label>
            <div className="ops-toggle-line">
              <span>启用托管 Runtime</span>
              <ToggleSwitch
                checked={managedDraft.enabled}
                disabled={managedBusy}
                onChange={(enabled) => setManagedDraft((current) => ({ ...current, enabled }))}
              />
            </div>
            <div className="action-row">
              <Button disabled={managedBusy} type="submit">
                <Save className="h-4 w-4" />
                {managedPending === "save" ? "保存中" : "保存连接"}
              </Button>
              <Button disabled={managedBusy} onClick={cancelManagedConnectionEdit} type="button" variant="outline">取消</Button>
            </div>
          </form>
        ) : (
          <div className="action-row">
            <Button disabled={managedBusy} onClick={beginManagedConnectionEdit} type="button" variant="outline">
              <Pencil className="h-4 w-4" />
              编辑连接
            </Button>
          </div>
        )}
        <div className="action-row">
          <Button disabled={managedBusy && !managedInstalling} onClick={() => void runManagedAction("ensure")}>
            <Download className={`h-4 w-4${managedPending === "ensure" ? " spin" : ""}`} />
            {managedPending === "ensure" ? "准备中" : managedInstallState.toLowerCase() === "ready" ? "重新检查" : "准备/重试下载"}
          </Button>
          <Button disabled={!(managedInstalling || managedPending === "ensure") || Boolean(managedPending && managedPending !== "ensure")} onClick={() => void runManagedAction("cancel")} variant="outline">
            <X className="h-4 w-4" />
            {managedPending === "cancel" ? "取消中" : "取消准备"}
          </Button>
          <Button disabled={managedBusy || !managedInstall?.previousVersion} onClick={() => void runManagedAction("rollback")} variant="outline">
            <ArchiveRestore className="h-4 w-4" />
            {managedPending === "rollback" ? "回滚中" : "回滚"}
          </Button>
          <Button disabled={managedBusy} onClick={() => void runManagedAction("refresh")} variant="outline">
            <RefreshCw className={`h-4 w-4${managedPending === "refresh" ? " spin" : ""}`} />
            {managedPending === "refresh" ? "刷新中" : "刷新状态"}
          </Button>
        </div>
        <div className="action-row">
          <Button disabled={managedBusy || !managedCanOperate || managedPayload?.loginStatus === "authenticated"} onClick={() => void runManagedAction("login")} variant="outline">
            <KeyRound className="h-4 w-4" />
            {managedPending === "login" ? "登录中" : "登录"}
          </Button>
          <Button disabled={managedBusy || !managedCanOperate || managedPayload?.loginStatus !== "authenticated"} onClick={() => void runManagedAction("logout")} variant="outline">
            <KeyRound className="h-4 w-4" />
            {managedPending === "logout" ? "退出中" : "退出登录"}
          </Button>
          <Button disabled={managedBusy || !managedConnection || managedEditing} onClick={() => void runManagedAction("enable")} variant="outline">
            <Power className="h-4 w-4" />
            {managedPending === "enable" ? "保存中" : managedConnection?.enabled ? "停用托管 Runtime" : "启用托管 Runtime"}
          </Button>
          <Button disabled={managedBusy || !managedCanOperate} onClick={() => void runManagedAction("check")} variant="outline">
            <Activity className={`h-4 w-4${managedPending === "check" ? " spin" : ""}`} />
            {managedPending === "check" ? "检查中" : "测试连接"}
          </Button>
          <Button disabled={managedBusy || !managedCanOperate || !managedConnection?.enabled || !managedConnection.sidecarConfigured || managedDaemonRunning} onClick={() => void runManagedAction("start")} variant="outline">
            <Play className="h-4 w-4" />
            {managedPending === "start" ? "启动中" : "启动"}
          </Button>
          <Button disabled={managedBusy || !managedCanOperate || !managedDaemonRunning} onClick={() => void runManagedAction("stop")} variant="outline">
            <Power className="h-4 w-4" />
            {managedPending === "stop" ? "停止中" : "停止"}
          </Button>
          <Button disabled={managedBusy || !managedCanOperate || !managedConnection?.enabled || !managedConnection.sidecarConfigured} onClick={() => void runManagedAction("restart")} variant="outline">
            <RefreshCw className="h-4 w-4" />
            {managedPending === "restart" ? "重启中" : "重启"}
          </Button>
        </div>
      </Panel>

      <div className="multica-runtime-layout">
        <div className="stack">
          <Panel title="连接" detail="地址按原值保存，不经过 CCP 供应商或本地代理 URL 改写。">
            <div className="multica-connection-list">
              {connectionList.length ? connectionList.map((connection) => {
                const connectionStatus = knownStatuses?.[connection.connectionId];
                const isSelected = selected?.connectionId === connection.connectionId;
                return (
                  <div
                    className={`multica-connection-row-wrap${isSelected ? " active" : ""}`}
                    key={connection.connectionId}
                  >
                    <button
                      aria-pressed={isSelected}
                      className={`multica-connection-row${isSelected ? " active" : ""}`}
                      onClick={() => {
                        setSelectedId(connection.connectionId);
                        setEditingId(null);
                      }}
                      type="button"
                    >
                      <span className="multica-connection-mark"><Server aria-hidden="true" /></span>
                      <span className="multica-connection-copy">
                        <strong>{connection.displayName || connection.connectionId}</strong>
                        <small title={multicaDisplayAddress(connection.serverUrlDisplay)}>{multicaDisplayAddress(connection.serverUrlDisplay)}</small>
                      </span>
                      <span className={`multica-status-chip ${multicaStatusTone(connectionStatus?.server.status)}`}>
                        {multicaStatusLabel(connectionStatus?.server.status ?? (connection.enabled ? "unknown" : "unconfigured"))}
                      </span>
                    </button>
                    <button
                      aria-label={`编辑 ${connection.displayName || connection.connectionId}`}
                      className="multica-connection-edit"
                      disabled={Boolean(pending)}
                      onClick={(event) => {
                        event.stopPropagation();
                        beginEditConnection(connection);
                      }}
                      title="编辑连接"
                      type="button"
                    >
                      <Pencil aria-hidden="true" className="h-4 w-4" />
                    </button>
                  </div>
                );
              }) : <Empty text={connectionListLoading
                ? "正在加载工作流连接..."
                : connectionLoadFailed
                  ? "加载工作流连接失败，请点击“刷新连接”重试。"
                  : "尚未配置工作流连接。"} />}
            </div>
            {error ? <p className="multica-error" role="alert">{multicaSafeText(error)}</p> : null}
          </Panel>

          {isEditing ? (
            <Panel title={isNew ? "新增连接" : "编辑连接"} detail="令牌仅引用受保护的环境变量名，页面不会回填或显示令牌原文。">
              <div className="multica-form-grid">
                <label className="ops-form-field">
                  <span>显示名称</span>
                  <input autoComplete="off" disabled={Boolean(pending)} onChange={(event) => updateDraft({ displayName: event.currentTarget.value })} value={draft.displayName} />
                </label>
                <label className="ops-form-field">
                  <span>{isNew ? "服务地址" : "服务地址（留空保持原值）"}</span>
                  <input autoComplete="url" disabled={Boolean(pending)} onChange={(event) => updateDraft({ serverUrl: event.currentTarget.value })} placeholder={isNew ? "https://workflow.example" : "留空保持已保存地址；输入新地址则替换"} value={draft.serverUrl} />
                </label>
                <label className="ops-form-field">
                  <span>API 前缀</span>
                  <input autoComplete="off" disabled={Boolean(pending)} onChange={(event) => updateDraft({ apiPrefix: event.currentTarget.value })} placeholder="可选，例如 v1" value={draft.apiPrefix ?? ""} />
                </label>
                <label className="ops-form-field">
                  <span>Workspace ID</span>
                  <input autoComplete="off" disabled={Boolean(pending)} onChange={(event) => updateDraft({ workspaceId: event.currentTarget.value })} placeholder="可选" value={draft.workspaceId ?? ""} />
                </label>
                <label className="ops-form-field">
                  <span>Workspace Slug</span>
                  <input autoComplete="off" disabled={Boolean(pending)} onChange={(event) => updateDraft({ workspaceSlug: event.currentTarget.value })} placeholder="可选" value={draft.workspaceSlug ?? ""} />
                </label>
                <label className="ops-form-field">
                  <span>令牌环境变量名</span>
                  <input autoComplete="off" disabled={Boolean(pending)} onChange={(event) => { setTokenEnvVarDraft(event.currentTarget.value); setClearToken(false); }} placeholder={selected?.tokenConfigured ? "已配置，输入新变量名覆盖" : "可选，例如 WORKFLOW_TOKEN"} value={tokenEnvVarDraft} />
                </label>
              </div>
              <label className="multica-checkbox-line">
                <input
                  checked={Boolean(draft.allowInsecureLanHttp)}
                  disabled={Boolean(pending)}
                  onChange={(event) => toggleInsecureLanHttp(event.currentTarget.checked)}
                  type="checkbox"
                />
                <span>允许已明确确认的局域网 HTTP（非加密）</span>
              </label>
              {draft.allowInsecureLanHttp ? (
                <p className="multica-diagnostic" role="alert">
                  非加密 HTTP 仅接受本机或私有局域网地址；请求内容可能被同一网络中的设备读取。服务地址不得包含用户名、密码、查询参数或片段。
                </p>
              ) : null}
              {selected?.tokenConfigured ? (
                <label className="multica-checkbox-line">
                  <input checked={clearToken} disabled={Boolean(pending)} onChange={(event) => setClearToken(event.currentTarget.checked)} type="checkbox" />
                  <span>清除已保存令牌引用</span>
                </label>
              ) : null}
              <label className="multica-checkbox-line">
                <input checked={draft.enabled} disabled={Boolean(pending)} onChange={(event) => toggleConnectionEnabled(event.currentTarget.checked)} type="checkbox" />
                <span>启用此连接</span>
              </label>

              <div className="multica-sidecar-editor">
                <div className="multica-subsection-heading"><div><Power aria-hidden="true" /><strong>独立 sidecar 监管</strong></div><label className="multica-checkbox-line"><input checked={sidecarEnabled} disabled={Boolean(pending)} onChange={(event) => toggleSidecarConfig(event.currentTarget.checked)} type="checkbox" /><span>启用配置</span></label></div>
                {draft.sidecar ? (
                  <div className="multica-form-grid">
                    <label className="ops-form-field">
                      <span>可执行文件</span>
                      <input autoComplete="off" disabled={Boolean(pending)} onChange={(event) => updateSidecar({ executable: event.currentTarget.value })} placeholder="选择已验证的 daemon .exe" value={draft.sidecar.executable} />
                    </label>
                    <label className="ops-form-field">
                      <span>工作目录</span>
                      <input autoComplete="off" disabled={Boolean(pending)} onChange={(event) => updateSidecar({ workingDir: event.currentTarget.value })} placeholder="可选" value={draft.sidecar.workingDir ?? ""} />
                    </label>
                    <label className="ops-form-field">
                      <span>启动参数（每行一个 argv）</span>
                      <textarea
                        autoComplete="off"
                        disabled={Boolean(pending)}
                        onChange={(event) => updateSidecar({ args: event.currentTarget.value ? event.currentTarget.value.split(/\r?\n/) : [] })}
                        placeholder={"可选，每行一个 argv\n不会按空格、引号或 shell 语法拆分"}
                        rows={4}
                        value={(draft.sidecar.args ?? []).join("\n")}
                      />
                    </label>
                    <label className="multica-checkbox-line">
                      <input checked={Boolean(draft.sidecar.autoStart)} disabled={Boolean(pending)} onChange={(event) => updateSidecar({ autoStart: event.currentTarget.checked })} type="checkbox" />
                      <span>应用启动时自动恢复（默认关闭）</span>
                    </label>
                  </div>
                ) : selected?.sidecarConfigured && draft.sidecar !== null ? (
                  <div className="multica-sidecar-summary">
                    <p className="multica-muted">
                      已配置 sidecar：{selected.sidecarExecutableName || "可执行文件已保存"}；原始路径和参数不会返回到前端。
                    </p>
                    <Button disabled={Boolean(pending)} onClick={() => updateSidecar({})} variant="outline">
                      <Pencil className="h-4 w-4" />替换 sidecar 配置
                    </Button>
                  </div>
                ) : <p className="multica-muted">未配置 sidecar；不会自动启动任何进程。</p>}
              </div>
              <div className="action-row">
                <Button disabled={Boolean(pending) || (isNew && !draft.serverUrl.trim())} onClick={() => void saveConnection()}>
                  <Save className="h-4 w-4" />
                  {pending === "save" ? "保存中" : "保存连接"}
                </Button>
                <Button disabled={Boolean(pending)} onClick={cancelEdit} variant="outline">取消</Button>
                {!isNew && selected ? <Button disabled={Boolean(pending)} onClick={() => void deleteConnection(selected)} variant="outline"><Trash2 className="h-4 w-4" />删除连接</Button> : null}
              </div>
            </Panel>
          ) : null}
        </div>

        <div className="stack">
          <Panel title="服务状态" detail={selected ? `连接：${selected.displayName || selected.connectionId}` : "选择或新增工作流连接后执行只读检查。"}>
            {selected ? (
              <>
                <div className="ops-status-list">
                  <StatusRow label="Server" status={serverStatus?.status ?? (selected.enabled ? "not_checked" : "unconfigured")} value={multicaStatusLabel(serverStatus?.status ?? (selected.enabled ? "not_checked" : "unconfigured"))} />
                  <StatusRow label="Daemon" status={daemonStatus?.status ?? (sidecarConfigured ? "stopped" : "unconfigured")} value={multicaStatusLabel(daemonStatus?.status ?? (sidecarConfigured ? "stopped" : "unconfigured"))} />
                  <StatusRow label="Server 端点" status={serverStatus?.endpoint ? "ok" : "not_checked"} value={serverStatus?.endpoint ? multicaSafeText(serverStatus.endpoint) : "未检查"} />
                  <StatusRow label="HTTP 状态" status={serverStatus?.httpStatus ? "ok" : "not_checked"} value={serverStatus?.httpStatus ? String(serverStatus.httpStatus) : "未返回"} />
                  <StatusRow label="服务版本" status={serverStatus?.version ? "ok" : "not_checked"} value={serverStatus?.version ? multicaSafeText(serverStatus.version) : "未返回"} />
                  <StatusRow label="最近检查" status={checkedAtMs ? "ok" : "not_checked"} value={multicaTimestamp(checkedAtMs)} />
                  <StatusRow label="耗时" status={serverStatus?.durationMs !== undefined ? "ok" : "not_checked"} value={serverStatus?.durationMs !== undefined ? `${serverStatus.durationMs} ms` : "未返回"} />
                  <StatusRow label="令牌" status={selected.tokenConfigured ? "ok" : "not_checked"} value={selected.tokenConfigured ? "已配置（原文不显示）" : "未配置"} />
                  <StatusRow label="数据新鲜度" status={snapshotIsStale ? "stale" : currentSnapshot ? "ok" : "not_checked"} value={currentSnapshot ? multicaStatusLabel(snapshotIsStale ? "stale" : "healthy") : "尚未获取快照"} />
                </div>
                {serverStatus?.diagnostic || daemonStatus?.diagnostic ? <p className="multica-diagnostic">{multicaSafeText(serverStatus?.diagnostic || daemonStatus?.diagnostic)}</p> : null}
                {daemonStatus?.pid ? <p className="multica-muted">sidecar PID {daemonStatus.pid} · 启动于 {multicaTimestamp(daemonStatus.startedAtMs)}</p> : null}
                <div className="action-row">
                  <Button disabled={Boolean(pending)} onClick={() => void checkConnection()}><Activity className={`h-4 w-4${pending === "check" ? " spin" : ""}`} />{pending === "check" ? "检查中" : "测试连接"}</Button>
                  <Button disabled={Boolean(pending)} onClick={() => void refreshSnapshot()} variant="outline"><RefreshCw className={`h-4 w-4${pending === "snapshot" ? " spin" : ""}`} />{pending === "snapshot" ? "刷新中" : "刷新快照"}</Button>
                  <Button disabled={Boolean(pending) || !sidecarConfigured || daemonRunning} onClick={() => void sidecarAction("start")} variant="outline"><Play className={`h-4 w-4${pending === "sidecar:start" ? " spin" : ""}`} />{pending === "sidecar:start" ? "启动中" : "启动 sidecar"}</Button>
                  <Button disabled={Boolean(pending) || !sidecarConfigured || !daemonRunning} onClick={() => void sidecarAction("stop")} variant="outline"><Power className={`h-4 w-4${pending === "sidecar:stop" ? " spin" : ""}`} />{pending === "sidecar:stop" ? "停止中" : "停止 sidecar"}</Button>
                  <Button disabled={Boolean(pending) || !sidecarConfigured} onClick={() => void sidecarAction("restart")} variant="outline"><RefreshCw className={`h-4 w-4${pending === "sidecar:restart" ? " spin" : ""}`} />{pending === "sidecar:restart" ? "重启中" : "重启 sidecar"}</Button>
                </div>
              </>
            ) : <Empty text="选择一个工作流连接后查看健康状态。" />}
          </Panel>

          <Panel title="运行时快照" detail="只读显示最近一次外部状态；刷新失败时保留旧快照并标记过期。">
            {currentSnapshot ? (
              <>
                <div className="multica-snapshot-meta">
                  <span>来源：{currentSnapshot.sourceConnectionId || "未知"}</span>
                  <span>获取于：{multicaTimestamp(currentSnapshot.fetchedAtMs)}</span>
                  <strong className={currentSnapshot.stale ? "stale" : "fresh"}>{currentSnapshot.stale ? "数据可能已过期" : "最新快照"}</strong>
                </div>
                {snapshotDiagnostic ? <p className="multica-diagnostic" role="status">{snapshotDiagnostic}</p> : null}
                <div className="context-tabs multica-snapshot-tabs" role="tablist" aria-label="工作流只读快照分类">
                  {(["runtimes", "agents", "tasks"] as const).map((kind) => {
                    const count = kind === "runtimes" ? currentSnapshot.runtimes.length : kind === "agents" ? currentSnapshot.agents.length : currentSnapshot.tasks.length;
                    const label = kind === "runtimes" ? "Runtime" : kind === "agents" ? "Agent" : "Task";
                    return <button aria-selected={tab === kind} className={tab === kind ? "active" : ""} key={kind} onClick={() => setTab(kind)} role="tab" type="button"><strong>{label}</strong><span>{count}</span></button>;
                  })}
                </div>
                <div className="multica-snapshot-list">
                  {items.length ? items.map((item) => (
                    <article className="multica-snapshot-row" key={`${tab}:${item.id}`}>
                      <div className="multica-snapshot-copy">
                        <strong>{multicaSafeText(item.title || item.name || item.id) || item.id}</strong>
                        <span>{multicaStatusLabel(item.status)}{item.runtimeType ? ` · ${multicaSafeText(item.runtimeType)}` : ""}</span>
                        {item.errorSummary ? <small>{multicaSafeText(item.errorSummary)}</small> : null}
                        <small>更新于 {multicaTimestamp(item.updatedAtMs)}</small>
                      </div>
                      <button aria-label={`复制 ${item.id} 稳定 ID`} className="context-entry-icon-button" onClick={() => void copyStableId(item.id)} title="复制稳定 ID" type="button"><Copy className="h-4 w-4" /></button>
                    </article>
                  )) : <Empty text={currentSnapshot.stale ? "没有可显示的最新数据；以上快照已标记过期。" : "该分类暂无数据。"} />}
                </div>
              </>
            ) : <Empty text={selected ? "尚未获取快照；点击“刷新快照”读取外部只读状态。" : "选择连接后读取工作流运行时、Agent 和 Task。"} />}
          </Panel>
        </div>
      </div>
    </div>
  );
}
