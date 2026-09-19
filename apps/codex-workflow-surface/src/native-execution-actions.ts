import type { Issue } from "@multica/core/types";
import { useCallback, useSyncExternalStore } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { requireWorkflowBridge } from "./runtime-bridge";

export type NativeExecutionCard = Pick<Issue, "id" | "workspace_id" | "updated_at" | "metadata">;
export const DEFAULT_CONTINUATION = "继续完成当前任务，检查剩余工作并完成验证；若已全部完成，简要确认结果。";
export const nativeExecutionIntent = (category: string | undefined) =>
  category === "in_progress" ? "continue" : category === "todo" ? "enqueue" : null;

export const isNativeExecutionCard = (issue: NativeExecutionCard) =>
  issue.metadata?.ccp_source === "codex-native" && typeof issue.metadata.ccp_thread_id === "string" && !!issue.metadata.ccp_thread_id;
const running = new Set(["inProgress", "in_progress", "running", "cancel_pending", "dispatched"]);
const queued = new Set(["queued", "pending", "binding_pending", "waiting_local_directory"]);
type Action = { issue: NativeExecutionCard; category: string; fingerprint: string; commandId: string; phase: "pending" | "accepted" | "error" };
let actions: Action[] = [];
const listeners = new Set<() => void>();
const subscribe = (listener: () => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; };
const identity = (issue: NativeExecutionCard) => `${issue.workspace_id}:${issue.metadata?.ccp_thread_id}`;
const fingerprint = (issue: NativeExecutionCard) => `${issue.updated_at}:${issue.metadata?.ccp_execution_state}`;
function publish(action: Action) {
  actions = [...actions.filter(value => identity(value.issue) !== identity(action.issue)), action];
  listeners.forEach(listener => listener());
}
export function useNativeExecutionActionState() {
  return useSyncExternalStore(subscribe, () => actions);
}

export function useNativeExecutionActions() {
  const client = useQueryClient();
  return useCallback(async (issue: NativeExecutionCard, category: string | undefined) => {
    await submitNativeExecutionIntent(issue, category);
    await Promise.all([
      client.invalidateQueries({ queryKey: ["issues", issue.workspace_id] }),
      client.invalidateQueries({ queryKey: ["workspaces", issue.workspace_id] }),
    ]);
  }, [client]);
}

export async function submitNativeExecutionIntent(issue: NativeExecutionCard, category: string | undefined) {
  const intent = nativeExecutionIntent(category);
  if (!intent || !isNativeExecutionCard(issue)) return;
  const state = String(issue.metadata?.ccp_execution_state);
  const previous = actions.find(value => identity(value.issue) === identity(issue));
  // An unresolved command must reconcile with its original identity, even if a
  // refresh has already observed a running turn or a different timestamp.
  const retry = previous?.phase === "error" ? previous : undefined;
  if (!retry && (running.has(state) || intent === "enqueue" && queued.has(state))) return;
  const current = fingerprint(issue);
  if (previous?.phase === "pending" || previous?.phase === "accepted" && previous.fingerprint === current && previous.category === category) return;
  const action: Action = {
    issue, category: retry?.category ?? category!, fingerprint: current, phase: "pending",
    commandId: retry?.commandId ?? crypto.randomUUID(),
  };
  publish(action);
  try {
    const result = await requireWorkflowBridge().postJson("/multica/native-executions/intent", {
      workspaceId: issue.workspace_id, threadId: issue.metadata?.ccp_thread_id, intent: nativeExecutionIntent(action.category),
      prompt: DEFAULT_CONTINUATION, idempotencyKey: action.commandId,
    });
    if (!result || typeof result !== "object" || !("status" in result) || result.status !== "ok") throw new Error("native_intent_failed");
    publish({ ...action, phase: "accepted" });
  } catch { publish({ ...action, phase: "error" }); }
}
