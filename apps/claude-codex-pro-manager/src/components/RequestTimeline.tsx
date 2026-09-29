import { useMemo, useState } from "react";
import { BarChart3 } from "lucide-react";
import type { RequestRecord, RequestTimelineResult, TimelineLogsState } from "@/types";
import "./request-timeline.css";

const number = (value: number | null | undefined) => value == null ? "未采集" : value.toLocaleString("zh-CN");
const sourceName = (source: string) => source === "proxy" ? "CCP 代理" : source === "codex_rollout" ? "Codex 用量" : "Claude 用量";
const elapsed = (value: number | null) => value == null ? "未采集" : `${number(value)} ms`;
const statusName = (status: string) => ({ success: "完成", failed: "失败", interrupted: "中断", observed: "已观测" }[status] ?? "状态未知");
const metrics = [["input_tokens", "输入"], ["output_tokens", "输出"], ["cached_tokens", "缓存读取"], ["cache_creation_tokens", "缓存写入"], ["total_tokens", "总量"]] as const;
function tokenTotal(record: RequestRecord) {
  if (record.total_tokens != null) return record.total_tokens;
  return [record.input_tokens, record.output_tokens, record.cached_tokens, record.cache_creation_tokens, record.reasoning_tokens].reduce<number>((sum, value) => sum + (value ?? 0), 0);
}
function snapshotRecords(usage: RequestTimelineResult["usage_snapshot"]): RequestRecord[] | null {
  if (!usage?.details) return null;
  return usage.details.map((item) => ({
    id: item.id,
    timestamp_ms: Date.parse(item.timestamp),
    source: item.source,
    agent: item.agent,
    session_id: item.sessionId,
    project: item.project === "unknown" ? null : item.project,
    provider: item.provider === "unknown" ? null : item.provider,
    model: item.model === "unknown" ? null : item.model,
    protocol: null,
    upstream_protocol: null,
    status: item.status,
    http_status: null,
    duration_ms: item.durationMs,
    first_byte_ms: null,
    input_tokens: item.inputTokens,
    output_tokens: item.outputTokens,
    cached_tokens: item.cachedInputTokens,
    cache_creation_tokens: item.cacheCreationInputTokens,
    reasoning_tokens: item.reasoningOutputTokens,
    total_tokens: item.totalTokens,
    streaming: false,
  }));
}
function requestDetail(record: RequestRecord) {
  return [`${sourceName(record.source)} · ${new Date(record.timestamp_ms).toLocaleString("zh-CN", { hour12: false })}`, `供应商：${record.provider ?? "未采集"} · 模型：${record.model ?? "未采集"}`, `协议：${record.protocol ?? "未采集"} → ${record.upstream_protocol ?? "未采集"}`, `HTTP：${record.http_status ?? "未采集"} · ${statusName(record.status)}`, `首字节：${elapsed(record.first_byte_ms)} · 总耗时：${elapsed(record.duration_ms)}`, ...metrics.map(([key, label]) => `${label}：${number(record[key])}`), `推理 Token：${number(record.reasoning_tokens)}`].join("\n");
}
type Range = "today" | "7d" | "30d" | "all";
const rangeLabels: Record<Range, string> = { today: "今天", "7d": "近 7 天", "30d": "近 30 天", all: "全部" };

export function RequestTimeline({ agentScope, timelineLogs, result }: { agentScope: "codex" | "claude"; timelineLogs: TimelineLogsState; result: RequestTimelineResult | null; }) {
  const [range, setRange] = useState<Range>("30d");
  const [group, setGroup] = useState<"model" | "project">("model");
  const records = useMemo(() => {
    const now = Date.now();
    const start = range === "today" ? new Date(new Date(now).setHours(0, 0, 0, 0)).getTime() : range === "7d" ? now - 7 * 24 * 60 * 60 * 1000 : range === "30d" ? now - 30 * 24 * 60 * 60 * 1000 : 0;
    const snapshot = snapshotRecords(result?.usage_snapshot);
    const source = snapshot ?? (result?.records ?? []).filter((record) => agentScope === "codex" ? record.agent === "codex" : record.agent.startsWith("claude"));
    return source.filter((record) => record.timestamp_ms >= start).sort((a, b) => b.timestamp_ms - a.timestamp_ms);
  }, [agentScope, range, result?.records, result?.usage_snapshot]);
  const models = useMemo(() => {
    const grouped = new Map<string, { model: string; tokens: number; events: number; records: RequestRecord[] }>();
    for (const record of records) {
      const model = group === "project" ? record.project?.trim() || "项目未采集" : record.model?.trim() || "模型未采集";
      const current = grouped.get(model) ?? { model, tokens: 0, events: 0, records: [] };
      current.tokens += tokenTotal(record); current.events += 1; current.records.push(record); grouped.set(model, current);
    }
    return [...grouped.values()].sort((a, b) => b.tokens - a.tokens || b.events - a.events);
  }, [group, records]);
  const maxTokens = Math.max(...models.map((item) => item.tokens), 1);
  const errors = [result?.status === "failed" ? result.message : null, timelineLogs.error].filter(Boolean);
  const notes = [...errors, ...(result?.warnings ?? [])].join("\n");
  const stateLabel = !result || timelineLogs.loading ? "加载中" : errors.length ? "读取失败" : models.length ? `${models.length} 个模型` : "暂无模型数据";
  return (
    <section className="overview-timeline overview-glass-panel request-timeline" aria-label="模型消耗明细">
      <header className="overview-panel-heading request-usage-heading">
        <div><BarChart3 aria-hidden="true" /><span><strong>模型消耗明细</strong><small>{stateLabel}</small></span></div>
        <div className="request-usage-controls" role="group" aria-label="消耗分组"><button type="button" className={group === "model" ? "active" : ""} onClick={() => setGroup("model")}>按模型</button><button type="button" className={group === "project" ? "active" : ""} onClick={() => setGroup("project")}>按项目</button></div>
        <span className="request-usage-model-count">{models.length} 个{group === "model" ? "模型" : "项目"}</span>
        <div className="request-usage-ranges" role="group" aria-label="时间范围">{(Object.keys(rangeLabels) as Range[]).map((key) => <button key={key} type="button" className={range === key ? "active" : ""} onClick={() => setRange(key)}>{rangeLabels[key]}</button>)}</div>
      </header>
      <div className="request-usage-list" aria-live="polite">
        {models.length ? models.map((item) => {
          const tokens = item.tokens;
          const recordDetails = item.records.map(requestDetail).join("\n\n");
          const cost = "未计价";
          return <div className="request-usage-row" key={item.model} title={recordDetails}><strong className="request-usage-model">{item.model}</strong><div className="request-usage-meter" aria-label={`${item.model} 用量占比`}><span style={{ width: `${Math.max(4, tokens / maxTokens * 100)}%` }} /></div><strong className="request-usage-tokens">{number(tokens)} <small>Token</small></strong><span className="request-usage-events">{number(item.events)} 事件</span><span className="request-usage-cost">{cost}</span></div>;
        }) : <div className="request-usage-empty">{stateLabel}{group === "project" ? " · 当前记录没有项目字段" : ""}</div>}
      </div>
      <p className="overview-timeline-note" role={errors.length ? "alert" : undefined} title={notes || undefined}>{errors.length ? errors.join(" · ") : notes ? `数据来源：${result?.warnings.length} 项提示` : models.length ? "费用数据：当前请求记录未提供计价字段" : "暂无本地请求记录"}</p>
    </section>
  );
}
