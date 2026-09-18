import type { QueryClient } from "@tanstack/react-query";
import { agentBuilderSessionKeys } from "@multica/core/agents/queries";
import { chatKeys } from "@multica/core/chat/queries";

type RecordValue = Record<string, unknown>;
const object = (value: unknown): RecordValue | undefined =>
  value !== null && typeof value === "object" && !Array.isArray(value) ? value as RecordValue : undefined;
const id = (value: unknown): string | undefined =>
  typeof value === "string" && value.length > 0 && value.length <= 240 && !/[\x00-\x1f\x7f]/.test(value) ? value : undefined;
const methods = new Set([
  "turn/started", "turn/completed", "item/started", "item/completed",
  "item/agentMessage/delta", "thread/status/changed", "error",
]);

/** Per-mounted-workspace invalidation, never a second event or execution server. */
export function createBuilderHostSync(client: QueryClient, workspaceId: string) {
  const bindings = new Map<string, { thread: string; turn?: string }>();
  const earlyThreads = new Set<string>();
  const dirty = new Set<string>();
  const running = new Set<string>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let disposed = false;

  const invalidate = (queryKey: readonly unknown[]) => client.invalidateQueries({ queryKey, exact: true, refetchType: "active" });
  async function refresh(sessionId: string) {
    running.add(sessionId);
    dirty.delete(sessionId);
    try {
      // Core reconciles the native transcript here. Only then may a cancelled
      // empty turn produce a durable input restore or update the draft preview.
      await Promise.all([
        invalidate(chatKeys.messages(sessionId)),
        invalidate(chatKeys.messagesPage(sessionId)),
        invalidate(chatKeys.pendingTask(sessionId)),
      ]);
      if (!disposed) await Promise.all([
        invalidate(chatKeys.draftRestores(sessionId)),
        invalidate(chatKeys.session(workspaceId, sessionId)),
        invalidate(agentBuilderSessionKeys.list(workspaceId)),
      ]);
    } finally {
      running.delete(sessionId);
      if (!disposed && dirty.size) schedule();
    }
  }
  function schedule() {
    if (disposed || timer !== undefined) return;
    timer = setTimeout(() => {
      timer = undefined;
      for (const sessionId of dirty) {
        if (!running.has(sessionId)) void refresh(sessionId).catch(() => {});
      }
    }, 50);
  }
  function queue(sessionId: string) {
    dirty.add(sessionId);
    if (!running.has(sessionId)) schedule();
  }
  function bind(sessionId: string, value: RecordValue) {
    const thread = id(value.native_thread_id), turn = id(value.native_turn_id);
    if (!thread) return;
    bindings.set(sessionId, { thread, turn });
    if (earlyThreads.delete(thread)) queue(sessionId);
  }

  return {
    /** Observe successful Core replies BEFORE returning them to the API adapter.
     * This preserves native IDs that upstream schema/optimistic writes omit. */
    observeReply(requestValue: unknown, resultValue: unknown) {
      if (disposed) return;
      const request = object(requestValue), result = object(resultValue);
      if (!request || !result || result.status === "failed" || result.status === "error") return;
      const sessionId = id(request.sessionId);
      if (request.operation === "delete" && sessionId) {
        bindings.delete(sessionId); dirty.delete(sessionId);
      } else if (["send", "pending_task"].includes(String(request.operation)) && sessionId) {
        bind(sessionId, result);
      } else if (["get", "create", "list"].includes(String(request.operation))) {
        const sessions = request.operation === "list" ? result.sessions : [result];
        if (Array.isArray(sessions)) for (const value of sessions) {
          const session = object(value), sid = id(session?.session_id);
          if (session?.workspace_id === workspaceId && sid) bind(sid, session);
        }
      }
    },
    /** Only feed notifications from the audited current-page native client.
     * Event contents are never copied into messages, logs, or draft state. */
    nativeEvent(value: unknown) {
      if (disposed) return;
      const event = object(value), params = object(event?.params);
      if (!event || !methods.has(String(event.method)) || !params) return;
      const thread = id(params.threadId), turn = id(params.turnId) ?? id(object(params.turn)?.id);
      if (!thread) return;
      let matched = false;
      for (const [sessionId, binding] of bindings) {
        if (binding.thread !== thread) continue;
        if (!turn || !binding.turn || binding.turn === turn) {
          matched = true;
          queue(sessionId);
        }
      }
      // A fast completion can precede the send response that teaches us the
      // binding. Retain IDs only, bounded; never cache raw event payloads.
      if (!matched) {
        earlyThreads.add(thread);
        if (earlyThreads.size > 128) earlyThreads.delete(earlyThreads.values().next().value!);
      }
    },
    reconnect() {
      if (disposed) return;
      for (const sessionId of bindings.keys()) queue(sessionId);
    },
    dispose() {
      disposed = true;
      if (timer !== undefined) clearTimeout(timer);
      timer = undefined;
      dirty.clear(); bindings.clear(); earlyThreads.clear();
    },
  };
}

export type BuilderHostSync = ReturnType<typeof createBuilderHostSync>;
