import { useCallback, useEffect, useRef, useState, useSyncExternalStore, type ReactNode } from "react";
import { useIsFetching, useQuery, useQueryClient, type Query } from "@tanstack/react-query";
import { issueDetailOptions } from "@multica/core/issues/queries";
import { requireWorkflowBridge } from "./runtime-bridge";
import { ExecutionBadge } from "./execution-issue";
import { NativeExecutionFeedback } from "./native-execution-feedback";
import { getExecutionBoardStatus, subscribeExecutionBoardStatus } from "./multica-execution-board";

export function ExecutionBoardRefresh({ workspaceId }: { workspaceId: string }) {
  const anchor = useRef<HTMLDivElement>(null);
  const client = useQueryClient();
  const status = useSyncExternalStore(subscribeExecutionBoardStatus, getExecutionBoardStatus);
  const relevant = useCallback((query: Query) => query.isActive() && (
    query.queryKey[0] === "issues" && query.queryKey[1] === workspaceId ||
    query.queryKey[0] === "workspaces" && query.queryKey[1] === workspaceId && ["working-agents", "agent-task-snapshot"].includes(String(query.queryKey[2]))
  ), [workspaceId]);
  const cache = client.getQueryCache();
  const fetching = useIsFetching({ predicate: relevant });
  const queryError = useSyncExternalStore(
    useCallback((listener) => cache.subscribe(listener), [cache]),
    () => cache.getAll().some(query => relevant(query) && query.state.status === "error"),
  );
  const refresh = useCallback(() => client.refetchQueries({ predicate: relevant }, { cancelRefetch: false }), [client, relevant]);
  useEffect(() => {
    const doc = anchor.current!.ownerDocument;
    const root = anchor.current!.getRootNode();
    const host = root instanceof ShadowRoot ? root.host : anchor.current!.parentElement;
    const visible = () => doc.visibilityState !== "hidden" && (!host || !host.hasAttribute("hidden") &&
      getComputedStyle(host).display !== "none" && getComputedStyle(host).visibility !== "hidden");
    let wasVisible = visible();
    const update = () => { const next = visible(); if (next && !wasVisible) void refresh(); wasVisible = next; };
    const observer = new MutationObserver(update);
    if (host) observer.observe(host, { attributes: true, attributeFilter: ["style", "class", "hidden"] });
    doc.addEventListener("visibilitychange", update);
    const timer = setInterval(() => { if (visible()) void refresh(); }, 5000);
    return () => { clearInterval(timer); observer.disconnect(); doc.removeEventListener("visibilitychange", update); };
  }, [refresh]);
  return <div ref={anchor} className="ccp-execution-refresh">
    {(status.stale || queryError) && <div role="alert">执行数据可能已过期，已保留最近可用结果。<button type="button" disabled={fetching > 0} onClick={() => void refresh()}>重试</button></div>}
  </div>;
}

export function ExecutionDetail({ id, workspaceId, onOpenThread, leadingAction }: {
  id: string; workspaceId: string; onOpenThread?: (id: string) => Promise<unknown>; leadingAction: ReactNode;
}) {
  const query = useQuery({ ...issueDetailOptions(workspaceId, id), retry: false, staleTime: 0 });
  const [openError, setOpenError] = useState(false);
  async function openThread(threadId: string) {
    setOpenError(false);
    try {
      const open = onOpenThread ?? requireWorkflowBridge().openThread;
      if (!open) throw new Error("thread_open_unavailable");
      const result = await open(threadId);
      if (result === false || result && typeof result === "object" && (
        "ok" in result && result.ok === false ||
        "status" in result && ["failed", "error"].includes(String(result.status))
      )) throw new Error("thread_open_failed");
    } catch { setOpenError(true); }
  }
  const issue = query.data;
  const thread = issue?.metadata?.ccp_thread_id;
  const parent = issue?.metadata?.ccp_parent_thread_id;
  return <section className="ccp-execution-detail" aria-label="执行详情">
    <div className="ccp-execution-detail-actions">{leadingAction}</div>
    {query.isPending && <p role="status">正在读取执行详情…</p>}
    {query.isError && <>
      <p role="alert">{issue ? "执行数据可能已过期，请重试。" : "执行详情读取失败，请重试。"}</p>
      <button type="button" onClick={() => void query.refetch()}>重试</button>
    </>}
    {issue && <>
      <h1>{issue.title}</h1>
      <ExecutionBadge issue={issue} />
      <NativeExecutionFeedback issues={[issue]} />
      <p>只读执行记录 · 状态随实际执行更新</p>
      <time dateTime={issue.updated_at}>{new Date(issue.updated_at).toLocaleString()}</time>
      <div className="ccp-execution-detail-actions">
        {typeof thread === "string" && thread && <button type="button" onClick={() => void openThread(thread)}>打开子会话</button>}
        {typeof parent === "string" && parent && <button type="button" onClick={() => void openThread(parent)}>打开父会话</button>}
      </div>
    </>}
    {openError && <p role="alert">会话打开失败，请重试。</p>}
  </section>;
}
