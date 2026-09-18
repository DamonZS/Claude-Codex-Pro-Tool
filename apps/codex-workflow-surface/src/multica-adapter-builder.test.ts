// @vitest-environment node
import { describe, expect, it, vi } from "vitest";
import { MulticaApiAdapter } from "./multica-api-adapter";
import { type JsonRecord } from "./multica-adapter-dto";
import { AgentBuilderSessionSchema, AgentBuilderSessionListSchema, ChatMessagesPageSchema, ChatSessionSchema, SendChatMessageResponseSchema } from "../vendor/multica/packages/core/api/schemas";
import { ApiClient } from "../vendor/multica/packages/core/api/client";
import * as workflowHost from "./upstream-host";

const date = "2026-09-18T00:00:00.000Z";
const init = (method: string, data: JsonRecord = {}, key?: string): RequestInit => ({ method, body: JSON.stringify(data), ...(key ? { headers: { "Idempotency-Key": key } } : {}) });

function fixture() {
  const session: JsonRecord = { id: "session-1", session_id: "session-1", builder_agent_id: "builder-session-1", agent_id: "builder-session-1", workspace_id: "workspace-1", creator_id: "user-1", runtime_id: "runtime-1", title: "", draft: null, revision: 1, created_at: date, updated_at: date, status: "active", has_unread: false, last_message_content: "", last_message_role: "", last_message_at: "" };
  const messages: JsonRecord[] = [];
  const sends = new Map<string, JsonRecord>();
  let pending = false;
  const postJson = vi.fn(async (path: string, payload: JsonRecord): Promise<unknown> => {
    if (path === "/multica/workspace/bootstrap") return { status: "ok", workspace: { id: "workspace-1", slug: "local" }, user: { id: "user-1" } };
    if (path === "/multica/executions/list") return { status: "ok", items: [], total: 0 };
    if (path !== "/multica/builder") return { status: "failed", code: "capability_unavailable" };
    if (payload.operation === "list") return { sessions: [structuredClone(session)] };
    if (payload.operation === "create" || payload.operation === "get") return structuredClone(session);
    if (payload.operation === "messages") return structuredClone(messages);
    if (payload.operation === "pending_task") return pending ? { task_id: "task-1", status: "running", created_at: date, supports_queue: false, native_thread_id: "thread-1", native_turn_id: "turn-1" } : { supports_queue: false };
    if (payload.operation === "cancel") {
      pending = false;
      return { id: "task-1", agent_id: session.agent_id, runtime_id: session.runtime_id, issue_id: "", status: "cancelled", priority: 0, created_at: date };
    }
    if (payload.operation === "draft_restores") return { restores: [] };
    if (payload.operation === "send" && sends.has(String(payload.idempotencyKey))) return sends.get(String(payload.idempotencyKey));
    if (payload.expectedRevision !== session.revision) return { status: "failed", code: "builder_revision_conflict" };
    session.revision = Number(session.revision) + 1;
    switch (payload.operation) {
      case "save_draft": session.draft = payload.draft; break;
      case "update": session.title = payload.title; break;
      case "archive": session.status = payload.archived ? "archived" : "active"; break;
      case "switch_runtime": session.runtime_id = payload.runtimeId; break;
      case "send": {
        pending = true;
        const result = { message_id: "message-1", task_id: "task-1", created_at: date, supports_queue: false, queued: false, native_thread_id: "thread-1", native_turn_id: "turn-1" };
        sends.set(String(payload.idempotencyKey), result);
        messages.push({ id: "message-1", chat_session_id: "session-1", role: "user", content: payload.content, task_id: "task-1", created_at: date });
        return result;
      }
    }
    return structuredClone(session);
  });
  const bridge = { postJson };
  return { session, messages, sends, postJson, bridge, adapter: new MulticaApiAdapter(bridge) };
}

describe("Core BuilderRequest to original upstream Builder/chat contracts", () => {
  it("runs the unmodified vendored API client create/send/read/cancel/archive lifecycle", async () => {
    const f = fixture();
    const transport = vi.spyOn(workflowHost, "workflowFetch").mockImplementation((path, request) => f.adapter.transport(String(path), request));
    try {
      const client = new ApiClient("");
      const session = await client.createAgentBuilderSession({ runtime_id: "runtime-1" });
      expect(session.session_id).toBe("session-1");
      const sent = await client.sendChatMessage(session.session_id, "Draft a reviewer");
      expect(sent.task_id).toBe("task-1");
      const messages = await client.listChatMessagesPage(session.session_id, { limit: 50 });
      expect(messages.messages[0].content).toBe("Draft a reviewer");
      const cancelled = await client.cancelTaskById(sent.task_id);
      expect(cancelled).toMatchObject({ id: "task-1", status: "cancelled" });
      const archived = await client.setChatSessionArchived(session.session_id, true);
      expect(archived.status).toBe("archived");
      expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/builder").map(([, payload]) => payload.operation)).toEqual(["create", "get", "send", "messages", "cancel", "get", "archive"]);
    } finally {
      transport.mockRestore();
    }
  });
  it("creates and restores the original Builder DTO then saves and switches using observed CAS", async () => {
    const f = fixture();
    const created = await (await f.adapter.transport("/api/agent-builder/sessions", init("POST", { runtime_id: "runtime-1" }, "create-builder"))).json();
    expect(AgentBuilderSessionSchema.safeParse(created).success).toBe(true);
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/builder", { operation: "create", runtimeId: "runtime-1", model: "", idempotencyKey: "create-builder" }]);
    const draft = { name: "Reviewer", description: "Reviews changes", instructions: "Review the diff", permission_scope: "private", skill_ids: [], conversation_starters: [] };
    expect((await f.adapter.transport("/api/agent-builder/sessions/session-1/draft", init("PUT", { draft }))).status).toBe(204);
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/builder", { operation: "save_draft", sessionId: "session-1", expectedRevision: 1, draft }]);
    const restored = await (await new MulticaApiAdapter(f.bridge).transport("/api/agent-builder/sessions")).json();
    expect(AgentBuilderSessionListSchema.safeParse(restored).success).toBe(true);
    expect(restored.sessions[0]).toMatchObject({ draft, revision: 2 });
    expect((await f.adapter.transport("/api/agent-builder/sessions/session-1/runtime", init("PATCH", { runtime_id: "runtime-1" }))).status).toBe(200);
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/builder", { operation: "switch_runtime", sessionId: "session-1", expectedRevision: 2, runtimeId: "runtime-1" }]);
    const read = await (await f.adapter.transport("/api/chat/sessions/session-1")).json();
    expect(ChatSessionSchema.safeParse(read).success).toBe(true);
  });
  it("sends once per native key, returns real IDs and cancels the same task after adapter reconstruction", async () => {
    const f = fixture();
    const request = init("POST", { content: "Build a reviewer" }, "native-send");
    const result = await (await f.adapter.transport("/api/chat/sessions/session-1/messages", request)).json();
    expect(SendChatMessageResponseSchema.safeParse(result).success).toBe(true);
    expect(result).toMatchObject({ message_id: "message-1", task_id: "task-1", native_thread_id: "thread-1" });
    expect(await (await new MulticaApiAdapter(f.bridge).transport("/api/chat/sessions/session-1/messages", request)).json()).toEqual(result);
    expect(f.messages).toHaveLength(1);
    const pending = await (await f.adapter.transport("/api/chat/sessions/session-1/pending-task")).json();
    expect(pending).toMatchObject({ task_id: "task-1", supports_queue: false, native_thread_id: "thread-1" });
    const cancelled = await new MulticaApiAdapter(f.bridge).transport("/api/tasks/task-1/cancel", { method: "POST", headers: { "Idempotency-Key": "cancel-native", "X-Client-Capabilities": "chat-draft-restore-v1" } });
    expect(cancelled.status).toBe(200);
    expect(await cancelled.json()).toMatchObject({ id: "task-1", status: "cancelled" });
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/builder", { operation: "cancel", sessionId: "session-1", taskId: "task-1", idempotencyKey: "cancel-native" }]);
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/executions/create")).toBe(false);
    expect((await f.adapter.transport("/api/tasks/task-1/cancel", { method: "POST", headers: { "X-Client-Capabilities": "arbitrary-capability" } })).status).toBe(400);
    expect((await f.adapter.transport("/api/agents", { headers: { "X-Client-Capabilities": "chat-draft-restore-v1" } })).status).toBe(400);
  });
  it("uses the original PATCH archive method after creation and CAS for title/delete", async () => {
    const f = fixture();
    expect((await f.adapter.transport("/api/chat/sessions/session-1", init("PATCH", { title: "Reviewer draft" }))).status).toBe(200);
    expect((await f.adapter.transport("/api/chat/sessions/session-1/archive", init("PATCH", { archived: true }))).status).toBe(200);
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/builder", { operation: "archive", sessionId: "session-1", expectedRevision: 2, archived: true }]);
    expect((await f.adapter.transport("/api/chat/sessions/session-1", init("DELETE"))).status).toBe(204);
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/builder", { operation: "delete", sessionId: "session-1", expectedRevision: 3 }]);
  });
  it("paginates only persisted native messages and rejects fabricated or stale cursors", async () => {
    const f = fixture();
    f.messages.push(...[1, 2, 3].map((id) => ({ id: `message-${id}`, chat_session_id: "session-1", role: id % 2 ? "user" : "assistant", content: `Content ${id}`, task_id: "task-1", created_at: date })));
    const page = await (await f.adapter.transport("/api/chat/sessions/session-1/messages/page?limit=2")).json();
    expect(ChatMessagesPageSchema.safeParse(page).success).toBe(true);
    expect(page).toMatchObject({ messages: [{ id: "message-2" }, { id: "message-3" }], has_more: true, next_cursor: { id: "message-2", created_at: date } });
    const older = await (await f.adapter.transport(`/api/chat/sessions/session-1/messages/page?limit=2&before_id=message-2&before_created_at=${encodeURIComponent(date)}`)).json();
    expect(older).toMatchObject({ messages: [{ id: "message-1" }], has_more: false, next_cursor: null });
    expect((await f.adapter.transport(`/api/chat/sessions/session-1/messages/page?before_id=missing&before_created_at=${encodeURIComponent(date)}`)).status).toBe(409);
    f.messages[0].chat_session_id = "other-session";
    expect((await f.adapter.transport("/api/chat/sessions/session-1/messages")).status).toBe(502);
  });
  it("keeps Core ownership, revision, offline and model-override errors as errors", async () => {
    const f = fixture();
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => payload.operation === "create" ? { status: "failed", code: "builder_model_override_unsupported" } : original(path, payload));
    const model = await f.adapter.transport("/api/agent-builder/sessions", init("POST", { runtime_id: "runtime-1", model: "other-model" }));
    expect(model.status).toBe(503);
    expect((await f.adapter.transport("/api/chat/sessions/session-1", init("PATCH", { title: "Stale", expected_revision: 9 }))).status).toBe(409);
    f.session.creator_id = "other-user";
    expect((await f.adapter.transport("/api/chat/sessions/session-1")).status).toBe(403);
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/builder" ? { status: "failed", code: "codex_host_unavailable" } : original(path, payload));
    expect((await f.adapter.transport("/api/agent-builder/sessions")).status).toBe(503);
    expect((await f.adapter.transport("/api/agent-builder/sessions/session-1/draft", init("PUT", { draft: { env: { EXAMPLE: "value" } } }))).status).toBe(400);
  });
});
