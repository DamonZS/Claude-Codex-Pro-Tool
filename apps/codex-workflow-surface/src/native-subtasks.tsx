import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";
import { requireWorkflowBridge } from "./runtime-bridge";

const responseSchema = z.object({
  status: z.literal("ok"),
  items: z.array(z.object({
    id: z.string().min(1), parent_thread_id: z.string().min(1),
    agent_nickname: z.string().nullable(), status: z.string(),
    updated_at_ms: z.number().nonnegative(),
    source: z.literal("codex_native"), read_only: z.literal(true),
  })).max(100),
  total: z.number().int().nonnegative(),
  stale: z.boolean().optional(),
});
const statusLabels: Record<string, string> = {
  pending: "等待执行", queued: "排队中", running: "执行中", in_progress: "执行中",
  inProgress: "执行中", completed: "已完成", failed: "失败",
  interrupted: "已中断", cancelled: "已取消",
};

export function NativeSubtasks({ onOpenThread }: { onOpenThread?: (id: string) => Promise<unknown> }) {
  const [expanded, setExpanded] = useState(false);
  const [openError, setOpenError] = useState(false);
  const region = useRef<HTMLElement>(null);
  const [visible, setVisible] = useState(true);
  useEffect(() => {
    const root = region.current?.getRootNode();
    if (!(root instanceof ShadowRoot)) return;
    const host = root.host as HTMLElement;
    const update = () => setVisible(host.style.display !== "none");
    const observer = new MutationObserver(update);
    observer.observe(host, { attributes: true, attributeFilter: ["style"] });
    update();
    return () => observer.disconnect();
  }, []);
  const client = useQueryClient();
  const queryKey = ["ccp", "codex-native-subtasks"];
  const query = useQuery({
    queryKey,
    enabled: visible,
    queryFn: async () => {
      const result = responseSchema.parse(await requireWorkflowBridge().postJson(
        "/multica/workspace/query", { resource: "codex_native_agents", limit: 100, offset: 0 },
      ));
      // A partial database read must not erase previously visible child threads.
      if (result.stale && client.getQueryData(queryKey)) throw new Error("native_subtasks_stale");
      return result;
    },
    refetchInterval: 5000,
    retry: false,
  });
  const items = query.data?.items ?? [];
  async function openThread(id: string) {
    setOpenError(false);
    try {
      const open = onOpenThread ?? requireWorkflowBridge().openThread;
      if (!open) throw new Error("thread_open_unavailable");
      const result = await open(id);
      if (result === false || result && typeof result === "object" && (
        "ok" in result && result.ok === false ||
        "status" in result && ["failed", "error"].includes(String(result.status))
      )) throw new Error("thread_open_failed");
    } catch { setOpenError(true); }
  }
  return <section ref={region} className="ccp-native-subtasks" aria-label="Codex 原生子任务">
    <div className="ccp-native-subtasks-header">
      <h2>Codex 原生子任务{query.data ? ` · ${query.data.total}` : ""}</h2>
      <button type="button" disabled={query.isFetching} onClick={() => void query.refetch()}>刷新子任务</button>
    </div>
    {query.isPending && <p>正在读取原生子任务…</p>}
    {(query.isError || query.data?.stale) && <p role="alert">
      {query.data ? "子任务数据可能已过期，请刷新重试。" : "子任务读取失败，请刷新重试。"}
    </p>}
    {query.isSuccess && !query.data.stale && !items.length && <p>暂无原生子任务</p>}
    {openError && <p role="alert">会话打开失败，请重试。</p>}
    {!!items.length && <ul>
      {(expanded ? items : items.slice(0, 4)).map((item) => <li key={item.id} data-native-thread-id={item.id}>
        <button type="button" className="ccp-native-subtask-name" onClick={() => void openThread(item.id)}>
          {item.agent_nickname?.trim() || item.id.slice(0, 8)}
        </button>
        <span>{statusLabels[item.status] ?? "状态待确认"}</span>
        <button type="button" onClick={() => void openThread(item.parent_thread_id)} aria-label={`打开 ${item.agent_nickname?.trim() || item.id.slice(0, 8)} 的父会话`}>
          父会话 {item.parent_thread_id.slice(0, 8)}
        </button>
        <time dateTime={item.updated_at_ms ? new Date(item.updated_at_ms).toISOString() : undefined}>
          {item.updated_at_ms ? new Date(item.updated_at_ms).toLocaleString() : "更新时间待确认"}
        </time>
      </li>)}
    </ul>}
    {items.length > 4 && <button type="button" aria-expanded={expanded} onClick={() => setExpanded(!expanded)}>
      {expanded ? "收起子任务" : `展开近期 ${items.length} 个子任务`}
    </button>}
    {query.data && query.data.total > items.length && expanded && <p>显示最近 {items.length} 个子任务</p>}
  </section>;
}
