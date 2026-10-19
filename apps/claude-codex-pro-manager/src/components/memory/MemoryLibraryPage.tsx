import { useState } from "react";

import type { AppActions } from "@/lib/actions";
import type { MemoryLibraryEntry, MemoryLibraryResult } from "@/types";

import { MemoryEditDialog } from "./MemoryEditDialog";
import { MemoryLibraryScreen } from "./MemoryLibraryScreen";
import type { MemorySaveInput } from "./types";

/**
 * Memory hub page: the screen owns search/filter state, this wrapper owns the
 * edit dialog and the delete confirmation, and forwards everything to Tauri
 * commands through the shared actions object.
 */
export function MemoryLibraryPage({
  actions,
  data,
}: {
  actions: AppActions;
  data: MemoryLibraryResult | null;
}) {
  const [editing, setEditing] = useState<MemoryLibraryEntry | null>(null);
  const [busy, setBusy] = useState(false);
  const entries = data?.entries ?? [];
  const counts = data?.counts ?? { total: 0, persona: 0, memory: 0 };

  const exportEntry = async (entry: MemoryLibraryEntry) => {
    const result = await actions.exportMemoryLibraryEntry(entry.id);
    const ok = result?.status === "ok";
    actions.showNotice({
      title: "导出记忆",
      message: ok ? `已导出到 ${result?.path ?? ""}` : (result?.message ?? "导出记忆失败。"),
      status: ok ? "ok" : "failed",
    });
  };

  const save = async (input: MemorySaveInput) => {
    setBusy(true);
    try {
      const result = await actions.saveMemoryLibraryEntry(input);
      if (result?.status === "ok") setEditing(null);
    } finally {
      setBusy(false);
    }
  };

  const remove = async (entry: MemoryLibraryEntry) => {
    if (!window.confirm(`删除记忆「${entry.title || "未命名记忆"}」？此操作不可撤销。`)) return;
    setBusy(true);
    try {
      await actions.deleteMemoryLibraryEntries([entry.id]);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="memory-library-page" aria-label="记忆库">
      <MemoryLibraryScreen
        busy={busy}
        counts={counts}
        entries={entries}
        updatedAt={data?.updatedAt ?? ""}
        onDelete={(entry) => void remove(entry)}
        onEdit={setEditing}
        onExport={(entry) => void exportEntry(entry)}
        onRefresh={() => {
          void actions.loadMemoryLibrary();
        }}
      />
      {editing ? (
        <MemoryEditDialog
          busy={busy}
          entry={editing}
          onClose={() => setEditing(null)}
          onExport={(entry) => void exportEntry(entry)}
          onSave={(input) => void save(input)}
        />
      ) : null}
    </section>
  );
}
