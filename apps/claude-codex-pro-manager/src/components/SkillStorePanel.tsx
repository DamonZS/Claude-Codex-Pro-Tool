import { useCallback, useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Empty, InfoRow, Panel } from "@/components/ui/ops";
import type { AppActions } from "@/lib/actions";
import type { SkillStoreEntry, SkillStorePayload } from "@/types";

const SOURCE_APP_LABELS: Record<string, string> = {
  codex: "Codex",
  claude: "Claude",
  unknown: "手动放入",
};

function sourceAppLabel(sourceApp: string) {
  return SOURCE_APP_LABELS[sourceApp] ?? (sourceApp || "未知来源");
}

function formatImportedAt(value: string) {
  if (!value) return "未知时间";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", { hour12: false });
}

/**
 * 「Skills 真源库」面板：列出真源库里的 Skill，并可分发到 Codex / Claude。
 * 数据全部来自 `list_skill_store_entries` 系列命令的 payload，`null` 表示本次操作失败。
 */
export function SkillStorePanel({ actions }: { actions: AppActions }) {
  const [store, setStore] = useState<SkillStorePayload | null>(null);
  const [pending, setPending] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setPending("refresh");
    try {
      const result = await actions.refreshSkillStore();
      if (result) setStore(result);
    } finally {
      setPending(null);
    }
  }, [actions]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const sync = async (entry: SkillStoreEntry, app: "codex" | "claude") => {
    setPending(`sync:${entry.id}:${app}`);
    try {
      const result = await actions.syncStoreSkill(entry.id, app);
      if (result) setStore(result);
    } finally {
      setPending(null);
    }
  };

  const entries = store?.entries ?? [];
  const busy = pending !== null;

  return (
    <Panel title="Skills 真源库" detail="本机统一保存的 Skill 源；分发只写入目标应用的 skills 目录，不改动真源库内容。">
      <div className="stack">
        <InfoRow label="真源库目录" value={store?.root ?? "尚未读取"} />
        <div className="action-row">
          <Button disabled={busy} onClick={() => void refresh()} size="sm" variant="outline">
            <RefreshCw className={`h-4 w-4${pending === "refresh" ? " spin" : ""}`} />
            {pending === "refresh" ? "刷新中…" : "刷新"}
          </Button>
        </div>
        <div className="context-entry-list unified-tool-list">
          {entries.length === 0 ? (
            <Empty text="真源库还没有 Skill：在下方列表里点『收进真源』" />
          ) : (
            entries.map((entry) => (
              <div className="context-entry-row unified-tool-row" key={entry.id}>
                <div className="unified-tool-copy">
                  <strong>{entry.name || entry.id}</strong>
                  <small>
                    来源：{sourceAppLabel(entry.sourceApp)} · 导入时间：{formatImportedAt(entry.importedAt)}
                  </small>
                </div>
                <div className="action-row">
                  <Button disabled={busy} onClick={() => void sync(entry, "codex")} size="sm">
                    {pending === `sync:${entry.id}:codex` ? "分发中…" : "分发到 Codex"}
                  </Button>
                  <Button disabled={busy} onClick={() => void sync(entry, "claude")} size="sm" variant="outline">
                    {pending === `sync:${entry.id}:claude` ? "分发中…" : "分发到 Claude"}
                  </Button>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </Panel>
  );
}