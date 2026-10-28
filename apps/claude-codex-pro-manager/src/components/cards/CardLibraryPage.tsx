import { useRef, useState } from "react";

import type { AppActions } from "@/lib/actions";
import type {
  CardLibraryResult,
  CardQueryInput,
  CardSaveInput,
  DistillationSkillAgent,
  KnowledgeCard,
} from "@/types";

import { CardEditDialog } from "./CardEditDialog";
import { CardLibraryScreen } from "./CardLibraryScreen";
import { DEFAULT_CARD_QUERY } from "./types";

/**
 * Card library page: the screen owns search/filter state, this wrapper owns
 * selection, the optional edit dialog and every Tauri command call.
 */
export function CardLibraryPage({
  actions,
  agents = [],
  data,
}: {
  actions: AppActions;
  agents?: readonly DistillationSkillAgent[];
  data: CardLibraryResult | null;
}) {
  const [editing, setEditing] = useState<KnowledgeCard | null>(null);
  const [busy, setBusy] = useState(false);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  // Last query the screen asked for, so refreshes keep the active filters.
  const queryRef = useRef<CardQueryInput>(DEFAULT_CARD_QUERY);

  const cards = data?.cards ?? [];
  const kinds = data?.kinds ?? {};
  const loading = data === null;
  const error = data && data.status !== "ok" ? data.message || "卡片库读取失败。" : "";

  const reload = (query?: CardQueryInput) => {
    if (query) queryRef.current = query;
    void actions.loadCardLibrary(queryRef.current);
  };

  const save = async (input: CardSaveInput) => {
    setBusy(true);
    try {
      const result = await actions.saveCard(input);
      const ok = result?.status === "ok";
      actions.showNotice({
        title: input.id ? "更新卡片" : "新建卡片",
        message: result?.message || "保存卡片失败：未收到后端响应。",
        status: ok ? "ok" : "failed",
      });
      if (ok) {
        setEditing(null);
        reload();
      }
      return ok;
    } finally {
      setBusy(false);
    }
  };

  const removeSelected = async () => {
    if (!selectedIds.length) return;
    if (!window.confirm(`删除所选 ${selectedIds.length} 张卡片？此操作不可撤销。`)) return;
    setBusy(true);
    try {
      const result = await actions.deleteCards(selectedIds);
      const ok = result?.status === "ok";
      if (ok) setSelectedIds([]);
      actions.showNotice({
        title: "删除卡片",
        message: result?.message || "删除卡片失败：未收到后端响应。",
        status: ok ? "ok" : "failed",
      });
      if (ok) reload();
    } finally {
      setBusy(false);
    }
  };

  const exportOne = async (card: KnowledgeCard) => {
    setBusy(true);
    try {
      const result = await actions.exportCard(card.id);
      const ok = result?.status === "ok";
      actions.showNotice({
        title: "导出单卡",
        message: ok
          ? result?.message || `已导出${result?.path ? `到 ${result.path}` : ""}。`
          : result?.message || "导出卡片失败：未收到后端响应。",
        status: ok ? "ok" : "failed",
      });
    } finally {
      setBusy(false);
    }
  };

  const installOne = async (card: KnowledgeCard) => {
    const agentIds = agents.map((agent) => agent.id).filter(Boolean);
    if (!agentIds.length) {
      // Never fire a call that the backend is guaranteed to reject.
      actions.showNotice({
        title: "一键安装到插件中心",
        message: "未检测到可写入的 Skill 目录，已跳过安装。请先在蒸馏工作台确认 Agent 环境。",
        status: "failed",
      });
      return;
    }
    setBusy(true);
    try {
      const result = await actions.installCardSkill(card.id, agentIds);
      const message = result?.message || "安装卡片为 Skill 失败：未收到后端响应。";
      // A partial conflict still returns `ok` and names the skipped agents in the
      // message ("…已存在同名目录，未覆盖…"), so it is surfaced as needs-attention
      // instead of a plain success.
      const conflict = Boolean(result && result.status === "ok" && message.includes("未覆盖"));
      actions.showNotice({
        title: "一键安装到插件中心",
        message: conflict ? `存在同名 Skill 目录，未覆盖全部目标：${message}` : message,
        status: result ? (conflict ? "needs_review" : result.status) : "failed",
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <section aria-label="卡片库" className="cards-library-page">
      <CardLibraryScreen
        agents={agents}
        busy={busy}
        cards={cards}
        error={error}
        kinds={kinds}
        loading={loading}
        selectedIds={selectedIds}
        total={data?.total ?? 0}
        onCreate={save}
        onClearSelection={() => setSelectedIds([])}
        onDeleteSelected={() => void removeSelected()}
        onEdit={setEditing}
        onExport={(card) => void exportOne(card)}
        onInstall={(card) => void installOne(card)}
        onQuery={(query) => {
          setSelectedIds([]);
          reload(query);
        }}
        onRefresh={() => reload()}
        onSetSelection={(ids) => setSelectedIds([...new Set(ids)])}
        onToggleSelect={(id) =>
          setSelectedIds((current) =>
            current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
          )
        }
      />
      {editing ? (
        <CardEditDialog
          busy={busy}
          card={editing}
          onClose={() => setEditing(null)}
          onExport={(card) => void exportOne(card)}
          onSave={(input) => void save(input)}
        />
      ) : null}
    </section>
  );
}