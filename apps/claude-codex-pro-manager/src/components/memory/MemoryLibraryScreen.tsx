import { useMemo, useState, type ReactNode } from "react";
import {
  Brain,
  Clock3,
  Download,
  Library,
  NotebookPen,
  Pencil,
  RefreshCw,
  Search,
  Sparkles,
  Trash2,
  UserRound,
} from "lucide-react";

import type { MemoryFilter, MemoryLibraryCounts, MemoryLibraryEntry } from "./types";

import "./memory-library.css";

const FILTER_LABELS: Record<MemoryFilter, string> = {
  all: "全部",
  persona: "画像",
  memory: "任务记忆",
};

function kindLabel(kind: MemoryLibraryEntry["kind"]) {
  return kind === "persona" ? "画像" : "任务记忆";
}

function KindIcon({ kind }: { kind: MemoryLibraryEntry["kind"] }) {
  return kind === "persona" ? <UserRound aria-hidden="true" /> : <NotebookPen aria-hidden="true" />;
}

/** Accepts both ISO strings and epoch-millisecond strings (or numbers). */
function timestampValue(value: string | number | null | undefined) {
  if (typeof value === "number") return Number.isFinite(value) ? value : Number.NaN;
  const normalized = (value ?? "").trim();
  if (!normalized) return Number.NaN;
  return /^\d+(?:\.\d+)?$/.test(normalized) ? Number(normalized) : Date.parse(normalized);
}

/** MM/DD HH:mm, or the fixed placeholder when the timestamp cannot be parsed. */
function formatStamp(value: string | null | undefined) {
  const time = timestampValue(value);
  const date = new Date(time);
  if (!Number.isFinite(time) || Number.isNaN(date.getTime())) return "--/-- --:--";
  const pad = (input: number) => String(input).padStart(2, "0");
  return `${pad(date.getMonth() + 1)}/${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function isoStamp(value: string | null | undefined) {
  const time = timestampValue(value);
  const date = new Date(time);
  return !Number.isFinite(time) || Number.isNaN(date.getTime()) ? undefined : date.toISOString();
}

export function MemoryLibraryScreen({
  entries,
  counts,
  updatedAt,
  busy = false,
  onRefresh,
  onExport,
  onEdit,
  onDelete,
}: {
  entries: readonly MemoryLibraryEntry[];
  counts: MemoryLibraryCounts;
  updatedAt: string;
  busy?: boolean;
  onRefresh: () => void;
  onExport: (entry: MemoryLibraryEntry) => void;
  onEdit: (entry: MemoryLibraryEntry) => void;
  onDelete: (entry: MemoryLibraryEntry) => void;
}) {
  const [keyword, setKeyword] = useState("");
  const [filter, setFilter] = useState<MemoryFilter>("all");

  const distilled = useMemo(() => entries.filter((entry) => entry.source === "distill").length, [entries]);

  const visible = useMemo(() => {
    const search = keyword.trim().toLocaleLowerCase();
    return entries.filter((entry) => {
      if (filter !== "all" && entry.kind !== filter) return false;
      if (!search) return true;
      return [entry.title, entry.body, entry.source]
        .some((value) => (value ?? "").toLocaleLowerCase().includes(search));
    });
  }, [entries, filter, keyword]);

  const stats: Array<{ label: string; value: string; sub: string; icon: ReactNode; time?: boolean }> = [
    {
      label: "记忆总数",
      value: `${counts.total}`,
      sub: `画像 ${counts.persona} 条 · 任务记忆 ${counts.memory} 条`,
      icon: <Library aria-hidden="true" />,
    },
    {
      label: "画像",
      value: `${counts.persona}`,
      sub: "关于你这个人：偏好与习惯",
      icon: <UserRound aria-hidden="true" />,
    },
    {
      label: "蒸馏沉淀",
      value: `${distilled}`,
      sub: "手动录入 0 条",
      icon: <Sparkles aria-hidden="true" />,
    },
    {
      label: "最近更新",
      value: formatStamp(updatedAt),
      sub: "导出后即可直接放进上下文",
      icon: <Clock3 aria-hidden="true" />,
      time: true,
    },
  ];

  const filterCounts: Record<MemoryFilter, number> = {
    all: counts.total,
    persona: counts.persona,
    memory: counts.memory,
  };

  return (
    <div aria-busy={busy || undefined} className="ml-root">
      <section aria-label="记忆统计" className="ml-stats">
        {stats.map((stat) => (
          <article className="ml-stat" key={stat.label}>
            <span className="ml-stat-icon">{stat.icon}</span>
            <span className="ml-stat-label">{stat.label}</span>
            <strong className={stat.time ? "ml-stat-value is-time" : "ml-stat-value"}>{stat.value}</strong>
            <small className="ml-stat-sub">{stat.sub}</small>
          </article>
        ))}
      </section>

      <section aria-label="搜索与筛选" className="ml-toolbar">
        <label className="ml-search">
          <Search aria-hidden="true" />
          <input
            aria-label="搜索记忆"
            onChange={(event) => setKeyword(event.currentTarget.value)}
            placeholder="搜索记忆标题、正文或来源"
            type="search"
            value={keyword}
          />
        </label>
        <div aria-label="记忆筛选" className="ml-filters" role="group">
          {(["all", "persona", "memory"] as const).map((value) => (
            <button
              aria-pressed={filter === value}
              className={filter === value ? "ml-filter on" : "ml-filter"}
              key={value}
              onClick={() => setFilter(value)}
              type="button"
            >
              {FILTER_LABELS[value]} {filterCounts[value]}
            </button>
          ))}
        </div>
        <button
          className="ml-reset"
          disabled={!keyword.trim() && filter === "all"}
          onClick={() => {
            setKeyword("");
            setFilter("all");
          }}
          type="button"
        >
          重置
        </button>
        <button
          aria-label="刷新记忆库"
          className={busy ? "ml-refresh ml-spin" : "ml-refresh"}
          disabled={busy}
          onClick={onRefresh}
          title="刷新记忆库"
          type="button"
        >
          <RefreshCw aria-hidden="true" />
        </button>
      </section>

      <div className="ml-list-head">
        <strong>共 {visible.length} 条记忆</strong>
        <span>导出成 .md 后，直接丢进任意 AI 助手的上下文即可</span>
      </div>

      {visible.length ? (
        <div className="ml-grid" role="list">
          {visible.map((entry, index) => (
            <article className="ml-card" data-kind={entry.kind} key={entry.id || `${entry.kind}-${index}`} role="listitem">
              <header className="ml-card-head">
                <span className="ml-kind-icon"><KindIcon kind={entry.kind} /></span>
                <span className="ml-kind-label">{kindLabel(entry.kind)}</span>
              </header>
              <h3 className="ml-card-title" title={entry.title || "未命名记忆"}>{entry.title || "未命名记忆"}</h3>
              <p className="ml-card-body">{entry.body}</p>
              <footer className="ml-card-foot">
                <time className="ml-card-time" dateTime={isoStamp(entry.updatedAt)}>{formatStamp(entry.updatedAt)}</time>
                <div className="ml-card-actions">
                  <button className="ml-action" onClick={() => onExport(entry)} title="导出 MD" type="button">
                    <Download aria-hidden="true" /><span>导出 MD</span>
                  </button>
                  <button className="ml-action" onClick={() => onEdit(entry)} title="编辑" type="button">
                    <Pencil aria-hidden="true" /><span>编辑</span>
                  </button>
                  <button className="ml-action danger" onClick={() => onDelete(entry)} title="删除" type="button">
                    <Trash2 aria-hidden="true" /><span>删除</span>
                  </button>
                </div>
              </footer>
            </article>
          ))}
        </div>
      ) : (
        <div className="ml-empty">
          <span className="ml-empty-icon">{entries.length ? <Search aria-hidden="true" /> : <Brain aria-hidden="true" />}</span>
          <strong>{entries.length ? "没有匹配的记忆" : "还没有记忆条目"}</strong>
          {entries.length ? null : <small>在蒸馏工作台选择「画像」或「任务记忆」，产物会自动沉淀到这里。</small>}
        </div>
      )}
    </div>
  );
}
