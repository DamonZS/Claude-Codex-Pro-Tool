import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NativeSubtasks } from "./native-subtasks";

const row = (name = "Descartes", status = "running") => ({
  id: `thread-${name}`, parent_thread_id: "parent-thread", agent_nickname: name,
  status, edge_status: "open", updated_at_ms: 1750000000000, source: "codex_native", read_only: true,
  title: "Private task instructions must not appear",
});
let client: QueryClient;
const postJson = vi.fn();
const openThread = vi.fn();
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
  postJson.mockReset().mockResolvedValue({ status: "ok", items: [row()], total: 1 });
  openThread.mockReset().mockResolvedValue(true);
  window.__CODEX_WORKFLOW_BRIDGE__ = { postJson, openThread };
});
afterEach(() => { cleanup(); client.clear(); delete window.__CODEX_WORKFLOW_BRIDGE__; vi.useRealTimers(); });
const mount = () => render(<QueryClientProvider client={client}><NativeSubtasks /></QueryClientProvider>);

describe("native Codex subtasks", () => {
  it("queries read-only metadata and opens child and parent through the native host", async () => {
    mount();
    fireEvent.click(await screen.findByRole("button", { name: "Descartes" }));
    await waitFor(() => expect(openThread).toHaveBeenLastCalledWith("thread-Descartes"));
    fireEvent.click(screen.getByRole("button", { name: "打开 Descartes 的父会话" }));
    await waitFor(() => expect(openThread).toHaveBeenLastCalledWith("parent-thread"));
    expect(screen.getByText("执行中")).toBeTruthy();
    expect(screen.queryByText(/Private task instructions/)).toBeNull();
    expect(postJson.mock.calls).toEqual([["/multica/workspace/query", { resource: "codex_native_agents", limit: 100, offset: 0 }]]);
  });

  it("shows loading, genuine empty state and retries a failed read", async () => {
    let resolve!: (value: unknown) => void;
    postJson.mockReturnValueOnce(new Promise((r) => { resolve = r; }));
    mount();
    expect(screen.getByText("正在读取原生子任务…")).toBeTruthy();
    await act(async () => resolve({ status: "failed" }));
    expect(await screen.findByRole("alert")).toBeTruthy();
    postJson.mockResolvedValue({ status: "ok", items: [], total: 0 });
    fireEvent.click(screen.getByRole("button", { name: "刷新子任务" }));
    expect(await screen.findByText("暂无原生子任务")).toBeTruthy();
  });

  it("retains cached rows after a refresh failure and marks them stale", async () => {
    mount();
    await screen.findByRole("button", { name: "Descartes" });
    postJson.mockRejectedValue(new Error("read failed"));
    fireEvent.click(screen.getByRole("button", { name: "刷新子任务" }));
    expect(await screen.findByText("子任务数据可能已过期，请刷新重试。")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Descartes" })).toBeTruthy();
  });

  it("does not mistake an open edge for execution and surfaces partial reads", async () => {
    postJson.mockResolvedValue({ status: "ok", items: [row("", "unknown")], total: 1, stale: true });
    mount();
    expect(await screen.findByText("状态待确认")).toBeTruthy();
    expect(screen.getByRole("button", { name: "thread-" })).toBeTruthy();
    expect(screen.getByText("子任务数据可能已过期，请刷新重试。")).toBeTruthy();
  });

  it("keeps the previous complete list when a database returns only partial rows", async () => {
    postJson.mockResolvedValueOnce({ status: "ok", items: [row("Descartes"), row("Tesla")], total: 2 });
    mount();
    await screen.findByRole("button", { name: "Tesla" });
    postJson.mockResolvedValue({ status: "ok", items: [row("Descartes")], total: 1, stale: true });
    fireEvent.click(screen.getByRole("button", { name: "刷新子任务" }));
    expect(await screen.findByText("子任务数据可能已过期，请刷新重试。")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Tesla" })).toBeTruthy();
    postJson.mockResolvedValue({ status: "ok", items: [row("Tesla", "completed")], total: 1 });
    fireEvent.click(screen.getByRole("button", { name: "刷新子任务" }));
    expect(await screen.findByText("已完成")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Descartes" })).toBeNull();
  });

  it.each([false, { status: "failed" }, { status: "error" }, { ok: false }])("shows a native open failure: %j", async (result) => {
    openThread.mockResolvedValue(result);
    mount();
    fireEvent.click(await screen.findByRole("button", { name: "Descartes" }));
    expect(await screen.findByText("会话打开失败，请重试。")).toBeTruthy();
  });

  it("expands recent rows without creating executions", async () => {
    postJson.mockResolvedValue({ status: "ok", items: ["Descartes", "Tesla", "Faraday", "Mendel", "Fifth"].map((n) => row(n)), total: 5 });
    mount();
    await screen.findByRole("button", { name: "Mendel" });
    expect(screen.queryByRole("button", { name: "Fifth" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "展开近期 5 个子任务" }));
    expect(screen.getByRole("button", { name: "Fifth" })).toBeTruthy();
    expect(openThread).not.toHaveBeenCalled();
  });

  it("refreshes while mounted and stops polling when leaving My Tasks", async () => {
    vi.useFakeTimers();
    const view = mount();
    await act(async () => { await vi.advanceTimersByTimeAsync(20); });
    expect(postJson).toHaveBeenCalledTimes(1);
    await act(async () => { await vi.advanceTimersByTimeAsync(5000); });
    expect(postJson).toHaveBeenCalledTimes(2);
    view.unmount();
    await act(async () => { await vi.advanceTimersByTimeAsync(15000); });
    expect(postJson).toHaveBeenCalledTimes(2);
  });

  it("pauses metadata reads when the native host hides the workflow and resumes on return", async () => {
    vi.useFakeTimers();
    const host = document.createElement("div");
    document.body.append(host);
    const container = document.createElement("div");
    host.attachShadow({ mode: "open" }).append(container);
    const view = render(<QueryClientProvider client={client}><NativeSubtasks /></QueryClientProvider>, { container });
    try {
      await act(async () => { await vi.advanceTimersByTimeAsync(20); });
      expect(postJson).toHaveBeenCalledTimes(1);
      await act(async () => { host.style.display = "none"; });
      await act(async () => { await client.invalidateQueries(); await vi.advanceTimersByTimeAsync(15000); });
      expect(postJson).toHaveBeenCalledTimes(1);
      await act(async () => { host.style.display = ""; });
      await act(async () => { await vi.advanceTimersByTimeAsync(20); });
      expect(postJson).toHaveBeenCalledTimes(2);
    } finally { view.unmount(); host.remove(); }
  });
});
