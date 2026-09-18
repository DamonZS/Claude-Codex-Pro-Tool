import { AdapterError, identifier, integer, keys, record, text, timestamp, type JsonRecord } from "./multica-adapter-dto";

type Context = { commandId: string; workspaceId: string; userId: string; signal?: AbortSignal | null };
type Call = (payload: JsonRecord, signal?: AbortSignal | null) => Promise<unknown>;
const controls = ["command_id", "idempotency_key", "expected_revision"];
const reply = (value: unknown, status = 200) => new Response(status === 204 ? null : JSON.stringify(value), { status, headers: { "Content-Type": "application/json" } });
const unavailable = () => { throw new AdapterError(503, "capability_unavailable"); };

export class MulticaBuilderAdapter {
  private readonly taskSessions = new Map<string, string>();

  constructor(private readonly call: Call) {}

  knowsTask(id: string): boolean { return this.taskSessions.has(id); }

  private session(value: unknown, context: Context): JsonRecord {
    const source = record(value);
    if (typeof source.session_id !== "string" || typeof source.runtime_id !== "string" || typeof source.builder_agent_id !== "string" || !Number.isSafeInteger(source.revision) || Number(source.revision) < 1) throw new AdapterError(502, "invalid_builder_session");
    if (source.workspace_id !== context.workspaceId || source.creator_id !== context.userId) throw new AdapterError(403, "workspace_mismatch");
    return source;
  }

  private async list(context: Context): Promise<JsonRecord[]> {
    const result = record(await this.call({ operation: "list" }, context.signal));
    if (!Array.isArray(result.sessions)) throw new AdapterError(502, "invalid_builder_list");
    return result.sessions.map((item) => this.session(item, context));
  }

  async route(path: string, method: string, params: JsonRecord, data: JsonRecord, context: Context): Promise<Response | null> {
    const parts = path.split("/").slice(2);
    if (parts[0] === "tasks") {
      if (method !== "POST") return null;
      keys(params, []);
      keys(data, controls);
      const taskId = identifier(parts[1]);
      let sessionId = this.taskSessions.get(taskId);
      // Reconstruct the native task/session association after a surface reload.
      if (!sessionId) for (const session of await this.list(context)) {
        const pending = record(await this.call({ operation: "pending_task", sessionId: session.session_id }, context.signal));
        if (pending.task_id === taskId) { sessionId = text(session.session_id); break; }
      }
      if (!sessionId) return null;
      return reply(await this.call({ operation: "cancel", sessionId, taskId, idempotencyKey: context.commandId }, context.signal));
    }
    const builder = parts[0] === "agent-builder";
    if (parts[1] !== "sessions") return unavailable();
    if (parts.length === 2) {
      keys(params, []);
      if (method === "GET" && builder) return reply({ sessions: await this.list(context) });
      if (method === "POST" && builder) {
        keys(data, ["runtime_id", "model", ...controls]);
        if (data.model !== undefined && typeof data.model !== "string") throw new AdapterError(400, "invalid_builder_model");
        return reply(this.session(await this.call({ operation: "create", runtimeId: identifier(data.runtime_id), model: data.model ?? "", idempotencyKey: context.commandId }, context.signal), context));
      }
      return unavailable();
    }
    const sessionId = identifier(parts[2]), action = parts[3], child = parts[4];
    if (parts.length > 5) return unavailable();
    if (method === "GET" && action === "messages" && (!child || child === "page") && !builder) {
      keys(params, child ? ["limit", "before_created_at", "before_id"] : []);
      const result = await this.call({ operation: "messages", sessionId }, context.signal);
      if (!Array.isArray(result)) throw new AdapterError(502, "invalid_builder_messages");
      let messages = result.map((value) => {
        const message = record(value);
        if (message.chat_session_id !== sessionId || typeof message.id !== "string" || !["user", "assistant"].includes(text(message.role)) || typeof message.content !== "string" || !timestamp(message.created_at)) throw new AdapterError(502, "invalid_builder_message");
        if (typeof message.task_id === "string") this.taskSessions.set(message.task_id, sessionId);
        return message;
      });
      if (!child) return reply(messages);
      const limit = integer(params.limit, 50, 500);
      if (!limit || !!params.before_created_at !== !!params.before_id) throw new AdapterError(400, "invalid_message_cursor");
      messages.sort((a, b) => text(a.created_at).localeCompare(text(b.created_at)) || text(a.id).localeCompare(text(b.id)));
      if (params.before_id) {
        const index = messages.findIndex((message) => message.id === params.before_id && message.created_at === params.before_created_at);
        if (index < 0) throw new AdapterError(409, "message_cursor_conflict");
        messages = messages.slice(0, index);
      }
      const hasMore = messages.length > limit, selected = messages.slice(-limit);
      return reply({ messages: selected, limit, has_more: hasMore, next_cursor: hasMore ? { created_at: selected[0].created_at, id: selected[0].id } : null });
    }
    if (!builder && method === "GET" && action === "pending-task" && !child) {
      keys(params, []);
      const pending = record(await this.call({ operation: "pending_task", sessionId }, context.signal));
      if (typeof pending.supports_queue !== "boolean") throw new AdapterError(502, "invalid_builder_pending_task");
      if (typeof pending.task_id === "string") this.taskSessions.set(pending.task_id, sessionId);
      return reply(pending);
    }
    if (!builder && action === "draft-restores") {
      keys(params, []); keys(data, controls);
      if (method === "GET" && !child) return reply(await this.call({ operation: "draft_restores", sessionId }, context.signal));
      if (method === "DELETE" && child) {
        await this.call({ operation: "consume_restore", sessionId, restoreId: identifier(child) }, context.signal);
        return reply(null, 204);
      }
    }
    keys(params, []);
    const operation = builder && action === "draft" && method === "PUT" ? "save_draft"
      : builder && action === "runtime" && method === "PATCH" ? "switch_runtime"
      : !builder && action === "messages" && method === "POST" ? "send"
      : !builder && action === "archive" && method === "PATCH" ? "archive"
      : !builder && !action && method === "PATCH" ? "update"
      : !builder && !action && method === "DELETE" ? "delete"
      : !builder && !action && method === "GET" ? "get" : null;
    if (!operation || child) return unavailable();
    const allowed: Record<string, string[]> = { save_draft: ["draft"], switch_runtime: ["runtime_id"], send: ["content", "attachment_ids"], archive: ["archived"], update: ["title"], delete: [], get: [] };
    keys(data, [...controls, ...allowed[operation]]);
    const current = this.session(await this.call({ operation: "get", sessionId }, context.signal), context);
    if (operation === "get") return reply(current);
    const expectedRevision = integer(data.expected_revision ?? current.revision, 0, Number.MAX_SAFE_INTEGER);
    if (!expectedRevision) throw new AdapterError(409, "revision_required");
    const request: JsonRecord = { operation, sessionId, expectedRevision };
    if (operation === "save_draft") request.draft = record(data.draft);
    if (operation === "switch_runtime") request.runtimeId = identifier(data.runtime_id);
    if (operation === "update") {
      if (typeof data.title !== "string" || data.title.length > 500) throw new AdapterError(400, "invalid_builder_title");
      request.title = data.title;
    }
    if (operation === "archive") {
      if (typeof data.archived !== "boolean") throw new AdapterError(400, "invalid_builder_archive");
      request.archived = data.archived;
    }
    if (operation === "send") {
      if (data.attachment_ids !== undefined && (!Array.isArray(data.attachment_ids) || data.attachment_ids.length)) return unavailable();
      if (typeof data.content !== "string" || !data.content.trim()) throw new AdapterError(400, "invalid_builder_content");
      request.content = data.content;
      request.idempotencyKey = context.commandId;
    }
    const result = record(await this.call(request, context.signal));
    if (operation === "send") {
      if (typeof result.message_id !== "string" || typeof result.task_id !== "string" || !timestamp(result.created_at)) throw new AdapterError(502, "invalid_builder_send");
      this.taskSessions.set(result.task_id, sessionId);
    } else this.session(result, context);
    return reply(operation === "save_draft" || operation === "delete" ? null : result, operation === "save_draft" || operation === "delete" ? 204 : 200);
  }
}
