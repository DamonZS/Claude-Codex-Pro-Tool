import { afterEach, describe, expect, it } from "vitest";
import { waitFor } from "@testing-library/react";
import { QueryClient, QueryObserver, type QueryObserverOptions } from "@tanstack/react-query";
import { ApiClient, setApiInstance } from "@multica/core/api";
import { chatDraftRestoresOptions, chatKeys, chatMessagesOptions, pendingChatTaskOptions } from "@multica/core/chat/queries";
import { agentBuilderSessionKeys, agentBuilderSessionListOptions } from "@multica/core/agents/queries";
import { connectWorkflowHost, disconnectWorkflowHost } from "./upstream-host";
import { createBuilderHostSync } from "./builder-host-sync";

const cleanup: Array<() => void> = [];
afterEach(() => { cleanup.splice(0).reverse().forEach((stop) => stop()); disconnectWorkflowHost(); });
const pause = () => new Promise((resolve) => setTimeout(resolve, 90));
const event = (turn = "turn", method = "turn/completed") => ({ method, params: { threadId: "thread", turnId: turn } });

async function fixture() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
  const sync = createBuilderHostSync(client, "workspace");
  cleanup.push(() => client.clear(), () => sync.dispose());
  const source = {
    messages: [] as unknown[], pending: { task_id: "task", supports_queue: false } as Record<string, unknown>,
    restores: [] as unknown[], transcriptGate: undefined as Promise<void> | undefined,
    failMessages: false,
  };
  const calls: string[] = [];
  connectWorkflowHost(async (path) => {
    calls.push(path);
    let value: unknown;
    if (path === "/api/chat/sessions/session/messages") {
      if (source.transcriptGate) await source.transcriptGate;
      if (source.failMessages) return new Response(JSON.stringify({ code: "builder_transcript_unavailable" }), { status: 503 });
      value = source.messages;
    } else if (path.endsWith("/pending-task")) value = source.pending;
    else if (path.endsWith("/draft-restores")) value = { restores: source.restores };
    else if (path === "/api/agent-builder/sessions") value = { sessions: [] };
    else throw new Error(`Unexpected fixture endpoint: ${path}`);
    return new Response(JSON.stringify(value), { headers: { "Content-Type": "application/json" } });
  }, document.createElement("div"));
  setApiInstance(new ApiClient(""));
  async function watch<T, K extends readonly unknown[]>(options: QueryObserverOptions<T, Error, T, T, K>) {
    await client.fetchQuery(options);
    const observer = new QueryObserver(client, options);
    cleanup.push(observer.subscribe(() => {}));
  }
  await watch(chatMessagesOptions("session"));
  await watch(pendingChatTaskOptions("session"));
  await watch(chatDraftRestoresOptions("session"));
  await watch(agentBuilderSessionListOptions("workspace"));
  await waitFor(() => expect(client.isFetching()).toBe(0));
  calls.length = 0;
  const bind = (turn = "turn") => sync.observeReply({ operation: "send", sessionId: "session" }, { native_thread_id: "thread", native_turn_id: turn });
  return { client, sync, source, calls, bind };
}

describe("Builder native host query synchronization", () => {
  it("refreshes original Infinity-stale queries through the actual API schemas, without inventing an answer", async () => {
    const f = await fixture();
    f.bind();
    f.client.setQueryData(["issues", "workspace"], [{ id: "unrelated" }]);
    f.client.setQueryData(chatKeys.messagesPage("session"), { pages: [], pageParams: [null] });
    f.source.messages = [{ id: "answer", chat_session_id: "session", role: "assistant", task_id: "task", created_at: "2026-09-18T00:00:00.000Z", content: 'Draft <agent_draft>{"name":"Reviewer"}</agent_draft>' }];
    f.source.pending = { supports_queue: false };
    f.sync.nativeEvent({ ...event(), params: { ...event().params, text: "not the answer" } });
    await waitFor(() => expect(f.client.getQueryData(chatKeys.messages("session"))).toMatchObject(f.source.messages));
    expect(f.client.getQueryData(chatKeys.pendingTask("session"))).toEqual({ supports_queue: false });
    expect(f.client.getQueryState(["issues", "workspace"])?.isInvalidated).toBe(false);
    expect(f.client.getQueryState(chatKeys.messagesPage("session"))?.isInvalidated).toBe(true);
    expect(JSON.stringify(f.client.getQueryData(chatKeys.messages("session")))).not.toContain("not the answer");
    expect(f.calls).toContain("/api/agent-builder/sessions");
  });

  it("coalesces delta bursts and only refreshes draft restores after the native read settles", async () => {
    const f = await fixture();
    f.bind();
    let release!: () => void;
    f.source.transcriptGate = new Promise<void>((resolve) => { release = resolve; });
    for (let i = 0; i < 50; i++) f.sync.nativeEvent(event("turn", "item/agentMessage/delta"));
    await waitFor(() => expect(f.calls.filter((path) => path.endsWith("/messages"))).toHaveLength(1));
    expect(f.calls.some((path) => path.endsWith("/draft-restores"))).toBe(false);
    f.source.restores = [{ id: "restore", chat_session_id: "session", content: "Preserved input" }];
    release();
    await waitFor(() => expect(f.client.getQueryData(chatKeys.draftRestores("session"))).toEqual({ restores: f.source.restores }));
    expect(f.calls.filter((path) => path.endsWith("/messages"))).toHaveLength(1);
  });

  it("does a follow-up read when a completion arrives during an in-flight refresh", async () => {
    const f = await fixture();
    f.bind();
    let release!: () => void;
    f.source.transcriptGate = new Promise<void>((resolve) => { release = resolve; });
    f.sync.nativeEvent(event("turn", "item/agentMessage/delta"));
    await waitFor(() => expect(f.calls.filter((path) => path.endsWith("/messages"))).toHaveLength(1));
    f.sync.nativeEvent(event());
    release();
    await waitFor(() => expect(f.calls.filter((path) => path.endsWith("/messages"))).toHaveLength(2));
  });

  it("handles fast completion before send returns, including the next turn on the same thread", async () => {
    const f = await fixture();
    f.sync.nativeEvent(event());
    f.bind();
    await waitFor(() => expect(f.calls.filter((path) => path.endsWith("/messages"))).toHaveLength(1));
    await waitFor(() => expect(f.client.isFetching()).toBe(0));
    f.sync.nativeEvent(event("next-turn"));
    f.bind("next-turn");
    await waitFor(() => expect(f.calls.filter((path) => path.endsWith("/messages"))).toHaveLength(2));
  });

  it("reconstructs the binding from a workspace-validated Core session after reload", async () => {
    const f = await fixture();
    f.sync.observeReply({ operation: "get", sessionId: "session" }, { workspace_id: "other", session_id: "session", native_thread_id: "thread", native_turn_id: "turn" });
    f.sync.nativeEvent(event());
    await pause();
    expect(f.calls).toHaveLength(0);
    f.sync.observeReply({ operation: "get", sessionId: "session" }, { workspace_id: "workspace", session_id: "session", native_thread_id: "thread", native_turn_id: "turn" });
    await waitFor(() => expect(f.calls).toContain("/api/chat/sessions/session/messages"));
    await waitFor(() => expect(f.client.isFetching()).toBe(0));
    f.calls.length = 0;
    f.sync.reconnect();
    await waitFor(() => expect(f.calls).toContain("/api/chat/sessions/session/messages"));
  });

  it("ignores malformed, unrelated and previous-turn notifications", async () => {
    const f = await fixture();
    f.bind();
    for (const value of [null, {}, { method: "unknown", params: { threadId: "thread" } }, { method: "turn/completed", params: { threadId: "other" } }, event("old-turn")]) f.sync.nativeEvent(value);
    await pause();
    expect(f.calls).toHaveLength(0);
  });

  it("does not fabricate idle state or erase history when the native read fails", async () => {
    const f = await fixture();
    f.bind();
    f.client.setQueryData(chatKeys.messages("session"), [{ id: "old", content: "Keep" }]);
    f.source.failMessages = true;
    f.sync.nativeEvent(event());
    await waitFor(() => expect(f.client.getQueryState(chatKeys.messages("session"))?.status).toBe("error"));
    expect(f.client.getQueryData(chatKeys.messages("session"))).toEqual([{ id: "old", content: "Keep" }]);
    expect(f.client.getQueryData(chatKeys.pendingTask("session"))).toMatchObject({ task_id: "task" });
  });

  it("disposes queued work and a completed delete removes the event binding", async () => {
    const f = await fixture();
    f.bind();
    f.sync.observeReply({ operation: "delete", sessionId: "session" }, { session_id: "session" });
    f.sync.nativeEvent(event());
    await pause();
    expect(f.calls).toHaveLength(0);
    f.bind();
    f.sync.nativeEvent(event());
    f.sync.dispose();
    f.sync.reconnect();
    await pause();
    expect(f.calls).toHaveLength(0);
    expect(f.client.getQueryState(agentBuilderSessionKeys.list("workspace"))?.status).toBe("success");
  });

  it("does not schedule recovery queries after disposal during a native read", async () => {
    const f = await fixture();
    f.bind();
    let release!: () => void;
    f.source.transcriptGate = new Promise<void>((resolve) => { release = resolve; });
    f.sync.nativeEvent(event());
    await waitFor(() => expect(f.calls).toContain("/api/chat/sessions/session/messages"));
    f.sync.nativeEvent(event());
    f.sync.dispose();
    release();
    await waitFor(() => expect(f.client.isFetching()).toBe(0));
    await pause();
    expect(f.calls.filter((path) => path.endsWith("/messages"))).toHaveLength(1);
    expect(f.calls.some((path) => path.endsWith("/draft-restores"))).toBe(false);
  });
});
