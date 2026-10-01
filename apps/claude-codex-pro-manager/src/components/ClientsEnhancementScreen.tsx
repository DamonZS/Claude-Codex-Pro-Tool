import { useEffect, useMemo, useState } from "react";
import {
  BookOpen,
  BrainCircuit,
  Check,
  Clock3,
  FlaskConical,
  FolderOpen,
  History,
  LoaderCircle,
  Send,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import type { AppActions } from "@/lib/actions";
import {
  filterDistillationSessions,
  groupDistillationSessionsByProject,
} from "@/lib/distillation-materials";
import type {
  AitrackerSessionDetailResult,
  DistillationCandidate,
  DistillationRunResult,
  DistillationSessionSelection,
  DistillationWorkbenchResult,
  DistillationWorkbenchSession,
} from "@/types";

// The layout and interaction order below follow AITRACKER's
// DistillationPage/DistillConfig/MaterialDrawer/ExpCard flow.  CCP owns the
// rendered components and liquid-glass styles; only the distillation semantics
// are shared with the local Rust workbench commands.

type DistillKind = "skill" | "brief" | "prompt" | "persona" | "memory";
type TimeRange = "today" | "7d" | "30d" | "all";
type MaterialMode = "session" | "project";
type Selection = DistillationSessionSelection;

const OUTPUT_TYPES: Array<{ id: DistillKind; label: string; group: "capability" | "memory" }> = [
  { id: "skill", label: "Skill", group: "capability" },
  { id: "brief", label: "Workflow", group: "capability" },
  { id: "prompt", label: "Prompt", group: "capability" },
  { id: "persona", label: "Profile / Persona", group: "memory" },
  { id: "memory", label: "Task Memory", group: "memory" },
];

function sessionKey(session: Pick<DistillationWorkbenchSession, "agent" | "sessionId">) {
  return `${session.agent}:${session.sessionId}`;
}

function agentLabel(agent: string) {
  const labels: Record<string, string> = {
    codex: "Codex",
    "claude-code": "Claude Code",
    "claude-desktop": "Claude Desktop",
    workbuddy: "WorkBuddy",
    cursor: "Cursor",
    openclaw: "OpenClaw",
  };
  return labels[agent.toLocaleLowerCase()] ?? agent;
}

function timestampValue(value: string | number | null | undefined) {
  if (typeof value === "number") return Number.isFinite(value) ? value : Number.NaN;
  const normalized = value?.trim();
  if (!normalized) return Number.NaN;
  return /^\d+(?:\.\d+)?$/.test(normalized) ? Number(normalized) : Date.parse(normalized);
}

function dateLabel(value: string | number | null | undefined) {
  const date = new Date(timestampValue(value));
  return Number.isNaN(date.getTime()) ? "时间未知" : date.toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

function materialRange(range: TimeRange) {
  return range === "7d" ? "7" : range === "30d" ? "30" : range;
}

function formatTokens(value: number) {
  if (value >= 1_000_000_000) return `${compactTokenValue(value / 1_000_000_000)}B`;
  if (value >= 1_000_000) return `${compactTokenValue(value / 1_000_000)}M`;
  if (value >= 1_000) return `${compactTokenValue(value / 1_000)}K`;
  return Math.round(value).toLocaleString("zh-CN");
}

function compactTokenValue(value: number) {
  return value.toFixed(value >= 100 ? 0 : value >= 10 ? 1 : 2).replace(/\.?0+$/, "");
}

function sessionRow(session: DistillationWorkbenchSession, selected: boolean, onToggle: () => void, onPreview: () => void) {
  return <article className={`distillation-session-row${selected ? " selected" : ""}`} key={sessionKey(session)}>
    <button aria-pressed={selected} className="distillation-session-select" onClick={onToggle} type="button">
      <span className="distillation-check">{selected ? <Check aria-hidden="true" className="h-3.5 w-3.5" /> : null}</span>
      <span className="distillation-session-copy">
        <strong>{session.title || session.sessionId}</strong>
        <small>{agentLabel(session.agent)} · {dateLabel(session.updatedAt)} · {session.events} 轮 · ~{formatTokens(session.tokens)} tokens</small>
      </span>
      <span className="distillation-session-model">{session.model || "未知模型"}</span>
    </button>
    <button aria-label={`预览 ${session.title || session.sessionId}`} className="distillation-preview-button" onClick={onPreview} type="button"><BookOpen className="h-4 w-4" />预览</button>
  </article>;
}

function AssetGroup({
  title,
  destination,
  items,
  value,
  onChange,
}: {
  title: string;
  destination: string;
  items: typeof OUTPUT_TYPES;
  value: DistillKind;
  onChange: (kind: DistillKind) => void;
}) {
  return <section className={`distillation-asset-group${items.some((item) => item.id === value) ? " active" : ""}`}>
    <header><strong>{title}</strong><span>→ {destination}</span></header>
    <div>{items.map((item) => <button aria-pressed={value === item.id} className={value === item.id ? "active" : ""} key={item.id} onClick={() => onChange(item.id)} type="button">{item.label}</button>)}</div>
  </section>;
}

function DistillationWorkbench({ actions, data }: { actions: AppActions; data: DistillationWorkbenchResult | null }) {
  const [range, setRange] = useState<TimeRange>("all");
  const [materialMode, setMaterialMode] = useState<MaterialMode>("session");
  const [selected, setSelected] = useState<Record<string, Selection>>({});
  const [providerId, setProviderId] = useState("");
  const [modelId, setModelId] = useState("");
  const [kind, setKind] = useState<DistillKind>("skill");
  const [mode, setMode] = useState<"model">("model");
  const [configMode, setConfigMode] = useState<"quick" | "advanced">("quick");
  const [prompt, setPrompt] = useState("");
  const [tab, setTab] = useState<"config" | "history">("config");
  const [preview, setPreview] = useState<DistillationWorkbenchSession | null>(null);
  const [detailCache, setDetailCache] = useState<Record<string, AitrackerSessionDetailResult["transcript"]>>({});
  const [task, setTask] = useState<DistillationRunResult | null>(null);
  const [notice, setNotice] = useState("");

  const sessions = data?.sessions ?? [];
  const candidates = data?.candidates ?? [];
  const providers = data?.providers ?? [];
  const visibleSessions = useMemo(() => {
    return [...filterDistillationSessions(sessions, materialRange(range) as "today" | "7" | "30" | "all")]
      .sort((left, right) => timestampValue(right.updatedAt) - timestampValue(left.updatedAt));
  }, [range, sessions]);
  const visibleSessionKeys = useMemo(() => new Set(visibleSessions.map(sessionKey)), [visibleSessions]);
  useEffect(() => {
    setSelected((current) => {
      const next = Object.fromEntries(Object.entries(current).filter(([key]) => visibleSessionKeys.has(key))) as Record<string, Selection>;
      return Object.keys(next).length === Object.keys(current).length ? current : next;
    });
  }, [visibleSessionKeys]);
  const currentProvider = providers.find((provider) => provider.id === providerId);
  const models = currentProvider?.models ?? [];
  const selectedSessions = visibleSessions.flatMap((session) => {
    const selection = selected[sessionKey(session)];
    return selection ? [selection] : [];
  });
  const selectedCount = selectedSessions.length;
  const estimatedTokens = selectedSessions.reduce((sum, selection) => sum + (sessions.find((session) => sessionKey(session) === sessionKey(selection))?.tokens ?? 0), 0);
  const savedCount = candidates.filter((candidate) => candidate.status === "saved").length;
  const activeTask = task?.phase && !["completed", "failed", "cancelled"].includes(task.phase);
  const transcript = preview ? detailCache[sessionKey(preview)] : null;
  const transcriptMessages = transcript?.messages ?? [];
  const previewSelection = preview ? selected[sessionKey(preview)] : null;

  useEffect(() => {
    if (!providerId && providers[0]) setProviderId(providers[0].id);
  }, [providerId, providers]);

  useEffect(() => {
    if ((!modelId || !models.includes(modelId)) && models[0]) setModelId(models[0]);
  }, [modelId, models]);

  useEffect(() => {
    if (!preview) return;
    const key = sessionKey(preview);
    if (Object.prototype.hasOwnProperty.call(detailCache, key)) return;
    let current = true;
    void actions.readAitrackerSessionDetail({ agent: preview.agent, sessionId: preview.sessionId }).then((result) => {
      if (current) setDetailCache((cache) => ({ ...cache, [key]: result?.transcript ?? null }));
    });
    return () => { current = false; };
  }, [actions, detailCache, preview]);

  useEffect(() => {
    if (!task?.taskId || !activeTask) return;
    let stopped = false;
    let timer = 0;
    const poll = async () => {
      const result = await actions.queryDistillationTask(task.taskId);
      if (stopped || !result) return;
      setTask(result);
      if (["completed", "failed", "cancelled"].includes(result.phase)) {
        if (result.phase === "completed") setNotice("蒸馏完成，候选已加入历史，等待审批。");
        if (result.phase === "failed") setNotice(result.detail || result.message || "蒸馏任务失败。");
        if (result.phase === "cancelled") setNotice("蒸馏任务已取消。");
        void actions.loadDistillationWorkbench();
      } else {
        timer = window.setTimeout(() => void poll(), 800);
      }
    };
    timer = window.setTimeout(() => void poll(), 350);
    return () => { stopped = true; window.clearTimeout(timer); };
  }, [actions, activeTask, task?.taskId]);

  const toggleSession = (session: DistillationWorkbenchSession) => {
    const key = sessionKey(session);
    setSelected((current) => {
      const next = { ...current };
      if (next[key]) delete next[key];
      else next[key] = { agent: session.agent, sessionId: session.sessionId };
      return next;
    });
  };

  const toggleProject = (projectSessions: readonly DistillationWorkbenchSession[]) => {
    const allSelected = projectSessions.every((session) => selected[sessionKey(session)]);
    setSelected((current) => {
      const next = { ...current };
      for (const session of projectSessions) {
        const key = sessionKey(session);
        if (allSelected) delete next[key];
        else next[key] = { agent: session.agent, sessionId: session.sessionId };
      }
      return next;
    });
  };

  const setRangeBoundary = (which: "startIndex" | "endIndex", index: number) => {
    if (!preview) return;
    const key = sessionKey(preview);
    const max = Math.max(0, transcriptMessages.length - 1);
    setSelected((current) => {
      const prior = current[key] ?? { agent: preview.agent, sessionId: preview.sessionId };
      const startIndex = which === "startIndex" ? index : Math.min(prior.startIndex ?? 0, index);
      const endIndex = which === "endIndex" ? index : Math.max(prior.endIndex ?? max, index);
      return { ...current, [key]: { ...prior, startIndex: Math.max(0, Math.min(max, startIndex)), endIndex: Math.max(0, Math.min(max, endIndex)) } };
    });
  };

  const selectVisible = () => setSelected((current) => ({
    ...current,
    ...Object.fromEntries(visibleSessions.map((session) => [sessionKey(session), { agent: session.agent, sessionId: session.sessionId }])) as Record<string, Selection>,
  }));

  const clearSelection = () => setSelected({});

  const start = async () => {
    if (!selectedCount || activeTask) return;
    setNotice("");
    const result = await actions.runDistillationWorkbench({
      selections: selectedSessions,
      providerId: currentProvider?.id ?? "",
      modelId,
      kind,
      mode,
      prompt: prompt.trim() || undefined,
    });
    setTask(result);
    if (result?.phase === "failed") setNotice(result.message || "蒸馏任务启动失败。");
  };

  const cancel = async () => {
    if (!task?.taskId) return;
    const result = await actions.cancelDistillationTask(task.taskId);
    if (result) setTask(result);
  };

  const approve = async (candidate: DistillationCandidate) => {
    const result = await actions.updateDistillationCandidate({ id: candidate.id });
    if (result?.candidates) void actions.loadDistillationWorkbench();
  };

  const cancelCandidate = async (candidate: DistillationCandidate) => {
    const result = await actions.cancelDistillationCandidate({ id: candidate.id });
    if (result?.candidates) void actions.loadDistillationWorkbench();
  };

  const save = async (candidate: DistillationCandidate) => {
    const result = await actions.saveDistillationOutput({ candidateId: candidate.id, target: candidate.kind ?? "memory", skillId: candidate.id });
    setNotice(result?.message ?? "蒸馏产出已写入目标库。");
    if (result?.status === "ok") void actions.loadDistillationWorkbench();
  };

  const outputKindLabel = OUTPUT_TYPES.find((item) => item.id === kind)?.label ?? "Skill";
  const sessionGroups = useMemo(() => {
    return groupDistillationSessionsByProject(visibleSessions).map((group) => [group.projectKey, [...group.sessions]] as const);
  }, [visibleSessions]);
  const groupedKeys = useMemo(() => new Set(sessionGroups.map(([key]) => key)), [sessionGroups]);
  const ungroupedSessions = visibleSessions.filter((session) => !groupedKeys.has(session.projectKey ?? session.project));

  return <section className="distillation-workbench" aria-label="蒸馏工作台">
    <div className="distillation-metrics">
      <article><small>已选素材</small><strong>{selectedCount}</strong><span>~{formatTokens(estimatedTokens)} tokens</span></article>
      <article><small>素材 Token</small><strong>{formatTokens(estimatedTokens)}</strong><span>本次输入预估</span></article>
      <article><small>蒸馏次数</small><strong>{candidates.length}</strong><span>{activeTask ? "正在蒸馏" : "历史任务"}</span></article>
      <article><small>已入库</small><strong>{savedCount}</strong><span>已审批并写入</span></article>
    </div>

    <div className="distillation-tabs" role="tablist" aria-label="蒸馏工作台视图">
      <button aria-selected={tab === "config"} className={tab === "config" ? "active" : ""} onClick={() => setTab("config")} role="tab" type="button"><FlaskConical className="h-4 w-4" />蒸馏配置</button>
      <button aria-selected={tab === "history"} className={tab === "history" ? "active" : ""} onClick={() => setTab("history")} role="tab" type="button"><History className="h-4 w-4" />蒸馏历史</button>
      <span>{tab === "config" ? "选素材、配参数、跑蒸馏" : `${candidates.length} 个候选 · ${savedCount} 个已入库`}</span>
    </div>

    {tab === "config" ? <>
      <section className="distillation-config glass-card">
          <header className="distillation-config-header"><div className="distillation-config-title"><strong>蒸馏配置</strong><div className="distillation-segmented"><button aria-pressed={configMode === "quick"} className={configMode === "quick" ? "active" : ""} onClick={() => setConfigMode("quick")} type="button">快速模式</button><button aria-pressed={configMode === "advanced"} className={configMode === "advanced" ? "active" : ""} onClick={() => setConfigMode("advanced")} type="button">高级配置</button></div><span>{providers.length ? "使用已添加的供应商模型" : "请先在供应商中添加模型配置"}</span></div>
          <div className="distillation-config-tools">
            <label className="distillation-provider-select"><span>供应商</span><select aria-label="蒸馏供应商" value={providerId} onChange={(event) => { setProviderId(event.target.value); setModelId(providers.find((provider) => provider.id === event.target.value)?.models[0] ?? ""); }}><option value="">{providers.length ? "选择供应商" : "暂无已配置供应商"}</option>{providers.map((provider) => <option key={provider.id} value={provider.id}>{provider.name}</option>)}</select></label>
            <label className="distillation-provider-select"><span>模型</span><select aria-label="蒸馏模型" disabled={!currentProvider} value={modelId} onChange={(event) => setModelId(event.target.value)}><option value="">{currentProvider ? "选择模型" : "先选择供应商"}</option>{models.map((model) => <option key={model} value={model}>{model}</option>)}</select></label>
            <a href="#supplier" onClick={(event) => { event.preventDefault(); void actions.refreshRoute("supplier"); }}>管理模型</a></div>
        </header>

        {configMode === "advanced" ? <div className="distillation-advanced-panel"><label className="distillation-prompt"><span>提示词补充要求</span><textarea onChange={(event) => setPrompt(event.target.value)} placeholder="例如：提炼重复流程、保留失败处理与验证步骤，使用中文输出。" value={prompt} /></label></div> : null}

        <div className="distillation-config-row distillation-filter-row">
          <span className="distillation-row-label">选素材</span><div className="distillation-segmented compact"><button aria-pressed={materialMode === "session"} className={materialMode === "session" ? "active" : ""} onClick={() => setMaterialMode("session")} type="button">按会话</button><button aria-pressed={materialMode === "project"} className={materialMode === "project" ? "active" : ""} onClick={() => setMaterialMode("project")} type="button">按项目</button></div>
          <div className="distillation-chip-row">{(["today", "7d", "30d", "all"] as const).map((value) => <button aria-pressed={range === value} className={range === value ? "active" : ""} key={value} onClick={() => setRange(value)} type="button">{value === "today" ? "今天" : value === "7d" ? "近 7 天" : value === "30d" ? "近 30 天" : "全部"}</button>)}</div>
          <span className="distillation-material-count">范围内 {visibleSessions.length} 个会话</span>
        </div>

        <div className="distillation-config-row distillation-material-row">
          <span className="distillation-row-label">{materialMode === "project" ? "项目" : "会话"}</span>
          <div className="distillation-material-surface">
            <div className="distillation-session-list">
              {materialMode === "session" ? visibleSessions.map((session) => sessionRow(session, Boolean(selected[sessionKey(session)]), () => toggleSession(session), () => setPreview(session))) : sessionGroups.map(([projectName, group]) => {
                const allSelected = group.every((session) => selected[sessionKey(session)]);
                const sources = [...new Set(group.map((session) => agentLabel(session.agent)))];
                const tokens = group.reduce((sum, item) => sum + item.tokens, 0);
                const latest = group.reduce((value, session) => Math.max(value, timestampValue(session.updatedAt)), Number.NEGATIVE_INFINITY);
                return <button aria-pressed={allSelected} className={`distillation-project-row${allSelected ? " selected" : ""}`} key={projectName} onClick={() => toggleProject(group)} type="button"><span className="distillation-check">{allSelected ? <Check className="h-3.5 w-3.5" /> : null}</span><FolderOpen aria-hidden="true" className="h-4 w-4" /><span className="distillation-project-copy"><strong>{projectName}</strong><small>{group.length} 个会话 · {sources.join("、")} · ~{formatTokens(tokens)} · {dateLabel(latest)}</small></span></button>;
              })}
              {materialMode === "project" ? ungroupedSessions.map((session) => sessionRow(session, Boolean(selected[sessionKey(session)]), () => toggleSession(session), () => setPreview(session))) : null}
              {!visibleSessions.length ? <div className="distillation-empty">筛选条件下没有本地会话。刷新本地会话数据后再试。</div> : null}
            </div>
            <footer className="distillation-material-footer"><div><button className="text-button" onClick={selectVisible} type="button">全选当前列表</button><button className="text-button" onClick={clearSelection} type="button">清空选择</button><span>已选 {selectedCount} 个会话</span></div><span>正文只在本次运行时读取</span></footer>
          </div>
        </div>

        <div className="distillation-config-row distillation-output-row"><span className="distillation-row-label">出产物</span><AssetGroup destination="Skill 库" items={OUTPUT_TYPES.filter((item) => item.group === "capability")} onChange={setKind} title="能力资产（关于‘事’）" value={kind} /><AssetGroup destination="记忆库" items={OUTPUT_TYPES.filter((item) => item.group === "memory")} onChange={setKind} title="记忆资产（关于‘人’）" value={kind} /></div>
        <footer className="distillation-run-bar">
          <span className="distillation-row-label">跑蒸馏</span><span className="distillation-run-hint"><Clock3 className="h-4 w-4" />{selectedCount ? `已选 ${selectedCount} 个会话 · ${outputKindLabel}` : "请先从材料库选择会话"}</span>
          {activeTask ? <><div className="distillation-task-progress"><div><span>{task?.phase === "reading-material" ? "读取材料" : task?.phase === "calling-model" ? "调用模型" : task?.phase === "quality-check" ? "质量检查" : task?.phase === "persisting-candidate" ? "保存候选" : "准备蒸馏"}</span><strong>{task?.percent ?? 0}%</strong></div><div className="progress-track"><i style={{ width: `${task?.percent ?? 0}%` }} /></div></div><Button onClick={() => void cancel()} variant="outline"><X className="h-4 w-4" />取消任务</Button></> : <Button className="distillation-run-button" disabled={!selectedCount || !providerId || !modelId} onClick={() => void start()}><Send className="h-4 w-4" />一键蒸馏 {outputKindLabel}</Button>}
        </footer>
      </section>
      {notice ? <p className="distillation-notice" role="status">{notice}</p> : null}
    </> : <div className="distillation-history">
      {candidates.length ? candidates.map((candidate) => <article key={candidate.id}>
        <div className="distillation-history-icon"><BrainCircuit className="h-4 w-4" /></div>
        <div className="distillation-history-copy"><div className="distillation-history-heading"><strong>{candidate.title || candidate.summary}</strong><span className={`distillation-status ${candidate.status}`}>{candidate.status === "pending" ? "待审批" : candidate.status === "approved" ? "已审批" : candidate.status === "saved" ? "已入库" : candidate.status === "cancelled" ? "已取消" : candidate.status}</span></div><small>{candidate.kind || "memory"} · {candidate.mode || "offline"} · {dateLabel(candidate.createdAt)} · {agentLabel(candidate.agent)}</small>{candidate.sourceRefs?.length ? <small>来源：{candidate.sourceRefs.map((source) => `${agentLabel(source.agent)} · ${source.project || source.sessionId} · ${source.startIndex + 1}-${source.endIndex + 1}`).join("；")}</small> : null}<p>{candidate.output || candidate.summary}</p></div>
        <div className="distillation-history-actions">{candidate.status === "pending" ? <><button onClick={() => void approve(candidate)} type="button">审批入库</button><button className="secondary" onClick={() => void cancelCandidate(candidate)} type="button">取消</button></> : null}{candidate.status === "approved" ? <button onClick={() => void save(candidate)} type="button">写入 {OUTPUT_TYPES.find((item) => item.id === candidate.kind)?.label ?? "目标库"}</button> : null}</div>
      </article>) : <div className="distillation-empty">还没有蒸馏候选。选择会话、配置产物类型后开始蒸馏。</div>}
    </div>}

    {preview ? <div className="distillation-preview-backdrop" onClick={() => setPreview(null)} role="presentation"><section aria-label="会话片段预览" aria-modal="true" className="distillation-preview-dialog" onClick={(event) => event.stopPropagation()} role="dialog"><header><div><strong>{preview.title || preview.sessionId}</strong><small>{agentLabel(preview.agent)} · {preview.project || "未命名项目"} · {preview.events} 轮</small></div><button aria-label="关闭预览" onClick={() => setPreview(null)} type="button"><X className="h-4 w-4" /></button></header>{transcript === undefined ? <div className="distillation-preview-loading"><LoaderCircle className="h-4 w-4 spin" />正在读取会话正文…</div> : transcript ? <><div className="distillation-range-controls"><label>起始消息<select onChange={(event) => setRangeBoundary("startIndex", Number(event.target.value))} value={previewSelection?.startIndex ?? 0}>{transcriptMessages.map((message, index) => <option key={index} value={index}>{index + 1}. {message.role} · {message.text.slice(0, 54) || "空消息"}</option>)}</select></label><label>结束消息<select onChange={(event) => setRangeBoundary("endIndex", Number(event.target.value))} value={previewSelection?.endIndex ?? Math.max(0, transcriptMessages.length - 1)}>{transcriptMessages.map((message, index) => <option key={index} value={index}>{index + 1}. {message.role} · {message.text.slice(0, 54) || "空消息"}</option>)}</select></label><span>范围含首尾 · {Math.max(0, (previewSelection?.endIndex ?? transcriptMessages.length - 1) - (previewSelection?.startIndex ?? 0) + 1)} 条消息</span></div><div className="distillation-transcript">{transcriptMessages.map((message, index) => { const startIndex = previewSelection?.startIndex ?? 0; const endIndex = previewSelection?.endIndex ?? transcriptMessages.length - 1; const included = index >= startIndex && index <= endIndex; return <article className={included ? "included" : ""} key={`${index}-${message.timestamp ?? ""}`}><small>{message.role} · {message.timestamp ? dateLabel(message.timestamp) : `消息 ${index + 1}`}</small><p>{message.text || "（空消息）"}</p></article>; })}</div><footer><span>{previewSelection?.startIndex != null ? "已选择消息片段" : "默认包含整个会话"}</span><Button onClick={() => { setSelected((current) => ({ ...current, [sessionKey(preview)]: current[sessionKey(preview)] ?? { agent: preview.agent, sessionId: preview.sessionId } })); setPreview(null); }}><Check className="h-4 w-4" />确认材料</Button></footer></> : <div className="distillation-empty">没有可预览的会话正文。</div>}</section></div> : null}
  </section>;
}

export function ClientsEnhancementScreen({ actions, distillationWorkbench }: { actions: AppActions; distillationWorkbench?: DistillationWorkbenchResult | null }) {
  return <DistillationWorkbench actions={actions} data={distillationWorkbench ?? null} />;
}
