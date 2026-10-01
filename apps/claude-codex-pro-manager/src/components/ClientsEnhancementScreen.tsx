import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowRight, ChevronLeft, ChevronRight, FlaskConical, FolderOpen, LoaderCircle, PackageCheck, Trash2, X, Zap } from "lucide-react";

import "./distillation/distillation.css";
import type { AppActions } from "@/lib/actions";
import {
  filterDistillationSessions,
  toggleMaterialSelection,
  toggleProjectSelection,
  type DistillationMaterialGranularity,
  type DistillationTimeRange,
} from "@/lib/distillation-materials";
import type { DistillationCandidate, DistillationRunResult, DistillationWorkbenchResult, DistillationWorkbenchSession } from "@/types";

import { AgentIcon, EST_TOKENS_PER_TURN, agentLabel, candidateRefs, formatDateTime, formatTokens, materialKeyOf, resolveCandidateSource, sessionTurns, timestampValue } from "./distillation/common";
import { ExpCard } from "./distillation/ExpCard";
import { MaterialDrawer, MaterialPicker, type SegmentRef } from "./distillation/MaterialDrawer";
import { ModelSelect, type DistillModelOption } from "./distillation/ModelSelect";
import { OUT_GROUPS, OUT_TYPES, PROMPT_PRESETS, kindMeta, outTypeMeta, type OutTypeId } from "./distillation/out-types";

// 蒸馏工作台：交互、文案与状态机对齐 AITracker DistillationPage / DistillConfig
// （经版权方授权移植），视觉使用 CCP 液态玻璃 token（distillation.css）。
// 出产物分两组：能力资产 → Skill 库，记忆资产 → 记忆库（见 out-types.ts）。

const HIST_PAGE = 10;
const OFFLINE_MODEL_ID = "offline";
const MODEL_STORAGE_KEY = "ccp.distillation.model";
const TASK_STORAGE_KEY = "ccp.distillation.active-task";
const TERMINAL_PHASES = ["completed", "failed", "cancelled"];

type Mode = "quick" | "pro";
type Toast = { id: number; tone: "ok" | "error"; text: string; action?: { label: string; run: () => void } };

function readStorage(key: string) {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key: string, value: string | null) {
  try {
    if (value === null) window.localStorage.removeItem(key);
    else window.localStorage.setItem(key, value);
  } catch {
    // Storage is a convenience only; the task keeps running server-side.
  }
}

function RowLabel({ children }: { children: React.ReactNode }) {
  return <span className="dw-row-label">{children}</span>;
}

function Chip({ on, onClick, children, icon }: { on?: boolean; onClick: () => void; children: React.ReactNode; icon?: React.ReactNode }) {
  return <button aria-pressed={on} className={`dw-chip${on ? " on" : ""}`} onClick={onClick} type="button">{icon}{children}</button>;
}

function DistillMetrics({ selectedCount, estTokens, runs, approved, busy }: { selectedCount: number; estTokens: number; runs: number; approved: number; busy: boolean }) {
  const cards = [
    { k: "已选素材", v: `${selectedCount}`, s: `~${formatTokens(estTokens)} tokens`, c: "var(--dw-chart-1)" },
    { k: "素材 Token", v: formatTokens(estTokens), s: "本次输入预估", c: "var(--dw-chart-4)" },
    { k: "蒸馏次数", v: `${runs}`, s: busy ? "进行中…" : "累计", c: "var(--dw-chart-2)" },
    { k: "已入库", v: `${approved}`, s: "保存为 Skill", c: "var(--dw-chart-3)" },
  ];
  return (
    <section aria-label="蒸馏统计" className="dw-metrics">
      {cards.map((card) => (
        <div className="dw-glass dw-metric" key={card.k}>
          <small>{card.k}</small>
          <strong style={{ color: card.c }}>{card.v}</strong>
          <span>{card.s}</span>
        </div>
      ))}
    </section>
  );
}

function RunningExpCard({ color, kindLabel, modelLabel, segCount, sources, progress: serverProgress, startedAt }: { color: string; kindLabel: string; modelLabel: string; segCount: number; sources: string; progress: number; startedAt: number }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 120);
    return () => window.clearInterval(timer);
  }, []);
  const elapsed = Math.max(0, Math.floor((now - startedAt) / 1000));
  const visible = serverProgress > 0 ? serverProgress : Math.min(0.92, (now - startedAt) / 30_000);
  return (
    <article className="dw-glass dw-running" style={{ ["--dw-accent" as string]: color }}>
      <div className="dw-running-meta">
        <span className="dw-kind-chip">{kindLabel}</span>
        <span>{formatDateTime(startedAt)}</span>
        <span>· {modelLabel}</span>
      </div>
      <div className="dw-running-material">素材：{segCount} 段 · {sources}</div>
      <div className="dw-progress"><i style={{ width: `${Math.round(visible * 100)}%` }} /></div>
      <p className="dw-running-copy">蒸馏中… {Math.round(visible * 100)}% · 已耗时 {elapsed}s</p>
    </article>
  );
}

function OutTypePicker({ value, onChange }: { value: OutTypeId; onChange: (value: OutTypeId) => void }) {
  return (
    <div className="dw-out-groups">
      {OUT_GROUPS.map((group) => {
        const items = OUT_TYPES.filter((meta) => meta.group === group.id);
        const active = items.some((meta) => meta.id === value);
        return (
          <div className={`dw-out-group${active ? " active" : ""}`} key={group.id}>
            <div className="dw-out-group-head">
              <strong>{group.label}</strong>
              <span className="dw-tag">→ {group.dest}</span>
            </div>
            <div className="dw-out-group-items">
              {items.map((meta) => (
                <button aria-pressed={meta.id === value} className={`dw-chip${meta.id === value ? " on" : ""}`} key={meta.id} onClick={() => onChange(meta.id)} title={meta.hint} type="button">{meta.label}</button>
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}

function Pagination({ page, pageCount, onChange }: { page: number; pageCount: number; onChange: (page: number) => void }) {
  return (
    <div className="dw-glass dw-pagination">
      <button aria-label="上一页" disabled={page <= 1} onClick={() => onChange(page - 1)} type="button"><ChevronLeft aria-hidden="true" />上一页</button>
      <span>{page} / {pageCount}</span>
      <button aria-label="下一页" disabled={page >= pageCount} onClick={() => onChange(page + 1)} type="button">下一页<ChevronRight aria-hidden="true" /></button>
    </div>
  );
}

function DistillationWorkbench({ actions, data }: { actions: AppActions; data: DistillationWorkbenchResult | null }) {
  const sessions = useMemo(
    () => [...(data?.sessions ?? [])].sort((left, right) => timestampValue(right.updatedAt) - timestampValue(left.updatedAt)),
    [data?.sessions],
  );
  const providers = useMemo(() => data?.providers ?? [], [data?.providers]);
  const skillAgents = useMemo(() => data?.skillAgents ?? [], [data?.skillAgents]);
  const candidates = useMemo(
    () => [...(data?.candidates ?? [])].filter((item) => item.status !== "cancelled").sort((left, right) => timestampValue(right.createdAt) - timestampValue(left.createdAt)),
    [data?.candidates],
  );

  const modelOptions = useMemo<DistillModelOption[]>(() => {
    const real = providers.flatMap((provider) => provider.models.map((model) => ({
      id: `${provider.id}::${model}`,
      providerId: provider.id,
      model,
      label: model,
      sub: provider.name,
      vendor: provider.vendor || provider.name,
      ok: (provider.status ?? "ok") === "ok",
      active: Boolean(provider.active) && (provider.activeModel ? provider.activeModel === model : true),
    })));
    return [...real, { id: OFFLINE_MODEL_ID, providerId: OFFLINE_MODEL_ID, model: OFFLINE_MODEL_ID, label: "离线回退（确定性）", vendor: "离线", offline: true, ok: true }];
  }, [providers]);
  const hasRealModel = modelOptions.some((option) => !option.offline);

  const [mode, setMode] = useState<Mode>("quick");
  const [outType, setOutType] = useState<OutTypeId>("skill");
  const [view, setView] = useState<"config" | "result">("config");
  const [timeRange, setTimeRange] = useState<DistillationTimeRange>("all");
  const [granularity, setGranularity] = useState<DistillationMaterialGranularity>("session");
  const [modelId, setModelId] = useState(() => readStorage(MODEL_STORAGE_KEY) ?? "");
  const [promptText, setPromptText] = useState("");
  const [segments, setSegments] = useState<SegmentRef[]>([]);
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [histPage, setHistPage] = useState(1);
  const [viewId, setViewId] = useState<string | null>(null);
  const [selectedCandidateIds, setSelectedCandidateIds] = useState<Set<string>>(() => new Set());
  const [task, setTask] = useState<DistillationRunResult | null>(null);
  const [taskStartedAt, setTaskStartedAt] = useState(() => Date.now());
  const [runningMeta, setRunningMeta] = useState<{ kind: string; modelLabel: string; segCount: number; sources: string } | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const toastSeq = useRef(0);

  const distilling = Boolean(task && !TERMINAL_PHASES.includes(task.phase));

  const pushToast = useCallback((toast: Omit<Toast, "id">) => {
    const id = ++toastSeq.current;
    setToasts((current) => [...current.slice(-2), { ...toast, id }]);
    window.setTimeout(() => setToasts((current) => current.filter((item) => item.id !== id)), 6000);
  }, []);

  // Model resolution (AITracker: "active profile, else first saved, else offline"):
  // 1. the user's last explicit pick, if that supplier/model still exists;
  // 2. the supplier CCP is currently running, with its current model;
  // 3. the first usable configured model; 4. the offline fallback.
  useEffect(() => {
    // Wait for the workbench payload; resolving against an empty provider list
    // would always land on the offline fallback.
    if (!data) return;
    if (modelOptions.some((option) => option.id === modelId)) return;
    const real = modelOptions.filter((option) => !option.offline);
    const next = real.find((option) => option.active && option.ok)
      ?? real.find((option) => option.active)
      ?? real.find((option) => option.ok)
      ?? real[0];
    setModelId(next?.id ?? OFFLINE_MODEL_ID);
  }, [data, modelId, modelOptions]);
  // Only an explicit user pick is remembered; automatic defaults keep
  // following the supplier CCP is currently running.
  const pickModel = useCallback((id: string) => {
    setModelId(id);
    writeStorage(MODEL_STORAGE_KEY, id);
  }, []);

  const materialSessions = useMemo(() => filterDistillationSessions(sessions, timeRange), [sessions, timeRange]);
  // A range is a selection boundary: refs outside the visible set are dropped.
  useEffect(() => {
    const available = new Set(materialSessions.map(materialKeyOf));
    setSelected((current) => {
      const next = new Set([...current].filter((key) => available.has(key)));
      return next.size === current.size ? current : next;
    });
  }, [materialSessions]);
  // A segment whose session leaves the selection is meaningless.
  useEffect(() => {
    if (segments.length === 0) return;
    setSegments((current) => {
      const next = current.filter((segment) => selected.has(materialKeyOf(segment)));
      return next.length === current.length ? current : next;
    });
  }, [selected, segments]);

  const selectedItems = useMemo(() => sessions.filter((item) => selected.has(materialKeyOf(item))), [sessions, selected]);
  const estTokens = selectedItems.reduce((sum, item) => sum + sessionTurns(item) * EST_TOKENS_PER_TURN, 0);
  const selectedOption = modelOptions.find((option) => option.id === modelId) ?? modelOptions[0];
  const pickedEmpty = mode === "pro" ? segments.length === 0 : selected.size === 0;
  const canRun = !distilling && !pickedEmpty && hasRealModel;
  const runs = candidates.length;
  const approved = candidates.filter((item) => item.status === "approved" || item.status === "saved").length;

  const histPageCount = Math.max(1, Math.ceil(runs / HIST_PAGE));
  const curHistPage = Math.min(histPage, histPageCount);
  const winStart = (curHistPage - 1) * HIST_PAGE;
  const shownCandidates = candidates.slice(winStart, winStart + HIST_PAGE);

  const finishTask = useCallback((result: DistillationRunResult) => {
    writeStorage(TASK_STORAGE_KEY, null);
    void actions.loadDistillationWorkbench();
    if (result.phase === "completed") {
      const candidate = result.candidate;
      const label = kindMeta(candidate?.kind ?? result.kind).label;
      if (candidate) {
        setViewId(candidate.id);
        setHistPage(1);
      }
      setView("result");
      pushToast({
        tone: "ok",
        text: `蒸馏完成，已生成${label} · 见蒸馏历史`,
        action: {
          label: "查看结果",
          run: () => {
            setView("result");
            if (candidate) setViewId(candidate.id);
          },
        },
      });
    } else if (result.phase === "failed") {
      pushToast({ tone: "error", text: result.detail || result.message || "蒸馏任务失败。" });
    } else {
      pushToast({ tone: "error", text: "蒸馏任务已取消。" });
    }
  }, [actions, pushToast]);

  // Poll the active task (500 ms) until a terminal phase.
  useEffect(() => {
    if (!task?.taskId || !distilling) return;
    let stopped = false;
    const timer = window.setInterval(async () => {
      const result = await actions.queryDistillationTask(task.taskId);
      if (stopped || !result) return;
      setTask(result);
      if (TERMINAL_PHASES.includes(result.phase)) {
        stopped = true;
        window.clearInterval(timer);
        finishTask(result);
      }
    }, 500);
    return () => {
      stopped = true;
      window.clearInterval(timer);
    };
  }, [actions, distilling, finishTask, task?.taskId]);

  // Re-entering the page resumes a task that is still running server-side.
  useEffect(() => {
    const stored = readStorage(TASK_STORAGE_KEY);
    if (!stored) return;
    let disposed = false;
    void actions.queryDistillationTask(stored).then((result) => {
      if (disposed) return;
      if (!result || !result.taskId || TERMINAL_PHASES.includes(result.phase)) {
        writeStorage(TASK_STORAGE_KEY, null);
        return;
      }
      const startedAt = timestampValue(result.startedAt);
      setTaskStartedAt(Number.isFinite(startedAt) ? startedAt : Date.now());
      setRunningMeta({ kind: result.kind ?? "skill", modelLabel: result.modelId || "模型", segCount: result.selectionCount ?? 0, sources: "" });
      setTask(result);
      setView("result");
    });
    return () => {
      disposed = true;
    };
  }, [actions]);

  const toggle = (item: DistillationWorkbenchSession) => setSelected((current) => toggleMaterialSelection(current, materialKeyOf(item)) as Set<string>);
  const toggleProject = (items: readonly DistillationWorkbenchSession[]) => setSelected((current) => toggleProjectSelection(current, items.map(materialKeyOf)) as Set<string>);

  function handleSegmentsChange(next: SegmentRef[]) {
    const nextSelected = new Set(selected);
    let changed = false;
    for (const segment of next) {
      const key = materialKeyOf(segment);
      if (nextSelected.has(key) || !sessions.some((item) => materialKeyOf(item) === key)) continue;
      nextSelected.add(key);
      changed = true;
    }
    if (changed) setSelected(nextSelected);
    setSegments(next.filter((segment) => nextSelected.has(materialKeyOf(segment))));
  }

  function buildPrompt(userPrompt: string | undefined) {
    const parts = [outTypeMeta(outType).instruction, userPrompt?.trim()].filter(Boolean);
    return parts.length ? parts.join("；") : undefined;
  }

  async function runDistillation(refs: Array<{ agent: string; sessionId: string; startIndex?: number; endIndex?: number }>) {
    if (refs.length === 0 || distilling) return;
    const option = selectedOption;
    const offline = !option || option.offline;
    const meta = outTypeMeta(outType);
    const result = await actions.runDistillationWorkbench({
      selections: refs,
      providerId: offline ? OFFLINE_MODEL_ID : option.providerId,
      modelId: offline ? OFFLINE_MODEL_ID : option.model,
      kind: meta.kind,
      mode: offline ? "offline" : "model",
      prompt: buildPrompt(mode === "pro" ? promptText : undefined),
    });
    if (!result || result.status === "failed" || result.phase === "failed") {
      pushToast({ tone: "error", text: result?.detail || result?.message || "蒸馏任务启动失败。" });
      return;
    }
    const startedAt = timestampValue(result.startedAt);
    setTaskStartedAt(Number.isFinite(startedAt) ? startedAt : Date.now());
    setRunningMeta({
      kind: meta.kind,
      modelLabel: offline ? "离线回退（确定性）" : option.label,
      segCount: refs.length,
      sources: [...new Set(refs.map((ref) => agentLabel(ref.agent)))].join(" / "),
    });
    writeStorage(TASK_STORAGE_KEY, result.taskId);
    setTask(result);
    if (TERMINAL_PHASES.includes(result.phase)) finishTask(result);
  }

  function handleStart() {
    if (!hasRealModel) return;
    setViewId(null);
    setView("result");
    const segmentByKey = new Map(segments.map((segment) => [materialKeyOf(segment), segment]));
    void runDistillation(selectedItems.map((item) => {
      const segment = segmentByKey.get(materialKeyOf(item));
      return segment
        ? { agent: item.agent, sessionId: item.sessionId, startIndex: segment.startIndex, endIndex: segment.endIndex }
        : { agent: item.agent, sessionId: item.sessionId };
    }));
  }

  function handleRegenerate(candidate: DistillationCandidate) {
    setViewId(null);
    setView("result");
    void runDistillation(candidateRefs(candidate).map((ref) => (
      candidate.sourceRefs?.length && ref.endIndex > ref.startIndex
        ? { agent: ref.agent, sessionId: ref.sessionId, startIndex: ref.startIndex, endIndex: ref.endIndex }
        : { agent: ref.agent, sessionId: ref.sessionId }
    )));
  }

  async function removeCandidates(ids: readonly string[]) {
    if (ids.length === 0) return;
    await actions.deleteDistillationCandidates(ids.slice(0, 100));
    setSelectedCandidateIds(new Set());
    setViewId((current) => (current && ids.includes(current) ? null : current));
  }

  const pickPreset = (text: string) => setPromptText((current) => (
    current.includes(text) ? current : (current.trim() ? `${current.trim()}；` : "") + text
  ));
  const goSupplier = () => void actions.refreshRoute("supplier");

  const typeMeta = outTypeMeta(outType);
  const statusLabel = selectedOption?.offline ? null : selectedOption?.ok ? "自有模型 · 已连接" : "自有模型 · 未配置 Endpoint";
  const segsBySession = useMemo(() => {
    const map = new Map<string, number>();
    for (const segment of segments) map.set(materialKeyOf(segment), (map.get(materialKeyOf(segment)) ?? 0) + 1);
    return map;
  }, [segments]);
  const titleByKey = useMemo(() => new Map(sessions.map((item) => [materialKeyOf(item), item.title])), [sessions]);
  const runHint = pickedEmpty
    ? mode === "quick"
      ? granularity === "project" ? "请先选择一个项目" : "请先勾选至少一场会话"
      : "请先在素材库选择会话选段"
    : mode === "quick"
      ? `${selectedItems.length} 场会话 · ~${formatTokens(estTokens)} tokens · ${selectedOption?.offline ? "离线回退" : selectedOption?.label ?? "模型"} 与推荐提示词，无需配置`
      : `${segments.length} 段素材 · ~${formatTokens(estTokens)} tokens`;
  const progress = task ? (task.phase === "completed" ? 1 : Math.min(0.92, (task.percent ?? 0) / 100)) : 0;

  const viewTabs = (
    <div className="dw-toolbar">
      <Chip icon={<FlaskConical aria-hidden="true" />} on={view === "config"} onClick={() => setView("config")}>蒸馏配置</Chip>
      <Chip icon={<PackageCheck aria-hidden="true" />} on={view === "result"} onClick={() => setView("result")}>
        蒸馏历史{runs > 0 ? <span className="dw-badge">{runs}</span> : null}
      </Chip>
    </div>
  );

  return (
    <section aria-label="蒸馏工作台" className="dw-root distillation-workbench">
      {view === "config" ? (
        <>
          <DistillMetrics approved={approved} busy={distilling} estTokens={estTokens} runs={runs} selectedCount={mode === "pro" ? segments.length : selected.size} />
          <div className="dw-glass dw-viewbar">
            {viewTabs}
            <span className="dw-viewbar-hint">选素材、配参数、跑蒸馏</span>
          </div>

          <section className="dw-glass dw-config">
            <header className="dw-config-head">
              <h2>蒸馏配置</h2>
              <div className="dw-toolbar">
                <Chip icon={<Zap aria-hidden="true" />} on={mode === "quick"} onClick={() => setMode("quick")}>快速模式</Chip>
                <Chip icon={<FlaskConical aria-hidden="true" />} on={mode === "pro"} onClick={() => setMode("pro")}>高级配置</Chip>
              </div>
              {statusLabel ? <span className="dw-config-status" title={statusLabel}>{statusLabel}</span> : <span className="dw-config-status" />}
              {mode === "quick" && hasRealModel ? <div className="dw-config-model"><ModelSelect onChange={pickModel} onManage={goSupplier} options={modelOptions} value={modelId} /></div> : null}
              <button className="dw-chip" onClick={goSupplier} type="button">管理模型</button>
            </header>

            <div className="dw-rows">
              <div className="dw-row">
                <RowLabel>选素材</RowLabel>
                {mode === "quick" ? (
                  <>
                    <div className="dw-toolbar">
                      <Chip on={granularity === "session"} onClick={() => setGranularity("session")}>按会话</Chip>
                      <Chip on={granularity === "project"} onClick={() => setGranularity("project")}>按项目</Chip>
                    </div>
                    <span className="dw-divider" />
                    <div className="dw-toolbar">
                      {([["today", "今天"], ["7", "近 7 天"], ["30", "近 30 天"], ["all", "全部"]] as const).map(([value, label]) => (
                        <Chip key={value} on={timeRange === value} onClick={() => setTimeRange(value)}>{label}</Chip>
                      ))}
                    </div>
                    <span className="dw-meta">
                      范围内 {materialSessions.length} 个会话{granularity === "session" && selected.size > 0 ? ` · 已选 ${selected.size}` : ""}
                    </span>
                    {granularity === "session" && selected.size > 0 ? <button className="dw-link" onClick={() => setSelected(new Set())} type="button">清空</button> : null}
                  </>
                ) : null}
              </div>

              <div className="dw-row top">
                <RowLabel>{granularity === "project" ? "项目" : "会话"}</RowLabel>
                <div className="dw-row-body">
                  {mode === "quick" ? (
                    <MaterialPicker granularity={granularity} onToggle={toggle} onToggleProject={toggleProject} selected={selected} sessions={materialSessions} />
                  ) : (
                    <div className={`dw-material-box${segments.length ? " filled" : ""}`}>
                      <div className="dw-material-box-head">
                        <span className="dw-material-box-icon"><FolderOpen aria-hidden="true" /></span>
                        <div className="dw-material-box-copy">
                          {segments.length === 0 ? (
                            <>
                              <strong>还没有选择素材</strong>
                              <small>在素材库里跨会话勾选对话区间（Hover 消息可设起点/终点）</small>
                            </>
                          ) : (
                            <>
                              <strong>{segsBySession.size} 场会话 · {segments.length} 条对话</strong>
                              <small>预估输入 ~{formatTokens(estTokens)} tokens</small>
                            </>
                          )}
                        </div>
                        <button className="dw-accent-button" onClick={() => setDrawerOpen(true)} type="button">
                          {segments.length === 0 ? <><FolderOpen aria-hidden="true" />打开素材库</> : "继续添加"}
                        </button>
                        {segments.length ? <button className="dw-soft-button" onClick={() => setSegments([])} type="button"><Trash2 aria-hidden="true" />清空</button> : null}
                      </div>
                      {segsBySession.size > 0 ? (
                        <ul className="dw-session-chips">
                          {[...segsBySession.entries()].slice(0, 8).map(([key, count]) => (
                            <li key={key}>
                              <AgentIcon agent={key.split(":")[0]!} />
                              <span>{titleByKey.get(key) || key.split(":").slice(1).join(":")}</span>
                              <em>{count} 条</em>
                            </li>
                          ))}
                          {segsBySession.size > 8 ? <li className="more">+{segsBySession.size - 8}</li> : null}
                        </ul>
                      ) : null}
                    </div>
                  )}
                </div>
              </div>

              {mode === "pro" ? (
                <>
                  <div className="dw-row">
                    <RowLabel>模型</RowLabel>
                    <div className="dw-row-body">
                      <ModelSelect onChange={pickModel} onManage={goSupplier} options={modelOptions} value={modelId} />
                    </div>
                  </div>
                  <div className="dw-row top">
                    <RowLabel>提示词预设</RowLabel>
                    <div className="dw-row-body dw-prompt">
                      <div className="dw-presets">
                        {PROMPT_PRESETS.map((preset) => (
                          <button key={preset.id} onClick={() => pickPreset(preset.text)} type="button">+ {preset.label}</button>
                        ))}
                        {promptText.trim() ? <button className="dw-link" onClick={() => setPromptText("")} type="button">清空</button> : null}
                      </div>
                      <div className="dw-prompt-box">
                        <textarea
                          aria-label="自定义蒸馏提示词"
                          onChange={(event) => setPromptText(event.target.value)}
                          onKeyDown={(event) => {
                            if ((event.metaKey || event.ctrlKey) && event.key === "Enter" && canRun) {
                              event.preventDefault();
                              handleStart();
                            }
                          }}
                          placeholder="自定义蒸馏提示词…（⌘↵ 运行）"
                          rows={3}
                          value={promptText}
                        />
                        <span>{promptText.length} 字 · ⌘↵ 运行</span>
                      </div>
                    </div>
                  </div>
                </>
              ) : null}

              <div className="dw-row top">
                <RowLabel>出产物</RowLabel>
                <OutTypePicker onChange={setOutType} value={outType} />
              </div>

              <div className="dw-row last">
                <RowLabel>跑蒸馏</RowLabel>
                <button className="dw-run-button" disabled={distilling || (!canRun && hasRealModel)} onClick={handleStart} type="button">
                  {distilling ? <LoaderCircle aria-hidden="true" className="spin" /> : mode === "quick" ? <Zap aria-hidden="true" /> : <FlaskConical aria-hidden="true" />}
                  {distilling ? "蒸馏中…" : mode === "quick" ? `一键蒸馏 ${typeMeta.label}` : `开始蒸馏 ${typeMeta.label}`}
                </button>
                <span className="dw-run-hint">
                  {hasRealModel ? runHint : <button className="dw-warn-link" onClick={goSupplier} type="button">蒸馏需配置模型。去配置 →</button>}
                </span>
              </div>
            </div>
          </section>
        </>
      ) : (
        <>
          <div className="dw-glass dw-viewbar">
            {viewTabs}
            <span className="dw-viewbar-hint">共 {runs} 次蒸馏 · 已入库 {approved}</span>
            {distilling ? <span className="dw-running-flag"><LoaderCircle aria-hidden="true" className="spin" />蒸馏中…</span> : null}
            {distilling && task?.taskId ? <button className="dw-link" onClick={() => void actions.cancelDistillationTask(task.taskId).then((result) => result && setTask(result))} type="button">取消任务</button> : null}
            <button className="dw-link accent" onClick={() => void actions.refreshRoute("tools")} type="button">去 Skill 管理 <ArrowRight aria-hidden="true" /></button>
          </div>

          <div className="dw-history">
            {distilling ? (
              <RunningExpCard
                color={kindMeta(runningMeta?.kind ?? typeMeta.kind).color}
                kindLabel={kindMeta(runningMeta?.kind ?? typeMeta.kind).label}
                modelLabel={runningMeta?.modelLabel ?? selectedOption?.label ?? "offline"}
                progress={progress}
                segCount={runningMeta?.segCount ?? selectedItems.length}
                sources={runningMeta?.sources ?? ""}
                startedAt={taskStartedAt}
              />
            ) : null}

            {runs === 0 && !distilling ? (
              <div className="dw-glass dw-history-empty">
                <FlaskConical aria-hidden="true" />
                <p>左侧选好素材后点「一键蒸馏」</p>
                <small>产物与历史记录都会显示在这里</small>
              </div>
            ) : null}

            {shownCandidates.length > 0 ? (
              <>
                <div className="dw-glass dw-history-tools">
                  <label>
                    <input
                      checked={shownCandidates.every((item) => selectedCandidateIds.has(item.id))}
                      onChange={(event) => {
                        const checked = event.currentTarget.checked;
                        setSelectedCandidateIds((current) => {
                          const next = new Set(current);
                          for (const item of shownCandidates) {
                            if (checked) next.add(item.id);
                            else next.delete(item.id);
                          }
                          return next;
                        });
                      }}
                      type="checkbox"
                    />
                    已选 {selectedCandidateIds.size} 条
                  </label>
                  {selectedCandidateIds.size > 0 ? <button className="dw-danger-link" onClick={() => void removeCandidates([...selectedCandidateIds])} type="button">删除选中</button> : null}
                </div>
                <ul className="dw-glass dw-history-list">
                  {shownCandidates.map((candidate) => {
                    const badge = kindMeta(candidate.kind);
                    const resolved = resolveCandidateSource(candidate, sessions);
                    const open = viewId === candidate.id;
                    const saved = candidate.status === "approved" || candidate.status === "saved";
                    return (
                      <li className={open ? "open" : ""} key={candidate.id} style={{ ["--dw-accent" as string]: badge.color }}>
                        <div className="dw-history-row">
                          <input
                            aria-label={`选择 ${candidate.title || badge.label}`}
                            checked={selectedCandidateIds.has(candidate.id)}
                            onChange={(event) => {
                              const checked = event.currentTarget.checked;
                              setSelectedCandidateIds((current) => {
                                const next = new Set(current);
                                if (checked) next.add(candidate.id);
                                else next.delete(candidate.id);
                                return next;
                              });
                            }}
                            type="checkbox"
                          />
                          <button aria-expanded={open} className="dw-history-toggle" onClick={() => setViewId(open ? null : candidate.id)} type="button">
                            <span className="dw-history-icon">{saved ? <PackageCheck aria-hidden="true" /> : <FlaskConical aria-hidden="true" />}</span>
                            <span className="dw-history-copy">
                              <span className="dw-history-title">
                                <strong>{candidate.title || badge.label}</strong>
                                <span className="dw-kind-tag">{badge.label}</span>
                                <span className="dw-history-when">{candidateRefs(candidate).length} 段 · {formatDateTime(candidate.createdAt)}</span>
                                <span className={`dw-saved-chip${saved ? " on" : ""}`}>{saved ? "已入库" : "未保存"}</span>
                              </span>
                              <small>{resolved.sources.join(" / ")}</small>
                            </span>
                            <ChevronRight aria-hidden="true" className={`dw-history-chevron${open ? " open" : ""}`} />
                          </button>
                        </div>
                        {open ? (
                          <div className="dw-history-detail">
                            <ExpCard actions={actions} busy={distilling} candidate={candidate} onRegenerate={() => handleRegenerate(candidate)} sessions={sessions} skillAgents={skillAgents} />
                          </div>
                        ) : null}
                      </li>
                    );
                  })}
                </ul>
              </>
            ) : null}

            {runs > HIST_PAGE ? <Pagination onChange={setHistPage} page={curHistPage} pageCount={histPageCount} /> : null}
          </div>
        </>
      )}

      {drawerOpen ? <MaterialDrawer actions={actions} onClose={() => setDrawerOpen(false)} onSegmentsChange={handleSegmentsChange} segments={segments} sessions={sessions} /> : null}

      {toasts.length ? (
        <div aria-live="polite" className="dw-toasts">
          {toasts.map((toast) => (
            <div className={`dw-toast ${toast.tone}`} key={toast.id} role="status">
              <span>{toast.text}</span>
              {toast.action ? <button onClick={() => { toast.action?.run(); setToasts((current) => current.filter((item) => item.id !== toast.id)); }} type="button">{toast.action.label}</button> : null}
              <button aria-label="关闭提示" onClick={() => setToasts((current) => current.filter((item) => item.id !== toast.id))} type="button"><X aria-hidden="true" /></button>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}

export function ClientsEnhancementScreen({ actions, distillationWorkbench }: { actions: AppActions; distillationWorkbench?: DistillationWorkbenchResult | null }) {
  return <DistillationWorkbench actions={actions} data={distillationWorkbench ?? null} />;
}
