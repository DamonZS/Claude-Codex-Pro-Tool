import { describe, expect, it, vi } from "vitest";
import { submitNativeExecutionIntent, nativeExecutionIntent } from "./native-execution-actions";
const issue = (thread: string, state = "completed") => ({
  id: `codex-native:${thread}`, workspace_id: "workspace", updated_at: "2026-09-19T00:00:00Z",
  metadata: { ccp_source: "codex-native", ccp_thread_id: thread, ccp_execution_state: state },
});
describe("native execution drag intents", () => {
  it("deduplicates pending and accepted drops until a fresh result permits continuation", async () => {
    let finish!: (value: unknown) => void;
    const postJson = vi.fn().mockImplementation(() => new Promise(resolve => { finish = resolve; }));
    window.__CODEX_WORKFLOW_BRIDGE__ = { postJson };
    const card = issue("dedup", "failed");
    const first = submitNativeExecutionIntent(card, "in_progress");
    await submitNativeExecutionIntent(card, "in_progress");
    expect(postJson).toHaveBeenCalledTimes(1);
    finish({ status: "ok" }); await first;
    await submitNativeExecutionIntent(card, "in_progress");
    expect(postJson).toHaveBeenCalledTimes(1);
    postJson.mockResolvedValue({ status: "ok" });
    await submitNativeExecutionIntent({ ...card, updated_at: "2026-09-19T01:00:00Z" }, "in_progress");
    expect(postJson).toHaveBeenCalledTimes(2);
  });
  it("guards running/queued work and unsupported cards or result columns", async () => {
    const postJson = vi.fn().mockResolvedValue({ status: "ok" });
    window.__CODEX_WORKFLOW_BRIDGE__ = { postJson };
    for (const state of ["inProgress", "running", "cancel_pending"]) {
      await submitNativeExecutionIntent(issue(`busy-${state}`, state), "in_progress");
      await submitNativeExecutionIntent(issue(`busy-${state}`, state), "todo");
    }
    await submitNativeExecutionIntent(issue("queued", "queued"), "todo");
    await submitNativeExecutionIntent(issue("done"), "done");
    await submitNativeExecutionIntent({ ...issue("other"), metadata: { ccp_source: "multica-execution" } }, "todo");
    expect(postJson).not.toHaveBeenCalled();
    await submitNativeExecutionIntent(issue("queued", "queued"), "in_progress");
    expect(postJson).toHaveBeenCalledTimes(1);
  });
  it("enqueues real work and preserves the command identity across a failed retry", async () => {
    const postJson = vi.fn().mockResolvedValueOnce({ status: "failed" }).mockResolvedValue({ status: "ok" });
    window.__CODEX_WORKFLOW_BRIDGE__ = { postJson };
    const card = issue("retry");
    await submitNativeExecutionIntent(card, "todo");
    await submitNativeExecutionIntent(card, "todo");
    expect(postJson).toHaveBeenCalledTimes(2);
    expect(postJson.mock.calls[0][1]).toEqual(postJson.mock.calls[1][1]);
    expect(postJson.mock.calls[1][1]).toMatchObject({ intent: "enqueue", threadId: "retry" });
  });
  it("promotes an accepted enqueue to continue before projection refresh, while deduplicating the same intent", async () => {
    const postJson = vi.fn().mockResolvedValue({ status: "ok" });
    window.__CODEX_WORKFLOW_BRIDGE__ = { postJson };
    const card = issue("promote");
    await submitNativeExecutionIntent(card, "todo");
    await submitNativeExecutionIntent(card, "todo");
    await submitNativeExecutionIntent(card, "in_progress");
    await submitNativeExecutionIntent(card, "in_progress");
    expect(postJson).toHaveBeenCalledTimes(2);
    expect(postJson.mock.calls.map(call => call[1].intent)).toEqual(["enqueue", "continue"]);
    expect(postJson.mock.calls[0][1].idempotencyKey).not.toBe(postJson.mock.calls[1][1].idempotencyKey);
  });
  it("reconciles the unresolved command with its original intent after refreshed running evidence", async () => {
    const postJson = vi.fn().mockResolvedValueOnce({ status: "failed" }).mockResolvedValue({ status: "ok" });
    window.__CODEX_WORKFLOW_BRIDGE__ = { postJson };
    const card = issue("recovery-running");
    await submitNativeExecutionIntent(card, "in_progress");
    await submitNativeExecutionIntent({ ...card, updated_at: "2026-09-19T01:00:00Z", metadata: { ...card.metadata, ccp_execution_state: "running" } }, "todo");
    expect(postJson).toHaveBeenCalledTimes(2);
    expect(postJson.mock.calls[1]).toEqual(postJson.mock.calls[0]);
  });
  it("retries transport errors and separates command identity by workspace", async () => {
    const postJson = vi.fn().mockRejectedValueOnce(new Error("bridge_timeout")).mockResolvedValue({ status: "ok" });
    window.__CODEX_WORKFLOW_BRIDGE__ = { postJson };
    const card = issue("network-retry");
    await submitNativeExecutionIntent(card, "in_progress");
    await submitNativeExecutionIntent(card, "in_progress");
    expect(postJson.mock.calls[0][1]).toEqual(postJson.mock.calls[1][1]);
    await submitNativeExecutionIntent({ ...card, workspace_id: "another-workspace" }, "in_progress");
    expect(postJson).toHaveBeenCalledTimes(3);
    expect(postJson.mock.calls[2][1]).toMatchObject({ workspaceId: "another-workspace", threadId: "network-retry" });
    expect(postJson.mock.calls[2][1].idempotencyKey).not.toBe(postJson.mock.calls[1][1].idempotencyKey);
  });
  it("maps only actionable columns and continues the child with the default instruction", async () => {
    expect(nativeExecutionIntent("in_progress")).toBe("continue");
    expect(nativeExecutionIntent("todo")).toBe("enqueue");
    expect(nativeExecutionIntent("done")).toBeNull();
    const postJson = vi.fn().mockResolvedValue({ status: "ok" });
    window.__CODEX_WORKFLOW_BRIDGE__ = { postJson };
    await submitNativeExecutionIntent(issue("continue"), "in_progress");
    expect(postJson).toHaveBeenCalledExactlyOnceWith("/multica/native-executions/intent", {
      workspaceId: "workspace", threadId: "continue", intent: "continue",
      prompt: "继续完成当前任务，检查剩余工作并完成验证；若已全部完成，简要确认结果。",
      idempotencyKey: expect.any(String),
    });
  });
});
