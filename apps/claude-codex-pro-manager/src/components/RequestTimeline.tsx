import { useEffect, useMemo, useRef, useState } from "react";
import { Clock3, RefreshCw } from "lucide-react";
import type { RequestRecord, RequestTimelineResult, TimelineLogsState } from "@/types";
import "./request-timeline.css";

const lanes = [
  { id: "provider", label: "Provider 请求" },
  { id: "protocol", label: "协议与代理" },
  { id: "agent", label: "Agent 响应" },
] as const;
type TimelineEvent = {
  id: string;
  lane: typeof lanes[number]["id"];
  timestamp: number;
  label: string;
  summary: string;
  tone: "ok" | "warning" | "failed" | "muted";
  detail: string;
};
const metrics = [
  ["input_tokens", "输入"], ["output_tokens", "输出"], ["cached_tokens", "缓存读取"],
  ["cache_creation_tokens", "缓存写入"], ["total_tokens", "总量"],
] as const;
const number = (value: number | null | undefined) => value == null ? "未采集" : value.toLocaleString("zh-CN");
const elapsed = (value: number | null) => value == null ? "未采集" : `${number(value)} ms`;
const time = (value: number) => new Date(value).toLocaleTimeString("zh-CN", { hour12: false });
const sourceName = (source: string) => source === "proxy" ? "CCP 代理" : source === "codex_rollout" ? "Codex 用量" : "Claude 用量";
const statusName = (status: string) => ({ success: "完成", failed: "失败", interrupted: "中断", observed: "已观测" }[status] ?? "状态未知");
const CHIP_WIDTH = 224;
const CHIP_HEIGHT = 36;

function requestDetail(record: RequestRecord) {
  return [
    `${sourceName(record.source)} · ${new Date(record.timestamp_ms).toLocaleString("zh-CN", { hour12: false })}`,
    `供应商：${record.provider ?? "未采集"} · 模型：${record.model ?? "未采集"}`,
    `协议：${record.protocol ?? "未采集"} → ${record.upstream_protocol ?? "未采集"}`,
    `HTTP：${record.http_status ?? "未采集"} · ${statusName(record.status)}`,
    `首字节：${elapsed(record.first_byte_ms)} · 总耗时：${elapsed(record.duration_ms)}`,
    ...metrics.map(([key, label]) => `${label}：${number(record[key])}`),
    `推理 Token：${number(record.reasoning_tokens)}`,
  ].join("\n");
}

export function RequestTimeline({ agentScope, timelineLogs, result, onRefresh }: {
  agentScope: "codex" | "claude";
  timelineLogs: TimelineLogsState;
  result: RequestTimelineResult | null;
  onRefresh: () => Promise<unknown>;
}) {
  const [refreshing, setRefreshing] = useState(false);
  const bodyRef = useRef<HTMLDivElement>(null);
  const [chartWidth, setChartWidth] = useState(640);
  useEffect(() => {
    const body = bodyRef.current;
    if (!body) return;
    const observer = new ResizeObserver(() => setChartWidth(Math.max(480, body.clientWidth - 132)));
    observer.observe(body);
    return () => observer.disconnect();
  }, []);
  const records = useMemo(() => (result?.records ?? [])
    .filter((record) => agentScope === "codex" ? record.agent === "codex" : record.agent.startsWith("claude"))
    .sort((a, b) => b.timestamp_ms - a.timestamp_ms), [result?.records, agentScope]);
  const usage = records.find((record) => metrics.some(([key]) => record[key] != null));
  const events = useMemo(() => {
    const parsed: TimelineEvent[] = [];
    for (const record of records) {
      for (const lane of lanes) {
        const local = record.source !== "proxy";
        parsed.push({
          id: `${record.id}-${lane.id}`, lane: lane.id, timestamp: record.timestamp_ms,
          label: lane.id === "provider" ? record.provider ?? "供应商未采集"
            : lane.id === "protocol" ? local ? "HTTP 未观测" : record.protocol ?? "协议未采集"
              : `${record.agent} · ${statusName(record.status)}`,
          summary: lane.id === "provider" ? record.model ?? "模型未采集"
            : lane.id === "protocol" ? local ? `${sourceName(record.source)} · 本地记录` : `代理 → ${record.upstream_protocol ?? "未采集"} · ${record.http_status ?? "未采集"}`
              : `耗时 ${elapsed(record.duration_ms)} · 输出 ${number(record.output_tokens)}`,
          tone: record.status === "success" ? "ok" : record.status === "failed" ? "failed" : record.status === "interrupted" ? "warning" : "muted",
          detail: requestDetail(record),
        });
      }
    }
    for (const [index, line] of (timelineLogs.logs?.text.split(/\r?\n/) ?? []).entries()) {
      try {
        const value = JSON.parse(line);
        if (typeof value?.event !== "string" || !/^[a-zA-Z0-9_.-]{1,120}$/.test(value.event) || value.event.toLowerCase().includes("memory")) continue;
        const timestamp = Number(value.timestamp_ms);
        if (!Number.isFinite(timestamp) || timestamp <= 0) continue;
        const name = value.event.toLowerCase();
        parsed.push({
          id: `log-${timestamp}-${index}`, timestamp, label: value.event, summary: "全局运行事件",
          lane: /provider|profile|relay_apply/.test(name) ? "provider" : /proxy|protocol|responses|chat_completion/.test(name) ? "protocol" : "agent",
          tone: /failed|error|unauthorized/.test(name) ? "failed" : /degraded|warning|retry|waiting/.test(name) ? "warning" : "ok",
          detail: `全局运行事件 · ${value.event} · ${new Date(timestamp).toLocaleString("zh-CN", { hour12: false })}`,
        });
      } catch { /* Legacy free-form log text is excluded. */ }
    }
    return parsed.sort((a, b) => b.timestamp - a.timestamp);
  }, [records, timelineLogs.logs?.text]);
  const newest = events[0]?.timestamp ?? result?.observed_at_ms ?? Date.now();
  const start = Math.min(events.at(-1)?.timestamp ?? newest, newest - 60_000);
  const axisWidth = chartWidth - CHIP_WIDTH - 16;
  const tickCount = Math.min(8, Math.floor(axisWidth / 60) + 1);
  const ticks = Array.from({ length: tickCount }, (_, index) => start + (newest - start) * index / (tickCount - 1));
  // Pack colliding chips into subrows without changing their shared time coordinate.
  const tracks = lanes.map((lane) => {
    const ends: number[] = [];
    const items = events.filter((event) => event.lane === lane.id).map((event) => {
      const left = 8 + (event.timestamp - start) / (newest - start) * axisWidth;
      let row = ends.findIndex((right) => left + CHIP_WIDTH + 8 <= right);
      if (row < 0) row = ends.length;
      ends[row] = left;
      return { ...event, left, top: 3 + row * (CHIP_HEIGHT + 6) };
    });
    return { ...lane, items, height: Math.max(42, ends.length * (CHIP_HEIGHT + 6)) };
  });
  const errors = [result?.status === "failed" ? result.message : null, timelineLogs.error].filter(Boolean);
  const notes = [...errors, ...(result?.warnings ?? [])].join("\n");

  return (
    <section className="overview-timeline overview-glass-panel request-timeline" aria-label="请求与事件时间线">
      <header className="overview-panel-heading">
        <div><Clock3 aria-hidden="true" /><span><strong>请求与事件时间线</strong><small>{records.length} 条请求/用量记录</small></span></div>
        <span className="overview-timeline-legend"><i className="ok" />成功<i className="warning" />降级<i className="failed" />失败</span>
        <button aria-label="刷新时间线" disabled={refreshing} onClick={async () => {
          setRefreshing(true);
          try { await onRefresh(); } finally { setRefreshing(false); }
        }} title="刷新时间线" type="button"><RefreshCw className={refreshing ? "spin" : ""} aria-hidden="true" /></button>
      </header>
      <div className="overview-token-usage" aria-label="最近一次 Token 用量" title={usage ? requestDetail(usage) : undefined}>
        <strong>Token</strong>
        <span>{usage ? `最近一次 · ${sourceName(usage.source)} · ${time(usage.timestamp_ms)}` : result ? "暂无用量" : "加载中"}</span>
        {metrics.map(([key, label]) => <span key={key}>{label} <b>{number(usage?.[key])}</b></span>)}
      </div>
      <div className="overview-timeline-body" ref={bodyRef}>
        <div className="overview-timeline-labels">
          <div className="overview-timeline-axis-spacer" />
          {tracks.map((lane) => <strong key={lane.id}><i />{lane.label}</strong>)}
        </div>
        <div className="overview-timeline-chart" style={{ width: chartWidth }}>
          <div className="overview-timeline-axis">{ticks.map((timestamp, index) => <time key={index} style={{ left: 8 + axisWidth * index / (tickCount - 1) }}>{time(timestamp)}</time>)}</div>
          <div className="overview-timeline-grid" aria-hidden="true" style={{ backgroundSize: `${axisWidth / (tickCount - 1)}px 100%` }} />
          {tracks.map((lane) => <div className="overview-timeline-track" key={lane.id} tabIndex={0} aria-label={lane.label}>
            <div className="overview-timeline-track-content" style={{ height: lane.height }}>
            {lane.items.length ? lane.items.map((event) => <span key={event.id} data-record={event.id} className={`overview-timeline-event ${event.tone}`} tabIndex={0} title={event.detail} aria-label={event.detail}
              style={{ left: event.left, top: event.top, width: CHIP_WIDTH, height: CHIP_HEIGHT }}>
              <i /><b>{event.label}</b><time>{time(event.timestamp)}</time><small>{event.summary}</small>
            </span>) : <span className="overview-timeline-empty">{timelineLogs.loading || !result ? "加载中" : errors.length ? "读取失败" : "暂无事件"}</span>}
            </div>
          </div>)}
          <span className="overview-timeline-now" aria-hidden="true" style={{ left: axisWidth + 8 }}>最新</span>
        </div>
      </div>
      <p className="overview-timeline-note" role={errors.length ? "alert" : undefined} title={notes || undefined}>
        {errors.length ? errors.join(" · ") : notes ? `数据来源：${result?.warnings.length} 项提示` : ""}
      </p>
    </section>
  );
}
