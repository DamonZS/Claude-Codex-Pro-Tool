import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Check, FolderOpen, LoaderCircle, Search, Trash2, X } from "lucide-react";

import type { AppActions } from "@/lib/actions";
import { groupDistillationSessionsByProject } from "@/lib/distillation-materials";
import type { DistillationSessionSelection, DistillationWorkbenchSession } from "@/types";

import {
  AgentIcon,
  EST_TOKENS_PER_TURN,
  agentColor,
  agentLabel,
  formatDate,
  formatDateTime,
  formatTokens,
  materialKeyOf,
  sessionProjectKey,
  sessionStartedAt,
  sessionTurns,
  timestampValue,
} from "./common";

/** A committed transcript window; indices are 0-based and inclusive. */
export type SegmentRef = Required<DistillationSessionSelection>;

type TranscriptMessage = { role: string; text: string; timestamp: string | null };
type TranscriptState = { messages: TranscriptMessage[]; status: "loading" | "ready" | "error" };
type Range = { s: number; e: number };

/**
 * Full-screen material library adapted from AITracker MaterialDrawer
 * (used with permission): left column lists sessions with filters and a
 * per-session join pill; the right pane picks a message range per session.
 */
export function MaterialDrawer({
  actions,
  sessions,
  segments,
  onSegmentsChange,
  onClose,
}: {
  actions: AppActions;
  sessions: readonly DistillationWorkbenchSession[];
  segments: readonly SegmentRef[];
  onSegmentsChange: (next: SegmentRef[]) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [source, setSource] = useState("all");
  const [range, setRange] = useState("all");
  const [proj, setProj] = useState("all");
  const [activeKey, setActiveKey] = useState<string | null>(() => (sessions[0] ? materialKeyOf(sessions[0]) : null));

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      document.body.style.overflow = previous;
    };
  }, [onClose]);

  const sources = useMemo(() => [...new Set(sessions.map((item) => item.agent))].sort(), [sessions]);
  const projects = useMemo(() => [...new Set(sessions.map(sessionProjectKey))].sort(), [sessions]);
  const filtered = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    const limitDays = range === "all" ? Infinity : Number(range);
    const cutoff = limitDays === Infinity ? -Infinity : Date.now() - limitDays * 86_400_000;
    return sessions.filter((item) => {
      if (source !== "all" && item.agent !== source) return false;
      if (proj !== "all" && sessionProjectKey(item) !== proj) return false;
      if (limitDays !== Infinity && timestampValue(sessionStartedAt(item)) < cutoff) return false;
      if (!needle) return true;
      return item.title.toLocaleLowerCase().includes(needle)
        || item.agent.toLocaleLowerCase().includes(needle)
        || sessionProjectKey(item).toLocaleLowerCase().includes(needle);
    });
  }, [sessions, query, source, range, proj]);
  const grouped = useMemo(() => {
    const map = new Map<string, { label: string; items: DistillationWorkbenchSession[] }>();
    for (const item of filtered) {
      const date = new Date(timestampValue(sessionStartedAt(item)));
      const dayKey = Number.isNaN(date.getTime())
        ? "0000-00-00"
        : `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
      const entry = map.get(dayKey) ?? { label: formatDate(sessionStartedAt(item)), items: [] };
      entry.items.push(item);
      map.set(dayKey, entry);
    }
    return [...map.entries()].sort(([a], [b]) => (a < b ? 1 : -1)).map(([, entry]) => entry);
  }, [filtered]);
  const byKey = useMemo(() => new Map(sessions.map((item) => [materialKeyOf(item), item])), [sessions]);
  const active = activeKey ? byKey.get(activeKey) ?? null : null;

  const cacheRef = useRef(new Map<string, TranscriptState>());
  const [activeTranscript, setActiveTranscript] = useState<TranscriptState>({ messages: [], status: "loading" });
  const activeKeyRef = useRef<string | null>(null);
  const pendingFullRef = useRef<string | null>(null);

  const [ranges, setRanges] = useState<Record<string, Range>>(() => {
    const init: Record<string, Range> = {};
    for (const segment of segments) init[materialKeyOf(segment)] = { s: segment.startIndex, e: segment.endIndex };
    return init;
  });
  const rangesRef = useRef(ranges);
  rangesRef.current = ranges;

  function updateRanges(mutate: (current: Record<string, Range>) => Record<string, Range>) {
    const next = mutate(rangesRef.current);
    rangesRef.current = next;
    setRanges(next);
    const list: SegmentRef[] = [];
    for (const [key, win] of Object.entries(next)) {
      const item = byKey.get(key);
      if (!item) continue;
      list.push({ agent: item.agent, sessionId: item.sessionId, startIndex: win.s, endIndex: win.e });
    }
    onSegmentsChange(list);
  }
  const commitRange = (key: string, s: number, e: number) => updateRanges((current) => ({ ...current, [key]: { s, e } }));

  useEffect(() => {
    const key = activeKey;
    const item = key ? byKey.get(key) : undefined;
    if (!key || !item) {
      activeKeyRef.current = null;
      return;
    }
    activeKeyRef.current = key;
    const cached = cacheRef.current.get(key);
    if (cached) {
      setActiveTranscript(cached);
      return;
    }
    const entry: TranscriptState = { messages: [], status: "loading" };
    cacheRef.current.set(key, entry);
    setActiveTranscript(entry);
    void actions.readDistillationTranscript({ agent: item.agent, sessionId: item.sessionId })
      .then((result) => {
        const transcript = result?.transcript;
        const next: TranscriptState = transcript
          ? { messages: [...transcript.messages], status: "ready" }
          : { messages: [], status: "error" };
        cacheRef.current.set(key, next);
        if (activeKeyRef.current === key) setActiveTranscript(next);
        if (pendingFullRef.current === key) {
          pendingFullRef.current = null;
          if (next.messages.length > 0) commitRange(key, 0, next.messages.length - 1);
        }
      })
      .catch(() => {
        const next: TranscriptState = { messages: [], status: "error" };
        cacheRef.current.set(key, next);
        if (activeKeyRef.current === key) setActiveTranscript(next);
        if (pendingFullRef.current === key) pendingFullRef.current = null;
      });
    // commitRange reads the latest ranges through rangesRef.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeKey, byKey, actions]);

  const inRange = (key: string, index: number) => {
    const r = ranges[key];
    return !!r && index >= r.s && index <= r.e;
  };
  const pickAt = (key: string, index: number) => updateRanges((current) => {
    const r = current[key];
    if (!r) return { ...current, [key]: { s: index, e: index } };
    if (r.s === r.e && r.s === index) {
      const next = { ...current };
      delete next[key];
      return next;
    }
    if (index < r.s) return { ...current, [key]: { s: index, e: r.e } };
    return { ...current, [key]: { s: r.s, e: index } };
  });
  const setStart = (key: string, index: number) => updateRanges((current) => {
    const r = current[key];
    return { ...current, [key]: { s: index, e: r ? Math.max(r.e, index) : index } };
  });
  const setEnd = (key: string, index: number) => updateRanges((current) => {
    const r = current[key];
    return { ...current, [key]: { s: r ? Math.min(r.s, index) : index, e: index } };
  });
  const clearChat = (key: string) => updateRanges((current) => {
    const next = { ...current };
    delete next[key];
    return next;
  });
  const selectAllOf = (key: string) => {
    const cached = cacheRef.current.get(key);
    if (cached && cached.status === "ready" && cached.messages.length > 0) {
      commitRange(key, 0, cached.messages.length - 1);
      return;
    }
    pendingFullRef.current = key;
    setActiveKey(key);
  };
  const pillClick = (item: DistillationWorkbenchSession) => {
    const key = materialKeyOf(item);
    if (ranges[key]) clearChat(key);
    else selectAllOf(key);
  };

  const rangeWindow = activeKey ? ranges[activeKey] : undefined;
  const allOn = rangeWindow !== undefined
    && activeTranscript.status === "ready"
    && activeTranscript.messages.length > 0
    && rangeWindow.s === 0
    && rangeWindow.e === activeTranscript.messages.length - 1;
  const rangeCount = Object.keys(ranges).length;
  const pickedSegs = Object.values(ranges).reduce((sum, r) => sum + (r.e - r.s + 1), 0);
  const pickedTokens = Object.keys(ranges).reduce((sum, key) => {
    const item = byKey.get(key);
    return item ? sum + sessionTurns(item) * EST_TOKENS_PER_TURN : sum;
  }, 0);

  const rightPane = active == null ? (
    <div className="dw-drawer-placeholder">
      <span><FolderOpen aria-hidden="true" /></span>
      <p>从左侧选择一场会话，再框定要蒸馏的对话区间</p>
    </div>
  ) : (
    <div className="dw-drawer-detail">
      <div className="dw-drawer-detail-head">
        <span className="dw-drawer-avatar" style={{ ["--dw-accent" as string]: agentColor(active.agent) }}><AgentIcon agent={active.agent} /></span>
        <div>
          <h3>{active.title || active.sessionId}</h3>
          <p>
            {agentLabel(active.agent)} · {sessionProjectKey(active)} · 共 {activeTranscript.messages.length} 条对话
            {rangeWindow ? ` · 已选 #${rangeWindow.s + 1} → #${rangeWindow.e + 1}（${rangeWindow.e - rangeWindow.s + 1} 条）` : ""}
          </p>
        </div>
        {rangeWindow ? <button className="dw-pill-button" onClick={() => clearChat(materialKeyOf(active))} type="button">清除区间</button> : null}
        <button className="dw-pill-button primary" onClick={() => (allOn ? clearChat(materialKeyOf(active)) : selectAllOf(materialKeyOf(active)))} style={{ ["--dw-accent" as string]: agentColor(active.agent) }} type="button">
          {allOn ? "取消全选" : "全选整场"}
        </button>
      </div>
      {activeTranscript.status === "ready" && activeTranscript.messages.length > 0 ? (
        <div className="dw-drawer-hints">
          <p>点击对话可依次设定开始 / 结束；也可用每条右侧的「起点 / 终点」精确指定</p>
          <p>支持跨会话蒸馏：切换左侧其它会话继续框选，选段会累加{rangeCount > 1 ? ` · 当前已跨 ${rangeCount} 场会话` : ""}</p>
        </div>
      ) : null}
      <div className="dw-drawer-messages">
        {activeTranscript.status === "loading" ? (
          <div className="dw-drawer-state"><LoaderCircle aria-hidden="true" className="spin" /></div>
        ) : activeTranscript.status === "error" || activeTranscript.messages.length === 0 ? (
          <div className="dw-drawer-state"><p>该会话没有可读取的对话正文</p></div>
        ) : (
          <ul>
            {activeTranscript.messages.map((message, index) => {
              const key = materialKeyOf(active);
              const mine = message.role === "user";
              const on = inRange(key, index);
              return (
                <li className={`dw-bubble-row${mine ? " mine" : ""}`} key={index} style={{ ["--dw-accent" as string]: agentColor(active.agent) }}>
                  <div className="dw-bubble-column">
                    <div className="dw-bubble-meta">
                      <span>#{index + 1}</span>
                      <span>{mine ? "我" : agentLabel(active.agent)}</span>
                      {rangeWindow?.s === index ? <em>开始</em> : null}
                      {rangeWindow?.e === index ? <em>结束</em> : null}
                    </div>
                    <button aria-pressed={on} className={`dw-bubble${on ? " in-range" : ""}`} onClick={() => pickAt(key, index)} type="button">
                      <span className="dw-bubble-check">{on ? <Check aria-hidden="true" /> : null}</span>
                      <span className="dw-bubble-text">{message.text.trim() || "（空消息）"}</span>
                    </button>
                    <div className="dw-bubble-pins">
                      <button onClick={() => setStart(key, index)} type="button">起点</button>
                      <button onClick={() => setEnd(key, index)} type="button">终点</button>
                    </div>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );

  return createPortal(
    <div aria-label="素材库 · 选择会话片段" aria-modal="true" className="dw-root dw-drawer-layer" role="dialog">
      <button aria-label="关闭" className="dw-drawer-backdrop" onClick={onClose} type="button" />
      <section className="dw-drawer">
        <header className="dw-drawer-header">
          <span className="dw-drawer-header-icon"><FolderOpen aria-hidden="true" /></span>
          <div>
            <h2>素材库 · 选择会话片段</h2>
            <p>左侧选会话，右侧勾选要蒸馏的具体片段</p>
          </div>
          <button aria-label="关闭" className="dw-icon-button" onClick={onClose} type="button"><X aria-hidden="true" /></button>
        </header>

        <div className="dw-drawer-body">
          <div className="dw-drawer-list">
            <div className="dw-drawer-filters">
              <label className="dw-search">
                <Search aria-hidden="true" />
                <input aria-label="搜索" onChange={(event) => setQuery(event.target.value)} placeholder="搜索会话 / 项目…" value={query} />
              </label>
              <div className="dw-drawer-selects">
                <select aria-label="来源" onChange={(event) => setSource(event.target.value)} value={source}>
                  <option value="all">来源</option>
                  {sources.map((item) => <option key={item} value={item}>{agentLabel(item)}</option>)}
                </select>
                <select aria-label="时间" onChange={(event) => setRange(event.target.value)} value={range}>
                  <option value="all">时间</option>
                  <option value="7">近 7 天</option>
                  <option value="30">近 30 天</option>
                </select>
                <select aria-label="项目" onChange={(event) => setProj(event.target.value)} value={proj}>
                  <option value="all">项目</option>
                  {projects.map((item) => <option key={item} value={item}>{item}</option>)}
                </select>
              </div>
            </div>
            <div className="dw-drawer-scroll">
              {filtered.length === 0 ? (
                <div className="dw-drawer-empty">
                  <Search aria-hidden="true" />
                  <p>没有符合条件的会话</p>
                  <small>调整筛选条件或刷新本地会话数据</small>
                  <button className="dw-pill-button" onClick={() => { setQuery(""); setSource("all"); setRange("all"); setProj("all"); }} type="button">重置筛选</button>
                </div>
              ) : grouped.map((group) => (
                <div key={group.label}>
                  <div className="dw-drawer-date">{group.label}</div>
                  <ul className="dw-drawer-items">
                    {group.items.map((item) => {
                      const itemKey = materialKeyOf(item);
                      const pill = ranges[itemKey];
                      const on = pill ? pill.e - pill.s + 1 : 0;
                      const total = cacheRef.current.get(itemKey)?.messages.length;
                      const minutes = Math.max(0, Math.round((timestampValue(item.updatedAt) - timestampValue(sessionStartedAt(item))) / 60_000)) || 0;
                      return (
                        <li key={itemKey} style={{ ["--dw-accent" as string]: agentColor(item.agent) }}>
                          <button aria-pressed={activeKey === itemKey} className={`dw-drawer-item${activeKey === itemKey ? " active" : ""}`} onClick={() => setActiveKey(itemKey)} type="button">
                            <AgentIcon agent={item.agent} />
                            <span>
                              <strong>{item.title || item.sessionId}</strong>
                              <small>{minutes}m · {sessionTurns(item)} 轮 · {sessionProjectKey(item)}</small>
                            </span>
                          </button>
                          <button className={`dw-join-pill${on > 0 ? " on" : ""}`} onClick={() => pillClick(item)} type="button">
                            <span>{on > 0 ? <Check aria-hidden="true" /> : null}</span>
                            {on > 0 ? (total ? `${on}/${total}` : `${on} 条`) : "全选"}
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              ))}
            </div>
          </div>
          <div className="dw-drawer-right">{rightPane}</div>
        </div>

        <footer className="dw-drawer-footer">
          {rangeCount > 0 ? (
            <div className="dw-range-chips">
              {Object.entries(ranges).map(([key, r]) => {
                const item = byKey.get(key);
                if (!item) return null;
                return (
                  <button key={key} onClick={() => clearChat(key)} style={{ ["--dw-accent" as string]: agentColor(item.agent) }} type="button">
                    <span>{(item.title || item.sessionId).slice(0, 16)}</span>
                    <em>#{r.s + 1}→#{r.e + 1}</em>
                    <X aria-hidden="true" />
                  </button>
                );
              })}
            </div>
          ) : null}
          <div className="dw-drawer-actions">
            <span>已选 <b>{rangeCount}</b> 场会话 · <b>{pickedSegs}</b> 条对话 · ~{formatTokens(pickedTokens)}</span>
            <button className="dw-text-button" disabled={rangeCount === 0} onClick={() => updateRanges(() => ({}))} type="button"><Trash2 aria-hidden="true" />清空</button>
            <button className="dw-pill-button" onClick={onClose} type="button">取消</button>
            <button className="dw-pill-button primary" disabled={pickedSegs === 0} onClick={onClose} type="button">确认选择</button>
          </div>
        </footer>
      </section>
    </div>,
    // Portal into the shell so the drawer inherits the liquid-glass theme tokens.
    document.querySelector(".ops-shell") ?? document.body,
  );
}

/** Quick-mode session / project picker adapted from AITracker MaterialPicker. */
export function MaterialPicker({
  sessions,
  selected,
  granularity,
  onToggle,
  onToggleProject,
}: {
  sessions: readonly DistillationWorkbenchSession[];
  selected: ReadonlySet<string>;
  granularity: "session" | "project";
  onToggle: (item: DistillationWorkbenchSession) => void;
  onToggleProject: (items: readonly DistillationWorkbenchSession[]) => void;
}) {
  if (sessions.length === 0) {
    return <div className="dw-empty"><strong>暂无可蒸馏的会话</strong><small>本地还没有采集到会话，使用 Agent 后再回来看看</small></div>;
  }
  if (granularity === "project") {
    const groups = groupDistillationSessionsByProject(sessions);
    if (groups.length === 0) return <div className="dw-empty"><strong>范围内没有 Git 项目</strong><small>切换到「按会话」可单独选择会话</small></div>;
    return (
      <ul className="dw-material-list">
        {groups.map((project) => {
          const keys = project.sessions.map(materialKeyOf);
          const checked = keys.every((key) => selected.has(key));
          const turns = project.sessions.reduce((sum, item) => sum + sessionTurns(item), 0);
          return (
            <li key={project.key}>
              <button aria-pressed={checked} className={`dw-material-row${checked ? " checked" : ""}`} onClick={() => onToggleProject(project.sessions)} type="button">
                <span className="dw-icon-stack">{project.sources.slice(0, 3).map((agent) => <AgentIcon agent={agent} key={agent} />)}</span>
                <span className="dw-material-copy">
                  <strong>{project.projectKey}</strong>
                  <small>{project.sessions.length} 场会话 · ~{formatTokens(turns * EST_TOKENS_PER_TURN)} · {formatDateTime(project.last)}</small>
                </span>
                {checked ? <Check aria-hidden="true" className="dw-check" /> : null}
              </button>
            </li>
          );
        })}
      </ul>
    );
  }
  return (
    <ul className="dw-material-list">
      {sessions.map((item) => {
        const key = materialKeyOf(item);
        const checked = selected.has(key);
        return (
          <li key={key}>
            <button aria-pressed={checked} className={`dw-material-row${checked ? " checked" : ""}`} onClick={() => onToggle(item)} type="button">
              <AgentIcon agent={item.agent} />
              <span className="dw-material-copy">
                <strong>{item.title || item.sessionId}</strong>
                <small>{sessionProjectKey(item)} · {formatDateTime(sessionStartedAt(item))} · ~{formatTokens(sessionTurns(item) * EST_TOKENS_PER_TURN)}</small>
              </span>
              {checked ? <Check aria-hidden="true" className="dw-check" /> : null}
            </button>
          </li>
        );
      })}
    </ul>
  );
}
