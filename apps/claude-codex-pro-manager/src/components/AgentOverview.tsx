import { useMemo, useRef, useState } from "react";
import { Boxes, CalendarDays, ChevronDown, ChevronLeft, ChevronRight, Coins, Info, MessagesSquare } from "lucide-react";
import type { AitrackerCapabilitiesResult, RequestRecord, RequestTimelineResult } from "@/types";
import "./agent-overview.css";

type Range = "today" | "7d" | "30d" | "all" | "custom";
const ranges: Record<Range, string> = { today: "今天", "7d": "近 7 天", "30d": "近 30 天", all: "全部", custom: "自定义" };
const dateInputValue = (date: Date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
const tokensOf = (record: RequestRecord) => record.total_tokens ?? [record.input_tokens, record.output_tokens, record.cached_tokens, record.cache_creation_tokens, record.reasoning_tokens].reduce<number>((sum, value) => sum + (value ?? 0), 0);
const compact = (value: number) => value >= 1e9 ? `${(value / 1e9).toFixed(value >= 10e9 ? 0 : 2)}B` : value >= 1e6 ? `${(value / 1e6).toFixed(value >= 10e6 ? 0 : 1)}M` : value >= 1e3 ? `${(value / 1e3).toFixed(value >= 10e3 ? 0 : 1)}K` : value.toLocaleString("zh-CN");
const smoothPath = (points: Array<{ x: number; y: number }>) => {
  if (!points.length) return "";
  if (points.length === 1) return `M ${points[0].x} ${points[0].y}`;
  return points.map((point, index) => {
    if (index === 0) return `M ${point.x} ${point.y}`;
    const previous = points[index - 1];
    const midpoint = (previous.x + point.x) / 2;
    return `C ${midpoint} ${previous.y}, ${midpoint} ${point.y}, ${point.x} ${point.y}`;
  }).join(" ");
};

function snapshotRecords(usage: RequestTimelineResult["usage_snapshot"]): RequestRecord[] | null {
  if (!usage?.details) return null;
  return usage.details.map((item) => ({
    id: item.id, timestamp_ms: Date.parse(item.timestamp), source: item.source, agent: item.agent,
    session_id: item.sessionId, project: item.project === "unknown" ? null : item.project,
    provider: item.provider === "unknown" ? null : item.provider,
    model: item.model === "unknown" ? null : item.model,
    protocol: null, upstream_protocol: null, status: item.status, http_status: null,
    duration_ms: item.durationMs, first_byte_ms: null, input_tokens: item.inputTokens,
    output_tokens: item.outputTokens, cached_tokens: item.cachedInputTokens,
    cache_creation_tokens: item.cacheCreationInputTokens, reasoning_tokens: item.reasoningOutputTokens,
    total_tokens: item.totalTokens, streaming: false,
  }));
}

function since(range: Range, from: string) {
  const now = Date.now();
  if (range === "all") return 0;
  if (range === "custom") return new Date(`${from}T00:00:00`).getTime();
  if (range === "today") return new Date().setHours(0, 0, 0, 0);
  return now - (range === "7d" ? 7 : 30) * 86400000;
}

function until(range: Range, to: string) {
  return range === "custom" ? new Date(`${to}T23:59:59.999`).getTime() : Date.now();
}

function RangeControl({ value, onChange, from, to, onFromChange, onToChange, label }: { value: Range; onChange: (range: Range) => void; from: string; to: string; onFromChange: (value: string) => void; onToChange: (value: string) => void; label: string }) {
  return <div className="agent-overview-range-wrap"><div className="agent-overview-range" role="group" aria-label={label}>{(Object.keys(ranges) as Range[]).map((range) => <button key={range} type="button" className={value === range ? "active" : ""} onClick={() => onChange(range)}>{range === "custom" && <CalendarDays aria-hidden="true" />}{ranges[range]}</button>)}</div>{value === "custom" && <div className="agent-overview-custom-dates"><input aria-label={`${label}开始日期`} type="date" value={from} max={to} onChange={(event) => onFromChange(event.target.value)} /><span>至</span><input aria-label={`${label}结束日期`} type="date" value={to} min={from} onChange={(event) => onToChange(event.target.value)} /></div>}</div>;
}

export function AgentOverview({ capabilities, timeline }: { capabilities: AitrackerCapabilitiesResult | null; timeline: RequestTimelineResult | null }) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [trendRange, setTrendRange] = useState<Range>("30d");
  const [detailRange, setDetailRange] = useState<Range>("30d");
  const [trendFrom, setTrendFrom] = useState(() => dateInputValue(new Date(Date.now() - 29 * 86400000)));
  const [trendTo, setTrendTo] = useState(() => dateInputValue(new Date()));
  const [detailFrom, setDetailFrom] = useState(() => dateInputValue(new Date(Date.now() - 29 * 86400000)));
  const [detailTo, setDetailTo] = useState(() => dateInputValue(new Date()));
  const [detailMode, setDetailMode] = useState<"model" | "project">("model");
  const [openContext, setOpenContext] = useState<string | null>(null);
  const [hoveredDay, setHoveredDay] = useState<number | null>(null);
  const rail = useRef<HTMLDivElement>(null);
  const source = timeline?.usage_snapshot ?? capabilities?.usage;
  const records = useMemo(() => snapshotRecords(source) ?? timeline?.records ?? [], [source, timeline?.records]);
  const registry = capabilities?.snapshot.registry ?? [];
  const sessionSummaries = capabilities?.snapshot.sessions ?? [];
  const skillTotal = registry.reduce((sum, agent) => sum + (agent.skillCount ?? 0), 0);
  const trendStart = since(trendRange, trendFrom);
  const trendEnd = until(trendRange, trendTo);
  const rangeRecords = records.filter((record) => record.timestamp_ms >= trendStart && record.timestamp_ms <= trendEnd);
  const rangeSessions = sessionSummaries.filter((session) => {
    const timestamp = Date.parse(session.endedAt || session.startedAt);
    return timestamp >= trendStart && timestamp <= trendEnd;
  });
  const agents = useMemo(() => {
    const byId = new Map(registry.map((agent) => [agent.id, agent]));
    const ids = new Set([...registry.map((agent) => agent.id), ...rangeRecords.map((record) => record.agent)]);
    return [...ids].map((id) => {
      const item = byId.get(id);
      const own = rangeRecords.filter((record) => record.agent === id);
      const ownSessions = rangeSessions.filter((session) => session.agent === id).map((session) => session.sessionId);
      return { id, name: item?.name ?? id, icon: item?.icon, color: item?.color || "#34d3ad", detected: item?.detected ?? (own.length > 0 || ownSessions.length > 0), skillCount: item?.skillCount ?? null, skillScanStatus: item?.skillScanStatus, tokens: own.reduce((sum, record) => sum + tokensOf(record), 0), sessions: new Set([...own.map((record) => record.session_id).filter(Boolean) as string[], ...ownSessions]).size };
    }).sort((a, b) => Number(b.detected) - Number(a.detected) || b.tokens - a.tokens || a.name.localeCompare(b.name));
  }, [registry, rangeRecords]);
  const selected = agents.find((agent) => agent.id === selectedId) ?? agents[0];
  const name = selected?.name ?? "Agent";
  const color = selected?.color ?? "#34d3ad";
  const own = records.filter((record) => record.agent === selected?.id);
  const trendRecords = rangeRecords.filter((record) => record.agent === selected?.id);
  const detailRecords = own.filter((record) => record.timestamp_ms >= since(detailRange, detailFrom) && record.timestamp_ms <= until(detailRange, detailTo));
  const total = trendRecords.reduce((sum, record) => sum + tokensOf(record), 0);
  const allTotal = rangeRecords.reduce((sum, record) => sum + tokensOf(record), 0);
  const sessionIds = new Set([...trendRecords.map((record) => record.session_id).filter(Boolean) as string[], ...rangeSessions.filter((session) => session.agent === selected?.id).map((session) => session.sessionId)]);
  const sessions = sessionIds.size;
  const allSessions = new Set([...rangeRecords.map((record) => record.session_id).filter(Boolean) as string[], ...rangeSessions.map((session) => session.sessionId)]).size;
  const hasSessionIds = sessionIds.size > 0;
  const cache = trendRecords.reduce((sum, record) => sum + (record.cached_tokens ?? 0), 0);
  const input = trendRecords.reduce((sum, record) => sum + (record.input_tokens ?? 0), 0);
  const cacheCreation = trendRecords.reduce((sum, record) => sum + (record.cache_creation_tokens ?? 0), 0);
  const output = trendRecords.reduce((sum, record) => sum + (record.output_tokens ?? 0), 0);
  const reasoning = trendRecords.reduce((sum, record) => sum + (record.reasoning_tokens ?? 0), 0);
  const toolCalls = capabilities?.snapshot.toolCalls.filter((event) => event.agent === selected?.id && Date.parse(event.timestamp) >= trendStart && Date.parse(event.timestamp) <= trendEnd).length;
  // 不能写成 Math.min(...records.map(...))：记录超过约 20 万条时展开参数会抛
  // RangeError（Maximum call stack size exceeded），整屏黑屏。
  const firstDay = trendRange === "all" ? trendRecords.reduce((min, record) => Math.min(min, record.timestamp_ms), Date.now()) : trendStart;
  const dayCount = Math.max(1, Math.min(366, Math.ceil((trendEnd - firstDay) / 86400000)));
  const days = Array.from({ length: dayCount }, (_, index) => {
    const date = new Date(firstDay + index * 86400000);
    const key = `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
    return { key, label: `${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`, tokens: 0 };
  });
  const byDate = new Map(days.map((day) => [day.key, day]));
  trendRecords.forEach((record) => { const date = new Date(record.timestamp_ms); const day = byDate.get(`${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`); if (day) day.tokens += tokensOf(record); });
  const peak = Math.max(0, ...days.map((day) => day.tokens));
  const chartMax = peak || 1;
  const chartWidth = 920;
  const chartHeight = 165;
  const plotLeft = 54;
  const plotWidth = chartWidth - plotLeft - 10;
  const pointX = (index: number) => plotLeft + (index + .5) * plotWidth / days.length;
  const pointY = (value: number) => chartHeight - value / chartMax * (chartHeight - 12);
  const rows = new Map<string, { tokens: number; events: number; sessions: Set<string> }>();
  detailRecords.forEach((record) => { const key = (detailMode === "model" ? record.model : record.project)?.trim() || (detailMode === "model" ? "模型未采集" : "项目未采集"); const row = rows.get(key) ?? { tokens: 0, events: 0, sessions: new Set<string>() }; row.tokens += tokensOf(record); row.events++; if (record.session_id) row.sessions.add(record.session_id); rows.set(key, row); });
  const ranked = [...rows.entries()].sort((a, b) => b[1].tokens - a[1].tokens);
  const maxRow = ranked[0]?.[1].tokens || 1;
  const tokenParts = [{ key: "messages", label: "对话消息", tokens: input + cache + cacheCreation + output, children: [{ label: "用户输入", tokens: input }, { label: "缓存读取", tokens: cache }, { label: "缓存写入", tokens: cacheCreation }, { label: "助手回复", tokens: output }] }, { key: "reasoning", label: "推理过程", tokens: reasoning, children: [{ label: "思考 Token", tokens: reasoning }] }];

  return <div className="agent-overview">
    <div className="agent-overview-metrics">
      <div><span><Coins />本 Agent 消耗</span><strong>{selected ? compact(total) : "未采集"}</strong><small>{selected && allTotal ? `占全部 ${Math.round(total / allTotal * 100)}%` : "暂无本地用量记录"} · 费用未计价</small></div>
      <div><span><MessagesSquare />会话场次</span><strong>{hasSessionIds ? sessions : "未采集"}</strong><small>{rangeRecords.some((record) => record.session_id) ? `全部 Agent 共 ${allSessions} 次` : "本地记录未提供会话 ID"}</small></div>
      <div><span><Boxes />SKILL 覆盖</span><strong>{selected?.skillScanStatus === "unsupported" ? "不支持" : selected?.skillScanStatus === "error" ? "采集失败" : selected?.skillCount ?? "未采集"}</strong><small>{selected?.skillScanStatus === "missing" ? "未发现本机 Skill 目录" : selected?.skillScanStatus === "error" ? "本机 Skill 目录读取失败" : selected?.skillScanStatus === "unsupported" ? "该 Agent 未提供 Skill 目录" : `本机已安装 · 全部 Agent 共 ${skillTotal} 个`}</small></div>
    </div>
    <div className="agent-overview-rail-wrap">
      <button className="agent-overview-rail-arrow" type="button" aria-label="向左滚动 Agent" onClick={() => rail.current?.scrollBy({ left: -240, behavior: "smooth" })}><ChevronLeft /></button>
      <div className="agent-overview-rail" ref={rail}>{agents.length ? agents.map((agent) => <button key={agent.id} type="button" className={agent.id === selected?.id ? "active" : ""} onClick={() => setSelectedId(agent.id)}><span className="agent-overview-logo" style={{ color: agent.color, background: `${agent.color}22` }}>{agent.icon || agent.name.slice(0, 1).toUpperCase()}</span><span className="agent-overview-rail-text"><strong>{agent.name}<i className={agent.detected ? "detected" : ""} title={agent.detected ? "已安装或有本地记录" : "未检测到安装或记录"} /></strong><small>{agent.tokens ? compact(agent.tokens) : "—"} · {agent.sessions} 会话</small></span></button>) : <span className="agent-overview-rail-empty">暂无 Agent 采集记录</span>}</div>
      <button className="agent-overview-rail-arrow" type="button" aria-label="向右滚动 Agent" onClick={() => rail.current?.scrollBy({ left: 240, behavior: "smooth" })}><ChevronRight /></button>
    </div>
    <section className="agent-overview-panel agent-overview-trend"><header><div><strong>{name} · 消耗趋势</strong><small>区间合计 {compact(total)} · 日均 {compact(Math.round(total / dayCount))} · 峰值 {compact(peak)}</small></div><RangeControl value={trendRange} onChange={setTrendRange} from={trendFrom} to={trendTo} onFromChange={setTrendFrom} onToChange={setTrendTo} label="消耗趋势时间范围" /></header>
      <div className="agent-overview-chart">{trendRecords.length ? <svg viewBox={`0 0 ${chartWidth} 205`} preserveAspectRatio="none" role="img" aria-label={`${name} 每日 Token 消耗柱线趋势图`}>
        {[0, .25, .5, .75, 1].map((fraction) => <g key={fraction}><line x1={plotLeft} x2={chartWidth} y1={pointY(chartMax * fraction)} y2={pointY(chartMax * fraction)} stroke="currentColor" strokeOpacity=".16" strokeDasharray="2 4" /><text x={plotLeft - 8} y={pointY(chartMax * fraction) + 4} textAnchor="end">{compact(Math.round(chartMax * fraction))}</text></g>)}
        {days.map((day, index) => <rect key={day.key} x={pointX(index) - Math.min(12, plotWidth / days.length * .4)} y={pointY(day.tokens)} width={Math.min(24, plotWidth / days.length * .8)} height={chartHeight - pointY(day.tokens)} rx="3" fill={color} fillOpacity=".35"><title>{day.label} · {compact(day.tokens)} Token</title></rect>)}
        <path d={smoothPath(days.map((day, index) => ({ x: pointX(index), y: pointY(day.tokens) })))} fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" vectorEffect="non-scaling-stroke" />
        {days.map((day, index) => <rect key={`hit-${day.key}`} x={pointX(index) - Math.max(10, plotWidth / days.length / 2)} y="0" width={Math.max(20, plotWidth / days.length)} height="175" fill="transparent" onMouseEnter={() => setHoveredDay(index)} onMouseLeave={() => setHoveredDay(null)} />)}
        {hoveredDay != null && days[hoveredDay] ? <g className="agent-overview-tooltip" pointerEvents="none"><line x1={pointX(hoveredDay)} x2={pointX(hoveredDay)} y1="0" y2={chartHeight} stroke={color} strokeOpacity=".45" strokeDasharray="2 3" /><circle cx={pointX(hoveredDay)} cy={pointY(days[hoveredDay].tokens)} r="3.5" fill={color} stroke="var(--workspace-surface)" strokeWidth="2" /><rect x={Math.min(Math.max(plotLeft, pointX(hoveredDay) - 62), chartWidth - 144)} y="8" width="136" height="35" rx="5" fill="var(--workspace-surface-raised)" stroke="var(--workspace-border)" /><text x={Math.min(Math.max(plotLeft + 68, pointX(hoveredDay) + 6), chartWidth - 72)} y="22" textAnchor="middle">{days[hoveredDay].label}</text><text x={Math.min(Math.max(plotLeft + 68, pointX(hoveredDay) + 6), chartWidth - 72)} y="35" textAnchor="middle" fill="var(--workspace-text)">{days[hoveredDay].tokens.toLocaleString("zh-CN")} Token</text></g> : null}
        {days.map((day, index) => index % Math.max(1, Math.ceil(days.length / 9)) === 0 || index === days.length - 1 ? <text key={day.key} x={pointX(index)} y="190" textAnchor="middle">{day.label}</text> : null)}
      </svg> : <p className="agent-overview-empty">当前区间暂无消耗记录</p>}</div>
    </section>
    <section className="agent-overview-panel agent-overview-context"><header><strong>Agent 体验 · 上下文构成</strong><small>{name}</small></header>
      <div className="agent-overview-cache">缓存命中率 <strong>{input + cache ? `${(cache / (input + cache) * 100).toFixed(1)}%` : "未采集"}</strong><span>{trendRecords.length ? `${compact(cache)} reused / ${compact(total)}` : "暂无 Token 记录"}</span></div>
      {tokenParts.map((part) => <div key={part.key}><button className="agent-overview-context-row" type="button" aria-expanded={openContext === part.key} onClick={() => setOpenContext(openContext === part.key ? null : part.key)}><ChevronDown className={openContext === part.key ? "open" : ""} /><b>{part.label}</b><span>{trendRecords.length ? compact(part.tokens) : "未采集"}</span><small>{total ? `${(part.tokens / total * 100).toFixed(1)}%` : "—"}</small></button>{openContext === part.key && part.children.map((child) => <div className="agent-overview-context-child" key={child.label}><span>{child.label}</span><b>{trendRecords.length ? compact(child.tokens) : "未采集"}</b><small>{total ? `${(child.tokens / total * 100).toFixed(1)}%` : "—"}</small></div>)}</div>)}
      <div className="agent-overview-context-row passive"><span /> <b>工具输出</b><span>未采集</span><small>—</small></div>
      <div className="agent-overview-context-row passive"><span /> <b>工具调用</b><span>{toolCalls == null ? "未采集" : `${toolCalls.toLocaleString("zh-CN")} 次`}</span><small>—</small></div>
      <p className="agent-overview-hint"><Info />工具调用次数来自本地事件；Token 由模型用量记录提供，并非供应商逐工具计费。</p>
      <div className="agent-overview-context-row passive"><span /> <b>技能调用</b><span>未采集</span><small>—</small></div>
    </section>
    <section className="agent-overview-panel agent-overview-details"><header><div className="agent-overview-detail-title"><strong>{name} · 消耗明细</strong><div role="group" aria-label="Agent 消耗分组"><button className={detailMode === "model" ? "active" : ""} onClick={() => setDetailMode("model")} type="button">按模型</button><button className={detailMode === "project" ? "active" : ""} onClick={() => setDetailMode("project")} type="button">按项目</button></div><small>{ranked.length} 个{detailMode === "model" ? "模型" : "项目"}</small></div><RangeControl value={detailRange} onChange={setDetailRange} from={detailFrom} to={detailTo} onFromChange={setDetailFrom} onToChange={setDetailTo} label="消耗明细时间范围" /></header>
      <div className="agent-overview-detail-list">{ranked.length ? ranked.map(([key, row]) => <div className="agent-overview-detail-row" key={key}><strong title={key}>{key}</strong><div className="agent-overview-meter"><span style={{ width: `${Math.max(row.tokens / maxRow * 100, 2)}%`, background: color }} /></div><b>{compact(row.tokens)}</b><span>{detailMode === "model" ? `${row.events.toLocaleString("zh-CN")} 事件` : `${row.sessions.size} 会话`}</span><small>{detailMode === "model" ? "未计价" : `${Math.round(row.tokens / Math.max(detailRecords.reduce((sum, record) => sum + tokensOf(record), 0), 1) * 100)}%`}</small></div>) : <p className="agent-overview-empty">当前区间暂无消耗明细</p>}</div>
    </section>
  </div>;
}
