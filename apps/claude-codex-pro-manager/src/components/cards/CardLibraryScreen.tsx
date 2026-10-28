import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  AlertTriangle,
  Archive,
  Bug,
  CheckSquare,
  Download,
  Layers,
  Lightbulb,
  Loader2,
  PackagePlus,
  Pencil,
  Plus,
  RefreshCw,
  Ruler,
  Search,
  Tag,
  Trash2,
  X,
} from "lucide-react";

import type { CardQueryInput, CardSaveInput, DistillationSkillAgent, KnowledgeCard } from "@/types";

import {
  CARD_KIND_OPTIONS,
  DEFAULT_CARD_QUERY,
  cardDraftToSaveInput,
  cardKindLabel,
  emptyCardDraft,
  kindCount,
  type CardDraft,
  type CardKind,
  type CardKindFilter,
} from "./types";

import "./card-library.css";

const FILTERS: ReadonlyArray<{ value: CardKindFilter; label: string }> = [
  { value: "all", label: "全部" },
  ...CARD_KIND_OPTIONS.map((option) => ({ value: option.value as CardKindFilter, label: option.label })),
];

const KIND_ICONS: Record<CardKind, ReactNode> = {
  experience: <Lightbulb aria-hidden="true" />,
  pitfall: <Bug aria-hidden="true" />,
  guidance: <Ruler aria-hidden="true" />,
  architecture: <Archive aria-hidden="true" />,
};

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

export function CardLibraryScreen({
  agents,
  busy = false,
  cards,
  error = "",
  kinds,
  loading = false,
  selectedIds,
  total,
  onCreate,
  onClearSelection,
  onDeleteSelected,
  onEdit,
  onExport,
  onInstall,
  onQuery,
  onRefresh,
  onSetSelection,
  onToggleSelect,
}: {
  agents: readonly DistillationSkillAgent[];
  busy?: boolean;
  cards: readonly KnowledgeCard[];
  error?: string;
  kinds: Record<string, number>;
  loading?: boolean;
  selectedIds: readonly string[];
  total: number;
  onCreate: (input: CardSaveInput) => Promise<boolean>;
  onClearSelection: () => void;
  onDeleteSelected: () => void;
  onEdit: (card: KnowledgeCard) => void;
  onExport: (card: KnowledgeCard) => void;
  onInstall: (card: KnowledgeCard) => void;
  onQuery: (query: CardQueryInput) => void;
  onRefresh: () => void;
  onSetSelection: (ids: string[]) => void;
  onToggleSelect: (id: string) => void;
}) {
  const [kind, setKind] = useState<CardKindFilter>("all");
  const [keyword, setKeyword] = useState("");
  const [tag, setTag] = useState("");
  const [appliedKeyword, setAppliedKeyword] = useState("");
  const [appliedTag, setAppliedTag] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [draft, setDraft] = useState<CardDraft>(() => emptyCardDraft());
  const [saving, setSaving] = useState(false);
  const firstQueryRef = useRef(true);
  const onQueryRef = useRef(onQuery);
  onQueryRef.current = onQuery;

  // The keyword is a substring match over title/body/tags, so it is debounced to
  // keep one filter request per pause instead of one per keystroke.
  useEffect(() => {
    const timer = window.setTimeout(() => setAppliedKeyword(keyword.trim()), 250);
    return () => window.clearTimeout(timer);
  }, [keyword]);

  // The tag filter is an exact (case-insensitive) match, so it also waits for a pause.
  useEffect(() => {
    const timer = window.setTimeout(() => setAppliedTag(tag.trim()), 300);
    return () => window.clearTimeout(timer);
  }, [tag]);

  useEffect(() => {
    // The route load already fetched the default query; only later changes refetch.
    if (firstQueryRef.current) {
      firstQueryRef.current = false;
      return;
    }
    onQueryRef.current({
      ...DEFAULT_CARD_QUERY,
      kind: kind === "all" ? "" : kind,
      tag: appliedTag,
      keyword: appliedKeyword,
    });
  }, [kind, appliedTag, appliedKeyword]);

  const visibleSelected = useMemo(() => new Set(selectedIds), [selectedIds]);
  const allVisibleSelected = cards.length > 0 && cards.every((card) => visibleSelected.has(card.id));
  const tagOptions = useMemo(() => {
    const unique = new Set<string>();
    for (const card of cards) for (const value of card.tags ?? []) unique.add(value);
    return [...unique].sort((left, right) => left.localeCompare(right, "zh"));
  }, [cards]);

  const knownKinds = CARD_KIND_OPTIONS.map((option) => option.value as string);
  const extraKinds = Object.keys(kinds).filter((value) => !knownKinds.includes(value));
  const extraCount = extraKinds.reduce((sum, value) => sum + kindCount(kinds, value), 0);
  const filtersActive = kind !== "all" || appliedKeyword.length > 0 || appliedTag.length > 0;
  const installReady = agents.length > 0;
  const installTargets = agents.map((agent) => agent.label || agent.id).join("、");

  const stats: Array<{ kind?: CardKind; label: string; value: string; sub: string; icon: ReactNode }> = [
    {
      label: "卡片总数",
      value: `${total}`,
      sub: extraCount ? `当前显示 ${cards.length} 张 · 另有 ${extraCount} 张其它类型` : `当前显示 ${cards.length} 张`,
      icon: <Layers aria-hidden="true" />,
    },
    ...CARD_KIND_OPTIONS.map((option) => ({
      kind: option.value,
      label: option.label,
      value: `${kindCount(kinds, option.value)}`,
      sub: option.hint,
      icon: KIND_ICONS[option.value],
    })),
  ];

  const submitDraft = async () => {
    if (saving || !draft.body.trim()) return;
    setSaving(true);
    try {
      const created = await onCreate(cardDraftToSaveInput(draft));
      if (created) {
        setDraft(emptyCardDraft());
        setShowCreate(false);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <div aria-busy={busy || loading || undefined} className="cl-root">
      <section aria-label="卡片统计" className="cl-stats">
        {stats.map((stat) => (
          <article className="cl-stat" data-kind={stat.kind} key={stat.label}>
            <span className="cl-stat-icon">{stat.icon}</span>
            <span className="cl-stat-label">{stat.label}</span>
            <strong className="cl-stat-value">{stat.value}</strong>
            <small className="cl-stat-sub">{stat.sub}</small>
          </article>
        ))}
      </section>

      <section aria-label="搜索与筛选" className="cl-toolbar">
        <label className="cl-search">
          <Search aria-hidden="true" />
          <input
            aria-label="搜索卡片"
            onChange={(event) => setKeyword(event.currentTarget.value)}
            placeholder="搜索卡片标题、正文、标签或类型"
            type="search"
            value={keyword}
          />
        </label>
        <label className="cl-tagfield" title="按标签精确过滤（不区分大小写，不是子串匹配）">
          <Tag aria-hidden="true" />
          <input
            aria-label="按标签过滤"
            list="cl-tag-options"
            onChange={(event) => setTag(event.currentTarget.value)}
            placeholder="标签精确过滤"
            value={tag}
          />
        </label>
        <datalist id="cl-tag-options">
          {tagOptions.map((option) => (
            <option key={option} value={option} />
          ))}
        </datalist>
        <div aria-label="卡片类型筛选" className="cl-filters" role="group">
          {FILTERS.map((filter) => (
            <button
              aria-pressed={kind === filter.value}
              className={kind === filter.value ? "cl-filter on" : "cl-filter"}
              key={filter.value}
              onClick={() => setKind(filter.value)}
              type="button"
            >
              {filter.label} {filter.value === "all" ? total : kindCount(kinds, filter.value)}
            </button>
          ))}
        </div>
        <button
          className="cl-reset"
          disabled={!filtersActive}
          onClick={() => {
            setKeyword("");
            setTag("");
            setKind("all");
          }}
          type="button"
        >
          重置
        </button>
        <button
          aria-label="刷新卡片库"
          className={busy || loading ? "cl-refresh cl-spin" : "cl-refresh"}
          disabled={busy || loading}
          onClick={onRefresh}
          title="刷新卡片库"
          type="button"
        >
          <RefreshCw aria-hidden="true" />
        </button>
      </section>

      {showCreate ? (
        <section aria-label="快速创建卡片" className="cl-create">
          <header className="cl-create-head">
            <strong>快速创建卡片</strong>
            <small>只填正文即可保存；标题留空会由正文首行生成。创建不需要打开编辑弹窗。</small>
          </header>
          <div className="cl-create-grid">
            <div className="cl-dialog-field is-wide">
              <span className="cl-dialog-label">卡片类型</span>
              <div aria-label="新卡片类型" className="cl-kind-options" role="group">
                {CARD_KIND_OPTIONS.map((option) => (
                  <button
                    aria-pressed={draft.kind === option.value}
                    className={draft.kind === option.value ? "cl-kind-option on" : "cl-kind-option"}
                    disabled={saving}
                    key={option.value}
                    onClick={() => setDraft((current) => ({ ...current, kind: option.value }))}
                    type="button"
                  >
                    <strong>{option.label}</strong>
                    <small>{option.hint}</small>
                  </button>
                ))}
              </div>
            </div>
            <label className="cl-dialog-field">
              <span className="cl-dialog-label">标题（可选）</span>
              <input
                maxLength={256}
                onChange={(event) => setDraft((current) => ({ ...current, title: event.currentTarget.value }))}
                placeholder="留空则用正文首行"
                value={draft.title}
              />
            </label>
            <label className="cl-dialog-field">
              <span className="cl-dialog-label">标签（逗号分隔）</span>
              <input
                onChange={(event) => setDraft((current) => ({ ...current, tags: event.currentTarget.value }))}
                placeholder="例如：codex，发布"
                value={draft.tags}
              />
            </label>
            <label className="cl-dialog-field is-wide">
              <span className="cl-dialog-label">正文</span>
              <textarea
                onChange={(event) => setDraft((current) => ({ ...current, body: event.currentTarget.value }))}
                placeholder="这张卡片要沉淀的具体经验、踩坑或规范"
                rows={4}
                value={draft.body}
              />
            </label>
          </div>
          <footer className="cl-create-actions">
            <button
              className="cl-dialog-button"
              disabled={saving}
              onClick={() => {
                setDraft(emptyCardDraft());
                setShowCreate(false);
              }}
              type="button"
            >
              取消
            </button>
            <button
              className="cl-dialog-button primary"
              disabled={saving || !draft.body.trim()}
              onClick={() => void submitDraft()}
              type="button"
            >
              保存卡片
            </button>
          </footer>
        </section>
      ) : null}

      <div className="cl-list-head">
        <strong>共 {total} 张卡片</strong>
        <span>
          当前显示 {cards.length} 张
          {cards.length >= (DEFAULT_CARD_QUERY.limit ?? 0)
            ? "（已达单页上限，请用筛选缩小范围）"
            : " · 按更新时间倒序"}
          {" · "}
          {installReady ? `一键安装目标：${installTargets}` : "未检测到可写入的 Skill 目录"}
        </span>
        <div className="cl-list-head-actions">
          <label className="cl-check">
            <input
              aria-label="全选当前列表"
              checked={allVisibleSelected}
              disabled={!cards.length}
              onChange={() =>
                onSetSelection(allVisibleSelected ? [] : cards.map((card) => card.id))
              }
              type="checkbox"
            />
            全选
          </label>
          <button
            className={showCreate ? "cl-action primary" : "cl-action"}
            onClick={() => setShowCreate((current) => !current)}
            title="展开快速创建表单（不需要打开编辑弹窗）"
            type="button"
          >
            <Plus aria-hidden="true" /><span>新建卡片</span>
          </button>
        </div>
      </div>

      {selectedIds.length ? (
        <div className="cl-selection">
          <CheckSquare aria-hidden="true" />
          <span className="cl-selection-count">已选 {selectedIds.length} 张</span>
          <button className="cl-action danger" disabled={busy} onClick={onDeleteSelected} type="button">
            <Trash2 aria-hidden="true" /><span>删除所选</span>
          </button>
          <button className="cl-action" disabled={busy} onClick={onClearSelection} type="button">
            <X aria-hidden="true" /><span>取消选择</span>
          </button>
        </div>
      ) : null}

      {loading ? (
        <div className="cl-empty">
          <span className="cl-empty-icon cl-spin"><Loader2 aria-hidden="true" /></span>
          <strong>正在加载卡片库…</strong>
          <small>正在读取本地卡片索引。</small>
        </div>
      ) : error ? (
        <div className="cl-alert" role="alert">
          <AlertTriangle aria-hidden="true" />
          <div>
            <strong>卡片库加载失败</strong>
            <small>{error}</small>
          </div>
          <button className="cl-action" onClick={onRefresh} type="button">
            <RefreshCw aria-hidden="true" /><span>重试</span>
          </button>
        </div>
      ) : cards.length ? (
        <div className="cl-grid" role="list">
          {cards.map((card, index) => {
            const selected = visibleSelected.has(card.id);
            return (
              <article
                className={selected ? "cl-card is-selected" : "cl-card"}
                data-kind={card.kind}
                key={card.id || `card-${index}`}
                role="listitem"
              >
                <header className="cl-card-head">
                  <label className="cl-check">
                    <input
                      aria-label={`选择卡片 ${card.title || "未命名卡片"}`}
                      checked={selected}
                      onChange={() => onToggleSelect(card.id)}
                      type="checkbox"
                    />
                  </label>
                  <span className="cl-kind-icon">{KIND_ICONS[(card.kind as CardKind) ?? "experience"] ?? <Layers aria-hidden="true" />}</span>
                  <span className="cl-kind-label">{cardKindLabel(card.kind)}</span>
                  {card.source ? <span className="cl-card-source" title={card.source}>来源 {card.source}</span> : null}
                </header>
                <h3 className="cl-card-title" title={card.title || "未命名卡片"}>{card.title || "未命名卡片"}</h3>
                <p className="cl-card-body">{card.body}</p>
                {card.tags?.length ? (
                  <div className="cl-tags">
                    {card.tags.map((value) => (
                      <button
                        className="cl-tag"
                        key={value}
                        onClick={() => setTag(value)}
                        title={`按标签「${value}」精确过滤`}
                        type="button"
                      >
                        {value}
                      </button>
                    ))}
                  </div>
                ) : null}
                <footer className="cl-card-foot">
                  <time className="cl-card-time" dateTime={isoStamp(card.updatedAt)}>{formatStamp(card.updatedAt)}</time>
                  <div className="cl-card-actions">
                    <button className="cl-action" disabled={busy} onClick={() => onExport(card)} title="导出单卡为 Markdown" type="button">
                      <Download aria-hidden="true" /><span>导出单卡</span>
                    </button>
                    <button
                      className="cl-action"
                      disabled={busy || !installReady}
                      onClick={() => onInstall(card)}
                      title={installReady ? `安装到：${installTargets}` : "未检测到可写入的 Skill 目录"}
                      type="button"
                    >
                      <PackagePlus aria-hidden="true" /><span>一键安装到插件中心</span>
                    </button>
                    <button className="cl-action" disabled={busy} onClick={() => onEdit(card)} title="编辑卡片（可选操作）" type="button">
                      <Pencil aria-hidden="true" /><span>编辑</span>
                    </button>
                  </div>
                </footer>
              </article>
            );
          })}
        </div>
      ) : (
        <div className="cl-empty">
          <span className="cl-empty-icon">{filtersActive ? <Search aria-hidden="true" /> : <Layers aria-hidden="true" />}</span>
          <strong>{filtersActive ? "没有匹配的卡片" : "还没有卡片"}</strong>
          {filtersActive ? (
            <>
              <small>当前筛选条件没有命中任何卡片，可以放宽条件或直接重置。</small>
              <div className="cl-empty-actions">
                <button
                  className="cl-action"
                  onClick={() => {
                    setKeyword("");
                    setTag("");
                    setKind("all");
                  }}
                  type="button"
                >
                  重置筛选
                </button>
              </div>
            </>
          ) : (
            <>
              <small>在蒸馏工作台沉淀经验，或用「新建卡片」手工录入一张；创建、检索、导出都不需要打开编辑弹窗。</small>
              <div className="cl-empty-actions">
                <button className="cl-action" onClick={() => setShowCreate(true)} type="button">
                  <Plus aria-hidden="true" /><span>新建卡片</span>
                </button>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}